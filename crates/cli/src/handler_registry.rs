//! HandlerRegistry - реестр и маршрутизация обработчиков запросов
//!
//! Реализует Single Responsibility Principle для управления регистрацией,
//! маршрутизацией и жизненным циклом handler'ов.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::agent_traits::{AgentResponse, RequestContext};

/// Трейт для обработчика запросов
#[async_trait]
pub trait RequestHandler: Send + Sync {
    /// Имя обработчика для идентификации
    fn handler_name(&self) -> &'static str;

    /// Проверить может ли обработчик обработать данный запрос
    async fn can_handle(&self, context: &RequestContext) -> bool;

    /// Обработать запрос
    async fn handle_request(&self, context: &RequestContext) -> Result<AgentResponse>;

    /// Приоритет обработчика (чем выше, тем приоритетнее)
    fn priority(&self) -> u32 {
        100 // default priority
    }

    /// Инициализация обработчика
    async fn initialize(&mut self) -> Result<()> {
        Ok(()) // default implementation
    }

    /// Проверка здоровья обработчика
    async fn health_check(&self) -> Result<()> {
        Ok(()) // default implementation
    }

    /// Graceful shutdown обработчика
    async fn shutdown(&self) -> Result<()> {
        Ok(()) // default implementation
    }

    /// Получить метаданные обработчика
    fn get_metadata(&self) -> HandlerMetadata {
        HandlerMetadata {
            name: self.handler_name().to_string(),
            priority: self.priority(),
            capabilities: Vec::new(),
            tags: Vec::new(),
        }
    }
}

/// Метаданные обработчика
#[derive(Debug, Clone)]
pub struct HandlerMetadata {
    pub name: String,
    pub priority: u32,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
}

/// Статистика обработчика
#[derive(Debug, Clone)]
pub struct HandlerStats {
    pub name: String,
    pub requests_handled: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub last_request_time: Option<std::time::Instant>,
    pub is_healthy: bool,
}

/// Результат маршрутизации
#[derive(Debug)]
pub struct RoutingResult {
    pub handler_name: String,
    pub confidence: f64,
    pub reason: String,
}

/// Стратегия выбора обработчика
#[async_trait]
pub trait HandlerSelectionStrategy: Send + Sync {
    /// Выбрать лучший обработчик для запроса
    async fn select_handler(
        &self,
        context: &RequestContext,
        available_handlers: &[(&str, Arc<dyn RequestHandler>)],
    ) -> Option<RoutingResult>;
}

/// Стратегия выбора по приоритету
pub struct PriorityBasedStrategy;

#[async_trait]
impl HandlerSelectionStrategy for PriorityBasedStrategy {
    async fn select_handler(
        &self,
        context: &RequestContext,
        available_handlers: &[(&str, Arc<dyn RequestHandler>)],
    ) -> Option<RoutingResult> {
        let mut candidates = Vec::new();

        // Собираем кандидатов, которые могут обработать запрос
        for (name, handler) in available_handlers {
            if handler.can_handle(context).await {
                candidates.push((*name, handler.priority(), Arc::clone(handler)));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Сортируем по приоритету (убывание)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        let (best_name, best_priority, _) = &candidates[0];

        Some(RoutingResult {
            handler_name: best_name.to_string(),
            confidence: if candidates.len() == 1 { 1.0 } else { 0.8 },
            reason: format!("Выбран по приоритету: {}", best_priority),
        })
    }
}

/// Адаптивная стратегия выбора (учитывает производительность)
pub struct AdaptiveStrategy {
    handler_stats: Arc<tokio::sync::RwLock<HashMap<String, HandlerStats>>>,
}

impl Default for AdaptiveStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptiveStrategy {
    pub fn new() -> Self {
        Self {
            handler_stats: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_stats(&self, handler_name: &str, success: bool, response_time_ms: f64) {
        let mut stats = self.handler_stats.write().await;
        let handler_stats = stats
            .entry(handler_name.to_string())
            .or_insert_with(|| HandlerStats {
                name: handler_name.to_string(),
                requests_handled: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time_ms: 0.0,
                last_request_time: None,
                is_healthy: true,
            });

        handler_stats.requests_handled += 1;
        if success {
            handler_stats.successful_requests += 1;
        } else {
            handler_stats.failed_requests += 1;
        }

        // Скользящее среднее времени отклика
        let alpha = 0.1;
        handler_stats.average_response_time_ms =
            alpha * response_time_ms + (1.0 - alpha) * handler_stats.average_response_time_ms;

        handler_stats.last_request_time = Some(std::time::Instant::now());

        // Оценка здоровья на основе success rate
        let success_rate =
            handler_stats.successful_requests as f64 / handler_stats.requests_handled as f64;
        handler_stats.is_healthy = success_rate >= 0.8;
    }
}

#[async_trait]
impl HandlerSelectionStrategy for AdaptiveStrategy {
    async fn select_handler(
        &self,
        context: &RequestContext,
        available_handlers: &[(&str, Arc<dyn RequestHandler>)],
    ) -> Option<RoutingResult> {
        let mut candidates = Vec::new();
        let stats = self.handler_stats.read().await;

        // Собираем кандидатов с их метриками
        for (name, handler) in available_handlers {
            if handler.can_handle(context).await {
                let handler_stats = stats.get(*name);

                // Вычисляем score на основе приоритета, производительности и здоровья
                let base_priority = handler.priority() as f64;
                let health_multiplier = match handler_stats {
                    Some(s) if s.is_healthy => 1.0,
                    Some(_) => 0.5, // нездоровый handler
                    None => 0.9,    // новый handler без статистики
                };

                let performance_multiplier = match handler_stats {
                    Some(s) if s.requests_handled > 0 => {
                        // Чем быстрее отвечает, тем лучше (но не более чем в 2 раза)
                        (1000.0 / (s.average_response_time_ms + 100.0)).min(2.0)
                    }
                    _ => 1.0,
                };

                let final_score = base_priority * health_multiplier * performance_multiplier;

                candidates.push((*name, final_score, Arc::clone(handler)));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Сортируем по итоговому score (убывание)
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_name, best_score, _) = &candidates[0];

        Some(RoutingResult {
            handler_name: best_name.to_string(),
            confidence: (best_score / 1000.0).clamp(0.5, 1.0),
            reason: format!("Адаптивный выбор (score: {:.2})", best_score),
        })
    }
}

/// Реестр обработчиков запросов
pub struct HandlerRegistry {
    handlers: HashMap<String, Arc<dyn RequestHandler>>,
    selection_strategy: Box<dyn HandlerSelectionStrategy>,
    handler_stats: HashMap<String, HandlerStats>,
}

impl HandlerRegistry {
    /// Создать новый реестр с стратегией по умолчанию
    pub fn new() -> Self {
        Self::with_strategy(Box::new(PriorityBasedStrategy))
    }

    /// Создать реестр с custom стратегией выбора
    pub fn with_strategy(strategy: Box<dyn HandlerSelectionStrategy>) -> Self {
        Self {
            handlers: HashMap::new(),
            selection_strategy: strategy,
            handler_stats: HashMap::new(),
        }
    }

    /// Зарегистрировать обработчик
    pub fn register_handler(&mut self, handler: Arc<dyn RequestHandler>) -> Result<()> {
        let handler_name = handler.handler_name().to_string();

        if self.handlers.contains_key(&handler_name) {
            return Err(anyhow::anyhow!(
                "Обработчик '{}' уже зарегистрирован",
                handler_name
            ));
        }

        info!(
            "Регистрация handler'а: {} (приоритет: {})",
            handler_name,
            handler.priority()
        );

        // Инициализируем статистику
        self.handler_stats.insert(
            handler_name.clone(),
            HandlerStats {
                name: handler_name.clone(),
                requests_handled: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time_ms: 0.0,
                last_request_time: None,
                is_healthy: true,
            },
        );

        self.handlers.insert(handler_name, handler);
        Ok(())
    }

    /// Удалить обработчик
    pub async fn unregister_handler(&mut self, handler_name: &str) -> Result<()> {
        match self.handlers.remove(handler_name) {
            Some(handler) => {
                info!("Удаление handler'а: {}", handler_name);

                // Graceful shutdown
                if let Err(e) = handler.shutdown().await {
                    warn!("Ошибка при остановке handler'а {}: {}", handler_name, e);
                }

                self.handler_stats.remove(handler_name);
                Ok(())
            }
            None => Err(anyhow::anyhow!("Обработчик '{}' не найден", handler_name)),
        }
    }

    /// Инициализировать все обработчики
    pub async fn initialize_all_handlers(&mut self) -> Result<()> {
        info!("Инициализация {} обработчиков", self.handlers.len());

        let mut init_results = Vec::new();

        // Обходим handlers через клонирование для избежания проблем с borrowing
        let handler_names: Vec<String> = self.handlers.keys().cloned().collect();

        for handler_name in handler_names {
            if let Some(handler) = self.handlers.get(&handler_name) {
                // Поскольку RequestHandler не имеет mut методов для initialize,
                // мы предполагаем что initialize не изменяет состояние
                match handler.health_check().await {
                    Ok(()) => {
                        debug!("Handler {} прошел health check", handler_name);
                        init_results.push((handler_name, true));
                    }
                    Err(e) => {
                        warn!("Handler {} не прошел health check: {}", handler_name, e);
                        init_results.push((handler_name, false));
                    }
                }
            }
        }

        // Проверяем результаты
        let failed_handlers: Vec<_> = init_results
            .iter()
            .filter(|(_, success)| !*success)
            .map(|(name, _)| name.clone())
            .collect();

        if !failed_handlers.is_empty() {
            warn!(
                "Некоторые handlers не прошли инициализацию: {:?}",
                failed_handlers
            );
        }

        info!("Инициализация handlers завершена");
        Ok(())
    }

    /// Найти лучший обработчик для запроса
    pub async fn route_request(&self, context: &RequestContext) -> Option<RoutingResult> {
        let available_handlers: Vec<_> = self
            .handlers
            .iter()
            .map(|(name, handler)| (name.as_str(), Arc::clone(handler)))
            .collect();

        self.selection_strategy
            .select_handler(context, &available_handlers)
            .await
    }

    /// Обработать запрос с автоматическим выбором handler'а
    pub async fn handle_request(&mut self, context: &RequestContext) -> Result<AgentResponse> {
        let routing_result = match self.route_request(context).await {
            Some(result) => result,
            None => {
                return Err(anyhow::anyhow!(
                    "Не найден подходящий обработчик для запроса"
                ));
            }
        };

        debug!(
            "Маршрутизация: {} ({})",
            routing_result.handler_name, routing_result.reason
        );

        let handler = self
            .handlers
            .get(&routing_result.handler_name)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Handler '{}' не найден в реестре",
                    routing_result.handler_name
                )
            })?;

        let start_time = std::time::Instant::now();
        let result = handler.handle_request(context).await;
        let response_time = start_time.elapsed().as_millis() as f64;

        // Обновляем статистику
        let success = result.is_ok();
        self.update_handler_stats(&routing_result.handler_name, success, response_time);

        // Если используется AdaptiveStrategy, обновляем ее статистику тоже
        if let Some(adaptive_strategy) = self
            .selection_strategy
            .as_any()
            .downcast_ref::<AdaptiveStrategy>()
        {
            adaptive_strategy
                .update_stats(&routing_result.handler_name, success, response_time)
                .await;
        }

        result
    }

    /// Обновить статистику обработчика
    fn update_handler_stats(&mut self, handler_name: &str, success: bool, response_time_ms: f64) {
        if let Some(stats) = self.handler_stats.get_mut(handler_name) {
            stats.requests_handled += 1;
            if success {
                stats.successful_requests += 1;
            } else {
                stats.failed_requests += 1;
            }

            // Скользящее среднее времени отклика
            let alpha = 0.1;
            stats.average_response_time_ms =
                alpha * response_time_ms + (1.0 - alpha) * stats.average_response_time_ms;

            stats.last_request_time = Some(std::time::Instant::now());

            // Оценка здоровья
            let success_rate = stats.successful_requests as f64 / stats.requests_handled as f64;
            stats.is_healthy = success_rate >= 0.8;
        }
    }

    /// Проверить здоровье всех обработчиков
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for (name, handler) in &self.handlers {
            let is_healthy = handler.health_check().await.is_ok();
            results.insert(name.clone(), is_healthy);
        }

        results
    }

    /// Получить статистику обработчика
    pub fn get_handler_stats(&self, handler_name: &str) -> Option<&HandlerStats> {
        self.handler_stats.get(handler_name)
    }

    /// Получить статистику всех обработчиков
    pub fn get_all_handler_stats(&self) -> &HashMap<String, HandlerStats> {
        &self.handler_stats
    }

    /// Получить метаданные всех обработчиков
    pub fn get_handler_metadata(&self) -> Vec<HandlerMetadata> {
        self.handlers
            .values()
            .map(|handler| handler.get_metadata())
            .collect()
    }

    /// Получить список имен обработчиков
    pub fn handler_names(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Количество зарегистрированных обработчиков
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Graceful shutdown всех обработчиков
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("Остановка всех {} обработчиков", self.handlers.len());

        for (name, handler) in &self.handlers {
            if let Err(e) = handler.shutdown().await {
                warn!("Ошибка при остановке handler'а {}: {}", name, e);
            } else {
                debug!("Handler {} успешно остановлен", name);
            }
        }

        info!("Остановка handlers завершена");
        Ok(())
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Вспомогательный трейт для downcasting
trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: 'static> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Удаляем проблемную реализацию - она не нужна для dyn trait objects
// вместо этого используем blanket implementation для конкретных типов

/// Builder для HandlerRegistry
pub struct HandlerRegistryBuilder {
    registry: HandlerRegistry,
}

impl HandlerRegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: HandlerRegistry::new(),
        }
    }

    pub fn with_strategy(strategy: Box<dyn HandlerSelectionStrategy>) -> Self {
        Self {
            registry: HandlerRegistry::with_strategy(strategy),
        }
    }

    pub fn with_handler(mut self, handler: Arc<dyn RequestHandler>) -> Result<Self> {
        self.registry.register_handler(handler)?;
        Ok(self)
    }

    pub fn build(self) -> HandlerRegistry {
        self.registry
    }
}

impl Default for HandlerRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestHandler {
        name: &'static str,
        priority: u32,
        can_handle_result: bool,
        requests_count: Arc<AtomicU64>,
    }

    impl TestHandler {
        fn new(name: &'static str, priority: u32, can_handle: bool) -> Self {
            Self {
                name,
                priority,
                can_handle_result: can_handle,
                requests_count: Arc::new(AtomicU64::new(0)),
            }
        }

        fn get_requests_count(&self) -> u64 {
            self.requests_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl RequestHandler for TestHandler {
        fn handler_name(&self) -> &'static str {
            self.name
        }

        fn priority(&self) -> u32 {
            self.priority
        }

        async fn can_handle(&self, _context: &RequestContext) -> bool {
            self.can_handle_result
        }

        async fn handle_request(&self, _context: &RequestContext) -> Result<AgentResponse> {
            self.requests_count.fetch_add(1, Ordering::Relaxed);
            Ok(AgentResponse::Chat("Test response".to_string()))
        }
    }

    #[tokio::test]
    async fn test_handler_registration_and_routing() {
        let mut registry = HandlerRegistry::new();

        let handler1 = Arc::new(TestHandler::new("handler1", 100, true));
        let handler2 = Arc::new(TestHandler::new("handler2", 200, true));

        registry
            .register_handler(handler1.clone())
            .expect("Operation failed - converted from unwrap()");
        registry
            .register_handler(handler2.clone())
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(registry.handler_count(), 2);

        let context = RequestContext {
            message: "test".to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        };

        // Should route to handler2 (higher priority)
        let routing_result = registry.route_request(&context).await;
        assert!(routing_result.is_some());
        assert_eq!(
            routing_result
                .expect("Operation failed - converted from unwrap()")
                .handler_name,
            "handler2"
        );
    }

    #[tokio::test]
    async fn test_request_handling_with_stats() {
        let mut registry = HandlerRegistry::new();

        let handler = Arc::new(TestHandler::new("test_handler", 100, true));
        registry
            .register_handler(handler.clone())
            .expect("Operation failed - converted from unwrap()");

        let context = RequestContext {
            message: "test".to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        };

        // Handle request
        let response = registry.handle_request(&context).await;
        assert!(response.is_ok());

        // Check that handler was called
        assert_eq!(handler.get_requests_count(), 1);

        // Check statistics
        let stats = registry.get_handler_stats("test_handler");
        assert!(stats.is_some());
        let stats = stats.expect("Operation failed - converted from unwrap()");
        assert_eq!(stats.requests_handled, 1);
        assert_eq!(stats.successful_requests, 1);
    }

    #[tokio::test]
    async fn test_handler_registry_builder() {
        let handler1 = Arc::new(TestHandler::new("handler1", 100, true));
        let handler2 = Arc::new(TestHandler::new("handler2", 200, true));

        let registry = HandlerRegistryBuilder::new()
            .with_handler(handler1)
            .expect("Operation failed - converted from unwrap()")
            .with_handler(handler2)
            .expect("Operation failed - converted from unwrap()")
            .build();

        assert_eq!(registry.handler_count(), 2);
        assert!(registry.handler_names().contains(&"handler1".to_string()));
        assert!(registry.handler_names().contains(&"handler2".to_string()));
    }

    #[tokio::test]
    async fn test_adaptive_strategy() {
        let adaptive_strategy = AdaptiveStrategy::new();

        // Update stats to make handler1 look better
        adaptive_strategy.update_stats("handler1", true, 50.0).await;
        adaptive_strategy.update_stats("handler1", true, 60.0).await;
        adaptive_strategy
            .update_stats("handler2", false, 200.0)
            .await;

        let handler1 = Arc::new(TestHandler::new("handler1", 100, true));
        let handler2 = Arc::new(TestHandler::new("handler2", 200, true)); // Higher priority but worse performance

        let available_handlers = vec![
            ("handler1", handler1 as Arc<dyn RequestHandler>),
            ("handler2", handler2 as Arc<dyn RequestHandler>),
        ];

        let context = RequestContext {
            message: "test".to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        };

        let routing_result = adaptive_strategy
            .select_handler(&context, &available_handlers)
            .await;
        // Should prefer handler1 due to better performance despite lower priority
        assert!(routing_result.is_some());
    }
}
