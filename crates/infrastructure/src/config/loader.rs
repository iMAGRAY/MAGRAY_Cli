use anyhow::{Context, Result};
use domain::config::*;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::fs;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum ConfigSource {
    File(PathBuf),
    Environment,
    Default,
}

pub struct ConfigLoader {
    config_paths: Vec<PathBuf>,
    env_prefix: String,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            config_paths: Self::default_config_paths(),
            env_prefix: "MAGRAY_".to_string(),
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.config_paths.insert(0, path);
        self
    }

    pub fn with_env_prefix(mut self, prefix: String) -> Self {
        self.env_prefix = prefix;
        self
    }

    fn default_config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Current directory
        paths.push(PathBuf::from(".magrayrc"));
        paths.push(PathBuf::from(".magrayrc.toml"));
        paths.push(PathBuf::from(".magrayrc.json"));
        paths.push(PathBuf::from("magray.toml"));
        paths.push(PathBuf::from("magray.json"));

        // User home directory
        if let Some(home_dir) = dirs::home_dir() {
            paths.push(home_dir.join(".magrayrc"));
            paths.push(home_dir.join(".magrayrc.toml"));
            paths.push(home_dir.join(".magrayrc.json"));
            paths.push(home_dir.join(".config").join("magray").join("config.toml"));
            paths.push(home_dir.join(".config").join("magray").join("config.json"));
        }

        // System config directory
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("magray").join("config.toml"));
            paths.push(config_dir.join("magray").join("config.json"));
        }

        paths
    }

    pub async fn load(&self) -> Result<MagrayConfig> {
        // Detect active profile from environment
        let profile = self.detect_profile();
        debug!("Detected profile: {}", profile.name());

        // Start with default config
        let mut config = MagrayConfig {
            profile: profile.clone(),
            ..MagrayConfig::default()
        };
        debug!(
            "Starting with default configuration for profile: {}",
            profile.name()
        );

        // Load base configuration first
        config = self.load_base_config(config).await?;

        // Load profile-specific configuration
        config = self.load_profile_config(config, &profile).await?;

        // Override with environment variables
        config = self.apply_env_overrides(config)?;

        // Apply profile configuration
        if let Some(profile_config) = config.profile_config.clone() {
            config.apply_profile(&profile_config);
            debug!("Applied profile configuration: {}", profile.name());
        }

        Ok(config)
    }

    /// Detect the active profile from environment variables
    pub fn detect_profile(&self) -> Profile {
        env::var(format!("{}ENV", self.env_prefix))
            .or_else(|_| env::var("MAGRAY_ENV"))
            .map(|env_val| Profile::from_str(&env_val).unwrap_or_default())
            .unwrap_or_default()
    }

    /// Load base configuration from standard paths
    async fn load_base_config(&self, mut config: MagrayConfig) -> Result<MagrayConfig> {
        // Try to load base config from files
        for path in &self.config_paths {
            if path.exists() {
                match self.load_file(path).await {
                    Ok(file_config) => {
                        info!("Loaded base configuration from: {}", path.display());
                        config = self.merge_configs(config, file_config);
                        break; // Use first found config file
                    }
                    Err(e) => {
                        warn!("Failed to load base config from {}: {}", path.display(), e);
                    }
                }
            }
        }
        Ok(config)
    }

    /// Load profile-specific configuration
    async fn load_profile_config(
        &self,
        mut config: MagrayConfig,
        profile: &Profile,
    ) -> Result<MagrayConfig> {
        let profile_paths = self.get_profile_config_paths(profile);

        for path in profile_paths {
            debug!("Checking profile path: {}", path.display());
            if path.exists() {
                debug!("Found profile file: {}", path.display());
                match self.load_profile_file(&path).await {
                    Ok(profile_config) => {
                        info!("Loaded profile configuration from: {}", path.display());
                        debug!(
                            "Loaded profile config: permissive_mode = {}",
                            profile_config.security.permissive_mode
                        );
                        config.profile_config = Some(profile_config);
                        break; // Use first found profile config
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load profile config from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        // If no profile config file found, use built-in defaults
        if config.profile_config.is_none() {
            let profile_config = match profile {
                Profile::Dev => ProfileConfig::dev(),
                Profile::Prod => ProfileConfig::prod(),
                Profile::Custom(_) => ProfileConfig::default(),
            };
            config.profile_config = Some(profile_config);
            debug!(
                "Using built-in profile configuration for: {}",
                profile.name()
            );
        }

        Ok(config)
    }

    /// Get profile-specific configuration file paths  
    fn get_profile_config_paths(&self, profile: &Profile) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        let profile_name = profile.name();

        // Current directory
        paths.push(PathBuf::from(format!("configs/{profile_name}.toml")));
        paths.push(PathBuf::from(format!("configs/{profile_name}.json")));
        paths.push(PathBuf::from(format!("magray.{profile_name}.toml")));
        paths.push(PathBuf::from(format!("magray.{profile_name}.json")));

        // User config directory
        if let Some(config_dir) = dirs::config_dir() {
            let magray_config = config_dir.join("magray");
            paths.push(
                magray_config
                    .join("profiles")
                    .join(format!("{profile_name}.toml")),
            );
            paths.push(
                magray_config
                    .join("profiles")
                    .join(format!("{profile_name}.json")),
            );
        }

        // User home directory
        if let Some(home_dir) = dirs::home_dir() {
            let magray_config = home_dir.join(".config").join("magray");
            paths.push(
                magray_config
                    .join("profiles")
                    .join(format!("{profile_name}.toml")),
            );
            paths.push(
                magray_config
                    .join("profiles")
                    .join(format!("{profile_name}.json")),
            );
        }

        paths
    }

    /// Load profile-specific configuration file
    async fn load_profile_file(&self, path: &Path) -> Result<ProfileConfig> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read profile config file")?;

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension {
            "toml" | "" => toml::from_str(&content).context("Failed to parse TOML profile config"),
            "json" => serde_json::from_str(&content).context("Failed to parse JSON profile config"),
            _ => {
                // Try TOML first, then JSON
                toml::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content))
                    .context("Failed to parse profile config file")
            }
        }
    }

    async fn load_file(&self, path: &Path) -> Result<MagrayConfig> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read config file")?;

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension {
            "toml" | "" => toml::from_str(&content).context("Failed to parse TOML config"),
            "json" => serde_json::from_str(&content).context("Failed to parse JSON config"),
            _ => {
                // Try TOML first, then JSON
                toml::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content))
                    .context("Failed to parse config file")
            }
        }
    }

    fn merge_configs(&self, _base: MagrayConfig, override_config: MagrayConfig) -> MagrayConfig {
        // For now, just replace the entire config
        // In a real implementation, you'd do a deep merge
        override_config
    }

    fn apply_env_overrides(&self, mut config: MagrayConfig) -> Result<MagrayConfig> {
        // AI provider settings
        if let Ok(provider) = env::var(format!("{}AI_PROVIDER", self.env_prefix)) {
            config.ai.default_provider = provider;
        }

        if let Ok(api_key) = env::var(format!("{}OPENAI_API_KEY", self.env_prefix)) {
            config
                .ai
                .providers
                .entry("openai".to_string())
                .or_insert_with(|| ProviderConfig {
                    provider_type: ProviderType::OpenAI,
                    api_key: None,
                    api_base: None,
                    model: None,
                    model_path: None,
                    options: Default::default(),
                })
                .api_key = Some(api_key);
        }

        if let Ok(api_key) = env::var(format!("{}ANTHROPIC_API_KEY", self.env_prefix)) {
            config
                .ai
                .providers
                .entry("anthropic".to_string())
                .or_insert_with(|| ProviderConfig {
                    provider_type: ProviderType::Anthropic,
                    api_key: None,
                    api_base: None,
                    model: None,
                    model_path: None,
                    options: Default::default(),
                })
                .api_key = Some(api_key);
        }

        // Memory settings
        if let Ok(backend) = env::var(format!("{}MEMORY_BACKEND", self.env_prefix)) {
            config.memory.backend = match backend.to_lowercase().as_str() {
                "sqlite" => MemoryBackend::SQLite,
                "inmemory" | "in-memory" => MemoryBackend::InMemory,
                "hybrid" => MemoryBackend::Hybrid,
                _ => config.memory.backend,
            };
        }

        if let Ok(cache_size) = env::var(format!("{}CACHE_SIZE_MB", self.env_prefix)) {
            if let Ok(size) = cache_size.parse() {
                config.memory.cache_size_mb = size;
            }
        }

        // Logging settings
        if let Ok(log_level) = env::var(format!("{}LOG_LEVEL", self.env_prefix)) {
            config.logging.level = log_level;
        }

        if let Ok(log_file) = env::var(format!("{}LOG_FILE", self.env_prefix)) {
            config.logging.file_enabled = true;
            config.logging.file_path = Some(PathBuf::from(log_file));
        }

        // Performance settings
        if let Ok(threads) = env::var(format!("{}WORKER_THREADS", self.env_prefix)) {
            if let Ok(num) = threads.parse() {
                config.performance.worker_threads = num;
            }
        }

        if let Ok(enable_gpu) = env::var(format!("{}ENABLE_GPU", self.env_prefix)) {
            config.performance.enable_gpu = enable_gpu.to_lowercase() == "true"
                || enable_gpu == "1"
                || enable_gpu.to_lowercase() == "yes";
        }

        // Paths
        if let Ok(data_dir) = env::var(format!("{}DATA_DIR", self.env_prefix)) {
            config.paths.data_dir = Some(PathBuf::from(data_dir));
        }

        if let Ok(cache_dir) = env::var(format!("{}CACHE_DIR", self.env_prefix)) {
            config.paths.cache_dir = Some(PathBuf::from(cache_dir));
        }

        if let Ok(models_dir) = env::var(format!("{}MODELS_DIR", self.env_prefix)) {
            config.paths.models_dir = Some(PathBuf::from(models_dir));
        }

        Ok(config)
    }

    pub async fn save_config(&self, config: &MagrayConfig, path: &Path) -> Result<()> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("toml");

        let content = match extension {
            "json" => serde_json::to_string_pretty(config)?,
            _ => toml::to_string_pretty(config)?,
        };

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;
        info!("Configuration saved to: {}", path.display());

        Ok(())
    }

    pub fn generate_example_config() -> String {
        let config = MagrayConfig {
            profile: Profile::Dev,
            profile_config: None,
            ai: AiConfig {
                default_provider: "openai".to_string(),
                providers: {
                    let mut providers = std::collections::HashMap::new();
                    providers.insert(
                        "openai".to_string(),
                        ProviderConfig {
                            provider_type: ProviderType::OpenAI,
                            api_key: Some("your-api-key-here".to_string()),
                            api_base: Some("https://api.openai.com/v1".to_string()),
                            model: Some("gpt-4".to_string()),
                            model_path: None,
                            options: Default::default(),
                        },
                    );
                    providers.insert(
                        "local".to_string(),
                        ProviderConfig {
                            provider_type: ProviderType::Local,
                            api_key: None,
                            api_base: None,
                            model: Some("qwen3-0.6b".to_string()),
                            model_path: Some(PathBuf::from("./models/qwen3-0.6b.onnx")),
                            options: Default::default(),
                        },
                    );
                    providers
                },
                fallback_chain: vec!["openai".to_string(), "local".to_string()],
                max_tokens: 4096,
                temperature: 0.7,
                retry_config: RetryConfig::default(),
            },
            memory: MemoryConfig {
                backend: MemoryBackend::SQLite,
                hnsw: HnswConfig::default(),
                embedding: EmbeddingConfig {
                    model: "qwen3-0.6b".to_string(),
                    dimension: 384,
                    use_gpu: false,
                    batch_size: 32,
                },
                cache_size_mb: 256,
                flush_interval_sec: 60,
                persistence: PersistenceConfig {
                    enabled: true,
                    path: Some(PathBuf::from("./data/memory.db")),
                    auto_save_interval_sec: 300,
                },
            },
            mcp: McpConfig {
                enabled: true,
                servers: vec![McpServerConfig {
                    name: "example-server".to_string(),
                    url: "ws://localhost:3000".to_string(),
                    auth_token: None,
                    capabilities: vec!["tools".to_string(), "resources".to_string()],
                }],
                timeout_sec: 30,
                auto_discovery: true,
            },
            plugins: PluginsConfig {
                enabled: true,
                plugin_dir: Some(PathBuf::from("./plugins")),
                auto_load: vec!["git".to_string(), "web".to_string()],
                sandbox_enabled: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file_enabled: true,
                file_path: Some(PathBuf::from("./logs/magray.log")),
                structured: false,
                max_size_mb: 100,
            },
            paths: PathsConfig {
                data_dir: Some(PathBuf::from("./data")),
                cache_dir: Some(PathBuf::from("./cache")),
                models_dir: Some(PathBuf::from("./models")),
                logs_dir: Some(PathBuf::from("./logs")),
            },
            performance: PerformanceConfig {
                worker_threads: 4,
                max_concurrent_requests: 10,
                enable_gpu: false,
                memory_limit_mb: 1024,
            },
        };

        toml::to_string_pretty(&config)
            .unwrap_or_else(|_| "Failed to generate example config".to_string())
    }

    /// Generate profile template configuration
    pub fn generate_profile_config(profile: &Profile) -> String {
        let profile_config = match profile {
            Profile::Dev => ProfileConfig::dev(),
            Profile::Prod => ProfileConfig::prod(),
            Profile::Custom(_) => ProfileConfig::default(),
        };

        toml::to_string_pretty(&profile_config)
            .unwrap_or_else(|_| "Failed to generate profile config".to_string())
    }

    /// Save profile configuration to file
    pub async fn save_profile_config(
        &self,
        profile: &Profile,
        profile_config: &ProfileConfig,
    ) -> Result<()> {
        // Create profiles directory if it doesn't exist
        let profiles_dir = PathBuf::from("configs");
        fs::create_dir_all(&profiles_dir).await?;

        let profile_path = profiles_dir.join(format!("{}.toml", profile.name()));
        let content = toml::to_string_pretty(profile_config)?;

        fs::write(&profile_path, content).await?;
        info!("Profile configuration saved to: {}", profile_path.display());

        Ok(())
    }

    /// Switch to different profile at runtime
    pub async fn switch_profile(
        &self,
        mut config: MagrayConfig,
        new_profile: Profile,
    ) -> Result<MagrayConfig> {
        info!(
            "Switching from profile '{}' to '{}'",
            config.profile.name(),
            new_profile.name()
        );

        // Update profile
        config.profile = new_profile.clone();

        // Load new profile configuration
        config = self.load_profile_config(config, &new_profile).await?;

        // Apply profile configuration
        if let Some(profile_config) = config.profile_config.clone() {
            config.apply_profile(&profile_config);
            debug!("Applied new profile configuration: {}", new_profile.name());
        }

        Ok(config)
    }

    /// Validate profile configuration against schema
    pub fn validate_profile_config(&self, profile_config: &ProfileConfig) -> Result<()> {
        // Validate security config
        let valid_policy_modes = ["ask", "allow", "deny"];
        if !valid_policy_modes.contains(&profile_config.security.default_policy_mode.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid policy mode: {}. Must be one of: {:?}",
                profile_config.security.default_policy_mode,
                valid_policy_modes
            ));
        }

        let valid_risk_levels = ["low", "medium", "high"];
        if !valid_risk_levels.contains(&profile_config.security.risk_level.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid risk level: {}. Must be one of: {:?}",
                profile_config.security.risk_level,
                valid_risk_levels
            ));
        }

        // Validate tools config
        let valid_whitelist_modes = ["expanded", "minimal", "custom"];
        if !valid_whitelist_modes.contains(&profile_config.tools.whitelist_mode.as_str()) {
            return Err(anyhow::anyhow!(
                "Invalid whitelist mode: {}. Must be one of: {:?}",
                profile_config.tools.whitelist_mode,
                valid_whitelist_modes
            ));
        }

        // Validate performance config
        if let Some(memory_limit) = profile_config.performance.memory_limit_override_mb {
            if !(64..=8192).contains(&memory_limit) {
                return Err(anyhow::anyhow!(
                    "Memory limit {} MB is out of valid range (64-8192 MB)",
                    memory_limit
                ));
            }
        }

        // Validate logging config
        if let Some(ref level) = profile_config.logging.level_override {
            let valid_levels = ["error", "warn", "info", "debug", "trace"];
            if !valid_levels.contains(&level.as_str()) {
                return Err(anyhow::anyhow!(
                    "Invalid log level: {}. Must be one of: {:?}",
                    level,
                    valid_levels
                ));
            }
        }

        Ok(())
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}
