//! LifecycleManager - управление жизненным циклом orchestrator'а
//!
//! Реализует Single Responsibility Principle для управления
//! инициализацией, запуском и остановкой координаторов.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

use crate::orchestration::traits::Coordinator;

/// Фазы жизненного цикла
#[derive(Debug, Clone, PartialEq)]
pub enum LifecyclePhase {
    Uninitialized,
    CriticalInfrastructure,
    CoreServices,
    BackgroundServices,
    HealthVerification,
    BackgroundTasks,
    Ready,
    ShuttingDown,
    Stopped,
}

/// Состояние компонента в жизненном цикле
#[derive(Debug, Clone)]
pub struct ComponentLifecycleState {
    pub name: String,
    pub phase: LifecyclePhase,
    pub initialized: bool,
    pub ready: bool,
    pub healthy: bool,
    pub last_health_check: Option<Instant>,
    pub initialization_time: Option<Duration>,
    pub error_message: Option<String>,
}

/// Конфигурация жизненного цикла
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// Таймауты для разных фаз инициализации
    pub critical_infrastructure_timeout: Duration,
    pub core_services_timeout: Duration,
    pub background_services_timeout: Duration,
    pub health_verification_timeout: Duration,
    pub shutdown_timeout: Duration,

    /// Интервал проверок готовности
    pub readiness_check_interval: Duration,

    /// Максимальное время ожидания готовности всех компонентов
    pub max_readiness_wait_time: Duration,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            critical_infrastructure_timeout: Duration::from_secs(30),
            core_services_timeout: Duration::from_secs(45),
            background_services_timeout: Duration::from_secs(60),
            health_verification_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(60),
            readiness_check_interval: Duration::from_millis(500),
            max_readiness_wait_time: Duration::from_secs(120),
        }
    }
}

/// Менеджер жизненного цикла orchestrator'а
pub struct LifecycleManager {
    config: LifecycleConfig,
    current_phase: Arc<RwLock<LifecyclePhase>>,
    components: Arc<RwLock<HashMap<String, ComponentLifecycleState>>>,
    start_time: Instant,
}

impl LifecycleManager {
    /// Создать новый менеджер жизненного цикла
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            config,
            current_phase: Arc::new(RwLock::new(LifecyclePhase::Uninitialized)),
            components: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Зарегистрировать компонент для управления жизненным циклом
    pub async fn register_component(&self, name: String) {
        let mut components = self.components.write().await;
        components.insert(
            name.clone(),
            ComponentLifecycleState {
                name: name.clone(),
                phase: LifecyclePhase::Uninitialized,
                initialized: false,
                ready: false,
                healthy: false,
                last_health_check: None,
                initialization_time: None,
                error_message: None,
            },
        );
        debug!(
            "Зарегистрирован компонент для lifecycle management: {}",
            name
        );
    }

    /// Получить текущую фазу жизненного цикла
    pub async fn current_phase(&self) -> LifecyclePhase {
        self.current_phase.read().await.clone()
    }

    /// Получить состояние всех компонентов
    pub async fn get_component_states(&self) -> HashMap<String, ComponentLifecycleState> {
        self.components.read().await.clone()
    }

    /// Получить состояние конкретного компонента
    pub async fn get_component_state(&self, name: &str) -> Option<ComponentLifecycleState> {
        let components = self.components.read().await;
        components.get(name).cloned()
    }

    /// Запустить полный цикл инициализации с новым CoordinatorRegistry
    pub async fn initialize_with_registry(
        &self,
        registry: &crate::orchestration::coordinator_registry::CoordinatorRegistry,
    ) -> Result<()> {
        info!("🚀 Запуск production инициализации LifecycleManager с CoordinatorRegistry");

        // Получаем порядок инициализации из реестра
        let init_order = registry.get_initialization_order();

        if init_order.is_empty() {
            warn!("CoordinatorRegistry не содержит координаторов для инициализации");
            return Ok(());
        }

        info!(
            "📋 Порядок инициализации: {:?}",
            init_order
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        );

        // Phase 1: Critical Infrastructure (критические координаторы)
        let critical_coordinators = registry.get_critical_coordinators();
        if !critical_coordinators.is_empty() {
            self.execute_phase_with_coordinators(
                LifecyclePhase::CriticalInfrastructure,
                critical_coordinators,
                self.config.critical_infrastructure_timeout,
            )
            .await?;
        }

        // Phase 2-4: Инициализация в порядке зависимостей
        for (name, coordinator) in &init_order {
            // Пропускаем критические координаторы (уже инициализированы)
            if let Some(metadata) = registry.get_metadata(name) {
                if metadata.is_critical {
                    continue;
                }
            }

            info!("🔧 Инициализация координатора: {}", name);

            // Проверяем готовность зависимостей
            if !registry.check_coordinator_dependencies_ready(name).await {
                return Err(anyhow::anyhow!(
                    "Зависимости координатора '{}' не готовы",
                    name
                ));
            }

            let init_start = std::time::Instant::now();

            // Обновляем фазу компонента
            self.register_component(name.clone()).await;

            // Инициализируем координатор
            if let Err(e) = coordinator.initialize().await {
                error!("❌ Ошибка инициализации {}: {}", name, e);
                return Err(e);
            }

            let init_duration = init_start.elapsed();
            info!("✅ {} инициализирован за {:?}", name, init_duration);
        }

        // Phase 5: Health Verification
        self.verify_all_components_health_with_registry(registry)
            .await?;

        // Phase 6: Mark as ready
        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Ready;
        }

        info!(
            "🎉 Инициализация с CoordinatorRegistry завершена успешно за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    /// Запустить полный цикл инициализации (старый API для совместимости)
    pub async fn initialize_production<T: Coordinator + Send + Sync + 'static>(
        &self,
        coordinators: HashMap<String, Arc<T>>,
    ) -> Result<()> {
        info!("🚀 Запуск production инициализации LifecycleManager");

        // Phase 1: Critical Infrastructure
        self.execute_phase(
            LifecyclePhase::CriticalInfrastructure,
            &coordinators,
            &["resources", "health"],
            self.config.critical_infrastructure_timeout,
        )
        .await?;

        // Phase 2: Core Services
        self.execute_phase(
            LifecyclePhase::CoreServices,
            &coordinators,
            &["embedding", "search"],
            self.config.core_services_timeout,
        )
        .await?;

        // Phase 3: Background Services
        self.execute_phase(
            LifecyclePhase::BackgroundServices,
            &coordinators,
            &["promotion", "backup"],
            self.config.background_services_timeout,
        )
        .await?;

        // Phase 4: Health Verification
        self.verify_all_components_health(&coordinators).await?;

        // Phase 5: Mark as ready
        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Ready;
        }

        info!(
            "🎉 Инициализация завершена успешно за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    /// Выполнить фазу инициализации
    async fn execute_phase<T: Coordinator + Send + Sync + 'static>(
        &self,
        phase: LifecyclePhase,
        coordinators: &HashMap<String, Arc<T>>,
        component_names: &[&str],
        phase_timeout: Duration,
    ) -> Result<()> {
        info!("📊 Phase: {:?} - инициализация компонентов", phase);

        // Обновляем фазу
        {
            let mut current_phase = self.current_phase.write().await;
            *current_phase = phase.clone();
        }

        // Запускаем инициализацию компонентов параллельно
        let mut initialization_tasks = Vec::new();

        for &component_name in component_names {
            if let Some(coordinator) = coordinators.get(component_name) {
                let coordinator = Arc::clone(coordinator);
                let components: Arc<RwLock<HashMap<String, ComponentLifecycleState>>> =
                    Arc::clone(&self.components);
                let name = component_name.to_string();
                let phase_clone = phase.clone();

                let task = tokio::spawn(async move {
                    let init_start = Instant::now();

                    // Обновляем состояние компонента - начало инициализации
                    {
                        let mut comp_states = components.write().await;
                        if let Some(state) = comp_states.get_mut(&name) {
                            state.phase = phase_clone.clone();
                        }
                    }

                    // Выполняем инициализацию
                    let result = coordinator.initialize().await;
                    let init_duration = init_start.elapsed();

                    // Обновляем состояние компонента - результат инициализации
                    {
                        let mut comp_states = components.write().await;
                        if let Some(state) = comp_states.get_mut(&name) {
                            state.initialization_time = Some(init_duration);
                            match &result {
                                Ok(()) => {
                                    state.initialized = true;
                                    state.error_message = None;
                                }
                                Err(e) => {
                                    state.initialized = false;
                                    state.error_message = Some(e.to_string());
                                }
                            }
                        }
                    }

                    (name, result)
                });

                initialization_tasks.push(task);
            }
        }

        // Ждем завершения всех инициализаций с timeout
        let results = timeout(phase_timeout, async {
            let mut results = Vec::new();
            for task in initialization_tasks {
                results.push(
                    task.await
                        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?,
                );
            }
            Ok::<Vec<(String, Result<()>)>, anyhow::Error>(results)
        })
        .await
        .map_err(|_| anyhow::anyhow!("Phase {:?} timeout after {:?}", phase, phase_timeout))??;

        // Анализируем результаты
        let mut failed_components = Vec::new();
        for (name, result) in results {
            match result {
                Ok(()) => {
                    info!("✅ {} инициализирован успешно", name);
                }
                Err(e) => {
                    error!("❌ Ошибка инициализации {}: {}", name, e);
                    failed_components.push(name);
                }
            }
        }

        if !failed_components.is_empty() {
            return Err(anyhow::anyhow!(
                "Не удалось инициализировать компоненты в фазе {:?}: {:?}",
                phase,
                failed_components
            ));
        }

        info!("✅ Phase {:?} завершена успешно", phase);
        Ok(())
    }

    /// Проверить готовность и здоровье всех компонентов
    async fn verify_all_components_health<T: Coordinator + Send + Sync + 'static>(
        &self,
        coordinators: &HashMap<String, Arc<T>>,
    ) -> Result<()> {
        info!("🏥 Phase: Health Verification - проверка готовности всех координаторов");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::HealthVerification;
        }

        let verification_start = Instant::now();
        let verification_timeout = self.config.health_verification_timeout;

        while verification_start.elapsed() < verification_timeout {
            let mut all_ready = true;
            let mut all_healthy = true;

            // Проверяем готовность и здоровье всех координаторов параллельно
            let mut health_tasks = Vec::new();

            for (name, coordinator) in coordinators {
                let coordinator = Arc::clone(coordinator);
                let name = name.clone();
                let components: Arc<RwLock<HashMap<String, ComponentLifecycleState>>> =
                    Arc::clone(&self.components);

                let task = tokio::spawn(async move {
                    let check_start = Instant::now();
                    let ready = coordinator.is_ready().await;
                    let healthy = match coordinator.health_check().await {
                        Ok(_) => true,
                        Err(_) => false,
                    };

                    // Обновляем состояние компонента
                    {
                        let mut comp_states = components.write().await;
                        if let Some(state) = comp_states.get_mut(&name) {
                            state.ready = ready;
                            state.healthy = healthy;
                            state.last_health_check = Some(check_start);
                        }
                    }

                    (name, ready, healthy)
                });

                health_tasks.push(task);
            }

            // Собираем результаты проверок
            for task in health_tasks {
                let (name, ready, healthy) = task.await?;
                if !ready {
                    debug!("Компонент {} не готов", name);
                    all_ready = false;
                }
                if !healthy {
                    debug!("Компонент {} нездоров", name);
                    all_healthy = false;
                }
            }

            if all_ready && all_healthy {
                info!("✅ Все компоненты готовы и здоровы");
                return Ok(());
            }

            debug!(
                "⏳ Ожидание готовности компонентов... (ready: {}, healthy: {})",
                all_ready, all_healthy
            );
            sleep(self.config.readiness_check_interval).await;
        }

        // Собираем информацию о проблемных компонентах
        let component_states = self.components.read().await;
        let not_ready: Vec<_> = component_states
            .iter()
            .filter(|(_, state)| !state.ready)
            .map(|(name, _)| name.clone())
            .collect();
        let not_healthy: Vec<_> = component_states
            .iter()
            .filter(|(_, state)| !state.healthy)
            .map(|(name, _)| name.clone())
            .collect();

        Err(anyhow::anyhow!(
            "Не все компоненты готовы после таймаута. Не готовы: {:?}, Нездоровы: {:?}",
            not_ready,
            not_healthy
        ))
    }

    /// Проверить готовность всех компонентов
    pub async fn all_components_ready<T: Coordinator + Send + Sync + 'static>(
        &self,
        coordinators: &HashMap<String, Arc<T>>,
    ) -> bool {
        for (name, coordinator) in coordinators {
            if !coordinator.is_ready().await {
                debug!("Компонент {} не готов", name);
                return false;
            }
        }
        true
    }

    /// Graceful shutdown всех компонентов
    pub async fn shutdown_production<T: Coordinator + Send + Sync + 'static>(
        &self,
        coordinators: HashMap<String, Arc<T>>,
    ) -> Result<()> {
        info!("🛡️ Начало production graceful shutdown");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::ShuttingDown;
        }

        // Останавливаем в обратном порядке приоритета
        let shutdown_order = vec![
            "backup",
            "promotion",
            "search",
            "embedding",
            "health",
            "resources",
        ];

        for component_name in shutdown_order {
            if let Some(coordinator) = coordinators.get(component_name) {
                let shutdown_result =
                    timeout(self.config.shutdown_timeout, coordinator.shutdown()).await;

                match shutdown_result {
                    Ok(Ok(())) => {
                        info!("✅ {} coordinator успешно остановлен", component_name);
                    }
                    Ok(Err(e)) => {
                        warn!(
                            "⚠️ Ошибка при остановке {} coordinator: {}",
                            component_name, e
                        );
                    }
                    Err(_) => {
                        error!("❌ Timeout при остановке {} coordinator", component_name);
                    }
                }

                // Обновляем состояние компонента
                {
                    let mut comp_states = self.components.write().await;
                    if let Some(state) = comp_states.get_mut(component_name) {
                        state.ready = false;
                        state.healthy = false;
                    }
                }
            }
        }

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Stopped;
        }

        info!(
            "🏁 Graceful shutdown завершен за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    /// Аварийное завершение системы
    pub async fn emergency_shutdown<T: Coordinator + Send + Sync + 'static>(
        &self,
        coordinators: HashMap<String, Arc<T>>,
    ) -> Result<()> {
        error!("🔴 EMERGENCY SHUTDOWN запущен!");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::ShuttingDown;
        }

        // Параллельное завершение всех координаторов с короткими timeout'ами
        let mut shutdown_tasks = Vec::new();

        for (name, coordinator) in coordinators {
            let coordinator = Arc::clone(&coordinator);
            let name = name.clone();

            let task = tokio::spawn(async move {
                let result = timeout(Duration::from_secs(5), coordinator.shutdown()).await;
                (name, result)
            });

            shutdown_tasks.push(task);
        }

        // Ждем завершения всех задач
        for task in shutdown_tasks {
            let (name, result) = task.await?;
            match result {
                Ok(Ok(())) => info!("✅ Emergency: {} остановлен", name),
                Ok(Err(e)) => warn!("⚠️ Emergency: {} ошибка: {}", name, e),
                Err(_) => error!("❌ Emergency: {} timeout", name),
            }
        }

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Stopped;
        }

        error!("🚑 EMERGENCY SHUTDOWN завершен");
        Ok(())
    }

    /// Получить подробную статистику жизненного цикла
    pub async fn get_lifecycle_stats(&self) -> String {
        let current_phase = self.current_phase.read().await;
        let component_states = self.components.read().await;

        let mut stats = String::new();
        stats.push_str("=== Lifecycle Manager Statistics ===\n\n");

        stats.push_str(&format!("Current Phase: {:?}\n", *current_phase));
        stats.push_str(&format!("Uptime: {:?}\n", self.start_time.elapsed()));
        stats.push_str(&format!("Total Components: {}\n\n", component_states.len()));

        stats.push_str("Component States:\n");
        for (name, state) in component_states.iter() {
            let status_icon = if state.ready && state.healthy {
                "✅"
            } else if state.initialized {
                "🟡"
            } else {
                "❌"
            };

            stats.push_str(&format!(
                "├─ {} {}: Phase={:?}, Init={}, Ready={}, Healthy={}\n",
                status_icon, name, state.phase, state.initialized, state.ready, state.healthy
            ));

            if let Some(init_time) = state.initialization_time {
                stats.push_str(&format!("   ├─ Init time: {:?}\n", init_time));
            }

            if let Some(last_health) = state.last_health_check {
                stats.push_str(&format!(
                    "   ├─ Last health check: {:?} ago\n",
                    last_health.elapsed()
                ));
            }

            if let Some(ref error) = state.error_message {
                stats.push_str(&format!("   └─ Error: {}\n", error));
            }
        }

        stats
    }

    /// Проверить можно ли принимать запросы
    pub async fn is_accepting_requests(&self) -> bool {
        let phase = self.current_phase.read().await;
        matches!(*phase, LifecyclePhase::Ready)
    }

    /// Получить время работы системы
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl LifecycleManager {
    /// Выполнить фазу инициализации с заданными координаторами
    async fn execute_phase_with_coordinators(
        &self,
        phase: LifecyclePhase,
        coordinators: Vec<Arc<dyn crate::orchestration::traits::Coordinator>>,
        phase_timeout: Duration,
    ) -> Result<()> {
        info!(
            "📊 Phase: {:?} - инициализация {} координаторов",
            phase,
            coordinators.len()
        );

        // Обновляем фазу
        {
            let mut current_phase = self.current_phase.write().await;
            *current_phase = phase.clone();
        }

        // Запускаем инициализацию координаторов параллельно
        let mut initialization_tasks = Vec::new();

        for (index, coordinator) in coordinators.iter().enumerate() {
            let coordinator = Arc::clone(coordinator);
            let components: Arc<RwLock<HashMap<String, ComponentLifecycleState>>> =
                Arc::clone(&self.components);
            let coord_name = format!("coordinator_{}", index);
            let phase_clone = phase.clone();

            let task = tokio::spawn(async move {
                let init_start = std::time::Instant::now();

                // Обновляем состояние компонента - начало инициализации
                {
                    let mut comp_states = components.write().await;
                    if let Some(state) = comp_states.get_mut(&coord_name) {
                        state.phase = phase_clone.clone();
                    }
                }

                // Выполняем инициализацию
                let result = coordinator.initialize().await;
                let init_duration = init_start.elapsed();

                // Обновляем состояние компонента - результат инициализации
                {
                    let mut comp_states = components.write().await;
                    if let Some(state) = comp_states.get_mut(&coord_name) {
                        state.initialization_time = Some(init_duration);
                        match &result {
                            Ok(()) => {
                                state.initialized = true;
                                state.error_message = None;
                            }
                            Err(e) => {
                                state.initialized = false;
                                state.error_message = Some(e.to_string());
                            }
                        }
                    }
                }

                (coord_name, result)
            });

            initialization_tasks.push(task);
        }

        // Ждем завершения всех инициализаций с timeout
        let results = tokio::time::timeout(phase_timeout, async {
            let mut results = Vec::new();
            for task in initialization_tasks {
                results.push(
                    task.await
                        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?,
                );
            }
            Ok::<Vec<(String, Result<()>)>, anyhow::Error>(results)
        })
        .await
        .map_err(|_| anyhow::anyhow!("Phase {:?} timeout after {:?}", phase, phase_timeout))??;

        // Анализируем результаты
        let mut failed_components = Vec::new();
        for (name, result) in results {
            match result {
                Ok(()) => {
                    info!("✅ {} инициализирован успешно", name);
                }
                Err(e) => {
                    error!("❌ Ошибка инициализации {}: {}", name, e);
                    failed_components.push(name);
                }
            }
        }

        if !failed_components.is_empty() {
            return Err(anyhow::anyhow!(
                "Не удалось инициализировать компоненты в фазе {:?}: {:?}",
                phase,
                failed_components
            ));
        }

        info!("✅ Phase {:?} завершена успешно", phase);
        Ok(())
    }

    /// Проверить готовность и здоровье всех компонентов с новым CoordinatorRegistry
    pub async fn verify_all_components_health_with_registry(
        &self,
        registry: &crate::orchestration::coordinator_registry::CoordinatorRegistry,
    ) -> Result<()> {
        info!("🏥 Phase: Health Verification - проверка готовности всех координаторов из реестра");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::HealthVerification;
        }

        let verification_start = std::time::Instant::now();
        let verification_timeout = self.config.health_verification_timeout;

        let all_coordinators = registry.get_all_coordinators();

        while verification_start.elapsed() < verification_timeout {
            let mut all_ready = true;
            let mut all_healthy = true;

            // Проверяем готовность и здоровье всех координаторов параллельно
            let mut health_tasks = Vec::new();

            for (name, coordinator) in &all_coordinators {
                let coordinator = Arc::clone(coordinator);
                let name = name.clone();
                let components: Arc<RwLock<HashMap<String, ComponentLifecycleState>>> =
                    Arc::clone(&self.components);

                let task = tokio::spawn(async move {
                    let check_start = std::time::Instant::now();
                    let ready = coordinator.is_ready().await;
                    let healthy = match coordinator.health_check().await {
                        Ok(_) => true,
                        Err(_) => false,
                    };

                    // Обновляем состояние компонента
                    {
                        let mut comp_states = components.write().await;
                        if let Some(state) = comp_states.get_mut(&name) {
                            state.ready = ready;
                            state.healthy = healthy;
                            state.last_health_check = Some(check_start);
                        }
                    }

                    (name, ready, healthy)
                });

                health_tasks.push(task);
            }

            // Собираем результаты проверок
            for task in health_tasks {
                let (name, ready, healthy) = task.await?;
                if !ready {
                    debug!("Координатор {} не готов", name);
                    all_ready = false;
                }
                if !healthy {
                    debug!("Координатор {} нездоров", name);
                    all_healthy = false;
                }
            }

            if all_ready && all_healthy {
                info!("✅ Все координаторы готовы и здоровы");
                return Ok(());
            }

            debug!(
                "⏳ Ожидание готовности координаторов... (ready: {}, healthy: {})",
                all_ready, all_healthy
            );
            tokio::time::sleep(self.config.readiness_check_interval).await;
        }

        // Собираем информацию о проблемных координаторах
        let component_states = self.components.read().await;
        let not_ready: Vec<_> = component_states
            .iter()
            .filter(|(_, state)| !state.ready)
            .map(|(name, _)| name.clone())
            .collect();
        let not_healthy: Vec<_> = component_states
            .iter()
            .filter(|(_, state)| !state.healthy)
            .map(|(name, _)| name.clone())
            .collect();

        Err(anyhow::anyhow!(
            "Не все компоненты готовы после таймаута. Не готовы: {:?}, Нездоровы: {:?}",
            not_ready,
            not_healthy
        ))
    }

    /// Graceful shutdown с CoordinatorRegistry
    pub async fn shutdown_with_registry(
        &self,
        registry: &crate::orchestration::coordinator_registry::CoordinatorRegistry,
    ) -> Result<()> {
        info!("🛡️ Начало production graceful shutdown с CoordinatorRegistry");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::ShuttingDown;
        }

        // Получаем все координаторы и останавливаем в обратном порядке инициализации
        let init_order = registry.get_initialization_order();
        let mut shutdown_order = init_order;
        shutdown_order.reverse();

        for (name, coordinator) in shutdown_order {
            let shutdown_result =
                tokio::time::timeout(self.config.shutdown_timeout, coordinator.shutdown()).await;

            match shutdown_result {
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

            // Обновляем состояние компонента
            {
                let mut comp_states = self.components.write().await;
                if let Some(state) = comp_states.get_mut(&name) {
                    state.ready = false;
                    state.healthy = false;
                }
            }
        }

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Stopped;
        }

        info!(
            "🏁 Graceful shutdown с CoordinatorRegistry завершен за {:?}",
            self.start_time.elapsed()
        );
        Ok(())
    }

    /// Аварийное завершение с CoordinatorRegistry
    pub async fn emergency_shutdown_with_registry(
        &self,
        registry: &crate::orchestration::coordinator_registry::CoordinatorRegistry,
    ) -> Result<()> {
        error!("🔴 EMERGENCY SHUTDOWN с CoordinatorRegistry запущен!");

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::ShuttingDown;
        }

        // Параллельное завершение всех координаторов с короткими timeout'ами
        let all_coordinators = registry.get_all_coordinators();
        let mut shutdown_tasks = Vec::new();

        for (name, coordinator) in all_coordinators {
            let coordinator = Arc::clone(&coordinator);
            let name = name.clone();

            let task = tokio::spawn(async move {
                let result =
                    tokio::time::timeout(Duration::from_secs(5), coordinator.shutdown()).await;
                (name, result)
            });

            shutdown_tasks.push(task);
        }

        // Ждем завершения всех задач
        for task in shutdown_tasks {
            let (name, result) = task.await?;
            match result {
                Ok(Ok(())) => info!("✅ Emergency: {} остановлен", name),
                Ok(Err(e)) => warn!("⚠️ Emergency: {} ошибка: {}", name, e),
                Err(_) => error!("❌ Emergency: {} timeout", name),
            }
        }

        {
            let mut phase = self.current_phase.write().await;
            *phase = LifecyclePhase::Stopped;
        }

        error!("🚑 EMERGENCY SHUTDOWN с CoordinatorRegistry завершен");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockCoordinator {
        name: String,
        initialized: Arc<AtomicBool>,
        ready: Arc<AtomicBool>,
        healthy: Arc<AtomicBool>,
    }

    impl MockCoordinator {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                initialized: Arc::new(AtomicBool::new(false)),
                ready: Arc::new(AtomicBool::new(false)),
                healthy: Arc::new(AtomicBool::new(true)),
            }
        }

        fn set_ready(&self, ready: bool) {
            self.ready.store(ready, Ordering::Relaxed);
        }

        fn set_healthy(&self, healthy: bool) {
            self.healthy.store(healthy, Ordering::Relaxed);
        }
    }

    #[async_trait]
    impl Coordinator for MockCoordinator {
        async fn initialize(&self) -> Result<()> {
            self.initialized.store(true, Ordering::Relaxed);
            self.ready.store(true, Ordering::Relaxed);
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        async fn health_check(&self) -> Result<()> {
            if self.healthy.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Mock coordinator is unhealthy"))
            }
        }

        async fn shutdown(&self) -> Result<()> {
            self.ready.store(false, Ordering::Relaxed);
            self.initialized.store(false, Ordering::Relaxed);
            Ok(())
        }

        async fn metrics(&self) -> serde_json::Value {
            serde_json::json!({
                "name": self.name,
                "initialized": self.initialized.load(Ordering::Relaxed),
                "ready": self.ready.load(Ordering::Relaxed),
                "healthy": self.healthy.load(Ordering::Relaxed)
            })
        }
    }

    #[tokio::test]
    async fn test_lifecycle_manager_basic_functionality() {
        let config = LifecycleConfig {
            critical_infrastructure_timeout: Duration::from_millis(100),
            core_services_timeout: Duration::from_millis(100),
            background_services_timeout: Duration::from_millis(100),
            health_verification_timeout: Duration::from_millis(500),
            ..Default::default()
        };

        let manager = LifecycleManager::new(config);

        // Регистрируем компоненты
        manager.register_component("resources".to_string()).await;
        manager.register_component("health".to_string()).await;
        manager.register_component("embedding".to_string()).await;
        manager.register_component("search".to_string()).await;

        // Создаем mock координаторы
        let mut coordinators: HashMap<String, Arc<MockCoordinator>> = HashMap::new();
        coordinators.insert(
            "resources".to_string(),
            Arc::new(MockCoordinator::new("resources")),
        );
        coordinators.insert(
            "health".to_string(),
            Arc::new(MockCoordinator::new("health")),
        );
        coordinators.insert(
            "embedding".to_string(),
            Arc::new(MockCoordinator::new("embedding")),
        );
        coordinators.insert(
            "search".to_string(),
            Arc::new(MockCoordinator::new("search")),
        );

        // Проверяем начальное состояние
        assert_eq!(manager.current_phase().await, LifecyclePhase::Uninitialized);
        assert!(!manager.is_accepting_requests().await);

        // Запускаем инициализацию (только для критической инфраструктуры)
        let result = manager
            .execute_phase(
                LifecyclePhase::CriticalInfrastructure,
                &coordinators,
                &["resources", "health"],
                Duration::from_millis(200),
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(
            manager.current_phase().await,
            LifecyclePhase::CriticalInfrastructure
        );

        // Проверяем что компоненты инициализированы
        let states = manager.get_component_states().await;
        assert!(states["resources"].initialized);
        assert!(states["health"].initialized);
    }

    #[tokio::test]
    async fn test_lifecycle_manager_health_verification() {
        let config = LifecycleConfig {
            health_verification_timeout: Duration::from_millis(200),
            readiness_check_interval: Duration::from_millis(50),
            ..Default::default()
        };

        let manager = LifecycleManager::new(config);

        manager.register_component("test".to_string()).await;

        let mut coordinators: HashMap<String, Arc<MockCoordinator>> = HashMap::new();
        let mock_coord = Arc::new(MockCoordinator::new("test"));
        coordinators.insert("test".to_string(), mock_coord.clone());

        // Компонент не готов - должен быть timeout
        mock_coord.set_ready(false);
        let result = manager.verify_all_components_health(&coordinators).await;
        assert!(result.is_err());

        // Делаем компонент готовым
        mock_coord.set_ready(true);
        mock_coord.set_healthy(true);
        let result = manager.verify_all_components_health(&coordinators).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lifecycle_manager_shutdown() {
        let manager = LifecycleManager::new(LifecycleConfig::default());

        manager.register_component("test".to_string()).await;

        let mut coordinators: HashMap<String, Arc<MockCoordinator>> = HashMap::new();
        let mock_coord = Arc::new(MockCoordinator::new("test"));
        coordinators.insert("test".to_string(), mock_coord.clone());

        // Инициализируем компонент
        mock_coord
            .initialize()
            .await
            .expect("Async operation should succeed");
        assert!(mock_coord.is_ready().await);

        // Shutdown
        manager
            .shutdown_production(coordinators)
            .await
            .expect("Async operation should succeed");
        assert_eq!(manager.current_phase().await, LifecyclePhase::Stopped);

        // Проверяем что компонент остановлен
        assert!(!mock_coord.is_ready().await);
    }

    #[tokio::test]
    async fn test_lifecycle_manager_stats() {
        let manager = LifecycleManager::new(LifecycleConfig::default());

        manager
            .register_component("test_component".to_string())
            .await;

        let stats = manager.get_lifecycle_stats().await;
        assert!(stats.contains("Lifecycle Manager Statistics"));
        assert!(stats.contains("test_component"));
        assert!(stats.contains("Uninitialized"));
    }
}
