-- Migration 013: Schema fixes for risk_score / event_kind on events.
--
-- The compliance_queries module references columns that were never added to
-- the base events table. Add them defensively (idempotent) so reads do not
-- error out on a fresh database.

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS risk_score  DOUBLE PRECISION NULL;

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS event_kind  TEXT NULL;

CREATE INDEX IF NOT EXISTS events_event_kind_timestamp_idx
    ON events (event_kind, timestamp DESC)
    WHERE event_kind IS NOT NULL;

CREATE INDEX IF NOT EXISTS events_risk_score_idx
    ON events (risk_score)
    WHERE risk_score IS NOT NULL;

COMMENT ON COLUMN events.risk_score IS
    'Optional 0-100 risk score attached by the policy engine.';
COMMENT ON COLUMN events.event_kind IS
    'Optional fine-grained event kind label (e.g. llm.request, policy.block).';
