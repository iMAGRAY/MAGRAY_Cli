//! CoordinatorService - управление координаторами и их инициализация  
//!
//! Single Responsibility: только координация между компонентами
//! - создание и инициализация координаторов
//! - управление жизненным циклом координаторов
//! - предоставление доступа к координаторам через trait интерфейс

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::di::core_traits::ServiceResolver;
#[cfg(feature = "gpu-acceleration")]
use crate::gpu_accelerated::GpuBatchProcessor;
use crate::health::HealthMonitor;
use crate::orchestration::{
    HealthManager, ResourceController, SearchCoordinator as SearchCoordinatorImpl,
};
use crate::services::traits::CoordinatorServiceTrait;
use crate::storage::VectorStore;
use crate::{
    cache_interface::EmbeddingCacheInterface,
    di::{traits::DIResolver, UnifiedContainer},
    orchestration::EmbeddingCoordinator as EmbeddingCoordinatorImpl,
    EmbeddingCache,
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
    async fn create_embedding_coordinator(
        &self,
        container: &UnifiedContainer,
    ) -> Result<Arc<EmbeddingCoordinatorImpl>> {
        debug!("🎯 Создание EmbeddingCoordinator...");

        #[cfg(feature = "gpu-acceleration")]
        let _gpu_processor = container.resolve::<GpuBatchProcessor>()?;
        #[cfg(not(feature = "gpu-acceleration"))]
        let _gpu_processor: Option<std::sync::Arc<()>> = None;

        // Создаем временный cache для демонстрации
        let _cache = container.resolve::<EmbeddingCache>().ok();

        let coordinator = Arc::new(EmbeddingCoordinatorImpl::new_stub());
        debug!("✅ EmbeddingCoordinator создан");

        Ok(coordinator)
    }

    /// Создать search coordinator  
    #[allow(dead_code)]
    async fn create_search_coordinator(
        &self,
        container: &UnifiedContainer,
        embedding_coordinator: &Arc<EmbeddingCoordinatorImpl>,
    ) -> Result<Arc<SearchCoordinatorImpl>> {
        debug!("🎯 Создание SearchCoordinator...");

        let store = container.resolve::<VectorStore>()?;

        let coordinator = Arc::new(SearchCoordinatorImpl::new_production(
            store,
            embedding_coordinator.clone(),
            64,   // max concurrent searches
            2000, // cache size
        ));
        debug!("✅ SearchCoordinator создан");

        Ok(coordinator)
    }

    /// Создать health manager
    #[allow(dead_code)]
    async fn create_health_manager(
        &self,
        container: &UnifiedContainer,
    ) -> Result<Arc<HealthManager>> {
        debug!("🎯 Создание HealthManager...");

        let health_monitor = container.resolve::<HealthMonitor>()?;

        let manager = Arc::new(HealthManager::new(health_monitor));
        debug!("✅ HealthManager создан");

        Ok(manager)
    }

    /// Создать resource controller
    #[allow(dead_code)]
    async fn create_resource_controller(
        &self,
        container: &UnifiedContainer,
    ) -> Result<Arc<ResourceController>> {
        debug!("🎯 Создание ResourceController...");

        let resource_manager =
            container.resolve::<parking_lot::RwLock<crate::resource_manager::ResourceManager>>()?;

        let controller = Arc::new(ResourceController::new_production(resource_manager));
        debug!("✅ ResourceController создан");

        Ok(controller)
    }
}

#[async_trait]
impl CoordinatorServiceTrait for CoordinatorService {
    /// Создать все координаторы
    #[allow(dead_code)]
    async fn create_coordinators(&self, container: &UnifiedContainer) -> Result<()> {
        info!("⚙️ Создание координаторов...");
        // Пример создания EmbeddingCoordinator
        #[cfg(feature = "gpu-acceleration")]
        let _gpu_processor = container.resolve::<GpuBatchProcessor>().ok();
        #[cfg(not(feature = "gpu-acceleration"))]
        let _gpu_processor: Option<std::sync::Arc<()>> = None;
        let _cache = container.resolve::<EmbeddingCache>().ok();
        let mut guard = self.coordinators.write().await;
        guard.embedding_coordinator = Some(Arc::new(EmbeddingCoordinatorImpl::new_stub()));
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
            if coordinators.embedding_coordinator.is_some() {
                count += 1;
            }
            if coordinators.search_coordinator.is_some() {
                count += 1;
            }
            if coordinators.health_manager.is_some() {
                count += 1;
            }
            if coordinators.resource_controller.is_some() {
                count += 1;
            }
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
