-- Migration 007: Create budget_config table
--
-- Stores per-agent budget limits (daily token, daily cost, monthly cost).
-- The special agent_id '__global__' is the sentinel for global limits that
-- apply across all agents.
--
-- Design notes:
--   * PRIMARY KEY on agent_id — one config row per agent.
--   * All limit columns are nullable — NULL means "no limit set".
--   * NUMERIC(10,6) for cost limits matches the precision used in budget_daily
--     and events tables.
--   * BIGINT for token limits — matches budget_daily.tokens_used precision.
--   * updated_at tracks the last config change for audit purposes.

CREATE TABLE IF NOT EXISTS budget_config (
    agent_id               VARCHAR(255)  NOT NULL PRIMARY KEY,
    daily_token_limit      BIGINT,
    daily_cost_limit_usd   NUMERIC(10,6),
    monthly_cost_limit_usd NUMERIC(10,6),
    created_at             TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ   NOT NULL DEFAULT now()
);
