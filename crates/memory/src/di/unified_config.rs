//! Unified DI Configuration System for MAGRAY CLI
//!
//! This module provides a centralized, type-safe configuration system that unifies
//! all previously scattered config structures across the project.
//!
//! ## Features
//! - Unified configuration structure using composer pattern
//! - Environment-specific presets (Production, Development, Test, Minimal)
//! - Comprehensive validation with detailed error reporting
//! - Environment variables support via serde
//! - Builder pattern for flexible configuration composition
//! - Backward compatibility with existing config structures

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, path::PathBuf};

// Re-exports for unified access
pub use crate::{
    batch_manager::BatchConfig, cache_lru::CacheConfig, health::HealthMonitorConfig,
    hnsw_index::HnswConfig, ml_promotion::MLPromotionConfig, notifications::NotificationConfig,
    resource_manager::ResourceConfig, streaming::StreamingConfig, types::PromotionConfig,
};
pub use ai::config::{AiConfig, EmbeddingConfig, RerankingConfig};
pub use common::config_base::*;
pub use common::service_traits::ConfigurationProfile;

/// Environment type for configuration selection
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Environment {
    /// Production environment with optimized performance settings
    Production,
    /// Development environment with debugging enabled
    Development,
    /// Testing environment with minimal footprint
    Test,
    /// Minimal configuration for resource-constrained environments
    Minimal,
    /// Custom environment with user-defined settings
    Custom(String),
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Production => write!(f, "production"),
            Environment::Development => write!(f, "development"),
            Environment::Test => write!(f, "test"),
            Environment::Minimal => write!(f, "minimal"),
            Environment::Custom(name) => write!(f, "custom({})", name),
        }
    }
}

/// Unified DI Configuration that encompasses all system configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDIConfiguration {
    /// Environment type
    pub environment: Environment,

    /// Application metadata
    pub metadata: ConfigurationMetadata,

    /// Core system configuration
    pub core: CoreSystemConfig,

    /// AI and machine learning configuration
    pub ai: AiConfig,

    /// Memory system configuration  
    pub memory: MemorySystemConfig,

    /// Orchestration configuration
    pub orchestration: OrchestrationConfig,

    /// Performance and monitoring configuration
    pub performance: PerformanceConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Feature toggles
    pub features: FeatureFlags,

    /// Environment variable overrides
    #[serde(skip)]
    pub env_overrides: HashMap<String, String>,
}

/// Configuration metadata and versioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationMetadata {
    /// Configuration format version
    pub version: String,
    /// Configuration schema version  
    pub schema_version: u32,
    /// Application name
    pub application: String,
    /// Configuration creation timestamp
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Configuration source (file, env, default)
    pub source: String,
    /// Configuration description
    pub description: Option<String>,
}

impl Default for ConfigurationMetadata {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: 1,
            application: "magray_cli".to_string(),
            created_at: Some(chrono::Utc::now()),
            source: "default".to_string(),
            description: Some("MAGRAY CLI unified configuration".to_string()),
        }
    }
}

/// Core system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreSystemConfig {
    /// Application data directory
    pub data_dir: PathBuf,
    /// Temporary files directory
    pub temp_dir: PathBuf,
    /// Log level
    pub log_level: String,
    /// Thread pool size (0 = auto-detect)
    pub thread_pool_size: usize,
    /// Maximum memory usage in MB (0 = unlimited)
    pub max_memory_mb: usize,
    /// Enable development mode features
    pub dev_mode: bool,
}

impl Default for CoreSystemConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("magray"),
            temp_dir: std::env::temp_dir().join("magray_temp"),
            log_level: "info".to_string(),
            thread_pool_size: 0, // Auto-detect
            max_memory_mb: 0,    // Unlimited
            dev_mode: false,
        }
    }
}

/// Unified memory system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySystemConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Batch operations configuration
    pub batch: BatchConfig,
    /// HNSW index configuration
    pub hnsw: HnswConfig,
    /// Vector promotion configuration
    pub promotion: PromotionConfig,
    /// ML-based promotion configuration
    pub ml_promotion: Option<MLPromotionConfig>,
    /// Streaming configuration
    pub streaming: Option<StreamingConfig>,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database file path
    pub db_path: PathBuf,
    /// Connection pool size
    pub pool_size: usize,
    /// WAL mode enabled
    pub wal_mode: bool,
    /// Pragma settings
    pub pragma_settings: HashMap<String, String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let mut pragma_settings = HashMap::new();
        pragma_settings.insert("synchronous".to_string(), "NORMAL".to_string());
        pragma_settings.insert("cache_size".to_string(), "10000".to_string());
        pragma_settings.insert("temp_store".to_string(), "memory".to_string());

        Self {
            db_path: dirs::data_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("magray")
                .join("memory.db"),
            pool_size: 10,
            wal_mode: true,
            pragma_settings,
        }
    }
}

/// Orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    /// Health monitoring configuration
    pub health: HealthMonitorConfig,
    /// Resource management configuration
    pub resources: ResourceConfig,
    /// Notification configuration
    pub notifications: NotificationConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfigBase,
    /// Retry configuration
    pub retry: RetryConfigBase,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Monitoring configuration
    pub monitoring: MonitoringConfigBase,
    /// Performance thresholds
    pub thresholds: PerformanceThresholds,
    /// Profiling settings
    pub profiling: ProfilingConfig,
}

/// Performance thresholds for alerting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum response time in milliseconds
    pub max_response_time_ms: u64,
    /// Maximum memory usage percentage
    pub max_memory_usage_percent: f32,
    /// Maximum CPU usage percentage
    pub max_cpu_usage_percent: f32,
    /// Maximum error rate percentage
    pub max_error_rate_percent: f32,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_response_time_ms: 5000, // 5 seconds
            max_memory_usage_percent: 80.0,
            max_cpu_usage_percent: 80.0,
            max_error_rate_percent: 1.0,
        }
    }
}

/// Profiling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilingConfig {
    /// Enable CPU profiling
    pub cpu_profiling: bool,
    /// Enable memory profiling
    pub memory_profiling: bool,
    /// Profiling sample rate (0.0-1.0)
    pub sample_rate: f32,
    /// Profiling output directory
    pub output_dir: Option<PathBuf>,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            cpu_profiling: false,
            memory_profiling: false,
            sample_rate: 0.01,
            output_dir: None,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable audit logging
    pub audit_logging: bool,
    /// Secure random seed (for testing reproducibility)
    pub secure_random_seed: Option<u64>,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Authentication configuration
    pub authentication: AuthenticationConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Requests per second per client
    pub requests_per_second: u32,
    /// Burst capacity
    pub burst_capacity: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 100,
            burst_capacity: 200,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Enable authentication
    pub enabled: bool,
    /// JWT secret key (base64 encoded)
    pub jwt_secret: Option<String>,
    /// Token expiration in seconds
    pub token_expiration_seconds: u64,
}

impl Default for AuthenticationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            token_expiration_seconds: 3600, // 1 hour
        }
    }
}

/// Feature flags for conditional functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable GPU acceleration
    pub gpu_acceleration: bool,
    /// Enable SIMD optimizations
    pub simd_optimizations: bool,
    /// Enable ML-based promotion
    pub ml_promotion: bool,
    /// Enable streaming API
    pub streaming_api: bool,
    /// Enable experimental features
    pub experimental: bool,
    /// Custom feature flags
    pub custom: HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            gpu_acceleration: true,
            simd_optimizations: true,
            ml_promotion: false,  // Experimental
            streaming_api: false, // Experimental
            experimental: false,
            custom: HashMap::new(),
        }
    }
}

impl Default for UnifiedDIConfiguration {
    fn default() -> Self {
        Self::development()
    }
}

impl UnifiedDIConfiguration {
    /// Create production-ready configuration
    pub fn production() -> Self {
        Self {
            environment: Environment::Production,
            metadata: ConfigurationMetadata {
                source: "production_preset".to_string(),
                description: Some("Production-optimized configuration".to_string()),
                ..Default::default()
            },
            core: CoreSystemConfig {
                data_dir: PathBuf::from("/opt/magray/data"),
                temp_dir: PathBuf::from("/tmp/magray"),
                log_level: "warn".to_string(),
                thread_pool_size: 0, // Auto-detect
                max_memory_mb: 8192, // 8GB limit
                dev_mode: false,
            },
            ai: AiConfig::production(),
            memory: MemorySystemConfig {
                database: DatabaseConfig {
                    db_path: PathBuf::from("/opt/magray/data/memory.db"),
                    pool_size: 20,
                    wal_mode: true,
                    ..Default::default()
                },
                cache: CacheConfig::production(),
                batch: BatchConfig::production(),
                hnsw: HnswConfig::high_quality(),
                promotion: PromotionConfig::production(),
                ml_promotion: Some(MLPromotionConfig::production()),
                streaming: Some(StreamingConfig::production()),
            },
            orchestration: OrchestrationConfig {
                health: HealthMonitorConfig::production(),
                resources: ResourceConfig::production(),
                notifications: NotificationConfig::production(),
                circuit_breaker: CircuitBreakerConfigBase::default(),
                retry: RetryConfigBase::default(),
            },
            performance: PerformanceConfig {
                monitoring: MonitoringConfigBase {
                    enable_metrics: true,
                    enable_tracing: true,
                    metrics_interval_seconds: 30,
                    sample_rate: 0.1,
                },
                thresholds: PerformanceThresholds {
                    max_response_time_ms: 2000, // 2 seconds for production
                    max_memory_usage_percent: 85.0,
                    max_cpu_usage_percent: 75.0,
                    max_error_rate_percent: 0.1,
                },
                profiling: ProfilingConfig::default(),
            },
            security: SecurityConfig {
                audit_logging: true,
                secure_random_seed: None,
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    requests_per_second: 1000,
                    burst_capacity: 2000,
                },
                authentication: AuthenticationConfig {
                    enabled: false, // TODO: implement when needed
                    ..Default::default()
                },
            },
            features: FeatureFlags {
                gpu_acceleration: true,
                simd_optimizations: true,
                ml_promotion: true,
                streaming_api: true,
                experimental: false,
                ..Default::default()
            },
            env_overrides: HashMap::new(),
        }
    }

    /// Create development configuration with debugging enabled
    pub fn development() -> Self {
        Self {
            environment: Environment::Development,
            metadata: ConfigurationMetadata {
                source: "development_preset".to_string(),
                description: Some("Development configuration with debugging".to_string()),
                ..Default::default()
            },
            core: CoreSystemConfig {
                log_level: "debug".to_string(),
                dev_mode: true,
                ..Default::default()
            },
            ai: AiConfig::default(),
            memory: MemorySystemConfig {
                database: DatabaseConfig::default(),
                cache: CacheConfig::default(),
                batch: BatchConfig::default(),
                hnsw: HnswConfig::default(),
                promotion: PromotionConfig::default(),
                ml_promotion: None, // Disabled in dev
                streaming: None,    // Disabled in dev
            },
            orchestration: OrchestrationConfig {
                health: HealthMonitorConfig::default(),
                resources: ResourceConfig::default(),
                notifications: NotificationConfig::default(),
                circuit_breaker: CircuitBreakerConfigBase::default(),
                retry: RetryConfigBase::default(),
            },
            performance: PerformanceConfig {
                monitoring: MonitoringConfigBase::default(),
                thresholds: PerformanceThresholds::default(),
                profiling: ProfilingConfig {
                    cpu_profiling: true,
                    memory_profiling: true,
                    ..Default::default()
                },
            },
            security: SecurityConfig {
                audit_logging: false,
                ..Default::default()
            },
            features: FeatureFlags {
                experimental: true, // Enable experimental features in dev
                ..Default::default()
            },
            env_overrides: HashMap::new(),
        }
    }

    /// Create testing configuration with minimal footprint
    pub fn test() -> Self {
        Self {
            environment: Environment::Test,
            metadata: ConfigurationMetadata {
                source: "test_preset".to_string(),
                description: Some("Testing configuration with minimal footprint".to_string()),
                ..Default::default()
            },
            core: CoreSystemConfig {
                data_dir: std::env::temp_dir().join("magray_test"),
                temp_dir: std::env::temp_dir().join("magray_test_temp"),
                log_level: "error".to_string(),
                thread_pool_size: 2,
                max_memory_mb: 512, // 512MB limit for tests
                dev_mode: false,
            },
            ai: AiConfig::minimal(),
            memory: MemorySystemConfig {
                database: DatabaseConfig {
                    db_path: std::env::temp_dir()
                        .join("magray_test")
                        .join("test_memory.db"),
                    pool_size: 2,
                    wal_mode: false, // Simpler for tests
                    ..Default::default()
                },
                cache: CacheConfig::minimal(),
                batch: BatchConfig::minimal(),
                hnsw: HnswConfig::small_dataset(),
                promotion: PromotionConfig::minimal(),
                ml_promotion: None, // Disabled in tests
                streaming: None,    // Disabled in tests
            },
            orchestration: OrchestrationConfig {
                health: HealthMonitorConfig::minimal(),
                resources: ResourceConfig::minimal(),
                notifications: NotificationConfig::minimal(),
                circuit_breaker: CircuitBreakerConfigBase::default(),
                retry: RetryConfigBase {
                    max_retries: 1, // Faster tests
                    initial_backoff_ms: 10,
                    max_backoff_ms: 100,
                    backoff_multiplier: 1.5,
                },
            },
            performance: PerformanceConfig {
                monitoring: MonitoringConfigBase {
                    enable_metrics: false,
                    enable_tracing: false,
                    metrics_interval_seconds: 300, // Less frequent
                    sample_rate: 0.0,              // No sampling in tests
                },
                thresholds: PerformanceThresholds {
                    max_response_time_ms: 10000, // More lenient for tests
                    ..Default::default()
                },
                profiling: ProfilingConfig {
                    cpu_profiling: false,
                    memory_profiling: false,
                    ..Default::default()
                },
            },
            security: SecurityConfig {
                audit_logging: false,
                secure_random_seed: Some(42), // Deterministic for tests
                ..Default::default()
            },
            features: FeatureFlags {
                gpu_acceleration: false,   // No GPU in CI
                simd_optimizations: false, // Deterministic tests
                ml_promotion: false,
                streaming_api: false,
                experimental: false,
                ..Default::default()
            },
            env_overrides: HashMap::new(),
        }
    }

    /// Create minimal configuration for resource-constrained environments
    pub fn minimal() -> Self {
        Self {
            environment: Environment::Minimal,
            metadata: ConfigurationMetadata {
                source: "minimal_preset".to_string(),
                description: Some(
                    "Minimal configuration for resource-constrained environments".to_string(),
                ),
                ..Default::default()
            },
            core: CoreSystemConfig {
                data_dir: dirs::data_dir()
                    .unwrap_or_else(|| std::env::temp_dir())
                    .join("magray_minimal"),
                temp_dir: std::env::temp_dir().join("magray_minimal_temp"),
                log_level: "error".to_string(),
                thread_pool_size: 1,
                max_memory_mb: 256, // 256MB limit
                dev_mode: false,
            },
            ai: AiConfig::minimal(),
            memory: MemorySystemConfig {
                database: DatabaseConfig {
                    pool_size: 1,
                    wal_mode: false, // Less resource intensive
                    ..Default::default()
                },
                cache: CacheConfig::minimal(),
                batch: BatchConfig::minimal(),
                hnsw: HnswConfig::small_dataset(),
                promotion: PromotionConfig::minimal(),
                ml_promotion: None, // Disabled
                streaming: None,    // Disabled
            },
            orchestration: OrchestrationConfig {
                health: HealthMonitorConfig::minimal(),
                resources: ResourceConfig::minimal(),
                notifications: NotificationConfig::minimal(),
                circuit_breaker: CircuitBreakerConfigBase::default(),
                retry: RetryConfigBase {
                    max_retries: 1,
                    ..Default::default()
                },
            },
            performance: PerformanceConfig {
                monitoring: MonitoringConfigBase {
                    enable_metrics: false,
                    enable_tracing: false,
                    metrics_interval_seconds: 600, // Very infrequent
                    sample_rate: 0.0,
                },
                thresholds: PerformanceThresholds {
                    max_memory_usage_percent: 95.0, // More aggressive in minimal
                    ..Default::default()
                },
                profiling: ProfilingConfig {
                    cpu_profiling: false,
                    memory_profiling: false,
                    ..Default::default()
                },
            },
            security: SecurityConfig {
                audit_logging: false,
                rate_limiting: RateLimitConfig {
                    enabled: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            features: FeatureFlags {
                gpu_acceleration: false,
                simd_optimizations: false,
                ml_promotion: false,
                streaming_api: false,
                experimental: false,
                ..Default::default()
            },
            env_overrides: HashMap::new(),
        }
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<()> {
        // Core overrides
        if let Ok(log_level) = env::var("MAGRAY_LOG_LEVEL") {
            self.core.log_level = log_level;
        }

        if let Ok(threads) = env::var("MAGRAY_THREADS") {
            self.core.thread_pool_size = threads
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid MAGRAY_THREADS value: {}", e))?;
        }

        if let Ok(max_memory) = env::var("MAGRAY_MAX_MEMORY_MB") {
            self.core.max_memory_mb = max_memory
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid MAGRAY_MAX_MEMORY_MB value: {}", e))?;
        }

        // Data directory override
        if let Ok(data_dir) = env::var("MAGRAY_DATA_DIR") {
            self.core.data_dir = PathBuf::from(data_dir);
            self.memory.database.db_path = self.core.data_dir.join("memory.db");
        }

        // GPU override
        if let Ok(gpu_enabled) = env::var("MAGRAY_GPU_ENABLED") {
            self.features.gpu_acceleration = gpu_enabled.parse().unwrap_or(false);
            self.ai.embedding.use_gpu = self.features.gpu_acceleration;
            self.ai.reranking.use_gpu = self.features.gpu_acceleration;
        }

        // SIMD override
        if let Ok(simd_enabled) = env::var("MAGRAY_SIMD_ENABLED") {
            self.features.simd_optimizations = simd_enabled.parse().unwrap_or(false);
        }

        // Development mode override
        if let Ok(dev_mode) = env::var("MAGRAY_DEV_MODE") {
            self.core.dev_mode = dev_mode.parse().unwrap_or(false);
        }

        Ok(())
    }

    /// Comprehensive validation of the entire configuration
    pub fn validate(&self) -> Result<ValidationReport> {
        let mut report = ValidationReport::new();

        // Validate core configuration
        if let Err(e) = self.validate_core() {
            report.errors.push(ValidationError::Core(e.to_string()));
        }

        // Validate AI configuration
        if let Err(e) = self.validate_ai() {
            report.errors.push(ValidationError::AI(e.to_string()));
        }

        // Validate memory configuration
        if let Err(e) = self.validate_memory() {
            report.errors.push(ValidationError::Memory(e.to_string()));
        }

        // Validate feature compatibility
        if let Err(e) = self.validate_features() {
            report.errors.push(ValidationError::Features(e.to_string()));
        }

        // Validate paths exist or can be created
        if let Err(e) = self.validate_paths() {
            report
                .warnings
                .push(ValidationWarning::Paths(e.to_string()));
        }

        // Performance recommendations
        self.add_performance_recommendations(&mut report);

        Ok(report)
    }

    fn validate_core(&self) -> Result<()> {
        if self.core.thread_pool_size > 256 {
            return Err(anyhow::anyhow!(
                "Thread pool size {} is excessive",
                self.core.thread_pool_size
            ));
        }

        if self.core.max_memory_mb > 0 && self.core.max_memory_mb < 64 {
            return Err(anyhow::anyhow!(
                "Max memory {} MB is too low",
                self.core.max_memory_mb
            ));
        }

        Ok(())
    }

    fn validate_ai(&self) -> Result<()> {
        if self.ai.embedding.model_name.is_empty() {
            return Err(anyhow::anyhow!("Embedding model name cannot be empty"));
        }

        if self.ai.embedding.batch_size == 0 {
            return Err(anyhow::anyhow!("Embedding batch size must be > 0"));
        }

        if self.ai.embedding.batch_size > 1024 {
            return Err(anyhow::anyhow!(
                "Embedding batch size {} is too large",
                self.ai.embedding.batch_size
            ));
        }

        Ok(())
    }

    fn validate_memory(&self) -> Result<()> {
        if let Err(e) = self.memory.hnsw.validate() {
            return Err(anyhow::anyhow!("HNSW config validation failed: {}", e));
        }

        if self.memory.batch.max_batch_size == 0 {
            return Err(anyhow::anyhow!("Batch max size must be > 0"));
        }

        if self.memory.database.pool_size == 0 {
            return Err(anyhow::anyhow!("Database pool size must be > 0"));
        }

        Ok(())
    }

    fn validate_features(&self) -> Result<()> {
        if self.features.gpu_acceleration && !self.ai.embedding.use_gpu {
            return Err(anyhow::anyhow!(
                "GPU acceleration enabled but AI embedding not configured for GPU"
            ));
        }

        Ok(())
    }

    fn validate_paths(&self) -> Result<()> {
        // Try to create directories if they don't exist
        std::fs::create_dir_all(&self.core.data_dir)?;
        std::fs::create_dir_all(&self.core.temp_dir)?;

        if let Some(parent) = self.memory.database.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    fn add_performance_recommendations(&self, report: &mut ValidationReport) {
        // Memory recommendations
        if self.core.max_memory_mb > 0 && self.core.max_memory_mb < 1024 {
            report.recommendations.push(
                "Consider increasing max_memory_mb to at least 1024 for better performance"
                    .to_string(),
            );
        }

        // Thread pool recommendations
        if self.core.thread_pool_size == 1 && self.environment != Environment::Minimal {
            report
                .recommendations
                .push("Consider increasing thread_pool_size for better parallelism".to_string());
        }

        // HNSW recommendations
        if self.memory.hnsw.ef_search < 32 && self.environment == Environment::Production {
            report.recommendations.push(
                "Consider increasing HNSW ef_search for better search quality in production"
                    .to_string(),
            );
        }
    }

    /// Get human-readable configuration summary
    pub fn summary(&self) -> String {
        format!(
            "MAGRAY Configuration Summary:\n\
            Environment: {}\n\
            Data Dir: {}\n\
            Log Level: {}\n\
            Threads: {}\n\
            Max Memory: {} MB\n\
            GPU Enabled: {}\n\
            SIMD Enabled: {}\n\
            AI Model: {}\n\
            HNSW Quality: ef_search={}, max_connections={}",
            self.environment,
            self.core.data_dir.display(),
            self.core.log_level,
            if self.core.thread_pool_size == 0 {
                "auto".to_string()
            } else {
                self.core.thread_pool_size.to_string()
            },
            if self.core.max_memory_mb == 0 {
                "unlimited".to_string()
            } else {
                self.core.max_memory_mb.to_string()
            },
            self.features.gpu_acceleration,
            self.features.simd_optimizations,
            self.ai.embedding.model_name,
            self.memory.hnsw.ef_search,
            self.memory.hnsw.max_connections,
        )
    }
}

/// Configuration validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub recommendations: Vec<String>,
}

impl ValidationReport {
    fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    Core(String),
    AI(String),
    Memory(String),
    Features(String),
    Integration(String),
}

#[derive(Debug, Clone)]
pub enum ValidationWarning {
    Paths(String),
    Performance(String),
    Compatibility(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Core(msg) => write!(f, "Core configuration error: {}", msg),
            ValidationError::AI(msg) => write!(f, "AI configuration error: {}", msg),
            ValidationError::Memory(msg) => write!(f, "Memory configuration error: {}", msg),
            ValidationError::Features(msg) => write!(f, "Feature configuration error: {}", msg),
            ValidationError::Integration(msg) => write!(f, "Integration error: {}", msg),
        }
    }
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationWarning::Paths(msg) => write!(f, "Path warning: {}", msg),
            ValidationWarning::Performance(msg) => write!(f, "Performance warning: {}", msg),
            ValidationWarning::Compatibility(msg) => write!(f, "Compatibility warning: {}", msg),
        }
    }
}

// Implement backward compatibility
impl Default for MemorySystemConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            cache: CacheConfig::default(),
            batch: BatchConfig::default(),
            hnsw: HnswConfig::default(),
            promotion: PromotionConfig::default(),
            ml_promotion: None,
            streaming: None,
        }
    }
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            health: HealthMonitorConfig::default(),
            resources: ResourceConfig::default(),
            notifications: NotificationConfig::default(),
            circuit_breaker: CircuitBreakerConfigBase::default(),
            retry: RetryConfigBase::default(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfigBase::default(),
            thresholds: PerformanceThresholds::default(),
            profiling: ProfilingConfig::default(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            audit_logging: false,
            secure_random_seed: None,
            rate_limiting: RateLimitConfig::default(),
            authentication: AuthenticationConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_presets() -> Result<()> {
        let prod_config = UnifiedDIConfiguration::production();
        assert_eq!(prod_config.environment, Environment::Production);
        assert!(prod_config.features.gpu_acceleration);
        assert_eq!(prod_config.core.log_level, "warn");

        let dev_config = UnifiedDIConfiguration::development();
        assert_eq!(dev_config.environment, Environment::Development);
        assert_eq!(dev_config.core.log_level, "debug");
        assert!(dev_config.core.dev_mode);

        let test_config = UnifiedDIConfiguration::test();
        assert_eq!(test_config.environment, Environment::Test);
        assert!(!test_config.features.gpu_acceleration);
        assert_eq!(test_config.core.log_level, "error");

        let minimal_config = UnifiedDIConfiguration::minimal();
        assert_eq!(minimal_config.environment, Environment::Minimal);
        assert_eq!(minimal_config.core.thread_pool_size, 1);
        assert_eq!(minimal_config.core.max_memory_mb, 256);

        Ok(())
    }

    #[test]
    fn test_configuration_validation() -> Result<()> {
        let mut config = UnifiedDIConfiguration::development();

        // Valid configuration should pass
        let report = config.validate()?;
        assert!(report.is_valid());

        // Invalid configuration should fail
        config.ai.embedding.model_name = "".to_string();
        let report = config.validate()?;
        assert!(!report.is_valid());
        assert!(!report.errors.is_empty());

        Ok(())
    }

    #[test]
    fn test_environment_variable_overrides() -> Result<()> {
        std::env::set_var("MAGRAY_LOG_LEVEL", "trace");
        std::env::set_var("MAGRAY_THREADS", "8");
        std::env::set_var("MAGRAY_GPU_ENABLED", "false");

        let mut config = UnifiedDIConfiguration::development();
        config.apply_env_overrides()?;

        assert_eq!(config.core.log_level, "trace");
        assert_eq!(config.core.thread_pool_size, 8);
        assert!(!config.features.gpu_acceleration);
        assert!(!config.ai.embedding.use_gpu);

        // Cleanup
        std::env::remove_var("MAGRAY_LOG_LEVEL");
        std::env::remove_var("MAGRAY_THREADS");
        std::env::remove_var("MAGRAY_GPU_ENABLED");

        Ok(())
    }

    #[test]
    fn test_configuration_summary() {
        let config = UnifiedDIConfiguration::production();
        let summary = config.summary();

        assert!(summary.contains("Environment: production"));
        assert!(summary.contains("GPU Enabled: true"));
        assert!(summary.contains("Log Level: warn"));
    }
}
