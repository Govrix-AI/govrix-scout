//! Fan-out / unusual-provider detector.
//!
//! Watches the `tool_cardinality` HLL estimate and the `providers_seen` set.
//! Fires when:
//!
//! 1. Distinct-tool estimate exceeds `fanout_mean + 3·σ` (after warm-up).
//! 2. A new provider name appears that the agent has never used before.

use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

const WARMUP_SAMPLES: u64 = 40;

pub struct FanoutDetector;

impl FanoutDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FanoutDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector for FanoutDetector {
    fn name(&self) -> &'static str {
        "fanout"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        let mut out = Vec::new();

        // 1. Tool fan-out z-score.
        if bundle.samples >= WARMUP_SAMPLES && bundle.fanout_mean > 0.0 {
            let sigma = bundle.fanout_var.sqrt();
            if sigma > 0.0 {
                let est = bundle.tool_cardinality.estimate();
                let z = (est - bundle.fanout_mean) / sigma;
                if z > 3.0 {
                    out.push(AnomalyAlert::new(
                        &event.agent_id,
                        Some(event.session_id),
                        "fanout_unusual_tools",
                        Severity::Warn,
                        z,
                        json!({
                            "tool_cardinality_est": est,
                            "fanout_mean": bundle.fanout_mean,
                            "fanout_sigma": sigma,
                            "z": z,
                        }),
                    ));
                }
            }
        }

        // 2. UnusualProvider: provider not previously seen for this agent
        //    (after the bundle has warmed up enough to have a stable view).
        let provider = event.provider.to_string();
        if bundle.samples >= WARMUP_SAMPLES && !bundle.providers_seen.contains(&provider) {
            out.push(AnomalyAlert::new(
                &event.agent_id,
                Some(event.session_id),
                "unusual_provider",
                Severity::Warn,
                1.0,
                json!({
                    "new_provider": provider,
                    "known_providers": bundle.providers_seen.iter().collect::<Vec<_>>(),
                }),
            ));
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
    use uuid::Uuid;

    fn ev(provider: Provider, tool: Option<&str>) -> AgentEvent {
        let mut e = AgentEvent::new(
            "a",
            Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "x",
            provider,
            "g",
            "audit:none",
        );
        e.tool_name = tool.map(String::from);
        e
    }

    #[test]
    fn new_provider_fires_after_warmup() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..50 {
            o.process(&ev(Provider::OpenAI, Some("search")));
        }
        let alerts = o.process(&ev(Provider::Anthropic, Some("search")));
        assert!(alerts.iter().any(|a| a.detector == "unusual_provider"));
    }

    #[test]
    fn same_provider_silent() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..50 {
            o.process(&ev(Provider::OpenAI, Some("search")));
        }
        let alerts = o.process(&ev(Provider::OpenAI, Some("search")));
        assert!(alerts.iter().all(|a| a.detector != "unusual_provider"));
    }
}
