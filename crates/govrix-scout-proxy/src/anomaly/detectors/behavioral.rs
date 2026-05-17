//! Behavioral detector — off-hours activity + first-time tool use.
//!
//! Composite of the legacy `OffHours` + `NewTool` heuristics, but reads from
//! the new sketch bundle (`hour_histogram` and `tool_cardinality`) rather than
//! its own state.

use chrono::Timelike;
use govrix_scout_common::models::event::AgentEvent;
use serde_json::json;

use crate::anomaly::{AnomalyAlert, Detector, Severity, SketchBundle};

const WARMUP_SAMPLES: u64 = 50;
const OFF_HOURS_MIN_RATIO: f64 = 0.02;

pub struct BehavioralDetector {
    seen_tools: std::collections::HashSet<(String, String)>,
}

impl BehavioralDetector {
    pub fn new() -> Self {
        Self {
            seen_tools: std::collections::HashSet::new(),
        }
    }
}

impl Default for BehavioralDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector for BehavioralDetector {
    fn name(&self) -> &'static str {
        "behavioral"
    }

    fn check(&mut self, event: &AgentEvent, bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        let mut out = Vec::new();

        // Off-hours: this event's hour bucket is < 2 % of total observed activity
        // (after warm-up).
        if bundle.samples >= WARMUP_SAMPLES {
            let total: u64 = bundle.hour_histogram.iter().sum();
            if total > 0 {
                let hour = event.timestamp.hour() as usize;
                let bucket = bundle.hour_histogram[hour] as f64;
                let ratio = bucket / total as f64;
                if ratio < OFF_HOURS_MIN_RATIO {
                    out.push(AnomalyAlert::new(
                        &event.agent_id,
                        Some(event.session_id),
                        "off_hours",
                        Severity::Info,
                        1.0 - ratio,
                        json!({
                            "hour_utc": hour,
                            "bucket_ratio": ratio,
                        }),
                    ));
                }
            }
        }

        // First-time tool: detector keeps a tiny seen-set keyed by (agent, tool).
        if let Some(tool) = &event.tool_name {
            let key = (event.agent_id.clone(), tool.clone());
            if !self.seen_tools.contains(&key) && bundle.samples >= WARMUP_SAMPLES {
                out.push(AnomalyAlert::new(
                    &event.agent_id,
                    Some(event.session_id),
                    "new_tool",
                    Severity::Info,
                    1.0,
                    json!({"tool": tool}),
                ));
            }
            self.seen_tools.insert(key);
            // Cap the set so this detector cannot leak memory.
            if self.seen_tools.len() > 50_000 {
                self.seen_tools.clear();
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
    use uuid::Uuid;

    fn ev(tool: Option<&str>) -> AgentEvent {
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
        e.tool_name = tool.map(String::from);
        e
    }

    #[test]
    fn new_tool_fires_after_warmup() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..60 {
            o.process(&ev(Some("search")));
        }
        let alerts = o.process(&ev(Some("exec_shell")));
        assert!(alerts.iter().any(|a| a.detector == "new_tool"));
    }

    #[test]
    fn repeat_tool_silent() {
        let mut o = AnomalyOrchestrator::from_config(&AnomalyConfig::default());
        for _ in 0..60 {
            o.process(&ev(Some("search")));
        }
        let alerts = o.process(&ev(Some("search")));
        assert!(alerts.iter().all(|a| a.detector != "new_tool"));
    }
}
