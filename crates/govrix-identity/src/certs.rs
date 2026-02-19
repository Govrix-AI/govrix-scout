pub fn issue_cert(_agent_id: &str) -> Result<String, String> {
    Ok("stub-certificate".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_issues_cert() {
        let cert = issue_cert("agent-1").unwrap();
        assert!(!cert.is_empty());
    }
}
