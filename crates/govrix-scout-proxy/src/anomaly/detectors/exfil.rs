//! Exfiltration detector — sudden spike in token volume OR response size.
//!
//! Extends the legacy TokenVolume EMA by also tracking `raw_size_bytes`
//! against an EMA. Either signal can fire.

use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

const WARMUP_SAMPLES: u64 = 15;
const TOKEN_SPIKE: f64 = 5.0;
const SIZE_SPIKE: f64 = 8.0;

pub struct ExfilDetector {
    size_ema: std::collections::HashMap<String, f64>,
}

impl ExfilDetector {
    pub fn new() -> Self {
        Self {
            size_ema: std::collections::HashMap::new(),
        }
    }
}

impl Default for ExfilDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector for ExfilDetector {
    fn name(&self) -> &'static str {
        "exfil_volume"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        let mut out = Vec::new();
        let tokens =
            event.input_tokens.unwrap_or(0) as f64 + event.output_tokens.unwrap_or(0) as f64;

        if bundle.samples >= WARMUP_SAMPLES
            && bundle.token_ema > 0.0
            && tokens > TOKEN_SPIKE * bundle.token_ema
        {
            out.push(AnomalyAlert::new(
                &event.agent_id,
                Some(event.session_id),
                "exfil_token_spike",
                Severity::Warn,
                tokens / bundle.token_ema,
                json!({
                    "tokens": tokens,
                    "ema": bundle.token_ema,
                    "ratio": tokens / bundle.token_ema,
                }),
            ));
        }

        // Response-size EMA — local to this detector, keyed by agent only.
        if let Some(sz) = event.raw_size_bytes {
            let sz = sz as f64;
            let entry = self.size_ema.entry(event.agent_id.clone()).or_insert(sz);
            if *entry > 0.0 && sz > SIZE_SPIKE * *entry && bundle.samples >= WARMUP_SAMPLES {
                out.push(AnomalyAlert::new(
                    &event.agent_id,
                    Some(event.session_id),
                    "exfil_size_spike",
                    Severity::Warn,
                    sz / *entry,
                    json!({
                        "raw_size_bytes": sz,
                        "ema": *entry,
                        "ratio": sz / *entry,
                    }),
                ));
            }
            *entry = *entry * 0.9 + sz * 0.1;
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

    fn ev(tokens: i32, size: i64) -> AgentEvent {
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
        e.input_tokens = Some(tokens / 2);
        e.output_tokens = Some(tokens / 2);
        e.raw_size_bytes = Some(size);
        e
    }

    #[test]
    fn token_spike_fires() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..30 {
            o.process(&ev(200, 500));
        }
        let alerts = o.process(&ev(50_000, 500));
        assert!(alerts.iter().any(|a| a.detector == "exfil_token_spike"));
    }

    #[test]
    fn size_spike_fires() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..30 {
            o.process(&ev(200, 1_000));
        }
        let alerts = o.process(&ev(200, 1_000_000));
        assert!(alerts.iter().any(|a| a.detector == "exfil_size_spike"));
    }
}
