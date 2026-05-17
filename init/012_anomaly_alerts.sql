-- Migration 012: Anomaly Alerts Hypertable (Stage 3)
--
-- Stores alerts produced by the post-flush anomaly v2 orchestrator.
-- Hypertable partitioned on `timestamp` (1-day chunks), compressed after 1 day,
-- retained for 30 days. Idempotent so re-runs are safe.
--
-- Note: TimescaleDB hypertables require any unique constraint to include the
-- partition column. We therefore do not declare `id` as PRIMARY KEY — we
-- create a separate non-unique index on `id` for lookup.

CREATE TABLE IF NOT EXISTS anomaly_alerts (
    id                  UUID            NOT NULL DEFAULT gen_random_uuid(),
    timestamp           TIMESTAMPTZ     NOT NULL DEFAULT now(),
    agent_id            TEXT            NOT NULL,
    session_id          UUID,
    detector            TEXT            NOT NULL,
    severity            TEXT            NOT NULL
                            CHECK (severity IN ('info', 'warn', 'critical')),
    score               DOUBLE PRECISION NOT NULL,
    details             JSONB           NOT NULL DEFAULT '{}'::jsonb,
    acknowledged_at     TIMESTAMPTZ
);

-- Convert to hypertable (idempotent via if_not_exists).
SELECT create_hypertable(
    'anomaly_alerts',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    migrate_data        => true,
    if_not_exists       => true
);

-- Indexes (id is non-unique because TimescaleDB requires the partition column
-- in any unique constraint).
CREATE INDEX IF NOT EXISTS anomaly_alerts_id_idx
    ON anomaly_alerts (id);

CREATE INDEX IF NOT EXISTS anomaly_alerts_timestamp_idx
    ON anomaly_alerts (timestamp DESC);

CREATE INDEX IF NOT EXISTS anomaly_alerts_agent_timestamp_idx
    ON anomaly_alerts (agent_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS anomaly_alerts_severity_timestamp_idx
    ON anomaly_alerts (severity, timestamp DESC);

CREATE INDEX IF NOT EXISTS anomaly_alerts_detector_timestamp_idx
    ON anomaly_alerts (detector, timestamp DESC);

-- ── Compression policy ───────────────────────────────────────────────────────
ALTER TABLE anomaly_alerts SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id',
    timescaledb.compress_orderby   = 'timestamp DESC'
);

SELECT add_compression_policy(
    'anomaly_alerts',
    compress_after => INTERVAL '1 day',
    if_not_exists  => true
);

-- ── Retention policy ─────────────────────────────────────────────────────────
SELECT add_retention_policy(
    'anomaly_alerts',
    drop_after    => INTERVAL '30 days',
    if_not_exists => true
);

COMMENT ON TABLE anomaly_alerts IS
    'Alerts produced by the post-flush anomaly v2 orchestrator. '
    'Hypertable (1-day chunks), compressed after 1 day, retained for 30 days.';
