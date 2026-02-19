#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Block { reason: String },
    Alert { message: String },
}

pub fn evaluate(_event: &agentmesh_common::models::AgentEvent) -> PolicyDecision {
    PolicyDecision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmesh_common::models::event::{EventDirection, Provider};
    use uuid::Uuid;

    #[test]
    fn stub_always_allows() {
        let event = agentmesh_common::models::AgentEvent::new(
            "agent-001",
            Uuid::now_v7(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "genesis",
            "audit:none",
        );
        assert_eq!(evaluate(&event), PolicyDecision::Allow);
    }
}
