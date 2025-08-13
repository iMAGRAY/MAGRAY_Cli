// Policy integration for configuration profiles
// This module bridges configuration profiles with the policy engine

use anyhow::{Context, Result};
use domain::config::{MagrayConfig, Profile, ProfileConfig, SecurityConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{debug, info, warn};

/// Policy integration configuration derived from profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyIntegrationConfig {
    /// Default policy mode (ask/allow/deny)
    pub default_mode: String,

    /// Risk tolerance level
    pub risk_level: RiskLevel,

    /// Whether to enable permissive mode
    pub permissive_mode: bool,

    /// Tool permissions configuration
    pub tool_permissions: ToolPermissionsConfig,

    /// Sandbox configuration
    pub sandbox_config: SandboxConfig,

    /// Emergency override settings
    pub emergency_overrides: EmergencyOverrideConfig,
}

/// Risk tolerance levels matching policy engine expectations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low, // Lowest risk (0)
    #[default]
    Medium, // Medium risk (1)
    High, // High risk (2)
    Critical, // Highest risk (3)
}

impl std::str::FromStr for RiskLevel {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            "critical" => RiskLevel::Critical,
            _ => RiskLevel::Medium, // Default fallback
        })
    }
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
}

/// Tool permissions configuration for policy integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionsConfig {
    /// Tool whitelist mode (expanded/minimal/custom)
    pub whitelist_mode: String,

    /// Whether to require signed tools
    pub require_signed_tools: bool,

    /// Whether to enable dry-run by default
    pub dry_run_default: bool,

    /// Maximum allowed tool execution time in seconds
    pub max_execution_time: Option<u64>,

    /// Custom tool restrictions
    pub tool_restrictions: HashMap<String, ToolRestriction>,
}

/// Per-tool restriction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRestriction {
    /// Whether this tool is allowed
    pub allowed: bool,

    /// Risk level required for this tool
    pub required_risk_level: RiskLevel,

    /// Whether user confirmation is required
    pub requires_confirmation: bool,

    /// Custom restrictions
    pub custom_limits: HashMap<String, serde_json::Value>,
}

/// Sandbox configuration for policy integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Whether sandbox is enabled
    pub enabled: bool,

    /// Resource limits
    pub resource_limits: SandboxResourceLimits,

    /// Network access configuration
    pub network_access: NetworkAccessConfig,

    /// File system access configuration
    pub filesystem_access: FilesystemAccessConfig,
}

/// Sandbox resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResourceLimits {
    /// Maximum memory usage in MB
    pub max_memory_mb: Option<u64>,

    /// Maximum CPU time in seconds
    pub max_cpu_time_secs: Option<u64>,

    /// Maximum number of open files
    pub max_open_files: Option<u32>,
}

/// Network access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAccessConfig {
    /// Whether network access is allowed
    pub allowed: bool,

    /// Allowed domains (empty = all allowed if network is allowed)
    pub allowed_domains: Vec<String>,

    /// Blocked domains
    pub blocked_domains: Vec<String>,
}

/// Filesystem access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemAccessConfig {
    /// Whether read access is allowed
    pub read_allowed: bool,

    /// Whether write access is allowed
    pub write_allowed: bool,

    /// Allowed paths
    pub allowed_paths: Vec<String>,

    /// Blocked paths
    pub blocked_paths: Vec<String>,
}

/// Emergency override configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyOverrideConfig {
    /// Whether emergency override is enabled
    pub enabled: bool,

    /// Override timeout in seconds
    pub timeout_secs: u64,

    /// Override reason logging required
    pub require_reason: bool,
}

/// Policy integration engine
#[derive(Debug)]
pub struct PolicyIntegrationEngine {
    /// Current integration configuration
    config: PolicyIntegrationConfig,

    /// Profile-specific overrides
    profile_overrides: HashMap<String, PolicyIntegrationConfig>,
}

impl PolicyIntegrationEngine {
    /// Create new policy integration engine
    pub fn new() -> Self {
        Self {
            config: PolicyIntegrationConfig::default(),
            profile_overrides: HashMap::new(),
        }
    }

    /// Generate policy integration configuration from Magray config
    pub fn generate_policy_config(
        &self,
        magray_config: &MagrayConfig,
    ) -> Result<PolicyIntegrationConfig> {
        let profile = &magray_config.profile;
        let profile_config = magray_config
            .profile_config
            .as_ref()
            .or_else(|| self.get_default_profile_config(profile));

        let security_config = magray_config.effective_security();
        let tools_config = magray_config.effective_tools();

        let policy_config = PolicyIntegrationConfig {
            default_mode: security_config.default_policy_mode.clone(),
            risk_level: RiskLevel::from_str(&security_config.risk_level).unwrap_or_default(),
            permissive_mode: security_config.permissive_mode,

            tool_permissions: ToolPermissionsConfig {
                whitelist_mode: tools_config.whitelist_mode.clone(),
                require_signed_tools: tools_config.require_signed_tools,
                dry_run_default: tools_config.dry_run_default,
                max_execution_time: self.get_max_execution_time(profile),
                tool_restrictions: self.generate_tool_restrictions(profile, security_config)?,
            },

            sandbox_config: SandboxConfig {
                enabled: magray_config.plugins.sandbox_enabled,
                resource_limits: self.generate_resource_limits(profile, profile_config)?,
                network_access: self.generate_network_access(profile, security_config)?,
                filesystem_access: self.generate_filesystem_access(profile, security_config)?,
            },

            emergency_overrides: self.generate_emergency_config(profile, security_config)?,
        };

        info!(
            "Generated policy integration config for profile: {}",
            profile.name()
        );
        debug!(
            "Policy config: permissive_mode={}, risk_level={:?}, sandbox_enabled={}",
            policy_config.permissive_mode,
            policy_config.risk_level,
            policy_config.sandbox_config.enabled
        );

        Ok(policy_config)
    }

    /// Apply profile-specific policy configuration
    pub async fn apply_profile_policy(&mut self, magray_config: &MagrayConfig) -> Result<()> {
        let policy_config = self.generate_policy_config(magray_config)?;

        // Store the configuration for later use
        self.config = policy_config.clone();
        self.profile_overrides
            .insert(magray_config.profile.name().to_string(), policy_config);

        info!(
            "Applied profile policy configuration for: {}",
            magray_config.profile.name()
        );

        Ok(())
    }

    /// Get current policy configuration
    pub fn get_policy_config(&self) -> &PolicyIntegrationConfig {
        &self.config
    }

    /// Get policy configuration for specific profile
    pub fn get_profile_policy_config(
        &self,
        profile_name: &str,
    ) -> Option<&PolicyIntegrationConfig> {
        self.profile_overrides.get(profile_name)
    }

    /// Check if operation is allowed under current policy
    pub fn check_operation_allowed(
        &self,
        operation: &str,
        context: &OperationContext,
    ) -> PolicyDecision {
        let config = &self.config;

        // Check permissive mode
        if config.permissive_mode {
            return PolicyDecision::Allow("Permissive mode enabled".to_string());
        }

        // Check risk level compatibility
        if context.risk_level > config.risk_level {
            return PolicyDecision::Deny(format!(
                "Operation risk level ({:?}) exceeds profile tolerance ({:?})",
                context.risk_level, config.risk_level
            ));
        }

        // Check tool-specific restrictions
        if let Some(tool_name) = &context.tool_name {
            if let Some(restriction) = config.tool_permissions.tool_restrictions.get(tool_name) {
                if !restriction.allowed {
                    return PolicyDecision::Deny(format!("Tool '{tool_name}' is not allowed"));
                }

                if context.risk_level > restriction.required_risk_level {
                    return PolicyDecision::Deny(format!(
                        "Tool '{}' requires max risk level {:?}, got {:?}",
                        tool_name, restriction.required_risk_level, context.risk_level
                    ));
                }

                if restriction.requires_confirmation {
                    return PolicyDecision::Ask(format!(
                        "Tool '{tool_name}' requires user confirmation"
                    ));
                }
            }
        }

        // Default policy based on mode
        match config.default_mode.as_str() {
            "allow" => PolicyDecision::Allow("Default policy allows".to_string()),
            "deny" => PolicyDecision::Deny("Default policy denies".to_string()),
            "ask" => PolicyDecision::Ask("Default policy asks for confirmation".to_string()),
            _ => PolicyDecision::Ask("Default policy asks for confirmation".to_string()),
        }
    }

    // Private helper methods

    fn get_default_profile_config(&self, profile: &Profile) -> Option<&ProfileConfig> {
        // This would be implemented to return default configs
        // For now, return None to use existing logic
        None
    }

    fn get_max_execution_time(&self, profile: &Profile) -> Option<u64> {
        match profile {
            Profile::Dev => Some(300),       // 5 minutes for dev
            Profile::Prod => Some(60),       // 1 minute for prod
            Profile::Custom(_) => Some(180), // 3 minutes for custom
        }
    }

    fn generate_tool_restrictions(
        &self,
        profile: &Profile,
        security_config: &SecurityConfig,
    ) -> Result<HashMap<String, ToolRestriction>> {
        let mut restrictions = HashMap::new();

        // Profile-specific tool restrictions
        match profile {
            Profile::Dev => {
                // Dev profile: more permissive, allow most tools
                restrictions.insert(
                    "shell_exec".to_string(),
                    ToolRestriction {
                        allowed: true,
                        required_risk_level: RiskLevel::Medium,
                        requires_confirmation: true,
                        custom_limits: HashMap::new(),
                    },
                );

                restrictions.insert(
                    "file_write".to_string(),
                    ToolRestriction {
                        allowed: true,
                        required_risk_level: RiskLevel::Low,
                        requires_confirmation: false,
                        custom_limits: HashMap::new(),
                    },
                );
            }
            Profile::Prod => {
                // Prod profile: strict, limited tools
                restrictions.insert(
                    "shell_exec".to_string(),
                    ToolRestriction {
                        allowed: false, // No shell exec in prod
                        required_risk_level: RiskLevel::Critical,
                        requires_confirmation: true,
                        custom_limits: HashMap::new(),
                    },
                );

                restrictions.insert(
                    "file_write".to_string(),
                    ToolRestriction {
                        allowed: true,
                        required_risk_level: RiskLevel::Low,
                        requires_confirmation: true, // Always ask in prod
                        custom_limits: HashMap::new(),
                    },
                );
            }
            Profile::Custom(_) => {
                // Custom profile: balanced approach
                restrictions.insert(
                    "shell_exec".to_string(),
                    ToolRestriction {
                        allowed: true,
                        required_risk_level: RiskLevel::High,
                        requires_confirmation: true,
                        custom_limits: HashMap::new(),
                    },
                );
            }
        }

        Ok(restrictions)
    }

    fn generate_resource_limits(
        &self,
        profile: &Profile,
        profile_config: Option<&ProfileConfig>,
    ) -> Result<SandboxResourceLimits> {
        let memory_limit = profile_config
            .and_then(|c| c.performance.memory_limit_override_mb)
            .map(|mb| mb as u64)
            .or(match profile {
                Profile::Dev => Some(2048),       // 2GB for dev
                Profile::Prod => Some(512),       // 512MB for prod
                Profile::Custom(_) => Some(1024), // 1GB for custom
            });

        let cpu_time_limit = match profile {
            Profile::Dev => Some(600),       // 10 minutes for dev
            Profile::Prod => Some(60),       // 1 minute for prod
            Profile::Custom(_) => Some(300), // 5 minutes for custom
        };

        Ok(SandboxResourceLimits {
            max_memory_mb: memory_limit,
            max_cpu_time_secs: cpu_time_limit,
            max_open_files: Some(1024), // Standard limit
        })
    }

    fn generate_network_access(
        &self,
        profile: &Profile,
        security_config: &SecurityConfig,
    ) -> Result<NetworkAccessConfig> {
        let allowed = match profile {
            Profile::Dev => true,                              // Allow network in dev
            Profile::Prod => !security_config.permissive_mode, // Strict in prod
            Profile::Custom(_) => false,                       // Deny by default for custom
        };

        let allowed_domains = match profile {
            Profile::Dev => vec![], // All domains allowed in dev
            Profile::Prod => vec![
                // Limited domains in prod
                "api.openai.com".to_string(),
                "api.anthropic.com".to_string(),
                "github.com".to_string(),
            ],
            Profile::Custom(_) => vec![],
        };

        Ok(NetworkAccessConfig {
            allowed,
            allowed_domains,
            blocked_domains: vec![
                "malicious-site.com".to_string(), // Example blocked domain
            ],
        })
    }

    fn generate_filesystem_access(
        &self,
        profile: &Profile,
        security_config: &SecurityConfig,
    ) -> Result<FilesystemAccessConfig> {
        let (read_allowed, write_allowed) = match profile {
            Profile::Dev => (true, true), // Full access in dev
            Profile::Prod => (true, !security_config.permissive_mode), // Limited write in prod
            Profile::Custom(_) => (true, false), // Read-only for custom
        };

        let allowed_paths = match profile {
            Profile::Dev => vec![".".to_string()], // Current directory and below
            Profile::Prod => vec![
                "./data".to_string(),
                "./logs".to_string(),
                "./cache".to_string(),
            ],
            Profile::Custom(_) => vec!["./data".to_string()],
        };

        let blocked_paths = vec![
            "/etc".to_string(),
            "/sys".to_string(),
            "/proc".to_string(),
            "C:\\Windows\\System32".to_string(), // Windows system dirs
        ];

        Ok(FilesystemAccessConfig {
            read_allowed,
            write_allowed,
            allowed_paths,
            blocked_paths,
        })
    }

    fn generate_emergency_config(
        &self,
        profile: &Profile,
        security_config: &SecurityConfig,
    ) -> Result<EmergencyOverrideConfig> {
        let enabled = match profile {
            Profile::Dev => true,   // Allow emergency overrides in dev
            Profile::Prod => false, // No emergency overrides in prod
            Profile::Custom(_) => security_config.permissive_mode,
        };

        Ok(EmergencyOverrideConfig {
            enabled,
            timeout_secs: 300, // 5 minute timeout
            require_reason: !security_config.permissive_mode,
        })
    }
}

impl Default for PolicyIntegrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PolicyIntegrationConfig {
    fn default() -> Self {
        Self {
            default_mode: "ask".to_string(),
            risk_level: RiskLevel::Medium,
            permissive_mode: false,

            tool_permissions: ToolPermissionsConfig {
                whitelist_mode: "minimal".to_string(),
                require_signed_tools: true,
                dry_run_default: true,
                max_execution_time: Some(60),
                tool_restrictions: HashMap::new(),
            },

            sandbox_config: SandboxConfig {
                enabled: true,
                resource_limits: SandboxResourceLimits {
                    max_memory_mb: Some(512),
                    max_cpu_time_secs: Some(60),
                    max_open_files: Some(1024),
                },
                network_access: NetworkAccessConfig {
                    allowed: false,
                    allowed_domains: vec![],
                    blocked_domains: vec![],
                },
                filesystem_access: FilesystemAccessConfig {
                    read_allowed: true,
                    write_allowed: false,
                    allowed_paths: vec!["./data".to_string()],
                    blocked_paths: vec![],
                },
            },

            emergency_overrides: EmergencyOverrideConfig {
                enabled: false,
                timeout_secs: 300,
                require_reason: true,
            },
        }
    }
}

/// Operation context for policy decisions
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub operation: String,
    pub tool_name: Option<String>,
    pub risk_level: RiskLevel,
    pub resource_requirements: ResourceRequirements,
    pub user_confirmation: bool,
}

/// Resource requirements for operations
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub memory_mb: Option<u64>,
    pub cpu_time_secs: Option<u64>,
    pub network_required: bool,
    pub filesystem_write: bool,
}

/// Policy decision result
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow(String),
    Deny(String),
    Ask(String),
}

/// Utility functions for policy integration
pub struct PolicyIntegrationUtils;

impl PolicyIntegrationUtils {
    /// Convert profile configuration to policy engine format
    pub fn profile_to_policy_format(
        profile_config: &ProfileConfig,
    ) -> HashMap<String, serde_json::Value> {
        let mut policy_map = HashMap::new();

        policy_map.insert(
            "default_policy_mode".to_string(),
            serde_json::Value::String(profile_config.security.default_policy_mode.clone()),
        );

        policy_map.insert(
            "risk_level".to_string(),
            serde_json::Value::String(profile_config.security.risk_level.clone()),
        );

        policy_map.insert(
            "permissive_mode".to_string(),
            serde_json::Value::Bool(profile_config.security.permissive_mode),
        );

        policy_map.insert(
            "require_signed_tools".to_string(),
            serde_json::Value::Bool(profile_config.tools.require_signed_tools),
        );

        policy_map.insert(
            "dry_run_default".to_string(),
            serde_json::Value::Bool(profile_config.tools.dry_run_default),
        );

        policy_map
    }

    /// Validate policy configuration compatibility
    pub fn validate_policy_compatibility(config: &PolicyIntegrationConfig) -> Result<()> {
        // Check for conflicting settings
        if config.permissive_mode && config.risk_level == RiskLevel::Low {
            warn!("Permissive mode enabled with low risk tolerance - potential security concern");
        }

        if config.sandbox_config.enabled && config.permissive_mode {
            info!("Sandbox enabled in permissive mode - security measures still active");
        }

        if !config.sandbox_config.enabled && config.risk_level == RiskLevel::Low {
            return Err(anyhow::anyhow!(
                "Sandbox disabled with low risk tolerance - incompatible security settings"
            ));
        }

        Ok(())
    }

    /// Generate policy configuration summary for logging
    pub fn generate_policy_summary(config: &PolicyIntegrationConfig) -> String {
        format!(
            "Policy Summary: mode={}, risk={:?}, permissive={}, sandbox={}, tools_require_signed={}",
            config.default_mode,
            config.risk_level,
            config.permissive_mode,
            config.sandbox_config.enabled,
            config.tool_permissions.require_signed_tools
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::config::{MagrayConfig, ProfileConfig};

    #[test]
    fn test_policy_integration_engine_creation() {
        let engine = PolicyIntegrationEngine::new();
        assert_eq!(engine.config.default_mode, "ask");
        assert_eq!(engine.config.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_risk_level_conversion() {
        assert_eq!(RiskLevel::from_str("low"), Ok(RiskLevel::Low));
        assert_eq!(RiskLevel::from_str("HIGH"), Ok(RiskLevel::High));
        assert_eq!(RiskLevel::from_str("invalid"), Ok(RiskLevel::Medium));

        assert_eq!(RiskLevel::Low.as_str(), "low");
        assert_eq!(RiskLevel::Critical.as_str(), "critical");
    }

    #[tokio::test]
    async fn test_dev_profile_policy_generation() {
        let mut engine = PolicyIntegrationEngine::new();

        let mut config = MagrayConfig {
            profile: Profile::Dev,
            profile_config: Some(ProfileConfig::dev()),
            ..Default::default()
        };
        config.apply_profile(&ProfileConfig::dev());

        let result = engine.apply_profile_policy(&config).await;
        assert!(result.is_ok());

        let policy_config = engine.get_policy_config();
        assert!(policy_config.permissive_mode);
        assert_eq!(policy_config.risk_level, RiskLevel::Medium);
        assert!(!policy_config.sandbox_config.enabled); // Disabled in dev (permissive mode)
    }

    #[tokio::test]
    async fn test_prod_profile_policy_generation() {
        let mut engine = PolicyIntegrationEngine::new();

        let mut config = MagrayConfig {
            profile: Profile::Prod,
            profile_config: Some(ProfileConfig::prod()),
            ..Default::default()
        };
        config.apply_profile(&ProfileConfig::prod());

        let result = engine.apply_profile_policy(&config).await;
        assert!(result.is_ok());

        let policy_config = engine.get_policy_config();
        assert!(!policy_config.permissive_mode);
        assert_eq!(policy_config.risk_level, RiskLevel::Low);
        assert!(policy_config.sandbox_config.enabled); // Enabled in prod (strict mode)
        assert!(policy_config.tool_permissions.require_signed_tools);
    }

    #[test]
    fn test_operation_policy_check() {
        let config = PolicyIntegrationConfig {
            permissive_mode: false,
            risk_level: RiskLevel::Medium,
            ..Default::default()
        };

        let engine = PolicyIntegrationEngine {
            config,
            profile_overrides: HashMap::new(),
        };

        let context = OperationContext {
            operation: "file_read".to_string(),
            tool_name: Some("file_reader".to_string()),
            risk_level: RiskLevel::Low,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(100),
                cpu_time_secs: Some(10),
                network_required: false,
                filesystem_write: false,
            },
            user_confirmation: false,
        };

        let decision = engine.check_operation_allowed("test", &context);
        matches!(decision, PolicyDecision::Ask(_));
    }

    #[test]
    fn test_policy_compatibility_validation() {
        let mut config = PolicyIntegrationConfig {
            permissive_mode: false,
            risk_level: RiskLevel::Low,
            ..Default::default()
        };

        // Valid configuration
        config.sandbox_config.enabled = true;

        assert!(PolicyIntegrationUtils::validate_policy_compatibility(&config).is_ok());

        // Invalid configuration
        config.sandbox_config.enabled = false;
        assert!(PolicyIntegrationUtils::validate_policy_compatibility(&config).is_err());
    }

    #[test]
    fn test_profile_to_policy_format_conversion() {
        let profile_config = ProfileConfig::prod();
        let policy_map = PolicyIntegrationUtils::profile_to_policy_format(&profile_config);

        assert_eq!(
            policy_map
                .get("default_policy_mode")
                .expect("Operation failed - converted from unwrap()"),
            &serde_json::Value::String("ask".to_string())
        );
        assert_eq!(
            policy_map
                .get("risk_level")
                .expect("Operation failed - converted from unwrap()"),
            &serde_json::Value::String("low".to_string())
        );
        assert_eq!(
            policy_map
                .get("permissive_mode")
                .expect("Operation failed - converted from unwrap()"),
            &serde_json::Value::Bool(false)
        );
        assert_eq!(
            policy_map
                .get("require_signed_tools")
                .expect("Operation failed - converted from unwrap()"),
            &serde_json::Value::Bool(true)
        );
    }
}
