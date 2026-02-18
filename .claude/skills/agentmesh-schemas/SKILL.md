---
name: agentmesh-schemas
description: Canonical database schemas for AgentMesh. PostgreSQL for agent registry (SaaS), ClickHouse for event log (SaaS), SQLite for Scout (OSS). Use when touching any DB code, models, migrations, or queries.
compatibility: sql, postgresql, clickhouse, sqlite, database, schema
---

# AgentMesh Database Schemas

Every subagent that touches DB code MUST use these exact table names, column types, and indexes. No deviations without updating this skill.

## PostgreSQL — Agent Registry (SaaS Platform)

The source of truth for agent identity, configuration, and policy assignments.

```sql
-- Agent Registry: core identity table
CREATE TABLE agents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id),
    name            VARCHAR(255),
    agent_type      VARCHAR(50) CHECK (agent_type IN (
                        'mcp_client', 'langchain', 'crewai', 'autogen',
                        'direct_api', 'a2a', 'custom', 'unknown'
                    )),
    first_seen      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    source_ip       INET,
    fingerprint     VARCHAR(128),          -- composite fingerprint hash
    target_apis     JSONB DEFAULT '[]',    -- LLM API endpoints this agent calls
    mcp_servers     JSONB DEFAULT '[]',    -- MCP servers this agent connects to
    total_requests  BIGINT NOT NULL DEFAULT 0,
    total_tokens_in BIGINT NOT NULL DEFAULT 0,
    total_tokens_out BIGINT NOT NULL DEFAULT 0,
    estimated_cost_usd NUMERIC(12,4) NOT NULL DEFAULT 0,
    status          VARCHAR(20) NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'idle', 'error', 'blocked')),
    policy_ids      UUID[] DEFAULT '{}',   -- assigned governance policies
    metadata        JSONB DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agents_org_id ON agents(org_id);
CREATE INDEX idx_agents_status ON agents(status);
CREATE INDEX idx_agents_last_seen ON agents(last_seen);
CREATE INDEX idx_agents_fingerprint ON agents(fingerprint);
CREATE INDEX idx_agents_agent_type ON agents(agent_type);

-- Governance Policies
CREATE TABLE policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id),
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    policy_type     VARCHAR(50) NOT NULL CHECK (policy_type IN (
                        'rate_limit', 'pii_redaction', 'model_allowlist',
                        'tool_allowlist', 'cost_budget', 'approval_gate',
                        'data_classification', 'custom'
                    )),
    rules           JSONB NOT NULL,        -- policy rule definitions
    enforcement     VARCHAR(20) NOT NULL DEFAULT 'monitor'
                        CHECK (enforcement IN ('monitor', 'warn', 'block')),
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_policies_org_id ON policies(org_id);
CREATE INDEX idx_policies_type ON policies(policy_type);

-- Alert Definitions & History
CREATE TABLE alerts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id),
    agent_id        UUID REFERENCES agents(id),
    request_id      UUID,                  -- references ClickHouse event
    session_id      UUID,                  -- compliance session
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    severity        VARCHAR(10) NOT NULL CHECK (severity IN ('critical', 'warning', 'info')),
    rule_name       VARCHAR(100) NOT NULL,
    message         TEXT NOT NULL,
    details         JSONB DEFAULT '{}',
    compliance_tag  VARCHAR(100),          -- policy-compliance tag
    acknowledged    BOOLEAN NOT NULL DEFAULT false,
    acknowledged_by UUID REFERENCES users(id),
    acknowledged_at TIMESTAMPTZ
);

CREATE INDEX idx_alerts_org_id_ts ON alerts(org_id, timestamp DESC);
CREATE INDEX idx_alerts_agent_id ON alerts(agent_id);
CREATE INDEX idx_alerts_severity ON alerts(severity);
CREATE INDEX idx_alerts_session_id ON alerts(session_id);

-- Scan Results (historical)
CREATE TABLE scan_results (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id),
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    scan_type       VARCHAR(30) NOT NULL CHECK (scan_type IN (
                        'full', 'quick', 'pii_only', 'cost_only'
                    )),
    agents_found    INTEGER NOT NULL DEFAULT 0,
    pii_findings    INTEGER NOT NULL DEFAULT 0,
    risk_score      NUMERIC(4,1) NOT NULL CHECK (risk_score >= 0 AND risk_score <= 100),
    findings        JSONB NOT NULL,
    report_path     TEXT,
    lineage_hash    VARCHAR(64)            -- SHA-256 for data lineage chain
);

CREATE INDEX idx_scan_results_org_ts ON scan_results(org_id, timestamp DESC);
```

## ClickHouse — Event Log (SaaS Platform)

High-volume event storage for all intercepted traffic. Optimized for time-series queries and aggregations.

```sql
-- Intercepted request/response events (main event log)
CREATE TABLE events (
    -- Identity
    id              UUID,
    org_id          UUID,
    agent_id        UUID,
    session_id      UUID,                  -- compliance session grouping

    -- Timing
    timestamp       DateTime64(3, 'UTC'),  -- millisecond precision
    latency_ms      UInt32,

    -- Request metadata
    direction       Enum8('outbound' = 1, 'inbound' = 2),
    method          LowCardinality(String),
    url             String,
    provider        LowCardinality(String),  -- openai, anthropic, google, mcp, a2a
    model           LowCardinality(String),

    -- Payload (stored compressed, queried rarely)
    request_body    String CODEC(ZSTD(3)),
    response_body   String CODEC(ZSTD(3)),
    status_code     UInt16,

    -- Parsed metrics
    tokens_in       UInt32,
    tokens_out      UInt32,
    estimated_cost  Float64,

    -- Governance
    pii_detected    Array(Tuple(
                        type String,
                        location String,
                        confidence Float32
                    )),
    tools_called    Array(String),
    compliance_tag  LowCardinality(String),
    lineage_hash    FixedString(64),       -- SHA-256 chain hash

    -- Partitioning / ordering
    event_date      Date DEFAULT toDate(timestamp)
)
ENGINE = MergeTree()
PARTITION BY toYYYYMM(event_date)
ORDER BY (org_id, agent_id, timestamp)
TTL event_date + INTERVAL 90 DAY
SETTINGS index_granularity = 8192;

-- Materialized view: hourly agent stats
CREATE MATERIALIZED VIEW agent_stats_hourly
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(hour)
ORDER BY (org_id, agent_id, hour)
AS SELECT
    org_id,
    agent_id,
    toStartOfHour(timestamp) AS hour,
    count()                   AS request_count,
    sum(tokens_in)           AS total_tokens_in,
    sum(tokens_out)          AS total_tokens_out,
    sum(estimated_cost)      AS total_cost,
    countIf(length(pii_detected) > 0) AS pii_request_count,
    avg(latency_ms)          AS avg_latency_ms
FROM events
GROUP BY org_id, agent_id, hour;

-- Materialized view: provider usage breakdown
CREATE MATERIALIZED VIEW provider_stats_daily
ENGINE = SummingMergeTree()
PARTITION BY toYYYYMM(day)
ORDER BY (org_id, provider, model, day)
AS SELECT
    org_id,
    provider,
    model,
    toDate(timestamp) AS day,
    count()           AS request_count,
    sum(tokens_in)   AS total_tokens_in,
    sum(tokens_out)  AS total_tokens_out,
    sum(estimated_cost) AS total_cost
FROM events
GROUP BY org_id, provider, model, day;
```

## SQLite — Scout OSS (Local Only)

Lightweight local schema for the open-source Scout tool. Mirrors the PostgreSQL structure but simplified for single-user, single-machine use.

```sql
-- Agents table
CREATE TABLE agents (
    id              TEXT PRIMARY KEY,       -- UUID as text
    name            TEXT,
    agent_type      TEXT CHECK (agent_type IN (
                        'mcp_client', 'langchain', 'crewai', 'autogen',
                        'direct_api', 'unknown'
                    )),
    first_seen      TEXT NOT NULL,          -- ISO 8601 datetime
    last_seen       TEXT NOT NULL,
    source_ip       TEXT,
    target_apis     TEXT DEFAULT '[]',      -- JSON array
    mcp_servers     TEXT DEFAULT '[]',      -- JSON array
    total_requests  INTEGER NOT NULL DEFAULT 0,
    total_tokens_in INTEGER NOT NULL DEFAULT 0,
    total_tokens_out INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'idle', 'error')),
    metadata        TEXT DEFAULT '{}'       -- JSON object
);

-- Requests table
CREATE TABLE requests (
    id              TEXT PRIMARY KEY,       -- UUID as text
    agent_id        TEXT NOT NULL REFERENCES agents(id),
    timestamp       TEXT NOT NULL,          -- ISO 8601 datetime
    direction       TEXT NOT NULL CHECK (direction IN ('outbound', 'inbound')),
    method          TEXT NOT NULL,
    url             TEXT NOT NULL,
    provider        TEXT NOT NULL,          -- openai, anthropic, google, mcp, custom
    model           TEXT,
    request_body    TEXT,
    response_body   TEXT,
    status_code     INTEGER,
    tokens_in       INTEGER,
    tokens_out      INTEGER,
    latency_ms      INTEGER,
    pii_detected    TEXT,                   -- JSON array
    tools_called    TEXT                    -- JSON array
);

CREATE INDEX idx_requests_agent_id ON requests(agent_id);
CREATE INDEX idx_requests_timestamp ON requests(timestamp);
CREATE INDEX idx_requests_provider ON requests(provider);

-- Alerts table
CREATE TABLE alerts (
    id              TEXT PRIMARY KEY,       -- UUID as text
    agent_id        TEXT REFERENCES agents(id),
    request_id      TEXT REFERENCES requests(id),
    timestamp       TEXT NOT NULL,
    severity        TEXT NOT NULL CHECK (severity IN ('critical', 'warning', 'info')),
    rule_name       TEXT NOT NULL,
    message         TEXT NOT NULL,
    details         TEXT DEFAULT '{}',      -- JSON
    acknowledged    INTEGER NOT NULL DEFAULT 0  -- boolean
);

CREATE INDEX idx_alerts_severity ON alerts(severity);
CREATE INDEX idx_alerts_timestamp ON alerts(timestamp);

-- Scan Results table
CREATE TABLE scan_results (
    id              TEXT PRIMARY KEY,       -- UUID as text
    timestamp       TEXT NOT NULL,
    scan_type       TEXT NOT NULL CHECK (scan_type IN (
                        'full', 'quick', 'pii_only', 'cost_only'
                    )),
    agents_found    INTEGER NOT NULL DEFAULT 0,
    pii_findings    INTEGER NOT NULL DEFAULT 0,
    risk_score      REAL NOT NULL CHECK (risk_score >= 0 AND risk_score <= 100),
    findings        TEXT NOT NULL,          -- JSON
    report_path     TEXT
);
```

## Schema Rules

1. **Table names are final.** Do not rename tables. All code references these exact names.
2. **UUID everywhere.** All primary keys are UUIDs. PostgreSQL uses native `uuid`, SQLite uses `TEXT`, ClickHouse uses `UUID`.
3. **JSON columns use JSONB** in PostgreSQL, `TEXT` in SQLite, `String` in ClickHouse.
4. **Timestamps:** PostgreSQL uses `TIMESTAMPTZ`, SQLite uses ISO 8601 text, ClickHouse uses `DateTime64(3, 'UTC')`.
5. **Indexes are mandatory.** Every foreign key and common query filter gets an index.
6. **ClickHouse TTL:** Default 90 days. Configurable per org in SaaS.
7. **No ORM-generated schemas.** SQLAlchemy models must match these schemas exactly — define models to match, don't auto-generate migrations that deviate.

## Checklist for Any DB Change

- [ ] Schema change reflected in ALL three stores (PG, CH, SQLite) where applicable
- [ ] Indexes added for new query patterns
- [ ] Migration script written (Alembic for PG, manual for SQLite)
- [ ] ClickHouse materialized views updated if new aggregatable columns added
- [ ] `lineage_hash` and `session_id` columns present on event/audit tables
- [ ] No PII values stored in alert `details` — only type and location
