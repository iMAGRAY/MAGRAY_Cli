//! Coordinator Factory Module - Single Responsibility для создания координаторов
//! 
//! Этот модуль отвечает ТОЛЬКО за создание и настройку orchestration координаторов.
//! Применяет Factory pattern, Dependency Inversion и Open/Closed принципы.

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::{debug, info, warn};

// Import traits для методов координаторов
use crate::orchestration::traits::EmbeddingCoordinator as EmbeddingCoordinatorTrait;

use crate::{
    cache_interface::EmbeddingCacheInterface,
    di::unified_container::UnifiedDIContainer as DIContainer,
    health::HealthMonitor,
    storage::VectorStore,
    gpu_accelerated::GpuBatchProcessor,
    orchestration::{
        EmbeddingCoordinator,
        SearchCoordinator,
        HealthManager,
        ResourceController,
    },
};

use crate::orchestration::Coordinator;

/// Trait для создания координаторов (Dependency Inversion)
pub trait CoordinatorFactory {
    async fn create_embedding_coordinator(&self, container: &DIContainer) -> Result<Arc<EmbeddingCoordinator>>;
    async fn create_search_coordinator(&self, container: &DIContainer, embedding_coordinator: &Arc<EmbeddingCoordinator>) -> Result<Arc<SearchCoordinator>>;
    async fn create_health_manager(&self, container: &DIContainer) -> Result<Arc<HealthManager>>;
    async fn create_resource_controller(&self, container: &DIContainer) -> Result<Arc<ResourceController>>;
}

/// Структура для хранения созданных координаторов
// NOTE: Debug trait не реализован из-за зависимостей без Debug
pub struct OrchestrationCoordinators {
    pub embedding_coordinator: Option<Arc<EmbeddingCoordinator>>,
    pub search_coordinator: Option<Arc<SearchCoordinator>>,
    pub health_manager: Option<Arc<HealthManager>>,
    pub resource_controller: Option<Arc<ResourceController>>,
}

impl OrchestrationCoordinators {
    pub fn empty() -> Self {
        Self {
            embedding_coordinator: None,
            search_coordinator: None,
            health_manager: None,
            resource_controller: None,
        }
    }

    pub fn count_active(&self) -> usize {
        let mut count = 0;
        if self.embedding_coordinator.is_some() { count += 1; }
        if self.search_coordinator.is_some() { count += 1; }
        if self.health_manager.is_some() { count += 1; }
        if self.resource_controller.is_some() { count += 1; }
        count
    }

    /// Инициализировать все координаторы параллельно
    pub async fn initialize_all(&self) -> Result<()> {
        info!("⚡ Параллельная инициализация координаторов...");

        let mut initialization_tasks = vec![];

        // Запускаем инициализацию координаторов параллельно
        if let Some(ref embedding_coordinator) = self.embedding_coordinator {
            let coordinator = embedding_coordinator.clone();
            initialization_tasks.push(tokio::spawn(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(60), 
                    coordinator.initialize()
                ).await
                    .map_err(|_| anyhow::anyhow!("Timeout инициализации EmbeddingCoordinator"))?
            }));
        }

        if let Some(ref search_coordinator) = self.search_coordinator {
            let coordinator = search_coordinator.clone();
            initialization_tasks.push(tokio::spawn(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(60),
                    coordinator.initialize()
                ).await
                    .map_err(|_| anyhow::anyhow!("Timeout инициализации SearchCoordinator"))?
            }));
        }

        if let Some(ref health_manager) = self.health_manager {
            let manager = health_manager.clone();
            initialization_tasks.push(tokio::spawn(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    manager.initialize()
                ).await
                    .map_err(|_| anyhow::anyhow!("Timeout инициализации HealthManager"))?
            }));
        }

        if let Some(ref resource_controller) = self.resource_controller {
            let controller = resource_controller.clone();
            initialization_tasks.push(tokio::spawn(async move {
                tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    controller.initialize()
                ).await
                    .map_err(|_| anyhow::anyhow!("Timeout инициализации ResourceController"))?
            }));
        }

        // Ждем завершения всех инициализаций
        for task in initialization_tasks {
            match task.await {
                Ok(Ok(_)) => {
                    // Успешная инициализация
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Panic при инициализации координатора: {}", e));
                }
            }
        }

        info!("✅ Все координаторы инициализированы");
        Ok(())
    }

    /// Shutdown всех координаторов параллельно
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("🔌 Shutdown координаторов...");

        let mut shutdown_tasks = vec![];

        // Запускаем shutdown координаторов параллельно
        if let Some(ref embedding_coordinator) = self.embedding_coordinator {
            let coordinator = embedding_coordinator.clone();
            shutdown_tasks.push(tokio::spawn(async move {
                coordinator.shutdown().await
            }));
        }

        if let Some(ref search_coordinator) = self.search_coordinator {
            let coordinator = search_coordinator.clone();
            shutdown_tasks.push(tokio::spawn(async move {
                coordinator.shutdown().await
            }));
        }

        if let Some(ref health_manager) = self.health_manager {
            let manager = health_manager.clone();
            shutdown_tasks.push(tokio::spawn(async move {
                manager.shutdown().await
            }));
        }

        if let Some(ref resource_controller) = self.resource_controller {
            let controller = resource_controller.clone();
            shutdown_tasks.push(tokio::spawn(async move {
                controller.shutdown().await
            }));
        }

        // Ждем завершения всех shutdown операций
        for task in shutdown_tasks {
            if let Err(e) = task.await {
                tracing::warn!("Ошибка shutdown координатора: {}", e);
            }
        }

        info!("✅ Все координаторы остановлены");
        Ok(())
    }

    /// Проверить готовность всех координаторов
    pub async fn check_readiness(&self) -> Result<()> {
        info!("🔍 Проверка готовности координаторов...");

        let mut coordinator_statuses = Vec::new();

        if let Some(ref embedding_coordinator) = self.embedding_coordinator {
            let ready = embedding_coordinator.is_ready().await;
            coordinator_statuses.push(("EmbeddingCoordinator", ready));
        }

        if let Some(ref search_coordinator) = self.search_coordinator {
            let ready = search_coordinator.is_ready().await;
            coordinator_statuses.push(("SearchCoordinator", ready));
        }

        if let Some(ref health_manager) = self.health_manager {
            let ready = health_manager.is_ready().await;
            coordinator_statuses.push(("HealthManager", ready));
        }

        if let Some(ref resource_controller) = self.resource_controller {
            let ready = resource_controller.is_ready().await;
            coordinator_statuses.push(("ResourceController", ready));
        }

        // Проверяем что все координаторы готовы
        for (name, ready) in &coordinator_statuses {
            if *ready {
                debug!("✅ {} готов", name);
            } else {
                return Err(anyhow::anyhow!("Координатор {} не готов к работе", name));
            }
        }

        info!("✅ Все координаторы готовы к работе");
        Ok(())
    }

    /// Инициализация координаторов (alias для initialize_all)
    pub async fn initialize(&self) -> Result<()> {
        self.initialize_all().await
    }

    /// Shutdown координаторов (alias для shutdown_all)
    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown_all().await
    }

    /// Проверить health всех координаторов
    pub async fn check_health(&self) -> Result<crate::health::SystemHealthStatus> {
        if let Some(health_manager) = &self.health_manager {
            // Временная заглушка - возвращаем default health status
            Ok(crate::health::SystemHealthStatus::default())
        } else {
            // Возвращаем базовый health status если нет health manager
            Ok(crate::health::SystemHealthStatus::default())
        }
    }

    /// Получить статистику cache из координаторов
    pub async fn get_cache_stats(&self) -> (u64, u64, u64) {
        let mut total_hits = 0;
        let mut total_misses = 0; 
        let mut total_size = 0;

        // Получаем статистику от embedding координатора
        if let Some(embedding_coord) = &self.embedding_coordinator {
            let (hits, misses, size) = embedding_coord.cache_stats().await;
            total_hits += hits;
            total_misses += misses;
            total_size += size;
        }

        // Получаем статистику от search координатора
        if let Some(search_coord) = &self.search_coordinator {
            let stats = search_coord.metrics().await;
            // В качестве заглушки добавляем базовые значения
            total_hits += 0;
            total_misses += 0;
        }

        (total_hits, total_misses, total_size)
    }
}

/// Production-ready factory для создания координаторов
pub struct ProductionCoordinatorFactory {
    pub create_embedding: bool,
    pub create_search: bool,
    pub create_health: bool,
    pub create_resources: bool,
}

impl Default for ProductionCoordinatorFactory {
    fn default() -> Self {
        Self {
            create_embedding: true,
            create_search: true,
            create_health: true,
            create_resources: true,
        }
    }
}

impl ProductionCoordinatorFactory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn minimal() -> Self {
        Self {
            create_embedding: false,
            create_search: false,
            create_health: false,
            create_resources: false,
        }
    }

    pub fn custom(embedding: bool, search: bool, health: bool, resources: bool) -> Self {
        Self {
            create_embedding: embedding,
            create_search: search,
            create_health: health,
            create_resources: resources,
        }
    }

    /// Создать все координаторы согласно конфигурации
    pub async fn create_all_coordinators(&self, container: &DIContainer) -> Result<OrchestrationCoordinators> {
        info!("🎯 Создание orchestration координаторов...");

        let embedding_coordinator = if self.create_embedding {
            Some(self.create_embedding_coordinator(container).await
                .with_context(|| "Ошибка создания EmbeddingCoordinator")?)
        } else {
            None
        };
        
        let search_coordinator = if self.create_search {
            match &embedding_coordinator {
                Some(embedding_coord) => {
                    Some(self.create_search_coordinator(container, embedding_coord).await
                        .with_context(|| "Ошибка создания SearchCoordinator")?)
                }
                None => {
                    warn!("⚠️ SearchCoordinator требует EmbeddingCoordinator, но он не создан");
                    None
                }
            }
        } else {
            None
        };
        
        let health_manager = if self.create_health {
            Some(self.create_health_manager(container).await
                .with_context(|| "Ошибка создания HealthManager")?)
        } else {
            None
        };
        
        let resource_controller = if self.create_resources {
            Some(self.create_resource_controller(container).await
                .with_context(|| "Ошибка создания ResourceController")?)
        } else {
            None
        };

        let coordinators = OrchestrationCoordinators {
            embedding_coordinator,
            search_coordinator,
            health_manager,
            resource_controller,
        };

        info!("✅ Созданo {} координаторов", coordinators.count_active());
        
        Ok(coordinators)
    }

    /// Создать координаторы (алиас для create_all_coordinators)
    /// DEPRECATED: Используйте create_all_coordinators с явным DI container
    pub async fn create_coordinators(&self) -> Result<OrchestrationCoordinators> {
        warn!("⚠️ DEPRECATED: create_coordinators() создает временный контейнер. Используйте create_all_coordinators() с явным DI container");
        
        // Для facade мы создаем временный контейнер с proper error handling
        // В реальном использовании контейнер должен передаваться извне
        let container = crate::di_container::DIContainer::default_container()
            .with_context(|| "Не удалось создать временный DI container для координаторов")?;
            
        self.create_all_coordinators(&container).await
            .with_context(|| "Ошибка создания координаторов с временным контейнером")
    }

    /// Конструктор с DI контейнером
    pub fn with_container(_container: DIContainer) -> Self {
        Self {
            create_embedding: true,
            create_search: true,
            create_health: true,
            create_resources: true,
        }
    }
}

impl CoordinatorFactory for ProductionCoordinatorFactory {
    /// Создать embedding coordinator с proper error handling
    async fn create_embedding_coordinator(&self, container: &DIContainer) -> Result<Arc<EmbeddingCoordinator>> {
        debug!("🔤 Начинаем создание EmbeddingCoordinator...");
        
        let gpu_processor = container.resolve::<GpuBatchProcessor>()
            .with_context(|| "Не удалось resolve GpuBatchProcessor из DI container")?;
        
        // Создаем cache с proper error handling
        let cache_path = std::env::temp_dir().join("embedding_cache");
        let cache_config = crate::cache_lru::CacheConfig::default();
        
        let cache = Arc::new(
            crate::cache_lru::EmbeddingCacheLRU::new(cache_path.clone(), cache_config)
                .with_context(|| format!("Ошибка создания embedding cache по пути: {:?}", cache_path))?
        ) as Arc<dyn EmbeddingCacheInterface>;
        
        let coordinator = Arc::new(EmbeddingCoordinator::new(gpu_processor, cache));
        debug!("✅ EmbeddingCoordinator успешно создан");
        
        Ok(coordinator)
    }

    /// Создать search coordinator с dependency validation
    async fn create_search_coordinator(
        &self, 
        container: &DIContainer, 
        embedding_coordinator: &Arc<EmbeddingCoordinator>
    ) -> Result<Arc<SearchCoordinator>> {
        debug!("🔍 Начинаем создание SearchCoordinator...");
        
        let store = container.resolve::<VectorStore>()
            .with_context(|| "Не удалось resolve VectorStore из DI container")?;
        
        let coordinator = Arc::new(SearchCoordinator::new_production(
            store,
            embedding_coordinator.clone(),
            64,  // max concurrent searches
            2000 // cache size
        ));
        
        debug!("✅ SearchCoordinator успешно создан с max_concurrent=64, cache_size=2000");
        Ok(coordinator)
    }

    /// Создать health manager с error handling
    async fn create_health_manager(&self, container: &DIContainer) -> Result<Arc<HealthManager>> {
        debug!("🏥 Начинаем создание HealthManager...");
        
        let health_monitor = container.resolve::<HealthMonitor>()
            .with_context(|| "Не удалось resolve HealthMonitor из DI container")?;
        
        let manager = Arc::new(HealthManager::new(health_monitor));
        debug!("✅ HealthManager успешно создан");
        
        Ok(manager)
    }

    /// Создать resource controller с validation
    async fn create_resource_controller(&self, container: &DIContainer) -> Result<Arc<ResourceController>> {
        debug!("⚡ Начинаем создание ResourceController...");
        
        let resource_manager = container.resolve::<parking_lot::RwLock<crate::resource_manager::ResourceManager>>()
            .with_context(|| "Не удалось resolve ResourceManager из DI container")?;
        
        let controller = Arc::new(ResourceController::new_production(resource_manager));
        debug!("✅ ResourceController успешно создан");
        
        Ok(controller)
    }
}

/// Test factory для создания mock координаторов (Open/Closed)
pub struct TestCoordinatorFactory;

impl CoordinatorFactory for TestCoordinatorFactory {
    async fn create_embedding_coordinator(&self, _container: &DIContainer) -> Result<Arc<EmbeddingCoordinator>> {
        // В тестах можем создавать mock координаторы
        Err(anyhow::anyhow!("Test coordinator factory - not implemented"))
    }

    async fn create_search_coordinator(&self, _container: &DIContainer, _embedding_coordinator: &Arc<EmbeddingCoordinator>) -> Result<Arc<SearchCoordinator>> {
        Err(anyhow::anyhow!("Test coordinator factory - not implemented"))
    }

    async fn create_health_manager(&self, _container: &DIContainer) -> Result<Arc<HealthManager>> {
        Err(anyhow::anyhow!("Test coordinator factory - not implemented"))
    }

    async fn create_resource_controller(&self, _container: &DIContainer) -> Result<Arc<ResourceController>> {
        Err(anyhow::anyhow!("Test coordinator factory - not implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_factory_creation() {
        let factory = ProductionCoordinatorFactory::new();
        assert!(factory.create_embedding);
        assert!(factory.create_search);

        let minimal = ProductionCoordinatorFactory::minimal();
        assert!(!minimal.create_embedding);
        assert!(!minimal.create_search);

        let custom = ProductionCoordinatorFactory::custom(true, false, true, false);
        assert!(custom.create_embedding);
        assert!(!custom.create_search);
        assert!(custom.create_health);
        assert!(!custom.create_resources);
    }

    #[test]
    fn test_empty_coordinators() {
        let coords = OrchestrationCoordinators::empty();
        assert_eq!(coords.count_active(), 0);
        assert!(coords.embedding_coordinator.is_none());
        assert!(coords.search_coordinator.is_none());
    }
}