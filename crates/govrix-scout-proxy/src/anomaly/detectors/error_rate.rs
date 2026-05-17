//! Error-rate detector.
//!
//! Tracks the rolling error-rate window in [`SketchBundle::error_window`]
//! (100-event ring) and compares the **current** window mean against a slowly
//! tracked baseline. Fires when ratio > `multiplier` and absolute rate ≥ 10 %.

use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

const MIN_WINDOW_FILL: usize = 30;
const MIN_ABSOLUTE_RATE: f64 = 0.10;

pub struct ErrorRateDetector {
    pub multiplier: f64,
    baseline: std::collections::HashMap<String, f64>,
}

impl ErrorRateDetector {
    pub fn new(multiplier: f64) -> Self {
        Self {
            multiplier,
            baseline: std::collections::HashMap::new(),
        }
    }
}

impl Detector for ErrorRateDetector {
    fn name(&self) -> &'static str {
        "error_rate"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        if bundle.error_window.len() < MIN_WINDOW_FILL {
            return Vec::new();
        }
        let rate = bundle.error_rate();
        let baseline_val = *self.baseline.entry(event.agent_id.clone()).or_insert(0.0);
        let detector_name = self.name();

        let mut out = Vec::new();
        if rate >= MIN_ABSOLUTE_RATE && baseline_val > 0.0 && rate > baseline_val * self.multiplier
        {
            let severity = if rate > baseline_val * self.multiplier * 2.0 {
                Severity::Critical
            } else {
                Severity::Warn
            };
            out.push(AnomalyAlert::new(
                &event.agent_id,
                Some(event.session_id),
                detector_name,
                severity,
                rate / baseline_val.max(1e-9),
                json!({
                    "window_rate": rate,
                    "baseline": baseline_val,
                    "multiplier": self.multiplier,
                }),
            ));
        }

        // Slow EWMA baseline update.
        let entry = self.baseline.entry(event.agent_id.clone()).or_insert(0.0);
        *entry = *entry * 0.95 + rate * 0.05;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly::AnomalyOrchestrator;
    use govrix_scout_common::config::AnomalyConfig;
    use govrix_scout_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    fn ev(status: u16) -> AgentEvent {
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
        e.status_code = Some(status);
        e
    }

    #[test]
    fn error_burst_fires() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        // Healthy baseline.
        for _ in 0..60 {
            o.process(&ev(200));
        }
        // Inject 90% errors.
        let mut fired = false;
        for _ in 0..60 {
            let alerts = o.process(&ev(500));
            if alerts.iter().any(|a| a.detector == "error_rate") {
                fired = true;
            }
        }
        assert!(fired, "expected an error_rate alert during the burst");
    }

    #[test]
    fn steady_state_silent() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..200 {
            let alerts = o.process(&ev(200));
            assert!(alerts.iter().all(|a| a.detector != "error_rate"));
        }
    }
}
