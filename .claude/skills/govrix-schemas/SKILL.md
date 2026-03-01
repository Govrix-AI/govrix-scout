---
name: govrix-schemas
description: Canonical database schemas for Govrix Scout. PostgreSQL 16 + TimescaleDB only. Use when touching any DB code, models, init SQL, or queries.
compatibility: sql, postgresql, timescaledb, database, schema
---

# Govrix Scout — Database Schemas

Single store: **PostgreSQL 16 + TimescaleDB**. No ClickHouse. No SQLite. No multi-tenancy.

Every subagent that touches DB code MUST use these exact table names, column types, and indexes.

---

## events (TimescaleDB hypertable)

Core audit log. Every intercepted request/response lands here.

```sql
CREATE TABLE IF NOT EXISTS events (
    -- Identity
    id                  UUID            NOT NULL,
    session_id          UUID            NOT NULL,   -- groups a conversation
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

    -- Payload
    payload             JSONB,
    raw_size_bytes      BIGINT,

    -- Token & cost metrics
    input_tokens        INTEGER,
    output_tokens       INTEGER,
    total_tokens        INTEGER,
    cost_usd            DECIMAL(12, 8),

    -- Compliance fields (ALL FOUR are REQUIRED — never nullable)
    pii_detected        JSONB           NOT NULL DEFAULT '[]',
    tools_called        JSONB           NOT NULL DEFAULT '[]',
    lineage_hash        VARCHAR(64)     NOT NULL,   -- SHA-256 Merkle chain
    compliance_tag      VARCHAR(100)    NOT NULL,   -- "pass:all", "warn:pii_email", etc.

    tags                JSONB           NOT NULL DEFAULT '{}',
    error_message       TEXT,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    PRIMARY KEY (id, timestamp)   -- timestamp required by TimescaleDB
);
```

**Compliance invariant:** `session_id`, `timestamp`, `lineage_hash`, `compliance_tag` must be present on every row. No exceptions.

---

## agents

Registry of every AI agent observed by the proxy.

```sql
CREATE TABLE IF NOT EXISTS agents (
    id                  VARCHAR(255)    NOT NULL PRIMARY KEY,  -- NOT uuid; from header/key/IP
    name                VARCHAR(255),
    description         TEXT,
    agent_type          VARCHAR(50)     NOT NULL DEFAULT 'unknown',
    status              VARCHAR(20)     NOT NULL DEFAULT 'active',

    first_seen_at       TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_seen_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_error_at       TIMESTAMPTZ,

    source_ip           INET,
    fingerprint         VARCHAR(64),

    target_apis         JSONB           NOT NULL DEFAULT '[]',
    mcp_servers         JSONB           NOT NULL DEFAULT '[]',

    -- Aggregate stats (incremented on every proxy request)
    total_requests      BIGINT          NOT NULL DEFAULT 0,
    total_tokens_in     BIGINT          NOT NULL DEFAULT 0,
    total_tokens_out    BIGINT          NOT NULL DEFAULT 0,
    total_cost_usd      DECIMAL(16, 8)  NOT NULL DEFAULT 0.0,
    last_model_used     VARCHAR(100),
    error_count         BIGINT          NOT NULL DEFAULT 0,

    labels              JSONB           NOT NULL DEFAULT '{}',
    metadata            JSONB           NOT NULL DEFAULT '{}',
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);
```

**Agent ID priority:** `X-Agent-Id` header → `Agent-Name` header → API key suffix → source IP → `"unknown"`

**OSS soft limit:** 25 agents. Enforced in application logic, not at DB level.

---

## cost_daily (materialized view)

Daily cost roll-up by agent × model × provider.

```sql
CREATE MATERIALIZED VIEW IF NOT EXISTS cost_daily AS
SELECT
    time_bucket('1 day', timestamp)  AS day,
    agent_id,
    COALESCE(model, 'unknown')       AS model,
    provider                         AS protocol,
    COUNT(*)                         AS request_count,
    COALESCE(SUM(input_tokens),  0)  AS total_input_tokens,
    COALESCE(SUM(output_tokens), 0)  AS total_output_tokens,
    COALESCE(SUM(total_tokens),  0)  AS total_tokens,
    COALESCE(SUM(cost_usd),      0)  AS total_cost_usd,
    AVG(latency_ms)                  AS avg_latency_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY latency_ms) AS p50_latency_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY latency_ms) AS p95_latency_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY latency_ms) AS p99_latency_ms
FROM events
GROUP BY day, agent_id, model, provider;

CREATE UNIQUE INDEX IF NOT EXISTS cost_daily_pkey
    ON cost_daily (day, agent_id, model, protocol);
```

Refresh: `REFRESH MATERIALIZED VIEW CONCURRENTLY cost_daily`

---

## Schema Rules

1. **One database** — PostgreSQL + TimescaleDB only. No ClickHouse, no SQLite.
2. **events primary key** is `(id, timestamp)` — TimescaleDB requires the partition column in PK.
3. **agents.id is VARCHAR** — not UUID. Agent IDs come from headers/API keys/IP, not generated by DB.
4. **All four compliance fields are mandatory** on every events row — never make them nullable.
5. **JSONB for arrays/objects** — `pii_detected`, `tools_called`, `payload`, `target_apis`, `mcp_servers`.
6. **TimescaleDB retention** — 7 days (OSS). Configured in `init/004_create_hypertables.sql`.
7. **Init files live in `init/`** — run in order: 001, 002, 004, 003, 005.

## Checklist for Any DB Change

- [ ] Update `init/` SQL files
- [ ] Update Rust model structs in `govrix-scout-store/src/`
- [ ] Update `govrix-scout-common` types if shared
- [ ] Add index for any new query filter column
- [ ] `lineage_hash` and `compliance_tag` present on any new event-like table
- [ ] No PII values stored — only type, location, confidence
