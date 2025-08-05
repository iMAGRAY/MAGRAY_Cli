//! Comprehensive tests for RequestRoutingService
//! 
//! Покрывает:
//! - Unit тесты для DefaultRequestRoutingService
//! - Mock'и для SmartRouter
//! - Business logic маршрутизации
//! - Resource estimation тестирование
//! - Security policies проверка
//! - Статистика и метрики

use cli::services::request_routing::{
    RequestRoutingService, DefaultRequestRoutingService, RoutingConfig, RoutingStats,
    RouteType, RoutingRecommendation, ResourceRequirements
};
use cli::services::types::{IntentDecision, RequestContext, AgentResponse, OperationResult};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tokio;
use chrono::Utc;
use std::collections::HashMap;

// @component: {"k":"T","id":"request_routing_service_tests","t":"Comprehensive request routing service tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","unit","mock","coverage","business_logic"]}

/// Mock SmartRouter для тестирования
pub struct MockSmartRouter {
    /// Предопределенные ответы для различных запросов
    responses: HashMap<String, Result<String>>,
    /// Задержка перед ответом (для timeout тестов)
    delay: Option<Duration>,
    /// Счетчик обращений
    call_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Симуляция ошибки для определенных запросов
    should_fail: bool,
}

impl MockSmartRouter {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        
        // Предустановленные ответы для тестов
        responses.insert(
            "покажи список файлов".to_string(),
            Ok("Файлы: test.rs, main.rs, lib.rs".to_string())
        );
        
        responses.insert(
            "создай файл test.txt".to_string(),
            Ok("Файл test.txt создан успешно".to_string())
        );
        
        responses.insert(
            "error_test".to_string(),
            Err(anyhow!("Simulated router error"))
        );
        
        Self {
            responses,
            delay: None,
            call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            should_fail: false,
        }
    }
    
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    
    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
    
    pub fn get_call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub fn add_response(&mut self, query: &str, response: Result<String>) {
        self.responses.insert(query.to_string(), response);
    }
    
    pub async fn process_smart_request(&self, message: &str) -> Result<String> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        
        if self.should_fail {
            return Err(anyhow!("Mock router configured to fail"));
        }
        
        self.responses.get(message)
            .cloned()
            .unwrap_or_else(|| Ok(format!("Default response for: {}", message)))
    }
}

fn create_test_context(message: &str) -> RequestContext {
    RequestContext {
        message: message.to_string(),
        user_id: Some("test_user".to_string()),
        session_id: Some("test_session".to_string()),
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    }
}

fn create_test_intent(action_type: &str, confidence: f64) -> IntentDecision {
    IntentDecision {
        action_type: action_type.to_string(),
        confidence,
        reasoning: "Test intent decision".to_string(),
        extracted_params: HashMap::new(),
    }
}

// Поскольку мы не можем напрямую создать SmartRouter в тестах,  
// создадим helper функцию для тестирования
fn create_test_service(mock_router: MockSmartRouter) -> DefaultRequestRoutingService {
    DefaultRequestRoutingService::new_for_testing(mock_router)
}

// Добавляем метод для тестирования в DefaultRequestRoutingService
impl DefaultRequestRoutingService {
    #[cfg(test)]
    pub fn new_for_testing(mock_router: MockSmartRouter) -> Self {
        Self {
            smart_router: TestSmartRouterWrapper::new(mock_router),
            stats: parking_lot::RwLock::new(RoutingStats::default()),
            config: RoutingConfig::default(),
        }
    }
    
    #[cfg(test)]
    pub fn with_config_for_testing(mock_router: MockSmartRouter, config: RoutingConfig) -> Self {
        Self {
            smart_router: TestSmartRouterWrapper::new(mock_router),
            stats: parking_lot::RwLock::new(RoutingStats::default()),
            config,
        }
    }
}

/// Wrapper для интеграции MockSmartRouter с реальным API
struct TestSmartRouterWrapper {
    mock: Arc<parking_lot::Mutex<MockSmartRouter>>,
}

impl TestSmartRouterWrapper {
    fn new(mock: MockSmartRouter) -> Self {
        Self {
            mock: Arc::new(parking_lot::Mutex::new(mock)),
        }
    }
    
    async fn process_smart_request(&self, message: &str) -> Result<String> {
        let mock = self.mock.lock();
        mock.process_smart_request(message).await
    }
}

#[test]
fn test_blocked_requests_detection() {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    // Проверяем заблокированные паттерны
    assert!(service.is_request_blocked("rm -rf /"));
    assert!(service.is_request_blocked("Please format c: drive"));
    assert!(service.is_request_blocked("delete system32 folder"));
    assert!(service.is_request_blocked("RM -RF /home"));  // Case insensitive
    
    // Проверяем разрешенные запросы
    assert!(!service.is_request_blocked("прочитай файл test.rs"));
    assert!(!service.is_request_blocked("покажи список файлов"));
    assert!(!service.is_request_blocked("создай новую папку"));
}

#[test]
fn test_route_type_determination() {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("прочитай файл test.rs");
    
    // Высокая уверенность в инструментах -> ToolExecution
    let intent = create_test_intent("tools", 0.9);
    let route_type = service.determine_route_type(&intent, &context);
    assert_eq!(route_type, RouteType::ToolExecution);
    
    // Высокая уверенность в чате -> DirectChat
    let intent = create_test_intent("chat", 0.9);
    let route_type = service.determine_route_type(&intent, &context);
    assert_eq!(route_type, RouteType::DirectChat);
    
    // Средняя уверенность в инструментах -> Hybrid
    let intent = create_test_intent("tools", 0.6);
    let route_type = service.determine_route_type(&intent, &context);
    assert_eq!(route_type, RouteType::Hybrid);
    
    // Низкая уверенность -> fallback
    let intent = create_test_intent("tools", 0.3);
    let route_type = service.determine_route_type(&intent, &context);
    assert_eq!(route_type, RouteType::ToolExecution); // Fallback to tools
    
    // Неизвестный тип -> Hybrid
    let intent = create_test_intent("unknown", 0.8);
    let route_type = service.determine_route_type(&intent, &context);
    assert_eq!(route_type, RouteType::Hybrid);
    
    // Заблокированный запрос -> Reject
    let blocked_context = create_test_context("rm -rf /");
    let intent = create_test_intent("tools", 0.9);
    let route_type = service.determine_route_type(&intent, &blocked_context);
    assert_eq!(route_type, RouteType::Reject);
}

#[test]
fn test_resource_estimation() {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("привет мир");
    
    // Проверяем оценку для DirectChat
    let chat_resources = service.estimate_resource_requirements(&RouteType::DirectChat, &context);
    assert_eq!(chat_resources.estimated_memory_mb, 50);
    assert_eq!(chat_resources.estimated_cpu_cores, 0.5);
    assert_eq!(chat_resources.requires_network, true);
    assert_eq!(chat_resources.requires_file_system, false);
    assert!(chat_resources.estimated_tokens.is_some());
    
    // Проверяем оценку для ToolExecution
    let tool_resources = service.estimate_resource_requirements(&RouteType::ToolExecution, &context);
    assert_eq!(tool_resources.estimated_memory_mb, 100);
    assert_eq!(tool_resources.estimated_cpu_cores, 1.0);
    assert_eq!(tool_resources.requires_network, true);
    assert_eq!(tool_resources.requires_file_system, true);
    
    // Проверяем оценку для Hybrid
    let hybrid_resources = service.estimate_resource_requirements(&RouteType::Hybrid, &context);
    assert_eq!(hybrid_resources.estimated_memory_mb, 150);
    assert_eq!(hybrid_resources.estimated_cpu_cores, 1.5);
    assert_eq!(hybrid_resources.requires_network, true);
    assert_eq!(hybrid_resources.requires_file_system, true);
    
    // Проверяем оценку для Reject
    let reject_resources = service.estimate_resource_requirements(&RouteType::Reject, &context);
    assert_eq!(reject_resources.estimated_memory_mb, 1);
    assert_eq!(reject_resources.estimated_cpu_cores, 0.01);
    assert_eq!(reject_resources.requires_network, false);
    assert_eq!(reject_resources.requires_file_system, false);
    assert!(reject_resources.estimated_tokens.is_none());
}

#[tokio::test]
async fn test_route_request_direct_chat() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("объясни что такое Rust");
    let intent = create_test_intent("chat", 0.9);
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_ok());
    if let Ok(AgentResponse::Chat(response)) = result.result {
        assert!(response.contains("ROUTE_TO_CHAT"));
        assert!(response.contains("объясни что такое Rust"));
    } else {
        panic!("Expected Chat response");
    }
    
    // Проверяем статистику
    let stats = service.get_routing_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_routes, 1);
    assert_eq!(*stats.route_distribution.get(&RouteType::DirectChat).unwrap_or(&0), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_route_request_tool_execution() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("покажи список файлов");
    let intent = create_test_intent("tools", 0.9);
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_ok());
    if let Ok(AgentResponse::ToolExecution(response)) = result.result {
        assert!(response.contains("Файлы: test.rs, main.rs, lib.rs"));
    } else {
        panic!("Expected ToolExecution response");
    }
    
    // Проверяем статистику
    let stats = service.get_routing_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_routes, 1);
    assert_eq!(*stats.route_distribution.get(&RouteType::ToolExecution).unwrap_or(&0), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_route_request_hybrid() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("покажи список файлов");
    let intent = create_test_intent("tools", 0.6); // Средняя уверенность -> Hybrid
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_ok());
    if let Ok(AgentResponse::ToolExecution(response)) = result.result {
        assert!(response.contains("HYBRID_RESULT"));
        assert!(response.contains("CHAT_FOLLOWUP"));
    } else {
        panic!("Expected ToolExecution response for hybrid");
    }
    
    let stats = service.get_routing_stats().await;
    assert_eq!(*stats.route_distribution.get(&RouteType::Hybrid).unwrap_or(&0), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_route_request_reject() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("rm -rf /");  // Заблокированный запрос
    let intent = create_test_intent("tools", 0.9);
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_err());
    if let Err(e) = result.result {
        assert!(e.to_string().contains("политике безопасности"));
    }
    
    let stats = service.get_routing_stats().await;
    assert_eq!(stats.rejected_routes, 1);
    assert_eq!(stats.failed_routes, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_tool_execution_error_handling() -> Result<()> {
    let mock_router = MockSmartRouter::new().with_failure();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("покажи список файлов");
    let intent = create_test_intent("tools", 0.9);
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_err());
    
    let stats = service.get_routing_stats().await;
    assert_eq!(stats.failed_routes, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_hybrid_fallback_to_chat() -> Result<()> {
    let mock_router = MockSmartRouter::new().with_failure();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("тест");
    let intent = create_test_intent("tools", 0.6); // Hybrid mode
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_ok());
    if let Ok(AgentResponse::Chat(response)) = result.result {
        assert!(response.contains("ROUTE_TO_CHAT"));
    } else {
        panic!("Expected Chat fallback in hybrid mode");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<()> {
    let mock_router = MockSmartRouter::new()
        .with_delay(Duration::from_secs(5)); // Дольше timeout'а
    
    let config = RoutingConfig {
        max_execution_time: Duration::from_millis(100),
        ..Default::default()
    };
    
    let service = DefaultRequestRoutingService::with_config_for_testing(mock_router, config);
    
    let context = create_test_context("покажи список файлов");
    let intent = create_test_intent("tools", 0.9);
    
    let result = service.route_request(&context, &intent).await?;
    
    assert!(result.result.is_err());
    if let Err(e) = result.result {
        assert!(e.to_string().contains("timeout"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_recommend_routing() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    let context = create_test_context("покажи список файлов");
    let intent = create_test_intent("tools", 0.9);
    
    let recommendation = service.recommend_routing(&context, &intent).await?;
    
    assert_eq!(recommendation.route_type, RouteType::ToolExecution);
    assert_eq!(recommendation.confidence, 0.9);
    assert!(recommendation.reasoning.contains("инструменты"));
    assert_eq!(recommendation.estimated_duration, Duration::from_secs(30));
    assert_eq!(recommendation.resource_requirements.estimated_memory_mb, 100);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_routing() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = Arc::new(create_test_service(mock_router));
    
    let mut handles = vec![];
    
    // Создаем 10 concurrent запросов
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let context = create_test_context(&format!("concurrent request {}", i));
            let intent = create_test_intent("chat", 0.8);
            service_clone.route_request(&context, &intent).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех запросов
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            if result.result.is_ok() {
                success_count += 1;
            }
        }
    }
    
    assert_eq!(success_count, 10);
    
    let stats = service.get_routing_stats().await;
    assert_eq!(stats.total_requests, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_tracking() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    // Выполняем запросы разных типов
    let chat_context = create_test_context("объясни Rust");
    let chat_intent = create_test_intent("chat", 0.9);
    service.route_request(&chat_context, &chat_intent).await?;
    
    let tool_context = create_test_context("покажи список файлов");
    let tool_intent = create_test_intent("tools", 0.9);
    service.route_request(&tool_context, &tool_intent).await?;
    
    let hybrid_context = create_test_context("анализируй");
    let hybrid_intent = create_test_intent("tools", 0.6); // Hybrid
    service.route_request(&hybrid_context, &hybrid_intent).await?;
    
    let reject_context = create_test_context("rm -rf /");
    let reject_intent = create_test_intent("tools", 0.9);
    service.route_request(&reject_context, &reject_intent).await?;
    
    let stats = service.get_routing_stats().await;
    
    assert_eq!(stats.total_requests, 4);
    assert_eq!(stats.successful_routes, 3);
    assert_eq!(stats.failed_routes, 1);
    assert_eq!(stats.rejected_routes, 1);
    
    assert_eq!(*stats.route_distribution.get(&RouteType::DirectChat).unwrap_or(&0), 1);
    assert_eq!(*stats.route_distribution.get(&RouteType::ToolExecution).unwrap_or(&0), 1);
    assert_eq!(*stats.route_distribution.get(&RouteType::Hybrid).unwrap_or(&0), 1);
    assert_eq!(*stats.route_distribution.get(&RouteType::Reject).unwrap_or(&0), 1);
    
    assert!(stats.avg_routing_time_ms > 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_reset() -> Result<()> {
    let mock_router = MockSmartRouter::new();
    let service = create_test_service(mock_router);
    
    // Выполняем несколько запросов
    let context = create_test_context("тест");
    let intent = create_test_intent("chat", 0.8);
    
    service.route_request(&context, &intent).await?;
    service.route_request(&context, &intent).await?;
    
    let stats_before = service.get_routing_stats().await;
    assert_eq!(stats_before.total_requests, 2);
    
    // Сбрасываем статистику
    service.reset_stats().await;
    
    let stats_after = service.get_routing_stats().await;
    assert_eq!(stats_after.total_requests, 0);
    assert_eq!(stats_after.successful_routes, 0);
    assert_eq!(stats_after.failed_routes, 0);
    assert_eq!(stats_after.rejected_routes, 0);
    assert_eq!(stats_after.avg_routing_time_ms, 0.0);
    assert!(stats_after.route_distribution.is_empty());
    
    Ok(())
}

#[test]
fn test_config_defaults() {
    let config = RoutingConfig::default();
    
    assert_eq!(config.min_confidence_direct, 0.7);
    assert_eq!(config.hybrid_threshold, 0.5);
    assert_eq!(config.max_execution_time, Duration::from_secs(120));
    assert_eq!(config.enable_smart_routing, true);
    assert!(config.blocked_patterns.contains(&"rm -rf".to_string()));
    assert!(config.blocked_patterns.contains(&"format c:".to_string()));
    assert!(config.blocked_patterns.contains(&"delete system32".to_string()));
}

#[test]
fn test_custom_config() {
    let custom_config = RoutingConfig {
        min_confidence_direct: 0.8,
        hybrid_threshold: 0.6,
        max_execution_time: Duration::from_secs(60),
        enable_smart_routing: false,
        blocked_patterns: vec!["sudo rm".to_string()],
    };
    
    let mock_router = MockSmartRouter::new();
    let service = DefaultRequestRoutingService::with_config_for_testing(mock_router, custom_config);
    
    assert!(service.is_request_blocked("sudo rm file"));
    assert!(!service.is_request_blocked("rm -rf /"));  // Not in custom patterns
}

#[test]
fn test_route_type_equality() {
    assert_eq!(RouteType::DirectChat, RouteType::DirectChat);
    assert_eq!(RouteType::ToolExecution, RouteType::ToolExecution);
    assert_eq!(RouteType::Hybrid, RouteType::Hybrid);
    assert_eq!(RouteType::Reject, RouteType::Reject);
    
    assert_ne!(RouteType::DirectChat, RouteType::ToolExecution);
    assert_ne!(RouteType::Hybrid, RouteType::Reject);
}

#[test]
fn test_resource_requirements_construction() {
    let requirements = ResourceRequirements {
        estimated_memory_mb: 256,
        estimated_cpu_cores: 2.0,
        requires_network: true,
        requires_file_system: false,
        estimated_tokens: Some(1000),
    };
    
    assert_eq!(requirements.estimated_memory_mb, 256);
    assert_eq!(requirements.estimated_cpu_cores, 2.0);
    assert_eq!(requirements.requires_network, true);
    assert_eq!(requirements.requires_file_system, false);
    assert_eq!(requirements.estimated_tokens, Some(1000));
}