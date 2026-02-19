pub fn check_budget(_agent_id: &str, _tokens: u64, _cost_usd: f64) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_always_within_budget() {
        assert!(check_budget("agent-1", 1_000_000, 100.0));
    }
}
