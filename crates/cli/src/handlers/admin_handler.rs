//! Admin Handler - специализированный компонент для административных операций
//! 
//! Реализует Single Responsibility для admin commands
//! Интегрируется через DI с административными сервисами

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{info, debug, error};

use crate::agent_traits::{
    AdminServiceTrait, ComponentLifecycleTrait, CircuitBreakerTrait,
    RequestContext, AgentResponse, AdminResponse
};

// @component: {"k":"C","id":"admin_handler","t":"Specialized admin operations handler","m":{"cur":85,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","di_ready"]}
pub struct AdminHandler<A, C>
where
    A: AdminServiceTrait,
    C: CircuitBreakerTrait,
{
    admin_service: A,
    circuit_breaker: C,
    initialized: bool,
}

impl<A, C> AdminHandler<A, C>
where
    A: AdminServiceTrait,
    C: CircuitBreakerTrait,
{
    /// Создание нового AdminHandler через DI
    pub fn new(admin_service: A, circuit_breaker: C) -> Self {
        Self {
            admin_service,
            circuit_breaker,
            initialized: false,
        }
    }
    
    /// Обработка административных команд
    pub async fn handle_admin_request(&self, context: &RequestContext) -> Result<AgentResponse> {
        if !self.initialized {
            return Err(anyhow::anyhow!("AdminHandler не инициализирован"));
        }
        
        debug!("AdminHandler: обработка административной команды: {}", context.message);
        
        // Определяем тип административной команды
        let admin_type = self.classify_admin_request(&context.message);
        
        let response = match admin_type.as_str() {
            "system_stats" => {
                self.handle_system_stats().await?
            }
            "health_check" => {
                self.handle_health_check().await?
            }
            "performance_metrics" => {
                self.handle_performance_metrics().await?
            }
            "custom_command" => {
                self.handle_custom_command(context).await?
            }
            _ => {
                return Err(anyhow::anyhow!("Неизвестная административная команда"));
            }
        };
        
        info!("AdminHandler: административная команда выполнена успешно");
        Ok(AgentResponse::Admin(response))
    }
    
    /// Получение системной статистики
    async fn handle_system_stats(&self) -> Result<AdminResponse> {
        debug!("AdminHandler: получение системной статистики");
        
        self.circuit_breaker.execute(async {
            self.admin_service.get_system_stats().await
        }).await
    }
    
    /// Проверка здоровья системы
    async fn handle_health_check(&self) -> Result<AdminResponse> {
        debug!("AdminHandler: проверка здоровья системы");
        
        self.circuit_breaker.execute(async {
            self.admin_service.check_system_health().await
        }).await
    }
    
    /// Получение метрик производительности
    async fn handle_performance_metrics(&self) -> Result<AdminResponse> {
        debug!("AdminHandler: получение метрик производительности");
        
        self.circuit_breaker.execute(async {
            self.admin_service.get_performance_metrics().await
        }).await
    }
    
    /// Выполнение пользовательской административной команды
    async fn handle_custom_command(&self, context: &RequestContext) -> Result<AdminResponse> {
        debug!("AdminHandler: выполнение пользовательской команды");
        
        // Извлекаем команду и аргументы из контекста
        let (command, args) = self.parse_admin_command(&context.message);
        
        self.circuit_breaker.execute(async {
            self.admin_service.execute_admin_command(&command, &args).await
        }).await
    }
    
    /// Классификация типа административного запроса
    fn classify_admin_request(&self, message: &str) -> String {
        let message_lower = message.to_lowercase();
        
        if message_lower.contains("статистика") || message_lower.contains("stats") {
            "system_stats".to_string()
        } else if message_lower.contains("здоровье") || message_lower.contains("health") {
            "health_check".to_string()
        } else if message_lower.contains("метрики") || message_lower.contains("metrics") || 
                  message_lower.contains("производительность") || message_lower.contains("performance") {
            "performance_metrics".to_string()
        } else {
            "custom_command".to_string()
        }
    }
    
    /// Парсинг административной команды и аргументов
    fn parse_admin_command(&self, message: &str) -> (String, HashMap<String, String>) {
        let mut args = HashMap::new();
        
        // Простой парсинг команды (в production версии будет более сложный)
        let parts: Vec<&str> = message.split_whitespace().collect();
        
        let command = if parts.len() > 0 {
            parts[0].to_string()
        } else {
            "unknown".to_string()
        };
        
        // Добавляем аргументы
        for (i, part) in parts.iter().enumerate().skip(1) {
            args.insert(format!("arg{}", i), part.to_string());
        }
        
        (command, args)
    }
    
    /// Проверка возможности обработки запроса
    pub async fn can_handle(&self, context: &RequestContext) -> bool {
        if !self.initialized {
            return false;
        }
        
        let message_lower = context.message.to_lowercase();
        
        // Индикаторы административных команд
        let admin_indicators = [
            // Системная информация
            "статистика", "stats", "система", "system",
            "информация", "info", "сведения", "details",
            
            // Здоровье системы
            "здоровье", "health", "состояние", "status",
            "проверка", "check", "диагностика", "diagnostic",
            
            // Производительность
            "метрики", "metrics", "производительность", "performance",
            "скорость", "speed", "время", "time", "латентность", "latency",
            
            // Административные команды
            "админ", "admin", "управление", "management",
            "конфигурация", "config", "настройки", "settings",
        ];
        
        admin_indicators.iter().any(|&indicator| message_lower.contains(indicator))
    }
    
    /// Получение поддерживаемых команд
    pub fn get_supported_commands(&self) -> Vec<&'static str> {
        vec![
            "system_stats",
            "health_check",
            "performance_metrics",
            "config_get",
            "config_set",
            "restart_component",
            "shutdown_system",
        ]
    }
    
    /// Получение статистики использования
    pub fn get_usage_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("admin_commands_executed".to_string(), 0);
        stats.insert("health_checks_performed".to_string(), 0);
        stats.insert("stats_requests".to_string(), 0);
        stats.insert("metrics_requests".to_string(), 0);
        stats.insert("circuit_breaker_trips".to_string(), 0);
        stats
    }
    
    /// Проверка административных прав (в production версии)
    pub async fn check_admin_permissions(&self, _session_id: &str) -> Result<bool> {
        // В production версии здесь проверка прав доступа
        debug!("AdminHandler: проверка административных прав");
        Ok(true) // Временно разрешаем всем
    }
}

#[async_trait]
impl<A, C> ComponentLifecycleTrait for AdminHandler<A, C>
where
    A: AdminServiceTrait,
    C: CircuitBreakerTrait,
{
    async fn initialize(&self) -> Result<()> {
        info!("AdminHandler: инициализация начата");
        
        // Здесь можно добавить проверки админ сервиса
        // self.admin_service.health_check().await
        //     .map_err(|e| anyhow::anyhow!("Admin сервис недоступен: {}", e))?;
        
        info!("AdminHandler: инициализация завершена");
        Ok(())
    }
    
    async fn health_check(&self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("AdminHandler не инициализирован"));
        }
        
        // Проверяем состояние Circuit Breaker
        let breaker_state = self.circuit_breaker.get_state().await;
        if breaker_state == "Open" {
            return Err(anyhow::anyhow!("Circuit breaker открыт"));
        }
        
        debug!("AdminHandler: health check прошел успешно");
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("AdminHandler: начинаем graceful shutdown");
        
        // В production версии здесь будет:
        // - Завершение активных админ операций
        // - Сохранение логов и аудита
        // - Очистка административных сессий
        // - Отправка уведомлений о shutdown
        
        info!("AdminHandler: shutdown завершен");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock implementations для тестирования
    struct MockAdminService;
    
    #[async_trait]
    impl AdminServiceTrait for MockAdminService {
        async fn get_system_stats(&self) -> Result<AdminResponse> {
            Ok(AdminResponse::SystemStats("Mock system stats".to_string()))
        }
        
        async fn check_system_health(&self) -> Result<AdminResponse> {
            Ok(AdminResponse::HealthStatus("System healthy".to_string()))
        }
        
        async fn get_performance_metrics(&self) -> Result<AdminResponse> {
            Ok(AdminResponse::PerformanceMetrics("Mock metrics".to_string()))
        }
        
        async fn execute_admin_command(&self, command: &str, _args: &HashMap<String, String>) -> Result<AdminResponse> {
            Ok(AdminResponse::OperationResult(format!("Executed: {}", command)))
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
    async fn test_admin_handler_stats_detection() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let context = create_test_context("Покажи статистику системы");
        
        // Должен определить, что это admin запрос
        assert!(handler.can_handle(&context).await);
        
        // Должен правильно классифицировать тип
        assert_eq!(handler.classify_admin_request(&context.message), "system_stats");
    }
    
    #[tokio::test]
    async fn test_admin_handler_health_detection() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let context = create_test_context("Проверь здоровье системы");
        
        // Должен определить, что это admin запрос
        assert!(handler.can_handle(&context).await);
        
        // Должен правильно классифицировать тип
        assert_eq!(handler.classify_admin_request(&context.message), "health_check");
    }
    
    #[tokio::test]
    async fn test_admin_handler_metrics_detection() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let context = create_test_context("Покажи метрики производительности");
        
        // Должен определить, что это admin запрос
        assert!(handler.can_handle(&context).await);
        
        // Должен правильно классифицировать тип
        assert_eq!(handler.classify_admin_request(&context.message), "performance_metrics");
    }
    
    #[tokio::test]
    async fn test_admin_handler_chat_rejection() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let context = create_test_context("Привет, как дела?");
        
        // Должен определить, что это НЕ admin запрос
        assert!(!handler.can_handle(&context).await);
    }
    
    #[tokio::test]
    async fn test_command_parsing() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let (command, args) = handler.parse_admin_command("restart component memory_service");
        
        assert_eq!(command, "restart");
        assert_eq!(args.get("arg1"), Some(&"component".to_string()));
        assert_eq!(args.get("arg2"), Some(&"memory_service".to_string()));
    }
    
    #[tokio::test]
    async fn test_supported_commands() {
        let handler = AdminHandler::new(MockAdminService, MockCircuitBreaker);
        let commands = handler.get_supported_commands();
        
        assert!(commands.contains(&"system_stats"));
        assert!(commands.contains(&"health_check"));
        assert!(commands.contains(&"performance_metrics"));
    }
}