//! Event channel and background writer for Govrix Scout proxy.
//!
//! Architecture:
//! - Bounded mpsc channel (10,000 capacity) for fire-and-forget event writes
//! - Proxy hot path uses `try_send` — drops events if channel is full (fail-open)
//! - Background task drains the channel and would batch-insert to DB
//! - Dropped events are counted via atomic counter for metrics
//!
//! Compliance-first invariant: every event sent through this channel MUST
//! already have session_id, timestamp, lineage_hash, and compliance_tag set.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use govrix_scout_common::models::event::AgentEvent;
use moka::sync::Cache;
use tokio::sync::{broadcast, mpsc};

use crate::anomaly::AnomalyAlert;

/// Default capacity of the event channel when no config is provided.
/// Config field `events.channel_capacity` overrides this at startup.
pub const EVENT_CHANNEL_CAPACITY: usize = 100_000;

/// Shared metrics counters for the event pipeline.
#[derive(Debug, Default)]
pub struct EventMetrics {
    /// Total events successfully sent to the channel.
    pub events_sent: AtomicU64,
    /// Total events dropped because the channel was full.
    pub events_dropped: AtomicU64,
    /// Total events processed by the background writer.
    pub events_processed: AtomicU64,
}

impl EventMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

/// Prometheus-facing metrics counters shared between the proxy and API servers.
///
/// A single `Arc<Metrics>` is created at startup and threaded through both
/// `InterceptorState` (proxy hot path) and `AppState` (management API).
/// All fields use `Ordering::Relaxed` — approximate counts are acceptable for metrics.
#[derive(Debug)]
pub struct Metrics {
    /// Total proxy requests intercepted (incremented per forwarded request).
    pub requests_total: AtomicU64,
    /// Total events successfully written to the database.
    pub events_total: AtomicU64,
    /// Number of distinct agents seen in the most recent flush batch.
    pub agents_active: AtomicU64,
    /// Total events dropped due to channel-full / closed channel (fail-open).
    pub events_dropped_total: AtomicU64,
    /// Current depth of the event channel (sampled by writer tasks).
    pub channel_depth: AtomicUsize,
    /// Histogram of upstream latency_ms (buckets: 1, 5, 10, 25, 50, 100, 250, 500, 1000, 5000, +Inf).
    pub upstream_latency_buckets: [AtomicU64; 11],
    /// Sum of upstream latency_ms (for Prometheus _sum).
    pub upstream_latency_sum_ms: AtomicU64,
    /// Total observations recorded into the upstream latency histogram.
    pub upstream_latency_count: AtomicU64,
    /// Kill-switch cache hits.
    pub agent_cache_hits: AtomicU64,
    /// Kill-switch cache misses.
    pub agent_cache_misses: AtomicU64,
}

/// Bucket boundaries (in ms) for the upstream latency histogram.
pub const UPSTREAM_LATENCY_BUCKETS_MS: [u64; 10] = [1, 5, 10, 25, 50, 100, 250, 500, 1000, 5000];

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            events_total: AtomicU64::new(0),
            agents_active: AtomicU64::new(0),
            events_dropped_total: AtomicU64::new(0),
            channel_depth: AtomicUsize::new(0),
            upstream_latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            upstream_latency_sum_ms: AtomicU64::new(0),
            upstream_latency_count: AtomicU64::new(0),
            agent_cache_hits: AtomicU64::new(0),
            agent_cache_misses: AtomicU64::new(0),
        }
    }
}

impl Metrics {
    /// Record an upstream latency observation into the histogram.
    pub fn observe_upstream_latency_ms(&self, value: u64) {
        for (i, b) in UPSTREAM_LATENCY_BUCKETS_MS.iter().enumerate() {
            if value <= *b {
                self.upstream_latency_buckets[i].fetch_add(1, Ordering::Relaxed);
            }
        }
        // +Inf bucket (always increments)
        self.upstream_latency_buckets[10].fetch_add(1, Ordering::Relaxed);
        self.upstream_latency_sum_ms
            .fetch_add(value, Ordering::Relaxed);
        self.upstream_latency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Cache-hit ratio in [0.0, 1.0]. Returns 0.0 if no observations yet.
    pub fn agent_cache_hit_ratio(&self) -> f64 {
        let hits = self.agent_cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.agent_cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

/// Sender side of the event channel.
///
/// Cloneable — one per request/connection is fine.
#[derive(Clone)]
pub struct EventSender {
    tx: mpsc::Sender<AgentEvent>,
    metrics: Arc<EventMetrics>,
    /// Optional Prometheus-facing metrics for drop/depth counters.
    prometheus: Option<Arc<Metrics>>,
}

impl EventSender {
    /// Attach Prometheus metrics to this sender so drop / depth counters
    /// are exposed at `/metrics`.
    pub fn with_prometheus(mut self, prom: Arc<Metrics>) -> Self {
        self.prometheus = Some(prom);
        self
    }
}

impl EventSender {
    /// Send an event fire-and-forget.
    ///
    /// If the channel is full, the event is dropped (counted in metrics).
    /// This NEVER blocks the caller.
    pub fn send(&self, event: AgentEvent) {
        match self.tx.try_send(event) {
            Ok(()) => {
                self.metrics.events_sent.fetch_add(1, Ordering::Relaxed);
                if let Some(ref prom) = self.prometheus {
                    // Track current depth as send_count - processed_count.
                    prom.channel_depth.store(
                        self.tx.max_capacity() - self.tx.capacity(),
                        Ordering::Relaxed,
                    );
                }
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                if let Some(ref prom) = self.prometheus {
                    prom.events_dropped_total.fetch_add(1, Ordering::Relaxed);
                }
                tracing::warn!(
                    dropped_total = self.metrics.events_dropped.load(Ordering::Relaxed),
                    "event channel full — dropping event (fail-open)"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Background writer has exited — count as dropped
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                if let Some(ref prom) = self.prometheus {
                    prom.events_dropped_total.fetch_add(1, Ordering::Relaxed);
                }
                tracing::error!("event channel closed — background writer may have exited");
            }
        }
    }

    /// Current metrics snapshot.
    pub fn metrics(&self) -> &Arc<EventMetrics> {
        &self.metrics
    }
}

/// Create a new event channel with the default capacity.
///
/// Returns `(EventSender, mpsc::Receiver<AgentEvent>)`.
/// The receiver should be passed to `run_background_writer`.
pub fn create_channel() -> (EventSender, mpsc::Receiver<AgentEvent>) {
    create_channel_with_capacity(EVENT_CHANNEL_CAPACITY)
}

/// Create a new event channel with a custom capacity.
pub fn create_channel_with_capacity(capacity: usize) -> (EventSender, mpsc::Receiver<AgentEvent>) {
    let metrics = EventMetrics::new();
    let (tx, rx) = mpsc::channel(capacity);
    let sender = EventSender {
        tx,
        metrics,
        prometheus: None,
    };
    (sender, rx)
}

/// Default capacity of the per-process anomaly alert broadcast channel.
pub const ALERT_BROADCAST_CAPACITY: usize = 64;

/// Configuration knobs for the event background writers.
#[derive(Clone)]
pub struct WriterConfig {
    pub writer_tasks: usize,
    pub batch_size: usize,
    pub batch_interval_ms: u64,
    /// Anomaly detection configuration, cloned into each writer task.
    pub anomaly: govrix_scout_common::config::AnomalyConfig,
    /// Optional broadcast sender used to fan out anomaly alerts to live
    /// subscribers (SSE consumers). `None` disables the fan-out.
    pub alert_tx: Option<broadcast::Sender<AnomalyAlert>>,
}

impl std::fmt::Debug for WriterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WriterConfig")
            .field("writer_tasks", &self.writer_tasks)
            .field("batch_size", &self.batch_size)
            .field("batch_interval_ms", &self.batch_interval_ms)
            .field("anomaly", &self.anomaly)
            .field("alert_tx", &self.alert_tx.is_some())
            .finish()
    }
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            writer_tasks: 4,
            batch_size: 500,
            batch_interval_ms: 50,
            anomaly: govrix_scout_common::config::AnomalyConfig::default(),
            alert_tx: None,
        }
    }
}

/// Shared `seen_agents` cache used by the writer pool.
///
/// Moka cache replaces the unbounded `HashSet` so memory cannot grow forever
/// as new agent IDs arrive. Cap 50k entries, idle-TTL 24h.
pub fn build_seen_agents_cache() -> Arc<Cache<String, ()>> {
    Arc::new(
        Cache::builder()
            .max_capacity(50_000)
            .time_to_idle(std::time::Duration::from_secs(24 * 60 * 60))
            .build(),
    )
}

/// Spawn N writer tasks that share a single mpsc receiver.
///
/// Each writer drains up to `batch_size` events (or waits up to
/// `batch_interval_ms`) then flushes via [`flush_batch`]. The receiver is
/// guarded by a `tokio::sync::Mutex` — only `recv()` is contended, all DB
/// work happens outside the lock and runs in parallel across tasks.
pub fn spawn_writer_pool(
    rx: mpsc::Receiver<AgentEvent>,
    event_metrics: Arc<EventMetrics>,
    pool: Option<govrix_scout_store::StorePool>,
    metrics: Arc<Metrics>,
    cfg: WriterConfig,
) -> Vec<tokio::task::JoinHandle<()>> {
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    let seen_agents = build_seen_agents_cache();
    let n = cfg.writer_tasks.max(1);
    let mut handles = Vec::with_capacity(n);
    for id in 0..n {
        let rx = rx.clone();
        let event_metrics = event_metrics.clone();
        let pool = pool.clone();
        let metrics = metrics.clone();
        let seen_agents = seen_agents.clone();
        let cfg = cfg.clone();
        handles.push(tokio::spawn(async move {
            tracing::info!(writer = id, "event writer task started");
            run_writer_task(rx, event_metrics, pool, metrics, seen_agents, cfg, id).await;
        }));
    }
    handles
}

/// Legacy entry point — single writer task. Kept for backward compatibility
/// with `main.rs` callers that pass a single receiver and don't want a pool.
pub async fn run_background_writer(
    rx: mpsc::Receiver<AgentEvent>,
    event_metrics: Arc<EventMetrics>,
    pool: Option<govrix_scout_store::StorePool>,
    metrics: Arc<Metrics>,
) {
    let rx = Arc::new(tokio::sync::Mutex::new(rx));
    let seen_agents = build_seen_agents_cache();
    run_writer_task(
        rx,
        event_metrics,
        pool,
        metrics,
        seen_agents,
        WriterConfig::default(),
        0,
    )
    .await;
}

async fn run_writer_task(
    rx: Arc<tokio::sync::Mutex<mpsc::Receiver<AgentEvent>>>,
    event_metrics: Arc<EventMetrics>,
    pool: Option<govrix_scout_store::StorePool>,
    metrics: Arc<Metrics>,
    seen_agents: Arc<Cache<String, ()>>,
    cfg: WriterConfig,
    _writer_id: usize,
) {
    let alert_tx = cfg.alert_tx.clone();
    let mut batch: Vec<AgentEvent> = Vec::with_capacity(cfg.batch_size);
    let batch_interval = tokio::time::Duration::from_millis(cfg.batch_interval_ms);
    // Per-task anomaly orchestrator (state is approximate per task — acceptable).
    let anomaly_cfg = cfg.anomaly.clone();
    let mut orchestrator = crate::anomaly::AnomalyOrchestrator::from_config(&anomaly_cfg);

    // Cold-start seed: load baseline (agent, model) stats from the DB once per
    // writer task. Fail-soft: errors are logged but never abort the writer.
    if anomaly_cfg.enabled {
        if let Some(p) = pool.as_ref() {
            let mut seeded_state =
                crate::anomaly::AnomalyState::with_capacity(anomaly_cfg.state_lru_cap);
            match crate::anomaly::state::seed_from_pool(
                p,
                &mut seeded_state,
                anomaly_cfg.cold_start_window_hours,
            )
            .await
            {
                Ok(n) => {
                    tracing::info!(
                        seeded_pairs = n,
                        window_hours = anomaly_cfg.cold_start_window_hours,
                        "anomaly cold-start seed complete"
                    );
                    orchestrator.replace_state(seeded_state);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "anomaly cold-start seed failed — detectors will warm up from live traffic"
                    );
                }
            }
        }
    }

    loop {
        let deadline = tokio::time::Instant::now() + batch_interval;

        // Drain a batch from the shared receiver.
        loop {
            let mut guard = rx.lock().await;
            tokio::select! {
                biased;
                event = guard.recv() => {
                    match event {
                        Some(ev) => {
                            drop(guard); // release before any heavy work
                            tracing::trace!(
                                event_id = %ev.id,
                                agent = %ev.agent_id,
                                "event received"
                            );
                            batch.push(ev);
                            event_metrics.events_processed.fetch_add(1, Ordering::Relaxed);
                            if batch.len() >= cfg.batch_size {
                                break;
                            }
                        }
                        None => {
                            // Channel closed — flush and exit.
                            drop(guard);
                            tracing::warn!(
                                "event channel closed, flushing {} remaining events",
                                batch.len()
                            );
                            flush_batch(&mut batch, &pool, &metrics, &seen_agents, &mut orchestrator, &alert_tx).await;
                            return;
                        }
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    break;
                }
            }
        }

        if !batch.is_empty() {
            flush_batch(
                &mut batch,
                &pool,
                &metrics,
                &seen_agents,
                &mut orchestrator,
                &alert_tx,
            )
            .await;
        }
    }
}

/// Flush the current batch of events to PostgreSQL (or just log if no pool).
///
/// Fail-open: database errors are logged as warnings but never crash the proxy.
async fn flush_batch(
    batch: &mut Vec<AgentEvent>,
    pool: &Option<govrix_scout_store::StorePool>,
    metrics: &Arc<Metrics>,
    seen_agents: &Arc<Cache<String, ()>>,
    orchestrator: &mut crate::anomaly::AnomalyOrchestrator,
    alert_tx: &Option<broadcast::Sender<AnomalyAlert>>,
) {
    if batch.is_empty() {
        return;
    }

    match pool {
        Some(p) => {
            match govrix_scout_store::events::insert_events_batch(p, batch).await {
                Ok(count) => {
                    tracing::debug!(count, "flushed event batch to PostgreSQL");
                    metrics
                        .events_total
                        .fetch_add(count as u64, Ordering::Relaxed);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        count = batch.len(),
                        "failed to flush event batch to PostgreSQL (fail-open, events dropped)"
                    );
                }
            }
            upsert_agents_from_batch(p, batch, metrics, seen_agents).await;
        }
        None => {
            tracing::debug!(
                count = batch.len(),
                "flushing event batch (no DB pool — events discarded)"
            );
        }
    }

    // ── Anomaly v2 post-flush ─────────────────────────────────────────────
    // Run all detectors *after* the DB insert. Zero hot-path cost.
    if orchestrator.enabled {
        let mut collected: Vec<AnomalyAlert> = Vec::new();
        for ev in batch.iter() {
            for alert in orchestrator.process(ev) {
                tracing::info!(
                    detector = %alert.detector,
                    severity = %alert.severity,
                    agent = %alert.agent_id,
                    score = alert.score,
                    "anomaly alert"
                );
                collected.push(alert);
            }
        }

        if !collected.is_empty() {
            tracing::debug!(
                emitted = collected.len(),
                "anomaly alerts produced from batch"
            );

            // Persist (fail-soft).
            if let Some(p) = pool {
                let new_alerts: Vec<govrix_scout_store::NewAlert> = collected
                    .iter()
                    .map(|a| govrix_scout_store::NewAlert {
                        id: a.id,
                        timestamp: a.timestamp,
                        agent_id: a.agent_id.clone(),
                        session_id: a.session_id,
                        detector: a.detector.clone(),
                        severity: a.severity.to_string(),
                        score: a.score,
                        details: a.details.clone(),
                    })
                    .collect();

                if let Err(e) = govrix_scout_store::insert_alerts_batch(p, &new_alerts).await {
                    tracing::warn!(
                        error = %e,
                        count = new_alerts.len(),
                        "failed to persist anomaly alerts (fail-soft, continuing)"
                    );
                }
            }

            // Broadcast for SSE subscribers (fail-soft — no receivers is fine).
            if let Some(tx) = alert_tx {
                for a in &collected {
                    let _ = tx.send(a.clone());
                }
            }
        }
    }

    batch.clear();
}

/// Upsert agent records from a batch of events in a single UNNEST round-trip.
///
/// Deduplicates by agent_id, aggregating token and cost stats across all events
/// for each unique agent within the batch.
///
/// Fail-open: errors are logged as warnings but never propagated.
async fn upsert_agents_from_batch(
    pool: &govrix_scout_store::StorePool,
    batch: &[AgentEvent],
    metrics: &Arc<Metrics>,
    seen_agents: &Arc<Cache<String, ()>>,
) {
    use govrix_scout_store::AgentBatchStats;

    let mut agents: HashMap<&str, AgentBatchStats> = HashMap::new();

    for ev in batch {
        let entry = agents
            .entry(ev.agent_id.as_str())
            .or_insert_with(|| AgentBatchStats {
                agent_id: ev.agent_id.clone(),
                last_model: None,
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                request_count: 0,
            });
        entry.request_count += 1;
        entry.tokens_in += ev.input_tokens.unwrap_or(0) as i64;
        entry.tokens_out += ev.output_tokens.unwrap_or(0) as i64;
        if let Some(d) = ev.cost_usd {
            entry.cost_usd += rust_decimal::prelude::ToPrimitive::to_f64(&d).unwrap_or(0.0);
        }
        if ev.model.is_some() {
            entry.last_model = ev.model.clone();
        }
    }

    if agents.is_empty() {
        return;
    }

    let stats: Vec<AgentBatchStats> = agents.values().cloned().collect();
    if let Err(e) = govrix_scout_store::upsert_agents_batch(pool, &stats).await {
        tracing::warn!(
            error = %e,
            count = stats.len(),
            "failed to upsert agents batch (fail-open, agent stats may be stale)"
        );
    }

    // Record into the moka cache. `entry_count()` is approximate but fine
    // for an agents_active gauge.
    for agent_id in agents.keys() {
        seen_agents.insert((*agent_id).to_string(), ());
    }
    seen_agents.run_pending_tasks();
    metrics
        .agents_active
        .store(seen_agents.entry_count(), Ordering::Relaxed);

    tracing::debug!(
        unique_agents = agents.len(),
        agents_cached = seen_agents.entry_count(),
        "upserted agents from event batch"
    );
}

/// Compute a SHA-256 lineage hash linking this event to the previous one.
///
/// The lineage hash creates a Merkle-like chain for tamper evidence.
/// First event in a session uses "GENESIS" as the previous hash.
///
/// Hash input: `"{prev_hash}|{event_id}|{agent_id}|{timestamp_ms}"`
pub fn compute_lineage_hash(
    prev_hash: &str,
    event_id: &uuid::Uuid,
    agent_id: &str,
    timestamp_ms: i64,
) -> String {
    use sha2::{Digest, Sha256};

    let input = format!("{}|{}|{}|{}", prev_hash, event_id, agent_id, timestamp_ms);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Session tracker — assigns and tracks session IDs per agent.
///
/// A session groups related requests from the same agent into a conversation.
/// Backed by a `DashMap` so all reads and writes are lock-free at the map level
/// and only ever hold a shard guard for a single atomic operation. The
/// `&self` methods can be called concurrently from many tasks.
pub struct SessionTracker {
    sessions: DashMap<String, SessionState>,
    session_idle_timeout: std::time::Duration,
}

#[derive(Clone)]
struct SessionState {
    session_id: uuid::Uuid,
    #[allow(dead_code)]
    last_event_id: uuid::Uuid,
    last_lineage_hash: String,
    last_seen: std::time::Instant,
}

impl SessionTracker {
    /// Create a new session tracker with a default 30-minute idle timeout.
    pub fn new() -> Self {
        Self::with_timeout(std::time::Duration::from_secs(30 * 60))
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            sessions: DashMap::new(),
            session_idle_timeout: timeout,
        }
    }

    /// Get or create a session for the given agent_id.
    ///
    /// Returns `(session_id, prev_lineage_hash)`.
    /// Performs a single atomic `entry().or_insert_with()` — no awaits, no
    /// long-held locks. Idle sessions (older than `session_idle_timeout`) are
    /// rotated in-place.
    pub fn get_or_create(&self, agent_id: &str, event_id: &uuid::Uuid) -> (uuid::Uuid, String) {
        let now = std::time::Instant::now();
        let timeout = self.session_idle_timeout;

        let mut entry = self
            .sessions
            .entry(agent_id.to_string())
            .or_insert_with(|| {
                let session_id = uuid::Uuid::now_v7();
                let genesis_hash = compute_lineage_hash("GENESIS", event_id, agent_id, 0);
                SessionState {
                    session_id,
                    last_event_id: *event_id,
                    last_lineage_hash: genesis_hash,
                    last_seen: now,
                }
            });

        // Rotate idle session in-place if needed
        if now.duration_since(entry.last_seen) > timeout {
            let session_id = uuid::Uuid::now_v7();
            let genesis_hash = compute_lineage_hash("GENESIS", event_id, agent_id, 0);
            entry.session_id = session_id;
            entry.last_event_id = *event_id;
            entry.last_lineage_hash = genesis_hash.clone();
            entry.last_seen = now;
            return (session_id, genesis_hash);
        }

        (entry.session_id, entry.last_lineage_hash.clone())
    }

    /// Record that an event was processed, updating the lineage chain.
    pub fn record_event(&self, agent_id: &str, event_id: uuid::Uuid, lineage_hash: String) {
        if let Some(mut state) = self.sessions.get_mut(agent_id) {
            state.last_event_id = event_id;
            state.last_lineage_hash = lineage_hash;
            state.last_seen = std::time::Instant::now();
        }
    }
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_hash_is_deterministic() {
        let id = uuid::Uuid::nil();
        let h1 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        let h2 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        assert_eq!(h1, h2);
    }

    #[test]
    fn lineage_hash_changes_with_prev() {
        let id = uuid::Uuid::nil();
        let h1 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        let h2 = compute_lineage_hash(&h1, &id, "agent-1", 2000);
        assert_ne!(h1, h2);
    }

    #[test]
    fn session_tracker_creates_session() {
        let tracker = SessionTracker::new();
        let event_id = uuid::Uuid::now_v7();
        let (session_id, hash) = tracker.get_or_create("agent-1", &event_id);
        assert!(!hash.is_empty());

        // Same agent gets same session
        let (session_id2, _) = tracker.get_or_create("agent-1", &event_id);
        assert_eq!(session_id, session_id2);
    }

    #[test]
    fn session_tracker_different_agents_get_different_sessions() {
        let tracker = SessionTracker::new();
        let event_id = uuid::Uuid::now_v7();
        let (s1, _) = tracker.get_or_create("agent-1", &event_id);
        let (s2, _) = tracker.get_or_create("agent-2", &event_id);
        assert_ne!(s1, s2);
    }

    #[test]
    fn event_channel_try_send_non_blocking() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (sender, mut rx) = create_channel();

            // Fill channel and then some — must not block
            let event = govrix_scout_common::models::event::AgentEvent::new(
                "agent-1",
                uuid::Uuid::now_v7(),
                govrix_scout_common::models::event::EventDirection::Outbound,
                "POST",
                "https://api.openai.com/v1/chat/completions",
                govrix_scout_common::models::event::Provider::OpenAI,
                "genesis",
                "audit:none",
            );

            sender.send(event.clone());
            sender.send(event.clone());

            // Drain
            let _ = rx.recv().await;
            let _ = rx.recv().await;
        });
    }
}
