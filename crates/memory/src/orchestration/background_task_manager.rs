//! Background Task Manager - управление фоновыми задачами
//!
//! Применяет Single Responsibility Principle - только управление background tasks.

use anyhow::Result;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::RwLock, task::JoinHandle, time::interval};
use tracing::{debug, error, info, warn};

use super::traits::{Coordinator, HealthCoordinator};
#[cfg(feature = "legacy-orchestrator")]
use super::{circuit_breaker_manager::CircuitBreakerManager, metrics_collector::MetricsCollector};
use common::{service_macros::CoordinatorMacroHelpers, service_traits::*};

/// Background Task Manager для управления фоновыми задачами
#[derive(Debug)]
pub struct BackgroundTaskManager {
    /// Handles активных задач
    task_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Флаг остановки
    shutdown_requested: Arc<AtomicBool>,
    /// Активность manager'а
    active: Arc<AtomicBool>,
}

impl CoordinatorMacroHelpers for BackgroundTaskManager {
    async fn perform_coordinator_init(&self) -> anyhow::Result<()> {
        info!("🚀 Инициализация BackgroundTaskManager");
        self.active.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn check_readiness(&self) -> bool {
        self.active.load(Ordering::Relaxed) && !self.shutdown_requested.load(Ordering::Relaxed)
    }

    async fn perform_health_check(&self) -> anyhow::Result<()> {
        if self.shutdown_requested.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!(
                "BackgroundTaskManager в процессе остановки"
            ));
        }

        let tasks = self.task_handles.read().await;
        let active_tasks = tasks.len();

        if active_tasks == 0 && self.active.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!(
                "Нет активных задач при работающем менеджере"
            ));
        }

        Ok(())
    }

    async fn perform_coordinator_shutdown(&self) -> anyhow::Result<()> {
        self.stop_all_tasks().await;
        Ok(())
    }

    async fn collect_coordinator_metrics(&self) -> serde_json::Value {
        let tasks = self.task_handles.read().await;
        serde_json::json!({
            "active_tasks": tasks.len(),
            "shutdown_requested": self.shutdown_requested.load(Ordering::Relaxed),
            "manager_active": self.active.load(Ordering::Relaxed),
        })
    }
}

impl BackgroundTaskManager {
    /// Создать новый Background Task Manager
    pub fn new() -> Self {
        Self {
            task_handles: Arc::new(RwLock::new(Vec::new())),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Запустить все background задачи
    pub async fn start_all_tasks(
        &self,
        health_coordinator: Arc<dyn HealthCoordinator>,
        #[cfg(feature = "legacy-orchestrator")]
        circuit_breaker_manager: Arc<CircuitBreakerManager>,
        #[cfg(feature = "legacy-orchestrator")]
        metrics_collector: Arc<MetricsCollector>,
    ) -> Result<()> {
        info!("🔄 Запуск всех background задач");

        if self.active.load(Ordering::Relaxed) {
            warn!("Background задачи уже запущены");
            return Ok(());
        }

        let mut tasks = self.task_handles.write().await;

        // Health monitoring task
        #[cfg(feature = "legacy-orchestrator")]
        let health_task =
            self.create_health_monitoring_task(health_coordinator, metrics_collector.clone());
        #[cfg(not(feature = "legacy-orchestrator"))]
        let health_task = tokio::spawn(async move {
            let _ = health_coordinator.health_check().await;
        });

        // Circuit breaker monitoring task
        #[cfg(feature = "legacy-orchestrator")]
        let circuit_breaker_task =
            self.create_circuit_breaker_monitoring_task(circuit_breaker_manager);
        #[cfg(not(feature = "legacy-orchestrator"))]
        let circuit_breaker_task = tokio::spawn(async move {});

        // Metrics collection task
        #[cfg(feature = "legacy-orchestrator")]
        let metrics_task = self.create_metrics_collection_task(metrics_collector);
        #[cfg(not(feature = "legacy-orchestrator"))]
        let metrics_task = tokio::spawn(async move {});

        tasks.push(health_task);
        tasks.push(circuit_breaker_task);
        tasks.push(metrics_task);

        self.active.store(true, Ordering::Relaxed);
        info!("✅ {} background задач запущено", tasks.len());

        Ok(())
    }

    /// Создать health monitoring task
    #[cfg(feature = "legacy-orchestrator")]
    fn create_health_monitoring_task(
        &self,
        health_coordinator: Arc<dyn HealthCoordinator>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> JoinHandle<()> {
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        tokio::spawn(async move {
            let mut health_interval = interval(Duration::from_secs(30));

            while !shutdown_requested.load(Ordering::Relaxed) {
                health_interval.tick().await;

                let start_time = std::time::Instant::now();
                match health_coordinator.health_check().await {
                    Ok(()) => {
                        let duration = start_time.elapsed().as_millis() as u64;
                        metrics_collector
                            .record_operation("health", duration, true)
                            .await;
                        debug!("✅ Health check прошел успешно за {}ms", duration);
                    }
                    Err(e) => {
                        let duration = start_time.elapsed().as_millis() as u64;
                        metrics_collector
                            .record_operation("health", duration, false)
                            .await;
                        error!("❌ Health check не удался: {}", e);
                    }
                }

                // Обновляем uptime метрики
                metrics_collector.update_resource_usage(0, 0).await; // Placeholder values
            }

            debug!("🛑 Health monitoring task завершена");
        })
    }

    /// Создать circuit breaker monitoring task
    #[cfg(feature = "legacy-orchestrator")]
    fn create_circuit_breaker_monitoring_task(
        &self,
        circuit_breaker_manager: Arc<CircuitBreakerManager>,
    ) -> JoinHandle<()> {
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        tokio::spawn(async move {
            let mut cb_interval = interval(Duration::from_secs(60));

            while !shutdown_requested.load(Ordering::Relaxed) {
                cb_interval.tick().await;

                // Мониторинг состояний circuit breakers
                circuit_breaker_manager.monitor_states().await;
            }

            debug!("🛑 Circuit breaker monitoring task завершена");
        })
    }

    /// Создать metrics collection task
    #[cfg(feature = "legacy-orchestrator")]
    fn create_metrics_collection_task(
        &self,
        metrics_collector: Arc<MetricsCollector>,
    ) -> JoinHandle<()> {
        let shutdown_requested = Arc::clone(&self.shutdown_requested);

        tokio::spawn(async move {
            let mut metrics_interval = interval(Duration::from_secs(60));

            while !shutdown_requested.load(Ordering::Relaxed) {
                metrics_interval.tick().await;

                // Сохраняем текущие метрики в историю
                metrics_collector.save_to_history().await;

                // Запускаем адаптивную оптимизацию если нужно
                if let Err(e) = metrics_collector.run_adaptive_optimization().await {
                    warn!("Ошибка адаптивной оптимизации: {}", e);
                }
            }

            debug!("🛑 Metrics collection task завершена");
        })
    }

    /// Остановить все background задачи (legacy method - используйте coordinator shutdown)
    #[deprecated(note = "Используйте coordinator shutdown из trait")]
    pub async fn stop_all_tasks(&self) {
        info!("🛑 Остановка всех background задач");

        // Устанавливаем флаг остановки
        self.shutdown_requested.store(true, Ordering::Relaxed);
        self.active.store(false, Ordering::Relaxed);

        // Ждем завершения всех задач
        let mut tasks = self.task_handles.write().await;
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

        info!("✅ Все background задачи остановлены");
    }

    /// Проверить активность manager'а
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Получить количество активных задач
    pub async fn get_active_tasks_count(&self) -> usize {
        let tasks = self.task_handles.read().await;
        tasks.iter().filter(|task| !task.is_finished()).count()
    }

    /// Добавить кастомную background задачу
    pub async fn add_custom_task<F, Fut>(&self, name: &str, task_fn: F) -> Result<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        if !self.active.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!(
                "Background task manager не активен, нельзя добавить задачу '{}'",
                name
            ));
        }

        let shutdown_requested = Arc::clone(&self.shutdown_requested);
        let task_name = name.to_string();

        let handle = tokio::spawn(async move {
            debug!("🚀 Запущена кастомная задача: {}", task_name);
            task_fn().await;
            debug!("✅ Кастомная задача завершена: {}", task_name);
        });

        let mut tasks = self.task_handles.write().await;
        tasks.push(handle);

        info!("➕ Добавлена кастомная задача: {}", name);
        Ok(())
    }
}

// Ручная реализация Coordinator без макроса, чтобы избежать привязки к конкретным ошибкам
use crate::orchestration::traits as _traits_mod;

#[async_trait::async_trait]
impl _traits_mod::Coordinator for BackgroundTaskManager {
    async fn initialize(&self) -> anyhow::Result<()> {
        self.perform_coordinator_init().await
    }

    async fn is_ready(&self) -> bool {
        self.check_readiness().await
    }

    async fn health_check(&self) -> anyhow::Result<()> {
        if !self.is_ready().await {
            return Err(anyhow::anyhow!("BackgroundTaskManager не готов"));
        }
        Ok(())
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        self.perform_coordinator_shutdown().await
    }

    async fn metrics(&self) -> serde_json::Value {
        self.collect_coordinator_metrics().await
    }
}

impl Default for BackgroundTaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "legacy-orchestrator"))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::{sleep, Duration};

    // Mock health coordinator for testing
    #[derive(Debug)]
    struct MockHealthCoordinator {
        call_count: Arc<AtomicU32>,
        should_fail: Arc<AtomicBool>,
    }

    impl MockHealthCoordinator {
        fn new() -> Self {
            Self {
                call_count: Arc::new(AtomicU32::new(0)),
                should_fail: Arc::new(AtomicBool::new(false)),
            }
        }

        fn get_call_count(&self) -> u32 {
            self.call_count.load(Ordering::Relaxed)
        }

        fn set_should_fail(&self, should_fail: bool) {
            self.should_fail.store(should_fail, Ordering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl std::fmt::Debug for MockHealthCoordinator {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MockHealthCoordinator").finish()
        }
    }

    impl _traits_mod::Coordinator for MockHealthCoordinator {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            true
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn metrics(&self) -> serde_json::Value {
            serde_json::json!({
                "mock_health_coordinator": true,
                "call_count": self.call_count.load(Ordering::Relaxed),
            })
        }
    }

    #[async_trait::async_trait]
    impl HealthCoordinator for MockHealthCoordinator {
        async fn system_health(&self) -> Result<crate::health::SystemHealthStatus> {
            Ok(crate::health::SystemHealthStatus {
                overall_status: crate::health::HealthStatus::Healthy,
                component_statuses: std::collections::HashMap::new(),
                active_alerts: vec![],
                metrics_summary: std::collections::HashMap::new(),
                last_updated: chrono::Utc::now(),
                uptime_seconds: 0,
            })
        }
        async fn component_health(&self, _component: &str) -> Result<bool> { Ok(true) }
        async fn run_health_check(&self) -> Result<()> { Ok(()) }
        async fn get_alerts(&self) -> Vec<String> { vec![] }
        async fn clear_alerts(&self) -> Result<()> { Ok(()) }
        async fn health_check(&self) -> Result<()> {
            self.call_count.fetch_add(1, Ordering::Relaxed);

            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock health check failed"))
            } else {
                Ok(())
            }
        }

        async fn system_health(&self) -> Result<crate::health::SystemHealthStatus> {
            Ok(crate::health::SystemHealthStatus {
                overall_status: crate::health::HealthStatus::Healthy,
                component_statuses: std::collections::HashMap::new(),
                active_alerts: vec![],
                metrics_summary: std::collections::HashMap::new(),
                last_updated: chrono::Utc::now(),
                uptime_seconds: 0,
            })
        }

        async fn run_health_check(&self) -> Result<()> {
            self.health_check().await
        }
    }

    #[tokio::test]
    async fn test_background_task_manager_creation() {
        let manager = BackgroundTaskManager::new();
        assert!(!manager.is_active());
        assert_eq!(manager.get_active_tasks_count().await, 0);
    }

    #[tokio::test]
    async fn test_task_lifecycle() {
        let manager = BackgroundTaskManager::new();
        let health_coordinator = Arc::new(MockHealthCoordinator::new());
        #[cfg(feature = "legacy-orchestrator")]
        let circuit_breaker_manager = Arc::new(CircuitBreakerManager::new());
        #[cfg(feature = "legacy-orchestrator")]
        let metrics_collector = Arc::new(MetricsCollector::new(100));

        #[cfg(feature = "legacy-orchestrator")]
        {
            // Инициализируем стандартные coordinators для circuit breaker manager
            circuit_breaker_manager
                .initialize_standard_coordinators()
                .await
                .expect("Should initialize circuit breakers");

            // Запускаем задачи c legacy поддержкой
            manager
                .start_all_tasks(
                    health_coordinator,
                    circuit_breaker_manager,
                    metrics_collector,
                )
                .await
                .expect("Should start all tasks");
        }
        #[cfg(not(feature = "legacy-orchestrator"))]
        {
            // Запускаем только health task
            manager
                .start_all_tasks(health_coordinator)
                .await
                .expect("Should start health task");
        }

        assert!(manager.is_active());
        #[cfg(feature = "legacy-orchestrator")]
        assert_eq!(manager.get_active_tasks_count().await, 3);
        #[cfg(not(feature = "legacy-orchestrator"))]
        assert_eq!(manager.get_active_tasks_count().await, 1);

        // Даем задачам немного поработать
        sleep(Duration::from_millis(50)).await;

        // Останавливаем задачи
        manager.stop_all_tasks().await;

        assert!(!manager.is_active());
        // После остановки все задачи должны быть завершены
        sleep(Duration::from_millis(50)).await;
        assert_eq!(manager.get_active_tasks_count().await, 0);
    }

    #[tokio::test]
    async fn test_custom_task_addition() {
        let manager = BackgroundTaskManager::new();
        let health_coordinator = Arc::new(MockHealthCoordinator::new());
        #[cfg(feature = "legacy-orchestrator")]
        let circuit_breaker_manager = Arc::new(CircuitBreakerManager::new());
        #[cfg(feature = "legacy-orchestrator")]
        let metrics_collector = Arc::new(MetricsCollector::new(100));

        #[cfg(feature = "legacy-orchestrator")]
        {
            circuit_breaker_manager
                .initialize_standard_coordinators()
                .await
                .expect("Should initialize circuit breakers");

            manager
                .start_all_tasks(
                    health_coordinator,
                    circuit_breaker_manager,
                    metrics_collector,
                )
                .await
                .expect("Should start all tasks");
        }
        #[cfg(not(feature = "legacy-orchestrator"))]
        {
            manager
                .start_all_tasks(health_coordinator)
                .await
                .expect("Should start health task");
        }

        // Добавляем кастомную задачу
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        manager
            .add_custom_task("test_custom", move || async move {
                counter_clone.fetch_add(1, Ordering::Relaxed);
                sleep(Duration::from_millis(10)).await;
            })
            .await
            .expect("Should add custom task");

        #[cfg(feature = "legacy-orchestrator")]
        assert_eq!(manager.get_active_tasks_count().await, 4); // 3 стандартные + 1 кастомная
        #[cfg(not(feature = "legacy-orchestrator"))]
        assert_eq!(manager.get_active_tasks_count().await, 2); // 1 стандартная + 1 кастомная

        // Ждем выполнения кастомной задачи
        sleep(Duration::from_millis(50)).await;

        // Кастомная задача должна была выполниться
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        manager.stop_all_tasks().await;
    }

    #[tokio::test]
    async fn test_cannot_add_custom_task_when_inactive() {
        let manager = BackgroundTaskManager::new();

        // Попытка добавить задачу без запуска manager'а должна завершиться ошибкой
        let result = manager.add_custom_task("test", || async {}).await;

        assert!(result.is_err());
        assert_eq!(manager.get_active_tasks_count().await, 0);
    }
}
