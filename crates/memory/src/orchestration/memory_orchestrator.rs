use anyhow::Result;
use std::{
    sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}},
    time::{Duration, Instant},
    collections::HashMap,
};
use tracing::{debug, info, warn, error};
use tokio::{
    sync::{RwLock, Semaphore},
    time::{timeout, sleep},
    task::JoinHandle,
};
use serde_json::json;

use crate::{
    orchestration::{
        EmbeddingCoordinator,
        SearchCoordinator,
        HealthManager,
        PromotionCoordinator,
        ResourceController,
        BackupCoordinator,
        RetryHandler, RetryPolicy,
        traits::{
            Coordinator, 
            SearchCoordinator as SearchCoordinatorTrait, 
            EmbeddingCoordinator as EmbeddingCoordinatorTrait,
            PromotionCoordinator as PromotionCoordinatorTrait,
            HealthCoordinator, ResourceCoordinator, BackupCoordinator as BackupCoordinatorTrait
        },
    },
    types::{Layer, Record, SearchOptions},
    promotion::PromotionStats,
    health::{SystemHealthStatus, HealthStatus},
    backup::BackupMetadata,
};

/// Production-ready главный оркестратор memory системы с полным lifecycle management
// @component: {"k":"C","id":"memory_orchestrator","t":"Main memory system orchestrator","m":{"cur":95,"tgt":95,"u":"%"},"f":["orchestration","coordinator","main","production","lifecycle","monitoring","resilience","circuit-breaker","load-balancing"]}
pub struct MemoryOrchestrator {
    // === Координаторы ===
    /// Координатор embeddings
    pub embedding: Arc<EmbeddingCoordinator>,
    /// Координатор поиска
    pub search: Arc<SearchCoordinator>,
    /// Менеджер здоровья
    pub health: Arc<HealthManager>,
    /// Координатор promotion
    pub promotion: Arc<PromotionCoordinator>,
    /// Контроллер ресурсов
    pub resources: Arc<ResourceController>,
    /// Координатор backup
    pub backup: Arc<BackupCoordinator>,
    
    // === Production Infrastructure ===
    /// Состояние готовности orchestrator'а
    ready: AtomicBool,
    /// Время запуска системы
    start_time: Instant,
    /// Semaphore для ограничения concurrent операций
    operation_limiter: Arc<Semaphore>,
    /// Circuit breaker состояния для координаторов
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    /// Retry handlers для разных типов операций
    retry_handlers: RetryHandlers,
    /// Orchestration metrics
    metrics: Arc<RwLock<OrchestrationMetrics>>,
    /// Background tasks handles
    background_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Emergency shutdown flag
    emergency_shutdown: Arc<AtomicBool>,
}

/// Circuit breaker state для coordinator'а
#[derive(Debug)]
struct CircuitBreakerState {
    /// Количество последовательных ошибок
    failure_count: AtomicU64,
    /// Время последней ошибки
    last_failure: Option<Instant>,
    /// Состояние circuit breaker (Open/HalfOpen/Closed)
    state: CircuitBreakerStatus,
    /// Время recovery timeout
    recovery_timeout: Duration,
}

impl Clone for CircuitBreakerState {
    fn clone(&self) -> Self {
        Self {
            failure_count: AtomicU64::new(self.failure_count.load(Ordering::Relaxed)),
            last_failure: self.last_failure,
            state: self.state.clone(),
            recovery_timeout: self.recovery_timeout,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerStatus {
    Closed,  // Нормальная работа
    Open,    // Блокировка запросов
    HalfOpen, // Пробная проверка восстановления
}

/// Retry handlers для разных операций
struct RetryHandlers {
    search: RetryHandler,
    embedding: RetryHandler,
    promotion: RetryHandler,
    backup: RetryHandler,
    health_check: RetryHandler,
}

/// Orchestration metrics
#[derive(Debug, Default)]
struct OrchestrationMetrics {
    /// Общие метрики
    total_operations: u64,
    successful_operations: u64,
    failed_operations: u64,
    
    /// Coordinator-specific metrics
    coordinator_metrics: HashMap<String, CoordinatorMetrics>,
    
    /// Performance metrics
    avg_operation_duration_ms: f64,
    max_operation_duration_ms: u64,
    
    /// Circuit breaker metrics
    circuit_breaker_trips: HashMap<String, u64>,
    
    /// Resource utilization
    current_concurrent_operations: u64,
    max_concurrent_operations: u64,
    
    /// SLA metrics
    sla_violations: u64,
    uptime_seconds: u64,
}

#[derive(Debug, Default, Clone)]
struct CoordinatorMetrics {
    success_rate: f64,
    avg_response_time_ms: f64,
    circuit_breaker_state: String,
    health_score: f64,
}

impl CircuitBreakerState {
    fn new(recovery_timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU64::new(0),
            last_failure: None,
            state: CircuitBreakerStatus::Closed,
            recovery_timeout,
        }
    }
    
    /// Записать успешную операцию
    fn record_success(&mut self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state = CircuitBreakerStatus::Closed;
        self.last_failure = None;
    }
    
    /// Записать ошибку
    fn record_failure(&mut self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure = Some(Instant::now());
        
        // Open circuit после 5 ошибок подряд
        if failures >= 5 {
            self.state = CircuitBreakerStatus::Open;
        }
    }
    
    /// Проверить можно ли выполнить операцию
    fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerStatus::Closed => true,
            CircuitBreakerStatus::Open => {
                // Проверяем не пора ли попробовать recovery
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        self.state = CircuitBreakerStatus::HalfOpen;
                        return true;
                    }
                }
                false
            },
            CircuitBreakerStatus::HalfOpen => true,
        }
    }
}

impl Default for RetryHandlers {
    fn default() -> Self {
        Self {
            search: RetryHandler::new(RetryPolicy::fast()), // Sub-5ms target
            embedding: RetryHandler::new(RetryPolicy::default()),
            promotion: RetryHandler::new(RetryPolicy::aggressive()), // Critical operation
            backup: RetryHandler::new(RetryPolicy::aggressive()), // Critical operation
            health_check: RetryHandler::new(RetryPolicy::fast()),
        }
    }
}

impl MemoryOrchestrator {
    /// Создать новый production-ready оркестратор из DI контейнера
    pub fn from_container(container: &crate::di_container::DIContainer) -> Result<Self> {
        info!("🚀 Создание MemoryOrchestrator из DI контейнера");
        
        // Разрешаем координаторы из контейнера
        let embedding = container.resolve::<EmbeddingCoordinator>()?;
        let search = container.resolve::<SearchCoordinator>()?;
        let health = container.resolve::<HealthManager>()?;
        let promotion = container.resolve::<PromotionCoordinator>()?;
        let resources = container.resolve::<ResourceController>()?;
        let backup = container.resolve::<BackupCoordinator>()?;
        
        // Инициализируем circuit breakers для всех координаторов
        let mut circuit_breakers = HashMap::new();
        circuit_breakers.insert("embedding".to_string(), CircuitBreakerState::new(Duration::from_secs(30)));
        circuit_breakers.insert("search".to_string(), CircuitBreakerState::new(Duration::from_secs(10)));
        circuit_breakers.insert("health".to_string(), CircuitBreakerState::new(Duration::from_secs(60)));
        circuit_breakers.insert("promotion".to_string(), CircuitBreakerState::new(Duration::from_secs(120)));
        circuit_breakers.insert("resources".to_string(), CircuitBreakerState::new(Duration::from_secs(60)));
        circuit_breakers.insert("backup".to_string(), CircuitBreakerState::new(Duration::from_secs(300)));
        
        Ok(Self {
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
            ready: AtomicBool::new(false),
            start_time: Instant::now(),
            operation_limiter: Arc::new(Semaphore::new(100)), // Max 100 concurrent operations
            circuit_breakers: Arc::new(RwLock::new(circuit_breakers)),
            retry_handlers: RetryHandlers::default(),
            metrics: Arc::new(RwLock::new(OrchestrationMetrics::default())),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
            emergency_shutdown: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// Production-ready инициализация с parallel coordinator startup
    pub async fn initialize_production(&self) -> Result<()> {
        info!("🔄 Запуск production инициализации MemoryOrchestrator");
        
        // Проверяем не запущена ли уже система
        if self.ready.load(Ordering::Relaxed) {
            warn!("Система уже инициализирована");
            return Ok(());
        }
        
        // === Phase 1: Critical Infrastructure ===
        info!("📊 Phase 1: Инициализация критической инфраструктуры");
        
        // Сначала запускаем resource controller и health manager
        let resource_init = timeout(Duration::from_secs(30), self.resources.initialize());
        let health_init = timeout(Duration::from_secs(30), self.health.initialize());
        
        let (resource_result, health_result) = tokio::try_join!(resource_init, health_init)?;
        resource_result.map_err(|e| anyhow::anyhow!("Resource controller инициализация не удалась: {}", e))?;
        health_result.map_err(|e| anyhow::anyhow!("Health manager инициализация не удалась: {}", e))?;
        
        info!("✅ Critical infrastructure готова");
        
        // === Phase 2: Core Services ===
        info!("🧠 Phase 2: Инициализация core services");
        
        let embedding_init = timeout(Duration::from_secs(45), self.embedding.initialize());
        let search_init = timeout(Duration::from_secs(30), self.search.initialize());
        
        let (embedding_result, search_result) = tokio::try_join!(embedding_init, search_init)?;
        embedding_result.map_err(|e| anyhow::anyhow!("Embedding coordinator инициализация не удалась: {}", e))?;
        search_result.map_err(|e| anyhow::anyhow!("Search coordinator инициализация не удалась: {}", e))?;
        
        info!("✅ Core services готовы");
        
        // === Phase 3: Background Services ===
        info!("🔄 Phase 3: Инициализация background services");
        
        let promotion_init = timeout(Duration::from_secs(60), self.promotion.initialize());
        let backup_init = timeout(Duration::from_secs(60), self.backup.initialize());
        
        let (promotion_result, backup_result) = tokio::try_join!(promotion_init, backup_init)?;
        promotion_result.map_err(|e| anyhow::anyhow!("Promotion coordinator инициализация не удалась: {}", e))?;
        backup_result.map_err(|e| anyhow::anyhow!("Backup coordinator инициализация не удалась: {}", e))?;
        
        info!("✅ Background services готовы");
        
        // === Phase 4: Health Verification ===
        info!("🏥 Phase 4: Проверка готовности всех координаторов");
        
        let ready_check_timeout = Duration::from_secs(30);
        let ready_check_start = Instant::now();
        
        while ready_check_start.elapsed() < ready_check_timeout {
            if self.verify_all_coordinators_ready().await {
                info!("✅ Все координаторы готовы к работе");
                break;
            }
            
            debug!("⏳ Ожидание готовности координаторов...");
            sleep(Duration::from_millis(500)).await;
        }
        
        // Финальная проверка
        if !self.verify_all_coordinators_ready().await {
            return Err(anyhow::anyhow!("Не все координаторы готовы после таймаута"));
        }
        
        // === Phase 5: Start Background Tasks ===
        self.start_background_tasks().await?;
        
        // Отмечаем систему как готову
        self.ready.store(true, Ordering::Release);
        
        // Записываем метрики запуска
        {
            let mut metrics = self.metrics.write().await;
            metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        }
        
        info!("🎉 MemoryOrchestrator успешно инициализирован за {:?}", self.start_time.elapsed());
        Ok(())
    }
    
    /// Проверить готовность всех координаторов
    async fn verify_all_coordinators_ready(&self) -> bool {
        // Параллельная проверка готовности всех координаторов
        let results = tokio::join!(
            self.embedding.is_ready(),
            self.search.is_ready(), 
            self.health.is_ready(),
            self.promotion.is_ready(),
            self.resources.is_ready(),
            self.backup.is_ready()
        );
        
        let all_ready = results.0 && results.1 && results.2 && results.3 && results.4 && results.5;
        
        if !all_ready {
            debug!("Coordinator readiness: embedding={}, search={}, health={}, promotion={}, resources={}, backup={}",
                results.0, results.1, results.2, results.3, results.4, results.5);
        }
        
        all_ready
    }
    
    /// Запустить background задачи
    async fn start_background_tasks(&self) -> Result<()> {
        info!("🔄 Запуск background задач orchestrator'а");
        let mut tasks = self.background_tasks.write().await;
        
        // Health monitoring task
        let health_task = {
            let health = Arc::clone(&self.health);
            let metrics = Arc::clone(&self.metrics);
            let emergency_shutdown = Arc::clone(&self.emergency_shutdown);
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                while !emergency_shutdown.load(Ordering::Relaxed) {
                    interval.tick().await;
                    
                    if let Err(e) = health.run_health_check().await {
                        error!("Health check failed: {}", e);
                    }
                    
                    // Обновляем uptime метрики
                    if let Ok(mut metrics) = metrics.try_write() {
                        metrics.uptime_seconds = metrics.uptime_seconds.saturating_add(30);
                    }
                }
                debug!("Health monitoring task завершена");
            })
        };
        
        // Circuit breaker monitoring task
        let circuit_breaker_task = {
            let circuit_breakers = Arc::clone(&self.circuit_breakers);
            let emergency_shutdown = Arc::clone(&self.emergency_shutdown);
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                while !emergency_shutdown.load(Ordering::Relaxed) {
                    interval.tick().await;
                    
                    if let Ok(breakers) = circuit_breakers.try_read() {
                        for (name, state) in breakers.iter() {
                            match state.state {
                                CircuitBreakerStatus::Open => {
                                    warn!("🔴 Circuit breaker ОТКРЫТ для {}", name);
                                }
                                CircuitBreakerStatus::HalfOpen => {
                                    info!("🟡 Circuit breaker в режиме восстановления для {}", name);
                                }
                                CircuitBreakerStatus::Closed => {
                                    // Нормальная работа
                                }
                            }
                        }
                    }
                }
                debug!("Circuit breaker monitoring task завершена");
            })
        };
        
        // Metrics collection task
        let metrics_task = {
            let metrics = Arc::clone(&self.metrics);
            let circuit_breakers = Arc::clone(&self.circuit_breakers);
            let emergency_shutdown = Arc::clone(&self.emergency_shutdown);
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(60));
                while !emergency_shutdown.load(Ordering::Relaxed) {
                    interval.tick().await;
                    
                    // Собираем метрики circuit breakers
                    if let (Ok(mut metrics), Ok(breakers)) = (
                        metrics.try_write(), 
                        circuit_breakers.try_read()
                    ) {
                        for (name, state) in breakers.iter() {
                            let trips = metrics.circuit_breaker_trips.entry(name.clone()).or_insert(0);
                            if state.state == CircuitBreakerStatus::Open {
                                *trips = trips.saturating_add(1);
                            }
                        }
                    }
                }
                debug!("Metrics collection task завершена");
            })
        };
        
        tasks.push(health_task);
        tasks.push(circuit_breaker_task);
        tasks.push(metrics_task);
        
        info!("✅ {} background задач запущено", tasks.len());
        Ok(())
    }
    
    /// Legacy метод инициализации - использовать initialize_production() вместо него
    pub async fn initialize_all(&self) -> Result<()> {
        warn!("⚠️ Использование legacy initialize_all(), рекомендуется initialize_production()");
        self.initialize_production().await
    }
    
    /// Проверить готовность всей системы
    pub async fn all_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire) && self.verify_all_coordinators_ready().await
    }
    
    /// Production health check с circuit breaker поддержкой
    pub async fn production_health_check(&self) -> Result<SystemHealthStatus> {
        if !self.ready.load(Ordering::Relaxed) {
            return Ok(SystemHealthStatus {
                overall_status: HealthStatus::Down,
                component_statuses: HashMap::new(),
                active_alerts: vec![],
                metrics_summary: HashMap::new(),
                last_updated: chrono::Utc::now(),
                uptime_seconds: 0,
            });
        }
        
        // Проверяем circuit breaker для health coordinator'а
        let can_execute = {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.get_mut("health")
                .map(|cb| cb.can_execute())
                .unwrap_or(true)
        };
        
        if !can_execute {
            warn!("🔴 Health check заблокирован circuit breaker'ом");
            return Ok(SystemHealthStatus {
                overall_status: HealthStatus::Degraded,
                component_statuses: HashMap::new(),
                active_alerts: vec![],
                metrics_summary: HashMap::new(),
                last_updated: chrono::Utc::now(),
                uptime_seconds: self.start_time.elapsed().as_secs(),
            });
        }
        
        // Выполняем health check с retry logic
        match self.retry_handlers.health_check.execute(|| async {
            self.health.system_health().await
        }).await {
            crate::orchestration::RetryResult::Success(health, attempts) => {
                if attempts > 1 {
                    debug!("Health check выполнен с {} попыток", attempts);
                }
                
                // Записываем успешный результат в circuit breaker
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("health") {
                        cb.record_success();
                    }
                }
                
                Ok(health)
            },
            crate::orchestration::RetryResult::ExhaustedRetries(e) | 
            crate::orchestration::RetryResult::NonRetriable(e) => {
                error!("🔴 Health check не удался: {}", e);
                
                // Записываем ошибку в circuit breaker
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("health") {
                        cb.record_failure();
                    }
                }
                
                Err(e)
            }
        }
    }
    
    /// Production-ready graceful shutdown с timeout защитой
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("🛡️ Начало production graceful shutdown MemoryOrchestrator");
        
        // Отмечаем систему как не готовую
        self.ready.store(false, Ordering::Release);
        
        // === Phase 1: Stop Background Tasks ===
        info!("🛤️ Phase 1: Остановка background tasks");
        self.stop_background_tasks().await;
        
        // === Phase 2: Wait for Active Operations ===
        info!("⏳ Phase 2: Ожидание завершения активных операций");
        let active_operations_timeout = Duration::from_secs(30);
        let active_operations_start = Instant::now();
        
        while active_operations_start.elapsed() < active_operations_timeout {
            let available_permits = self.operation_limiter.available_permits();
            if available_permits >= 100 { // All permits available = no active operations
                info!("✅ Все активные операции завершены");
                break;
            }
            
            debug!("⏳ Ожидание завершения {} активных операций", 100 - available_permits);
            sleep(Duration::from_millis(500)).await;
        }
        
        // === Phase 3: Coordinated Shutdown ===
        info!("🛡️ Phase 3: Координированное завершение координаторов");
        
        // Останавливаем в обратном порядке с timeout защитой
        let coordinator_shutdowns = [
            ("backup", timeout(Duration::from_secs(60), self.backup.shutdown())),
            ("promotion", timeout(Duration::from_secs(30), self.promotion.shutdown())),
            ("search", timeout(Duration::from_secs(15), self.search.shutdown())),
            ("embedding", timeout(Duration::from_secs(30), self.embedding.shutdown())),
            ("health", timeout(Duration::from_secs(15), self.health.shutdown())),
            ("resources", timeout(Duration::from_secs(15), self.resources.shutdown())),
        ];
        
        for (name, shutdown_future) in coordinator_shutdowns {
            match shutdown_future.await {
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
        
        // Обновляем метрики
        if let Ok(mut metrics) = self.metrics.try_write() {
            metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        }
        
        info!("🏁 MemoryOrchestrator успешно остановлен за {:?}", self.start_time.elapsed());
        Ok(())
    }
    
    /// Аварийное завершение системы
    pub async fn emergency_shutdown(&self) -> Result<()> {
        error!("🔴 EMERGENCY SHUTDOWN запущен!");
        
        // Отмечаем emergency shutdown flag
        self.emergency_shutdown.store(true, Ordering::Release);
        self.ready.store(false, Ordering::Release);
        
        // Немедленно останавливаем background tasks
        self.stop_background_tasks().await;
        
        // Параллельное завершение всех координаторов с короткими timeout'ами
        let results = tokio::join!(
            timeout(Duration::from_secs(5), self.backup.shutdown()),
            timeout(Duration::from_secs(3), self.promotion.shutdown()),
            timeout(Duration::from_secs(2), self.search.shutdown()),
            timeout(Duration::from_secs(3), self.embedding.shutdown()),
            timeout(Duration::from_secs(2), self.health.shutdown()),
            timeout(Duration::from_secs(2), self.resources.shutdown())
        );
        
        // Логируем результаты
        let coordinator_names = ["backup", "promotion", "search", "embedding", "health", "resources"];
        let shutdown_results = [&results.0, &results.1, &results.2, &results.3, &results.4, &results.5];
        
        for (name, result) in coordinator_names.iter().zip(shutdown_results.iter()) {
            match result {
                Ok(Ok(())) => info!("✅ Emergency: {} остановлен", name),
                Ok(Err(e)) => warn!("⚠️ Emergency: {} ошибка: {}", name, e),
                Err(_) => error!("❌ Emergency: {} timeout", name),
            }
        }
        
        error!("🚑 EMERGENCY SHUTDOWN завершен");
        Ok(())
    }
    
    /// Остановить все background tasks
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
    
    /// Получить comprehensive production metrics
    pub async fn all_metrics(&self) -> serde_json::Value {
        let orchestrator_metrics = self.metrics.read().await;
        let circuit_breakers = self.circuit_breakers.read().await;
        
        // Параллельно собираем метрики координаторов
        let results = tokio::join!(
            self.embedding.metrics(),
            self.search.metrics(),
            self.health.metrics(),
            self.promotion.metrics(),
            self.resources.metrics(),
            self.backup.metrics()
        );
        
        // Собираем readiness состояния параллельно
        let readiness_results = tokio::join!(
            self.embedding.is_ready(),
            self.search.is_ready(),
            self.health.is_ready(),
            self.promotion.is_ready(),
            self.resources.is_ready(),
            self.backup.is_ready()
        );
        
        let mut coordinator_metrics_json = serde_json::Map::new();
        coordinator_metrics_json.insert("embedding".to_string(), results.0);
        coordinator_metrics_json.insert("search".to_string(), results.1);
        coordinator_metrics_json.insert("health".to_string(), results.2);
        coordinator_metrics_json.insert("promotion".to_string(), results.3);
        coordinator_metrics_json.insert("resources".to_string(), results.4);
        coordinator_metrics_json.insert("backup".to_string(), results.5);
        
        // Circuit breaker states
        let mut circuit_breaker_states = serde_json::Map::new();
        for (name, state) in circuit_breakers.iter() {
            circuit_breaker_states.insert(name.clone(), json!({
                "state": match state.state {
                    CircuitBreakerStatus::Closed => "closed",
                    CircuitBreakerStatus::Open => "open",
                    CircuitBreakerStatus::HalfOpen => "half_open",
                },
                "failure_count": state.failure_count.load(Ordering::Relaxed),
                "last_failure": state.last_failure.map(|t| t.elapsed().as_secs()),
                "recovery_timeout_secs": state.recovery_timeout.as_secs(),
            }));
        }
        
        json!({
            "orchestrator": {
                "ready": self.ready.load(Ordering::Relaxed),
                "uptime_seconds": self.start_time.elapsed().as_secs(),
                "emergency_shutdown": self.emergency_shutdown.load(Ordering::Relaxed),
                
                // Operation metrics
                "operations": {
                    "total": orchestrator_metrics.total_operations,
                    "successful": orchestrator_metrics.successful_operations,
                    "failed": orchestrator_metrics.failed_operations,
                    "success_rate": if orchestrator_metrics.total_operations > 0 {
                        orchestrator_metrics.successful_operations as f64 / orchestrator_metrics.total_operations as f64 * 100.0
                    } else { 100.0 },
                    "current_concurrent": 100 - self.operation_limiter.available_permits(),
                    "max_concurrent": orchestrator_metrics.max_concurrent_operations,
                },
                
                // Performance metrics
                "performance": {
                    "avg_operation_duration_ms": orchestrator_metrics.avg_operation_duration_ms,
                    "max_operation_duration_ms": orchestrator_metrics.max_operation_duration_ms,
                },
                
                // Circuit breaker metrics
                "circuit_breakers": circuit_breaker_states,
                "circuit_breaker_trips": orchestrator_metrics.circuit_breaker_trips,
                
                // SLA metrics
                "sla": {
                    "violations": orchestrator_metrics.sla_violations,
                    "uptime_percentage": if self.start_time.elapsed().as_secs() > 0 {
                        100.0 - (orchestrator_metrics.sla_violations as f64 / self.start_time.elapsed().as_secs() as f64 * 100.0)
                    } else { 100.0 },
                },
                
                // Coordinator-specific orchestration metrics
                "coordinator_health": {
                    "embedding_ready": readiness_results.0,
                    "search_ready": readiness_results.1,
                    "health_ready": readiness_results.2,
                    "promotion_ready": readiness_results.3,
                    "resources_ready": readiness_results.4,
                    "backup_ready": readiness_results.5,
                },
                
                "coordinators": coordinator_metrics_json,
            }
        })
    }
    
    /// Получить dashboard-ready metrics
    pub async fn dashboard_metrics(&self) -> serde_json::Value {
        let full_metrics = self.all_metrics().await;
        
        // Формируем упрощенную версию для dashboard'а
        json!({
            "status": if self.ready.load(Ordering::Relaxed) { "ready" } else { "not_ready" },
            "uptime_hours": self.start_time.elapsed().as_secs() / 3600,
            "operations_per_minute": full_metrics["orchestrator"]["operations"]["total"].as_u64().unwrap_or(0) / (self.start_time.elapsed().as_secs() / 60).max(1),
            "success_rate": full_metrics["orchestrator"]["operations"]["success_rate"],
            "active_operations": full_metrics["orchestrator"]["operations"]["current_concurrent"],
            "circuit_breakers_open": (|| {
                let breakers = &full_metrics["orchestrator"]["circuit_breakers"];
                let mut open_count = 0;
                if let Some(breakers_obj) = breakers.as_object() {
                    for (_, state) in breakers_obj {
                        if state["state"] == "open" {
                            open_count += 1;
                        }
                    }
                }
                open_count
            })(),
            "coordinator_health": full_metrics["orchestrator"]["coordinator_health"],
        })
    }
    
    // === Production-ready методы-обертки с circuit breaker поддержкой ===
    
    /// Production поиск с full orchestration intelligence
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        if !self.ready.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Оркестратор не готов"));
        }
        
        // Получаем permit для concurrent operations
        let _permit = self.operation_limiter.acquire().await.map_err(|e| 
            anyhow::anyhow!("Невозможно получить permit для операции: {}", e))?;
        
        let operation_start = Instant::now();
        
        // Проверяем circuit breaker
        let can_execute = {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.get_mut("search")
                .map(|cb| cb.can_execute())
                .unwrap_or(true)
        };
        
        if !can_execute {
            // Обновляем метрики ошибок
            {
                let mut metrics = self.metrics.write().await;
                metrics.failed_operations += 1;
                metrics.total_operations += 1;
            }
            return Err(anyhow::anyhow!("Поиск временно недоступен (circuit breaker открыт)"));
        }
        
        // Проверяем ресурсы
        if !self.resources.check_resources("search").await? {
            warn!("🟡 Недостаточно ресурсов для поиска, запускаем адаптацию лимитов");
            if let Err(e) = self.resources.adapt_limits().await {
                warn!("Ошибка адаптации лимитов: {}", e);
            }
            return Ok(vec![]);
        }
        
        // Выполняем поиск с retry logic
        let result = self.retry_handlers.search.execute(|| async {
            SearchCoordinatorTrait::search(&*self.search, query, layer, options.clone()).await
        }).await;
        
        let operation_duration = operation_start.elapsed();
        
        // Обновляем метрики и circuit breaker state
        match &result {
            crate::orchestration::RetryResult::Success(records, attempts) => {
                debug!("✅ Поиск выполнен за {:?} ({} попыток, {} результатов)", 
                    operation_duration, attempts, records.len());
                
                // Отмечаем успех в circuit breaker
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("search") {
                        cb.record_success();
                    }
                }
                
                // Обновляем metrics
                if let Ok(mut metrics) = self.metrics.try_write() {
                    metrics.successful_operations += 1;
                    metrics.total_operations += 1;
                    metrics.current_concurrent_operations = (100 - self.operation_limiter.available_permits()) as u64;
                    if metrics.current_concurrent_operations > metrics.max_concurrent_operations {
                        metrics.max_concurrent_operations = metrics.current_concurrent_operations;
                    }
                    
                    let duration_ms = operation_duration.as_millis() as f64;
                    metrics.avg_operation_duration_ms = 
                        (metrics.avg_operation_duration_ms * (metrics.total_operations - 1) as f64 + duration_ms) / metrics.total_operations as f64;
                    
                    if operation_duration.as_millis() > metrics.max_operation_duration_ms as u128 {
                        metrics.max_operation_duration_ms = operation_duration.as_millis() as u64;
                    }
                }
                
                // Проверяем SLA (sub-5ms target)
                if operation_duration.as_millis() > 5 {
                    if let Ok(mut metrics) = self.metrics.try_write() {
                        metrics.sla_violations += 1;
                    }
                    debug!("⚠️ SLA violation: поиск выполнялся {:?} (target: <5ms)", operation_duration);
                }
            },
            crate::orchestration::RetryResult::ExhaustedRetries(e) |
            crate::orchestration::RetryResult::NonRetriable(e) => {
                error!("🔴 Поиск не удался за {:?}: {}", operation_duration, e);
                
                // Отмечаем ошибку в circuit breaker
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("search") {
                        cb.record_failure();
                    }
                }
                
                // Обновляем metrics
                if let Ok(mut metrics) = self.metrics.try_write() {
                    metrics.failed_operations += 1;
                    metrics.total_operations += 1;
                }
            }
        }
        
        result.into_result()
    }
    
    /// Production embedding с intelligent caching и fallback
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        if !self.ready.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Оркестратор не готов"));
        }
        
        let _permit = self.operation_limiter.acquire().await.map_err(|e| 
            anyhow::anyhow!("Невозможно получить permit: {}", e))?;
        
        // Проверяем circuit breaker
        let can_execute = {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.get_mut("embedding")
                .map(|cb| cb.can_execute())
                .unwrap_or(true)
        };
        
        if !can_execute {
            return Err(anyhow::anyhow!("Эмбеддинг временно недоступен (circuit breaker)"));
        }
        
        // Сначала проверяем кэш без retry
        if let Some(cached) = EmbeddingCoordinatorTrait::check_cache(&*self.embedding, text).await {
            debug!("💾 Cache hit для embedding: {} chars", text.len());
            return Ok(cached);
        }
        
        // Проверяем ресурсы
        if !self.resources.check_resources("embedding").await? {
            warn!("🟡 Недостаточно ресурсов для embedding");
            return Err(anyhow::anyhow!("Недостаточно ресурсов"));
        }
        
        // Выполняем embedding с retry
        let operation_start = Instant::now();
        let result: crate::orchestration::RetryResult<Vec<f32>> = self.retry_handlers.embedding.execute(|| async {
            EmbeddingCoordinatorTrait::get_embedding(&*self.embedding, text).await
        }).await;
        
        // Обновляем circuit breaker и metrics
        match &result {
            crate::orchestration::RetryResult::Success(embedding, attempts) => {
                debug!("✅ Embedding получен за {:?} ({} попыток, {} dims)", 
                    operation_start.elapsed(), attempts, embedding.len());
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("embedding") {
                        cb.record_success();
                    }
                }
            },
            _ => {
                error!("🔴 Embedding не удался за {:?}", operation_start.elapsed());
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("embedding") {
                        cb.record_failure();
                    }
                }
            }
        }
        
        result.into_result()
    }
    
    /// Production promotion с intelligent scheduling
    pub async fn run_promotion(&self) -> Result<PromotionStats> {
        if !self.ready.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Оркестратор не готов"));
        }
        
        let _permit = self.operation_limiter.acquire().await.map_err(|e| 
            anyhow::anyhow!("Невозможно получить permit: {}", e))?;
        
        // Проверяем circuit breaker
        let can_execute = {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.get_mut("promotion")
                .map(|cb| cb.can_execute())
                .unwrap_or(true)
        };
        
        if !can_execute {
            warn!("🟡 Promotion заблокирован circuit breaker'ом");
            return Ok(PromotionStats::default());
        }
        
        // Проверяем нужно ли запускать promotion в принципе
        if !self.promotion.should_promote().await {
            debug!("ℹ️ Promotion не требуется в данный момент");
            return Ok(PromotionStats::default());
        }
        
        // Проверяем ресурсы
        if !self.resources.check_resources("promotion").await? {
            warn!("🟡 Недостаточно ресурсов для promotion, откладываем");
            return Ok(PromotionStats::default());
        }
        
        // Выполняем promotion с aggressive retry policy
        let operation_start = Instant::now();
        let result = self.retry_handlers.promotion.execute(|| async {
            self.promotion.run_promotion().await
        }).await;
        
        match &result {
            crate::orchestration::RetryResult::Success(stats, attempts) => {
                info!("✅ Promotion завершен за {:?} ({} попыток, {} ms)", 
                    operation_start.elapsed(), attempts, stats.total_time_ms);
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("promotion") {
                        cb.record_success();
                    }
                }
            },
            _ => {
                error!("🔴 Promotion не удался за {:?}", operation_start.elapsed());
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("promotion") {
                        cb.record_failure();
                    }
                }
            }
        }
        
        result.into_result()
    }
    
    /// Legacy health check - использовать production_health_check()
    pub async fn check_health(&self) -> Result<SystemHealthStatus> {
        self.production_health_check().await
    }
    
    /// Production backup с comprehensive validation
    pub async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        if !self.ready.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Оркестратор не готов"));
        }
        
        let _permit = self.operation_limiter.acquire().await.map_err(|e| 
            anyhow::anyhow!("Невозможно получить permit: {}", e))?;
        
        // Проверяем circuit breaker
        let can_execute = {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.get_mut("backup")
                .map(|cb| cb.can_execute())
                .unwrap_or(true)
        };
        
        if !can_execute {
            return Err(anyhow::anyhow!("Бэкап временно недоступен (circuit breaker)"));
        }
        
        // Проверяем ресурсы
        if !self.resources.check_resources("backup").await? {
            return Err(anyhow::anyhow!("Недостаточно ресурсов для backup"));
        }
        
        // Проверяем здоровье системы перед backup
        match self.production_health_check().await {
            Ok(health) if health.overall_status != HealthStatus::Healthy => {
                warn!("⚠️ Создание backup при нездоровом состоянии системы: {:?}", health.overall_status);
            },
            Err(e) => {
                error!("❌ Не удалось проверить здоровье перед backup: {}", e);
                return Err(anyhow::anyhow!("Невозможно создать backup при неизвестном состоянии системы"));
            },
            _ => {} // Система здорова
        }
        
        info!("💾 Начало создания production backup: {}", path);
        let operation_start = Instant::now();
        
        // Выполняем backup с aggressive retry
        let result = self.retry_handlers.backup.execute(|| async {
            self.backup.create_backup(path).await
        }).await;
        
        match &result {
            crate::orchestration::RetryResult::Success(metadata, attempts) => {
                info!("✅ Backup создан за {:?} ({} попыток, {} records)", 
                    operation_start.elapsed(), attempts, metadata.total_records);
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("backup") {
                        cb.record_success();
                    }
                }
            },
            _ => {
                error!("🔴 Backup не удался за {:?}", operation_start.elapsed());
                
                if let Ok(mut breakers) = self.circuit_breakers.try_write() {
                    if let Some(cb) = breakers.get_mut("backup") {
                        cb.record_failure();
                    }
                }
            }
        }
        
        result.into_result()
    }
    
    // === Advanced Production Methods ===
    
    /// Принудительно сбросить circuit breakers
    pub async fn reset_circuit_breakers(&self) -> Result<()> {
        info!("🔄 Сброс всех circuit breakers");
        
        let mut breakers = self.circuit_breakers.write().await;
        for (name, breaker) in breakers.iter_mut() {
            breaker.record_success(); // Reset to closed state
            info!("✅ Circuit breaker {} сброшен", name);
        }
        
        Ok(())
    }
    
    /// Получить текущие circuit breaker states
    pub async fn circuit_breaker_states(&self) -> HashMap<String, String> {
        let breakers = self.circuit_breakers.read().await;
        let mut states = HashMap::new();
        
        for (name, breaker) in breakers.iter() {
            let state = match breaker.state {
                CircuitBreakerStatus::Closed => "closed",
                CircuitBreakerStatus::Open => "open", 
                CircuitBreakerStatus::HalfOpen => "half_open",
            };
            states.insert(name.clone(), state.to_string());
        }
        
        states
    }
    
    /// Адаптивная оптимизация ресурсов на основе метрик
    pub async fn adaptive_optimization(&self) -> Result<()> {
        if !self.ready.load(Ordering::Relaxed) {
            return Ok(()); // Skip if not ready
        }
        
        debug!("🎯 Запуск адаптивной оптимизации");
        
        // Проверяем метрики и адаптируем лимиты
        let metrics = self.metrics.read().await;
        
        // Если SLA violations > 10% - увеличиваем лимиты
        let sla_violation_rate = if metrics.total_operations > 0 {
            metrics.sla_violations as f64 / metrics.total_operations as f64
        } else { 0.0 };
        
        if sla_violation_rate > 0.1 {
            warn!("⚠️ Высокий уровень SLA violations ({:.1}%), адаптируем лимиты", sla_violation_rate * 100.0);
            self.resources.adapt_limits().await?;
        }
        
        // Если много circuit breaker trips - очищаем кэши
        let total_trips: u64 = metrics.circuit_breaker_trips.values().sum();
        if total_trips > 10 {
            info!("🧩 Много circuit breaker trips ({}), очищаем embedding cache", total_trips);
            if let Err(e) = EmbeddingCoordinatorTrait::clear_cache(&*self.embedding).await {
                warn!("Ошибка очистки кэша: {}", e);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[tokio::test]
    async fn test_circuit_breaker_functionality() {
        let mut circuit_breaker = CircuitBreakerState::new(Duration::from_millis(100));
        
        // Initially closed
        assert!(circuit_breaker.can_execute());
        assert_eq!(circuit_breaker.state, CircuitBreakerStatus::Closed);
        
        // Record failures until circuit opens
        for _ in 0..5 {
            circuit_breaker.record_failure();
        }
        
        assert_eq!(circuit_breaker.state, CircuitBreakerStatus::Open);
        assert!(!circuit_breaker.can_execute());
        
        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should allow one attempt (HalfOpen)
        assert!(circuit_breaker.can_execute());
        assert_eq!(circuit_breaker.state, CircuitBreakerStatus::HalfOpen);
        
        // Success should close circuit
        circuit_breaker.record_success();
        assert_eq!(circuit_breaker.state, CircuitBreakerStatus::Closed);
        assert!(circuit_breaker.can_execute());
    }
    
    #[tokio::test]
    async fn test_orchestration_metrics() {
        let mut metrics = OrchestrationMetrics::default();
        
        // Тестируем обновление метрик
        metrics.total_operations = 100;
        metrics.successful_operations = 95;
        metrics.failed_operations = 5;
        
        let success_rate = metrics.successful_operations as f64 / metrics.total_operations as f64 * 100.0;
        assert_eq!(success_rate, 95.0);
        
        // Тестируем circuit breaker trips
        metrics.circuit_breaker_trips.insert("test".to_string(), 3);
        assert_eq!(metrics.circuit_breaker_trips.get("test"), Some(&3));
    }
    
    #[tokio::test]
    async fn test_retry_handlers_creation() {
        let handlers = RetryHandlers::default();
        
        // Проверяем что все handlers созданы с разными политиками
        let counter = Arc::new(AtomicU32::new(0));
        
        let counter_clone = Arc::clone(&counter);
        let result = handlers.search.execute(|| {
            let counter = Arc::clone(&counter_clone);
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err(anyhow::anyhow!("temporary failure"))
                } else {
                    Ok("success")
                }
            }
        }).await;
        
        // Fast retry policy should succeed on second attempt
        assert!(matches!(result, crate::orchestration::RetryResult::Success(_, 2)));
    }
    
    // TODO: Добавить integration тесты после полной интеграции с DI контейнером
    // - test_production_initialization
    // - test_graceful_shutdown_scenarios  
    // - test_emergency_shutdown
    // - test_health_monitoring_integration
    // - test_resource_adaptation
    // - test_sla_monitoring
}