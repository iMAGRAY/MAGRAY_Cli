//! CoordinatorService - управление координаторами и их инициализация  
//!
//! Single Responsibility: только координация между компонентами
//! - создание и инициализация координаторов
//! - управление жизненным циклом координаторов
//! - предоставление доступа к координаторам через trait интерфейс

use std::sync::Arc;
use anyhow::Result;
use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::{
    di_container::DIContainer,
    orchestration::{
        EmbeddingCoordinator as EmbeddingCoordinatorImpl,
        SearchCoordinator as SearchCoordinatorImpl,
        HealthManager,
        ResourceController,
    },
    services::traits::CoordinatorServiceTrait,
    storage::VectorStore,
    gpu_accelerated::GpuBatchProcessor,
    health::HealthMonitor,
    cache_interface::EmbeddingCacheInterface,
};

/// Структура для хранения всех координаторов
#[allow(dead_code)]
struct CoordinatorRefs {
    embedding_coordinator: Option<Arc<EmbeddingCoordinatorImpl>>,
    search_coordinator: Option<Arc<SearchCoordinatorImpl>>,
    health_manager: Option<Arc<HealthManager>>,
    resource_controller: Option<Arc<ResourceController>>,
}

impl Default for CoordinatorRefs {
    fn default() -> Self {
        Self {
            embedding_coordinator: None,
            search_coordinator: None,
            health_manager: None,
            resource_controller: None,
        }
    }
}

/// Реализация управления координаторами
/// Отвечает ТОЛЬКО за создание, инициализацию и управление координаторами
#[allow(dead_code)]
pub struct CoordinatorService {
    /// Ссылки на все координаторы
    coordinators: Arc<tokio::sync::RwLock<CoordinatorRefs>>,
}

impl CoordinatorService {
    /// Создать новый CoordinatorService
    pub fn new() -> Self {
        info!("🎯 Создание CoordinatorService для управления координаторами");
        
        Self {
            coordinators: Arc::new(tokio::sync::RwLock::new(CoordinatorRefs::default())),
        }
    }

    /// Создать embedding coordinator
    #[allow(dead_code)]
    async fn create_embedding_coordinator(&self, container: &DIContainer) -> Result<Arc<EmbeddingCoordinatorImpl>> {
        debug!("🎯 Создание EmbeddingCoordinator...");
        
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
    #[allow(dead_code)]
    async fn create_search_coordinator(
        &self, 
        container: &DIContainer, 
        embedding_coordinator: &Arc<EmbeddingCoordinatorImpl>
    ) -> Result<Arc<SearchCoordinatorImpl>> {
        debug!("🎯 Создание SearchCoordinator...");
        
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
    #[allow(dead_code)]
    async fn create_health_manager(&self, container: &DIContainer) -> Result<Arc<HealthManager>> {
        debug!("🎯 Создание HealthManager...");
        
        let health_monitor = container.resolve::<HealthMonitor>()?;
        
        let manager = Arc::new(HealthManager::new(health_monitor));
        debug!("✅ HealthManager создан");
        
        Ok(manager)
    }

    /// Создать resource controller
    #[allow(dead_code)]
    async fn create_resource_controller(&self, container: &DIContainer) -> Result<Arc<ResourceController>> {
        debug!("🎯 Создание ResourceController...");
        
        let resource_manager = container.resolve::<parking_lot::RwLock<crate::resource_manager::ResourceManager>>()?;
        
        let controller = Arc::new(ResourceController::new_production(resource_manager));
        debug!("✅ ResourceController создан");
        
        Ok(controller)
    }
}

#[async_trait]
impl CoordinatorServiceTrait for CoordinatorService {
    /// Создать все координаторы
    #[allow(dead_code)]
    async fn create_coordinators(&self, container: &DIContainer) -> Result<()> {
        info!("🎯 Создание всех координаторов...");

        let mut coordinators = self.coordinators.write().await;

        // Создаём embedding coordinator
        let embedding_coordinator = self.create_embedding_coordinator(container).await?;
        coordinators.embedding_coordinator = Some(embedding_coordinator.clone());
        
        // Создаём search coordinator (зависит от embedding coordinator)
        let search_coordinator = self.create_search_coordinator(container, &embedding_coordinator).await?;
        coordinators.search_coordinator = Some(search_coordinator);
        
        // Создаём health manager
        let health_manager = self.create_health_manager(container).await?;
        coordinators.health_manager = Some(health_manager);
        
        // Создаём resource controller
        let resource_controller = self.create_resource_controller(container).await?;
        coordinators.resource_controller = Some(resource_controller);

        info!("✅ Все координаторы созданы");
        Ok(())
    }

    /// Инициализировать все координаторы
    #[allow(dead_code)]
    async fn initialize_coordinators(&self) -> Result<()> {
        info!("⚡ Инициализация координаторов...");

        let coordinators = self.coordinators.read().await;
        let mut initialization_tasks = vec![];

        // NOTE: В текущей реализации координаторы не имеют initialize метода
        // Используем заглушки для совместимости
        if coordinators.embedding_coordinator.is_some() {
            initialization_tasks.push(tokio::spawn(async move {
                debug!("✅ EmbeddingCoordinator инициализирован (заглушка)");
                Ok(())
            }));
        }

        if coordinators.search_coordinator.is_some() {
            initialization_tasks.push(tokio::spawn(async move {
                debug!("✅ SearchCoordinator инициализирован (заглушка)");
                Ok(())
            }));
        }

        if coordinators.health_manager.is_some() {
            initialization_tasks.push(tokio::spawn(async move {
                debug!("✅ HealthManager инициализирован (заглушка)");
                Ok(())
            }));
        }

        if coordinators.resource_controller.is_some() {
            initialization_tasks.push(tokio::spawn(async move {
                debug!("✅ ResourceController инициализирован (заглушка)");
                Ok(())
            }));
        }

        // Ждем завершения всех инициализаций
        for task in initialization_tasks {
            match task.await {
                Ok(Ok(_)) => {
                    // Успешная инициализация
                }
                Ok(Err(e)) => {
                    warn!("⚠️ Ошибка инициализации координатора: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    warn!("⚠️ Panic при инициализации координатора: {}", e);
                    return Err(anyhow::anyhow!("Panic при инициализации: {}", e));
                }
            }
        }

        info!("✅ Все координаторы инициализированы");
        Ok(())
    }

    /// Получить embedding coordinator
    #[allow(dead_code)]
    fn get_embedding_coordinator(&self) -> Option<Arc<EmbeddingCoordinatorImpl>> {
        // NOTE: Это blocking call, но мы используем try_read для неблокирующего доступа
        if let Ok(coordinators) = self.coordinators.try_read() {
            coordinators.embedding_coordinator.clone()
        } else {
            None
        }
    }

    /// Получить search coordinator
    #[allow(dead_code)]
    fn get_search_coordinator(&self) -> Option<Arc<SearchCoordinatorImpl>> {
        if let Ok(coordinators) = self.coordinators.try_read() {
            coordinators.search_coordinator.clone()
        } else {
            None
        }
    }

    /// Получить health manager
    #[allow(dead_code)]
    fn get_health_manager(&self) -> Option<Arc<HealthManager>> {
        if let Ok(coordinators) = self.coordinators.try_read() {
            coordinators.health_manager.clone()
        } else {
            None
        }
    }

    /// Получить resource controller
    #[allow(dead_code)]
    fn get_resource_controller(&self) -> Option<Arc<ResourceController>> {
        if let Ok(coordinators) = self.coordinators.try_read() {
            coordinators.resource_controller.clone()
        } else {
            None
        }
    }

    /// Shutdown всех координаторов
    #[allow(dead_code)]
    async fn shutdown_coordinators(&self) -> Result<()> {
        info!("🔌 Shutdown координаторов...");

        let coordinators = self.coordinators.read().await;
        let mut shutdown_tasks: Vec<tokio::task::JoinHandle<Result<(), anyhow::Error>>> = vec![];

        // NOTE: Используем заглушки для shutdown
        if coordinators.embedding_coordinator.is_some() {
            shutdown_tasks.push(tokio::spawn(async move {
                debug!("✅ EmbeddingCoordinator остановлен (заглушка)");
                Ok(())
            }));
        }

        if coordinators.search_coordinator.is_some() {
            shutdown_tasks.push(tokio::spawn(async move {
                debug!("✅ SearchCoordinator остановлен (заглушка)");
                Ok(())
            }));
        }

        if coordinators.health_manager.is_some() {
            shutdown_tasks.push(tokio::spawn(async move {
                debug!("✅ HealthManager остановлен (заглушка)");
                Ok(())
            }));
        }

        if coordinators.resource_controller.is_some() {
            shutdown_tasks.push(tokio::spawn(async move {
                debug!("✅ ResourceController остановлен (заглушка)");
                Ok(())
            }));
        }

        // Ждем завершения всех shutdown операций
        for task in shutdown_tasks {
            if let Err(e) = task.await {
                warn!("⚠️ Ошибка shutdown координатора: {}", e);
            }
        }

        info!("✅ Все координаторы остановлены");
        Ok(())
    }

    /// Подсчитать активные координаторы
    #[allow(dead_code)]
    fn count_active_coordinators(&self) -> usize {
        if let Ok(coordinators) = self.coordinators.try_read() {
            let mut count = 0;
            if coordinators.embedding_coordinator.is_some() { count += 1; }
            if coordinators.search_coordinator.is_some() { count += 1; }
            if coordinators.health_manager.is_some() { count += 1; }
            if coordinators.resource_controller.is_some() { count += 1; }
            count
        } else {
            0
        }
    }
}

impl Default for CoordinatorService {
    fn default() -> Self {
        Self::new()
    }
}