// Tool Capability System
// P1.2.3: Comprehensive capability-based security for Tools Platform 2.0

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;

pub mod checker;
pub mod validation;

/// Tool capability enumeration with detailed permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Filesystem access with specific modes and path restrictions
    Filesystem {
        mode: AccessMode,
        paths: Vec<PathBuf>,
    },
    /// Network access with direction and domain filtering
    Network {
        mode: NetworkMode,
        domains: Vec<String>,
    },
    /// Shell command execution with command whitelist and elevation control
    Shell {
        commands: Vec<String>,
        elevated: bool,
    },
    /// User interface interactions with mode restrictions
    UI { modes: Vec<UIMode> },
    /// Memory usage limits
    Memory { max_mb: u64 },
    /// Compute resource limits
    Compute {
        max_cpu_percent: u8,
        max_duration_ms: u64,
    },
}

/// File system access modes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
    Execute,
}

/// Network access modes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkMode {
    Inbound,
    Outbound,
    Both,
}

/// User interface interaction modes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UIMode {
    Display,
    Input,
    Notification,
}

/// Tool capability specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySpec {
    /// Required capabilities for tool operation
    pub required: HashSet<Capability>,
    /// Optional capabilities that enhance functionality
    pub optional: HashSet<Capability>,
    /// Capability justification for security audit
    pub justification: String,
}

/// Capability check result
#[derive(Debug, Clone)]
pub enum CapabilityCheckResult {
    /// All capabilities granted
    Granted,
    /// Some capabilities denied
    Denied { missing: Vec<Capability> },
    /// Capability check failed
    Failed { error: String },
}

/// Capability-related errors
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability denied: {capability:?}")]
    Denied { capability: Capability },

    #[error("Invalid capability specification: {reason}")]
    InvalidSpec { reason: String },

    #[error("Capability validation failed: {error}")]
    ValidationFailed { error: String },

    #[error("Security policy violation: {details}")]
    PolicyViolation { details: String },

    #[error("Resource limit exceeded: {resource} = {current}, max = {limit}")]
    ResourceLimitExceeded {
        resource: String,
        current: u64,
        limit: u64,
    },
}

/// Capability checker trait for different contexts
pub trait CapabilityChecker {
    /// Check if a specific capability is allowed
    fn check_capability(&self, capability: &Capability) -> Result<bool, CapabilityError>;

    /// Request multiple capabilities at once
    fn request_capabilities(
        &mut self,
        capabilities: Vec<Capability>,
    ) -> Result<(), CapabilityError>;

    /// Check if capability is currently held
    fn has_capability(&self, capability: &Capability) -> bool;

    /// Get current capability set
    fn get_capabilities(&self) -> &HashSet<Capability>;
}

impl Default for CapabilitySpec {
    fn default() -> Self {
        Self {
            required: HashSet::new(),
            optional: HashSet::new(),
            justification: "No specific capabilities required".to_string(),
        }
    }
}

impl CapabilitySpec {
    /// Create a new capability specification
    pub fn new(required: Vec<Capability>) -> Self {
        Self {
            required: required.into_iter().collect(),
            optional: HashSet::new(),
            justification: "Tool-specific capabilities".to_string(),
        }
    }

    /// Add required capability
    pub fn require(mut self, capability: Capability) -> Self {
        self.required.insert(capability);
        self
    }

    /// Add optional capability
    pub fn optional(mut self, capability: Capability) -> Self {
        self.optional.insert(capability);
        self
    }

    /// Set justification
    pub fn with_justification(mut self, justification: String) -> Self {
        self.justification = justification;
        self
    }

    /// Get all capabilities (required + optional)
    pub fn all_capabilities(&self) -> HashSet<Capability> {
        self.required.union(&self.optional).cloned().collect()
    }

    /// Check if specification is minimal (only necessary capabilities)
    pub fn is_minimal(&self) -> bool {
        // Heuristic: minimal if has few capabilities and good justification
        self.required.len() <= 3 && !self.justification.is_empty()
    }
}

impl Capability {
    /// Get human-readable description of capability
    pub fn description(&self) -> String {
        match self {
            Capability::Filesystem { mode, paths } => {
                format!("Filesystem {:?} access to {} paths", mode, paths.len())
            }
            Capability::Network { mode, domains } => {
                format!("Network {:?} access to {} domains", mode, domains.len())
            }
            Capability::Shell { commands, elevated } => {
                format!(
                    "Shell execution of {} commands (elevated: {})",
                    commands.len(),
                    elevated
                )
            }
            Capability::UI { modes } => {
                format!("UI interactions: {modes:?}")
            }
            Capability::Memory { max_mb } => {
                format!("Memory limit: {max_mb} MB")
            }
            Capability::Compute {
                max_cpu_percent,
                max_duration_ms,
            } => {
                format!("Compute limit: {max_cpu_percent}% CPU, {max_duration_ms} ms duration")
            }
        }
    }

    /// Get security risk level (0-10 scale)
    pub fn risk_level(&self) -> u8 {
        match self {
            Capability::Filesystem {
                mode: AccessMode::Read,
                ..
            } => 3,
            Capability::Filesystem {
                mode: AccessMode::Write | AccessMode::ReadWrite,
                ..
            } => 7,
            Capability::Filesystem {
                mode: AccessMode::Execute,
                ..
            } => 9,
            Capability::Network {
                mode: NetworkMode::Outbound,
                ..
            } => 5,
            Capability::Network {
                mode: NetworkMode::Inbound | NetworkMode::Both,
                ..
            } => 7,
            Capability::Shell {
                elevated: false, ..
            } => 6,
            Capability::Shell { elevated: true, .. } => 9,
            Capability::UI { .. } => 4,
            Capability::Memory { max_mb } if *max_mb > 1000 => 5,
            Capability::Memory { .. } => 2,
            Capability::Compute {
                max_cpu_percent, ..
            } if *max_cpu_percent > 80 => 6,
            Capability::Compute { .. } => 3,
        }
    }

    /// Check if capability conflicts with another
    pub fn conflicts_with(&self, other: &Capability) -> bool {
        match (self, other) {
            // Filesystem conflicts
            (
                Capability::Filesystem { paths: p1, .. },
                Capability::Filesystem { paths: p2, .. },
            ) => p1.iter().any(|path1| p2.iter().any(|path2| path1 == path2)),
            // Network conflicts on same domains
            (Capability::Network { domains: d1, .. }, Capability::Network { domains: d2, .. }) => {
                d1.iter()
                    .any(|domain1| d2.iter().any(|domain2| domain1 == domain2))
            }
            // Shell conflicts on same commands
            (Capability::Shell { commands: c1, .. }, Capability::Shell { commands: c2, .. }) => {
                c1.iter().any(|cmd1| c2.iter().any(|cmd2| cmd1 == cmd2))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_spec_creation() {
        let spec = CapabilitySpec::new(vec![Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        }]);

        assert_eq!(spec.required.len(), 1);
        assert_eq!(spec.optional.len(), 0);
    }

    #[test]
    fn test_capability_risk_levels() {
        let read_fs = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        };

        let elevated_shell = Capability::Shell {
            commands: vec!["sudo".to_string()],
            elevated: true,
        };

        assert!(read_fs.risk_level() < elevated_shell.risk_level());
        assert_eq!(elevated_shell.risk_level(), 9);
    }

    #[test]
    fn test_capability_conflicts() {
        let fs1 = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        };

        let fs2 = Capability::Filesystem {
            mode: AccessMode::Write,
            paths: vec![PathBuf::from("/tmp")],
        };

        assert!(fs1.conflicts_with(&fs2));
    }

    #[test]
    fn test_capability_description() {
        let net_cap = Capability::Network {
            mode: NetworkMode::Outbound,
            domains: vec!["example.com".to_string()],
        };

        let desc = net_cap.description();
        assert!(desc.contains("Network"));
        assert!(desc.contains("Outbound"));
    }
}
