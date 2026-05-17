-- Migration 014: TimescaleDB Continuous Aggregates
--
-- Replaces manual materialized views with incrementally refreshed continuous
-- aggregates for cost and latency rollups.
--
-- Both views are created with `IF NOT EXISTS` (TimescaleDB 2.10+ supports it
-- directly on continuous aggregates). They CANNOT be wrapped in a `DO $$`
-- block because `CREATE MATERIALIZED VIEW ... WITH (timescaledb.continuous)`
-- is not allowed inside PL/pgSQL.

-- ── Daily cost by (agent, model, provider) ───────────────────────────────────
CREATE MATERIALIZED VIEW IF NOT EXISTS cost_daily
    WITH (timescaledb.continuous) AS
SELECT
    time_bucket(INTERVAL '1 day', timestamp) AS bucket,
    agent_id,
    provider,
    model,
    SUM(cost_usd)::DOUBLE PRECISION AS total_cost_usd,
    SUM(input_tokens)::BIGINT       AS total_input_tokens,
    SUM(output_tokens)::BIGINT      AS total_output_tokens,
    COUNT(*)::BIGINT                AS request_count
FROM events
GROUP BY bucket, agent_id, provider, model
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'cost_daily',
    start_offset      => INTERVAL '7 days',
    end_offset        => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists     => true
);

-- ── Hourly latency percentiles by provider ───────────────────────────────────
CREATE MATERIALIZED VIEW IF NOT EXISTS latency_by_provider_hour
    WITH (timescaledb.continuous) AS
SELECT
    time_bucket(INTERVAL '1 hour', timestamp) AS bucket,
    provider,
    percentile_cont(0.5)  WITHIN GROUP (ORDER BY latency_ms)::DOUBLE PRECISION AS p50_ms,
    percentile_cont(0.95) WITHIN GROUP (ORDER BY latency_ms)::DOUBLE PRECISION AS p95_ms,
    percentile_cont(0.99) WITHIN GROUP (ORDER BY latency_ms)::DOUBLE PRECISION AS p99_ms,
    COUNT(*)::BIGINT AS sample_count
FROM events
WHERE latency_ms IS NOT NULL
GROUP BY bucket, provider
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'latency_by_provider_hour',
    start_offset      => INTERVAL '24 hours',
    end_offset        => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '15 minutes',
    if_not_exists     => true
);

COMMENT ON MATERIALIZED VIEW cost_daily IS
    'Daily cost rollup by (agent, model, provider) — TimescaleDB continuous aggregate.';
COMMENT ON MATERIALIZED VIEW latency_by_provider_hour IS
    'Hourly p50/p95/p99 latency by provider — TimescaleDB continuous aggregate.';
