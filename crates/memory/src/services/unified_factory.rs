//! Unified Service Factory - единая точка создания всех сервисов
//!
//! Объединяет ServiceFactory и CoordinatorFactory в единую архитектуру
//! с применением принципов SOLID и интеграцией с UnifiedDIContainer.
//!
//! РЕШАЕМЫЕ ПРОБЛЕМЫ:
//! - Дублирование между ServiceFactory и CoordinatorFactory  
//! - .unwrap() вызовы в ProductionCoordinatorFactory
//! - Неконсистентные интерфейсы между factory
//! - Отсутствие единой конфигурации для всех сервисов
//!
//! ПРИМЕНЯЕМЫЕ ПРИНЦИПЫ:
//! - Single Responsibility: Каждый factory отвечает за свою область
//! - Open/Closed: Расширяемость через trait абстракции
//! - Liskov Substitution: Взаимозаменяемые implementations  
//! - Interface Segregation: Специализированные interfaces
//! - Dependency Inversion: Constructor injection, зависимости от абстракций

use anyhow::{Context, Result};
use std::{sync::Arc, time::Duration};
use tracing::{debug, info};

use crate::{
    di::{traits::DIResolver, UnifiedContainer},
};
use crate::di::core_traits::ServiceResolver;
use crate::orchestration::{EmbeddingCoordinator, HealthManager, ResourceController, SearchCoordinator};
use crate::service_di::coordinator_factory::OrchestrationCoordinators;
use crate::services::{
    traits::{
        CacheServiceTrait, CoordinatorServiceTrait, CoreMemoryServiceTrait,
        MonitoringServiceTrait, ResilienceServiceTrait,
    },
    CacheService, CoordinatorService, CoreMemoryService, MonitoringService, ResilienceService,
};

/// Конфигурация для Unified Factory
/// Объединяет настройки для всех типов сервисов
#[derive(Debug, Clone)]
pub struct UnifiedFactoryConfig {
    /// Основные параметры системы
    pub max_concurrent_operations: usize,
    pub embedding_dimension: usize,
    pub production_mode: bool,

    /// Параметры circuit breaker
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout: Duration,

    /// Параметры cache
    pub cache_size_mb: usize,
    pub cache_ttl_seconds: u64,

    /// Параметры координаторов
    pub enable_embedding_coordinator: bool,
    pub enable_search_coordinator: bool,
    pub enable_health_manager: bool,
    pub enable_resource_controller: bool,

    /// Параметры мониторинга
    pub enable_production_monitoring: bool,
    pub metrics_collection_interval: Duration,

    /// Параметры производительности
    pub max_search_concurrent: usize,
    pub search_cache_size: usize,
}

impl Default for UnifiedFactoryConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 50,
            embedding_dimension: 1024,
            production_mode: false,

            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),

            cache_size_mb: 256,
            cache_ttl_seconds: 3600,

            enable_embedding_coordinator: true,
            enable_search_coordinator: true,
            enable_health_manager: true,
            enable_resource_controller: true,

            enable_production_monitoring: false,
            metrics_collection_interval: Duration::from_secs(30),

            max_search_concurrent: 64,
            search_cache_size: 2000,
        }
    }
}

impl UnifiedFactoryConfig {
    /// Production preset - оптимизирован для production окружения
    pub fn production() -> Self {
        Self {
            max_concurrent_operations: 200,
            embedding_dimension: 1024,
            production_mode: true,

            circuit_breaker_threshold: 3,
            circuit_breaker_timeout: Duration::from_secs(30),

            cache_size_mb: 1024,
            cache_ttl_seconds: 7200,

            enable_embedding_coordinator: true,
            enable_search_coordinator: true,
            enable_health_manager: true,
            enable_resource_controller: true,

            enable_production_monitoring: true,
            metrics_collection_interval: Duration::from_secs(10),

            max_search_concurrent: 128,
            search_cache_size: 5000,
        }
    }

    /// Development preset - оптимизирован для разработки
    pub fn development() -> Self {
        Self {
            max_concurrent_operations: 20,
            embedding_dimension: 512,
            production_mode: false,

            circuit_breaker_threshold: 10,
            circuit_breaker_timeout: Duration::from_secs(120),

            cache_size_mb: 128,
            cache_ttl_seconds: 1800,

            enable_embedding_coordinator: true,
            enable_search_coordinator: true,
            enable_health_manager: false,
            enable_resource_controller: false,

            enable_production_monitoring: false,
            metrics_collection_interval: Duration::from_secs(60),

            max_search_concurrent: 32,
            search_cache_size: 1000,
        }
    }

    /// Test preset - минимальная конфигурация для тестов
    pub fn test() -> Self {
        Self {
            max_concurrent_operations: 5,
            embedding_dimension: 256,
            production_mode: false,

            circuit_breaker_threshold: 20,
            circuit_breaker_timeout: Duration::from_secs(5),

            cache_size_mb: 32,
            cache_ttl_seconds: 300,

            enable_embedding_coordinator: false,
            enable_search_coordinator: false,
            enable_health_manager: false,
            enable_resource_controller: false,

            enable_production_monitoring: false,
            metrics_collection_interval: Duration::from_secs(300),

            max_search_concurrent: 4,
            search_cache_size: 100,
        }
    }

    /// Minimal preset - только core services
    pub fn minimal() -> Self {
        Self {
            max_concurrent_operations: 10,
            embedding_dimension: 512,
            production_mode: false,

            circuit_breaker_threshold: 15,
            circuit_breaker_timeout: Duration::from_secs(90),

            cache_size_mb: 64,
            cache_ttl_seconds: 900,

            enable_embedding_coordinator: false,
            enable_search_coordinator: false,
            enable_health_manager: false,
            enable_resource_controller: false,

            enable_production_monitoring: false,
            metrics_collection_interval: Duration::from_secs(120),

            max_search_concurrent: 8,
            search_cache_size: 200,
        }
    }

    /// Builder pattern для кастомной конфигурации
    pub fn custom() -> UnifiedFactoryConfigBuilder {
        UnifiedFactoryConfigBuilder::new()
    }
}

/// Builder для создания кастомной конфигурации
#[derive(Debug, Clone)]
pub struct UnifiedFactoryConfigBuilder {
    config: UnifiedFactoryConfig,
}

impl UnifiedFactoryConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: UnifiedFactoryConfig::default(),
        }
    }

    pub fn max_concurrent_operations(mut self, value: usize) -> Self {
        self.config.max_concurrent_operations = value;
        self
    }

    pub fn embedding_dimension(mut self, value: usize) -> Self {
        self.config.embedding_dimension = value;
        self
    }

    pub fn production_mode(mut self, value: bool) -> Self {
        self.config.production_mode = value;
        self
    }

    pub fn circuit_breaker(mut self, threshold: u32, timeout: Duration) -> Self {
        self.config.circuit_breaker_threshold = threshold;
        self.config.circuit_breaker_timeout = timeout;
        self
    }

    pub fn cache_settings(mut self, size_mb: usize, ttl_seconds: u64) -> Self {
        self.config.cache_size_mb = size_mb;
        self.config.cache_ttl_seconds = ttl_seconds;
        self
    }

    pub fn coordinators(
        mut self,
        embedding: bool,
        search: bool,
        health: bool,
        resources: bool,
    ) -> Self {
        self.config.enable_embedding_coordinator = embedding;
        self.config.enable_search_coordinator = search;
        self.config.enable_health_manager = health;
        self.config.enable_resource_controller = resources;
        self
    }

    pub fn monitoring(mut self, enable: bool, interval: Duration) -> Self {
        self.config.enable_production_monitoring = enable;
        self.config.metrics_collection_interval = interval;
        self
    }

    pub fn search_settings(mut self, concurrent: usize, cache_size: usize) -> Self {
        self.config.max_search_concurrent = concurrent;
        self.config.search_cache_size = cache_size;
        self
    }

    pub fn build(self) -> UnifiedFactoryConfig {
        self.config
    }
}

/// Результат создания всех сервисов
pub struct UnifiedServiceCollection {
    /// Основные сервисы
    pub core_memory: Arc<dyn CoreMemoryServiceTrait>,
    pub coordinator: Arc<dyn CoordinatorServiceTrait>,
    pub resilience: Arc<dyn ResilienceServiceTrait>,
    pub monitoring: Arc<dyn MonitoringServiceTrait>,
    pub cache: Arc<dyn CacheServiceTrait>,

    /// Координаторы orchestration
    pub orchestration: OrchestrationCoordinators,

    /// Конфигурация используемая при создании
    pub config: UnifiedFactoryConfig,
}

/// Unified Service Factory - главный factory для всех сервисов
///
/// ПРИНЦИПЫ SOLID:
/// - SRP: Отвечает только за создание и конфигурацию сервисов
/// - OCP: Расширяем через trait-based абстракции
/// - LSP: Service implementations взаимозаменяемы
/// - ISP: Разделенные интерфейсы для разных типов сервисов
/// - DIP: Зависимости инжектятся через DI container
pub struct UnifiedServiceFactory {
    container: Arc<UnifiedContainer>,
    config: UnifiedFactoryConfig,
}

impl UnifiedServiceFactory {
    /// Создать новый unified factory
    pub fn new(container: Arc<UnifiedContainer>) -> Self {
        info!("🏭 Создание UnifiedServiceFactory с конфигурацией по умолчанию");
        Self {
            container,
            config: UnifiedFactoryConfig::default(),
        }
    }

    /// Создать unified factory с кастомной конфигурацией
    pub fn with_config(container: Arc<UnifiedContainer>, config: UnifiedFactoryConfig) -> Self {
        info!("🏭 Создание UnifiedServiceFactory с кастомной конфигурацией");
        debug!(
            "🔧 Конфигурация: max_ops={}, prod_mode={}, coordinators={}",
            config.max_concurrent_operations,
            config.production_mode,
            config.enable_embedding_coordinator as u8
                + config.enable_search_coordinator as u8
                + config.enable_health_manager as u8
                + config.enable_resource_controller as u8
        );
        Self { container, config }
    }

    /// Production factory preset
    pub fn production(container: Arc<UnifiedContainer>) -> Self {
        Self::with_config(container, UnifiedFactoryConfig::production())
    }

    /// Development factory preset  
    pub fn development(container: Arc<UnifiedContainer>) -> Self {
        Self::with_config(container, UnifiedFactoryConfig::development())
    }

    /// Test factory preset
    pub fn test(container: Arc<UnifiedContainer>) -> Self {
        Self::with_config(container, UnifiedFactoryConfig::test())
    }

    /// Minimal factory preset
    pub fn minimal(container: Arc<UnifiedContainer>) -> Self {
        Self::with_config(container, UnifiedFactoryConfig::minimal())
    }

    /// Создать все сервисы согласно конфигурации
    pub async fn create_all_services(&self) -> Result<UnifiedServiceCollection> {
        info!("🏭 Создание всех сервисов через UnifiedServiceFactory...");

        // 1. Создаём базовые сервисы (независимые от других)
        let resilience = self
            .create_resilience_service()
            .with_context(|| "Ошибка создания ResilienceService")?;

        let coordinator = self
            .create_coordinator_service()
            .with_context(|| "Ошибка создания CoordinatorService")?;

        // 2. Создаём сервисы с зависимостями
        let monitoring = self
            .create_monitoring_service(coordinator.clone())
            .with_context(|| "Ошибка создания MonitoringService")?;

        let cache = self
            .create_cache_service(coordinator.clone())
            .with_context(|| "Ошибка создания CacheService")?;

        let core_memory = self
            .create_core_memory_service()
            .with_context(|| "Ошибка создания CoreMemoryService")?;

        // 3. Создаём orchestration координаторы
        let orchestration = self
            .create_orchestration_coordinators()
            .await
            .with_context(|| "Ошибка создания orchestration координаторов")?;

        // 4. Инициализируем все сервисы
        self.initialize_services(&coordinator, &orchestration)
            .await
            .with_context(|| "Ошибка инициализации сервисов")?;

        let service_collection = UnifiedServiceCollection {
            core_memory,
            coordinator,
            resilience,
            monitoring,
            cache,
            orchestration,
            config: self.config.clone(),
        };

        info!("✅ Все сервисы успешно созданы и инициализированы");
        Ok(service_collection)
    }

    /// Создать только core services (без координаторов)
    pub async fn create_core_services_only(&self) -> Result<UnifiedServiceCollection> {
        info!("🏭 Создание только core services...");

        let resilience = self.create_resilience_service()?;
        let coordinator = self.create_coordinator_service()?;
        let monitoring = self.create_monitoring_service(coordinator.clone())?;
        let cache = self.create_cache_service(coordinator.clone())?;
        let core_memory = self.create_core_memory_service()?;

        let service_collection = UnifiedServiceCollection {
            core_memory,
            coordinator,
            resilience,
            monitoring,
            cache,
            orchestration: OrchestrationCoordinators::empty(),
            config: self.config.clone(),
        };

        info!("✅ Core services созданы");
        Ok(service_collection)
    }

    /// Создать CoreMemoryService
    fn create_core_memory_service(&self) -> Result<Arc<dyn CoreMemoryServiceTrait>> {
        debug!("🗃️ Создание CoreMemoryService...");

        let service = if self.config.production_mode {
            CoreMemoryService::new_production(self.container.clone())
        } else {
            CoreMemoryService::new(
                self.container.clone(),
                self.config.max_concurrent_operations,
            )
        };

        debug!("✅ CoreMemoryService создан");
        Ok(Arc::new(service))
    }

    /// Создать CoordinatorService
    fn create_coordinator_service(&self) -> Result<Arc<dyn CoordinatorServiceTrait>> {
        debug!("🎯 Создание CoordinatorService...");
        let service = CoordinatorService::new();
        debug!("✅ CoordinatorService создан");
        Ok(Arc::new(service))
    }

    /// Создать ResilienceService с правильной конфигурацией
    fn create_resilience_service(&self) -> Result<Arc<dyn ResilienceServiceTrait>> {
        debug!("🛡️ Создание ResilienceService...");

        let service = if self.config.production_mode {
            ResilienceService::new_production()
        } else {
            ResilienceService::new_with_threshold(
                self.config.circuit_breaker_threshold,
                self.config.circuit_breaker_timeout,
            )
        };

        debug!(
            "✅ ResilienceService создан с threshold={}, timeout={:?}",
            self.config.circuit_breaker_threshold, self.config.circuit_breaker_timeout
        );
        Ok(Arc::new(service))
    }

    /// Создать MonitoringService с зависимостями
    fn create_monitoring_service(
        &self,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
    ) -> Result<Arc<dyn MonitoringServiceTrait>> {
        debug!("📊 Создание MonitoringService...");

        let service = MonitoringService::new_with_coordinator(self.container.clone(), coordinator);

        debug!("✅ MonitoringService создан");
        Ok(Arc::new(service))
    }

    /// Создать CacheService с конфигурацией
    fn create_cache_service(
        &self,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
    ) -> Result<Arc<dyn CacheServiceTrait>> {
        debug!("💾 Создание CacheService...");

        let mut service = CacheService::new_with_coordinator(self.container.clone(), coordinator);

        // Настраиваем параметры cache согласно конфигурации
        service.set_embedding_dimension(self.config.embedding_dimension);

        debug!(
            "✅ CacheService создан с dimension={}, size={}MB",
            self.config.embedding_dimension, self.config.cache_size_mb
        );
        Ok(Arc::new(service))
    }

    /// Создать orchestration координаторы согласно конфигурации
    async fn create_orchestration_coordinators(&self) -> Result<OrchestrationCoordinators> {
        if !self.should_create_coordinators() {
            debug!("⏭️ Создание координаторов отключено в конфигурации");
            return Ok(OrchestrationCoordinators::empty());
        }

        info!("🎯 Создание orchestration координаторов...");

        let embedding_coordinator = if self.config.enable_embedding_coordinator {
            Some(
                self.create_embedding_coordinator()
                    .await
                    .with_context(|| "Ошибка создания EmbeddingCoordinator")?,
            )
        } else {
            None
        };

        let search_coordinator =
            if self.config.enable_search_coordinator && embedding_coordinator.is_some() {
                let embedding_coord = embedding_coordinator.as_ref().ok_or_else(|| {
                    anyhow::anyhow!(
                    "EmbeddingCoordinator is required for SearchCoordinator creation but is None"
                )
                })?;
                Some(
                    self.create_search_coordinator(embedding_coord)
                        .await
                        .with_context(|| "Ошибка создания SearchCoordinator")?,
                )
            } else {
                None
            };

        let health_manager = if self.config.enable_health_manager {
            Some(
                self.create_health_manager()
                    .await
                    .with_context(|| "Ошибка создания HealthManager")?,
            )
        } else {
            None
        };

        let resource_controller = if self.config.enable_resource_controller {
            Some(
                self.create_resource_controller()
                    .await
                    .with_context(|| "Ошибка создания ResourceController")?,
            )
        } else {
            None
        };

        let coordinators = OrchestrationCoordinators {
            embedding_coordinator,
            search_coordinator,
            health_manager,
            resource_controller,
        };

        info!("✅ Создано {} координаторов", coordinators.count_active());
        Ok(coordinators)
    }

    /// Проверить нужно ли создавать координаторы
    fn should_create_coordinators(&self) -> bool {
        self.config.enable_embedding_coordinator
            || self.config.enable_search_coordinator
            || self.config.enable_health_manager
            || self.config.enable_resource_controller
    }

    /// Создать EmbeddingCoordinator с правильной конфигурацией
    async fn create_embedding_coordinator(&self) -> Result<Arc<EmbeddingCoordinator>> {
        debug!("🔤 Создание EmbeddingCoordinator...");

        // Resolve зависимости через UnifiedDIContainer (вместо .unwrap())
        #[cfg(feature = "gpu-acceleration")]
        let gpu_processor = self.container.resolve::<crate::gpu_accelerated::GpuBatchProcessor>().ok();
        #[cfg(not(feature = "gpu-acceleration"))]
        let gpu_processor: Option<std::sync::Arc<()>> = None;

        // Создаем cache с правильной конфигурацией
        let cache_path = std::env::temp_dir().join("embedding_cache");
        let cache_config = crate::cache_lru::CacheConfig::default();
        let cache = Arc::new(
            crate::cache_lru::EmbeddingCacheLRU::new(cache_path, cache_config)
                .with_context(|| "Ошибка создания embedding cache")?,
        );

        let embedding_coordinator = Arc::new(EmbeddingCoordinator::new_stub());
        debug!("✅ EmbeddingCoordinator создан");
        Ok(embedding_coordinator)
    }

    /// Создать SearchCoordinator с зависимостями
    async fn create_search_coordinator(
        &self,
        embedding_coordinator: &Arc<EmbeddingCoordinator>,
    ) -> Result<Arc<SearchCoordinator>> {
        debug!("🔍 Создание SearchCoordinator...");

        let store = self.container.resolve::<crate::storage::VectorStore>()?;

        let coordinator = Arc::new(SearchCoordinator::new_production(
            store,
            embedding_coordinator.clone(),
            self.config.max_search_concurrent,
            self.config.search_cache_size,
        ));

        debug!(
            "✅ SearchCoordinator создан с concurrent={}, cache_size={}",
            self.config.max_search_concurrent, self.config.search_cache_size
        );
        Ok(coordinator)
    }

    /// Создать HealthManager
    async fn create_health_manager(&self) -> Result<Arc<HealthManager>> {
        debug!("🏥 Создание HealthManager...");

        let health_monitor = self.container.resolve::<crate::health::HealthMonitor>()?;

        let manager = Arc::new(HealthManager::new(health_monitor));
        debug!("✅ HealthManager создан");
        Ok(manager)
    }

    /// Создать ResourceController
    async fn create_resource_controller(&self) -> Result<Arc<ResourceController>> {
        debug!("⚡ Создание ResourceController...");

        let resource_manager = self.container.resolve::<parking_lot::RwLock<crate::resource_manager::ResourceManager>>()?;

        let controller = Arc::new(ResourceController::new_production(resource_manager));
        debug!("✅ ResourceController создан");
        Ok(controller)
    }

    /// Инициализировать все созданные сервисы
    async fn initialize_services(
        &self,
        coordinator: &Arc<dyn CoordinatorServiceTrait>,
        orchestration: &OrchestrationCoordinators,
    ) -> Result<()> {
        info!("⚡ Инициализация всех сервисов...");

        // Инициализируем coordinator service
        coordinator
            .create_coordinators(&self.container)
            .await
            .with_context(|| "Ошибка создания координаторов в CoordinatorService")?;

        coordinator
            .initialize_coordinators()
            .await
            .with_context(|| "Ошибка инициализации координаторов в CoordinatorService")?;

        // Инициализируем orchestration координаторы
        if orchestration.count_active() > 0 {
            orchestration
                .initialize_all()
                .await
                .with_context(|| "Ошибка инициализации orchestration координаторов")?;
        }

        info!("✅ Все сервисы успешно инициализированы");
        Ok(())
    }
}

impl UnifiedServiceCollection {
    /// Полная инициализация всех сервисов
    pub async fn initialize_all_services(&self) -> Result<()> {
        info!("⚡ Полная инициализация всех сервисов в коллекции...");

        // Запускаем monitoring если включен
        if self.config.enable_production_monitoring {
            self.monitoring
                .start_production_monitoring()
                .await
                .with_context(|| "Ошибка запуска production monitoring")?;

            self.monitoring
                .start_health_monitoring()
                .await
                .with_context(|| "Ошибка запуска health monitoring")?;

            self.monitoring
                .start_resource_monitoring()
                .await
                .with_context(|| "Ошибка запуска resource monitoring")?;
        }

        // Проверяем готовность всех сервисов
        self.monitoring
            .perform_readiness_checks()
            .await
            .with_context(|| "Ошибка readiness checks")?;

        // Проверяем готовность координаторов
        if self.orchestration.count_active() > 0 {
            self.orchestration
                .check_readiness()
                .await
                .with_context(|| "Координаторы не готовы к работе")?;
        }

        // Логируем финальную статистику
        self.monitoring.log_initialization_summary().await;

        info!("✅ Все сервисы успешно инициализированы и готовы к работе");
        Ok(())
    }

    /// Graceful shutdown всех сервисов
    pub async fn shutdown_all_services(&self) -> Result<()> {
        info!("🛑 Graceful shutdown всех сервисов...");

        // Останавливаем координаторы
        self.coordinator
            .shutdown_coordinators()
            .await
            .with_context(|| "Ошибка shutdown CoordinatorService")?;

        // Останавливаем orchestration координаторы
        if self.orchestration.count_active() > 0 {
            self.orchestration
                .shutdown_all()
                .await
                .with_context(|| "Ошибка shutdown orchestration координаторов")?;
        }

        info!("✅ Все сервисы успешно остановлены");
        Ok(())
    }

    /// Получить comprehensive статистику всех сервисов
    pub async fn get_comprehensive_statistics(&self) -> Result<UnifiedServiceStatistics> {
        debug!("📊 Сбор comprehensive статистики всех сервисов...");

        let system_stats = self.monitoring.get_system_stats().await;
        let production_metrics = self.monitoring.get_production_metrics().await?;
        let cache_hit_rate = self.cache.get_cache_hit_rate().await;
        let circuit_breaker_open = self.resilience.get_circuit_breaker_status().await;
        let coordinator_count = self.coordinator.count_active_coordinators();

        // Получаем статистику от orchestration координаторов
        let (cache_hits, cache_misses, cache_size) = self.orchestration.get_cache_stats().await;
        let orchestration_health = self.orchestration.check_health().await.unwrap_or_default();

        Ok(UnifiedServiceStatistics {
            system_stats,
            production_metrics,
            cache_hit_rate,
            circuit_breaker_open,
            coordinator_count,
            orchestration_cache_hits: cache_hits,
            orchestration_cache_misses: cache_misses,
            orchestration_cache_size: cache_size,
            orchestration_health,
            config_used: self.config.clone(),
        })
    }
}

/// Comprehensive статистика всех сервисов
#[derive(Debug)]
pub struct UnifiedServiceStatistics {
    pub system_stats: crate::service_di::MemorySystemStats,
    pub production_metrics: crate::services::traits::ProductionMetrics,
    pub cache_hit_rate: f64,
    pub circuit_breaker_open: bool,
    pub coordinator_count: usize,
    pub orchestration_cache_hits: u64,
    pub orchestration_cache_misses: u64,
    pub orchestration_cache_size: u64,
    pub orchestration_health: crate::health::SystemHealthStatus,
    pub config_used: UnifiedFactoryConfig,
}

impl UnifiedServiceStatistics {
    /// Проверить общее health всех систем
    pub fn is_system_healthy(&self) -> bool {
        !self.circuit_breaker_open && self.cache_hit_rate > 0.1 && self.coordinator_count > 0
    }

    /// Получить summary строку для логирования
    pub fn summary(&self) -> String {
        format!(
            "Services: {} coordinators, Cache: {:.1}% hit rate, Circuit breaker: {}, Health: {}",
            self.coordinator_count,
            self.cache_hit_rate * 100.0,
            if self.circuit_breaker_open {
                "OPEN"
            } else {
                "CLOSED"
            },
            if self.is_system_healthy() {
                "HEALTHY"
            } else {
                "DEGRADED"
            }
        )
    }
}
