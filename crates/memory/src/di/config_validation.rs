//! Configuration Validation Engine
//!
//! This module provides comprehensive validation for the unified configuration system.
//! It performs deep validation checks, cross-component compatibility validation,
//! and provides detailed error reporting and recommendations.

use anyhow::Result;
use std::{collections::HashMap, path::Path};

use super::unified_config::{
    Environment, UnifiedDIConfiguration, ValidationError, ValidationReport, ValidationWarning,
};

/// Configuration validation engine with comprehensive checks
pub struct ConfigurationValidator {
    /// Custom validation rules
    custom_rules: Vec<Box<dyn ValidationRule>>,
    /// Environment-specific validations
    env_rules: HashMap<Environment, Vec<Box<dyn ValidationRule>>>,
    /// System constraints detector
    system_detector: SystemConstraintsDetector,
}

impl Default for ConfigurationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationValidator {
    /// Create a new validator with default rules
    pub fn new() -> Self {
        let mut validator = Self {
            custom_rules: Vec::new(),
            env_rules: HashMap::new(),
            system_detector: SystemConstraintsDetector::new(),
        };

        validator.register_default_rules();
        validator
    }

    /// Register default validation rules
    fn register_default_rules(&mut self) {
        // Core validation rules
        self.add_rule(Box::new(CoreConfigValidation));
        self.add_rule(Box::new(AIConfigValidation));
        self.add_rule(Box::new(MemoryConfigValidation));
        self.add_rule(Box::new(PerformanceValidation));
        self.add_rule(Box::new(SecurityValidation));
        self.add_rule(Box::new(CompatibilityValidation));

        // Environment-specific rules
        self.add_env_rule(Environment::Production, Box::new(ProductionValidation));
        self.add_env_rule(Environment::Test, Box::new(TestValidation));
    }

    /// Add a custom validation rule
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.custom_rules.push(rule);
    }

    /// Add environment-specific validation rule
    pub fn add_env_rule(&mut self, env: Environment, rule: Box<dyn ValidationRule>) {
        self.env_rules.entry(env).or_default().push(rule);
    }

    /// Perform comprehensive validation
    pub fn validate(&self, config: &UnifiedDIConfiguration) -> Result<ValidationReport> {
        let mut report = ValidationReport {
            errors: Vec::new(),
            warnings: Vec::new(),
            recommendations: Vec::new(),
        };

        // System constraints check
        if let Ok(constraints) = self.system_detector.detect() {
            self.validate_against_system(&config, &constraints, &mut report);
        }

        // Apply general validation rules
        for rule in &self.custom_rules {
            rule.validate(config, &mut report)?;
        }

        // Apply environment-specific rules
        if let Some(env_rules) = self.env_rules.get(&config.environment) {
            for rule in env_rules {
                rule.validate(config, &mut report)?;
            }
        }

        // Cross-component validation
        self.validate_cross_component_compatibility(config, &mut report);

        // Add performance recommendations
        self.add_performance_recommendations(config, &mut report);

        Ok(report)
    }

    /// Validate configuration against detected system constraints
    fn validate_against_system(
        &self,
        config: &UnifiedDIConfiguration,
        constraints: &SystemConstraints,
        report: &mut ValidationReport,
    ) {
        // Memory validation
        if config.core.max_memory_mb > 0 {
            let requested_mb = config.core.max_memory_mb as u64;
            let available_mb = constraints.total_memory_mb;

            if requested_mb > available_mb {
                report.errors.push(ValidationError::Core(format!(
                    "Requested memory {}MB exceeds available {}MB",
                    requested_mb, available_mb
                )));
            } else if requested_mb > (available_mb * 90 / 100) {
                report.warnings.push(ValidationWarning::Performance(format!(
                    "Requested memory {}MB is >90% of available {}MB",
                    requested_mb, available_mb
                )));
            }
        }

        // CPU validation
        if config.core.thread_pool_size > constraints.cpu_cores {
            report.warnings.push(ValidationWarning::Performance(format!(
                "Thread pool size {} exceeds CPU cores {}",
                config.core.thread_pool_size, constraints.cpu_cores
            )));
        }

        // Disk space validation
        let estimated_disk_usage = self.estimate_disk_usage(config);
        if estimated_disk_usage > constraints.available_disk_mb {
            report.errors.push(ValidationError::Core(format!(
                "Estimated disk usage {}MB exceeds available {}MB",
                estimated_disk_usage, constraints.available_disk_mb
            )));
        }
    }

    /// Validate cross-component compatibility
    fn validate_cross_component_compatibility(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) {
        // GPU feature consistency
        if config.features.gpu_acceleration {
            if !config.ai.embedding.use_gpu && !config.ai.reranking.use_gpu {
                report.warnings.push(ValidationWarning::Compatibility(
                    "GPU acceleration enabled but no AI components configured for GPU".to_string(),
                ));
            }
        }

        // SIMD consistency
        if config.features.simd_optimizations {
            // Check if environment supports SIMD (this is a simplified check)
            if config.environment == Environment::Test {
                report.warnings.push(ValidationWarning::Compatibility(
                    "SIMD optimizations may cause test determinism issues".to_string(),
                ));
            }
        }

        // Memory configuration consistency
        if config.memory.hnsw.max_elements > 1_000_000 && config.core.max_memory_mb < 4096 {
            report.warnings.push(ValidationWarning::Performance(
                "Large HNSW index with limited memory may cause performance issues".to_string(),
            ));
        }

        // Batch size consistency
        if config.memory.batch.max_batch_size > config.ai.embedding.batch_size * 10 {
            report.warnings.push(ValidationWarning::Performance(
                "Memory batch size much larger than AI batch size may cause inefficiency"
                    .to_string(),
            ));
        }
    }

    /// Add performance-specific recommendations
    fn add_performance_recommendations(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) {
        // HNSW recommendations
        let hnsw = &config.memory.hnsw;
        if hnsw.ef_search < 32 && config.environment == Environment::Production {
            report.recommendations.push(
                "Consider increasing HNSW ef_search to ≥32 for better search quality in production"
                    .to_string(),
            );
        }

        if hnsw.max_connections < 16 && config.environment == Environment::Production {
            report.recommendations.push(
                "Consider increasing HNSW max_connections to ≥16 for better connectivity"
                    .to_string(),
            );
        }

        // Thread recommendations
        if config.core.thread_pool_size == 1 && config.environment != Environment::Minimal {
            report.recommendations.push(
                "Consider using more threads for better parallelism (0 for auto-detection)"
                    .to_string(),
            );
        }

        // Cache recommendations
        if config.memory.cache.base.max_cache_size < 10000
            && config.environment == Environment::Production
        {
            report.recommendations.push(
                "Consider increasing cache size for better hit rates in production".to_string(),
            );
        }

        // Batch size recommendations
        if config.memory.batch.max_batch_size < 1000
            && config.environment == Environment::Production
        {
            report.recommendations.push(
                "Consider increasing batch size for better throughput in production".to_string(),
            );
        }
    }

    /// Estimate disk usage based on configuration
    fn estimate_disk_usage(&self, config: &UnifiedDIConfiguration) -> u64 {
        let mut total_mb = 0u64;

        // Database size estimation
        let vector_size = config.memory.hnsw.dimension * 4; // f32 = 4 bytes
        let estimated_vectors = config.memory.hnsw.max_elements;
        let db_size_mb = (estimated_vectors * vector_size / 1024 / 1024) as u64;
        total_mb += db_size_mb;

        // Cache size estimation
        total_mb += (config.memory.cache.base.max_cache_size * vector_size / 1024 / 1024) as u64;

        // Add 50% overhead for metadata, indices, etc.
        total_mb = total_mb * 150 / 100;

        total_mb
    }
}

/// Trait for validation rules
pub trait ValidationRule: Send + Sync {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()>;
    fn rule_name(&self) -> &'static str;
}

/// Core configuration validation
struct CoreConfigValidation;

impl ValidationRule for CoreConfigValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Thread pool validation
        if config.core.thread_pool_size > 256 {
            report.errors.push(ValidationError::Core(format!(
                "Thread pool size {} is excessive (max recommended: 256)",
                config.core.thread_pool_size
            )));
        }

        // Memory validation
        if config.core.max_memory_mb > 0 && config.core.max_memory_mb < 64 {
            report.errors.push(ValidationError::Core(format!(
                "Memory limit {}MB is too low (minimum: 64MB)",
                config.core.max_memory_mb
            )));
        }

        // Log level validation
        let valid_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_levels.contains(&config.core.log_level.as_str()) {
            report.errors.push(ValidationError::Core(format!(
                "Invalid log level '{}', must be one of: {:?}",
                config.core.log_level, valid_levels
            )));
        }

        // Path validation
        if !config.core.data_dir.is_absolute() {
            report.warnings.push(ValidationWarning::Paths(
                "Data directory should be an absolute path for production use".to_string(),
            ));
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "CoreConfigValidation"
    }
}

/// AI configuration validation
struct AIConfigValidation;

impl ValidationRule for AIConfigValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let ai = &config.ai;

        // Embedding validation
        if ai.embedding.model_name.is_empty() {
            report.errors.push(ValidationError::AI(
                "Embedding model name cannot be empty".to_string(),
            ));
        }

        if ai.embedding.batch_size == 0 {
            report.errors.push(ValidationError::AI(
                "Embedding batch size must be > 0".to_string(),
            ));
        }

        if ai.embedding.batch_size > 1024 {
            report.warnings.push(ValidationWarning::Performance(format!(
                "Large embedding batch size {} may cause memory issues",
                ai.embedding.batch_size
            )));
        }

        if ai.embedding.max_length > 8192 {
            report.warnings.push(ValidationWarning::Performance(format!(
                "Large max_length {} may impact performance",
                ai.embedding.max_length
            )));
        }

        // Reranking validation
        if ai.reranking.model_name.is_empty() {
            report.warnings.push(ValidationWarning::Compatibility(
                "Reranking model name is empty - reranking will be disabled".to_string(),
            ));
        }

        // GPU configuration consistency
        if ai.embedding.use_gpu != ai.reranking.use_gpu {
            report.warnings.push(ValidationWarning::Compatibility(
                "Embedding and reranking have different GPU settings - may cause GPU memory issues"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "AIConfigValidation"
    }
}

/// Memory system validation
struct MemoryConfigValidation;

impl ValidationRule for MemoryConfigValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let memory = &config.memory;

        // HNSW validation
        if let Err(e) = memory.hnsw.validate() {
            report.errors.push(ValidationError::Memory(format!(
                "HNSW validation failed: {}",
                e
            )));
        }

        // Batch validation
        if memory.batch.max_batch_size == 0 {
            report.errors.push(ValidationError::Memory(
                "Batch max size must be > 0".to_string(),
            ));
        }

        if memory.batch.worker_threads == 0 {
            report.errors.push(ValidationError::Memory(
                "Batch worker threads must be > 0".to_string(),
            ));
        }

        if memory.batch.worker_threads > 64 {
            report.warnings.push(ValidationWarning::Performance(format!(
                "High number of batch worker threads {} may cause contention",
                memory.batch.worker_threads
            )));
        }

        // Database validation
        if memory.database.pool_size == 0 {
            report.errors.push(ValidationError::Memory(
                "Database pool size must be > 0".to_string(),
            ));
        }

        if memory.database.pool_size > 100 {
            report.warnings.push(ValidationWarning::Performance(format!(
                "Large database pool size {} may waste resources",
                memory.database.pool_size
            )));
        }

        // Cache validation
        if memory.cache.base.max_cache_size == 0 {
            report.warnings.push(ValidationWarning::Performance(
                "Cache size is 0 - caching disabled".to_string(),
            ));
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "MemoryConfigValidation"
    }
}

/// Performance configuration validation
struct PerformanceValidation;

impl ValidationRule for PerformanceValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let perf = &config.performance;

        // Threshold validation
        if perf.thresholds.max_response_time_ms == 0 {
            report.warnings.push(ValidationWarning::Performance(
                "Max response time threshold is 0 - no response time monitoring".to_string(),
            ));
        }

        if perf.thresholds.max_memory_usage_percent > 100.0 {
            report.errors.push(ValidationError::Core(format!(
                "Invalid memory usage threshold: {}%",
                perf.thresholds.max_memory_usage_percent
            )));
        }

        if perf.thresholds.max_cpu_usage_percent > 100.0 {
            report.errors.push(ValidationError::Core(format!(
                "Invalid CPU usage threshold: {}%",
                perf.thresholds.max_cpu_usage_percent
            )));
        }

        // Monitoring validation
        if perf.monitoring.sample_rate < 0.0 || perf.monitoring.sample_rate > 1.0 {
            report.errors.push(ValidationError::Core(format!(
                "Invalid sample rate: {} (must be 0.0-1.0)",
                perf.monitoring.sample_rate
            )));
        }

        // Profiling validation
        if perf.profiling.cpu_profiling && config.environment == Environment::Production {
            report.warnings.push(ValidationWarning::Performance(
                "CPU profiling enabled in production - may impact performance".to_string(),
            ));
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "PerformanceValidation"
    }
}

/// Security configuration validation
struct SecurityValidation;

impl ValidationRule for SecurityValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        let security = &config.security;

        // Rate limiting validation
        if security.rate_limiting.enabled {
            if security.rate_limiting.requests_per_second == 0 {
                report.errors.push(ValidationError::Core(
                    "Rate limiting enabled but requests_per_second is 0".to_string(),
                ));
            }

            if security.rate_limiting.burst_capacity < security.rate_limiting.requests_per_second {
                report.warnings.push(ValidationWarning::Compatibility(
                    "Burst capacity should be >= requests_per_second for rate limiting".to_string(),
                ));
            }
        }

        // Authentication validation
        if security.authentication.enabled {
            if security.authentication.jwt_secret.is_none() {
                report.errors.push(ValidationError::Core(
                    "Authentication enabled but no JWT secret configured".to_string(),
                ));
            }

            if security.authentication.token_expiration_seconds < 300 {
                report.warnings.push(ValidationWarning::Compatibility(
                    "Very short token expiration may cause frequent re-authentication".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "SecurityValidation"
    }
}

/// Cross-component compatibility validation
struct CompatibilityValidation;

impl ValidationRule for CompatibilityValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // GPU and AI compatibility
        if config.features.gpu_acceleration && !config.ai.embedding.use_gpu {
            report.warnings.push(ValidationWarning::Compatibility(
                "GPU acceleration enabled globally but embedding not using GPU".to_string(),
            ));
        }

        // Memory and performance compatibility
        if config.core.max_memory_mb > 0 {
            let estimated_usage = config
                .memory
                .hnsw
                .estimate_memory_usage(config.memory.hnsw.max_elements);
            let estimated_mb = (estimated_usage / 1024 / 1024) as usize;

            if estimated_mb > config.core.max_memory_mb {
                report.warnings.push(ValidationWarning::Performance(format!(
                    "HNSW estimated memory usage {}MB exceeds limit {}MB",
                    estimated_mb, config.core.max_memory_mb
                )));
            }
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "CompatibilityValidation"
    }
}

/// Production environment validation
struct ProductionValidation;

impl ValidationRule for ProductionValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Production-specific checks
        if config.core.dev_mode {
            report.warnings.push(ValidationWarning::Compatibility(
                "Development mode enabled in production environment".to_string(),
            ));
        }

        if config.core.log_level == "trace" || config.core.log_level == "debug" {
            report.warnings.push(ValidationWarning::Performance(format!(
                "Verbose logging '{}' in production may impact performance",
                config.core.log_level
            )));
        }

        if !config.security.audit_logging {
            report
                .recommendations
                .push("Consider enabling audit logging in production for security".to_string());
        }

        if !config.performance.monitoring.enable_metrics {
            report.warnings.push(ValidationWarning::Performance(
                "Metrics disabled in production - monitoring will be limited".to_string(),
            ));
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "ProductionValidation"
    }
}

/// Test environment validation
struct TestValidation;

impl ValidationRule for TestValidation {
    fn validate(
        &self,
        config: &UnifiedDIConfiguration,
        report: &mut ValidationReport,
    ) -> Result<()> {
        // Test-specific checks
        if config.features.gpu_acceleration {
            report.recommendations.push(
                "Consider disabling GPU acceleration in tests for reproducibility".to_string(),
            );
        }

        if config.security.secure_random_seed.is_none() {
            report
                .recommendations
                .push("Consider setting secure_random_seed for deterministic tests".to_string());
        }

        if config.performance.profiling.cpu_profiling
            || config.performance.profiling.memory_profiling
        {
            report
                .recommendations
                .push("Consider disabling profiling in tests for performance".to_string());
        }

        Ok(())
    }

    fn rule_name(&self) -> &'static str {
        "TestValidation"
    }
}

/// System constraints detector
struct SystemConstraintsDetector;

impl SystemConstraintsDetector {
    fn new() -> Self {
        Self
    }

    fn detect(&self) -> Result<SystemConstraints> {
        Ok(SystemConstraints {
            cpu_cores: self.detect_cpu_cores(),
            total_memory_mb: self.detect_total_memory()?,
            available_disk_mb: self.detect_available_disk()?,
            has_gpu: self.detect_gpu(),
            has_simd: self.detect_simd(),
        })
    }

    fn detect_cpu_cores(&self) -> usize {
        num_cpus::get()
    }

    fn detect_total_memory(&self) -> Result<u64> {
        // This is a simplified implementation
        // In a real system, you'd use proper system APIs
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("free").arg("-m").output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.starts_with("Mem:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 1 {
                            if let Ok(mb) = parts[1].parse::<u64>() {
                                return Ok(mb);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: assume 8GB
        Ok(8192)
    }

    fn detect_available_disk(&self) -> Result<u64> {
        // Simplified implementation
        Ok(10240) // Assume 10GB available
    }

    fn detect_gpu(&self) -> bool {
        // Simplified GPU detection
        std::env::var("CUDA_VISIBLE_DEVICES").is_ok()
            || Path::new("/usr/bin/nvidia-smi").exists()
            || Path::new("/opt/rocm").exists()
    }

    fn detect_simd(&self) -> bool {
        // Most modern CPUs support SIMD
        true
    }
}

/// Detected system constraints
struct SystemConstraints {
    cpu_cores: usize,
    total_memory_mb: u64,
    available_disk_mb: u64,
    has_gpu: bool,
    has_simd: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::di::config_presets::ConfigPresets;

    #[test]
    fn test_validation_engine() -> Result<()> {
        let validator = ConfigurationValidator::new();
        let config = ConfigPresets::production_server();

        let report = validator.validate(&config)?;
        assert!(report.is_valid());

        Ok(())
    }

    #[test]
    fn test_invalid_config_validation() -> Result<()> {
        let validator = ConfigurationValidator::new();
        let mut config = UnifiedDIConfiguration::development();

        // Make configuration invalid
        config.ai.embedding.model_name = "".to_string();
        config.core.thread_pool_size = 500; // Too high
        config.core.log_level = "invalid".to_string();

        let report = validator.validate(&config)?;
        assert!(!report.is_valid());
        assert!(!report.errors.is_empty());

        Ok(())
    }

    #[test]
    fn test_performance_recommendations() -> Result<()> {
        let validator = ConfigurationValidator::new();
        let mut config = ConfigPresets::production_server();

        // Set low values that should trigger recommendations
        config.memory.hnsw.ef_search = 8;
        config.memory.cache.base.max_cache_size = 100;

        let report = validator.validate(&config)?;
        assert!(!report.recommendations.is_empty());

        Ok(())
    }
}
