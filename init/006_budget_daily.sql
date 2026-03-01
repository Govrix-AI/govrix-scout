-- Migration 006: Create budget_daily table
--
-- Persists daily token and cost usage per agent so that in-memory budget
-- counters survive proxy restarts.  The hot path still reads from in-memory
-- counters (fast, no DB latency); this table is the persistence layer loaded
-- at startup and updated via fire-and-forget background writes.
--
-- Design notes:
--   • PRIMARY KEY (agent_id, date) enables idempotent ON CONFLICT upserts.
--   • BIGINT for tokens — supports 9.2 × 10^18 tokens per agent per day.
--   • NUMERIC(10,6) for cost — six decimal places matches the cost_usd
--     precision used throughout the events table.
--   • updated_at is useful for debugging / drift analysis; set automatically.

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
