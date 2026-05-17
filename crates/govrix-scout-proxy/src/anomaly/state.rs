//! Bounded per-(agent, model) sketch state for anomaly detection.
//!
//! `AnomalyState` wraps an `LruCache<Key, SketchBundle>` capped at
//! `config.anomaly.state_lru_cap` so memory cannot grow unbounded as new
//! agent/model pairs arrive.
//!
//! Cold-start seeding via [`seed_from_db`] runs once at proxy startup so the
//! first events after a deploy do not trigger false positives during warm-up.

use std::num::NonZeroUsize;

use chrono::{DateTime, Utc};
use govrix_scout_common::models::event::AgentEvent;
use lru::LruCache;

use super::sketches::{record_hour, SketchBundle};

/// Composite key — every sketch is partitioned by (agent_id, model).
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct AgentModelKey {
    pub agent_id: String,
    pub model: String,
}

impl AgentModelKey {
    pub fn from_event(e: &AgentEvent) -> Self {
        Self {
            agent_id: e.agent_id.clone(),
            model: e.model.clone().unwrap_or_else(|| "_none_".to_string()),
        }
    }
}

/// LRU-backed map of (agent, model) → sketches.
///
/// Not `Send + Sync` on its own — wrap in a single-writer task path or a
/// `Mutex` if accessed from multiple tasks. The orchestrator owns this and
/// runs in the writer-task path (single-threaded per task).
pub struct AnomalyState {
    inner: LruCache<AgentModelKey, SketchBundle>,
}

impl AnomalyState {
    /// Create a bounded state map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        let cap = NonZeroUsize::new(cap.max(1)).unwrap();
        Self {
            inner: LruCache::new(cap),
        }
    }

    /// Number of (agent, model) pairs currently tracked.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get a mutable handle to the bundle for the event, creating an empty
    /// bundle if this is the first event for that pair. Promotes the key to
    /// MRU position.
    pub fn get_or_insert(&mut self, key: AgentModelKey) -> &mut SketchBundle {
        if !self.inner.contains(&key) {
            self.inner.put(key.clone(), SketchBundle::new());
        }
        self.inner.get_mut(&key).unwrap()
    }

    /// Lookup without touching LRU order. Returns `None` if no entry.
    pub fn peek(&self, key: &AgentModelKey) -> Option<&SketchBundle> {
        self.inner.peek(key)
    }

    /// Insert a seed bundle without bumping LRU order (used at startup).
    pub fn put_seed(&mut self, key: AgentModelKey, bundle: SketchBundle) {
        self.inner.put(key, bundle);
    }

    /// Apply the event to the sketch bundle (mutate-only — detectors do their
    /// own reads against the pre-update state).
    pub fn ingest(&mut self, e: &AgentEvent) {
        let key = AgentModelKey::from_event(e);
        let bundle = self.get_or_insert(key);
        let cost = e
            .cost_usd
            .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
            .unwrap_or(0.0);
        bundle.update_cost(cost);
        bundle.update_digests(e.latency_ms, cost);

        let tokens = e.input_tokens.unwrap_or(0) as f64 + e.output_tokens.unwrap_or(0) as f64;
        let alpha = 0.1_f64;
        bundle.token_ema = if bundle.token_ema == 0.0 {
            tokens
        } else {
            bundle.token_ema * (1.0 - alpha) + tokens * alpha
        };

        if let Some(tn) = &e.tool_name {
            bundle.tool_cardinality.insert(tn);
            let est = bundle.tool_cardinality.estimate();
            bundle.update_fanout(est);
        }
        bundle.providers_seen.insert(e.provider.to_string());

        let is_error = e.status_code.map(|s| s >= 500 || s == 429).unwrap_or(false);
        bundle.error_window.push(if is_error { 1.0 } else { 0.0 });

        record_hour(bundle, e.timestamp);
        bundle.samples = bundle.samples.saturating_add(1);
    }
}

/// Seed from a `StorePool` — used by the proxy binary on startup.
///
/// The query is intentionally idempotent and read-only:
///
/// ```sql
/// SELECT agent_id, model,
///        AVG(cost_usd), STDDEV(cost_usd),
///        AVG(latency_ms), STDDEV(latency_ms),
///        COUNT(*)
/// FROM events
/// WHERE timestamp > now() - interval 'N hours'
///   AND model IS NOT NULL
/// GROUP BY agent_id, model
/// ```
///
/// `STDDEV` returns `NULL` for groups with a single row; we treat that as
/// `Option<f64>::None` and let the detector warm up.
pub async fn seed_from_pool(
    pool: &sqlx::PgPool,
    store: &mut AnomalyState,
    window_hours: u32,
) -> anyhow::Result<usize> {
    let sql = format!(
        "SELECT agent_id, \
                COALESCE(model, '_none_') AS model, \
                AVG(cost_usd)::float8        AS avg_cost, \
                STDDEV(cost_usd)::float8     AS stddev_cost, \
                AVG(latency_ms)::float8      AS avg_latency, \
                STDDEV(latency_ms)::float8   AS stddev_latency, \
                COUNT(*)::int8               AS samples \
         FROM events \
         WHERE timestamp > now() - interval '{} hours' \
         GROUP BY agent_id, model",
        window_hours
    );

    let rows: Vec<SeedRow> = match sqlx::query_as::<_, SeedRow>(&sql).fetch_all(pool).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "anomaly cold-start seed query failed — detectors will warm up from live traffic"
            );
            return Ok(0);
        }
    };

    let mut count = 0usize;
    for row in rows {
        let bundle = SketchBundle::from_seed(
            row.avg_cost.unwrap_or(0.0),
            row.stddev_cost,
            row.avg_latency.unwrap_or(0.0),
            row.stddev_latency,
            row.samples.max(0) as u64,
        );
        store.put_seed(
            AgentModelKey {
                agent_id: row.agent_id,
                model: row.model,
            },
            bundle,
        );
        count += 1;
    }
    Ok(count)
}

#[derive(sqlx::FromRow)]
struct SeedRow {
    agent_id: String,
    model: String,
    avg_cost: Option<f64>,
    stddev_cost: Option<f64>,
    avg_latency: Option<f64>,
    stddev_latency: Option<f64>,
    samples: i64,
}

/// Convenience: timestamp accessor for last-seen heuristics.
pub fn last_seen(bundle: &SketchBundle) -> DateTime<Utc> {
    bundle.last_seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{AgentEvent, EventDirection, Provider};
    use uuid::Uuid;

    fn ev(agent: &str, model: Option<&str>) -> AgentEvent {
        let mut e = AgentEvent::new(
            agent,
            Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "https://example.com",
            Provider::OpenAI,
            "g",
            "audit:none",
        );
        e.model = model.map(String::from);
        e
    }

    #[test]
    fn ingest_creates_bundle() {
        let mut s = AnomalyState::with_capacity(8);
        s.ingest(&ev("a", Some("gpt-4")));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn lru_cap_evicts() {
        let mut s = AnomalyState::with_capacity(2);
        s.ingest(&ev("a", Some("m1")));
        s.ingest(&ev("b", Some("m2")));
        s.ingest(&ev("c", Some("m3")));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn distinct_models_distinct_buckets() {
        let mut s = AnomalyState::with_capacity(8);
        s.ingest(&ev("a", Some("gpt-4")));
        s.ingest(&ev("a", Some("gpt-3")));
        assert_eq!(s.len(), 2);
    }
}
