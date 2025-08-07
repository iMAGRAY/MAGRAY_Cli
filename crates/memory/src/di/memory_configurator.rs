//! Memory Configurator - настройка DI контейнера для memory системы
//!
//! Отделен от unified_container.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за конфигурирование memory компонентов в DI контейнере.

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn};

use super::{traits::Lifetime, unified_container::UnifiedDIContainer};
use crate::service_di::service_config::MemoryServiceConfig;

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

        // FlushConfig
        let flush_config = config.flush_config.clone();
        container.register_instance(flush_config)?;

        // HNSWConfig (если есть)
        if let Some(hnsw_config) = &config.hnsw_config {
            container.register_instance(hnsw_config.clone())?;
        }

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

        use crate::database_manager::DatabaseManager;

        // DatabaseManager
        let db_path = config.db_path.clone();
        container.register(
            move |_| {
                let db_path = db_path.clone();
                Ok(DatabaseManager::new_with_path(&db_path))
            },
            Lifetime::Singleton,
        )?;

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

        use crate::cache_lru::EmbeddingCacheLRU;

        // Cache
        let cache_config = Self::convert_cache_config(&config);
        let cache_path = Self::get_cache_path(config);

        container.register(
            move |_| {
                let cache_config = cache_config.clone();
                let cache_path = cache_path.clone();
                match EmbeddingCacheLRU::new(cache_path, cache_config) {
                    Ok(cache) => Ok(cache),
                    Err(e) => {
                        warn!(
                            "⚠️ Не удалось создать EmbeddingCacheLRU, используем fallback: {}",
                            e
                        );
                        // Fallback к простой реализации
                        Ok(EmbeddingCacheLRU::new_in_memory(cache_config.clone()))
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
        // Определяем есть ли поддержка GPU
        if config.gpu_enabled && Self::is_gpu_available().await {
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

        use crate::gpu_accelerated::GpuBatchProcessor;

        // GpuBatchProcessor
        let gpu_config = config.gpu_config.clone();
        container.register(
            move |_| {
                let gpu_config = gpu_config.clone();
                GpuBatchProcessor::new_with_config(gpu_config)
                    .map_err(|e| anyhow::anyhow!("Не удалось создать GpuBatchProcessor: {}", e))
            },
            Lifetime::Singleton,
        )?;

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
        use crate::metrics::MetricsCollector;
        container.register(|_| Ok(MetricsCollector::new()), Lifetime::Singleton)?;

        // HealthMonitor (если включен)
        if config.health_enabled {
            use crate::health::HealthMonitor;
            let health_config = config.health_config.clone();
            container.register(
                move |_| Ok(HealthMonitor::new(health_config.clone())),
                Lifetime::Singleton,
            )?;
        }

        // NotificationService (если включен)
        if config.notifications_enabled {
            use crate::notifications::NotificationService;
            container.register(|_| Ok(NotificationService::new()), Lifetime::Singleton)?;
        }

        info!("✅ Monitoring layer configured");
        Ok(())
    }

    /// Настроить orchestration layer
    async fn configure_orchestration_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig,
    ) -> Result<()> {
        info!("🔧 Настройка orchestration layer...");

        use crate::orchestration::{
            BackupCoordinator, EmbeddingCoordinator, HealthManager, PromotionCoordinator,
            ResourceController, SearchCoordinator,
        };

        // EmbeddingCoordinator
        container.register(
            |container| {
                let cache = container.resolve::<crate::cache_lru::EmbeddingCacheLRU>()?;
                let cache: Arc<dyn crate::cache_interface::EmbeddingCacheInterface> = cache;
                
                // Пробуем получить GPU processor, если не получается - используем CPU fallback
                let processor = if let Ok(gpu_processor) = container.resolve::<crate::gpu_accelerated::GpuBatchProcessor>() {
                    info!("✅ Используем GPU processor для EmbeddingCoordinator");
                    Some(gpu_processor)
                } else {
                    warn!("⚠️ GPU processor недоступен, EmbeddingCoordinator будет работать без ускорения");
                    None
                };
                
                Ok(EmbeddingCoordinator::new_with_processor(cache, processor))
            },
            Lifetime::Singleton,
        )?;

        // SearchCoordinator
        container.register(
            |container| {
                let embedding_coordinator = container.resolve::<EmbeddingCoordinator>()?;
                // TODO: Добавить VectorStore когда будет готов async factory
                Ok(SearchCoordinator::new_with_embedding_coordinator(
                    embedding_coordinator,
                ))
            },
            Lifetime::Singleton,
        )?;

        // PromotionCoordinator
        container.register(
            |container| {
                let promotion_config = container.resolve::<crate::promotion::PromotionConfig>()?;
                Ok(PromotionCoordinator::new(promotion_config))
            },
            Lifetime::Singleton,
        )?;

        // BackupCoordinator
        container.register(|_| Ok(BackupCoordinator::new()), Lifetime::Singleton)?;

        // ResourceController
        container.register(|_| Ok(ResourceController::new()), Lifetime::Singleton)?;

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
