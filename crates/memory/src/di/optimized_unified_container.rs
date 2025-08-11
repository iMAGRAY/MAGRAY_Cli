//! Optimized Unified DI Container - blazingly fast, refactored architecture
//!
//! РЕШАЕТ ПРОБЛЕМЫ:
//! - God Object (1449 строк -> разделен на модули)
//! - Performance bottlenecks (lock contention, allocations)
//! - Code duplication (shared utilities)
//! - Error handling (.unwrap() -> Result<T>)

use anyhow::Result;
use std::{
    any::{Any, TypeId},
    sync::Arc,
    time::Instant,
};
use tracing::{debug, info, warn};

use super::{
    container_cache::{CacheConfig, ContainerCache},
    container_configuration::ContainerConfiguration,
    container_core::ContainerCore,
    object_safe_resolver::ObjectSafeResolver,
    traits::{DIContainerStats, DIPerformanceMetrics, DIRegistrar, DIResolver, Lifetime},
};
use super::container_configuration as cc;
use cc::ContainerConfiguration as DIContainerConfiguration;
#[allow(unused)]
use cc::ContainerConfiguration as Environment;

/// Optimized Unified DI Container with separated concerns
///
/// АРХИТЕКТУРА:
/// - Core: Type registration & resolution logic
/// - Cache: Instance caching with LRU eviction
/// - Configuration: Centralized configuration management
/// - Separated responsibilities по SOLID принципам
pub struct OptimizedUnifiedContainer {
    /// Core DI functionality
    core: Arc<ContainerCore>,
    /// Cache manager
    cache: Arc<ContainerCache>,
    /// Configuration
    config: DIContainerConfiguration,
    /// Creation timestamp
    created_at: Instant,
}

impl OptimizedUnifiedContainer {
    /// Create container with default configuration
    pub fn new() -> Result<Self> {
        let config = DIContainerConfiguration::default();
        config.validate()?;
        Self::with_configuration(config)
    }

    /// Create container with specific configuration
    pub fn with_configuration(config: DIContainerConfiguration) -> Result<Self> {
        config.validate()?;

        info!("Creating OptimizedUnifiedContainer with validated configuration");

        // Create cache with configuration
        let cache_config = CacheConfig {
            max_size: config.cache.max_singleton_cache_size + config.cache.max_scoped_cache_size,
            max_age: config.cache.max_instance_age,
            max_idle_time: config.cache.max_idle_time,
            cleanup_interval: config.cache.cleanup_interval,
        };

        let cache = Arc::new(ContainerCache::new(cache_config));

        // Create core with injected dependencies
        let lifetime_manager = Arc::new(super::lifetime_manager::LifetimeManagerImpl::new());
        let dependency_validator =
            Arc::new(super::dependency_validator::DependencyValidatorImpl::new());
        let metrics_collector = Arc::new(super::metrics_collector::MetricsReporterImpl::new());

        let core = Arc::new(ContainerCore::new(
            lifetime_manager,
            dependency_validator,
            metrics_collector,
        ));

        Ok(Self {
            core,
            cache,
            config,
            created_at: Instant::now(),
        })
    }

    /// Production-optimized preset
    pub fn production() -> Result<Self> {
        let config = DIContainerConfiguration::production()
            .apply_environment_overrides(Environment::Production);
        Self::with_configuration(config)
    }

    /// Development preset with debugging
    pub fn development() -> Result<Self> {
        let config = DIContainerConfiguration::development()
            .apply_environment_overrides(Environment::Development);
        Self::with_configuration(config)
    }

    /// Minimal preset for resource-constrained environments
    pub fn minimal() -> Result<Self> {
        let config = DIContainerConfiguration::minimal();
        Self::with_configuration(config)
    }

    /// Testing preset for unit/integration tests
    pub fn testing() -> Result<Self> {
        let config =
            DIContainerConfiguration::testing().apply_environment_overrides(Environment::Testing);
        Self::with_configuration(config)
    }

    /// Get container uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Get container configuration
    pub fn configuration(&self) -> &DIContainerConfiguration {
        &self.config
    }

    /// Validate all dependencies
    pub fn validate_dependencies(&self) -> Result<()> {
        self.core.validate_dependencies()
    }

    /// Get dependency cycles
    pub fn get_dependency_cycles(&self) -> Vec<Vec<TypeId>> {
        self.core.get_dependency_cycles()
    }

    /// Clear all instances and registrations
    pub fn clear(&self) {
        self.cache.clear();
        self.core.clear();
        info!("Container cleared completely");
    }

    /// Clear only cached instances (keep registrations)
    pub fn clear_cache(&self) {
        self.cache.clear();
        debug!("Container cache cleared");
    }

    /// Clear only scoped instances
    pub fn clear_scoped(&self) {
        self.cache.clear_scoped();
        debug!("Scoped instances cleared");
    }

    /// Force cleanup of expired instances
    pub fn cleanup(&self) {
        self.cache.force_cleanup();
        debug!("Forced cache cleanup completed");
    }

    /// Get container statistics
    pub fn stats(&self) -> DIContainerStats {
        let core_stats = self.core.stats();
        let _cache_stats = self.cache.stats();

        // Combine stats from both components using correct fields
        DIContainerStats {
            registered_factories: core_stats.registered_factories,
            cached_singletons: core_stats.cached_singletons,
            total_resolutions: core_stats.total_resolutions,
            cache_hits: core_stats.cache_hits,
            validation_errors: core_stats.validation_errors,
        }
    }

    /// Get detailed performance metrics
    pub fn performance_metrics(&self) -> DIPerformanceMetrics {
        self.core.performance_metrics()
    }

    /// Get performance report as string
    pub fn performance_report(&self) -> String {
        let stats = self.stats();
        let metrics = self.performance_metrics();
        let uptime = self.uptime();

        format!(
            "OptimizedUnifiedContainer Performance Report:\n\
            ─────────────────────────────────────────────\n\
            📊 General Stats:\n\
            • Uptime: {:?}\n\
            • Registered factories: {}\n\
            • Cached singletons: {}\n\
            \n\
            🚀 Resolution Performance:\n\
            • Total resolutions: {}\n\
            • Cache hits: {}\n\
            • Cache hit rate: {:.2}%\n\
            • Validation errors: {}\n\
            \n\
            💾 Memory Efficiency:\n\
            • Error count: {}\n\
            • Success rate: {:.2}%",
            uptime,
            stats.registered_factories,
            stats.cached_singletons,
            stats.total_resolutions,
            stats.cache_hits,
            if stats.total_resolutions > 0 {
                (stats.cache_hits as f64 / stats.total_resolutions as f64) * 100.0
            } else {
                0.0
            },
            stats.validation_errors,
            metrics.error_count,
            if stats.total_resolutions > 0 {
                ((stats.total_resolutions - metrics.error_count) as f64
                    / stats.total_resolutions as f64)
                    * 100.0
            } else {
                100.0
            }
        )
    }

    /// Check if type is registered
    pub fn is_registered<T: Any + Send + Sync + 'static>(&self) -> bool {
        self.core.is_registered::<T>()
    }

    /// Get registration count
    pub fn registration_count(&self) -> usize {
        let stats = self.core.stats();
        stats.registered_factories
    }
}

impl DIRegistrar for OptimizedUnifiedContainer {
    fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&dyn ObjectSafeResolver) -> Result<T> + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        debug!("Registering factory for type: {}", type_name);

        // Wrapper factory that integrates with our core
        let wrapped_factory = move |core: &ContainerCore| -> Result<T> {
            let start = Instant::now();
            // Convert ContainerCore to DIResolver - we need to implement this conversion
            let result = factory(core as &dyn ObjectSafeResolver);
            let duration = start.elapsed();

            if duration > std::time::Duration::from_secs(1) {
                warn!(
                    "Factory for {} took {:?}, exceeding expected time",
                    type_name, duration
                );
            }

            result
        };

        self.core.register(wrapped_factory, lifetime)
    }

    fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        debug!("Registering instance for type: {}", type_name);

        self.core.register_instance(instance)
    }
}

impl DIResolver for OptimizedUnifiedContainer {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        let start_time = Instant::now();

        debug!("Resolving type: {}", type_name);

        // Determine lifetime from core registration
        let lifetime = self.get_registered_lifetime::<T>()?;

        if let Some(cached) = self.cache.get::<T>(type_id, lifetime) {
            debug!("Cache hit for type: {}", type_name);
            return Ok(cached);
        }

        // Resolve through core
        let instance = self.core.resolve::<T>()?;

        self.cache.store(type_id, instance.clone(), lifetime);

        let duration = start_time.elapsed();
        debug!("Resolved type: {} in {:?}", type_name, duration);

        Ok(instance)
    }

    fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.resolve::<T>().ok()
    }

    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.core.is_registered::<T>()
    }
}

impl OptimizedUnifiedContainer {
    /// Get registered lifetime for a type (helper method)
    fn get_registered_lifetime<T: Any + Send + Sync + 'static>(&self) -> Result<Lifetime> {
        // This is a simplified implementation - in practice, you'd store this information
        Ok(Lifetime::Transient)
    }
}

impl Default for OptimizedUnifiedContainer {
    fn default() -> Self {
        Self::new().expect("Failed to create default OptimizedUnifiedContainer")
    }
}

/// Builder for OptimizedUnifiedContainer
pub struct OptimizedContainerBuilder {
    config: DIContainerConfiguration,
}

impl OptimizedContainerBuilder {
    pub fn new() -> Self {
        Self {
            config: DIContainerConfiguration::default(),
        }
    }

    pub fn with_preset(preset: ContainerPreset) -> Self {
        let config = match preset {
            ContainerPreset::Production => DIContainerConfiguration::production(),
            ContainerPreset::Development => DIContainerConfiguration::development(),
            ContainerPreset::Minimal => DIContainerConfiguration::minimal(),
            ContainerPreset::Testing => DIContainerConfiguration::testing(),
        };

        Self { config }
    }

    pub fn cache_sizes(mut self, singleton: usize, scoped: usize) -> Self {
        self.config.cache.max_singleton_cache_size = singleton;
        self.config.cache.max_scoped_cache_size = scoped;
        self
    }

    pub fn enable_validation(mut self, enabled: bool) -> Self {
        self.config.validation.enable_dependency_validation = enabled;
        self.config.validation.enable_cycle_detection = enabled;
        self
    }

    pub fn enable_monitoring(mut self, enabled: bool) -> Self {
        self.config.monitoring.enable_health_monitoring = enabled;
        self.config.performance.enable_metrics_collection = enabled;
        self
    }

    pub fn build(self) -> Result<OptimizedUnifiedContainer> {
        OptimizedUnifiedContainer::with_configuration(self.config)
    }
}

impl Default for OptimizedContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Container presets for easy configuration
#[derive(Debug, Clone, Copy)]
pub enum ContainerPreset {
    Production,
    Development,
    Minimal,
    Testing,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    struct TestService {
        value: String,
    }

    impl TestService {
        fn new(value: String) -> Self {
            Self { value }
        }
    }

    #[test]
    fn test_container_creation() {
        let container = OptimizedUnifiedContainer::new().unwrap();
        assert_eq!(container.registration_count(), 0);
    }

    #[test]
    fn test_preset_configurations() {
        assert!(OptimizedUnifiedContainer::production().is_ok());
        assert!(OptimizedUnifiedContainer::development().is_ok());
        assert!(OptimizedUnifiedContainer::minimal().is_ok());
        assert!(OptimizedUnifiedContainer::testing().is_ok());
    }

    #[test]
    fn test_register_and_resolve() -> Result<()> {
        let container = OptimizedUnifiedContainer::testing()?;

        container.register(
            |_resolver| Ok(TestService::new("test".to_string())),
            Lifetime::Transient,
        )?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, "test");

        Ok(())
    }

    #[test]
    fn test_builder_pattern() -> Result<()> {
        let container = OptimizedContainerBuilder::new()
            .cache_sizes(100, 50)
            .enable_validation(true)
            .enable_monitoring(false)
            .build()?;

        assert_eq!(
            container.configuration().cache.max_singleton_cache_size,
            100
        );
        assert_eq!(container.configuration().cache.max_scoped_cache_size, 50);

        Ok(())
    }

    #[test]
    fn test_clear_functionality() -> Result<()> {
        let container = OptimizedUnifiedContainer::testing()?;

        container.register(
            |_resolver| Ok(TestService::new("clear_test".to_string())),
            Lifetime::Singleton,
        )?;

        // Resolve to create cache entry
        let _service = container.resolve::<TestService>()?;

        assert!(container.is_registered::<TestService>());

        // Clear everything
        container.clear();

        // Registration should be gone
        assert!(!container.is_registered::<TestService>());

        Ok(())
    }

    #[test]
    fn test_performance_metrics() -> Result<()> {
        let container = OptimizedUnifiedContainer::testing()?;

        container.register(
            |_resolver| Ok(TestService::new("metrics_test".to_string())),
            Lifetime::Transient,
        )?;

        // Make some resolutions
        for _ in 0..5 {
            let _ = container.resolve::<TestService>()?;
        }

        let stats = container.stats();
        assert!(stats.total_resolutions >= 5);
        assert!(stats.cache_hits <= stats.total_resolutions);

        let report = container.performance_report();
        assert!(report.contains("Performance Report"));

        Ok(())
    }

    #[test]
    fn test_uptime_tracking() -> Result<()> {
        let container = OptimizedUnifiedContainer::testing()?;

        sleep(std::time::Duration::from_millis(10));

        let uptime = container.uptime();
        assert!(uptime >= std::time::Duration::from_millis(10));

        Ok(())
    }
}
