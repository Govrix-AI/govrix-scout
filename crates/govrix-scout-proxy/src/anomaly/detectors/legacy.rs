//! Legacy detector aggregator.
//!
//! The previous `anomaly.rs` module shipped three detectors (`OffHours`,
//! `TokenVolume`, `NewTool`). Their behaviour is now covered by the new
//! detector set (`BehavioralDetector` + `ExfilDetector`). This module
//! survives as a stable name so external configs / dashboards that referenced
//! `"legacy"` still resolve; it currently emits no alerts of its own.

use govrix_scout_common::models::event::AgentEvent;

use crate::anomaly::{AnomalyAlert, Detector, SketchBundle};

pub struct LegacyDetector;

impl LegacyDetector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LegacyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector for LegacyDetector {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn check(&mut self, _event: &AgentEvent, _bundle: &SketchBundle) -> Vec<AnomalyAlert> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_scout_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    #[test]
    fn legacy_is_silent() {
        let mut d = LegacyDetector::new();
        let e = AgentEvent::new(
            "a",
            Uuid::nil(),
            EventDirection::Outbound,
            "POST",
            "x",
            Provider::OpenAI,
            "g",
            "audit:none",
        );
        assert!(d.check(&e, &SketchBundle::new()).is_empty());
    }
}
