//! ServiceFactory - Dependency Injection integration для специализированных сервисов
//!
//! Управляет созданием и конфигурацией всех специализированных сервисов:
//! - CoreMemoryService
//! - CoordinatorService  
//! - ResilienceService
//! - MonitoringService
//! - CacheService

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    di::UnifiedContainer,
    services::traits::{
        CacheServiceTrait, CoordinatorServiceTrait, CoreMemoryServiceTrait,
        MonitoringServiceTrait, ResilienceServiceTrait,
    },
    CacheServiceTrait as _, CoordinatorServiceTrait as _, CoreMemoryServiceTrait as _, MonitoringServiceTrait as _, ResilienceServiceTrait as _,
    services::{
        CacheService, CoordinatorService, CoreMemoryService, MonitoringService, ResilienceService,
    },
};

/// Фабрика для создания всех специализированных сервисов
/// Обеспечивает правильное Dependency Injection между сервисами
#[allow(dead_code)]
pub struct ServiceFactory {
    container: Arc<UnifiedContainer>,
}

/// Результат создания всех сервисов
#[allow(dead_code)]
pub struct ServiceCollection {
    pub core_memory: Arc<dyn CoreMemoryServiceTrait>,
    pub coordinator: Arc<dyn CoordinatorServiceTrait>,
    pub resilience: Arc<dyn ResilienceServiceTrait>,
    pub monitoring: Arc<dyn MonitoringServiceTrait>,
    pub cache: Arc<dyn CacheServiceTrait>,
}

/// Конфигурация для создания сервисов
#[derive(Debug, Clone)]
pub struct ServiceFactoryConfig {
    pub max_concurrent_operations: usize,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout_secs: u64,
    pub embedding_dimension: usize,
    pub production_mode: bool,
}

impl Default for ServiceFactoryConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 50,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout_secs: 60,
            embedding_dimension: 1024,
            production_mode: false,
        }
    }
}

impl ServiceFactoryConfig {
    /// Создать конфигурацию для production
    #[allow(dead_code)]
    pub fn production() -> Self {
        Self {
            max_concurrent_operations: 100,
            circuit_breaker_threshold: 3,
            circuit_breaker_timeout_secs: 30,
            embedding_dimension: 1024,
            production_mode: true,
        }
    }

    /// Создать конфигурацию для тестов
    #[allow(dead_code)]
    pub fn test() -> Self {
        Self {
            max_concurrent_operations: 10,
            circuit_breaker_threshold: 10,
            circuit_breaker_timeout_secs: 5,
            embedding_dimension: 512,
            production_mode: false,
        }
    }
}

impl ServiceFactory {
    /// Создать новую фабрику сервисов
    pub fn new(container: Arc<UnifiedContainer>) -> Self {
        info!("🏭 Создание ServiceFactory для инициализации специализированных сервисов");
        Self { container }
    }

    /// Создать все сервисы с конфигурацией по умолчанию
    #[allow(dead_code)]
    pub async fn create_services(&self) -> Result<ServiceCollection> {
        let config = ServiceFactoryConfig::default();
        self.create_services_with_config(config).await
    }

    /// Создать все сервисы с кастомной конфигурацией
    #[allow(dead_code)]
    pub async fn create_services_with_config(
        &self,
        config: ServiceFactoryConfig,
    ) -> Result<ServiceCollection> {
        info!("🏭 Создание всех специализированных сервисов...");
        debug!(
            "🔧 Конфигурация: max_ops={}, threshold={}, timeout={}s, dim={}, prod={}",
            config.max_concurrent_operations,
            config.circuit_breaker_threshold,
            config.circuit_breaker_timeout_secs,
            config.embedding_dimension,
            config.production_mode
        );

        // 1. Создаём базовые сервисы (без зависимостей)
        let resilience = self.create_resilience_service(&config)?;
        let coordinator = self.create_coordinator_service()?;

        // 2. Создаём сервисы с зависимостями
        let monitoring = self.create_monitoring_service(coordinator.clone())?;
        let cache = self.create_cache_service(coordinator.clone(), &config)?;
        let core_memory = self.create_core_memory_service(&config)?;

        // 3. Инициализируем координаторы (требует DI контейнера)
        coordinator.create_coordinators(&self.container).await?;
        coordinator.initialize_coordinators().await?;

        info!("✅ Все специализированные сервисы созданы и инициализированы");

        Ok(ServiceCollection {
            core_memory,
            coordinator,
            resilience,
            monitoring,
            cache,
        })
    }

    /// Создать только core memory service (для минимальных конфигураций)
    #[allow(dead_code)]
    pub fn create_core_memory_only(
        &self,
        config: &ServiceFactoryConfig,
    ) -> Result<Arc<dyn CoreMemoryServiceTrait>> {
        debug!("🗃️ Создание только CoreMemoryService...");
        self.create_core_memory_service(config)
    }

    /// Создать CoreMemoryService
    #[allow(dead_code)]
    fn create_core_memory_service(
        &self,
        config: &ServiceFactoryConfig,
    ) -> Result<Arc<dyn CoreMemoryServiceTrait>> {
        let service = if config.production_mode {
            CoreMemoryService::new_production(self.container.clone())
        } else {
            CoreMemoryService::new(self.container.clone(), config.max_concurrent_operations)
        };

        debug!("✅ CoreMemoryService создан");
        Ok(Arc::new(service))
    }

    /// Создать CoordinatorService
    #[allow(dead_code)]
    fn create_coordinator_service(&self) -> Result<Arc<dyn CoordinatorServiceTrait>> {
        let service = CoordinatorService::new();
        debug!("✅ CoordinatorService создан");
        Ok(Arc::new(service))
    }

    /// Создать ResilienceService
    #[allow(dead_code)]
    fn create_resilience_service(
        &self,
        config: &ServiceFactoryConfig,
    ) -> Result<Arc<dyn ResilienceServiceTrait>> {
        let service = if config.production_mode {
            ResilienceService::new_production()
        } else {
            ResilienceService::new_with_threshold(
                config.circuit_breaker_threshold,
                std::time::Duration::from_secs(config.circuit_breaker_timeout_secs),
            )
        };

        debug!("✅ ResilienceService создан");
        Ok(Arc::new(service))
    }

    /// Создать MonitoringService
    #[allow(dead_code)]
    fn create_monitoring_service(
        &self,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
    ) -> Result<Arc<dyn MonitoringServiceTrait>> {
        let service = MonitoringService::new_with_coordinator(self.container.clone(), coordinator);

        debug!("✅ MonitoringService создан");
        Ok(Arc::new(service))
    }

    /// Создать CacheService
    #[allow(dead_code)]
    fn create_cache_service(
        &self,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
        config: &ServiceFactoryConfig,
    ) -> Result<Arc<dyn CacheServiceTrait>> {
        let mut service = CacheService::new_with_coordinator(self.container.clone(), coordinator);

        // Настраиваем embedding dimension
        service.set_embedding_dimension(config.embedding_dimension);

        debug!("✅ CacheService создан");
        Ok(Arc::new(service))
    }
}

impl ServiceCollection {
    /// Инициализировать все сервисы
    #[allow(dead_code)]
    pub async fn initialize_all(&self) -> Result<()> {
        info!("⚡ Инициализация всех специализированных сервисов...");

        // Запускаем monitoring
        self.monitoring.start_production_monitoring().await?;
        self.monitoring.start_health_monitoring().await?;
        self.monitoring.start_resource_monitoring().await?;

        // Проверяем готовность
        self.monitoring.perform_readiness_checks().await?;

        // Логируем итоговую статистику
        self.monitoring.log_initialization_summary().await;

        info!("✅ Все специализированные сервисы инициализированы");
        Ok(())
    }

    /// Shutdown всех сервисов
    #[allow(dead_code)]
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("🛑 Shutdown всех специализированных сервисов...");

        // Останавливаем координаторы
        self.coordinator.shutdown_coordinators().await?;

        info!("✅ Все специализированные сервисы остановлены");
        Ok(())
    }

    /// Получить статистику всех сервисов
    #[allow(dead_code)]
    pub async fn get_comprehensive_stats(&self) -> Result<ComprehensiveStats> {
        debug!("📊 Сбор статистики всех сервисов...");

        let system_stats = self.monitoring.get_system_stats().await;
        let production_metrics = self.monitoring.get_production_metrics().await?;
        let cache_hit_rate = self.cache.get_cache_hit_rate().await;
        let circuit_breaker_open = self.resilience.get_circuit_breaker_status().await;
        let coordinator_count = self.coordinator.count_active_coordinators();

        Ok(ComprehensiveStats {
            system_stats,
            production_metrics,
            cache_hit_rate,
            circuit_breaker_open,
            coordinator_count,
        })
    }
}

/// Комплексная статистика всех сервисов
#[derive(Debug)]
pub struct ComprehensiveStats {
    pub system_stats: crate::service_di::MemorySystemStats,
    pub production_metrics: crate::services::traits::ProductionMetrics,
    pub cache_hit_rate: f64,
    pub circuit_breaker_open: bool,
    pub coordinator_count: usize,
}
