// @component: {"k":"C","id":"tool_metadata","t":"Tool metadata system with versioning and security","m":{"cur":0,"tgt":95,"u":"%"},"f":["metadata","versioning","security","registry"]}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Comprehensive tool metadata with security and versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub version: SemanticVersion,
    pub description: String,
    pub author: String,
    pub category: ToolCategory,
    pub tags: Vec<String>,

    // Security metadata
    pub permissions: ToolPermissions,
    pub security_level: SecurityLevel,
    pub trusted: bool,
    pub signature: Option<String>,

    // Runtime metadata
    pub resource_requirements: ResourceRequirements,
    pub dependencies: Vec<ToolDependency>,

    // Usage metadata
    pub registration_time: u64,
    pub last_used: Option<u64>,
    pub usage_count: u64,
    pub performance_metrics: PerformanceMetrics,

    // Schema and examples
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub examples: Vec<ToolExample>,
}

/// Semantic versioning for tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl SemanticVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }

    pub fn is_compatible(&self, required: &SemanticVersion) -> bool {
        if self.major != required.major {
            return false;
        }

        if self.minor > required.minor {
            return true;
        } else if self.minor == required.minor {
            return self.patch >= required.patch;
        }

        false
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }

        if let Some(ref build) = self.build {
            write!(f, "+{}", build)?;
        }

        Ok(())
    }
}

/// Tool categories for organization and filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCategory {
    FileSystem,
    Git,
    Web,
    Database,
    System,
    Development,
    Analysis,
    Communication,
    Security,
    Custom(String),
}

impl ToolCategory {
    pub fn is_high_risk(&self) -> bool {
        matches!(
            self,
            ToolCategory::System | ToolCategory::Security | ToolCategory::Database
        )
    }
}

/// Security permissions for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub file_system: FileSystemPermissions,
    pub network: NetworkPermissions,
    pub system: SystemPermissions,
    pub custom: HashMap<String, bool>,
}

impl Default for ToolPermissions {
    fn default() -> Self {
        Self {
            file_system: FileSystemPermissions::None,
            network: NetworkPermissions::None,
            system: SystemPermissions::None,
            custom: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemPermissions {
    None,
    ReadOnly,
    ReadWrite,
    FullAccess,
    Restricted { allowed_paths: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPermissions {
    None,
    LocalHost,
    InternalNetworks,
    Internet,
    Restricted { allowed_hosts: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemPermissions {
    None,
    ProcessQuery,
    ProcessControl,
    EnvironmentRead,
    EnvironmentWrite,
    FullAccess,
}

/// Security levels for risk assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    Safe,       // Read-only, no external access
    LowRisk,    // Limited write access
    MediumRisk, // Network access, file modifications
    HighRisk,   // System access, external process execution
    Critical,   // Full system access, dangerous operations
}

impl SecurityLevel {
    pub fn requires_confirmation(&self) -> bool {
        matches!(self, SecurityLevel::HighRisk | SecurityLevel::Critical)
    }

    pub fn requires_admin(&self) -> bool {
        matches!(self, SecurityLevel::Critical)
    }
}

/// Resource requirements for execution planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_cores: Option<u32>,
    pub max_execution_time: Option<Duration>,
    pub requires_gpu: bool,
    pub requires_network: bool,
    pub temp_disk_space_mb: Option<u64>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(512), // Default 512MB limit
            max_cpu_cores: Some(1),
            max_execution_time: Some(Duration::from_secs(30)),
            requires_gpu: false,
            requires_network: false,
            temp_disk_space_mb: Some(100),
        }
    }
}

/// Tool dependencies with version constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDependency {
    pub tool_id: String,
    pub min_version: SemanticVersion,
    pub max_version: Option<SemanticVersion>,
    pub optional: bool,
}

/// Performance metrics tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub average_execution_time: Duration,
    pub success_rate: f32,
    pub memory_usage_mb: f32,
    pub cpu_usage_percent: f32,
    pub error_count: u64,
    pub timeout_count: u64,
    pub last_performance_update: Option<u64>,
}

impl PerformanceMetrics {
    pub fn update_execution(&mut self, execution_time: Duration, success: bool) {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.average_execution_time = (self.average_execution_time + execution_time) / 2;

        if success {
            self.success_rate = self.success_rate * 0.9 + 0.1;
        } else {
            self.success_rate *= 0.9;
            self.error_count += 1;
        }

        self.last_performance_update = Some(current_time);
    }

    pub fn is_healthy(&self) -> bool {
        self.success_rate > 0.8 && self.error_count < 5
    }
}

/// Tool usage examples with expected outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub context: Option<String>,
}

impl ToolMetadata {
    pub fn new(id: String, name: String, version: SemanticVersion) -> Self {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id,
            name,
            version,
            description: String::new(),
            author: String::new(),
            category: ToolCategory::Custom("unknown".to_string()),
            tags: Vec::new(),

            permissions: ToolPermissions::default(),
            security_level: SecurityLevel::Safe,
            trusted: false,
            signature: None,

            resource_requirements: ResourceRequirements::default(),
            dependencies: Vec::new(),

            registration_time: current_time,
            last_used: None,
            usage_count: 0,
            performance_metrics: PerformanceMetrics::default(),

            input_schema: serde_json::Value::Object(serde_json::Map::new()),
            output_schema: serde_json::Value::Object(serde_json::Map::new()),
            examples: Vec::new(),
        }
    }

    /// Создать ToolMetadata из упрощённого ToolSpec (для регистрации в ExecutionPipeline)
    pub fn from_spec(spec: &crate::ToolSpec) -> Self {
        let mut meta = ToolMetadata::new(
            spec.name.clone(),
            spec.name.clone(),
            SemanticVersion::new(0, 1, 0),
        );
        meta.description = spec.description.clone();
        meta.input_schema = match serde_json::from_str::<serde_json::Value>(&spec.input_schema) {
            Ok(v) => v,
            Err(_) => serde_json::json!({}),
        };
        meta.examples = spec
            .examples
            .iter()
            .map(|e| ToolExample {
                description: e.clone(),
                input: serde_json::json!({}),
                expected_output: serde_json::json!({}),
                context: None,
            })
            .collect();
        // Грубая оценка категоризации по имени
        meta.category = if spec.name.contains("file") {
            ToolCategory::FileSystem
        } else if spec.name.contains("git") {
            ToolCategory::Git
        } else if spec.name.contains("web") {
            ToolCategory::Web
        } else {
            ToolCategory::Custom("general".into())
        };
        // Пермишены по возможности из ToolSpec.permissions (упрощённая проекция)
        if let Some(p) = &spec.permissions {
            meta.permissions.file_system = if !p.fs_write_roots.is_empty() {
                FileSystemPermissions::ReadWrite
            } else if !p.fs_read_roots.is_empty() {
                FileSystemPermissions::ReadOnly
            } else {
                FileSystemPermissions::None
            };
            meta.permissions.network = if p.net_allowlist.is_empty() {
                NetworkPermissions::None
            } else {
                NetworkPermissions::Restricted {
                    allowed_hosts: p.net_allowlist.clone(),
                }
            };
            meta.permissions.system = if p.allow_shell {
                SystemPermissions::ProcessControl
            } else {
                SystemPermissions::None
            };
        }
        meta
    }

    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.category = category;
        self
    }

    pub fn with_permissions(mut self, permissions: ToolPermissions) -> Self {
        self.permissions = permissions;
        self.update_security_level();
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_author(mut self, author: String) -> Self {
        self.author = author;
        self
    }

    pub fn add_dependency(mut self, dependency: ToolDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn mark_as_trusted(mut self) -> Self {
        self.trusted = true;
        self
    }

    /// Update security level based on permissions
    fn update_security_level(&mut self) {
        let mut level = SecurityLevel::Safe;

        // Check file system permissions
        match &self.permissions.file_system {
            FileSystemPermissions::None | FileSystemPermissions::ReadOnly => {}
            FileSystemPermissions::ReadWrite => level = SecurityLevel::LowRisk,
            FileSystemPermissions::FullAccess => level = SecurityLevel::HighRisk,
            FileSystemPermissions::Restricted { .. } => level = SecurityLevel::LowRisk,
        }

        // Check network permissions
        match &self.permissions.network {
            NetworkPermissions::None => {}
            NetworkPermissions::LocalHost => level = level.max(SecurityLevel::LowRisk),
            NetworkPermissions::InternalNetworks => level = level.max(SecurityLevel::MediumRisk),
            NetworkPermissions::Internet => level = level.max(SecurityLevel::HighRisk),
            NetworkPermissions::Restricted { .. } => level = level.max(SecurityLevel::MediumRisk),
        }

        // Check system permissions
        match &self.permissions.system {
            SystemPermissions::None => {}
            SystemPermissions::ProcessQuery | SystemPermissions::EnvironmentRead => {
                level = level.max(SecurityLevel::LowRisk);
            }
            SystemPermissions::ProcessControl | SystemPermissions::EnvironmentWrite => {
                level = level.max(SecurityLevel::MediumRisk);
            }
            SystemPermissions::FullAccess => level = SecurityLevel::Critical,
        }

        self.security_level = level;
    }

    pub fn record_usage(&mut self) {
        self.usage_count += 1;
        self.last_used = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    pub fn is_dependencies_satisfied(
        &self,
        available_tools: &HashMap<String, ToolMetadata>,
    ) -> Result<()> {
        for dependency in &self.dependencies {
            if dependency.optional {
                continue;
            }

            let dep_tool = available_tools
                .get(&dependency.tool_id)
                .ok_or_else(|| anyhow::anyhow!("Missing dependency: {}", dependency.tool_id))?;

            if !dep_tool.version.is_compatible(&dependency.min_version) {
                return Err(anyhow::anyhow!(
                    "Incompatible dependency version: {} requires {} but {} is available",
                    dependency.tool_id,
                    dependency.min_version,
                    dep_tool.version
                ));
            }

            if let Some(ref max_version) = dependency.max_version {
                if dep_tool.version > *max_version {
                    return Err(anyhow::anyhow!(
                        "Dependency version too high: {} requires <= {} but {} is available",
                        dependency.tool_id,
                        max_version,
                        dep_tool.version
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn can_execute_safely(&self) -> bool {
        self.trusted
            || self.security_level <= SecurityLevel::LowRisk
            || self.performance_metrics.is_healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_version_compatibility() {
        let v1 = SemanticVersion::new(1, 2, 3);
        let v2 = SemanticVersion::new(1, 2, 0);
        let v3 = SemanticVersion::new(2, 0, 0);

        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));
    }

    #[test]
    fn test_security_level_from_permissions() {
        let mut metadata = ToolMetadata::new(
            "test".to_string(),
            "Test Tool".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        metadata = metadata.with_permissions(ToolPermissions {
            file_system: FileSystemPermissions::FullAccess,
            network: NetworkPermissions::Internet,
            system: SystemPermissions::None,
            custom: HashMap::new(),
        });

        assert_eq!(metadata.security_level, SecurityLevel::HighRisk);
    }

    #[test]
    fn test_performance_metrics_update() {
        let mut metrics = PerformanceMetrics::default();

        metrics.update_execution(Duration::from_secs(1), true);
        assert!(metrics.success_rate > 0.0);

        metrics.update_execution(Duration::from_secs(2), false);
        assert!(metrics.error_count == 1);
    }
}
