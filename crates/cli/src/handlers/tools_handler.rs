//! Tools Handler - специализированный компонент для выполнения инструментов
//!
//! Реализует Single Responsibility для tool execution
//! Интегрируется через DI с интеллектуальной маршрутизацией

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::agent_traits::{
    AgentResponse, CircuitBreakerTrait, ComponentLifecycleTrait, IntelligentRoutingTrait,
    RequestContext,
};
use common::service_traits::HealthCheckService;

pub struct ToolsHandler<R, C>
where
    R: IntelligentRoutingTrait + HealthCheckService,
    C: CircuitBreakerTrait,
{
    routing_service: R,
    circuit_breaker: C,
    initialized: bool,
}

impl<R, C> ToolsHandler<R, C>
where
    R: IntelligentRoutingTrait + HealthCheckService,
    C: CircuitBreakerTrait,
{
    /// Создание нового ToolsHandler через DI
    pub fn new(routing_service: R, circuit_breaker: C) -> Self {
        Self {
            routing_service,
            circuit_breaker,
            initialized: false,
        }
    }

    /// Обработка tool execution запроса
    pub async fn handle_tools(&self, context: &RequestContext) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("ToolsHandler не инициализирован"));
        }

        debug!("ToolsHandler: обработка tool запроса: {}", context.message);

        // Используем Circuit Breaker для защиты от сбоев routing
        let result = self
            .circuit_breaker
            .execute(async { self.routing_service.process_request(&context.message).await })
            .await?;

        info!("ToolsHandler: успешно выполнен tool запрос");
        Ok(AgentResponse::ToolExecution(result))
    }

    /// Анализ tool запроса без выполнения (предварительная оценка)
    pub async fn analyze_tools_request(&self, context: &RequestContext) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("ToolsHandler не инициализирован"));
        }

        debug!("ToolsHandler: анализ tool запроса без выполнения");

        let analysis = self
            .circuit_breaker
            .execute(async { self.routing_service.analyze_request(&context.message).await })
            .await?;

        info!("ToolsHandler: анализ завершен");
        Ok(analysis)
    }

    /// Проверка возможности обработки запроса
    pub async fn can_handle(&self, context: &RequestContext) -> bool {
        if !self.initialized {
            return false;
        }

        let message_lower = context.message.to_lowercase();

        // Индикаторы tool команд
        let tool_indicators = [
            // Файловые операции
            "файл",
            "file",
            "папка",
            "folder",
            "directory",
            "dir",
            "прочитай",
            "read",
            "запиши",
            "write",
            "создай",
            "create",
            // Git операции
            "git",
            "commit",
            "status",
            "diff",
            "push",
            "pull",
            // Shell команды
            "команда",
            "command",
            "shell",
            "выполни",
            "execute",
            "запусти",
            "run",
            // Поиск и навигация
            "найди",
            "search",
            "покажи",
            "show",
            "список",
            "list",
            // Web операции
            "скачай",
            "download",
            "сайт",
            "website",
            "url",
            "http",
        ];

        tool_indicators
            .iter()
            .any(|&indicator| message_lower.contains(indicator))
    }

    /// Получение поддерживаемых типов операций
    pub fn get_supported_operations(&self) -> Vec<&'static str> {
        vec![
            "file_operations",
            "git_operations",
            "shell_commands",
            "web_operations",
            "search_operations",
        ]
    }

    /// Получение статистики использования
    pub fn get_usage_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("tools_executed".to_string(), 0);
        stats.insert("avg_execution_time_ms".to_string(), 0);
        stats.insert("success_rate_percent".to_string(), 0);
        stats.insert("circuit_breaker_trips".to_string(), 0);
        stats
    }

    /// Проверка доступности конкретного инструмента
    pub async fn check_tool_availability(&self, tool_name: &str) -> Result<bool> {
        if !self.initialized {
            return Ok(false);
        }

        // В production версии здесь проверяется реальная доступность
        debug!(
            "ToolsHandler: проверка доступности инструмента: {}",
            tool_name
        );

        // Проверяем базовую доступность routing service
        self.routing_service.health_check().await?;

        Ok(true)
    }
}

#[async_trait]
impl<R, C> ComponentLifecycleTrait for ToolsHandler<R, C>
where
    R: IntelligentRoutingTrait + HealthCheckService,
    C: CircuitBreakerTrait,
{
    async fn initialize(&self) -> Result<()> {
        super::standard_component_initialize("ToolsHandler", self.routing_service.health_check()).await
    }

    async fn health_check(&self) -> Result<()> {
        super::standard_component_health_check(
            "ToolsHandler",
            self.initialized,
            self.routing_service.health_check(),
            self.circuit_breaker.get_state(),
        )
        .await
    }

    async fn shutdown(&self) -> Result<()> {
        info!("ToolsHandler: начинаем graceful shutdown");

        // В production версии здесь будет:
        // - Завершение активных tool executions
        // - Сохранение состояния и метрик
        // - Очистка ресурсов

        info!("ToolsHandler: shutdown завершен");
        Ok(())
    }
}

#[cfg(all(test, feature = "extended-tests", feature = "legacy-tests"))]
mod tests {
    use super::*;

    // Mock implementations для тестирования
    struct MockRoutingService;

    #[async_trait]
    impl IntelligentRoutingTrait for MockRoutingService {
        async fn process_request(&self, query: &str) -> Result<String> {
            Ok(format!("Mock tool execution result for: {}", query))
        }

        async fn analyze_request(&self, query: &str) -> Result<String> {
            Ok(format!("Mock analysis for: {}", query))
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
    async fn test_tools_handler_file_operations() {
        let handler = ToolsHandler::new(MockRoutingService, MockCircuitBreaker);
        let context = create_test_context("Прочитай файл test.txt");

        // Должен определить, что это tools запрос
        assert!(handler.can_handle(&context).await);
    }

    #[tokio::test]
    async fn test_tools_handler_git_operations() {
        let handler = ToolsHandler::new(MockRoutingService, MockCircuitBreaker);
        let context = create_test_context("git status");

        // Должен определить, что это tools запрос
        assert!(handler.can_handle(&context).await);
    }

    #[tokio::test]
    async fn test_tools_handler_chat_rejection() {
        let handler = ToolsHandler::new(MockRoutingService, MockCircuitBreaker);
        let context = create_test_context("Привет, как дела?");

        // Должен определить, что это НЕ tools запрос
        assert!(!handler.can_handle(&context).await);
    }

    #[tokio::test]
    async fn test_supported_operations() {
        let handler = ToolsHandler::new(MockRoutingService, MockCircuitBreaker);
        let operations = handler.get_supported_operations();

        assert!(operations.contains(&"file_operations"));
        assert!(operations.contains(&"git_operations"));
        assert!(operations.contains(&"shell_commands"));
    }
}
