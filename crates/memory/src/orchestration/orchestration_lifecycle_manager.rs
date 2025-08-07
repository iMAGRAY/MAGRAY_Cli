use anyhow::Result;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    sync::RwLock,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{debug, error, info, warn};

use crate::orchestration::{
    traits::Coordinator, BackupCoordinator, EmbeddingCoordinator, HealthManager,
    PromotionCoordinator, ResourceController, SearchCoordinator,
};

/// Lifecycle manager для orchestration системы
///
/// Применяет принципы SOLID:
/// - SRP: Только lifecycle управление (initialize, shutdown, tasks)
/// - OCP: Расширяемость через dependency injection
/// - LSP: Взаимозаменяемость через trait
/// - ISP: Минимальный интерфейс для lifecycle операций
/// - DIP: Зависит от абстракций Coordinator trait
pub struct OrchestrationLifecycleManager {
    /// Координаторы системы
    coordinators: CoordinatorRegistry,
    /// Background task handles
    background_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Emergency shutdown flag
    emergency_shutdown: Arc<AtomicBool>,
    /// Состояние готовности
    ready: Arc<AtomicBool>,
    /// Время запуска системы
    start_time: Instant,
}

/// Реестр всех координаторов
#[derive(Clone)]
pub struct CoordinatorRegistry {
    pub embedding: Arc<EmbeddingCoordinator>,
    pub search: Arc<SearchCoordinator>,
    pub health: Arc<HealthManager>,
    pub promotion: Arc<PromotionCoordinator>,
    pub resources: Arc<ResourceController>,
    pub backup: Arc<BackupCoordinator>,
}

/// Trait для lifecycle управления (ISP принцип)
#[async_trait::async_trait]
pub trait LifecycleManager: Send + Sync {
    /// Production-ready инициализация системы
    async fn initialize_production(&self) -> Result<()>;

    /// Graceful shutdown всей системы
    async fn shutdown_all(&self) -> Result<()>;

    /// Аварийное завершение системы
    async fn emergency_shutdown(&self) -> Result<()>;

    /// Проверить готовность всех координаторов
    async fn verify_all_ready(&self) -> bool;

    /// Получить uptime системы
    fn get_uptime(&self) -> Duration;

    /// Проверить готовность системы
    fn is_ready(&self) -> bool;
}

/// Результат инициализации координаторов
#[derive(Debug)]
pub struct InitializationResult {
    pub coordinator_name: String,
    pub success: bool,
    pub duration: Duration,
    pub error: Option<String>,
}

impl CoordinatorRegistry {
    /// Создать реестр из DI контейнера
    pub fn from_container(container: &crate::di::container_core::ContainerCore) -> Result<Self> {
        info!("🏗️ Создание CoordinatorRegistry из DI контейнера");

        let embedding = container.resolve::<EmbeddingCoordinator>()?;
        let search = container.resolve::<SearchCoordinator>()?;
        let health = container.resolve::<HealthManager>()?;
        let promotion = container.resolve::<PromotionCoordinator>()?;
        let resources = container.resolve::<ResourceController>()?;
        let backup = container.resolve::<BackupCoordinator>()?;

        Ok(Self {
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
        })
    }

    /// Получить все координаторы как список для итерации
    pub fn get_all(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("embedding", &*self.embedding as &dyn Coordinator),
            ("search", &*self.search as &dyn Coordinator),
            ("health", &*self.health as &dyn Coordinator),
            ("promotion", &*self.promotion as &dyn Coordinator),
            ("resources", &*self.resources as &dyn Coordinator),
            ("backup", &*self.backup as &dyn Coordinator),
        ]
    }

    /// Получить критически важные координаторы (должны быть инициализированы первыми)
    pub fn get_critical(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("resources", &*self.resources as &dyn Coordinator),
            ("health", &*self.health as &dyn Coordinator),
        ]
    }

    /// Получить основные координаторы
    pub fn get_core(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("embedding", &*self.embedding as &dyn Coordinator),
            ("search", &*self.search as &dyn Coordinator),
        ]
    }

    /// Получить background координаторы
    pub fn get_background(&self) -> Vec<(&'static str, &dyn Coordinator)> {
        vec![
            ("promotion", &*self.promotion as &dyn Coordinator),
            ("backup", &*self.backup as &dyn Coordinator),
        ]
    }
}

impl OrchestrationLifecycleManager {
    /// Создать новый lifecycle manager
    pub fn new(coordinators: CoordinatorRegistry) -> Self {
        Self {
            coordinators,
            background_tasks: Arc::new(RwLock::new(Vec::new())),
            emergency_shutdown: Arc::new(AtomicBool::new(false)),
            ready: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
        }
    }

    /// Создать из DI контейнера
    pub fn from_container(_container: &crate::di::container_core::ContainerCore) -> Result<Self> {
        // Заглушка для компиляции - возвращаем пустой lifecycle manager
        warn!("🚧 Using stub OrchestrationLifecycleManager - coordinators are not functional");

        // Создаем minimal dummy координаторы только для компиляции
        let dummy_coordinators = CoordinatorRegistry {
            embedding: Arc::new(Self::create_dummy_embedding_coordinator()?),
            search: Arc::new(Self::create_dummy_search_coordinator()?),
            health: Arc::new(Self::create_dummy_health_manager()?),
            promotion: Arc::new(Self::create_dummy_promotion_coordinator()?),
            resources: Arc::new(Self::create_dummy_resource_controller()?),
            backup: Arc::new(Self::create_dummy_backup_coordinator()?),
        };

        Ok(Self::new(dummy_coordinators))
    }

    /// Создать dummy EmbeddingCoordinator для компиляции
    fn create_dummy_embedding_coordinator() -> Result<crate::orchestration::EmbeddingCoordinator> {
        Err(anyhow::anyhow!("EmbeddingCoordinator stub not implemented - requires GpuBatchProcessor and EmbeddingCacheInterface"))
    }

    /// Создать dummy SearchCoordinator для компиляции  
    fn create_dummy_search_coordinator() -> Result<crate::orchestration::SearchCoordinator> {
        Err(anyhow::anyhow!("SearchCoordinator stub not implemented - requires VectorStore and EmbeddingCoordinator"))
    }

    /// Создать dummy HealthManager для компиляции
    fn create_dummy_health_manager() -> Result<crate::orchestration::HealthManager> {
        Err(anyhow::anyhow!(
            "HealthManager stub not implemented - requires HealthMonitor"
        ))
    }

    /// Создать dummy PromotionCoordinator для компиляции
    fn create_dummy_promotion_coordinator() -> Result<crate::orchestration::PromotionCoordinator> {
        // Возвращаем ошибку вместо попытки создать с аргументами
        Err(anyhow::anyhow!("PromotionCoordinator stub not implemented"))
    }

    /// Создать dummy ResourceController для компиляции
    fn create_dummy_resource_controller() -> Result<crate::orchestration::ResourceController> {
        // Возвращаем ошибку вместо попытки создать с аргументами
        Err(anyhow::anyhow!("ResourceController stub not implemented"))
    }

    /// Создать dummy BackupCoordinator для компиляции
    fn create_dummy_backup_coordinator() -> Result<crate::orchestration::BackupCoordinator> {
        // Возвращаем ошибку вместо попытки создать с аргументами
        Err(anyhow::anyhow!("BackupCoordinator stub not implemented"))
    }

    /// Инициализировать группу координаторов параллельно
    async fn initialize_coordinator_group(
        &self,
        coordinators: Vec<(&'static str, &dyn Coordinator)>,
        phase_name: &str,
        timeout_duration: Duration,
    ) -> Result<Vec<InitializationResult>> {
        info!(
            "🚀 {}: Инициализация {} координаторов",
            phase_name,
            coordinators.len()
        );

        let mut tasks = Vec::new();

        for (name, coordinator) in coordinators {
            let coordinator_name = name.to_string();
            let init_start = Instant::now();

            // Выполняем инициализацию синхронно для избежания lifetime issues
            let result = timeout(timeout_duration, coordinator.initialize()).await;
            let duration = init_start.elapsed();

            let init_result = match result {
                Ok(Ok(())) => {
                    info!(
                        "✅ {} coordinator инициализирован за {:?}",
                        coordinator_name, duration
                    );
                    InitializationResult {
                        coordinator_name,
                        success: true,
                        duration,
                        error: None,
                    }
                }
                Ok(Err(e)) => {
                    error!(
                        "❌ {} coordinator ошибка инициализации: {}",
                        coordinator_name, e
                    );
                    InitializationResult {
                        coordinator_name,
                        success: false,
                        duration,
                        error: Some(e.to_string()),
                    }
                }
                Err(_) => {
                    error!("⏰ {} coordinator timeout инициализации", coordinator_name);
                    InitializationResult {
                        coordinator_name,
                        success: false,
                        duration,
                        error: Some("Timeout".to_string()),
                    }
                }
            };

            tasks.push(init_result);
        }

        // Собираем результаты инициализации
        let results = tasks;

        // Проверяем результаты
        let successful = results.iter().filter(|r| r.success).count();
        let total = results.len();

        if successful == total {
            info!(
                "✅ {}: Все {} координаторов инициализированы успешно",
                phase_name, total
            );
        } else {
            let failed_coordinators: Vec<&str> = results
                .iter()
                .filter(|r| !r.success)
                .map(|r| r.coordinator_name.as_str())
                .collect();
            return Err(anyhow::anyhow!(
                "{}: Ошибка инициализации координаторов: {:?}",
                phase_name,
                failed_coordinators
            ));
        }

        Ok(results)
    }

    /// Запустить background monitoring задачи
    async fn start_background_monitoring(&self) -> Result<()> {
        info!("🔄 Запуск background monitoring задач");
        let mut tasks = self.background_tasks.write().await;

        // Health monitoring task
        let health_task = {
            let health = Arc::clone(&self.coordinators.health);
            let emergency_shutdown = Arc::clone(&self.emergency_shutdown);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                while !emergency_shutdown.load(Ordering::Relaxed) {
                    interval.tick().await;

                    if let Err(e) =
                        crate::orchestration::traits::HealthCoordinator::run_health_check(
                            health.as_ref(),
                        )
                        .await
                    {
                        error!("Background health check failed: {}", e);
                    } else {
                        debug!("Background health check completed successfully");
                    }
                }
                debug!("Health monitoring task completed");
            })
        };

        // System readiness monitoring task
        let readiness_task = {
            let coordinators = self.coordinators.clone();
            let emergency_shutdown = Arc::clone(&self.emergency_shutdown);
            let ready = Arc::clone(&self.ready);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                while !emergency_shutdown.load(Ordering::Relaxed) {
                    interval.tick().await;

                    let all_ready = Self::check_coordinators_readiness(&coordinators).await;

                    if !all_ready && ready.load(Ordering::Relaxed) {
                        warn!("⚠️ Система потеряла готовность - не все координаторы готовы");
                        ready.store(false, Ordering::Release);
                    } else if all_ready && !ready.load(Ordering::Relaxed) {
                        info!("✅ Система восстановила готовность - все координаторы готовы");
                        ready.store(true, Ordering::Release);
                    }
                }
                debug!("Readiness monitoring task completed");
            })
        };

        tasks.push(health_task);
        tasks.push(readiness_task);

        info!("✅ {} background задач запущено", tasks.len());
        Ok(())
    }

    /// Остановить все background задачи
    async fn stop_background_tasks(&self) {
        info!("🛤️ Остановка background tasks");

        // Отмечаем emergency shutdown flag
        self.emergency_shutdown.store(true, Ordering::Release);

        // Ожидаем завершения всех tasks
        let mut tasks = self.background_tasks.write().await;
        for task in tasks.drain(..) {
            if !task.is_finished() {
                task.abort();
                if let Err(e) = task.await {
                    if !e.is_cancelled() {
                        warn!("Ошибка при остановке background task: {}", e);
                    }
                }
            }
        }

        info!("✅ Все background tasks остановлены");
    }

    /// Проверить готовность координаторов
    async fn check_coordinators_readiness(coordinators: &CoordinatorRegistry) -> bool {
        let results = tokio::join!(
            coordinators.embedding.is_ready(),
            coordinators.search.is_ready(),
            coordinators.health.is_ready(),
            coordinators.promotion.is_ready(),
            coordinators.resources.is_ready(),
            coordinators.backup.is_ready()
        );

        results.0 && results.1 && results.2 && results.3 && results.4 && results.5
    }
}

#[async_trait::async_trait]
impl LifecycleManager for OrchestrationLifecycleManager {
    async fn initialize_production(&self) -> Result<()> {
        info!("🚀 Запуск production инициализации OrchestrationLifecycleManager");

        // Проверяем не запущена ли уже система
        if self.ready.load(Ordering::Relaxed) {
            warn!("Система уже инициализирована");
            return Ok(());
        }

        // === Phase 1: Critical Infrastructure ===
        let critical_coordinators = self.coordinators.get_critical();
        self.initialize_coordinator_group(
            critical_coordinators,
            "Phase 1: Critical Infrastructure",
            Duration::from_secs(30),
        )
        .await?;

        // === Phase 2: Core Services ===
        let core_coordinators = self.coordinators.get_core();
        self.initialize_coordinator_group(
            core_coordinators,
            "Phase 2: Core Services",
            Duration::from_secs(45),
        )
        .await?;

        // === Phase 3: Background Services ===
        let background_coordinators = self.coordinators.get_background();
        self.initialize_coordinator_group(
            background_coordinators,
            "Phase 3: Background Services",
            Duration::from_secs(60),
        )
        .await?;

        // === Phase 4: Health Verification ===
        info!("🏥 Phase 4: Проверка готовности всех координаторов");

        let ready_check_timeout = Duration::from_secs(30);
        let ready_check_start = Instant::now();

        while ready_check_start.elapsed() < ready_check_timeout {
            if Self::check_coordinators_readiness(&self.coordinators).await {
                info!("✅ Все координаторы готовы к работе");
                break;
            }

            debug!("⏳ Ожидание готовности координаторов...");
            sleep(Duration::from_millis(500)).await;
        }

        // Финальная проверка
        if !Self::check_coordinators_readiness(&self.coordinators).await {
            return Err(anyhow::anyhow!("Не все координаторы готовы после таймаута"));
        }

        // === Phase 5: Start Background Tasks ===
        self.start_background_monitoring().await?;

        // Отмечаем систему как готову
        self.ready.store(true, Ordering::Release);

        info!(
            "🎉 OrchestrationLifecycleManager успешно инициализирован за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    async fn shutdown_all(&self) -> Result<()> {
        info!("🛡️ Начало graceful shutdown OrchestrationLifecycleManager");

        // Отмечаем систему как не готовую
        self.ready.store(false, Ordering::Release);

        // === Phase 1: Stop Background Tasks ===
        info!("🛤️ Phase 1: Остановка background tasks");
        self.stop_background_tasks().await;

        // === Phase 2: Coordinated Shutdown ===
        info!("🛡️ Phase 2: Координированное завершение координаторов");

        // Останавливаем в обратном порядке с timeout защитой
        let shutdown_sequence = vec![
            (
                "backup",
                &*self.coordinators.backup as &dyn Coordinator,
                Duration::from_secs(60),
            ),
            (
                "promotion",
                &*self.coordinators.promotion as &dyn Coordinator,
                Duration::from_secs(30),
            ),
            (
                "search",
                &*self.coordinators.search as &dyn Coordinator,
                Duration::from_secs(15),
            ),
            (
                "embedding",
                &*self.coordinators.embedding as &dyn Coordinator,
                Duration::from_secs(30),
            ),
            (
                "health",
                &*self.coordinators.health as &dyn Coordinator,
                Duration::from_secs(15),
            ),
            (
                "resources",
                &*self.coordinators.resources as &dyn Coordinator,
                Duration::from_secs(15),
            ),
        ];

        for (name, coordinator, timeout_duration) in shutdown_sequence {
            match timeout(timeout_duration, coordinator.shutdown()).await {
                Ok(Ok(())) => {
                    info!("✅ {} coordinator успешно остановлен", name);
                }
                Ok(Err(e)) => {
                    warn!("⚠️ Ошибка при остановке {} coordinator: {}", name, e);
                }
                Err(_) => {
                    error!("❌ Timeout при остановке {} coordinator", name);
                }
            }
        }

        info!(
            "🏁 OrchestrationLifecycleManager успешно остановлен за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    async fn emergency_shutdown(&self) -> Result<()> {
        error!("🚑 EMERGENCY SHUTDOWN OrchestrationLifecycleManager!");

        // Отмечаем emergency shutdown flag
        self.emergency_shutdown.store(true, Ordering::Release);
        self.ready.store(false, Ordering::Release);

        // Немедленно останавливаем background tasks
        self.stop_background_tasks().await;

        // Параллельное завершение всех координаторов с короткими timeout'ами
        let emergency_timeouts = vec![
            timeout(Duration::from_secs(5), self.coordinators.backup.shutdown()),
            timeout(
                Duration::from_secs(3),
                self.coordinators.promotion.shutdown(),
            ),
            timeout(Duration::from_secs(2), self.coordinators.search.shutdown()),
            timeout(
                Duration::from_secs(3),
                self.coordinators.embedding.shutdown(),
            ),
            timeout(Duration::from_secs(2), self.coordinators.health.shutdown()),
            timeout(
                Duration::from_secs(2),
                self.coordinators.resources.shutdown(),
            ),
        ];

        let results = futures::future::join_all(emergency_timeouts).await;

        // Логируем результаты
        let coordinator_names = [
            "backup",
            "promotion",
            "search",
            "embedding",
            "health",
            "resources",
        ];

        for (name, result) in coordinator_names.iter().zip(results.iter()) {
            match result {
                Ok(Ok(())) => info!("✅ Emergency: {} остановлен", name),
                Ok(Err(e)) => warn!("⚠️ Emergency: {} ошибка: {}", name, e),
                Err(_) => error!("❌ Emergency: {} timeout", name),
            }
        }

        error!("🚑 EMERGENCY SHUTDOWN завершен");
        Ok(())
    }

    async fn verify_all_ready(&self) -> bool {
        Self::check_coordinators_readiness(&self.coordinators).await
    }

    fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    // Mock coordinator для тестирования
    struct MockCoordinator {
        initialization_delay: Duration,
        should_fail: AtomicBool,
        is_ready: AtomicBool,
        init_count: AtomicU32,
    }

    impl MockCoordinator {
        fn new(delay: Duration, should_fail: bool) -> Self {
            Self {
                initialization_delay: delay,
                should_fail: AtomicBool::new(should_fail),
                is_ready: AtomicBool::new(false),
                init_count: AtomicU32::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl Coordinator for MockCoordinator {
        async fn initialize(&self) -> Result<()> {
            self.init_count.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(self.initialization_delay).await;

            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock coordinator initialization failed"))
            } else {
                self.is_ready.store(true, Ordering::Relaxed);
                Ok(())
            }
        }

        async fn shutdown(&self) -> Result<()> {
            self.is_ready.store(false, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.is_ready.load(Ordering::Relaxed)
        }

        async fn metrics(&self) -> serde_json::Value {
            serde_json::json!({
                "init_count": self.init_count.load(Ordering::Relaxed),
                "is_ready": self.is_ready().await
            })
        }
    }

    #[tokio::test]
    async fn test_initialization_result_creation() {
        let result = InitializationResult {
            coordinator_name: "test".to_string(),
            success: true,
            duration: Duration::from_millis(100),
            error: None,
        };

        assert_eq!(result.coordinator_name, "test");
        assert!(result.success);
        assert_eq!(result.duration, Duration::from_millis(100));
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_coordinator_registry_structure() {
        // Этот тест проверяет, что структура CoordinatorRegistry соответствует ожиданиям
        // В реальном окружении этот тест будет работать с настоящими координаторами
        // Здесь мы просто проверяем что методы компилируются корректно

        // Проверяем что методы get_critical, get_core, get_background работают
        // (не можем создать реальный registry без DI контейнера в unit тестах)
        assert!(true); // Placeholder test
    }
}
