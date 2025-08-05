//! Memory Handler - специализированный компонент для управления памятью
//! 
//! Реализует Single Responsibility для memory operations
//! Интегрируется через DI с системой памяти

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{info, debug, error};

use crate::agent_traits::{
    MemoryManagementTrait, ComponentLifecycleTrait, CircuitBreakerTrait,
    RequestContext, AgentResponse
};

// @component: {"k":"C","id":"memory_handler","t":"Specialized memory management handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"]}
pub struct MemoryHandler<M, C>
where
    M: MemoryManagementTrait,
    C: CircuitBreakerTrait,
{
    memory_service: M,
    circuit_breaker: C,
    initialized: bool,
}

impl<M, C> MemoryHandler<M, C>
where
    M: MemoryManagementTrait,
    C: CircuitBreakerTrait,
{
    /// Создание нового MemoryHandler через DI
    pub fn new(memory_service: M, circuit_breaker: C) -> Self {
        Self {
            memory_service,
            circuit_breaker,
            initialized: false,
        }
    }
    
    /// Сохранение пользовательского сообщения в память
    pub async fn store_user_message(&self, context: &RequestContext) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        debug!("MemoryHandler: сохранение сообщения в память");
        
        // Используем Circuit Breaker для защиты от сбоев памяти
        self.circuit_breaker.execute(async {
            self.memory_service.store_message(&context.message, &context.metadata).await
        }).await?;
        
        info!("MemoryHandler: сообщение успешно сохранено в память");
        Ok(AgentResponse::ToolExecution("Сообщение сохранено в память".to_string()))
    }
    
    /// Поиск релевантной информации в памяти
    pub async fn search_memory(&self, query: &str, limit: usize) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        debug!("MemoryHandler: поиск в памяти по запросу: {}", query);
        
        let results = self.circuit_breaker.execute(async {
            self.memory_service.search_memory(query, limit).await
        }).await?;
        
        let formatted_results = if results.is_empty() {
            "Релевантная информация не найдена".to_string()
        } else {
            format!("Найдено {} результатов:\n{}", results.len(), results.join("\n---\n"))
        };
        
        info!("MemoryHandler: поиск завершен, найдено {} результатов", results.len());
        Ok(AgentResponse::ToolExecution(formatted_results))
    }
    
    /// Запуск процесса продвижения между слоями памяти
    pub async fn run_memory_promotion(&self) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        debug!("MemoryHandler: запуск процесса promotion");
        
        let result = self.circuit_breaker.execute(async {
            self.memory_service.run_promotion().await
        }).await?;
        
        info!("MemoryHandler: процесс promotion завершен");
        Ok(AgentResponse::ToolExecution(result))
    }
    
    /// Получение статистики системы памяти
    pub async fn get_memory_statistics(&self) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        debug!("MemoryHandler: получение статистики памяти");
        
        let stats = self.circuit_breaker.execute(async {
            self.memory_service.get_memory_stats().await
        }).await?;
        
        info!("MemoryHandler: статистика получена");
        Ok(AgentResponse::ToolExecution(stats))
    }
    
    /// Проверка возможности обработки запроса
    pub async fn can_handle(&self, context: &RequestContext) -> bool {
        if !self.initialized {
            return false;
        }
        
        let message_lower = context.message.to_lowercase();
        
        // Индикаторы memory операций
        let memory_indicators = [
            // Основные операции
            "память", "memory", "сохрани", "save", "store",
            "найди", "search", "поиск", "ищи",
            
            // Специализированные операции
            "promotion", "продвижение", "слои", "layers",
            "статистика памяти", "memory stats", "память статистика",
            
            // Контекстные индикаторы
            "запомни", "remember", "вспомни", "recall",
            "забудь", "forget", "удали из памяти", "delete from memory",
        ];
        
        memory_indicators.iter().any(|&indicator| message_lower.contains(indicator))
    }
    
    /// Получение поддерживаемых операций
    pub fn get_supported_operations(&self) -> Vec<&'static str> {
        vec![
            "store_message",
            "search_memory",
            "run_promotion", 
            "get_statistics",
            "health_check",
        ]
    }
    
    /// Получение статистики использования
    pub fn get_usage_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("messages_stored".to_string(), 0);
        stats.insert("searches_performed".to_string(), 0);
        stats.insert("promotions_run".to_string(), 0);
        stats.insert("avg_search_time_ms".to_string(), 0);
        stats.insert("circuit_breaker_trips".to_string(), 0);
        stats
    }
    
    /// Проверка доступности памяти
    pub async fn check_memory_health(&self) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        debug!("MemoryHandler: проверка здоровья памяти");
        
        self.circuit_breaker.execute(async {
            self.memory_service.health_check().await
        }).await?;
        
        Ok("Система памяти функционирует нормально".to_string())
    }
}

#[async_trait]
impl<M, C> ComponentLifecycleTrait for MemoryHandler<M, C>
where
    M: MemoryManagementTrait,
    C: CircuitBreakerTrait,
{
    async fn initialize(&self) -> Result<()> {
        info!("MemoryHandler: инициализация начата");
        
        // Проверяем доступность memory сервиса
        self.memory_service.health_check().await
            .map_err(|e| anyhow::anyhow!("Memory сервис недоступен: {}", e))?;
        
        info!("MemoryHandler: инициализация завершена");
        Ok(())
    }
    
    async fn health_check(&self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("MemoryHandler не инициализирован"));
        }
        
        // Проверяем все зависимости
        self.memory_service.health_check().await?;
        
        // Проверяем состояние Circuit Breaker
        let breaker_state = self.circuit_breaker.get_state().await;
        if breaker_state == "Open" {
            return Err(anyhow::anyhow!("Circuit breaker открыт"));
        }
        
        debug!("MemoryHandler: health check прошел успешно");
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("MemoryHandler: начинаем graceful shutdown");
        
        // В production версии здесь будет:
        // - Завершение активных memory операций
        // - Сохранение кэшей и метрик
        // - Flush pending operations
        // - Очистка ресурсов
        
        info!("MemoryHandler: shutdown завершен");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock implementations для тестирования
    struct MockMemoryService;
    
    #[async_trait]
    impl MemoryManagementTrait for MockMemoryService {
        async fn store_message(&self, message: &str, _context: &HashMap<String, String>) -> Result<()> {
            debug!("MockMemoryService: storing message: {}", message);
            Ok(())
        }
        
        async fn search_memory(&self, query: &str, limit: usize) -> Result<Vec<String>> {
            Ok(vec![format!("Mock result for '{}' (limit: {})", query, limit)])
        }
        
        async fn run_promotion(&self) -> Result<String> {
            Ok("Mock promotion completed".to_string())
        }
        
        async fn get_memory_stats(&self) -> Result<String> {
            Ok("Mock memory statistics".to_string())
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
    async fn test_memory_handler_store_detection() {
        let handler = MemoryHandler::new(MockMemoryService, MockCircuitBreaker);
        let context = create_test_context("Сохрани это в память");
        
        // Должен определить, что это memory запрос
        assert!(handler.can_handle(&context).await);
    }
    
    #[tokio::test]
    async fn test_memory_handler_search_detection() {
        let handler = MemoryHandler::new(MockMemoryService, MockCircuitBreaker);
        let context = create_test_context("Найди в памяти информацию о проекте");
        
        // Должен определить, что это memory запрос
        assert!(handler.can_handle(&context).await);
    }
    
    #[tokio::test]
    async fn test_memory_handler_promotion_detection() {
        let handler = MemoryHandler::new(MockMemoryService, MockCircuitBreaker);
        let context = create_test_context("Запусти promotion слоев памяти");
        
        // Должен определить, что это memory запрос
        assert!(handler.can_handle(&context).await);
    }
    
    #[tokio::test]
    async fn test_memory_handler_chat_rejection() {
        let handler = MemoryHandler::new(MockMemoryService, MockCircuitBreaker);
        let context = create_test_context("Привет, как дела?");
        
        // Должен определить, что это НЕ memory запрос
        assert!(!handler.can_handle(&context).await);
    }
    
    #[tokio::test]
    async fn test_supported_operations() {
        let handler = MemoryHandler::new(MockMemoryService, MockCircuitBreaker);
        let operations = handler.get_supported_operations();
        
        assert!(operations.contains(&"store_message"));
        assert!(operations.contains(&"search_memory"));
        assert!(operations.contains(&"run_promotion"));
        assert!(operations.contains(&"get_statistics"));
    }
}