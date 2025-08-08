//! Chat Handler - специализированный компонент для обработки chat запросов
//!
//! Реализует Single Responsibility для LLM взаимодействия
//! Интегрируется через DI с абстракциями

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::agent_traits::{
    AgentResponse, CircuitBreakerTrait, ComponentLifecycleTrait, LlmServiceTrait, RequestContext,
};

pub struct ChatHandler<L, C>
where
    L: LlmServiceTrait,
    C: CircuitBreakerTrait,
{
    llm_service: L,
    circuit_breaker: C,
    initialized: bool,
}

impl<L, C> ChatHandler<L, C>
where
    L: LlmServiceTrait,
    C: CircuitBreakerTrait,
{
    /// Создание нового ChatHandler через DI
    pub fn new(llm_service: L, circuit_breaker: C) -> Self {
        Self {
            llm_service,
            circuit_breaker,
            initialized: false,
        }
    }

    /// Обработка простого chat запроса
    pub async fn handle_chat(&self, context: &RequestContext) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("ChatHandler не инициализирован"));
        }

        debug!(
            "ChatHandler: обработка запроса длиной {}",
            context.message.len()
        );

        // Используем Circuit Breaker для защиты от сбоев LLM
        let response = self
            .circuit_breaker
            .execute(async { self.llm_service.chat(&context.message).await })
            .await?;

        info!("ChatHandler: успешно обработан chat запрос");
        Ok(AgentResponse::Chat(response))
    }

    /// Обработка chat запроса с дополнительным контекстом
    pub async fn handle_chat_with_context(
        &self,
        context: &RequestContext,
    ) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("ChatHandler не инициализирован"));
        }

        debug!(
            "ChatHandler: обработка запроса с контекстом: {:?}",
            context.metadata
        );

        let response = self
            .circuit_breaker
            .execute(async {
                self.llm_service
                    .chat_with_context(&context.message, &context.metadata)
                    .await
            })
            .await?;

        info!("ChatHandler: успешно обработан контекстный chat запрос");
        Ok(AgentResponse::Chat(response))
    }

    /// Проверка возможности обработки запроса
    pub async fn can_handle(&self, context: &RequestContext) -> bool {
        if !self.initialized {
            return false;
        }

        // Простая эвристика для определения chat запросов
        let message_lower = context.message.to_lowercase();

        // Исключаем явные tool команды
        let tool_indicators = [
            "файл",
            "file",
            "папка",
            "folder",
            "directory",
            "dir",
            "git",
            "commit",
            "status",
            "команда",
            "command",
            "shell",
            "создай",
            "create",
            "покажи",
            "show",
            "список",
            "list",
            "прочитай",
            "read",
            "запиши",
            "write",
            "найди",
            "search",
        ];

        let has_tool_indicators = tool_indicators
            .iter()
            .any(|&indicator| message_lower.contains(indicator));

        // Исключаем административные команды
        let admin_indicators = [
            "статистика",
            "stats",
            "здоровье",
            "health",
            "метрики",
            "metrics",
            "система",
            "system",
            "производительность",
            "performance",
        ];

        let has_admin_indicators = admin_indicators
            .iter()
            .any(|&indicator| message_lower.contains(indicator));

        // Это chat запрос если нет явных индикаторов других типов
        !has_tool_indicators && !has_admin_indicators
    }

    /// Получение статистики использования
    pub fn get_usage_stats(&self) -> HashMap<String, u64> {
        // В production версии здесь будут реальные метрики
        let mut stats = HashMap::new();
        stats.insert("requests_processed".to_string(), 0);
        stats.insert("avg_response_time_ms".to_string(), 0);
        stats.insert("circuit_breaker_trips".to_string(), 0);
        stats
    }
}

#[async_trait]
impl<L, C> ComponentLifecycleTrait for ChatHandler<L, C>
where
    L: LlmServiceTrait,
    C: CircuitBreakerTrait,
{
    async fn initialize(&self) -> Result<()> {
        info!("ChatHandler: инициализация начата");

        // Проверяем доступность LLM сервиса
        self.llm_service
            .health_check()
            .await
            .map_err(|e| anyhow::anyhow!("LLM сервис недоступен: {}", e))?;

        // Здесь можно добавить дополнительную инициализацию
        // unsafe { &mut *(self as *const _ as *mut Self) }.initialized = true;

        info!("ChatHandler: инициализация завершена");
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("ChatHandler не инициализирован"));
        }

        // Проверяем все зависимости
        self.llm_service.health_check().await?;

        // Проверяем состояние Circuit Breaker
        let breaker_state = self.circuit_breaker.get_state().await;
        if breaker_state == "Open" {
            return Err(anyhow::anyhow!("Circuit breaker открыт"));
        }

        debug!("ChatHandler: health check прошел успешно");
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        info!("ChatHandler: начинаем graceful shutdown");

        // В production версии здесь будет:
        // - Завершение активных операций
        // - Сохранение метрик
        // - Очистка ресурсов

        info!("ChatHandler: shutdown завершен");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock implementations для тестирования
    struct MockLlmService;

    #[async_trait]
    impl LlmServiceTrait for MockLlmService {
        async fn chat(&self, message: &str) -> Result<String> {
            Ok(format!("Mock response to: {}", message))
        }

        async fn chat_with_context(
            &self,
            message: &str,
            _context: &HashMap<String, String>,
        ) -> Result<String> {
            Ok(format!("Mock contextual response to: {}", message))
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }
    }

    struct MockCircuitBreaker;

    #[async_trait]
    impl CircuitBreakerTrait for MockCircuitBreaker {
        async fn execute<F, T>(&self, operation: F) -> Result<T>
        where
            F: std::future::Future<Output = Result<T>> + Send,
            T: Send,
        {
            operation.await
        }

        async fn force_open(&self) {}
        async fn force_close(&self) {}

        async fn get_state(&self) -> String {
            "Closed".to_string()
        }
    }

    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_chat_handler_basic() {
        let handler = ChatHandler::new(MockLlmService, MockCircuitBreaker);
        let context = create_test_context("Привет!");

        // Должен определить, что это chat запрос
        assert!(handler.can_handle(&context).await);
    }

    #[tokio::test]
    async fn test_chat_handler_tool_detection() {
        let handler = ChatHandler::new(MockLlmService, MockCircuitBreaker);
        let context = create_test_context("Прочитай файл test.txt");

        // Должен определить, что это НЕ chat запрос
        assert!(!handler.can_handle(&context).await);
    }

    #[tokio::test]
    async fn test_chat_handler_admin_detection() {
        let handler = ChatHandler::new(MockLlmService, MockCircuitBreaker);
        let context = create_test_context("Покажи статистику системы");

        // Должен определить, что это НЕ chat запрос
        assert!(!handler.can_handle(&context).await);
    }
}

// Реализуем ServiceMacroHelpers для использования макросов
impl<L, C> ServiceMacroHelpers for ChatHandler<L, C>
where
    L: LlmServiceTrait,
    C: CircuitBreakerTrait,
{
    type HealthData = String;
    type Stats = HashMap<String, u64>;

    async fn create_health_data(&self) -> Result<Self::HealthData, MagrayCoreError> {
        Ok("ChatHandler is healthy".to_string())
    }

    fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn set_initialized(&self, _initialized: bool) {
        // На данный момент не можем изменять из-за немутабельной ссылки
        // В будущем можно использовать AtomicBool
    }

    async fn perform_initialization<T>(&mut self, _config: T) -> Result<(), MagrayCoreError> {
        info!("🚀 Инициализация ChatHandler");

        // Проверяем доступность LLM сервиса
        self.llm_service.health_check().await.map_err(|e| {
            MagrayCoreError::ServiceInitializationFailed(format!("LLM сервис недоступен: {}", e))
        })?;

        self.initialized = true;
        info!("✅ ChatHandler инициализирован");
        Ok(())
    }

    async fn perform_shutdown(&self) -> Result<(), MagrayCoreError> {
        info!("🛑 Остановка ChatHandler");
        // Здесь можно добавить очистку ресурсов
        info!("✅ ChatHandler остановлен");
        Ok(())
    }

    fn collect_stats(&self) -> Self::Stats {
        // В production версии здесь будут реальные метрики
        let mut stats = HashMap::new();
        stats.insert("requests_processed".to_string(), 0);
        stats.insert("avg_response_time_ms".to_string(), 0);
        stats.insert("circuit_breaker_trips".to_string(), 0);
        stats
    }

    fn perform_stats_reset(&mut self) {
        // Сброс статистики - реализация в будущем
        debug!("🔄 Сброс статистики ChatHandler");
    }

    fn is_clearable(&self) -> bool {
        true // ChatHandler можно очистить
    }

    async fn perform_clear(&mut self) -> Result<(), MagrayCoreError> {
        info!("🧹 Очистка ChatHandler");
        // Очистка кэша, статистики и т.д.
        Ok(())
    }
}

// Применяем макросы для автоматической генерации service traits
common::impl_health_check_service!(ChatHandler<L, C>, String);
common::impl_statistics_provider!(ChatHandler<L, C>, HashMap<String, u64>);
common::impl_clearable_service!(ChatHandler<L, C>);
common::impl_service_defaults!(ChatHandler<L, C>, name: "ChatHandler", version: "1.0.0");
