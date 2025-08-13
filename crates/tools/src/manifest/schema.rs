// Tool Manifest Schema Definitions
// P1.2.2.a: Tool JSON Schema Implementation
// P1.2.3: Enhanced with Capability System Integration

use crate::capabilities::{Capability, CapabilitySpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    /// WebAssembly tool
    Wasm,
    /// Native executable
    Native,
    /// Script (Python, JavaScript, etc.)
    Script,
}

/// Tool capability enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolCapability {
    /// Filesystem access
    Filesystem,
    /// Network access
    Network,
    /// Shell command execution
    Shell,
    /// User interface interactions
    Ui,
}

/// Permission type for filesystem access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemPermission {
    Read,
    Write,
}

/// Permission type for network access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkPermission {
    Outbound,
    Inbound,
}

/// Permission type for shell access
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShellPermission {
    Execute,
}

/// Runtime configuration for tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Maximum memory in MB
    pub max_memory_mb: Option<u64>,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: Option<u64>,
    /// Fuel limit for WASM (instruction count)
    pub fuel_limit: Option<u64>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(64),
            max_execution_time_ms: Some(30000), // 30 seconds
            fuel_limit: Some(1_000_000),        // 1M instructions
        }
    }
}

/// Permission configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionConfig {
    /// Filesystem permissions
    pub filesystem: Option<Vec<FilesystemPermission>>,
    /// Network permissions
    pub network: Option<Vec<NetworkPermission>>,
    /// Shell permissions
    pub shell: Option<Vec<ShellPermission>>,
}

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool author
    pub author: String,
    /// License identifier (SPDX)
    pub license: String,
    /// Repository URL
    pub repository: Option<String>,
    /// Tool website/homepage
    pub homepage: Option<String>,
    /// Documentation URL
    pub documentation: Option<String>,
}

/// Tool manifest structure - complete tool.json schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Tool name (unique identifier)
    pub name: String,
    /// Tool version (semantic versioning)
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Tool type
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    /// Required capabilities (legacy format)
    #[serde(default)]
    pub capabilities: Vec<ToolCapability>,
    /// Entry point file/executable
    pub entry_point: String,
    /// Runtime configuration
    #[serde(default)]
    pub runtime_config: RuntimeConfig,
    /// Permission configuration (legacy format)
    #[serde(default)]
    pub permissions: PermissionConfig,
    /// Tool metadata
    pub metadata: ToolMetadata,
    /// Additional tool-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// Enhanced capability specification (P1.2.3)
    #[serde(default)]
    pub capability_spec: CapabilitySpec,
}

impl ToolManifest {
    /// Create a new tool manifest with minimum required fields
    pub fn new(
        name: String,
        version: String,
        description: String,
        tool_type: ToolType,
        entry_point: String,
        author: String,
        license: String,
    ) -> Self {
        Self {
            name,
            version,
            description,
            tool_type,
            capabilities: Vec::new(),
            entry_point,
            runtime_config: RuntimeConfig::default(),
            permissions: PermissionConfig::default(),
            metadata: ToolMetadata {
                author,
                license,
                repository: None,
                homepage: None,
                documentation: None,
            },
            config: HashMap::new(),
            capability_spec: CapabilitySpec::default(),
        }
    }

    /// Add capability to the tool
    pub fn with_capability(mut self, capability: ToolCapability) -> Self {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
        self
    }

    /// Set runtime configuration
    pub fn with_runtime_config(mut self, config: RuntimeConfig) -> Self {
        self.runtime_config = config;
        self
    }

    /// Set filesystem permissions
    pub fn with_filesystem_permissions(mut self, permissions: Vec<FilesystemPermission>) -> Self {
        self.permissions.filesystem = Some(permissions);
        self
    }

    /// Set network permissions
    pub fn with_network_permissions(mut self, permissions: Vec<NetworkPermission>) -> Self {
        self.permissions.network = Some(permissions);
        self
    }

    /// Set shell permissions
    pub fn with_shell_permissions(mut self, permissions: Vec<ShellPermission>) -> Self {
        self.permissions.shell = Some(permissions);
        self
    }

    /// Set enhanced capability specification (P1.2.3)
    pub fn with_capability_spec(mut self, spec: CapabilitySpec) -> Self {
        self.capability_spec = spec;
        self
    }

    /// Add required capability to enhanced spec
    pub fn require_capability(mut self, capability: Capability) -> Self {
        self.capability_spec = self.capability_spec.require(capability);
        self
    }

    /// Add optional capability to enhanced spec
    pub fn optional_capability(mut self, capability: Capability) -> Self {
        self.capability_spec = self.capability_spec.optional(capability);
        self
    }

    /// Get effective capability specification (combines legacy and enhanced)
    pub fn effective_capability_spec(&self) -> CapabilitySpec {
        let mut spec = self.capability_spec.clone();

        // Convert legacy capabilities to enhanced format
        for legacy_cap in &self.capabilities {
            let enhanced_cap = match legacy_cap {
                ToolCapability::Filesystem => Capability::Filesystem {
                    mode: crate::capabilities::AccessMode::Read,
                    paths: vec![std::path::PathBuf::from(".")],
                },
                ToolCapability::Network => Capability::Network {
                    mode: crate::capabilities::NetworkMode::Outbound,
                    domains: vec!["*".to_string()],
                },
                ToolCapability::Shell => Capability::Shell {
                    commands: vec!["*".to_string()],
                    elevated: false,
                },
                ToolCapability::Ui => Capability::UI {
                    modes: vec![crate::capabilities::UIMode::Display],
                },
            };
            spec.required.insert(enhanced_cap);
        }

        spec
    }

    /// Check if tool requires elevated privileges
    pub fn requires_elevated_privileges(&self) -> bool {
        self.effective_capability_spec().required.iter().any(|cap| {
            matches!(
                cap,
                Capability::Shell { elevated: true, .. }
                    | Capability::Filesystem {
                        mode: crate::capabilities::AccessMode::Execute,
                        ..
                    }
            )
        })
    }

    /// Get maximum risk level of tool capabilities
    pub fn max_risk_level(&self) -> u8 {
        self.effective_capability_spec()
            .all_capabilities()
            .iter()
            .map(|cap| cap.risk_level())
            .max()
            .unwrap_or(0)
    }

    /// Add metadata field
    pub fn with_repository(mut self, repository: String) -> Self {
        self.metadata.repository = Some(repository);
        self
    }

    /// Add custom configuration
    pub fn with_config(mut self, key: String, value: serde_json::Value) -> Self {
        self.config.insert(key, value);
        self
    }

    /// Validate manifest consistency
    pub fn validate_consistency(&self) -> Result<(), String> {
        // Check that capabilities match permissions
        for capability in &self.capabilities {
            match capability {
                ToolCapability::Filesystem => {
                    if self.permissions.filesystem.is_none() {
                        return Err(
                            "Tool declares filesystem capability but has no filesystem permissions"
                                .to_string(),
                        );
                    }
                }
                ToolCapability::Network => {
                    if self.permissions.network.is_none() {
                        return Err(
                            "Tool declares network capability but has no network permissions"
                                .to_string(),
                        );
                    }
                }
                ToolCapability::Shell => {
                    if self.permissions.shell.is_none() {
                        return Err(
                            "Tool declares shell capability but has no shell permissions"
                                .to_string(),
                        );
                    }
                }
                ToolCapability::Ui => {
                    // UI capability doesn't require specific permissions
                }
            }
        }

        // Check resource limits are reasonable
        if let Some(memory) = self.runtime_config.max_memory_mb {
            if memory == 0 || memory > 2048 {
                return Err("Memory limit must be between 1MB and 2048MB".to_string());
            }
        }

        if let Some(time) = self.runtime_config.max_execution_time_ms {
            if time == 0 || time > 300_000 {
                // Max 5 minutes
                return Err("Execution time limit must be between 1ms and 300000ms".to_string());
            }
        }

        if let Some(fuel) = self.runtime_config.fuel_limit {
            if fuel == 0 || fuel > 100_000_000 {
                // Max 100M instructions
                return Err("Fuel limit must be between 1 and 100000000 instructions".to_string());
            }
        }

        Ok(())
    }

    /// Check if tool requires specific capability
    pub fn has_capability(&self, capability: &ToolCapability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get effective memory limit (with default fallback)
    pub fn effective_memory_limit(&self) -> u64 {
        self.runtime_config.max_memory_mb.unwrap_or(64)
    }

    /// Get effective execution timeout (with default fallback)
    pub fn effective_timeout(&self) -> u64 {
        self.runtime_config.max_execution_time_ms.unwrap_or(30000)
    }

    /// Get effective fuel limit (with default fallback)
    pub fn effective_fuel_limit(&self) -> u64 {
        self.runtime_config.fuel_limit.unwrap_or(1_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_creation() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "A test tool".to_string(),
            ToolType::Wasm,
            "main.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        assert_eq!(manifest.name, "test-tool");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.tool_type, ToolType::Wasm);
        assert_eq!(manifest.entry_point, "main.wasm");
    }

    #[test]
    fn test_capability_consistency_validation() {
        let mut manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "A test tool".to_string(),
            ToolType::Wasm,
            "main.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .with_capability(ToolCapability::Filesystem);

        // Should fail - capability without permission
        assert!(manifest.validate_consistency().is_err());

        // Add permission - should pass
        manifest = manifest.with_filesystem_permissions(vec![FilesystemPermission::Read]);
        assert!(manifest.validate_consistency().is_ok());
    }

    #[test]
    fn test_resource_limit_validation() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "A test tool".to_string(),
            ToolType::Wasm,
            "main.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .with_runtime_config(RuntimeConfig {
            max_memory_mb: Some(4096), // Too high
            max_execution_time_ms: Some(1000),
            fuel_limit: Some(1000),
        });

        assert!(manifest.validate_consistency().is_err());
    }

    #[test]
    fn test_json_serialization() {
        let manifest = ToolManifest::new(
            "example-tool".to_string(),
            "1.2.3".to_string(),
            "Example tool for testing".to_string(),
            ToolType::Wasm,
            "example.wasm".to_string(),
            "Example Author".to_string(),
            "Apache-2.0".to_string(),
        )
        .with_capability(ToolCapability::Filesystem)
        .with_capability(ToolCapability::Network)
        .with_filesystem_permissions(vec![
            FilesystemPermission::Read,
            FilesystemPermission::Write,
        ])
        .with_network_permissions(vec![NetworkPermission::Outbound])
        .with_repository("https://github.com/example/tool".to_string());

        let json = serde_json::to_string_pretty(&manifest)
            .expect("Operation failed - converted from unwrap()");
        let deserialized: ToolManifest =
            serde_json::from_str(&json).expect("Operation failed - converted from unwrap()");

        assert_eq!(manifest.name, deserialized.name);
        assert_eq!(manifest.capabilities, deserialized.capabilities);
        assert_eq!(
            manifest.metadata.repository,
            deserialized.metadata.repository
        );
    }
}
