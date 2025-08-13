// Capability Checker Implementation
// P1.2.3: Permission checking logic for Tools Platform 2.0

use super::{
    AccessMode, Capability, CapabilityChecker, CapabilityError, CapabilitySpec, NetworkMode,
};
use std::collections::HashSet;
use std::path::PathBuf;
use tracing::{debug, error, warn};

/// Default capability checker with policy-based validation
#[derive(Debug, Clone)]
pub struct DefaultCapabilityChecker {
    /// Currently granted capabilities
    granted_capabilities: HashSet<Capability>,
    /// Policy settings for capability checking
    policy: CapabilityPolicy,
}

/// Capability policy configuration
#[derive(Debug, Clone)]
pub struct CapabilityPolicy {
    /// Maximum allowed risk level (0-10)
    pub max_risk_level: u8,
    /// Allowed filesystem root paths
    pub allowed_fs_roots: Vec<PathBuf>,
    /// Allowed network domains
    pub allowed_domains: Vec<String>,
    /// Allowed shell commands
    pub allowed_commands: Vec<String>,
    /// Maximum memory limit (MB)
    pub max_memory_mb: u64,
    /// Maximum CPU percentage
    pub max_cpu_percent: u8,
    /// Allow elevated shell commands
    pub allow_elevated_shell: bool,
}

impl Default for CapabilityPolicy {
    fn default() -> Self {
        Self {
            max_risk_level: 7, // Moderate security by default
            allowed_fs_roots: vec![
                PathBuf::from("."),
                PathBuf::from("/tmp"),
                std::env::temp_dir(),
            ],
            allowed_domains: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            allowed_commands: vec![
                "ls".to_string(),
                "pwd".to_string(),
                "cat".to_string(),
                "echo".to_string(),
            ],
            max_memory_mb: 512,
            max_cpu_percent: 50,
            allow_elevated_shell: false,
        }
    }
}

impl CapabilityPolicy {
    /// Create a permissive policy for development
    pub fn permissive() -> Self {
        Self {
            max_risk_level: 9,
            allowed_fs_roots: vec![PathBuf::from("/")],
            allowed_domains: vec!["*".to_string()],
            allowed_commands: vec!["*".to_string()],
            max_memory_mb: 4096,
            max_cpu_percent: 90,
            allow_elevated_shell: true,
        }
    }

    /// Create a strict policy for production
    pub fn strict() -> Self {
        Self {
            max_risk_level: 5,
            allowed_fs_roots: vec![PathBuf::from("./sandbox")],
            allowed_domains: vec![],
            allowed_commands: vec!["echo".to_string()],
            max_memory_mb: 128,
            max_cpu_percent: 25,
            allow_elevated_shell: false,
        }
    }
}

impl Default for DefaultCapabilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultCapabilityChecker {
    /// Create new checker with default policy
    pub fn new() -> Self {
        Self {
            granted_capabilities: HashSet::new(),
            policy: CapabilityPolicy::default(),
        }
    }

    /// Create checker with custom policy
    pub fn with_policy(policy: CapabilityPolicy) -> Self {
        Self {
            granted_capabilities: HashSet::new(),
            policy,
        }
    }

    /// Grant a specific capability
    pub fn grant_capability(&mut self, capability: Capability) -> Result<(), CapabilityError> {
        if self.validate_capability(&capability)? {
            debug!("Granting capability: {:?}", capability);
            self.granted_capabilities.insert(capability);
            Ok(())
        } else {
            Err(CapabilityError::Denied { capability })
        }
    }

    /// Revoke a capability
    pub fn revoke_capability(&mut self, capability: &Capability) {
        if self.granted_capabilities.remove(capability) {
            debug!("Revoked capability: {:?}", capability);
        }
    }

    /// Validate capability against policy
    fn validate_capability(&self, capability: &Capability) -> Result<bool, CapabilityError> {
        // Check risk level
        if capability.risk_level() > self.policy.max_risk_level {
            warn!(
                "Capability risk level {} exceeds maximum {}",
                capability.risk_level(),
                self.policy.max_risk_level
            );
            return Err(CapabilityError::PolicyViolation {
                details: format!(
                    "Capability risk level {} exceeds maximum allowed {}",
                    capability.risk_level(),
                    self.policy.max_risk_level
                ),
            });
        }

        match capability {
            Capability::Filesystem { mode, paths } => {
                self.validate_filesystem_capability(mode, paths)
            }
            Capability::Network { mode, domains } => {
                self.validate_network_capability(mode, domains)
            }
            Capability::Shell { commands, elevated } => {
                self.validate_shell_capability(commands, *elevated)
            }
            Capability::UI { .. } => {
                // UI capabilities are generally low risk
                Ok(true)
            }
            Capability::Memory { max_mb } => {
                if *max_mb > self.policy.max_memory_mb {
                    return Err(CapabilityError::ResourceLimitExceeded {
                        resource: "memory".to_string(),
                        current: *max_mb,
                        limit: self.policy.max_memory_mb,
                    });
                }
                Ok(true)
            }
            Capability::Compute {
                max_cpu_percent, ..
            } => {
                if *max_cpu_percent > self.policy.max_cpu_percent {
                    return Err(CapabilityError::ResourceLimitExceeded {
                        resource: "cpu".to_string(),
                        current: *max_cpu_percent as u64,
                        limit: self.policy.max_cpu_percent as u64,
                    });
                }
                Ok(true)
            }
        }
    }

    fn validate_filesystem_capability(
        &self,
        mode: &AccessMode,
        paths: &[PathBuf],
    ) -> Result<bool, CapabilityError> {
        for path in paths {
            let path_allowed = self
                .policy
                .allowed_fs_roots
                .iter()
                .any(|root| path.starts_with(root) || root.to_string_lossy() == "*");

            if !path_allowed {
                return Err(CapabilityError::PolicyViolation {
                    details: format!("Path {path:?} not in allowed roots"),
                });
            }

            // Extra validation for write/execute modes
            if matches!(
                mode,
                AccessMode::Write | AccessMode::ReadWrite | AccessMode::Execute
            ) && path.to_string_lossy().contains("..")
            {
                return Err(CapabilityError::PolicyViolation {
                    details: "Path traversal detected".to_string(),
                });
            }
        }
        Ok(true)
    }

    fn validate_network_capability(
        &self,
        _mode: &NetworkMode,
        domains: &[String],
    ) -> Result<bool, CapabilityError> {
        for domain in domains {
            let domain_allowed = self.policy.allowed_domains.iter().any(|allowed| {
                allowed == "*" || allowed == domain || domain.ends_with(&format!(".{allowed}"))
            });

            if !domain_allowed {
                return Err(CapabilityError::PolicyViolation {
                    details: format!("Domain {domain} not in allowed list"),
                });
            }
        }
        Ok(true)
    }

    fn validate_shell_capability(
        &self,
        commands: &[String],
        elevated: bool,
    ) -> Result<bool, CapabilityError> {
        if elevated && !self.policy.allow_elevated_shell {
            return Err(CapabilityError::PolicyViolation {
                details: "Elevated shell commands not allowed".to_string(),
            });
        }

        for command in commands {
            let command_allowed = self
                .policy
                .allowed_commands
                .iter()
                .any(|allowed| allowed == "*" || allowed == command);

            if !command_allowed {
                return Err(CapabilityError::PolicyViolation {
                    details: format!("Command {command} not in allowed list"),
                });
            }
        }

        Ok(true)
    }
}

impl CapabilityChecker for DefaultCapabilityChecker {
    fn check_capability(&self, capability: &Capability) -> Result<bool, CapabilityError> {
        // First check if already granted
        if self.granted_capabilities.contains(capability) {
            return Ok(true);
        }

        // Then validate against policy
        self.validate_capability(capability)
    }

    fn request_capabilities(
        &mut self,
        capabilities: Vec<Capability>,
    ) -> Result<(), CapabilityError> {
        // Validate all capabilities first
        for capability in &capabilities {
            if !self.validate_capability(capability)? {
                return Err(CapabilityError::Denied {
                    capability: capability.clone(),
                });
            }
        }

        // Grant all if validation passes
        for capability in capabilities {
            self.granted_capabilities.insert(capability);
        }

        Ok(())
    }

    fn has_capability(&self, capability: &Capability) -> bool {
        self.granted_capabilities.contains(capability)
    }

    fn get_capabilities(&self) -> &HashSet<Capability> {
        &self.granted_capabilities
    }
}

/// Capability checking utilities
pub struct CapabilityUtils;

impl CapabilityUtils {
    /// Check capability specification against checker
    pub fn check_capability_spec(
        checker: &dyn CapabilityChecker,
        spec: &CapabilitySpec,
    ) -> Result<Vec<Capability>, CapabilityError> {
        let mut missing = Vec::new();

        // Check required capabilities
        for capability in &spec.required {
            if !checker.check_capability(capability)? {
                missing.push(capability.clone());
            }
        }

        if missing.is_empty() {
            debug!(
                "All required capabilities satisfied for spec: {}",
                spec.justification
            );
            Ok(missing)
        } else {
            warn!("Missing required capabilities: {:?}", missing);
            Ok(missing)
        }
    }

    /// Create default capabilities for common tool types
    pub fn default_capabilities_for_tool_type(tool_type: &str) -> CapabilitySpec {
        match tool_type {
            "file_reader" => CapabilitySpec::new(vec![Capability::Filesystem {
                mode: AccessMode::Read,
                paths: vec![PathBuf::from(".")],
            }])
            .with_justification("Tool needs to read files".to_string()),

            "web_scraper" => CapabilitySpec::new(vec![Capability::Network {
                mode: NetworkMode::Outbound,
                domains: vec!["*".to_string()],
            }])
            .with_justification("Tool needs to access websites".to_string()),

            "shell_executor" => CapabilitySpec::new(vec![Capability::Shell {
                commands: ["ls", "cat", "echo"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                elevated: false,
            }])
            .with_justification("Tool needs to execute shell commands".to_string()),

            _ => CapabilitySpec::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_checker() {
        let checker = DefaultCapabilityChecker::new();

        let read_capability = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from(".")],
        };

        assert!(checker
            .check_capability(&read_capability)
            .expect("Operation failed - converted from unwrap()"));
    }

    #[test]
    fn test_policy_enforcement() {
        let strict_policy = CapabilityPolicy::strict();
        let checker = DefaultCapabilityChecker::with_policy(strict_policy);

        let high_risk_capability = Capability::Shell {
            commands: vec!["rm".to_string()],
            elevated: true,
        };

        assert!(checker.check_capability(&high_risk_capability).is_err());
    }

    #[test]
    fn test_capability_granting() {
        let mut checker = DefaultCapabilityChecker::new();

        let capability = Capability::Memory { max_mb: 100 };

        assert!(checker.grant_capability(capability.clone()).is_ok());
        assert!(checker.has_capability(&capability));
    }

    #[test]
    fn test_capability_spec_checking() {
        let checker = DefaultCapabilityChecker::new();

        let spec = CapabilitySpec::new(vec![Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from(".")],
        }]);

        let missing = CapabilityUtils::check_capability_spec(&checker, &spec)
            .expect("Operation failed - converted from unwrap()");
        assert!(missing.is_empty());
    }
}
