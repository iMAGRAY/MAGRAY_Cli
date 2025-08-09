// @component: {"k":"C","id":"plugin_manager","t":"Comprehensive plugin management system","m":{"cur":0,"tgt":95,"u":"%"},"f":["plugin","manager","registry","configuration"]}

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::registry::{SecurityLevel, ToolPermissions};
// use crate::{Tool, ToolInput, ToolOutput, ToolSpec}; // Не используется пока

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPermissionsFs {
    pub mode: String, // none|ro|rw|full|restricted
    #[serde(default)]
    pub allowed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPermissionsNet {
    pub mode: String, // none|localhost|internal|internet|restricted
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPermissionsSystem {
    pub mode: String, // none|proc_query|proc_control|env_read|env_write|full
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestPermissions {
    #[serde(default)]
    pub fs: Option<ManifestPermissionsFs>,
    #[serde(default)]
    pub net: Option<ManifestPermissionsNet>,
    #[serde(default)]
    pub system: Option<ManifestPermissionsSystem>,
    #[serde(default)]
    pub custom: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub id: String,
    pub name: String,
    pub version: String, // semver string
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default = "default_mit")]
    pub license: String,
    #[serde(default)]
    pub keywords: Vec<String>,

    pub plugin_type: String, // wasm|external|shared|script:<lang>|container
    pub entry_point: String,
    #[serde(default)]
    pub configuration_schema: serde_json::Value,
    #[serde(default)]
    pub default_config: serde_json::Value,

    #[serde(default)]
    pub permissions: ManifestPermissions,
}

fn default_mit() -> String { "MIT".into() }

impl ToolManifest {
    fn parse_version(&self) -> Result<PluginVersion> {
        let mut parts = self.version.split('.');
        let major = parts.next().ok_or_else(|| anyhow!("bad version"))?.parse::<u32>()?;
        let minor = parts.next().ok_or_else(|| anyhow!("bad version"))?.parse::<u32>()?;
        let patch = parts.next().unwrap_or("0").split(|c| c == '-' || c == '+').next().unwrap_or("0").parse::<u32>()?;
        Ok(PluginVersion { major, minor, patch, pre_release: None, build_metadata: None })
    }

    fn parse_type(&self) -> PluginType {
        let p = self.plugin_type.to_lowercase();
        if p == "wasm" { PluginType::Wasm }
        else if p == "external" { PluginType::ExternalProcess }
        else if p == "container" { PluginType::Container }
        else if p.starts_with("script:") { PluginType::Script(p[7..].to_string()) }
        else { PluginType::SharedLibrary }
    }

    fn to_registry_permissions(&self) -> ToolPermissions {
        use crate::registry::{FileSystemPermissions, NetworkPermissions, SystemPermissions};
        let mut perms = ToolPermissions::default();
        if let Some(fs) = &self.permissions.fs {
            perms.file_system = match fs.mode.as_str() {
                "none" => FileSystemPermissions::None,
                "ro" => FileSystemPermissions::ReadOnly,
                "rw" => FileSystemPermissions::ReadWrite,
                "full" => FileSystemPermissions::FullAccess,
                "restricted" => FileSystemPermissions::Restricted { allowed_paths: fs.allowed_paths.clone() },
                _ => FileSystemPermissions::None,
            };
        }
        if let Some(net) = &self.permissions.net {
            perms.network = match net.mode.as_str() {
                "none" => NetworkPermissions::None,
                "localhost" => NetworkPermissions::LocalHost,
                "internal" => NetworkPermissions::InternalNetworks,
                "internet" => NetworkPermissions::Internet,
                "restricted" => NetworkPermissions::Restricted { allowed_hosts: net.allowed_hosts.clone() },
                _ => NetworkPermissions::None,
            };
        }
        if let Some(sys) = &self.permissions.system {
            perms.system = match sys.mode.as_str() {
                "none" => SystemPermissions::None,
                "proc_query" => SystemPermissions::ProcessQuery,
                "proc_control" => SystemPermissions::ProcessControl,
                "env_read" => SystemPermissions::EnvironmentRead,
                "env_write" => SystemPermissions::EnvironmentWrite,
                "full" => SystemPermissions::FullAccess,
                _ => SystemPermissions::None,
            };
        }
        perms.custom = self.permissions.custom.clone();
        perms
    }

    fn into_plugin_metadata(self, installation_path: Option<PathBuf>) -> Result<PluginMetadata> {
        // Compute derived fields before moving parts of self to avoid partial move borrow issues
        let version = self.parse_version()?;
        let plugin_type = self.parse_type();
        let registry_permissions = self.to_registry_permissions();
        let configuration_schema = if self.configuration_schema.is_null() { serde_json::json!({}) } else { self.configuration_schema.clone() };
        let default_config = if self.default_config.is_null() { serde_json::json!({}) } else { self.default_config.clone() };
        let metadata = PluginMetadata {
            id: self.id,
            name: self.name,
            version,
            description: self.description,
            author: self.author,
            homepage: self.homepage,
            repository: self.repository,
            license: self.license,
            keywords: self.keywords,
            plugin_type,
            entry_point: self.entry_point,
            configuration_schema,
            default_config,
            runtime_requirements: RuntimeRequirements::default(),
            dependencies: Vec::new(),
            required_permissions: registry_permissions,
            security_level: SecurityLevel::Safe,
            signed: false,
            signature: None,
            installed_at: None,
            last_updated: None,
            installation_path,
            state: PluginState::Uninstalled,
            load_count: 0,
            error_count: 0,
            last_error: None,
        };
        Ok(metadata)
    }
}

/// Plugin metadata with comprehensive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: PluginVersion,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: String,
    pub keywords: Vec<String>,

    // Plugin configuration
    pub plugin_type: PluginType,
    pub entry_point: String,
    pub configuration_schema: serde_json::Value,
    pub default_config: serde_json::Value,

    // Runtime requirements
    pub runtime_requirements: RuntimeRequirements,
    pub dependencies: Vec<PluginDependency>,

    // Security and permissions
    pub required_permissions: ToolPermissions,
    pub security_level: SecurityLevel,
    pub signed: bool,
    pub signature: Option<String>,

    // Installation and lifecycle
    pub installed_at: Option<u64>,
    pub last_updated: Option<u64>,
    pub installation_path: Option<PathBuf>,

    // Plugin state
    pub state: PluginState,
    pub load_count: u64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

/// Plugin version with semantic versioning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
}

impl PluginVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build_metadata: None,
        }
    }

    pub fn is_compatible(&self, required: &PluginVersion) -> bool {
        // Major version must match, minor/patch must be >= required
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

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }

        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }

        Ok(())
    }
}

/// Plugin types supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginType {
    Wasm,            // WebAssembly plugin
    ExternalProcess, // External process plugin
    SharedLibrary,   // Dynamic library (.so/.dll/.dylib)
    Script(String),  // Script plugin (Python, JS, etc.)
    Container,       // Container-based plugin
}

/// Runtime requirements for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRequirements {
    pub min_rust_version: Option<String>,
    pub required_features: Vec<String>,
    pub optional_features: Vec<String>,
    pub system_dependencies: Vec<String>,
    pub environment_variables: Vec<String>,
}

impl Default for RuntimeRequirements {
    fn default() -> Self {
        Self {
            min_rust_version: None,
            required_features: Vec::new(),
            optional_features: Vec::new(),
            system_dependencies: Vec::new(),
            environment_variables: Vec::new(),
        }
    }
}

/// Plugin dependencies with version constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub min_version: PluginVersion,
    pub max_version: Option<PluginVersion>,
    pub optional: bool,
    pub features: Vec<String>,
}

/// Plugin state tracking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginState {
    Uninstalled,
    Installing,
    Installed,
    Loading,
    Loaded,
    Active,
    Error(String),
    Disabled,
    Unloading,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub plugin_id: String,
    pub config: serde_json::Value,
    pub environment: HashMap<String, String>,
    pub resource_limits: PluginResourceLimits,
    pub auto_start: bool,
    pub auto_reload: bool,
}

/// Resource limits for plugin execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<u32>,
    pub max_disk_usage_mb: Option<u64>,
    pub max_network_connections: Option<u32>,
    pub execution_timeout: Option<Duration>,
}

impl Default for PluginResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(256),
            max_cpu_percent: Some(50),
            max_disk_usage_mb: Some(100),
            max_network_connections: Some(10),
            execution_timeout: Some(Duration::from_secs(30)),
        }
    }
}

/// Plugin registry for managing installed plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, PluginMetadata>>>,
    configurations: Arc<RwLock<HashMap<String, PluginConfiguration>>>,
    plugin_directory: PathBuf,
    config_directory: PathBuf,
}

impl PluginRegistry {
    pub fn new(plugin_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            configurations: Arc::new(RwLock::new(HashMap::new())),
            plugin_directory: plugin_dir,
            config_directory: config_dir,
        }
    }

    /// Create a Tool instance from a registered plugin (supports: Wasm, ExternalProcess)
    pub async fn materialize_as_tool(&self, plugin_id: &str) -> Result<Box<dyn crate::Tool>> {
        use super::external_process::{ExternalProcessPlugin, ProcessConfig, ProcessIsolation, ProcessResourceLimits, StderrMode, StdinMode, StdoutMode};
        use super::wasm_plugin::WasmPlugin;

        let metadata = self
            .get_plugin(plugin_id)
            .await
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;
        let config = self
            .get_plugin_configuration(plugin_id)
            .await
            .ok_or_else(|| anyhow!("Plugin configuration not found: {}", plugin_id))?;

        match metadata.plugin_type {
            PluginType::Wasm => {
                let wasm_path = metadata
                    .installation_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("No installation path for WASM plugin"))?
                    .join(&metadata.entry_point);
                let mut instance = WasmPlugin::new(metadata.clone(), config.clone(), &wasm_path).await?;
                instance.start().await?;
                Ok(Box::new(instance))
            }
            PluginType::ExternalProcess => {
                let executable_path = metadata
                    .installation_path
                    .as_ref()
                    .ok_or_else(|| anyhow!("No installation path for external process plugin"))?
                    .join(&metadata.entry_point);
                let process_config = ProcessConfig {
                    executable_path,
                    arguments: Vec::new(),
                    working_directory: metadata.installation_path.clone(),
                    environment_variables: HashMap::new(),
                    stdin_mode: StdinMode::Pipe,
                    stdout_mode: StdoutMode::Pipe,
                    stderr_mode: StderrMode::Pipe,
                    timeout: Duration::from_secs(60),
                    kill_on_timeout: true,
                };
                let mut instance = ExternalProcessPlugin::new(
                    metadata.clone(),
                    config.clone(),
                    process_config,
                    ProcessResourceLimits::default(),
                    ProcessIsolation::User,
                );
                instance.start().await?;
                Ok(Box::new(instance))
            }
            other => Err(anyhow!("Plugin type {:?} not supported as Tool", other)),
        }
    }

    /// Register a new plugin
    pub async fn register_plugin(&self, mut metadata: PluginMetadata) -> Result<()> {
        // Validate metadata
        self.validate_plugin_metadata(&metadata)?;

        // Set installation timestamp
        metadata.installed_at = Some(self.current_timestamp());
        metadata.state = PluginState::Installed;

        // Store in registry
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(metadata.id.clone(), metadata.clone());
        }

        // Create default configuration
        let default_config = PluginConfiguration {
            plugin_id: metadata.id.clone(),
            config: metadata.default_config.clone(),
            environment: HashMap::new(),
            resource_limits: PluginResourceLimits::default(),
            auto_start: false,
            auto_reload: false,
        };

        {
            let mut configs = self.configurations.write().await;
            configs.insert(metadata.id.clone(), default_config);
        }

        // Persist to filesystem
        self.save_plugin_metadata(&metadata).await?;

        info!(
            "📝 Registered plugin: {} v{}",
            metadata.name, metadata.version
        );
        Ok(())
    }

    /// Get plugin metadata by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).cloned()
    }

    /// Get all plugins with optional filtering
    pub async fn list_plugins(&self, filter: Option<PluginFilter>) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;

        match filter {
            Some(filter) => plugins
                .values()
                .filter(|p| filter.matches(p))
                .cloned()
                .collect(),
            None => plugins.values().cloned().collect(),
        }
    }

    /// Update plugin state
    pub async fn update_plugin_state(&self, plugin_id: &str, state: PluginState) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins
            .get_mut(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;

        let old_state = plugin.state.clone();
        plugin.state = state.clone();

        debug!(
            "🔄 Plugin {} state changed: {:?} -> {:?}",
            plugin_id, old_state, state
        );

        // Update error count on error states
        if let PluginState::Error(_) = state {
            plugin.error_count += 1;
        }

        Ok(())
    }

    /// Get plugin configuration
    pub async fn get_plugin_configuration(&self, plugin_id: &str) -> Option<PluginConfiguration> {
        let configs = self.configurations.read().await;
        configs.get(plugin_id).cloned()
    }

    /// Update plugin configuration
    pub async fn update_plugin_configuration(
        &self,
        plugin_id: &str,
        config: PluginConfiguration,
    ) -> Result<()> {
        // Validate configuration against schema
        self.validate_plugin_configuration(&config).await?;

        {
            let mut configs = self.configurations.write().await;
            configs.insert(plugin_id.to_string(), config.clone());
        }

        // Persist to filesystem
        self.save_plugin_configuration(&config).await?;

        debug!("⚙️ Updated configuration for plugin: {}", plugin_id);
        Ok(())
    }

    /// Check plugin dependencies
    pub async fn check_dependencies(&self, plugin_id: &str) -> Result<Vec<String>> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;

        let mut missing_deps = Vec::new();

        for dependency in &plugin.dependencies {
            if dependency.optional {
                continue;
            }

            if let Some(dep_plugin) = plugins.get(&dependency.plugin_id) {
                if !dep_plugin.version.is_compatible(&dependency.min_version) {
                    missing_deps.push(format!(
                        "Incompatible dependency: {} requires {} but {} is installed",
                        dependency.plugin_id, dependency.min_version, dep_plugin.version
                    ));
                }
            } else {
                missing_deps.push(format!("Missing dependency: {}", dependency.plugin_id));
            }
        }

        Ok(missing_deps)
    }

    /// Remove plugin from registry
    pub async fn unregister_plugin(&self, plugin_id: &str) -> Result<()> {
        // Check if other plugins depend on this one
        let dependents = self.find_dependent_plugins(plugin_id).await;
        if !dependents.is_empty() {
            return Err(anyhow!(
                "Cannot unregister plugin: {} plugins depend on it: {:?}",
                dependents.len(),
                dependents
            ));
        }

        // Remove from registry
        let metadata = {
            let mut plugins = self.plugins.write().await;
            plugins
                .remove(plugin_id)
                .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?
        };

        // Remove configuration
        {
            let mut configs = self.configurations.write().await;
            configs.remove(plugin_id);
        }

        // Clean up filesystem
        self.cleanup_plugin_files(&metadata).await?;

        info!("🗑️ Unregistered plugin: {}", metadata.name);
        Ok(())
    }

    /// Find plugins that depend on the given plugin
    async fn find_dependent_plugins(&self, plugin_id: &str) -> Vec<String> {
        let plugins = self.plugins.read().await;

        plugins
            .values()
            .filter(|p| p.dependencies.iter().any(|dep| dep.plugin_id == plugin_id))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Validate plugin metadata
    fn validate_plugin_metadata(&self, metadata: &PluginMetadata) -> Result<()> {
        if metadata.id.is_empty() {
            return Err(anyhow!("Plugin ID cannot be empty"));
        }

        if metadata.name.is_empty() {
            return Err(anyhow!("Plugin name cannot be empty"));
        }

        if metadata.version == PluginVersion::new(0, 0, 0) {
            return Err(anyhow!("Plugin version must be specified"));
        }

        if metadata.entry_point.is_empty() {
            return Err(anyhow!("Plugin entry point cannot be empty"));
        }

        // Validate plugin type specific requirements
        match metadata.plugin_type {
            PluginType::Wasm => {
                if !metadata.entry_point.ends_with(".wasm") {
                    return Err(anyhow!("WASM plugin entry point must end with .wasm"));
                }
            }
            PluginType::Script(ref lang) => {
                let valid_extensions = match lang.as_str() {
                    "python" => vec![".py", ".python"],
                    "javascript" => vec![".js", ".mjs"],
                    "typescript" => vec![".ts"],
                    _ => return Err(anyhow!("Unsupported script language: {}", lang)),
                };

                if !valid_extensions
                    .iter()
                    .any(|ext| metadata.entry_point.ends_with(ext))
                {
                    return Err(anyhow!(
                        "Script entry point has invalid extension for {}",
                        lang
                    ));
                }
            }
            _ => {} // Other types have different validation rules
        }

        Ok(())
    }

    /// Validate plugin configuration against schema
    async fn validate_plugin_configuration(&self, config: &PluginConfiguration) -> Result<()> {
        let plugins = self.plugins.read().await;
        let _plugin = plugins
            .get(&config.plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", config.plugin_id))?;

        // In production, use a proper JSON schema validator
        // For now, just check if it's a valid JSON object
        if !config.config.is_object() {
            return Err(anyhow!("Plugin configuration must be a JSON object"));
        }

        Ok(())
    }

    /// Save plugin metadata to filesystem
    async fn save_plugin_metadata(&self, metadata: &PluginMetadata) -> Result<()> {
        let metadata_path = self.plugin_directory.join(format!("{}.json", metadata.id));
        let json = serde_json::to_string_pretty(metadata)?;
        tokio::fs::write(metadata_path, json).await?;
        Ok(())
    }

    /// Save plugin configuration to filesystem
    async fn save_plugin_configuration(&self, config: &PluginConfiguration) -> Result<()> {
        let config_path = self
            .config_directory
            .join(format!("{}.json", config.plugin_id));
        let json = serde_json::to_string_pretty(config)?;
        tokio::fs::write(config_path, json).await?;
        Ok(())
    }

    /// Load plugins from filesystem
    pub async fn load_from_filesystem(&self) -> Result<usize> {
        let mut loaded_count = 0;

        // Load plugin metadata
        if self.plugin_directory.exists() {
            let mut entries = tokio::fs::read_dir(&self.plugin_directory).await?;

            while let Some(entry) = entries.next_entry().await? {
                if let Some(extension) = entry.path().extension() {
                    if extension == "json" {
                        if let Ok(metadata) =
                            self.load_plugin_metadata_from_file(&entry.path()).await
                        {
                            let plugin_id = metadata.id.clone();

                            // Load configuration if exists
                            if let Ok(config) =
                                self.load_plugin_configuration_from_file(&plugin_id).await
                            {
                                let mut configs = self.configurations.write().await;
                                configs.insert(plugin_id.clone(), config);
                            }

                            // Add to registry
                            let mut plugins = self.plugins.write().await;
                            plugins.insert(plugin_id.clone(), metadata);

                            loaded_count += 1;
                        }
                    }
                }
            }
        }

        debug!("📚 Loaded {} plugins from filesystem", loaded_count);
        Ok(loaded_count)
    }

    /// Scan plugin_directory for tool.json manifests and register plugins
    pub async fn load_manifests_from_directory(&self) -> Result<usize> {
        let mut count = 0usize;
        let root = self.plugin_directory.clone();
        if !root.exists() { return Ok(0); }
        let mut dirs = vec![root.clone()];
        while let Some(dir) = dirs.pop() {
            let mut rd = tokio::fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = rd.next_entry().await {
                let p = entry.path();
                if p.is_dir() {
                    dirs.push(p);
                    continue;
                }
                if p.file_name().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("tool.json")).unwrap_or(false) {
                    let parent = p.parent().map(|x| x.to_path_buf());
                    let content = tokio::fs::read_to_string(&p).await?;
                    let manifest: ToolManifest = serde_json::from_str(&content)?;
                    let meta = manifest.into_plugin_metadata(parent)?;
                    self.register_plugin(meta).await?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    async fn load_plugin_metadata_from_file(&self, path: &Path) -> Result<PluginMetadata> {
        let contents = tokio::fs::read_to_string(path).await?;
        let metadata: PluginMetadata = serde_json::from_str(&contents)?;
        Ok(metadata)
    }

    async fn load_plugin_configuration_from_file(
        &self,
        plugin_id: &str,
    ) -> Result<PluginConfiguration> {
        let config_path = self.config_directory.join(format!("{}.json", plugin_id));
        let contents = tokio::fs::read_to_string(config_path).await?;
        let config: PluginConfiguration = serde_json::from_str(&contents)?;
        Ok(config)
    }

    /// Clean up plugin files
    async fn cleanup_plugin_files(&self, metadata: &PluginMetadata) -> Result<()> {
        // Remove metadata file
        let metadata_path = self.plugin_directory.join(format!("{}.json", metadata.id));
        if metadata_path.exists() {
            tokio::fs::remove_file(metadata_path).await?;
        }

        // Remove configuration file
        let config_path = self.config_directory.join(format!("{}.json", metadata.id));
        if config_path.exists() {
            tokio::fs::remove_file(config_path).await?;
        }

        // Remove plugin installation directory if exists
        if let Some(ref install_path) = metadata.installation_path {
            if install_path.exists() {
                tokio::fs::remove_dir_all(install_path).await?;
            }
        }

        Ok(())
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Filter for plugin queries
#[derive(Debug, Clone)]
pub struct PluginFilter {
    pub plugin_type: Option<PluginType>,
    pub state: Option<PluginState>,
    pub security_level: Option<SecurityLevel>,
    pub keywords: Vec<String>,
}

impl PluginFilter {
    pub fn matches(&self, plugin: &PluginMetadata) -> bool {
        if let Some(ref plugin_type) = self.plugin_type {
            if &plugin.plugin_type != plugin_type {
                return false;
            }
        }

        if let Some(ref state) = self.state {
            if &plugin.state != state {
                return false;
            }
        }

        if let Some(ref security_level) = self.security_level {
            if &plugin.security_level != security_level {
                return false;
            }
        }

        if !self.keywords.is_empty() {
            let has_keyword = self.keywords.iter().any(|keyword| {
                plugin.keywords.iter().any(|pk| pk.contains(keyword))
                    || plugin.name.to_lowercase().contains(&keyword.to_lowercase())
                    || plugin
                        .description
                        .to_lowercase()
                        .contains(&keyword.to_lowercase())
            });

            if !has_keyword {
                return false;
            }
        }

        true
    }
}

/// Main plugin manager
pub struct PluginManager {
    registry: PluginRegistry,
    active_plugins: Arc<RwLock<HashMap<String, Arc<dyn PluginInstance>>>>,
    plugin_loaders: Arc<RwLock<HashMap<PluginType, Box<dyn PluginLoader>>>>,
}

/// Plugin instance trait
#[async_trait::async_trait]
pub trait PluginInstance: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn metadata(&self) -> &PluginMetadata;
    fn is_loaded(&self) -> bool;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn reload(&mut self) -> Result<()>;
    async fn health_check(&self) -> Result<()>;
}

/// Plugin loader trait for different plugin types
#[async_trait::async_trait]
pub trait PluginLoader: Send + Sync {
    async fn load_plugin(
        &self,
        metadata: &PluginMetadata,
        config: &PluginConfiguration,
    ) -> Result<Box<dyn PluginInstance>>;

    fn supports_type(&self) -> PluginType;
    async fn unload_plugin(&self, instance: Box<dyn PluginInstance>) -> Result<()>;
}

impl PluginManager {
    pub fn new(plugin_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            registry: PluginRegistry::new(plugin_dir, config_dir),
            active_plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_loaders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin loader for a specific plugin type
    pub async fn register_loader(&self, loader: Box<dyn PluginLoader>) {
        let plugin_type = loader.supports_type();
        let mut loaders = self.plugin_loaders.write().await;
        loaders.insert(plugin_type.clone(), loader);
        info!("🔌 Registered plugin loader for: {:?}", plugin_type);
    }

    /// Load and activate a plugin
    pub async fn load_plugin(&self, plugin_id: &str) -> Result<()> {
        // Get plugin metadata and configuration
        let metadata = self
            .registry
            .get_plugin(plugin_id)
            .await
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;

        let config = self
            .registry
            .get_plugin_configuration(plugin_id)
            .await
            .ok_or_else(|| anyhow!("Plugin configuration not found: {}", plugin_id))?;

        // Check dependencies
        let missing_deps = self.registry.check_dependencies(plugin_id).await?;
        if !missing_deps.is_empty() {
            return Err(anyhow!("Missing dependencies: {:?}", missing_deps));
        }

        // Update state to loading
        self.registry
            .update_plugin_state(plugin_id, PluginState::Loading)
            .await?;

        // Get appropriate loader
        let loaders = self.plugin_loaders.read().await;
        let loader = loaders.get(&metadata.plugin_type).ok_or_else(|| {
            anyhow!(
                "No loader available for plugin type: {:?}",
                metadata.plugin_type
            )
        })?;

        // Load plugin instance
        match loader.load_plugin(&metadata, &config).await {
            Ok(instance) => {
                // Start the plugin
                let mut instance = instance;
                instance.start().await?;

                // Store active instance
                {
                    let mut active = self.active_plugins.write().await;
                    active.insert(plugin_id.to_string(), Arc::from(instance));
                }

                // Update state
                self.registry
                    .update_plugin_state(plugin_id, PluginState::Active)
                    .await?;

                info!("✅ Loaded and activated plugin: {}", metadata.name);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to load plugin: {}", e);
                self.registry
                    .update_plugin_state(plugin_id, PluginState::Error(error_msg.clone()))
                    .await?;
                Err(anyhow!(error_msg))
            }
        }
    }

    /// Unload and deactivate a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        // Get active instance
        let _instance = {
            let mut active = self.active_plugins.write().await;
            active
                .remove(plugin_id)
                .ok_or_else(|| anyhow!("Plugin not active: {}", plugin_id))?
        };

        // Update state to unloading
        self.registry
            .update_plugin_state(plugin_id, PluginState::Unloading)
            .await?;

        // Get plugin metadata for loader type
        let metadata = self
            .registry
            .get_plugin(plugin_id)
            .await
            .ok_or_else(|| anyhow!("Plugin metadata not found: {}", plugin_id))?;

        // Get appropriate loader for cleanup
        let loaders = self.plugin_loaders.read().await;
        if let Some(_loader) = loaders.get(&metadata.plugin_type) {
            // Convert Arc to Box for unloading
            // This is a simplified approach - in production you'd need better instance management
            // loader.unload_plugin(instance_box).await?;
        }

        // Update state
        self.registry
            .update_plugin_state(plugin_id, PluginState::Installed)
            .await?;

        info!("🔄 Unloaded plugin: {}", metadata.name);
        Ok(())
    }

    /// Get reference to plugin registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get list of active plugins
    pub async fn get_active_plugins(&self) -> Vec<String> {
        let active = self.active_plugins.read().await;
        active.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_registry() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("plugins");
        let config_dir = temp_dir.path().join("configs");

        tokio::fs::create_dir_all(&plugin_dir).await.unwrap();
        tokio::fs::create_dir_all(&config_dir).await.unwrap();

        let registry = PluginRegistry::new(plugin_dir, config_dir);

        let metadata = PluginMetadata {
            id: "test_plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: PluginVersion::new(1, 0, 0),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            homepage: None,
            repository: None,
            license: "MIT".to_string(),
            keywords: vec!["test".to_string()],
            plugin_type: PluginType::Wasm,
            entry_point: "test.wasm".to_string(),
            configuration_schema: serde_json::json!({}),
            default_config: serde_json::json!({}),
            runtime_requirements: RuntimeRequirements::default(),
            dependencies: Vec::new(),
            required_permissions: ToolPermissions::default(),
            security_level: SecurityLevel::Safe,
            signed: false,
            signature: None,
            installed_at: None,
            last_updated: None,
            installation_path: None,
            state: PluginState::Uninstalled,
            load_count: 0,
            error_count: 0,
            last_error: None,
        };

        // Test registration
        assert!(registry.register_plugin(metadata).await.is_ok());

        // Test retrieval
        let retrieved = registry.get_plugin("test_plugin").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Plugin");
    }

    #[test]
    fn test_plugin_version() {
        let v1 = PluginVersion::new(1, 2, 3);
        let v2 = PluginVersion::new(1, 2, 0);
        let v3 = PluginVersion::new(2, 0, 0);

        assert!(v1.is_compatible(&v2));
        assert!(!v1.is_compatible(&v3));

        assert_eq!(v1.to_string(), "1.2.3");
    }

    #[tokio::test]
    async fn test_manifest_loader_registers_plugin() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("plugins");
        let config_dir = temp_dir.path().join("configs");
        tokio::fs::create_dir_all(plugin_dir.join("p1")).await.unwrap();
        tokio::fs::create_dir_all(&config_dir).await.unwrap();

        // Write tool.json
        let manifest = serde_json::json!({
            "id": "p1",
            "name": "Plugin One",
            "version": "1.0.0",
            "description": "Test plugin",
            "author": "ACME",
            "plugin_type": "external",
            "entry_point": "run.sh",
            "permissions": {"net": {"mode": "restricted", "allowed_hosts": ["example.com"]}},
            "configuration_schema": {}
        });
        let mp = plugin_dir.join("p1").join("tool.json");
        tokio::fs::write(&mp, serde_json::to_string_pretty(&manifest).unwrap()).await.unwrap();

        let registry = PluginRegistry::new(plugin_dir.clone(), config_dir);
        let loaded = registry.load_manifests_from_directory().await.unwrap();
        assert_eq!(loaded, 1);
        let meta = registry.get_plugin("p1").await.expect("plugin registered");
        assert_eq!(meta.name, "Plugin One");
        // Check permissions mapping
        match &meta.required_permissions.network {
            crate::registry::NetworkPermissions::Restricted { allowed_hosts } => {
                assert!(allowed_hosts.contains(&"example.com".into()));
            }
            _ => panic!("expected restricted network"),
        }
        assert!(meta.installation_path.is_some());
    }
}
