use anyhow::{bail, Result};
use domain::config::*;
use tracing::warn;

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, config: &MagrayConfig) -> Result<()> {
        self.validate_ai_config(&config.ai)?;
        self.validate_memory_config(&config.memory)?;
        self.validate_mcp_config(&config.mcp)?;
        self.validate_plugins_config(&config.plugins)?;
        self.validate_logging_config(&config.logging)?;
        self.validate_paths_config(&config.paths)?;
        self.validate_performance_config(&config.performance)?;
        Ok(())
    }

    fn validate_ai_config(&self, config: &AiConfig) -> Result<()> {
        // Check if default provider exists
        if !config.default_provider.is_empty()
            && !config.providers.contains_key(&config.default_provider)
        {
            warn!(
                "Default AI provider '{}' not found in providers list",
                config.default_provider
            );
        }

        // Validate each provider
        for (name, provider) in &config.providers {
            self.validate_provider_config(name, provider)?;
        }

        // Validate fallback chain
        for provider in &config.fallback_chain {
            if !config.providers.contains_key(provider) {
                warn!(
                    "Fallback provider '{}' not found in providers list",
                    provider
                );
            }
        }

        // Validate temperature
        if config.temperature < 0.0 || config.temperature > 2.0 {
            bail!(
                "Temperature must be between 0.0 and 2.0, got {}",
                config.temperature
            );
        }

        // Validate max_tokens
        if config.max_tokens == 0 {
            bail!("max_tokens must be greater than 0");
        }

        self.validate_retry_config(&config.retry_config)?;

        Ok(())
    }

    fn validate_provider_config(&self, name: &str, config: &ProviderConfig) -> Result<()> {
        match config.provider_type {
            ProviderType::OpenAI
            | ProviderType::Anthropic
            | ProviderType::Google
            | ProviderType::Azure
            | ProviderType::Groq => {
                if config.api_key.is_none() {
                    warn!("Provider '{}' is missing API key", name);
                }
            }
            ProviderType::Local => {
                if let Some(path) = &config.model_path {
                    if !path.exists() {
                        warn!("Local model path does not exist: {}", path.display());
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_memory_config(&self, config: &MemoryConfig) -> Result<()> {
        // Validate HNSW parameters
        if config.hnsw.m == 0 {
            bail!("HNSW M parameter must be greater than 0");
        }
        if config.hnsw.ef_construction < config.hnsw.m {
            bail!("HNSW ef_construction must be >= M");
        }
        if config.hnsw.ef_search == 0 {
            bail!("HNSW ef_search must be greater than 0");
        }

        // Validate embedding config
        if config.embedding.dimension == 0 {
            bail!("Embedding dimension must be greater than 0");
        }
        if config.embedding.batch_size == 0 {
            bail!("Batch size must be greater than 0");
        }

        // Validate cache size
        if config.cache_size_mb == 0 {
            warn!("Cache size is set to 0, caching will be disabled");
        }

        // Validate persistence
        if config.persistence.enabled {
            if let Some(path) = &config.persistence.path {
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        warn!(
                            "Persistence path parent directory does not exist: {}",
                            parent.display()
                        );
                    }
                }
            } else {
                bail!("Persistence is enabled but no path is specified");
            }
        }

        Ok(())
    }

    fn validate_mcp_config(&self, config: &McpConfig) -> Result<()> {
        if config.enabled {
            if config.servers.is_empty() && !config.auto_discovery {
                warn!(
                    "MCP is enabled but no servers are configured and auto-discovery is disabled"
                );
            }

            for server in &config.servers {
                if server.name.is_empty() {
                    bail!("MCP server name cannot be empty");
                }
                if server.url.is_empty() {
                    bail!(
                        "MCP server URL cannot be empty for server '{}'",
                        server.name
                    );
                }
            }

            if config.timeout_sec == 0 {
                bail!("MCP timeout must be greater than 0");
            }
        }
        Ok(())
    }

    fn validate_plugins_config(&self, config: &PluginsConfig) -> Result<()> {
        if config.enabled {
            if let Some(dir) = &config.plugin_dir {
                if !dir.exists() {
                    warn!("Plugin directory does not exist: {}", dir.display());
                }
            }
        }
        Ok(())
    }

    fn validate_logging_config(&self, config: &LoggingConfig) -> Result<()> {
        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error", "off"];
        if !valid_levels.contains(&config.level.to_lowercase().as_str()) {
            bail!(
                "Invalid log level '{}'. Must be one of: {:?}",
                config.level,
                valid_levels
            );
        }

        // Validate file logging
        if config.file_enabled {
            if let Some(path) = &config.file_path {
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        warn!(
                            "Log file parent directory does not exist: {}",
                            parent.display()
                        );
                    }
                }
            } else {
                bail!("File logging is enabled but no file path is specified");
            }
        }

        if config.max_size_mb == 0 {
            bail!("Max log size must be greater than 0");
        }

        Ok(())
    }

    fn validate_paths_config(&self, config: &PathsConfig) -> Result<()> {
        let paths = [
            ("data_dir", &config.data_dir),
            ("cache_dir", &config.cache_dir),
            ("models_dir", &config.models_dir),
            ("logs_dir", &config.logs_dir),
        ];

        for (name, path) in paths {
            if let Some(p) = path {
                if !p.exists() {
                    warn!("{} does not exist: {}", name, p.display());
                }
            }
        }

        Ok(())
    }

    fn validate_performance_config(&self, config: &PerformanceConfig) -> Result<()> {
        if config.worker_threads == 0 {
            bail!("Worker threads must be greater than 0");
        }

        if config.max_concurrent_requests == 0 {
            bail!("Max concurrent requests must be greater than 0");
        }

        if config.memory_limit_mb == 0 {
            warn!("Memory limit is set to 0, no memory limit will be enforced");
        }

        Ok(())
    }

    fn validate_retry_config(&self, config: &RetryConfig) -> Result<()> {
        if config.initial_delay_ms == 0 {
            bail!("Initial retry delay must be greater than 0");
        }

        if config.max_delay_ms < config.initial_delay_ms {
            bail!("Max retry delay must be >= initial delay");
        }

        if config.exponential_base <= 1.0 {
            bail!("Exponential base must be greater than 1.0");
        }

        Ok(())
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}
