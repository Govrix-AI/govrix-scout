//! Anomaly detection v2 — post-flush, sketch-backed, multi-detector.
//!
//! Architecture (Stage 2):
//!
//! ```text
//! writer task ──► flush_batch (DB UNNEST) ──► orchestrator.process(event)
//!                                                  │
//!                                                  ▼
//!                                             AnomalyState (LRU)
//!                                                  │
//!                                                  ▼
//!                                       [detector_1 … detector_N]
//!                                                  │
//!                                                  ▼
//!                                          Vec<AnomalyAlert>
//!                                                  │
//!                                                  ▼
//!                                      mpsc to alert sink (Stage 3 persists)
//! ```
//!
//! # Guarantees
//!
//! * **Post-flush.** Detectors run *after* the DB insert succeeds. Zero hot-path cost.
//! * **Fail-soft.** A detector panic is caught and logged — never bubbles out.
//! * **Bounded memory.** State is an `LruCache` capped via `AnomalyConfig::state_lru_cap`.
//! * **No detector talks to the DB during `process`.** Only [`state::seed_from_pool`] does, once at startup.

use std::panic::{catch_unwind, AssertUnwindSafe};

use chrono::{DateTime, Utc};
use govrix_scout_common::config::AnomalyConfig;
use govrix_scout_common::models::event::AgentEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod detectors;
pub mod sketches;
pub mod state;

pub use sketches::SketchBundle;
pub use state::{AgentModelKey, AnomalyState};

// ── Alert types ──────────────────────────────────────────────────────────────

/// Severity rating for an alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warn => write!(f, "warn"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// A single anomaly alert produced by a detector.
///
/// Stage 3 will persist this; Stage 2 only emits via the writer-pool log sink.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    /// Stable alert id (UUIDv7).
    pub id: Uuid,
    /// When the alert was raised (UTC).
    pub timestamp: DateTime<Utc>,
    /// Agent the alert concerns.
    pub agent_id: String,
    /// Optional session correlation.
    pub session_id: Option<Uuid>,
    /// Detector that emitted the alert (machine-readable, stable).
    pub detector: String,
    /// Severity bucket.
    pub severity: Severity,
    /// Numeric score the detector produced (z-score, ratio, etc.).
    pub score: f64,
    /// Free-form structured evidence.
    pub details: serde_json::Value,
}

impl AnomalyAlert {
    /// Helper used by detectors so the shape is uniform.
    pub fn new(
        agent_id: impl Into<String>,
        session_id: Option<Uuid>,
        detector: &'static str,
        severity: Severity,
        score: f64,
        details: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            timestamp: Utc::now(),
            agent_id: agent_id.into(),
            session_id,
            detector: detector.to_string(),
            severity,
            score,
            details,
        }
    }
}

// ── Detector trait ───────────────────────────────────────────────────────────

/// Single-event detector. Implementors keep zero internal mutable state —
/// shared state lives in [`AnomalyState`] and is passed as `&SketchBundle`.
///
/// Detectors must be `Send` so the orchestrator can live in the writer task.
pub trait Detector: Send {
    /// Stable machine name (e.g. `"cost_zscore"`).
    fn name(&self) -> &'static str;

    /// Inspect one event using the pre-update sketch. Returns 0+ alerts.
    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert>;
}

// ── Orchestrator ─────────────────────────────────────────────────────────────

/// Coordinates the detector chain and the sketch state.
///
/// Owned by the writer task. **Not** wrapped in `Arc<Mutex<_>>` — there is
/// exactly one writer task per pool member, each owning its own orchestrator
/// (independent sketches per task is acceptable because state is approximate).
pub struct AnomalyOrchestrator {
    pub state: AnomalyState,
    pub detectors: Vec<Box<dyn Detector>>,
    pub enabled: bool,
}

impl AnomalyOrchestrator {
    /// Build the default orchestrator from config.
    pub fn from_config(cfg: &AnomalyConfig) -> Self {
        let detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(detectors::CostDetector::new(cfg.cost_z_threshold)),
            Box::new(detectors::LatencyDetector::new(cfg.latency_p99_multiplier)),
            Box::new(detectors::ExfilDetector::new()),
            Box::new(detectors::SecurityDetector::new()),
            Box::new(detectors::FanoutDetector::new()),
            Box::new(detectors::ErrorRateDetector::new(cfg.error_rate_multiplier)),
            Box::new(detectors::BehavioralDetector::new()),
            Box::new(detectors::LegacyDetector::new()),
        ];

        Self {
            state: AnomalyState::with_capacity(cfg.state_lru_cap),
            detectors,
            enabled: cfg.enabled,
        }
    }

    /// Construct with empty defaults — useful for tests.
    pub fn for_tests() -> Self {
        Self::from_config(&AnomalyConfig::default())
    }

    /// Run all detectors against one event then update the sketches.
    ///
    /// **Order matters**: detectors see the *pre-update* sketch so a single
    /// spike cannot mask itself. Then `state.ingest` folds the event in.
    ///
    /// Fail-soft: every detector runs inside `catch_unwind`; a panic logs &
    /// continues with the next detector.
    pub fn process(&mut self, event: &AgentEvent) -> Vec<AnomalyAlert> {
        if !self.enabled {
            return Vec::new();
        }

        let mut alerts = Vec::new();
        let key = AgentModelKey::from_event(event);
        // Snapshot a bundle for the detectors. We clone to avoid holding a
        // borrow across the mutation in `ingest` below.
        let snapshot: SketchBundle = self
            .state
            .peek(&key)
            .cloned()
            .unwrap_or_else(SketchBundle::new);

        for d in self.detectors.iter_mut() {
            let name = d.name();
            let res = catch_unwind(AssertUnwindSafe(|| d.check(event, &snapshot)));
            match res {
                Ok(mut v) => alerts.append(&mut v),
                Err(_) => {
                    tracing::error!(
                        detector = name,
                        agent = %event.agent_id,
                        "anomaly detector panicked — skipping (fail-soft)"
                    );
                }
            }
        }

        // Now fold the event into the sketches.
        self.state.ingest(event);

        alerts
    }

    /// Replace the live state with a freshly-seeded one. Called once at startup
    /// after [`state::seed_from_pool`] has populated a builder.
    pub fn replace_state(&mut self, new_state: AnomalyState) {
        self.state = new_state;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{AgentEvent, EventDirection, Provider};

    fn ev(agent: &str) -> AgentEvent {
        AgentEvent::new(
            agent,
            Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "g",
            "audit:none",
        )
    }

    /// Pathological detector that panics on every event — orchestrator must
    /// not crash.
    struct PanicDetector;
    impl Detector for PanicDetector {
        fn name(&self) -> &'static str {
            "panic_test"
        }
        fn check(&mut self, _e: &AgentEvent, _b: &SketchBundle) -> Vec<AnomalyAlert> {
            panic!("intentional");
        }
    }

    #[test]
    fn orchestrator_swallows_detector_panic() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        o.detectors.push(Box::new(PanicDetector));
        let alerts = o.process(&ev("a"));
        // The panic was swallowed — other detectors may or may not fire on a
        // bare event, but the call returned.
        let _ = alerts;
    }

    #[test]
    fn disabled_orchestrator_is_noop() {
        let cfg = AnomalyConfig {
            enabled: false,
            ..AnomalyConfig::default()
        };
        let mut o = AnomalyOrchestrator::from_config(&cfg);
        let alerts = o.process(&ev("a"));
        assert!(alerts.is_empty());
    }

    #[test]
    fn alert_serialises_to_json() {
        let a = AnomalyAlert::new(
            "agent-1",
            None,
            "cost_zscore",
            Severity::Warn,
            4.2,
            serde_json::json!({"z": 4.2}),
        );
        let s = serde_json::to_string(&a).unwrap();
        assert!(s.contains("cost_zscore"));
        assert!(s.contains("warn"));
    }
}
