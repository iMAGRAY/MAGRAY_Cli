//! Configuration Presets - Ready-to-use environment configurations
//!
//! This module provides pre-configured settings optimized for different environments
//! and use cases. Each preset is carefully tuned for specific performance and
//! resource characteristics.

use anyhow::Result;
use std::{collections::HashMap, path::PathBuf};

use super::unified_config::{
    CoreSystemConfig, DatabaseConfig, Environment, FeatureFlags, MemorySystemConfig,
    OrchestrationConfig, PerformanceConfig, PerformanceThresholds, ProfilingConfig,
    RateLimitConfig, SecurityConfig, UnifiedDIConfiguration,
};

/// Configuration preset factory
pub struct ConfigPresets;

impl ConfigPresets {
    /// High-performance production configuration for server environments
    pub fn production_server() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::production();

        // Optimize for server environment
        config.core.thread_pool_size = 0; // Auto-detect all cores
        config.core.max_memory_mb = 16384; // 16GB limit
        config.core.data_dir = PathBuf::from("/opt/magray/data");

        // Aggressive performance settings
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 64,
            ef_construction: 1000,
            ef_search: 200,
            max_elements: 10_000_000,
            use_parallel: true,
            ..Default::default()
        };

        config.memory.batch.max_batch_size = 10000;
        config.memory.batch.worker_threads = 16;

        // Enhanced monitoring
        config.performance.monitoring.enable_metrics = true;
        config.performance.monitoring.enable_tracing = true;
        config.performance.monitoring.metrics_interval_seconds = 15;

        config.performance.thresholds = PerformanceThresholds {
            max_response_time_ms: 1000, // 1 second SLA
            max_memory_usage_percent: 75.0,
            max_cpu_usage_percent: 70.0,
            max_error_rate_percent: 0.01, // 0.01% error rate
        };

        config.security.rate_limiting = RateLimitConfig {
            enabled: true,
            requests_per_second: 5000,
            burst_capacity: 10000,
        };

        config.metadata.source = "production_server_preset".to_string();
        config.metadata.description = Some("High-performance server configuration".to_string());

        config
    }

    /// Edge deployment configuration for resource-constrained environments
    pub fn production_edge() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::production();

        // Constrain resources for edge
        config.core.thread_pool_size = 4;
        config.core.max_memory_mb = 2048; // 2GB limit

        // Optimize for low latency, lower quality
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 16,
            ef_construction: 200,
            ef_search: 32,
            max_elements: 100_000,
            use_parallel: true,
            ..Default::default()
        };

        config.memory.batch.max_batch_size = 1000;
        config.memory.batch.worker_threads = 2;

        // Reduce monitoring overhead
        config.performance.monitoring.metrics_interval_seconds = 60;
        config.performance.monitoring.sample_rate = 0.01; // 1% sampling

        // Disable expensive features
        config.features.ml_promotion = false;
        config.features.experimental = false;

        config.metadata.source = "production_edge_preset".to_string();
        config.metadata.description = Some("Edge deployment configuration".to_string());

        config
    }

    /// Development configuration optimized for iteration speed
    pub fn development_fast() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::development();

        // Fast iteration settings
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 8,
            ef_construction: 100,
            ef_search: 16,
            max_elements: 10_000,
            use_parallel: false, // Simpler debugging
            ..Default::default()
        };

        config.memory.batch.max_batch_size = 100;
        config.memory.batch.worker_threads = 1;

        // Enhanced debugging
        config.core.log_level = "trace".to_string();
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: true,
            memory_profiling: true,
            sample_rate: 0.1, // 10% sampling
            output_dir: Some(PathBuf::from("./profiling")),
        };

        config.metadata.source = "development_fast_preset".to_string();
        config.metadata.description = Some("Fast iteration development configuration".to_string());

        config
    }

    /// CI/CD configuration for automated testing
    pub fn ci_cd() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::test();

        // CI-specific settings
        config.core.thread_pool_size = 2; // Limited CI resources
        config.core.max_memory_mb = 1024; // 1GB limit
        config.core.log_level = "warn".to_string(); // Reduce noise

        // Fast, deterministic settings
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 4,
            ef_construction: 50,
            ef_search: 8,
            max_elements: 1000,
            use_parallel: false, // Deterministic
            ..Default::default()
        };

        config.memory.batch.max_batch_size = 50;
        config.memory.batch.async_flush = false; // Synchronous for testing

        // Disable all expensive features
        config.features = FeatureFlags {
            gpu_acceleration: false,
            simd_optimizations: false,
            ml_promotion: false,
            streaming_api: false,
            experimental: false,
            custom: HashMap::new(),
        };

        // No profiling in CI
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: false,
            memory_profiling: false,
            sample_rate: 0.0,
            output_dir: None,
        };

        // Deterministic random seed
        config.security.secure_random_seed = Some(12345);

        config.metadata.source = "ci_cd_preset".to_string();
        config.metadata.description = Some("CI/CD testing configuration".to_string());

        config
    }

    /// Benchmark configuration for performance testing
    pub fn benchmark() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::production();

        // Optimize for pure performance
        config.core.thread_pool_size = 0; // Use all cores
        config.core.max_memory_mb = 0; // No memory limit
        config.core.log_level = "error".to_string(); // Minimal logging

        // Maximum quality settings
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 96,
            ef_construction: 1500,
            ef_search: 500,
            max_elements: 10_000_000,
            use_parallel: true,
            ..Default::default()
        };

        config.memory.batch.max_batch_size = 50000;
        config.memory.batch.worker_threads = 32;

        // Enable all performance features
        config.features.gpu_acceleration = true;
        config.features.simd_optimizations = true;
        config.features.experimental = true;

        // Comprehensive profiling
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: true,
            memory_profiling: true,
            sample_rate: 1.0, // 100% sampling
            output_dir: Some(PathBuf::from("./benchmark_profiles")),
        };

        // Disable rate limiting for benchmarks
        config.security.rate_limiting.enabled = false;

        config.metadata.source = "benchmark_preset".to_string();
        config.metadata.description = Some("Performance benchmark configuration".to_string());

        config
    }

    /// Demo configuration for presentations and showcases
    pub fn demo() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::development();

        // Demo-friendly settings
        config.core.log_level = "info".to_string();
        config.core.max_memory_mb = 4096; // 4GB reasonable for demos

        // Balanced performance for demos
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 24,
            ef_construction: 300,
            ef_search: 64,
            max_elements: 50_000,
            use_parallel: true,
            ..Default::default()
        };

        // Enable interesting features for demo
        config.features.ml_promotion = true;
        config.features.streaming_api = true;
        config.features.experimental = true;

        // Nice profiling for demos
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: true,
            memory_profiling: false, // Less intrusive
            sample_rate: 0.05,       // 5% sampling
            output_dir: Some(PathBuf::from("./demo_profiles")),
        };

        config.metadata.source = "demo_preset".to_string();
        config.metadata.description = Some("Demo and showcase configuration".to_string());

        config
    }

    /// Research configuration for AI/ML experiments
    pub fn research() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::development();

        // Research-oriented settings
        config.core.log_level = "debug".to_string();
        config.core.max_memory_mb = 8192; // 8GB for large experiments

        // Experimental HNSW settings
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 48,
            ef_construction: 800,
            ef_search: 150,
            max_elements: 1_000_000,
            use_parallel: true,
            ..Default::default()
        };

        // Enable all experimental features
        config.features.ml_promotion = true;
        config.features.streaming_api = true;
        config.features.experimental = true;

        // Add custom research flags
        config
            .features
            .custom
            .insert("advanced_reranking".to_string(), true);
        config
            .features
            .custom
            .insert("hybrid_search".to_string(), true);
        config
            .features
            .custom
            .insert("dynamic_embeddings".to_string(), true);

        // Comprehensive profiling for research
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: true,
            memory_profiling: true,
            sample_rate: 0.2, // 20% sampling
            output_dir: Some(PathBuf::from("./research_profiles")),
        };

        // Research-specific security (deterministic but complex)
        config.security.secure_random_seed = None; // Use real randomness
        config.security.audit_logging = true; // Track all experiments

        config.metadata.source = "research_preset".to_string();
        config.metadata.description = Some("AI/ML research configuration".to_string());

        config
    }

    /// Docker container configuration
    pub fn docker() -> UnifiedDIConfiguration {
        let mut config = UnifiedDIConfiguration::production();

        // Container-optimized paths
        config.core.data_dir = PathBuf::from("/app/data");
        config.core.temp_dir = PathBuf::from("/tmp/magray");
        config.memory.database.db_path = PathBuf::from("/app/data/memory.db");

        // Container resource constraints
        config.core.thread_pool_size = 0; // Auto-detect container limits
        config.core.max_memory_mb = 4096; // 4GB default container limit

        // Optimize for container startup time
        config.memory.hnsw = crate::HnswConfig {
            max_connections: 16,
            ef_construction: 200,
            ef_search: 50,
            max_elements: 500_000,
            use_parallel: true,
            ..Default::default()
        };

        // Container-friendly logging
        config.core.log_level = "info".to_string();

        // Disable profiling by default (can be enabled via env vars)
        config.performance.profiling = ProfilingConfig {
            cpu_profiling: false,
            memory_profiling: false,
            sample_rate: 0.0,
            output_dir: None,
        };

        config.metadata.source = "docker_preset".to_string();
        config.metadata.description = Some("Docker container configuration".to_string());

        config
    }

    /// Get all available presets with descriptions
    pub fn available_presets() -> Vec<(String, String)> {
        vec![
            (
                "production_server".to_string(),
                "High-performance server environment".to_string(),
            ),
            (
                "production_edge".to_string(),
                "Edge deployment with resource constraints".to_string(),
            ),
            (
                "development_fast".to_string(),
                "Fast iteration development".to_string(),
            ),
            (
                "ci_cd".to_string(),
                "Automated testing in CI/CD".to_string(),
            ),
            (
                "benchmark".to_string(),
                "Performance benchmarking".to_string(),
            ),
            (
                "demo".to_string(),
                "Presentations and showcases".to_string(),
            ),
            (
                "research".to_string(),
                "AI/ML research and experiments".to_string(),
            ),
            (
                "docker".to_string(),
                "Docker container deployment".to_string(),
            ),
        ]
    }

    /// Create configuration from preset name
    pub fn from_preset_name(preset_name: &str) -> Result<UnifiedDIConfiguration> {
        match preset_name.to_lowercase().as_str() {
            "production_server" => Ok(Self::production_server()),
            "production_edge" => Ok(Self::production_edge()),
            "development_fast" => Ok(Self::development_fast()),
            "ci_cd" => Ok(Self::ci_cd()),
            "benchmark" => Ok(Self::benchmark()),
            "demo" => Ok(Self::demo()),
            "research" => Ok(Self::research()),
            "docker" => Ok(Self::docker()),
            "production" => Ok(UnifiedDIConfiguration::production()),
            "development" => Ok(UnifiedDIConfiguration::development()),
            "test" => Ok(UnifiedDIConfiguration::test()),
            "minimal" => Ok(UnifiedDIConfiguration::minimal()),
            _ => Err(anyhow::anyhow!("Unknown preset: {}", preset_name)),
        }
    }

    /// Auto-detect best preset based on environment
    pub fn auto_detect() -> UnifiedDIConfiguration {
        // Check environment variables first
        if let Ok(preset) = std::env::var("MAGRAY_CONFIG_PRESET") {
            if let Ok(config) = Self::from_preset_name(&preset) {
                return config;
            }
        }

        // Check if running in CI
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return Self::ci_cd();
        }

        // Check if running in Docker
        if std::path::Path::new("/.dockerenv").exists() {
            return Self::docker();
        }

        // Check if this looks like a server environment
        if std::env::var("SERVER_MODE").is_ok() || std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
        {
            return Self::production_server();
        }

        // Check if development indicators are present
        if std::env::var("CARGO_MANIFEST_DIR").is_ok()
            || std::path::Path::new("./Cargo.toml").exists()
        {
            return Self::development_fast();
        }

        // Default to development
        UnifiedDIConfiguration::development()
    }
}

/// Configuration builder for creating custom configurations
pub struct ConfigBuilder {
    config: UnifiedDIConfiguration,
}

impl ConfigBuilder {
    /// Start with a preset
    pub fn from_preset(preset_name: &str) -> Result<Self> {
        Ok(Self {
            config: ConfigPresets::from_preset_name(preset_name)?,
        })
    }

    /// Start with default development config
    pub fn new() -> Self {
        Self {
            config: UnifiedDIConfiguration::development(),
        }
    }

    /// Set environment
    pub fn environment(mut self, env: Environment) -> Self {
        self.config.environment = env;
        self
    }

    /// Set data directory
    pub fn data_dir(mut self, path: PathBuf) -> Self {
        self.config.core.data_dir = path.clone();
        self.config.memory.database.db_path = path.join("memory.db");
        self
    }

    /// Set log level
    pub fn log_level(mut self, level: String) -> Self {
        self.config.core.log_level = level;
        self
    }

    /// Set thread pool size
    pub fn threads(mut self, count: usize) -> Self {
        self.config.core.thread_pool_size = count;
        self
    }

    /// Set memory limit
    pub fn memory_limit_mb(mut self, limit: usize) -> Self {
        self.config.core.max_memory_mb = limit;
        self
    }

    /// Enable/disable GPU acceleration
    pub fn gpu(mut self, enabled: bool) -> Self {
        self.config.features.gpu_acceleration = enabled;
        self.config.ai.embedding.use_gpu = enabled;
        self.config.ai.reranking.use_gpu = enabled;
        self
    }

    /// Enable/disable SIMD optimizations
    pub fn simd(mut self, enabled: bool) -> Self {
        self.config.features.simd_optimizations = enabled;
        self
    }

    /// Enable development mode
    pub fn dev_mode(mut self, enabled: bool) -> Self {
        self.config.core.dev_mode = enabled;
        self
    }

    /// Set AI model
    pub fn ai_model(mut self, embedding_model: String, reranking_model: String) -> Self {
        self.config.ai.embedding.model_name = embedding_model;
        self.config.ai.reranking.model_name = reranking_model;
        self
    }

    /// Apply environment variable overrides
    pub fn with_env_overrides(mut self) -> Result<Self> {
        self.config.apply_env_overrides()?;
        Ok(self)
    }

    /// Build and validate configuration
    pub fn build(self) -> Result<UnifiedDIConfiguration> {
        let report = self.config.validate()?;
        if !report.is_valid() {
            return Err(anyhow::anyhow!(
                "Configuration validation failed: {:?}",
                report.errors
            ));
        }
        Ok(self.config)
    }

    /// Build without validation (for testing)
    pub fn build_unchecked(self) -> UnifiedDIConfiguration {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_creation() -> Result<()> {
        let prod_server = ConfigPresets::production_server();
        assert_eq!(prod_server.environment, Environment::Production);
        assert!(prod_server.memory.hnsw.max_connections >= 32);

        let edge = ConfigPresets::production_edge();
        assert!(edge.core.max_memory_mb <= 2048);

        let ci_cd = ConfigPresets::ci_cd();
        assert_eq!(ci_cd.environment, Environment::Test);
        assert!(!ci_cd.features.gpu_acceleration);

        Ok(())
    }

    #[test]
    fn test_preset_from_name() -> Result<()> {
        let config = ConfigPresets::from_preset_name("benchmark")?;
        assert_eq!(config.metadata.source, "benchmark_preset");

        let invalid_result = ConfigPresets::from_preset_name("invalid_preset");
        assert!(invalid_result.is_err());

        Ok(())
    }

    #[test]
    fn test_config_builder() -> Result<()> {
        let config = ConfigBuilder::new()
            .environment(Environment::Production)
            .threads(8)
            .memory_limit_mb(4096)
            .gpu(true)
            .log_level("info".to_string())
            .build()?;

        assert_eq!(config.environment, Environment::Production);
        assert_eq!(config.core.thread_pool_size, 8);
        assert_eq!(config.core.max_memory_mb, 4096);
        assert!(config.features.gpu_acceleration);
        assert_eq!(config.core.log_level, "info");

        Ok(())
    }

    #[test]
    fn test_available_presets() {
        let presets = ConfigPresets::available_presets();
        assert!(!presets.is_empty());

        // Check that all presets can be created
        for (name, _description) in presets {
            assert!(ConfigPresets::from_preset_name(&name).is_ok());
        }
    }
}
