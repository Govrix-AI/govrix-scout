//! Latency-anomaly detector.
//!
//! Uses the t-digest p99 estimate from the sketch bundle. Fires when the
//! observed latency exceeds `p99 * multiplier`.

use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

const WARMUP_SAMPLES: u64 = 30;

pub struct LatencyDetector {
    pub p99_multiplier: f64,
}

impl LatencyDetector {
    pub fn new(p99_multiplier: f64) -> Self {
        Self { p99_multiplier }
    }
}

impl Detector for LatencyDetector {
    fn name(&self) -> &'static str {
        "latency_p99"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        let lat = match event.latency_ms {
            Some(l) => l as f64,
            None => return Vec::new(),
        };
        if bundle.samples < WARMUP_SAMPLES {
            return Vec::new();
        }
        let p99 = bundle.p99_latency();
        if p99 <= 0.0 {
            return Vec::new();
        }
        let threshold = p99 * self.p99_multiplier;
        if lat <= threshold {
            return Vec::new();
        }
        let severity = if lat > p99 * (self.p99_multiplier * 2.0) {
            Severity::Critical
        } else {
            Severity::Warn
        };
        vec![AnomalyAlert::new(
            &event.agent_id,
            Some(event.session_id),
            self.name(),
            severity,
            lat / p99,
            json!({
                "latency_ms": lat,
                "p99_ms": p99,
                "threshold_ms": threshold,
                "model": event.model,
            }),
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly::AnomalyOrchestrator;
    use govrix_scout_common::config::AnomalyConfig;
    use govrix_scout_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    fn ev(latency_ms: u32) -> AgentEvent {
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
        e.latency_ms = Some(latency_ms);
        e
    }

    #[test]
    fn latency_spike_fires_alert() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..60 {
            o.process(&ev(100));
        }
        let alerts = o.process(&ev(5000));
        assert!(
            alerts.iter().any(|a| a.detector == "latency_p99"),
            "expected latency alert, got: {alerts:?}"
        );
    }

    #[test]
    fn no_alert_for_steady_latency() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..60 {
            o.process(&ev(100));
        }
        let alerts = o.process(&ev(110));
        assert!(alerts.iter().all(|a| a.detector != "latency_p99"));
    }
}
