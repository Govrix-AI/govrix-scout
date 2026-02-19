use std::collections::HashMap;
use std::sync::Mutex;

/// Budget limit configuration for an agent or global scope.
#[derive(Debug, Clone)]
pub struct BudgetLimit {
    pub max_tokens: Option<u64>,
    pub max_cost_usd: Option<f64>,
}

/// Tracks cumulative usage per agent and globally.
pub struct BudgetTracker {
    limits: HashMap<String, BudgetLimit>,
    global_limit: Option<BudgetLimit>,
    usage: Mutex<HashMap<String, Usage>>,
    global_usage: Mutex<Usage>,
}

#[derive(Debug, Clone, Default)]
struct Usage {
    tokens: u64,
    cost_usd: f64,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self {
            limits: HashMap::new(),
            global_limit: None,
            usage: Mutex::new(HashMap::new()),
            global_usage: Mutex::new(Usage::default()),
        }
    }

    pub fn set_global_limit(&mut self, limit: BudgetLimit) {
        self.global_limit = Some(limit);
    }

    pub fn set_agent_limit(&mut self, agent_id: String, limit: BudgetLimit) {
        self.limits.insert(agent_id, limit);
    }

    /// Record usage and check if within budget. Returns `BudgetResult::WithinBudget`
    /// when the request is allowed, or `BudgetResult::Exceeded` with a reason when
    /// a limit would be breached.
    ///
    /// Global usage is always recorded first; agent-level usage is only recorded when
    /// the global check passes.
    pub fn check_and_record(&self, agent_id: &str, tokens: u64, cost_usd: f64) -> BudgetResult {
        // --- global limit check + record -----------------------------------
        {
            let mut global = self.global_usage.lock().unwrap();
            if let Some(ref limit) = self.global_limit {
                if let Some(max_tokens) = limit.max_tokens {
                    if global.tokens + tokens > max_tokens {
                        return BudgetResult::Exceeded {
                            reason: format!(
                                "global token budget exceeded: {} + {} > {}",
                                global.tokens, tokens, max_tokens
                            ),
                        };
                    }
                }
                if let Some(max_cost) = limit.max_cost_usd {
                    if global.cost_usd + cost_usd > max_cost {
                        return BudgetResult::Exceeded {
                            reason: format!(
                                "global cost budget exceeded: {:.4} + {:.4} > {:.4}",
                                global.cost_usd, cost_usd, max_cost
                            ),
                        };
                    }
                }
            }
            global.tokens += tokens;
            global.cost_usd += cost_usd;
        }

        // --- agent-specific limit check + record ---------------------------
        {
            let mut usage_map = self.usage.lock().unwrap();
            let agent_usage = usage_map.entry(agent_id.to_string()).or_default();

            if let Some(limit) = self.limits.get(agent_id) {
                if let Some(max_tokens) = limit.max_tokens {
                    if agent_usage.tokens + tokens > max_tokens {
                        return BudgetResult::Exceeded {
                            reason: format!("agent {} token budget exceeded", agent_id),
                        };
                    }
                }
                if let Some(max_cost) = limit.max_cost_usd {
                    if agent_usage.cost_usd + cost_usd > max_cost {
                        return BudgetResult::Exceeded {
                            reason: format!("agent {} cost budget exceeded", agent_id),
                        };
                    }
                }
            }
            agent_usage.tokens += tokens;
            agent_usage.cost_usd += cost_usd;
        }

        BudgetResult::WithinBudget
    }

    /// Get current cumulative usage for a specific agent.
    /// Returns `(tokens, cost_usd)`.
    pub fn get_agent_usage(&self, agent_id: &str) -> (u64, f64) {
        let usage_map = self.usage.lock().unwrap();
        usage_map
            .get(agent_id)
            .map(|u| (u.tokens, u.cost_usd))
            .unwrap_or((0, 0.0))
    }

    /// Reset all tracked usage (e.g. at the start of a new billing period).
    pub fn reset(&self) {
        self.usage.lock().unwrap().clear();
        let mut global = self.global_usage.lock().unwrap();
        *global = Usage::default();
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BudgetResult {
    WithinBudget,
    Exceeded { reason: String },
}

// ── Backwards-compatible helper ─────────────────────────────────────────────

/// Configuration for daily budget enforcement.
///
/// Kept for backwards compatibility. Prefer [`BudgetTracker`] for stateful
/// tracking with per-agent limits.
#[derive(Debug, Clone, Default)]
pub struct BudgetConfig {
    /// Maximum tokens allowed per day. `None` means no limit.
    pub daily_token_limit: Option<u64>,
    /// Maximum cost in USD allowed per day. `None` means no limit.
    pub daily_cost_limit_usd: Option<f64>,
}

/// Returns `true` when the current usage is within the configured budgets.
///
/// This is a stateless helper: it receives the *already-accumulated* totals and
/// checks them against the configured limits. It does **not** record anything.
/// For stateful tracking, use [`BudgetTracker`].
pub fn check_budget(config: &BudgetConfig, current_tokens: u64, current_cost: f64) -> bool {
    let limit = BudgetLimit {
        max_tokens: config.daily_token_limit,
        max_cost_usd: config.daily_cost_limit_usd,
    };

    // Build a one-shot tracker that already holds the previous usage so that
    // adding a zero-cost, zero-token event reflects the accumulated totals.
    let mut tracker = BudgetTracker::new();
    tracker.set_global_limit(limit);

    // Pre-load accumulated usage directly (bypass limit check for the seed).
    {
        let mut global = tracker.global_usage.lock().unwrap();
        global.tokens = current_tokens;
        global.cost_usd = current_cost;
    }

    // Now check whether another zero-unit call would still be within budget.
    // Because we've already stored the values, a 0-token/0-cost probe will only
    // trigger an exceeded result when the pre-loaded totals already breach the
    // limit (i.e. current > max after the seed, which means current > max).
    //
    // Simpler: just replicate the original stateless logic using BudgetResult.
    let result = {
        let global = tracker.global_usage.lock().unwrap();
        if let Some(max_tokens) = config.daily_token_limit {
            if global.tokens > max_tokens {
                return false;
            }
        }
        if let Some(max_cost) = config.daily_cost_limit_usd {
            if global.cost_usd > max_cost {
                return false;
            }
        }
        BudgetResult::WithinBudget
    };

    result == BudgetResult::WithinBudget
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BudgetTracker tests ──────────────────────────────────────────────────

    /// No limits set → every call is allowed regardless of usage.
    #[test]
    fn tracker_no_limits_always_within_budget() {
        let tracker = BudgetTracker::new();
        assert_eq!(
            tracker.check_and_record("agent-a", 1_000_000, 9_999.99),
            BudgetResult::WithinBudget
        );
    }

    /// Global token limit: first call fits, second call exceeds.
    #[test]
    fn tracker_global_token_limit_exceeded() {
        let mut tracker = BudgetTracker::new();
        tracker.set_global_limit(BudgetLimit {
            max_tokens: Some(1_000),
            max_cost_usd: None,
        });

        assert_eq!(
            tracker.check_and_record("agent-a", 500, 0.0),
            BudgetResult::WithinBudget
        );
        let result = tracker.check_and_record("agent-a", 600, 0.0);
        assert!(
            matches!(result, BudgetResult::Exceeded { .. }),
            "expected Exceeded, got {result:?}"
        );
    }

    /// Global cost limit: first call fits, second call exceeds.
    #[test]
    fn tracker_global_cost_limit_exceeded() {
        let mut tracker = BudgetTracker::new();
        tracker.set_global_limit(BudgetLimit {
            max_tokens: None,
            max_cost_usd: Some(1.0),
        });

        assert_eq!(
            tracker.check_and_record("agent-a", 0, 0.50),
            BudgetResult::WithinBudget
        );
        let result = tracker.check_and_record("agent-a", 0, 0.60);
        assert!(
            matches!(result, BudgetResult::Exceeded { .. }),
            "expected Exceeded, got {result:?}"
        );
    }

    /// Agent-specific token limit exceeded while global is fine.
    #[test]
    fn tracker_agent_token_limit_exceeded() {
        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            "agent-x".to_string(),
            BudgetLimit {
                max_tokens: Some(200),
                max_cost_usd: None,
            },
        );

        assert_eq!(
            tracker.check_and_record("agent-x", 100, 0.0),
            BudgetResult::WithinBudget
        );
        let result = tracker.check_and_record("agent-x", 150, 0.0);
        assert!(
            matches!(result, BudgetResult::Exceeded { .. }),
            "expected Exceeded, got {result:?}"
        );
    }

    /// Agent-specific cost limit exceeded while global is fine.
    #[test]
    fn tracker_agent_cost_limit_exceeded() {
        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            "agent-y".to_string(),
            BudgetLimit {
                max_tokens: None,
                max_cost_usd: Some(0.50),
            },
        );

        assert_eq!(
            tracker.check_and_record("agent-y", 0, 0.25),
            BudgetResult::WithinBudget
        );
        let result = tracker.check_and_record("agent-y", 0, 0.30);
        assert!(
            matches!(result, BudgetResult::Exceeded { .. }),
            "expected Exceeded, got {result:?}"
        );
    }

    /// Calls within budget accumulate usage correctly.
    #[test]
    fn tracker_records_usage_correctly() {
        let tracker = BudgetTracker::new();
        tracker.check_and_record("agent-a", 100, 0.10);
        tracker.check_and_record("agent-a", 200, 0.20);

        let (tokens, cost) = tracker.get_agent_usage("agent-a");
        assert_eq!(tokens, 300);
        assert!((cost - 0.30).abs() < 1e-9, "cost mismatch: {cost}");
    }

    /// Reset clears all accumulated usage.
    #[test]
    fn tracker_reset_clears_usage() {
        let tracker = BudgetTracker::new();
        tracker.check_and_record("agent-a", 500, 1.0);
        tracker.reset();

        let (tokens, cost) = tracker.get_agent_usage("agent-a");
        assert_eq!(tokens, 0);
        assert_eq!(cost, 0.0);
    }

    /// Multiple agents are tracked independently.
    #[test]
    fn tracker_multiple_agents_independent() {
        let tracker = BudgetTracker::new();
        tracker.check_and_record("agent-1", 100, 0.10);
        tracker.check_and_record("agent-2", 999, 9.99);

        let (t1, c1) = tracker.get_agent_usage("agent-1");
        let (t2, c2) = tracker.get_agent_usage("agent-2");

        assert_eq!(t1, 100);
        assert!((c1 - 0.10).abs() < 1e-9);
        assert_eq!(t2, 999);
        assert!((c2 - 9.99).abs() < 1e-9);
    }

    /// Agent with no configured limit is never blocked, even when another agent
    /// has a limit set.
    #[test]
    fn tracker_unlimited_agent_never_blocked() {
        let mut tracker = BudgetTracker::new();
        tracker.set_agent_limit(
            "limited".to_string(),
            BudgetLimit {
                max_tokens: Some(50),
                max_cost_usd: None,
            },
        );

        // "unlimited" has no per-agent limit — large request must pass.
        assert_eq!(
            tracker.check_and_record("unlimited", 1_000_000, 0.0),
            BudgetResult::WithinBudget
        );
    }

    // ── check_budget (backwards-compat) tests ───────────────────────────────

    fn cfg(token_limit: Option<u64>, cost_limit: Option<f64>) -> BudgetConfig {
        BudgetConfig {
            daily_token_limit: token_limit,
            daily_cost_limit_usd: cost_limit,
        }
    }

    #[test]
    fn no_limits_always_within_budget() {
        let c = cfg(None, None);
        assert!(check_budget(&c, 1_000_000, 9_999.99));
    }

    #[test]
    fn within_token_limit() {
        let c = cfg(Some(10_000), None);
        assert!(check_budget(&c, 5_000, 0.0));
    }

    #[test]
    fn at_token_limit_not_exceeded() {
        let c = cfg(Some(10_000), None);
        assert!(check_budget(&c, 10_000, 0.0));
    }

    #[test]
    fn token_limit_exceeded() {
        let c = cfg(Some(10_000), None);
        assert!(!check_budget(&c, 10_001, 0.0));
    }

    #[test]
    fn cost_limit_exceeded() {
        let c = cfg(None, Some(5.0));
        assert!(!check_budget(&c, 0, 5.01));
    }

    #[test]
    fn within_cost_limit() {
        let c = cfg(None, Some(100.0));
        assert!(check_budget(&c, 0, 50.0));
    }

    #[test]
    fn tokens_ok_cost_exceeded() {
        let c = cfg(Some(100_000), Some(10.0));
        assert!(!check_budget(&c, 1_000, 10.01));
    }

    #[test]
    fn cost_ok_tokens_exceeded() {
        let c = cfg(Some(1_000), Some(100.0));
        assert!(!check_budget(&c, 1_001, 0.5));
    }
}
