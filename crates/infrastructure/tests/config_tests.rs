#[cfg(test)]
mod tests {
    use domain::config::*;
    use infrastructure::config::{ConfigLoader, ConfigValidator};
    use std::env;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_default_config_creation() {
        let config = MagrayConfig::default();

        assert_eq!(config.ai.default_provider, "openai");
        assert_eq!(config.memory.backend, MemoryBackend::SQLite);
        assert_eq!(config.logging.level, "info");
        assert_eq!(config.performance.worker_threads, 4);
    }

    #[tokio::test]
    async fn test_config_loader_from_toml() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join(".magrayrc.toml");

        let toml_content = r#"
[ai]
default_provider = "anthropic"
max_tokens = 8192
temperature = 0.5

[memory]
cache_size_mb = 512

[logging]
level = "debug"
"#;

        fs::write(&config_path, toml_content).await?;

        let loader = ConfigLoader::new().with_path(config_path);
        let config = loader.load().await?;

        assert_eq!(config.ai.default_provider, "anthropic");
        assert_eq!(config.ai.max_tokens, 8192);
        assert_eq!(config.ai.temperature, 0.5);
        assert_eq!(config.memory.cache_size_mb, 512);
        assert_eq!(config.logging.level, "debug");

        Ok(())
    }

    #[tokio::test]
    async fn test_config_loader_from_json() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join(".magrayrc.json");

        let json_content = r#"{
  "ai": {
    "default_provider": "google",
    "max_tokens": 2048
  },
  "performance": {
    "worker_threads": 8,
    "enable_gpu": true
  }
}"#;

        fs::write(&config_path, json_content).await?;

        let loader = ConfigLoader::new().with_path(config_path);
        let config = loader.load().await?;

        assert_eq!(config.ai.default_provider, "google");
        assert_eq!(config.ai.max_tokens, 2048);
        assert_eq!(config.performance.worker_threads, 8);
        assert!(config.performance.enable_gpu);

        Ok(())
    }

    #[tokio::test]
    async fn test_env_override() -> anyhow::Result<()> {
        // Set environment variables
        env::set_var("MAGRAY_AI_PROVIDER", "local");
        env::set_var("MAGRAY_LOG_LEVEL", "trace");
        env::set_var("MAGRAY_WORKER_THREADS", "16");
        env::set_var("MAGRAY_ENABLE_GPU", "true");
        env::set_var("MAGRAY_CACHE_SIZE_MB", "1024");

        let loader = ConfigLoader::new();
        let config = loader.load().await?;

        assert_eq!(config.ai.default_provider, "local");
        assert_eq!(config.logging.level, "trace");
        assert_eq!(config.performance.worker_threads, 16);
        assert!(config.performance.enable_gpu);
        assert_eq!(config.memory.cache_size_mb, 1024);

        // Clean up
        env::remove_var("MAGRAY_AI_PROVIDER");
        env::remove_var("MAGRAY_LOG_LEVEL");
        env::remove_var("MAGRAY_WORKER_THREADS");
        env::remove_var("MAGRAY_ENABLE_GPU");
        env::remove_var("MAGRAY_CACHE_SIZE_MB");

        Ok(())
    }

    #[tokio::test]
    async fn test_config_validation_success() {
        let config = MagrayConfig::default();
        let validator = ConfigValidator::new();

        assert!(validator.validate(&config).is_ok());
    }

    #[tokio::test]
    async fn test_config_validation_invalid_temperature() {
        let mut config = MagrayConfig::default();
        config.ai.temperature = 3.0; // Invalid: > 2.0

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Temperature"));
    }

    #[tokio::test]
    async fn test_config_validation_invalid_hnsw() {
        let mut config = MagrayConfig::default();
        config.memory.hnsw.m = 0; // Invalid: must be > 0

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HNSW M"));
    }

    #[tokio::test]
    async fn test_config_validation_invalid_log_level() {
        let mut config = MagrayConfig::default();
        config.logging.level = "invalid".to_string();

        let validator = ConfigValidator::new();
        let result = validator.validate(&config);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("log level"));
    }

    #[tokio::test]
    async fn test_save_config() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test_config.toml");

        let mut config = MagrayConfig::default();
        config.ai.default_provider = "test_provider".to_string();
        config.memory.cache_size_mb = 999;

        let loader = ConfigLoader::new();
        loader.save_config(&config, &config_path).await?;

        // Read back and verify
        let content = fs::read_to_string(&config_path).await?;
        assert!(content.contains("test_provider"));
        assert!(content.contains("999"));

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_example_config() {
        let example = ConfigLoader::generate_example_config();

        assert!(!example.is_empty());
        assert!(example.contains("openai"));
        assert!(example.contains("memory"));
        assert!(example.contains("plugins"));
        assert!(example.contains("logging"));
    }

    #[tokio::test]
    async fn test_api_key_env_override() -> anyhow::Result<()> {
        env::set_var("MAGRAY_OPENAI_API_KEY", "test-openai-key");
        env::set_var("MAGRAY_ANTHROPIC_API_KEY", "test-anthropic-key");

        let loader = ConfigLoader::new();
        let config = loader.load().await?;

        assert_eq!(
            config
                .ai
                .providers
                .get("openai")
                .expect("Test operation should succeed")
                .api_key,
            Some("test-openai-key".to_string())
        );
        assert_eq!(
            config
                .ai
                .providers
                .get("anthropic")
                .expect("Test operation should succeed")
                .api_key,
            Some("test-anthropic-key".to_string())
        );

        env::remove_var("MAGRAY_OPENAI_API_KEY");
        env::remove_var("MAGRAY_ANTHROPIC_API_KEY");

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_backend_env_override() -> anyhow::Result<()> {
        env::set_var("MAGRAY_MEMORY_BACKEND", "hybrid");

        let loader = ConfigLoader::new();
        let config = loader.load().await?;

        assert_eq!(config.memory.backend, MemoryBackend::Hybrid);

        env::remove_var("MAGRAY_MEMORY_BACKEND");

        Ok(())
    }

    #[tokio::test]
    async fn test_paths_env_override() -> anyhow::Result<()> {
        env::set_var("MAGRAY_DATA_DIR", "/custom/data");
        env::set_var("MAGRAY_CACHE_DIR", "/custom/cache");
        env::set_var("MAGRAY_MODELS_DIR", "/custom/models");

        let loader = ConfigLoader::new();
        let config = loader.load().await?;

        assert_eq!(config.paths.data_dir, Some(PathBuf::from("/custom/data")));
        assert_eq!(config.paths.cache_dir, Some(PathBuf::from("/custom/cache")));
        assert_eq!(
            config.paths.models_dir,
            Some(PathBuf::from("/custom/models"))
        );

        env::remove_var("MAGRAY_DATA_DIR");
        env::remove_var("MAGRAY_CACHE_DIR");
        env::remove_var("MAGRAY_MODELS_DIR");

        Ok(())
    }
}
