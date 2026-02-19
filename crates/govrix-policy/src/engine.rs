use agentmesh_common::models::AgentEvent;
use serde::{Deserialize, Serialize};

/// The outcome of evaluating an event against the policy engine.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Block { reason: String },
    Alert { message: String },
}

/// The action a rule takes when its condition matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Allow,
    Block,
    Alert,
}

/// The condition that a rule checks against an event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyCondition {
    /// Block / alert when the event model is in the provided list.
    ModelBlocked { models: Vec<String> },
    /// Block / alert when total_tokens exceeds the given threshold.
    MaxTokens { limit: u64 },
    /// Block / alert when cost_usd exceeds the given threshold.
    MaxCostUsd { limit_usd: f64 },
    /// Block / alert when the agent_id is in the provided list.
    AgentBlocked { agents: Vec<String> },
}

/// A single named policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub description: Option<String>,
    pub condition: PolicyCondition,
    pub action: RuleAction,
}

impl PolicyRule {
    /// Returns `true` when this rule's condition matches the event.
    fn matches(&self, event: &AgentEvent) -> bool {
        match &self.condition {
            PolicyCondition::ModelBlocked { models } => event
                .model
                .as_deref()
                .map(|m| models.iter().any(|blocked| blocked == m))
                .unwrap_or(false),

            PolicyCondition::MaxTokens { limit } => event
                .total_tokens
                .map(|t| t as u64 > *limit)
                .unwrap_or(false),

            PolicyCondition::MaxCostUsd { limit_usd } => event
                .cost_usd
                .map(|c| {
                    // rust_decimal::Decimal → f64 for comparison
                    let cost_f64: f64 = c.try_into().unwrap_or(f64::MAX);
                    cost_f64 > *limit_usd
                })
                .unwrap_or(false),

            PolicyCondition::AgentBlocked { agents } => {
                agents.iter().any(|blocked| blocked == &event.agent_id)
            }
        }
    }
}

/// YAML schema for loading a list of rules.
#[derive(Debug, Deserialize)]
struct RulesFile {
    rules: Vec<PolicyRule>,
}

/// The policy engine that evaluates events against an ordered list of rules.
///
/// Rules are evaluated in order; the first matching Block or Alert terminates
/// evaluation.  If no rule matches (or all matching rules are Allow), the
/// engine returns `PolicyDecision::Allow`.
#[derive(Debug, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    /// Create an empty engine (no rules — every event is allowed).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a rule to the engine.
    pub fn add_rule(&mut self, rule: PolicyRule) {
        self.rules.push(rule);
    }

    /// Parse rules from a YAML string and add them to the engine.
    ///
    /// Expected format:
    /// ```yaml
    /// rules:
    ///   - name: "block-gpt4"
    ///     description: "Disallow GPT-4"
    ///     condition:
    ///       type: model_blocked
    ///       models: ["gpt-4"]
    ///     action: block
    /// ```
    pub fn load_from_yaml(&mut self, yaml_str: &str) -> Result<(), serde_yaml::Error> {
        let file: RulesFile = serde_yaml::from_str(yaml_str)?;
        self.rules.extend(file.rules);
        Ok(())
    }

    /// Evaluate the event against all rules in insertion order.
    ///
    /// First matching Block → `PolicyDecision::Block`.
    /// First matching Alert → `PolicyDecision::Alert`.
    /// No match (or only Allow matches) → `PolicyDecision::Allow`.
    pub fn evaluate(&self, event: &AgentEvent) -> PolicyDecision {
        for rule in &self.rules {
            if rule.matches(event) {
                match rule.action {
                    RuleAction::Block => {
                        return PolicyDecision::Block {
                            reason: format!("rule '{}' blocked this event", rule.name),
                        };
                    }
                    RuleAction::Alert => {
                        return PolicyDecision::Alert {
                            message: format!("rule '{}' triggered an alert", rule.name),
                        };
                    }
                    RuleAction::Allow => {
                        // explicit Allow — continue to next rule
                    }
                }
            }
        }
        PolicyDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmesh_common::models::event::{EventDirection, Provider};
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn base_event() -> AgentEvent {
        AgentEvent::new(
            "agent-001",
            Uuid::now_v7(),
            EventDirection::Outbound,
            "POST",
            "https://api.openai.com/v1/chat/completions",
            Provider::OpenAI,
            "genesis",
            "audit:none",
        )
    }

    // ── 1. Empty engine always allows ──────────────────────────────────────────

    #[test]
    fn empty_engine_allows_everything() {
        let engine = PolicyEngine::new();
        let event = base_event();
        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // ── 2. Model blocked ───────────────────────────────────────────────────────

    #[test]
    fn model_blocked_returns_block() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "no-gpt4".to_string(),
            description: Some("Disallow GPT-4".to_string()),
            condition: PolicyCondition::ModelBlocked {
                models: vec!["gpt-4".to_string()],
            },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.model = Some("gpt-4".to_string());

        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Block { .. }
        ));
    }

    #[test]
    fn model_not_in_blocklist_allows() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "no-gpt4".to_string(),
            description: None,
            condition: PolicyCondition::ModelBlocked {
                models: vec!["gpt-4".to_string()],
            },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.model = Some("gpt-3.5-turbo".to_string());

        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // ── 3. Max tokens exceeded ─────────────────────────────────────────────────

    #[test]
    fn max_tokens_exceeded_blocks() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "token-limit".to_string(),
            description: None,
            condition: PolicyCondition::MaxTokens { limit: 1_000 },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.total_tokens = Some(2_000);

        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Block { .. }
        ));
    }

    #[test]
    fn max_tokens_within_limit_allows() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "token-limit".to_string(),
            description: None,
            condition: PolicyCondition::MaxTokens { limit: 5_000 },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.total_tokens = Some(500);

        assert_eq!(engine.evaluate(&event), PolicyDecision::Allow);
    }

    // ── 4. Agent blocked ───────────────────────────────────────────────────────

    #[test]
    fn agent_blocked_returns_block() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "ban-rogue".to_string(),
            description: None,
            condition: PolicyCondition::AgentBlocked {
                agents: vec!["rogue-agent".to_string()],
            },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.agent_id = "rogue-agent".to_string();

        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Block { .. }
        ));
    }

    // ── 5. Multiple rules — first-match semantics ──────────────────────────────

    #[test]
    fn first_matching_rule_wins() {
        let mut engine = PolicyEngine::new();

        // Rule 1: alert on gpt-4
        engine.add_rule(PolicyRule {
            name: "alert-gpt4".to_string(),
            description: None,
            condition: PolicyCondition::ModelBlocked {
                models: vec!["gpt-4".to_string()],
            },
            action: RuleAction::Alert,
        });

        // Rule 2: block anything (would fire for same event)
        engine.add_rule(PolicyRule {
            name: "block-all".to_string(),
            description: None,
            condition: PolicyCondition::ModelBlocked {
                models: vec!["gpt-4".to_string()],
            },
            action: RuleAction::Block,
        });

        let mut event = base_event();
        event.model = Some("gpt-4".to_string());

        // Rule 1 fires first → Alert, not Block
        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Alert { .. }
        ));
    }

    // ── 6. Load from YAML ──────────────────────────────────────────────────────

    #[test]
    fn load_from_yaml_and_evaluate() {
        let yaml = r#"
rules:
  - name: "block-gpt4"
    description: "No GPT-4 allowed"
    condition:
      type: model_blocked
      models:
        - "gpt-4"
    action: block
  - name: "token-alert"
    condition:
      type: max_tokens
      limit: 1000
    action: alert
"#;
        let mut engine = PolicyEngine::new();
        engine.load_from_yaml(yaml).expect("yaml parse failed");

        let mut event = base_event();
        event.model = Some("gpt-4".to_string());
        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Block { .. }
        ));

        let mut event2 = base_event();
        event2.total_tokens = Some(5_000);
        assert!(matches!(
            engine.evaluate(&event2),
            PolicyDecision::Alert { .. }
        ));
    }

    // ── 7. Max cost exceeded → Alert ──────────────────────────────────────────

    #[test]
    fn max_cost_exceeded_alerts() {
        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule {
            name: "cost-guard".to_string(),
            description: None,
            condition: PolicyCondition::MaxCostUsd { limit_usd: 1.0 },
            action: RuleAction::Alert,
        });

        let mut event = base_event();
        event.cost_usd = Some(Decimal::try_from(2.5_f64).unwrap());

        assert!(matches!(
            engine.evaluate(&event),
            PolicyDecision::Alert { .. }
        ));
    }
}
