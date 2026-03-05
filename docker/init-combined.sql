-- Migration 001: Create the events table (TimescaleDB hypertable)
--
-- This is the core audit log table. Every intercepted agent action lands here.
-- Compliance invariant: session_id, timestamp, lineage_hash, compliance_tag are REQUIRED.
--
-- After creating the table, migration 004 converts it to a TimescaleDB hypertable
-- and applies retention + compression policies.

CREATE TABLE IF NOT EXISTS events (
    -- Identity
    id                  UUID            NOT NULL,
    session_id          UUID            NOT NULL,
    agent_id            VARCHAR(255)    NOT NULL,

    -- Timing (partition key for TimescaleDB)
    timestamp           TIMESTAMPTZ     NOT NULL,
    latency_ms          INTEGER,

    -- Request metadata
    direction           VARCHAR(20)     NOT NULL DEFAULT 'outbound',
    method              VARCHAR(20)     NOT NULL DEFAULT '',
    upstream_target     VARCHAR(1024)   NOT NULL,
    provider            VARCHAR(20)     NOT NULL DEFAULT 'unknown',
    model               VARCHAR(100),

    -- Response metadata
    status_code         INTEGER,
    finish_reason       VARCHAR(50),

    -- Payload storage
    payload             JSONB,
    raw_size_bytes      BIGINT,

    -- Token & cost metrics
    input_tokens        INTEGER,
    output_tokens       INTEGER,
    total_tokens        INTEGER,
    cost_usd            DECIMAL(12, 8),

    -- Governance fields (compliance-first)
    pii_detected        JSONB           NOT NULL DEFAULT '[]',
    tools_called        JSONB           NOT NULL DEFAULT '[]',
    lineage_hash        VARCHAR(64)     NOT NULL,
    compliance_tag      VARCHAR(100)    NOT NULL,
    tags                JSONB           NOT NULL DEFAULT '{}',
    error_message       TEXT,

    -- Audit
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    -- TimescaleDB requires the partition column to be part of the primary key
    PRIMARY KEY (id, timestamp)
);

COMMENT ON TABLE events IS
    'Core audit log: every agent request/response intercepted by the Govrix Scout proxy.';
COMMENT ON COLUMN events.session_id IS
    'Groups related requests in a single agent conversation session.';
COMMENT ON COLUMN events.lineage_hash IS
    'SHA-256 Merkle-chain hash linking this event to its predecessor.';
COMMENT ON COLUMN events.compliance_tag IS
    'Policy evaluation result, e.g. "pass:all", "warn:cost_budget", "audit:pii".';
COMMENT ON COLUMN events.pii_detected IS
    'Array of PII findings: [{pii_type, location, confidence}]. Never stores actual PII values.';
-- Migration 002: Create the agents registry table
--
-- Tracks identity, capabilities, and aggregate statistics for every AI agent
-- observed by the Govrix Scout proxy. The primary key is a VARCHAR agent identifier
-- (not UUID) because agent IDs come from headers, API key mappings, or source IP.
--
-- OSS soft limit: 25 agents. Enforced in application logic, not at DB level.

CREATE TABLE IF NOT EXISTS agents (
    -- Identity
    id                  VARCHAR(255)    NOT NULL PRIMARY KEY,
    name                VARCHAR(255),
    description         TEXT,
    agent_type          VARCHAR(50)     NOT NULL DEFAULT 'unknown',
    status              VARCHAR(20)     NOT NULL DEFAULT 'active',

    -- Lifecycle timestamps
    first_seen_at       TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_seen_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_error_at       TIMESTAMPTZ,

    -- Network identity
    source_ip           INET,
    fingerprint         VARCHAR(64),

    -- API targets observed (JSONB arrays)
    target_apis         JSONB           NOT NULL DEFAULT '[]',
    mcp_servers         JSONB           NOT NULL DEFAULT '[]',

    -- Aggregate statistics (updated on every proxy request via increment_agent_stats)
    total_requests      BIGINT          NOT NULL DEFAULT 0,
    total_tokens_in     BIGINT          NOT NULL DEFAULT 0,
    total_tokens_out    BIGINT          NOT NULL DEFAULT 0,
    total_cost_usd      DECIMAL(16, 8)  NOT NULL DEFAULT 0.0,
    last_model_used     VARCHAR(100),
    error_count         BIGINT          NOT NULL DEFAULT 0,

    -- Labels and metadata
    labels              JSONB           NOT NULL DEFAULT '{}',
    metadata            JSONB           NOT NULL DEFAULT '{}',

    -- Audit timestamps
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE agents IS
    'Agent registry: identity, lifecycle status, and aggregate statistics for every observed AI agent.';
COMMENT ON COLUMN agents.id IS
    'Agent identifier extracted from X-govrix-scout-Agent-Id header, Agent-Name header, API key suffix, or source IP fallback.';
COMMENT ON COLUMN agents.agent_type IS
    'Framework classification: mcp_client, langchain, crewai, autogen, direct_api, a2a, custom, unknown.';
COMMENT ON COLUMN agents.status IS
    'Lifecycle status: active, idle, error, blocked.';
COMMENT ON COLUMN agents.fingerprint IS
    'Composite fingerprint hash for identifying agents without explicit ID (IP + User-Agent + API key prefix).';
COMMENT ON COLUMN agents.source_ip IS
    'Source IP address stored as PostgreSQL INET type for network-aware queries.';

-- Trigger to keep updated_at current on every row update
CREATE OR REPLACE FUNCTION agents_set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER agents_updated_at
    BEFORE UPDATE ON agents
    FOR EACH ROW
    EXECUTE FUNCTION agents_set_updated_at();
-- Migration 003: Create the cost_daily materialized view
--
-- Aggregates cost, token, and latency data from the events table
-- by day Ã— agent Ã— model Ã— protocol.
--
-- Requires TimescaleDB's time_bucket() function (installed via migration 004).
-- Run migration 004 (create_hypertables) before refreshing this view.

CREATE MATERIALIZED VIEW IF NOT EXISTS cost_daily AS
SELECT
    time_bucket('1 day', timestamp)                                     AS day,
    agent_id,
    COALESCE(model, 'unknown')                                          AS model,
    provider                                                            AS protocol,
    COUNT(*)                                                            AS request_count,
    COALESCE(SUM(input_tokens),  0)                                     AS total_input_tokens,
    COALESCE(SUM(output_tokens), 0)                                     AS total_output_tokens,
    COALESCE(SUM(total_tokens),  0)                                     AS total_tokens,
    COALESCE(SUM(cost_usd),      0)                                     AS total_cost_usd,
    AVG(latency_ms)                                                     AS avg_latency_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY latency_ms)            AS p50_latency_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY latency_ms)            AS p95_latency_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY latency_ms)            AS p99_latency_ms
FROM events
GROUP BY day, agent_id, model, provider;

-- Unique index is required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX IF NOT EXISTS cost_daily_pkey
    ON cost_daily (day, agent_id, model, protocol);

COMMENT ON MATERIALIZED VIEW cost_daily IS
    'Daily cost roll-up by agent, model, and provider protocol. Refresh with REFRESH MATERIALIZED VIEW CONCURRENTLY cost_daily.';
-- Migration 004: Convert events to a TimescaleDB hypertable and apply policies
--
-- This migration MUST run after 001_create_events.sql and after the
-- TimescaleDB extension is installed (done automatically by the
-- timescale/timescaledb Docker image used in docker-compose.yml).
--
-- Policies applied:
--   - 1-day chunk interval  (matches MEMORY.md spec)
--   - 7-day data retention  (OSS tier; commercial = unlimited)
--   - 1-day compression     (segments on agent_id, orders on timestamp)

-- Enable the TimescaleDB extension (idempotent)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Convert the events table to a hypertable partitioned on timestamp.
-- migrate_data => true preserves any rows already in the table.
SELECT create_hypertable(
    'events',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    migrate_data        => true,
    if_not_exists       => true
);

-- â”€â”€ Compression â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
-- Compress chunks older than 1 day.
-- Segment on agent_id (keeps per-agent queries fast after decompression).
-- Order on timestamp DESC (most recent rows first within a chunk).
ALTER TABLE events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id',
    timescaledb.compress_orderby   = 'timestamp DESC'
);

SELECT add_compression_policy(
    'events',
    compress_after => INTERVAL '1 day',
    if_not_exists  => true
);

-- â”€â”€ Retention â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
-- Drop chunks older than 7 days (OSS tier).
-- Commercial tier overrides this via the policy engine.
SELECT add_retention_policy(
    'events',
    drop_after    => INTERVAL '7 days',
    if_not_exists => true
);

COMMENT ON TABLE events IS
    'Core audit log (TimescaleDB hypertable): every agent request/response '
    'intercepted by the Govrix Scout proxy. Partitioned by 1-day chunks, '
    'compressed after 1 day, retained for 7 days (OSS).';
-- Migration 005: Create all query-path indexes
--
-- TimescaleDB automatically creates the composite (id, timestamp) primary key
-- index defined in migration 001.  Every index below supports a specific
-- API query pattern from MEMORY.md.
--
-- Naming convention: idx_<table>_<column(s)>

-- â”€â”€ events: common filter / sort columns â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

-- Agent timeline â€” most common query: "show me all events for agent X"
CREATE INDEX IF NOT EXISTS idx_events_agent_id
    ON events (agent_id, timestamp DESC);

-- Session audit trail â€” "reconstruct session S"
CREATE INDEX IF NOT EXISTS idx_events_session_id
    ON events (session_id, timestamp ASC);

-- Filter by provider (openai / anthropic / mcp / â€¦)
CREATE INDEX IF NOT EXISTS idx_events_provider
    ON events (provider, timestamp DESC);

-- Filter by model name (gpt-4o, claude-3-5-sonnet, â€¦)
CREATE INDEX IF NOT EXISTS idx_events_model
    ON events (model, timestamp DESC)
    WHERE model IS NOT NULL;

-- Cost analysis â€” sum cost_usd efficiently
CREATE INDEX IF NOT EXISTS idx_events_cost_usd
    ON events (agent_id, timestamp DESC, cost_usd)
    WHERE cost_usd IS NOT NULL;

-- Status-code filtering â€” find errors quickly
CREATE INDEX IF NOT EXISTS idx_events_status_code
    ON events (status_code, timestamp DESC)
    WHERE status_code IS NOT NULL;

-- JSONB payload full-text search (GIN)
CREATE INDEX IF NOT EXISTS idx_events_payload_gin
    ON events USING GIN (payload jsonb_path_ops)
    WHERE payload IS NOT NULL;

-- JSONB tags filtering (GIN) â€” e.g. tags @> '{"env":"prod"}'
CREATE INDEX IF NOT EXISTS idx_events_tags_gin
    ON events USING GIN (tags);

-- PII findings search (GIN) â€” "find all events with PII type EMAIL_ADDRESS"
CREATE INDEX IF NOT EXISTS idx_events_pii_detected_gin
    ON events USING GIN (pii_detected);

-- Lineage chain lookups â€” follow the Merkle chain
CREATE INDEX IF NOT EXISTS idx_events_lineage_hash
    ON events (lineage_hash);

-- Compliance tag filtering â€” "show all warn:cost_budget events"
CREATE INDEX IF NOT EXISTS idx_events_compliance_tag
    ON events (compliance_tag, timestamp DESC);

-- â”€â”€ agents: common filter / sort columns â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

-- Status filter â€” "list all active agents"
CREATE INDEX IF NOT EXISTS idx_agents_status
    ON agents (status, last_seen_at DESC);

-- Last-seen sort â€” default dashboard sort order
CREATE INDEX IF NOT EXISTS idx_agents_last_seen_at
    ON agents (last_seen_at DESC);

-- Cost sort â€” "which agent costs the most?"
CREATE INDEX IF NOT EXISTS idx_agents_total_cost_usd
    ON agents (total_cost_usd DESC);

-- Source IP lookup â€” used for fingerprint-based agent identification
CREATE INDEX IF NOT EXISTS idx_agents_source_ip
    ON agents (source_ip)
    WHERE source_ip IS NOT NULL;

-- Labels GIN â€” e.g. labels @> '{"team":"ml"}'
CREATE INDEX IF NOT EXISTS idx_agents_labels_gin
    ON agents USING GIN (labels);

-- â”€â”€ cost_daily: query support â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
-- The unique index on (day, agent_id, model, protocol) is already created in
-- migration 003 as a precondition for REFRESH CONCURRENTLY.
-- Add an extra index for time-range + agent lookups.

CREATE INDEX IF NOT EXISTS idx_cost_daily_agent_day
    ON cost_daily (agent_id, day DESC);
-- Migration 006: Create budget_daily table
--
-- Persists daily token and cost usage per agent so that in-memory budget
-- counters survive proxy restarts.  The hot path still reads from in-memory
-- counters (fast, no DB latency); this table is the persistence layer loaded
-- at startup and updated via fire-and-forget background writes.
--
-- Design notes:
--   â€¢ PRIMARY KEY (agent_id, date) enables idempotent ON CONFLICT upserts.
--   â€¢ BIGINT for tokens â€” supports 9.2 Ã— 10^18 tokens per agent per day.
--   â€¢ NUMERIC(10,6) for cost â€” six decimal places matches the cost_usd
--     precision used throughout the events table.
--   â€¢ updated_at is useful for debugging / drift analysis; set automatically.

CREATE TABLE IF NOT EXISTS budget_daily (
    agent_id    VARCHAR(255) NOT NULL,
    date        DATE         NOT NULL DEFAULT CURRENT_DATE,
    tokens_used BIGINT       NOT NULL DEFAULT 0,
    cost_usd    NUMERIC(10,6) NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, date)
);

-- Support fast per-agent lookups for a date range (e.g. monthly reporting).
CREATE INDEX IF NOT EXISTS idx_budget_daily_agent_date
    ON budget_daily (agent_id, date DESC);

-- Support global-total queries (SUM across all agents for a given date).
CREATE INDEX IF NOT EXISTS idx_budget_daily_date
    ON budget_daily (date DESC);
-- Migration 007: Create budget_config table
--
-- Stores per-agent budget limits (daily token, daily cost, monthly cost).
-- The special agent_id '__global__' is the sentinel for global limits that
-- apply across all agents.
--
-- Design notes:
--   * PRIMARY KEY on agent_id â€” one config row per agent.
--   * All limit columns are nullable â€” NULL means "no limit set".
--   * NUMERIC(10,6) for cost limits matches the precision used in budget_daily
--     and events tables.
--   * BIGINT for token limits â€” matches budget_daily.tokens_used precision.
--   * updated_at tracks the last config change for audit purposes.

CREATE TABLE IF NOT EXISTS budget_config (
    agent_id               VARCHAR(255)  NOT NULL PRIMARY KEY,
    daily_token_limit      BIGINT,
    daily_cost_limit_usd   NUMERIC(10,6),
    monthly_cost_limit_usd NUMERIC(10,6),
    created_at             TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ   NOT NULL DEFAULT now()
);
-- Migration 008: Create the projects table â€” groups agents by project/team
--
-- Adds a UUID-keyed projects table and a nullable project_id FK on agents.
-- Backward compatible: existing agents keep project_id = NULL.

CREATE TABLE IF NOT EXISTS projects (
    id          UUID NOT NULL DEFAULT gen_random_uuid() PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    description TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Add project_id to agents table (nullable = backward compatible)
ALTER TABLE agents ADD COLUMN IF NOT EXISTS project_id UUID REFERENCES projects(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_agents_project_id ON agents(project_id) WHERE project_id IS NOT NULL;
