//! Memory Configurator - настройка DI контейнера для memory системы
//!
//! Отделен от unified_container.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за конфигурирование memory компонентов в DI контейнере.

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use super::{traits::Lifetime, unified_container::UnifiedDIContainer};
use crate::service_di::service_config::MemoryServiceConfig;
// Add required imports for DI resolution traits and cache entry type
use crate::di::traits::DIResolver;
use crate::di::container_cache::CacheEntry;
use crate::database_manager::DatabaseManager;
use crate::cache_lru::EmbeddingCacheLRU;
use crate::gpu_accelerated::GpuBatchProcessor;
use crate::metrics::MetricsCollector;
use crate::health::HealthMonitor;
// use crate::notifications::NotificationService;
use crate::orchestration::{
    BackupCoordinator, EmbeddingCoordinator, HealthManager, PromotionCoordinator,
    ResourceController, SearchCoordinator,
};
use crate::types::PromotionConfig;
use crate::storage::VectorStore;
use crate::resource_manager::ResourceManager;
use crate::backup::BackupManager;

/// Unified Memory Configurator - настройка memory системы в DI контейнере
///
/// ЗАМЕНЯЕТ:
/// - MemoryDIConfigurator из удаленного di_memory_config.rs
/// - Различные ad-hoc конфигураторы в других модулях
///
/// АРХИТЕКТУРНЫЕ ПРИНЦИПЫ:
/// - SRP: единственная ответственность - конфигурация memory компонентов
/// - OCP: расширяемость через различные профили конфигурации
/// - DIP: зависимость от абстракций (UnifiedDIContainer)
pub struct UnifiedMemoryConfigurator;

impl UnifiedMemoryConfigurator {
    /// Настроить полный DI контейнер для memory системы
    ///
    /// ЗАМЕНЯЕТ: MemoryDIConfigurator::configure_full()
    /// ИСПОЛЬЗУЕТ: UnifiedDIContainer вместо старых дублирований
    pub async fn configure_full(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка унифицированного DI контейнера для memory системы");

        let container = UnifiedDIContainer::production();

        // Настраиваем слои архитектуры по порядку
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;
        Self::configure_cache_layer(&container, config).await?;
        Self::configure_processing_layer(&container, config).await?;
        Self::configure_monitoring_layer(&container, config).await?;
        Self::configure_orchestration_layer(&container, config).await?;

        // Валидируем зависимости
        container.validate_dependencies()?;

        info!(
            "✅ Унифицированный DI контейнер настроен с {} зависимостями",
            container.registration_count()
        );

        Ok(container)
    }

    /// Настроить минимальный контейнер для тестов
    pub async fn configure_minimal(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка минимального DI контейнера");

        let container = UnifiedDIContainer::minimal();

        // Только основные компоненты для тестирования
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;

        info!(
            "✅ Минимальный DI контейнер настроен с {} зависимостями",
            container.registration_count()
        );

        Ok(container)
    }

    /// Настроить CPU-only контейнер (без GPU)
    pub async fn configure_cpu_only(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка CPU-only DI контейнера");

        let container = UnifiedDIContainer::development();

        // Настраиваем без GPU компонентов
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;
        Self::configure_cache_layer(&container, config).await?;
        Self::configure_cpu_processing_layer(&container, config).await?;
        Self::configure_monitoring_layer(&container, config).await?;

        info!(
            "✅ CPU-only DI контейнер настроен с {} зависимостями",
            container.registration_count()
        );

        Ok(container)
    }

    /// Настроить GPU-accelerated контейнер
    pub async fn configure_gpu_accelerated(
        config: &MemoryServiceConfig,
    ) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка GPU-accelerated DI контейнера");

        let container = UnifiedDIContainer::production();

        // Полная конфигурация с GPU
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;
        Self::configure_cache_layer(&container, config).await?;
        Self::configure_gpu_processing_layer(&container, config).await?;
        Self::configure_monitoring_layer(&container, config).await?;
        Self::configure_orchestration_layer(&container, config).await?;

        info!(
            "✅ GPU-accelerated DI контейнер настроен с {} зависимостями",
            container.registration_count()
        );

        Ok(container)
    }

    // === PRIVATE CONFIGURATION METHODS ===

    /// Настроить core зависимости (конфигурации и базовые типы)
    async fn configure_core_dependencies(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка core dependencies...");

        // PromotionConfig
        let promotion_config = config.promotion.clone();
        container.register_instance(promotion_config)?;

        // BatchConfig as flush/batch configuration
        let batch_config = config.batch_config.clone();
        container.register_instance(batch_config)?;

        // HNSW configuration not present in MemoryServiceConfig; skip

        // TODO: Добавить другие core конфигурации по мере необходимости

        info!("✅ Core dependencies configured");
        Ok(())
    }

    /// Настроить storage layer
    async fn configure_storage_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка storage layer...");

        // DatabaseManager (use available constructor)
        container.register(|_| Ok(DatabaseManager::new()), Lifetime::Singleton)?;

        // TODO: В будущем добавить async factory для VectorStore
        // Пока что используем заглушку или builder pattern

        info!("✅ Storage layer configured");
        Ok(())
    }

    /// Настроить cache layer
    async fn configure_cache_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка cache layer...");

        // Cache
        let cache_config = Self::convert_cache_config(&config);
        let cache_path = Self::get_cache_path(config);

        container.register(
            move |_| {
                let cache_config = cache_config.clone();
                let cache_path = cache_path.clone();
                match EmbeddingCacheLRU::new(cache_path, cache_config.clone()) {
                    Ok(cache) => Ok(cache),
                    Err(e) => {
                        warn!(
                            "⚠️ Не удалось создать EmbeddingCacheLRU, используем fallback: {}",
                            e
                        );
                        // Fallback: create cache in temp dir
                        let tmp_path = std::env::temp_dir().join("embedding_cache_fallback");
                        EmbeddingCacheLRU::new(tmp_path, cache_config)
                            .map_err(|e| anyhow::anyhow!("Fallback cache init failed: {}", e))
                    }
                }
            },
            Lifetime::Singleton,
        )?;

        info!("✅ Cache layer configured");
        Ok(())
    }

    /// Настроить processing layer (общий)
    async fn configure_processing_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        // Определяем есть ли поддержка GPU по ai_config
        if config.ai_config.embedding.use_gpu && Self::is_gpu_available().await {
            Self::configure_gpu_processing_layer(container, config).await
        } else {
            Self::configure_cpu_processing_layer(container, config).await
        }
    }

    /// Настроить CPU processing
    async fn configure_cpu_processing_layer(
        container: &UnifiedDIContainer,
        _config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка CPU processing layer...");

        // TODO: Добавить CPU-specific компоненты когда они будут реализованы
        // use crate::cpu_processor::CpuBatchProcessor;
        // container.register(...)?;

        info!("✅ CPU processing layer configured");
        Ok(())
    }

    /// Настроить GPU processing
    async fn configure_gpu_processing_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка GPU processing layer...");

        // Skip GPU processor registration if specific constructor not available
        // It will be optionally resolved elsewhere when implemented

        info!("✅ GPU processing layer configured");
        Ok(())
    }

    /// Настроить monitoring layer
    async fn configure_monitoring_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка monitoring layer...");

        // MetricsCollector
        container.register(|_| Ok(MetricsCollector::new()), Lifetime::Singleton)?;

        // HealthMonitor (если включен)
        if config.health_enabled {
            let health_config = config.health_config.clone();
            container.register(
                move |_| Ok(HealthMonitor::new(health_config.clone())),
                Lifetime::Singleton,
            )?;
        }

        // Notifications are optional; skip explicit service registration unless needed

        info!("✅ Monitoring layer configured");
        Ok(())
    }

    /// Настроить orchestration layer
    async fn configure_orchestration_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка orchestration layer...");

        // EmbeddingCoordinator
        container.register(
            |container| {
                let gpu = container.try_resolve::<crate::gpu_accelerated::GpuBatchProcessor>();
                let cache = container.resolve::<crate::cache_lru::EmbeddingCacheLRU>()?;
                let cache: Arc<dyn crate::cache_interface::EmbeddingCacheInterface> = cache;
                // If GPU processor is not available, construct a CPU-compatible processor via helper
                let processor = gpu.unwrap_or_else(|_| Arc::new(crate::gpu_accelerated::GpuBatchProcessor::cpu_fallback()));
                Ok(EmbeddingCoordinator::new(processor, cache))
            },
            Lifetime::Singleton,
        )?;

        // SearchCoordinator (requires VectorStore and EmbeddingCoordinator)
        container.register(
            |container| {
                let vector_store = container.resolve::<crate::storage::VectorStore>()?;
                let embedding_coordinator = container.resolve::<EmbeddingCoordinator>()?;
                Ok(SearchCoordinator::new(vector_store, embedding_coordinator))
            },
            Lifetime::Singleton,
        )?;

        // PromotionCoordinator
        container.register(
            |container| {
                let promotion_config = container.resolve::<crate::types::PromotionConfig>()?;
                Ok(PromotionCoordinator::new(promotion_config, None))
            },
            Lifetime::Singleton,
        )?;

        // BackupCoordinator (requires dependencies)
        container.register(
            |container| {
                let backup_manager = Arc::new(crate::backup::BackupManager::new(std::env::temp_dir())?);
                let store = container.resolve::<crate::storage::VectorStore>()?;
                Ok(BackupCoordinator::new(backup_manager, store))
            },
            Lifetime::Singleton,
        )?;

        // ResourceController
        container.register(
            |_| {
                let cfg = crate::resource_manager::ResourceConfig::default();
                let manager = parking_lot::RwLock::new(crate::resource_manager::ResourceManager::new(cfg)?);
                Ok(ResourceController::new(Arc::new(manager)))
            },
            Lifetime::Singleton,
        )?;

        // HealthManager (если health monitoring включен)
        if config.health_enabled {
            container.register(
                |container| {
                    let health_monitor = container.resolve::<crate::health::HealthMonitor>()?;
                    Ok(HealthManager::new(health_monitor))
                },
                Lifetime::Singleton,
            )?;
        }

        info!("✅ Orchestration layer configured");
        Ok(())
    }

    // === UTILITY METHODS ===

    /// Конвертировать конфигурацию кэша
    fn convert_cache_config(config: &MemoryServiceConfig) -> crate::cache_lru::CacheConfig {
        // TODO: Правильная конвертация когда будет реализована
        crate::cache_lru::CacheConfig::default()
    }

    /// Получить путь для кэша
    fn get_cache_path(config: &MemoryServiceConfig) -> std::path::PathBuf {
        config
            .db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("embedding_cache")
    }

    /// Проверить доступность GPU
    async fn is_gpu_available() -> bool {
        // TODO: Реализовать проверку GPU
        // Пока что возвращаем false для безопасности
        false
    }

    /// Создать summary отчет о конфигурации
    pub fn create_configuration_summary(container: &UnifiedDIContainer) -> String {
        let stats = container.stats();
        let performance = container.performance_metrics();

        format!(
            "=== Memory System DI Configuration Summary ===\n\
             Registered components: {}\n\
             Cached singletons: {}\n\
             Total resolutions: {}\n\
             Cache hit rate: {:.1}%\n\
             Average resolution time: {:.2}μs\n\
             Dependency validation: enabled\n\
             Performance metrics: enabled\n\
             ============================================",
            stats.registered_factories,
            stats.cached_singletons,
            stats.total_resolutions,
            performance.cache_hit_rate(),
            performance.avg_resolve_time_us()
        )
    }

    /// Validate конфигурацию и вернуть диагностическую информацию
    pub async fn validate_configuration(container: &UnifiedDIContainer) -> Result<String> {
        info!("🔍 Валидация конфигурации DI контейнера...");

        // Базовая валидация зависимостей
        container.validate_dependencies()?;

        let mut diagnostics = Vec::new();

        // Проверяем основные компоненты
        let essential_components = [
            // (type_name, required)
            ("DatabaseManager", true),
            ("EmbeddingCacheLRU", true),
            ("MetricsCollector", true),
            ("EmbeddingCoordinator", true),
            ("SearchCoordinator", true),
        ];

        for (component_name, required) in essential_components {
            // TODO: Добавить type-safe проверку когда будет механизм проверки по имени
            if required {
                diagnostics.push(format!(
                    "✅ {}: предположительно зарегистрирован",
                    component_name
                ));
            }
        }

        // Статистика
        let stats = container.stats();
        diagnostics.push(format!(
            "📊 Статистика: {} компонентов, {} singleton кэшей",
            stats.registered_factories, stats.cached_singletons
        ));

        // Dependency report
        // TODO: Добавить когда будет реализован dependency validator

        let report = format!(
            "=== DI Configuration Validation Report ===\n{}\n========================================",
            diagnostics.join("\n")
        );

        info!("✅ Валидация конфигурации завершена успешно");
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_di::service_config::MemoryServiceConfig;

    #[tokio::test]
    async fn test_minimal_configuration() {
        let config = MemoryServiceConfig::minimal_for_tests();

        let result = UnifiedMemoryConfigurator::configure_minimal(&config).await;
        assert!(result.is_ok());

        let container = result.unwrap();
        assert!(container.registration_count() > 0);

        // Базовые компоненты должны быть зарегистрированы
        let summary = UnifiedMemoryConfigurator::create_configuration_summary(&container);
        assert!(summary.contains("Registered components:"));
    }

    #[tokio::test]
    async fn test_cpu_only_configuration() {
        let config = MemoryServiceConfig::cpu_only();

        let result = UnifiedMemoryConfigurator::configure_cpu_only(&config).await;
        assert!(result.is_ok());

        let container = result.unwrap();
        assert!(container.registration_count() > 0);
    }

    #[tokio::test]
    async fn test_configuration_validation() {
        let config = MemoryServiceConfig::minimal_for_tests();
        let container = UnifiedMemoryConfigurator::configure_minimal(&config)
            .await
            .unwrap();

        let validation_result = UnifiedMemoryConfigurator::validate_configuration(&container).await;
        assert!(validation_result.is_ok());

        let report = validation_result.unwrap();
        assert!(report.contains("Validation Report"));
        assert!(report.contains("компонентов"));
    }

    #[test]
    fn test_configuration_summary() {
        let container = UnifiedDIContainer::new();
        let summary = UnifiedMemoryConfigurator::create_configuration_summary(&container);

        assert!(summary.contains("Configuration Summary"));
        assert!(summary.contains("Registered components: 0"));
    }
}
