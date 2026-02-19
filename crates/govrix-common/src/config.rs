use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    #[serde(default)]
    pub platform: PlatformSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSection {
    #[serde(default)]
    pub policy_enabled: bool,
    #[serde(default)]
    pub pii_masking_enabled: bool,
    #[serde(default)]
    pub a2a_identity_enabled: bool,
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_max_agents() -> u32 {
    100
}
fn default_retention_days() -> u32 {
    30
}

impl Default for PlatformSection {
    fn default() -> Self {
        Self {
            policy_enabled: false,
            pii_masking_enabled: false,
            a2a_identity_enabled: false,
            license_key: None,
            max_agents: default_max_agents(),
            retention_days: default_retention_days(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_platform_section() {
        let s = PlatformSection::default();
        assert!(!s.policy_enabled);
        assert!(!s.pii_masking_enabled);
        assert_eq!(s.max_agents, 100);
        assert_eq!(s.retention_days, 30);
        assert!(s.license_key.is_none());
    }
}
