//! Fallback Strategies - различные стратегии для обработки ошибок и fallback сценариев
//!
//! Реализует Strategy pattern для graceful degradation при сбоях

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::agent_traits::{AgentResponse, FallbackStrategy, LlmServiceTrait, RequestContext};

// ============================================================================
// ОСНОВНЫЕ FALLBACK STRATEGIES
// ============================================================================

/// Простая fallback стратегия с хардкоженными ответами
pub struct SimpleFallbackStrategy {
    default_responses: HashMap<String, String>,
}

impl SimpleFallbackStrategy {
    pub fn new() -> Self {
        let mut default_responses = HashMap::new();

        // Типичные ошибки и ответы
        default_responses.insert(
            "network_error".to_string(),
            "Извините, возникли проблемы с сетевым соединением. Попробуйте позже.".to_string(),
        );
        default_responses.insert(
            "timeout_error".to_string(),
            "Операция заняла слишком много времени. Попробуйте упростить запрос.".to_string(),
        );
        default_responses.insert(
            "llm_error".to_string(),
            "Временно недоступен AI сервис. Могу выполнить базовые операции с файлами.".to_string(),
        );
        default_responses.insert(
            "memory_error".to_string(),
            "Проблемы с системой памяти. Функциональность временно ограничена.".to_string(),
        );
        default_responses.insert(
            "tools_error".to_string(),
            "Ошибка выполнения инструментов. Проверьте корректность команды.".to_string(),
        );
        default_responses.insert(
            "unknown_error".to_string(),
            "Произошла неожиданная ошибка. Попробуйте переформулировать запрос.".to_string(),
        );

        Self { default_responses }
    }

    fn classify_error(&self, error: &anyhow::Error) -> String {
        let error_string = error.to_string().to_lowercase();

        if error_string.contains("network") || error_string.contains("connection") {
            "network_error".to_string()
        } else if error_string.contains("timeout") || error_string.contains("timed out") {
            "timeout_error".to_string()
        } else if error_string.contains("llm")
            || error_string.contains("openai")
            || error_string.contains("anthropic")
        {
            "llm_error".to_string()
        } else if error_string.contains("memory") || error_string.contains("embedding") {
            "memory_error".to_string()
        } else if error_string.contains("tool") || error_string.contains("command") {
            "tools_error".to_string()
        } else {
            "unknown_error".to_string()
        }
    }
}

impl Default for SimpleFallbackStrategy {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl FallbackStrategy for SimpleFallbackStrategy {
    async fn handle_fallback(
        &self,
        context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse> {
        warn!(
            "SimpleFallbackStrategy: обработка ошибки для '{}'",
            context.message
        );
        debug!("Ошибка: {}", error);

        let error_type = self.classify_error(error);
        let response_text = self
            .default_responses
            .get(&error_type)
            .unwrap_or(&self.default_responses["unknown_error"])
            .clone();

        info!(
            "SimpleFallbackStrategy: возвращаем fallback ответ для типа '{}'",
            error_type
        );

        Ok(AgentResponse::Error(response_text))
    }

    fn can_handle(&self, _error: &anyhow::Error) -> bool {
        true // Может обработать любую ошибку
    }

    fn priority(&self) -> u8 {
        1 // Самый низкий приоритет - используется как последний resort
    }
}

/// Умная fallback стратегия с попыткой альтернативных подходов
pub struct SmartFallbackStrategy<L>
where
    L: LlmServiceTrait,
{
    llm_service: Option<L>,
    #[allow(dead_code)] // Количество повторов для fallback стратегии
    retry_count: u32,
}

impl<L> SmartFallbackStrategy<L>
where
    L: LlmServiceTrait,
{
    pub fn new(llm_service: Option<L>, retry_count: u32) -> Self {
        Self {
            llm_service,
            retry_count,
        }
    }

    async fn try_alternative_approach(
        &self,
        context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse> {
        let error_string = error.to_string().to_lowercase();

        // Если LLM недоступен, пробуем локальную обработку
        if error_string.contains("llm")
            || error_string.contains("openai")
            || error_string.contains("anthropic")
        {
            return self.handle_llm_fallback(context).await;
        }

        // Если проблемы с tools, предлагаем альтернативы
        if error_string.contains("tool") || error_string.contains("command") {
            return self.handle_tools_fallback(context).await;
        }

        // Если проблемы с памятью, работаем без неё
        if error_string.contains("memory") || error_string.contains("embedding") {
            return self.handle_memory_fallback(context).await;
        }

        // Общий fallback
        Ok(AgentResponse::Error(
            "Не удалось выполнить запрос. Попробуйте упростить или переформулировать.".to_string(),
        ))
    }

    async fn handle_llm_fallback(&self, context: &RequestContext) -> Result<AgentResponse> {
        debug!(
            "SmartFallbackStrategy: LLM fallback для '{}'",
            context.message
        );

        // Пробуем простые локальные команды
        let message_lower = context.message.to_lowercase();

        if message_lower.contains("статус") || message_lower.contains("status") {
            return Ok(AgentResponse::Chat(
                "LLM недоступен. Статус системы можно проверить командой 'health check'."
                    .to_string(),
            ));
        }

        if message_lower.contains("помощь") || message_lower.contains("help") {
            return Ok(AgentResponse::Chat(
                "Доступные команды: file operations, git commands, system status. LLM временно недоступен.".to_string()
            ));
        }

        Ok(AgentResponse::Chat(
            "AI временно недоступен. Могу выполнить базовые операции с файлами и git.".to_string(),
        ))
    }

    async fn handle_tools_fallback(&self, context: &RequestContext) -> Result<AgentResponse> {
        debug!(
            "SmartFallbackStrategy: Tools fallback для '{}'",
            context.message
        );

        let message_lower = context.message.to_lowercase();

        // Предлагаем альтернативные инструменты
        if message_lower.contains("файл") || message_lower.contains("file") {
            return Ok(AgentResponse::ToolExecution(
                "Ошибка инструмента файлов. Попробуйте проверить путь и права доступа.".to_string(),
            ));
        }

        if message_lower.contains("git") {
            return Ok(AgentResponse::ToolExecution(
                "Git команда не выполнена. Проверьте, что находитесь в git репозитории."
                    .to_string(),
            ));
        }

        Ok(AgentResponse::ToolExecution(
            "Инструмент недоступен. Попробуйте альтернативный подход или обратитесь к администратору.".to_string()
        ))
    }

    async fn handle_memory_fallback(&self, context: &RequestContext) -> Result<AgentResponse> {
        debug!(
            "SmartFallbackStrategy: Memory fallback для '{}'",
            context.message
        );

        Ok(AgentResponse::Chat(
            "Система памяти временно недоступна. Работаю без персистентного контекста.".to_string(),
        ))
    }
}

#[async_trait]
impl<L> FallbackStrategy for SmartFallbackStrategy<L>
where
    L: LlmServiceTrait,
{
    async fn handle_fallback(
        &self,
        context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse> {
        warn!("SmartFallbackStrategy: обработка ошибки с попыткой альтернативных подходов");
        debug!("Ошибка: {}", error);

        // Пробуем альтернативные подходы
        match self.try_alternative_approach(context, error).await {
            Ok(response) => {
                info!("SmartFallbackStrategy: успешно обработано через альтернативный подход");
                Ok(response)
            }
            Err(fallback_error) => {
                error!(
                    "SmartFallbackStrategy: альтернативный подход тоже не сработал: {}",
                    fallback_error
                );

                // Последняя попытка через LLM если доступен
                if let Some(ref llm) = self.llm_service {
                    match llm
                        .chat(&format!(
                            "Пользователь пытался: {}. Произошла ошибка: {}. Как лучше ответить?",
                            context.message, error
                        ))
                        .await
                    {
                        Ok(llm_response) => {
                            info!("SmartFallbackStrategy: получен ответ от LLM для fallback");
                            return Ok(AgentResponse::Chat(llm_response));
                        }
                        Err(_) => {
                            debug!("SmartFallbackStrategy: LLM тоже недоступен для fallback");
                        }
                    }
                }

                // Финальный fallback
                Ok(AgentResponse::Error(
                    "Извините, возникла техническая проблема. Попробуйте позже или обратитесь к поддержке.".to_string()
                ))
            }
        }
    }

    fn can_handle(&self, error: &anyhow::Error) -> bool {
        let error_string = error.to_string().to_lowercase();

        // Не обрабатываем критические системные ошибки
        !error_string.contains("panic")
            && !error_string.contains("segfault")
            && !error_string.contains("out of memory")
    }

    fn priority(&self) -> u8 {
        5 // Средний приоритет
    }
}

/// Circuit Breaker fallback стратегия
pub struct CircuitBreakerFallbackStrategy {
    #[allow(dead_code)] // Порог сбоев для circuit breaker
    failure_threshold: u32,
    recovery_timeout_seconds: u64,
}

impl CircuitBreakerFallbackStrategy {
    pub fn new(failure_threshold: u32, recovery_timeout_seconds: u64) -> Self {
        Self {
            failure_threshold,
            recovery_timeout_seconds,
        }
    }
}

#[async_trait]
impl FallbackStrategy for CircuitBreakerFallbackStrategy {
    async fn handle_fallback(
        &self,
        _context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse> {
        warn!("CircuitBreakerFallbackStrategy: обнаружен паттерн частых сбоев");
        debug!("Ошибка: {}", error);

        // Логика circuit breaker
        let error_string = error.to_string().to_lowercase();

        if error_string.contains("circuit") && error_string.contains("open") {
            info!(
                "CircuitBreakerFallbackStrategy: circuit breaker открыт, возвращаем быстрый ответ"
            );

            return Ok(AgentResponse::Error(format!(
                "Сервис временно недоступен (circuit breaker активен). Повторите попытку через {} секунд.",
                self.recovery_timeout_seconds
            )));
        }

        // Для других ошибок предлагаем режим деградации
        Ok(AgentResponse::Error(
            "Система работает в режиме ограниченной функциональности. Доступны только базовые операции.".to_string()
        ))
    }

    fn can_handle(&self, error: &anyhow::Error) -> bool {
        let error_string = error.to_string().to_lowercase();
        error_string.contains("circuit")
            || error_string.contains("timeout")
            || error_string.contains("overload")
    }

    fn priority(&self) -> u8 {
        8 // Высокий приоритет для circuit breaker ошибок
    }
}

/// Композитная fallback стратегия - объединяет несколько стратегий
pub struct CompositeFallbackStrategy {
    strategies: Vec<Box<dyn FallbackStrategy>>,
}

impl Default for CompositeFallbackStrategy {
    fn default() -> Self { Self::new() }
}

impl CompositeFallbackStrategy {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    pub fn add_strategy(mut self, strategy: Box<dyn FallbackStrategy>) -> Self {
        self.strategies.push(strategy);
        // Сортируем по приоритету (высший приоритет первым)
        self.strategies.sort_by_key(|b| std::cmp::Reverse(b.priority()));
        self
    }
}

#[async_trait]
impl FallbackStrategy for CompositeFallbackStrategy {
    async fn handle_fallback(
        &self,
        context: &RequestContext,
        error: &anyhow::Error,
    ) -> Result<AgentResponse> {
        debug!(
            "CompositeFallbackStrategy: пробуем {} стратегий",
            self.strategies.len()
        );

        for (i, strategy) in self.strategies.iter().enumerate() {
            if strategy.can_handle(error) {
                debug!(
                    "CompositeFallbackStrategy: пробуем стратегию {} (приоритет {})",
                    i,
                    strategy.priority()
                );

                match strategy.handle_fallback(context, error).await {
                    Ok(response) => {
                        info!(
                            "CompositeFallbackStrategy: стратегия {} успешно обработала ошибку",
                            i
                        );
                        return Ok(response);
                    }
                    Err(strategy_error) => {
                        warn!(
                            "CompositeFallbackStrategy: стратегия {} не смогла обработать: {}",
                            i, strategy_error
                        );
                        continue;
                    }
                }
            }
        }

        error!("CompositeFallbackStrategy: ни одна стратегия не смогла обработать ошибку");

        // Финальный fallback
        Ok(AgentResponse::Error(
            "Критическая ошибка системы. Обратитесь к администратору.".to_string(),
        ))
    }

    fn can_handle(&self, error: &anyhow::Error) -> bool {
        // Может обработать если хотя бы одна стратегия может
        self.strategies.iter().any(|s| s.can_handle(error))
    }

    fn priority(&self) -> u8 {
        // Приоритет как у самой высокоприоритетной стратегии
        self.strategies
            .iter()
            .map(|s| s.priority())
            .max()
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            session_id: "test_session".to_string(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_simple_fallback_network_error() {
        let strategy = SimpleFallbackStrategy::new();
        let context = create_test_context("test message");
        let error = anyhow::anyhow!("Network connection failed");

        let result = strategy.handle_fallback(&context, &error).await.unwrap();
        if let AgentResponse::Error(msg) = result {
            assert!(msg.contains("сетевым соединением"));
        } else {
            panic!("Expected error response");
        }
    }

    #[tokio::test]
    async fn test_simple_fallback_llm_error() {
        let strategy = SimpleFallbackStrategy::new();
        let context = create_test_context("test message");
        let error = anyhow::anyhow!("OpenAI API error");

        let result = strategy.handle_fallback(&context, &error).await.unwrap();
        if let AgentResponse::Error(msg) = result {
            assert!(msg.contains("AI сервис"));
        } else {
            panic!("Expected error response");
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_fallback() {
        let strategy = CircuitBreakerFallbackStrategy::new(5, 30);
        let context = create_test_context("test message");
        let error = anyhow::anyhow!("Circuit breaker is open");

        assert!(strategy.can_handle(&error));

        let result = strategy.handle_fallback(&context, &error).await.unwrap();
        if let AgentResponse::Error(msg) = result {
            assert!(msg.contains("circuit breaker"));
            assert!(msg.contains("30 секунд"));
        } else {
            panic!("Expected error response");
        }
    }

    #[tokio::test]
    async fn test_composite_fallback() {
        let mut composite = CompositeFallbackStrategy::new();
        composite = composite.add_strategy(Box::new(CircuitBreakerFallbackStrategy::new(5, 30)));
        composite = composite.add_strategy(Box::new(SimpleFallbackStrategy::new()));

        let context = create_test_context("test message");
        let error = anyhow::anyhow!("Circuit breaker is open");

        // Должен использовать circuit breaker стратегию (высший приоритет)
        let result = composite.handle_fallback(&context, &error).await.unwrap();
        if let AgentResponse::Error(msg) = result {
            assert!(msg.contains("circuit breaker"));
        } else {
            panic!("Expected error response");
        }
    }
}
