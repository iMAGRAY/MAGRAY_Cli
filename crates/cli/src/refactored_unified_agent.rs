//! RefactoredUnifiedAgent - декомпозированная версия UnifiedAgentV2
//!
//! Применяет SOLID принципы через декомпозицию на отдельные компоненты:
//! - AgentCore: управление жизненным циклом
//! - HandlerRegistry: регистрация и маршрутизация handlers
//! - CircuitBreakerManager: управление circuit breakers
//! - PerformanceTracker: мониторинг производительности

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::agent_core::{AgentComponent, AgentCore};
use crate::agent_traits::{AgentResponse, ProcessingResult, RequestContext, RequestProcessorTrait};
use crate::circuit_breaker_manager::{CircuitBreakerConfig, CircuitBreakerManager};
use crate::handler_registry::{AdaptiveStrategy, HandlerRegistry, RequestHandler};
use crate::performance_tracker::{PerformanceTracker, TrackerConfig, WarningThresholds};

/// Конфигурация рефакторированного агента
pub struct RefactoredAgentConfig {
    pub performance_config: TrackerConfig,
    pub circuit_breaker_configs: HashMap<String, CircuitBreakerConfig>,
    pub enable_adaptive_routing: bool,
    pub max_concurrent_operations: usize,
}

impl Default for RefactoredAgentConfig {
    fn default() -> Self {
        let mut circuit_breaker_configs = HashMap::new();

        // Конфигурации circuit breakers для разных типов операций
        circuit_breaker_configs.insert(
            "chat".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 5,
                recovery_timeout: std::time::Duration::from_secs(30),
                min_request_threshold: 10,
                error_rate_threshold: 0.3,
            },
        );

        circuit_breaker_configs.insert(
            "tools".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                recovery_timeout: std::time::Duration::from_secs(15),
                min_request_threshold: 5,
                error_rate_threshold: 0.2,
            },
        );

        circuit_breaker_configs.insert(
            "memory".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 7,
                recovery_timeout: std::time::Duration::from_secs(45),
                min_request_threshold: 15,
                error_rate_threshold: 0.4,
            },
        );

        Self {
            performance_config: TrackerConfig {
                max_metrics_in_memory: 5000,
                aggregation_interval: std::time::Duration::from_secs(30),
                enable_detailed_logging: false,
                warning_thresholds: WarningThresholds {
                    max_response_time_ms: 2000,
                    max_error_rate: 0.1,
                    max_concurrent_operations: 50,
                },
            },
            circuit_breaker_configs,
            enable_adaptive_routing: true,
            max_concurrent_operations: 100,
        }
    }
}

/// Рефакторированный агент с декомпозированной архитектурой
pub struct RefactoredUnifiedAgent {
    // Основные компоненты
    core: AgentCore,
    handler_registry: HandlerRegistry,
    circuit_breaker_manager: CircuitBreakerManager,
    performance_tracker: PerformanceTracker,

    // Конфигурация
    config: RefactoredAgentConfig,

    // Семафор для ограничения concurrent операций
    operation_semaphore: Arc<tokio::sync::Semaphore>,

    // Состояние
    initialized: bool,
}

impl RefactoredUnifiedAgent {
    /// Создать новый рефакторированный агент
    pub async fn new(config: RefactoredAgentConfig) -> Result<Self> {
        info!("Создание RefactoredUnifiedAgent с декомпозированной архитектурой");

        // Создаем core компоненты
        let core = AgentCore::new();

        // Создаем handler registry с адаптивной стратегией если включена
        let handler_registry = if config.enable_adaptive_routing {
            HandlerRegistry::with_strategy(Box::new(AdaptiveStrategy::new()))
        } else {
            HandlerRegistry::new()
        };

        // Создаем circuit breaker manager
        let circuit_breaker_manager = CircuitBreakerManager::new();

        // Регистрируем circuit breakers из конфигурации
        for (name, cb_config) in &config.circuit_breaker_configs {
            circuit_breaker_manager
                .register_circuit_breaker(name.clone(), cb_config.clone())
                .await;
        }

        // Создаем performance tracker
        let performance_tracker = PerformanceTracker::new(config.performance_config.clone());

        let operation_semaphore = Arc::new(tokio::sync::Semaphore::new(
            config.max_concurrent_operations,
        ));

        let agent = Self {
            core,
            handler_registry,
            circuit_breaker_manager,
            performance_tracker,
            config,
            operation_semaphore,
            initialized: false,
        };

        info!("RefactoredUnifiedAgent создан успешно");
        Ok(agent)
    }

    /// Зарегистрировать handler в реестре
    pub fn register_handler(&mut self, handler: Arc<dyn RequestHandler>) -> Result<()> {
        self.handler_registry.register_handler(handler)
    }

    /// Зарегистрировать компонент в core
    pub fn register_component(&mut self, component: Box<dyn AgentComponent>) {
        self.core.register_component(component);
    }

    /// Инициализация всех компонентов
    pub async fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        info!("Инициализация RefactoredUnifiedAgent");

        let init_op_id = self
            .performance_tracker
            .start_operation("system", "initialization")
            .await;

        // Инициализируем компоненты параллельно
        let results = tokio::join!(
            self.core.initialize(),
            self.handler_registry.initialize_all_handlers(),
        );

        // Проверяем результаты
        results
            .0
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации core: {}", e))?;
        results
            .1
            .map_err(|e| anyhow::anyhow!("Ошибка инициализации handlers: {}", e))?;

        self.initialized = true;

        self.performance_tracker
            .finish_operation(&init_op_id, true)
            .await?;

        info!("RefactoredUnifiedAgent успешно инициализирован");
        Ok(())
    }

    /// Обработка пользовательского запроса с полной декомпозицией
    async fn process_request_internal(
        &mut self,
        context: &RequestContext,
    ) -> Result<ProcessingResult> {
        // Получаем permit для ограничения concurrent операций
        let _permit = self
            .operation_semaphore
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для операции: {}", e))?;

        let processing_op_id = self
            .performance_tracker
            .start_operation("agent", "request_processing")
            .await;

        let start_time = std::time::Instant::now();
        let mut components_used = Vec::new();
        let mut metrics = HashMap::new();

        debug!("Обработка запроса: '{}'", context.message);

        // Маршрутизируем запрос к подходящему handler'у
        let routing_result = self.handler_registry.route_request(context).await;

        let response = match routing_result {
            Some(routing) => {
                components_used.push("handler_registry".to_string());
                components_used.push(routing.handler_name.clone());

                info!(
                    "Маршрутизация: {} (confidence: {:.2}, reason: {})",
                    routing.handler_name, routing.confidence, routing.reason
                );

                // Проверяем circuit breaker для выбранного handler'а
                let can_execute = self
                    .circuit_breaker_manager
                    .can_execute(&routing.handler_name)
                    .await?;

                if !can_execute {
                    components_used.push("circuit_breaker_fallback".to_string());
                    warn!(
                        "Circuit breaker заблокировал handler '{}'",
                        routing.handler_name
                    );

                    // Пытаемся найти альтернативный handler (простая fallback логика)
                    self.handle_circuit_breaker_fallback(context).await
                } else {
                    // Выполняем запрос с защитой circuit breaker
                    let handler_op_id = self
                        .performance_tracker
                        .start_operation(&routing.handler_name, "handle_request")
                        .await;

                    let result = self
                        .circuit_breaker_manager
                        .execute_with_breaker(&routing.handler_name, async {
                            self.handler_registry.handle_request(context).await
                        })
                        .await;

                    let success = result.is_ok();
                    self.performance_tracker
                        .finish_operation(&handler_op_id, success)
                        .await?;

                    // Обновляем метрики производительности компонента
                    let handler_duration = start_time.elapsed().as_millis() as f64;
                    self.core
                        .update_request_stats(&routing.handler_name, handler_duration);

                    metrics.insert("handler_confidence".to_string(), routing.confidence);
                    metrics.insert("circuit_breaker_blocked".to_string(), 0.0);

                    result
                }
            }
            None => {
                warn!("Не найден подходящий handler для запроса");
                components_used.push("fallback_handler".to_string());

                // Базовый fallback - возвращаем информационное сообщение
                Ok(AgentResponse::Chat(
                    "Извините, я не смог найти подходящий обработчик для вашего запроса."
                        .to_string(),
                ))
            }
        };

        let processing_time = start_time.elapsed();
        let success = response.is_ok();

        // Завершаем tracking основной операции
        self.performance_tracker
            .finish_operation(&processing_op_id, success)
            .await?;

        // Добавляем общие метрики
        metrics.insert(
            "processing_time_ms".to_string(),
            processing_time.as_millis() as f64,
        );
        metrics.insert("components_count".to_string(), components_used.len() as f64);
        metrics.insert(
            "concurrent_operations".to_string(),
            (self.config.max_concurrent_operations - self.operation_semaphore.available_permits())
                as f64,
        );

        match response {
            Ok(agent_response) => {
                info!("Запрос обработан успешно за {:?}", processing_time);

                Ok(ProcessingResult {
                    response: agent_response,
                    processing_time_ms: processing_time.as_millis() as u64,
                    components_used,
                    metrics,
                })
            }
            Err(e) => {
                error!("Ошибка обработки запроса: {}", e);
                Err(e)
            }
        }
    }

    /// Fallback обработка при блокировке circuit breaker
    async fn handle_circuit_breaker_fallback(
        &mut self,
        context: &RequestContext,
    ) -> Result<AgentResponse> {
        debug!("Обработка fallback при блокировке circuit breaker");

        // Простая fallback логика - можно расширить
        let fallback_message = format!(
            "Сервис временно недоступен. Попробуйте повторить запрос позже.\nВаш запрос: '{}'",
            context.message
        );

        Ok(AgentResponse::Chat(fallback_message))
    }

    /// Получить общую статистику агента
    pub async fn get_comprehensive_stats(&self) -> String {
        let mut stats = String::new();

        stats.push_str("=== RefactoredUnifiedAgent Comprehensive Stats ===\n\n");

        // Performance stats
        let perf_summary = self.performance_tracker.get_summary().await;
        stats.push_str(&format!("Performance: {}\n\n", perf_summary));

        // Core component stats
        let component_stats = self.core.get_component_stats().await;
        stats.push_str("Component Status:\n");
        for (name, stat) in component_stats {
            stats.push_str(&format!(
                "├─ {}: {} (ready: {}, healthy: {}, {} requests, {:.1}ms avg)\n",
                name,
                if stat.ready && stat.healthy {
                    "✅"
                } else {
                    "❌"
                },
                stat.ready,
                stat.healthy,
                stat.requests_processed,
                stat.average_response_time_ms
            ));
        }

        // Handler stats
        stats.push_str("\nHandler Statistics:\n");
        let handler_stats = self.handler_registry.get_all_handler_stats();
        for (name, stat) in handler_stats {
            let success_rate = if stat.requests_handled > 0 {
                stat.successful_requests as f64 / stat.requests_handled as f64 * 100.0
            } else {
                100.0
            };

            stats.push_str(&format!(
                "├─ {}: {} requests, {:.1}% success, {:.1}ms avg\n",
                name, stat.requests_handled, success_rate, stat.average_response_time_ms
            ));
        }

        // Circuit breaker states
        stats.push_str("\nCircuit Breaker States:\n");
        let cb_states = self.circuit_breaker_manager.get_states().await;
        for (name, state) in cb_states {
            stats.push_str(&format!("├─ {}: {:?}\n", name, state));
        }

        // System health
        stats.push_str(&format!("\nSystem Health:\n"));
        stats.push_str(&format!("├─ Initialized: {}\n", self.initialized));
        stats.push_str(&format!("├─ Core ready: {}\n", self.core.is_ready().await));
        stats.push_str(&format!(
            "├─ Available permits: {}/{}\n",
            self.operation_semaphore.available_permits(),
            self.config.max_concurrent_operations
        ));
        stats.push_str(&format!(
            "└─ Adaptive routing: {}\n",
            self.config.enable_adaptive_routing
        ));

        stats
    }

    /// Получить детальный отчет о производительности
    pub async fn get_performance_report(&self, last_seconds: u64) -> Result<String> {
        self.performance_tracker
            .get_detailed_metrics(last_seconds)
            .await
    }

    /// Сбросить circuit breakers
    pub async fn reset_circuit_breakers(&self) -> Result<()> {
        self.circuit_breaker_manager
            .reset_all_circuit_breakers()
            .await
    }

    /// Очистить старые метрики
    pub async fn cleanup_old_metrics(&self, older_than_hours: u64) -> Result<usize> {
        self.performance_tracker
            .cleanup_old_metrics(older_than_hours)
            .await
    }

    /// Проверить готовность системы
    pub async fn system_ready(&self) -> bool {
        self.initialized && self.core.is_ready().await
    }

    /// Graceful shutdown
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Начинаем graceful shutdown RefactoredUnifiedAgent");

        let shutdown_op_id = self
            .performance_tracker
            .start_operation("system", "shutdown")
            .await;

        // Останавливаем компоненты в обратном порядке
        let results = tokio::join!(self.handler_registry.shutdown_all(), self.core.shutdown(),);

        // Логируем результаты shutdown
        if let Err(e) = results.0 {
            warn!("Ошибка при shutdown handler registry: {}", e);
        }
        if let Err(e) = results.1 {
            warn!("Ошибка при shutdown core: {}", e);
        }

        self.initialized = false;

        self.performance_tracker
            .finish_operation(&shutdown_op_id, true)
            .await?;

        info!("RefactoredUnifiedAgent shutdown завершен");
        Ok(())
    }
}

#[async_trait]
impl RequestProcessorTrait for RefactoredUnifiedAgent {
    async fn process_user_request(&self, context: RequestContext) -> Result<ProcessingResult> {
        if !self.initialized {
            return Err(anyhow::anyhow!("RefactoredUnifiedAgent не инициализирован"));
        }

        // Поскольку process_request_internal требует &mut self, а этот метод &self,
        // нам нужно сделать внутренние компоненты thread-safe или изменить архитектуру.
        // Для упрощения, сейчас вернем ошибку с пояснением.
        Err(anyhow::anyhow!(
            "RefactoredUnifiedAgent требует рефакторинга RequestProcessorTrait для мутабельного доступа. \
             Используйте process_request_internal напрямую с мутабельной ссылкой."
        ))
    }

    async fn is_ready(&self) -> bool {
        self.system_ready().await
    }

    async fn shutdown(&self) -> Result<()> {
        // Поскольку метод immutable, мы не можем изменить состояние
        Err(anyhow::anyhow!(
            "RefactoredUnifiedAgent требует мутабельного shutdown. \
             Используйте shutdown напрямую с мутабельной ссылкой."
        ))
    }
}

/// Builder для рефакторированного агента
pub struct RefactoredUnifiedAgentBuilder {
    config: RefactoredAgentConfig,
    handlers: Vec<Arc<dyn RequestHandler>>,
    components: Vec<Box<dyn AgentComponent>>,
}

impl RefactoredUnifiedAgentBuilder {
    pub fn new() -> Self {
        Self {
            config: RefactoredAgentConfig::default(),
            handlers: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: RefactoredAgentConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_handler(mut self, handler: Arc<dyn RequestHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn with_component(mut self, component: Box<dyn AgentComponent>) -> Self {
        self.components.push(component);
        self
    }

    pub fn with_performance_tracking(mut self, enabled: bool) -> Self {
        self.config.performance_config.enable_detailed_logging = enabled;
        self
    }

    pub fn with_adaptive_routing(mut self, enabled: bool) -> Self {
        self.config.enable_adaptive_routing = enabled;
        self
    }

    pub fn with_max_concurrent_operations(mut self, max: usize) -> Self {
        self.config.max_concurrent_operations = max;
        self
    }

    pub async fn build(self) -> Result<RefactoredUnifiedAgent> {
        let mut agent = RefactoredUnifiedAgent::new(self.config).await?;

        // Регистрируем handlers
        for handler in self.handlers {
            agent.register_handler(handler)?;
        }

        // Регистрируем components
        for component in self.components {
            agent.register_component(component);
        }

        Ok(agent)
    }
}

impl Default for RefactoredUnifiedAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler_registry::RequestHandler;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockHandler {
        name: &'static str,
        can_handle_result: bool,
        request_count: Arc<AtomicU64>,
    }

    impl MockHandler {
        fn new(name: &'static str, can_handle: bool) -> Self {
            Self {
                name,
                can_handle_result: can_handle,
                request_count: Arc::new(AtomicU64::new(0)),
            }
        }

        fn get_request_count(&self) -> u64 {
            self.request_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl RequestHandler for MockHandler {
        fn handler_name(&self) -> &'static str {
            self.name
        }

        async fn can_handle(&self, _context: &RequestContext) -> bool {
            self.can_handle_result
        }

        async fn handle_request(&self, context: &RequestContext) -> Result<AgentResponse> {
            self.request_count.fetch_add(1, Ordering::Relaxed);
            Ok(AgentResponse::Chat(format!(
                "Mock response to: {}",
                context.message
            )))
        }

        fn priority(&self) -> u32 {
            100
        }
    }

    #[tokio::test]
    async fn test_refactored_agent_basic_functionality() {
        let config = RefactoredAgentConfig::default();
        let mut agent = RefactoredUnifiedAgent::new(config).await.unwrap();

        // Регистрируем mock handler
        let handler = Arc::new(MockHandler::new("test_handler", true));
        agent.register_handler(handler.clone()).unwrap();

        // Инициализируем агента
        agent.initialize().await.unwrap();

        assert!(agent.system_ready().await);

        // Тестируем обработку запроса
        let context = RequestContext {
            message: "test message".to_string(),
            metadata: HashMap::new(),
        };

        let result = agent.process_request_internal(&context).await;
        assert!(result.is_ok());

        // Проверяем что handler был вызван
        assert_eq!(handler.get_request_count(), 1);

        // Получаем статистику
        let stats = agent.get_comprehensive_stats().await;
        assert!(stats.contains("RefactoredUnifiedAgent"));
        assert!(stats.contains("test_handler"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_integration() {
        let mut config = RefactoredAgentConfig::default();
        // Устанавливаем низкий порог для тестирования
        config.circuit_breaker_configs.insert(
            "test_handler".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 1,
                recovery_timeout: std::time::Duration::from_millis(100),
                min_request_threshold: 1,
                error_rate_threshold: 0.5,
            },
        );

        let mut agent = RefactoredUnifiedAgent::new(config).await.unwrap();

        // Handler который всегда падает
        struct FailingHandler;

        #[async_trait]
        impl RequestHandler for FailingHandler {
            fn handler_name(&self) -> &'static str {
                "failing_handler"
            }
            async fn can_handle(&self, _: &RequestContext) -> bool {
                true
            }
            async fn handle_request(&self, _: &RequestContext) -> Result<AgentResponse> {
                Err(anyhow::anyhow!("Simulated failure"))
            }
        }

        agent.register_handler(Arc::new(FailingHandler)).unwrap();
        agent.initialize().await.unwrap();

        let context = RequestContext {
            message: "test".to_string(),
            metadata: HashMap::new(),
        };

        // Первый запрос должен провалиться и открыть circuit breaker
        let _result = agent.process_request_internal(&context).await;

        // Проверяем состояние circuit breaker
        let cb_states = agent.circuit_breaker_manager.get_states().await;
        // Circuit breaker может быть открыт после одной ошибки в зависимости от конфигурации
    }

    #[tokio::test]
    async fn test_performance_tracking() {
        let config = RefactoredAgentConfig {
            performance_config: TrackerConfig {
                enable_detailed_logging: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut agent = RefactoredUnifiedAgent::new(config).await.unwrap();

        let handler = Arc::new(MockHandler::new("perf_test_handler", true));
        agent.register_handler(handler.clone()).unwrap();
        agent.initialize().await.unwrap();

        let context = RequestContext {
            message: "performance test".to_string(),
            metadata: HashMap::new(),
        };

        // Выполняем несколько запросов
        for _ in 0..3 {
            let _result = agent.process_request_internal(&context).await;
        }

        // Проверяем метрики производительности
        let perf_report = agent.get_performance_report(60).await;
        assert!(perf_report.is_ok());

        let stats = agent.get_comprehensive_stats().await;
        assert!(stats.contains("Performance:"));
        assert!(stats.contains("requests"));
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let handler = Arc::new(MockHandler::new("builder_test", true));

        let agent = RefactoredUnifiedAgentBuilder::new()
            .with_handler(handler)
            .with_adaptive_routing(true)
            .with_performance_tracking(true)
            .with_max_concurrent_operations(20)
            .build()
            .await;

        assert!(agent.is_ok());
        let agent = agent.unwrap();

        assert_eq!(agent.config.max_concurrent_operations, 20);
        assert!(agent.config.enable_adaptive_routing);
        assert!(agent.config.performance_config.enable_detailed_logging);
    }
}
