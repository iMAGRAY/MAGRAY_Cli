//! Coordinator Factory Module - Single Responsibility для создания координаторов
//! 
//! Этот модуль отвечает ТОЛЬКО за создание и настройку orchestration координаторов.
//! Применяет Factory pattern, Dependency Inversion и Open/Closed принципы.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    cache_interface::EmbeddingCacheInterface,
    di_container::DIContainer,
    health::HealthMonitor,
    storage::VectorStore,
    gpu_accelerated::GpuBatchProcessor,
    orchestration::{
        EmbeddingCoordinator as EmbeddingCoordinatorImpl,
        SearchCoordinator as SearchCoordinatorImpl,
        HealthManager,
        ResourceController,
    },
};

use crate::orchestration::Coordinator;

/// Trait для создания координаторов (Dependency Inversion)
pub trait CoordinatorFactory {
    async fn create_embedding_coordinator(&self, container: &DIContainer) -> Result<Arc<EmbeddingCoordinatorImpl>>;
    async fn create_search_coordinator(&self, container: &DIContainer, embedding_coordinator: &Arc<EmbeddingCoordinatorImpl>) -> Result<Arc<SearchCoordinatorImpl>>;
    async fn create_health_manager(&self, container: &DIContainer) -> Result<Arc<HealthManager>>;
    async fn create_resource_controller(&self, container: &DIContainer) -> Result<Arc<ResourceController>>;
}

/// Структура для хранения созданных координаторов
#[derive(Debug)]
pub struct OrchestrationCoordinators {
    pub embedding_coordinator: Option<Arc<EmbeddingCoordinatorImpl>>,
    pub search_coordinator: Option<Arc<SearchCoordinatorImpl>>,
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
            Some(self.create_embedding_coordinator(container).await?)
        } else {
            None
        };
        
        let search_coordinator = if self.create_search && embedding_coordinator.is_some() {
            Some(self.create_search_coordinator(container, embedding_coordinator.as_ref().unwrap()).await?)
        } else {
            None
        };
        
        let health_manager = if self.create_health {
            Some(self.create_health_manager(container).await?)
        } else {
            None
        };
        
        let resource_controller = if self.create_resources {
            Some(self.create_resource_controller(container).await?)
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
}

impl CoordinatorFactory for ProductionCoordinatorFactory {
    /// Создать embedding coordinator
    async fn create_embedding_coordinator(&self, container: &DIContainer) -> Result<Arc<EmbeddingCoordinatorImpl>> {
        let gpu_processor = container.resolve::<GpuBatchProcessor>()?;
        
        // Создаем временный cache для демонстрации
        let cache_path = std::env::temp_dir().join("embedding_cache");
        let cache = Arc::new(crate::cache_lru::EmbeddingCacheLRU::new(
            cache_path,
            crate::cache_lru::CacheConfig::default()
        )?) as Arc<dyn EmbeddingCacheInterface>;
        
        let coordinator = Arc::new(EmbeddingCoordinatorImpl::new(gpu_processor, cache));
        debug!("✅ EmbeddingCoordinator создан");
        
        Ok(coordinator)
    }

    /// Создать search coordinator  
    async fn create_search_coordinator(
        &self, 
        container: &DIContainer, 
        embedding_coordinator: &Arc<EmbeddingCoordinatorImpl>
    ) -> Result<Arc<SearchCoordinatorImpl>> {
        let store = container.resolve::<VectorStore>()?;
        
        let coordinator = Arc::new(SearchCoordinatorImpl::new_production(
            store,
            embedding_coordinator.clone(),
            64,  // max concurrent searches
            2000 // cache size
        ));
        debug!("✅ SearchCoordinator создан");
        
        Ok(coordinator)
    }

    /// Создать health manager
    async fn create_health_manager(&self, container: &DIContainer) -> Result<Arc<HealthManager>> {
        let health_monitor = container.resolve::<HealthMonitor>()?;
        
        let manager = Arc::new(HealthManager::new(health_monitor));
        debug!("✅ HealthManager создан");
        
        Ok(manager)
    }

    /// Создать resource controller
    async fn create_resource_controller(&self, container: &DIContainer) -> Result<Arc<ResourceController>> {
        let resource_manager = container.resolve::<parking_lot::RwLock<crate::resource_manager::ResourceManager>>()?;
        
        let controller = Arc::new(ResourceController::new_production(resource_manager));
        debug!("✅ ResourceController создан");
        
        Ok(controller)
    }
}

/// Test factory для создания mock координаторов (Open/Closed)
pub struct TestCoordinatorFactory;

impl CoordinatorFactory for TestCoordinatorFactory {
    async fn create_embedding_coordinator(&self, _container: &DIContainer) -> Result<Arc<EmbeddingCoordinatorImpl>> {
        // В тестах можем создавать mock координаторы
        Err(anyhow::anyhow!("Test coordinator factory - not implemented"))
    }

    async fn create_search_coordinator(&self, _container: &DIContainer, _embedding_coordinator: &Arc<EmbeddingCoordinatorImpl>) -> Result<Arc<SearchCoordinatorImpl>> {
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