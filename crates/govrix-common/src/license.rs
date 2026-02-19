use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LicenseTier {
    Community,
    Starter,
    Growth,
    Enterprise,
}

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub tier: LicenseTier,
    pub max_agents: u32,
    pub retention_days: u32,
    pub policy_enabled: bool,
    pub pii_masking_enabled: bool,
    pub compliance_enabled: bool,
    pub a2a_identity_enabled: bool,
}

pub fn validate_license(_key: Option<&str>) -> LicenseInfo {
    LicenseInfo {
        tier: LicenseTier::Community,
        max_agents: 100,
        retention_days: 30,
        policy_enabled: false,
        pii_masking_enabled: false,
        compliance_enabled: false,
        a2a_identity_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_none_returns_community() {
        let info = validate_license(None);
        assert_eq!(info.tier, LicenseTier::Community);
        assert!(!info.policy_enabled);
    }

    #[test]
    fn validate_any_key_returns_community_in_stub() {
        let info = validate_license(Some("test-key-123"));
        assert_eq!(info.tier, LicenseTier::Community);
    }
}
