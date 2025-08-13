// P1.2.4.a: Wasmtime Sandboxing Implementation
// Comprehensive WASM sandbox with WASI capabilities, resource limits, and security enforcement

pub mod resource_limits;
pub mod sandbox_violations;
pub mod wasi_config;
pub mod wasm_sandbox;

pub use resource_limits::{ResourceLimiter, ResourceLimits};
pub use sandbox_violations::{SandboxViolation, ViolationType};
pub use wasi_config::{FileSystemAccess, NetworkAccess, WasiSandboxConfig};
pub use wasm_sandbox::{SandboxedModule, WasmSandbox};

use crate::capabilities::Capability;
use crate::manifest::ToolManifest;
use anyhow::Result;
use thiserror::Error;

/// Sandbox configuration errors
#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("WASI configuration failed: {0}")]
    WasiConfigError(String),

    #[error("Resource limit exceeded: {resource} = {current}, max = {limit}")]
    ResourceLimitExceeded {
        resource: String,
        current: u64,
        limit: u64,
    },

    #[error("Sandbox violation detected: {violation_type:?} - {details}")]
    SandboxViolation {
        violation_type: ViolationType,
        details: String,
    },

    #[error("Sandbox initialization failed: {0}")]
    InitializationError(String),

    #[error("Permission denied: {operation} not allowed")]
    PermissionDenied { operation: String },
}

/// Sandbox configuration derived from tool manifest and capabilities
#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    /// WASI configuration for filesystem and networking
    pub wasi_config: WasiSandboxConfig,
    /// Resource limits for memory, CPU, and execution time
    pub resource_limits: ResourceLimits,
    /// Enable debug mode for sandbox inspection
    pub debug_mode: bool,
    /// Capability-based permissions
    pub allowed_capabilities: Vec<Capability>,
}

impl SandboxConfig {
    /// Create sandbox configuration from tool manifest
    pub fn from_manifest(manifest: &ToolManifest) -> Result<Self, SandboxError> {
        let capability_spec = manifest.effective_capability_spec();

        // Extract filesystem and network capabilities
        let mut wasi_config = WasiSandboxConfig::default();
        let mut allowed_capabilities = Vec::new();

        for capability in &capability_spec.required {
            match capability {
                Capability::Filesystem { mode, paths } => {
                    for path in paths {
                        match mode {
                            crate::capabilities::AccessMode::Read => {
                                wasi_config.add_read_only_dir(path.clone())?;
                            }
                            crate::capabilities::AccessMode::Write
                            | crate::capabilities::AccessMode::ReadWrite => {
                                wasi_config.add_read_write_dir(path.clone())?;
                            }
                            crate::capabilities::AccessMode::Execute => {
                                // Execute permissions require special handling
                                return Err(SandboxError::PermissionDenied {
                                    operation: "Execute filesystem permissions in WASM sandbox"
                                        .to_string(),
                                });
                            }
                        }
                    }
                }
                Capability::Network { mode, domains } => match mode {
                    crate::capabilities::NetworkMode::Outbound => {
                        for domain in domains {
                            wasi_config.add_allowed_host(domain.clone());
                        }
                    }
                    crate::capabilities::NetworkMode::Inbound
                    | crate::capabilities::NetworkMode::Both => {
                        return Err(SandboxError::PermissionDenied {
                            operation: "Inbound network access in WASM sandbox".to_string(),
                        });
                    }
                },
                _ => {
                    // Other capabilities are noted but not directly used in WASI config
                }
            }
            allowed_capabilities.push(capability.clone());
        }

        // Set resource limits from manifest
        let resource_limits = ResourceLimits {
            max_memory_bytes: manifest.effective_memory_limit() * 1024 * 1024, // MB to bytes
            max_execution_time_ms: manifest.effective_timeout(),
            fuel_limit: Some(manifest.effective_fuel_limit()),
            max_wasm_stack: 512 * 1024, // 512KB default stack
        };

        Ok(Self {
            wasi_config,
            resource_limits,
            debug_mode: false,
            allowed_capabilities,
        })
    }

    /// Enable debug mode for detailed sandbox inspection
    pub fn with_debug(mut self) -> Self {
        self.debug_mode = true;
        self
    }

    /// Override resource limits
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    /// Validate configuration consistency
    pub fn validate(&self) -> Result<(), SandboxError> {
        self.wasi_config.validate()?;
        self.resource_limits.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ToolMetadata, ToolType};
    use std::path::PathBuf;

    #[test]
    fn test_sandbox_config_from_manifest() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "Test tool".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .require_capability(Capability::Filesystem {
            mode: crate::capabilities::AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        });

        let config = SandboxConfig::from_manifest(&manifest)
            .expect("Operation failed - converted from unwrap()");
        assert!(!config.allowed_capabilities.is_empty());
        assert!(config
            .wasi_config
            .filesystem_access
            .read_only_dirs
            .contains(&PathBuf::from("/tmp")));
    }

    #[test]
    fn test_sandbox_config_validation() {
        let config = SandboxConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_execute_permissions_rejection() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "Test tool".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .require_capability(Capability::Filesystem {
            mode: crate::capabilities::AccessMode::Execute,
            paths: vec![PathBuf::from("/bin")],
        });

        let result = SandboxConfig::from_manifest(&manifest);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SandboxError::PermissionDenied { .. }
        ));
    }
}
