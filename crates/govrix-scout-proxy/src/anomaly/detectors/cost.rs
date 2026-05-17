//! Cost-anomaly detector.
//!
//! Fires `cost_zscore` when the new event's cost deviates from the per-(agent,
//! model) EWMA mean by `|z| > threshold` standard deviations. A second check
//! against the *slow* mean covers sustained "RapidCostEscalation" drift.

use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

/// Minimum samples before z-score is meaningful.
const WARMUP_SAMPLES: u64 = 20;

pub struct CostDetector {
    pub z_threshold: f64,
}

impl CostDetector {
    pub fn new(z_threshold: f64) -> Self {
        Self { z_threshold }
    }
}

impl Detector for CostDetector {
    fn name(&self) -> &'static str {
        "cost_zscore"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        let cost = event
            .cost_usd
            .and_then(|d| rust_decimal::prelude::ToPrimitive::to_f64(&d))
            .unwrap_or(0.0);
        if cost <= 0.0 || bundle.samples < WARMUP_SAMPLES {
            return Vec::new();
        }

        let stddev = bundle.cost_stddev();
        if stddev <= 0.0 {
            return Vec::new();
        }

        let z = (cost - bundle.cost_mean) / stddev;
        let mut out = Vec::new();
        if z.abs() > self.z_threshold {
            let severity = if z.abs() > self.z_threshold * 2.0 {
                Severity::Critical
            } else {
                Severity::Warn
            };
            out.push(AnomalyAlert::new(
                &event.agent_id,
                Some(event.session_id),
                self.name(),
                severity,
                z,
                json!({
                    "cost_usd": cost,
                    "ewma_mean": bundle.cost_mean,
                    "ewma_stddev": stddev,
                    "z_score": z,
                    "model": event.model,
                }),
            ));
        }

        // RapidCostEscalation: sustained drift vs the slow mean.
        if bundle.cost_mean_slow > 0.0 {
            let ratio = bundle.cost_mean / bundle.cost_mean_slow;
            if ratio > 2.5 {
                out.push(AnomalyAlert::new(
                    &event.agent_id,
                    Some(event.session_id),
                    "rapid_cost_escalation",
                    Severity::Critical,
                    ratio,
                    json!({
                        "fast_mean": bundle.cost_mean,
                        "slow_mean": bundle.cost_mean_slow,
                        "ratio": ratio,
                    }),
                ));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly::AnomalyOrchestrator;
    use govrix_scout_common::config::AnomalyConfig;
    use govrix_scout_common::models::event::{EventDirection, Provider};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn ev(cost: f64) -> AgentEvent {
        let mut e = AgentEvent::new(
            "a",
            Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "x",
            Provider::OpenAI,
            "g",
            "audit:none",
        );
        e.model = Some("gpt-4".into());
        e.cost_usd = Decimal::from_f64_retain(cost);
        e
    }

    #[test]
    fn cost_spike_fires_alert() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        // Warm up with steady $0.01 costs.
        for _ in 0..50 {
            o.process(&ev(0.01));
        }
        let alerts = o.process(&ev(100.0));
        assert!(
            alerts.iter().any(|a| a.detector == "cost_zscore"),
            "expected cost_zscore alert, got: {alerts:?}"
        );
    }

    #[test]
    fn no_alert_during_warmup() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..5 {
            let alerts = o.process(&ev(0.01));
            assert!(alerts.iter().all(|a| a.detector != "cost_zscore"));
        }
    }
}
