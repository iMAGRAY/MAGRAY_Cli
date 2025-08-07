//! Configuration Loader
//!
//! This module provides flexible configuration loading from various sources:
//! - Environment variables
//! - Configuration files (TOML, JSON, YAML)
//! - Command-line arguments
//! - Preset combinations
//!
//! Features:
//! - Hierarchical configuration merging
//! - Environment variable interpolation
//! - Validation on load
//! - Hot-reload support (optional)

use anyhow::Result;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::{
    config_presets::ConfigPresets,
    config_validation::ConfigurationValidator,
    unified_config::{Environment, UnifiedDIConfiguration},
};

/// Configuration loader with support for multiple sources
pub struct ConfigurationLoader {
    /// Configuration search paths
    search_paths: Vec<PathBuf>,
    /// Environment variable prefix
    env_prefix: String,
    /// Validation enabled
    validate_on_load: bool,
    /// Validator instance
    validator: ConfigurationValidator,
}

impl Default for ConfigurationLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationLoader {
    /// Create a new configuration loader with default settings
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // Add standard configuration paths
        if let Some(config_dir) = dirs::config_dir() {
            search_paths.push(config_dir.join("magray"));
        }

        // Add current directory
        search_paths.push(PathBuf::from("."));

        // Add system configuration directory
        search_paths.push(PathBuf::from("/etc/magray"));

        Self {
            search_paths,
            env_prefix: "MAGRAY".to_string(),
            validate_on_load: true,
            validator: ConfigurationValidator::new(),
        }
    }

    /// Add a search path for configuration files
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.search_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Set environment variable prefix (default: "MAGRAY")
    pub fn env_prefix<S: Into<String>>(&mut self, prefix: S) -> &mut Self {
        self.env_prefix = prefix.into();
        self
    }

    /// Enable/disable validation on load
    pub fn validate(&mut self, enabled: bool) -> &mut Self {
        self.validate_on_load = enabled;
        self
    }

    /// Load configuration from multiple sources with priority order:
    /// 1. Command line arguments (highest priority)
    /// 2. Environment variables
    /// 3. Configuration files
    /// 4. Preset defaults (lowest priority)
    pub fn load(&self) -> Result<UnifiedDIConfiguration> {
        let args = self.parse_command_line_args()?;
        self.load_with_args(args)
    }

    /// Load configuration with explicit command-line arguments
    pub fn load_with_args(&self, args: ConfigArgs) -> Result<UnifiedDIConfiguration> {
        // Start with preset or default
        let mut config = if let Some(preset) = &args.preset {
            ConfigPresets::from_preset_name(preset)?
        } else {
            ConfigPresets::auto_detect()
        };

        // Load and merge configuration file if specified
        if let Some(config_file) = &args.config_file {
            let file_config = self.load_from_file(config_file)?;
            config = self.merge_configurations(config, file_config)?;
        } else {
            // Try to find configuration file automatically
            if let Some(found_config) = self.find_configuration_file()? {
                let file_config = self.load_from_file(&found_config)?;
                config = self.merge_configurations(config, file_config)?;
            }
        }

        // Apply environment variables
        self.apply_environment_variables(&mut config)?;

        // Apply command-line overrides
        self.apply_command_line_args(&mut config, &args)?;

        // Validate if enabled
        if self.validate_on_load {
            let report = self.validator.validate(&config)?;
            if !report.is_valid() {
                return Err(anyhow::anyhow!(
                    "Configuration validation failed: {}",
                    self.format_validation_errors(&report.errors)
                ));
            }

            // Log warnings
            for warning in &report.warnings {
                eprintln!("Config Warning: {}", warning);
            }
        }

        Ok(config)
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<UnifiedDIConfiguration> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;

        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => self.load_from_toml(&content),
            Some("json") => self.load_from_json(&content),
            Some("yaml") | Some("yml") => self.load_from_yaml(&content),
            _ => Err(anyhow::anyhow!(
                "Unsupported configuration file format: {:?}",
                path.extension()
            )),
        }
    }

    /// Load configuration from TOML string
    pub fn load_from_toml(&self, content: &str) -> Result<UnifiedDIConfiguration> {
        let config: UnifiedDIConfiguration = toml::from_str(content)?;
        Ok(config)
    }

    /// Load configuration from JSON string
    pub fn load_from_json(&self, content: &str) -> Result<UnifiedDIConfiguration> {
        let config: UnifiedDIConfiguration = serde_json::from_str(content)?;
        Ok(config)
    }

    /// Load configuration from YAML string
    pub fn load_from_yaml(&self, content: &str) -> Result<UnifiedDIConfiguration> {
        let config: UnifiedDIConfiguration = serde_yaml::from_str(content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(
        &self,
        config: &UnifiedDIConfiguration,
        path: P,
    ) -> Result<()> {
        let path = path.as_ref();

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::to_string_pretty(config)?,
            Some("json") => serde_json::to_string_pretty(config)?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(config)?,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file format: {:?}",
                    path.extension()
                ))
            }
        };

        fs::write(path, content)?;
        Ok(())
    }

    /// Find configuration file automatically
    fn find_configuration_file(&self) -> Result<Option<PathBuf>> {
        let filenames = [
            "magray.toml",
            "magray.json",
            "magray.yaml",
            "magray.yml",
            ".magray.toml",
            "config.toml",
        ];

        for search_path in &self.search_paths {
            for filename in &filenames {
                let file_path = search_path.join(filename);
                if file_path.exists() && file_path.is_file() {
                    return Ok(Some(file_path));
                }
            }
        }

        Ok(None)
    }

    /// Apply environment variables to configuration
    fn apply_environment_variables(&self, config: &mut UnifiedDIConfiguration) -> Result<()> {
        let prefix = format!("{}_", self.env_prefix);

        for (key, value) in env::vars() {
            if !key.starts_with(&prefix) {
                continue;
            }

            let config_key = key.trim_start_matches(&prefix);
            self.apply_env_var(config, config_key, &value)?;
        }

        // Apply built-in overrides
        config.apply_env_overrides()?;

        Ok(())
    }

    /// Apply a single environment variable
    fn apply_env_var(
        &self,
        config: &mut UnifiedDIConfiguration,
        key: &str,
        value: &str,
    ) -> Result<()> {
        match key.to_uppercase().as_str() {
            "ENVIRONMENT" => match value.to_lowercase().as_str() {
                "production" => config.environment = Environment::Production,
                "development" => config.environment = Environment::Development,
                "test" => config.environment = Environment::Test,
                "minimal" => config.environment = Environment::Minimal,
                custom => config.environment = Environment::Custom(custom.to_string()),
            },
            "DATA_DIR" => {
                config.core.data_dir = PathBuf::from(value);
                config.memory.database.db_path = config.core.data_dir.join("memory.db");
            }
            "TEMP_DIR" => {
                config.core.temp_dir = PathBuf::from(value);
            }
            "LOG_LEVEL" => {
                config.core.log_level = value.to_lowercase();
            }
            "THREADS" => {
                config.core.thread_pool_size = value.parse()?;
            }
            "MAX_MEMORY_MB" => {
                config.core.max_memory_mb = value.parse()?;
            }
            "DEV_MODE" => {
                config.core.dev_mode = value.parse().unwrap_or(false);
            }
            "AI_EMBEDDING_MODEL" => {
                config.ai.embedding.model_name = value.to_string();
            }
            "AI_EMBEDDING_BATCH_SIZE" => {
                config.ai.embedding.batch_size = value.parse()?;
            }
            "AI_GPU_ENABLED" => {
                let enabled = value.parse().unwrap_or(false);
                config.ai.embedding.use_gpu = enabled;
                config.ai.reranking.use_gpu = enabled;
                config.features.gpu_acceleration = enabled;
            }
            "HNSW_MAX_CONNECTIONS" => {
                config.memory.hnsw.max_connections = value.parse()?;
            }
            "HNSW_EF_SEARCH" => {
                config.memory.hnsw.ef_search = value.parse()?;
            }
            "BATCH_SIZE" => {
                config.memory.batch.max_batch_size = value.parse()?;
            }
            "CACHE_SIZE" => {
                config.memory.cache.base.max_cache_size = value.parse()?;
            }
            "DB_POOL_SIZE" => {
                config.memory.database.pool_size = value.parse()?;
            }
            _ => {
                // Store unknown variables for custom processing
                config
                    .env_overrides
                    .insert(key.to_string(), value.to_string());
            }
        }

        Ok(())
    }

    /// Apply command-line arguments to configuration
    fn apply_command_line_args(
        &self,
        config: &mut UnifiedDIConfiguration,
        args: &ConfigArgs,
    ) -> Result<()> {
        if let Some(env) = &args.environment {
            match env.to_lowercase().as_str() {
                "production" => config.environment = Environment::Production,
                "development" => config.environment = Environment::Development,
                "test" => config.environment = Environment::Test,
                "minimal" => config.environment = Environment::Minimal,
                custom => config.environment = Environment::Custom(custom.to_string()),
            }
        }

        if let Some(data_dir) = &args.data_dir {
            config.core.data_dir = data_dir.clone();
            config.memory.database.db_path = data_dir.join("memory.db");
        }

        if let Some(log_level) = &args.log_level {
            config.core.log_level = log_level.clone();
        }

        if let Some(threads) = args.threads {
            config.core.thread_pool_size = threads;
        }

        if let Some(max_memory) = args.max_memory_mb {
            config.core.max_memory_mb = max_memory;
        }

        if let Some(gpu_enabled) = args.gpu {
            config.features.gpu_acceleration = gpu_enabled;
            config.ai.embedding.use_gpu = gpu_enabled;
            config.ai.reranking.use_gpu = gpu_enabled;
        }

        if args.dev_mode {
            config.core.dev_mode = true;
        }

        Ok(())
    }

    /// Parse command-line arguments
    fn parse_command_line_args(&self) -> Result<ConfigArgs> {
        let args: Vec<String> = env::args().collect();
        let mut config_args = ConfigArgs::default();

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--config" | "-c" => {
                    if i + 1 < args.len() {
                        config_args.config_file = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --config"));
                    }
                }
                "--preset" | "-p" => {
                    if i + 1 < args.len() {
                        config_args.preset = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --preset"));
                    }
                }
                "--environment" | "-e" => {
                    if i + 1 < args.len() {
                        config_args.environment = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --environment"));
                    }
                }
                "--data-dir" => {
                    if i + 1 < args.len() {
                        config_args.data_dir = Some(PathBuf::from(&args[i + 1]));
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --data-dir"));
                    }
                }
                "--log-level" => {
                    if i + 1 < args.len() {
                        config_args.log_level = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --log-level"));
                    }
                }
                "--threads" => {
                    if i + 1 < args.len() {
                        config_args.threads = Some(args[i + 1].parse()?);
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --threads"));
                    }
                }
                "--max-memory" => {
                    if i + 1 < args.len() {
                        config_args.max_memory_mb = Some(args[i + 1].parse()?);
                        i += 2;
                    } else {
                        return Err(anyhow::anyhow!("Missing value for --max-memory"));
                    }
                }
                "--gpu" => {
                    config_args.gpu = Some(true);
                    i += 1;
                }
                "--no-gpu" => {
                    config_args.gpu = Some(false);
                    i += 1;
                }
                "--dev" => {
                    config_args.dev_mode = true;
                    i += 1;
                }
                "--validate" => {
                    config_args.validate = true;
                    i += 1;
                }
                _ => i += 1,
            }
        }

        Ok(config_args)
    }

    /// Merge two configurations, with higher priority config overriding lower
    fn merge_configurations(
        &self,
        _base: UnifiedDIConfiguration,
        override_config: UnifiedDIConfiguration,
    ) -> Result<UnifiedDIConfiguration> {
        // This is a simplified merge - in production, you'd want a more sophisticated merging strategy
        // For now, we'll just use the override config entirely if it's provided
        Ok(override_config)
    }

    /// Format validation errors for user display
    fn format_validation_errors(
        &self,
        errors: &[crate::di::unified_config::ValidationError],
    ) -> String {
        errors
            .iter()
            .map(|e| format!("  - {}", e))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Command-line arguments structure
#[derive(Debug, Default)]
pub struct ConfigArgs {
    pub config_file: Option<PathBuf>,
    pub preset: Option<String>,
    pub environment: Option<String>,
    pub data_dir: Option<PathBuf>,
    pub log_level: Option<String>,
    pub threads: Option<usize>,
    pub max_memory_mb: Option<usize>,
    pub gpu: Option<bool>,
    pub dev_mode: bool,
    pub validate: bool,
}

/// Configuration file template generator
pub struct ConfigTemplateGenerator;

impl ConfigTemplateGenerator {
    /// Generate a template configuration file
    pub fn generate_template(format: &str, preset: Option<&str>) -> Result<String> {
        let config = if let Some(preset_name) = preset {
            ConfigPresets::from_preset_name(preset_name)?
        } else {
            UnifiedDIConfiguration::development()
        };

        match format.to_lowercase().as_str() {
            "toml" => Ok(toml::to_string_pretty(&config)?),
            "json" => Ok(serde_json::to_string_pretty(&config)?),
            "yaml" | "yml" => Ok(serde_yaml::to_string(&config)?),
            _ => Err(anyhow::anyhow!("Unsupported format: {}", format)),
        }
    }

    /// Save template to file
    pub fn save_template<P: AsRef<Path>>(
        path: P,
        format: &str,
        preset: Option<&str>,
    ) -> Result<()> {
        let template = Self::generate_template(format, preset)?;
        fs::write(path, template)?;
        Ok(())
    }
}

/// Hot-reload configuration watcher (optional feature)
#[cfg(feature = "hot-reload")]
pub struct ConfigWatcher {
    config_path: PathBuf,
    loader: ConfigurationLoader,
    callback: Box<dyn Fn(UnifiedDIConfiguration) -> Result<()> + Send + Sync>,
}

#[cfg(feature = "hot-reload")]
impl ConfigWatcher {
    pub fn new<P, F>(config_path: P, loader: ConfigurationLoader, callback: F) -> Self
    where
        P: AsRef<Path>,
        F: Fn(UnifiedDIConfiguration) -> Result<()> + Send + Sync + 'static,
    {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            loader,
            callback: Box::new(callback),
        }
    }

    pub async fn watch(&self) -> Result<()> {
        use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel(100);

        let mut watcher: RecommendedWatcher = notify::Watcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)?;

        while let Some(event) = rx.recv().await {
            if event.paths.contains(&self.config_path) {
                match self.loader.load_from_file(&self.config_path) {
                    Ok(new_config) => {
                        if let Err(e) = (self.callback)(new_config) {
                            eprintln!("Failed to apply configuration reload: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to load configuration during hot reload: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_load_from_toml() -> Result<()> {
        let loader = ConfigurationLoader::new();

        let toml_content = r#"
        [metadata]
        version = "1.0.0"
        application = "test"
        
        [core]
        log_level = "debug"
        thread_pool_size = 4
        
        [ai.embedding]
        model_name = "test_model"
        batch_size = 32
        "#;

        let config = loader.load_from_toml(toml_content)?;
        assert_eq!(config.core.log_level, "debug");
        assert_eq!(config.core.thread_pool_size, 4);
        assert_eq!(config.ai.embedding.model_name, "test_model");
        assert_eq!(config.ai.embedding.batch_size, 32);

        Ok(())
    }

    #[test]
    fn test_load_from_json() -> Result<()> {
        let loader = ConfigurationLoader::new();

        let json_content = r#"
        {
            "environment": "Development",
            "core": {
                "log_level": "info",
                "thread_pool_size": 8
            },
            "ai": {
                "embedding": {
                    "model_name": "json_model",
                    "batch_size": 64
                }
            }
        }
        "#;

        let config = loader.load_from_json(json_content)?;
        assert_eq!(config.core.log_level, "info");
        assert_eq!(config.core.thread_pool_size, 8);
        assert_eq!(config.ai.embedding.model_name, "json_model");

        Ok(())
    }

    #[test]
    fn test_save_to_file() -> Result<()> {
        let loader = ConfigurationLoader::new();
        let config = UnifiedDIConfiguration::development();

        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().with_extension("toml");

        loader.save_to_file(&config, &path)?;

        // Verify we can load it back
        let loaded_config = loader.load_from_file(&path)?;
        assert_eq!(loaded_config.environment, config.environment);

        Ok(())
    }

    #[test]
    fn test_template_generation() -> Result<()> {
        let toml_template = ConfigTemplateGenerator::generate_template("toml", Some("production"))?;
        assert!(toml_template.contains("[core]"));
        assert!(toml_template.contains("[ai"));

        let json_template = ConfigTemplateGenerator::generate_template("json", None)?;
        assert!(json_template.contains("\"core\""));
        assert!(json_template.contains("\"ai\""));

        Ok(())
    }

    #[test]
    fn test_environment_variables() -> Result<()> {
        // Set test environment variables
        env::set_var("MAGRAY_LOG_LEVEL", "trace");
        env::set_var("MAGRAY_THREADS", "16");
        env::set_var("MAGRAY_AI_GPU_ENABLED", "false");

        let loader = ConfigurationLoader::new();
        let mut config = UnifiedDIConfiguration::development();

        loader.apply_environment_variables(&mut config)?;

        assert_eq!(config.core.log_level, "trace");
        assert_eq!(config.core.thread_pool_size, 16);
        assert!(!config.features.gpu_acceleration);

        // Cleanup
        env::remove_var("MAGRAY_LOG_LEVEL");
        env::remove_var("MAGRAY_THREADS");
        env::remove_var("MAGRAY_AI_GPU_ENABLED");

        Ok(())
    }
}
