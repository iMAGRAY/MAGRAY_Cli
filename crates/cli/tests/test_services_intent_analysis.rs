//! Comprehensive tests for IntentAnalysisService
//! 
//! Покрывает:
//! - Unit тесты для DefaultIntentAnalysisService
//! - Mock'и для IntentAnalyzerAgent
//! - Error scenarios и edge cases
//! - Performance и timeout тестирование
//! - Статистика и метрики

use cli::services::intent_analysis::{
    IntentAnalysisService, DefaultIntentAnalysisService, IntentAnalysisConfig, IntentAnalysisStats
};
use cli::services::types::{IntentDecision, RequestContext};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Duration;
use tokio;
use chrono::Utc;
use std::collections::HashMap;


/// Mock IntentAnalyzerAgent для тестирования
pub struct MockIntentAnalyzerAgent {
    /// Предопределенные ответы для различных запросов
    responses: HashMap<String, Result<MockIntentDecision>>,
    /// Задержка перед ответом (для timeout тестов)
    delay: Option<Duration>,
    /// Счетчик обращений
    call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[derive(Debug, Clone)]
pub struct MockIntentDecision {
    pub action_type: String,
    pub confidence: f64,
    pub reasoning: String,
}

impl MockIntentAnalyzerAgent {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        
        // Предустановленные ответы для тестов
        responses.insert(
            "покажи список файлов".to_string(),
            Ok(MockIntentDecision {
                action_type: "tools".to_string(),
                confidence: 0.9,
                reasoning: "File listing detected".to_string(),
            })
        );
        
        responses.insert(
            "привет мир".to_string(),
            Ok(MockIntentDecision {
                action_type: "chat".to_string(),  
                confidence: 0.8,
                reasoning: "Greeting detected".to_string(),
            })
        );
        
        responses.insert(
            "error_test".to_string(),
            Err(anyhow!("Simulated ML error"))
        );
        
        Self {
            responses,
            delay: None,
            call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    
    pub fn get_call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    pub fn add_response(&mut self, query: &str, response: Result<MockIntentDecision>) {
        self.responses.insert(query.to_string(), response);
    }
    
    pub async fn analyze_intent(&self, message: &str) -> Result<MockIntentDecision> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        
        self.responses.get(message)
            .cloned()
            .unwrap_or_else(|| Ok(MockIntentDecision {
                action_type: "unknown".to_string(),
                confidence: 0.5,
                reasoning: "Default response".to_string(),
            }))
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

/// Wrapper для интеграции MockIntentAnalyzerAgent с реальным API
/// В production это была бы настоящая реализация IntentAnalyzerAgent
struct TestIntentAnalyzerWrapper {
    mock: Arc<parking_lot::Mutex<MockIntentAnalyzerAgent>>,
}

impl TestIntentAnalyzerWrapper {
    fn new(mock: MockIntentAnalyzerAgent) -> Self {
        Self {
            mock: Arc::new(parking_lot::Mutex::new(mock)),
        }
    }
    
    async fn analyze_intent(&self, message: &str) -> Result<llm::IntentDecision> {
        let mock_result = {
            let mock = self.mock.lock();
            mock.analyze_intent(message).await
        };
        
        match mock_result {
            Ok(mock_decision) => Ok(llm::IntentDecision {
                action_type: mock_decision.action_type,
                confidence: mock_decision.confidence,
                reasoning: mock_decision.reasoning,
            }),
            Err(e) => Err(e),
        }
    }
}

// Поскольку мы не можем напрямую создать IntentAnalyzerAgent в тестах,
// создадим helper функцию для тестирования
fn create_test_service(mock: MockIntentAnalyzerAgent) -> DefaultIntentAnalysisService {
    // В реальной реализации здесь был бы настоящий IntentAnalyzerAgent
    // Для тестов мы обходим это ограничение через прямую конструкцию
    DefaultIntentAnalysisService::new_for_testing(mock)
}

// Добавляем метод для тестирования в DefaultIntentAnalysisService
impl DefaultIntentAnalysisService {
    // Этот метод существует только для тестов
    #[cfg(test)]
    pub fn new_for_testing(mock: MockIntentAnalyzerAgent) -> Self {
        Self {
            intent_analyzer: TestIntentAnalyzerWrapper::new(mock),
            stats: parking_lot::RwLock::new(IntentAnalysisStats::default()),
            config: IntentAnalysisConfig::default(),
        }
    }
}

#[tokio::test]
async fn test_heuristic_tool_detection() {
    let mock = MockIntentAnalyzerAgent::new();
    let service = create_test_service(mock);
    
    // Тестируем определение инструментальных запросов
    let tool_decision = service.quick_heuristic("прочитай файл test.rs");
    assert_eq!(tool_decision.action_type, "tools");
    assert_eq!(tool_decision.confidence, 0.6);
    
    let tool_decision2 = service.quick_heuristic("покажи status git");
    assert_eq!(tool_decision2.action_type, "tools");
    
    let tool_decision3 = service.quick_heuristic("создай папку docs");
    assert_eq!(tool_decision3.action_type, "tools");
    
    // Тестируем определение чат запросов  
    let chat_decision = service.quick_heuristic("привет, как дела?");
    assert_eq!(chat_decision.action_type, "chat");
    assert_eq!(chat_decision.confidence, 0.6);
    
    let chat_decision2 = service.quick_heuristic("объясни концепцию dependency injection");
    assert_eq!(chat_decision2.action_type, "chat");
    
    let chat_decision3 = service.quick_heuristic("что такое Rust?");
    assert_eq!(chat_decision3.action_type, "chat");
}

#[tokio::test]
async fn test_analyze_intent_ml_success() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new();
    let service = create_test_service(mock);
    
    let context = create_test_context("покажи список файлов");
    let decision = service.analyze_intent(&context).await?;
    
    assert_eq!(decision.action_type, "tools");
    assert_eq!(decision.confidence, 0.9);
    assert_eq!(decision.reasoning, "File listing detected");
    
    // Проверяем что статистика обновилась
    let stats = service.get_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.ml_analysis_success, 1);
    assert_eq!(stats.ml_analysis_failures, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_analyze_intent_ml_failure_fallback() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new();
    let service = create_test_service(mock);
    
    let context = create_test_context("error_test");
    let decision = service.analyze_intent(&context).await?;
    
    // Должен использовать эвристику после ML ошибки
    assert_eq!(decision.confidence, 0.6); // Эвристическая уверенность
    
    // Проверяем статистику
    let stats = service.get_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.ml_analysis_failures, 1);
    assert_eq!(stats.heuristic_fallbacks, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_analyze_intent_timeout_fallback() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new()
        .with_delay(Duration::from_secs(15)); // Дольше timeout'а
    
    let config = IntentAnalysisConfig {
        ml_timeout: Duration::from_millis(100),
        min_confidence: 0.3,
        enable_heuristic_fallback: true,
    };
    
    let service = DefaultIntentAnalysisService::with_config_for_testing(mock, config);
    let context = create_test_context("любое сообщение");
    
    let decision = service.analyze_intent(&context).await?;
    
    // Должен использовать эвристику после timeout
    assert_eq!(decision.confidence, 0.6);
    
    let stats = service.get_stats().await;
    assert_eq!(stats.heuristic_fallbacks, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_low_confidence_fallback() -> Result<()> {
    let mut mock = MockIntentAnalyzerAgent::new();
    mock.add_response(
        "низкая уверенность",
        Ok(MockIntentDecision {
            action_type: "tools".to_string(),
            confidence: 0.1, // Ниже минимальной уверенности
            reasoning: "Low confidence decision".to_string(),
        })
    );
    
    let config = IntentAnalysisConfig {
        ml_timeout: Duration::from_secs(5),
        min_confidence: 0.3,
        enable_heuristic_fallback: true,
    };
    
    let service = DefaultIntentAnalysisService::with_config_for_testing(mock, config);
    let context = create_test_context("низкая уверенность");
    
    let decision = service.analyze_intent(&context).await?;
    
    // Должен использовать эвристику из-за низкой уверенности
    assert_eq!(decision.confidence, 0.6);
    
    Ok(())
}

#[tokio::test]
async fn test_confidence_distribution_tracking() -> Result<()> {
    let mut mock = MockIntentAnalyzerAgent::new();
    
    // Добавляем ответы с разной уверенностью
    mock.add_response("high", Ok(MockIntentDecision {
        action_type: "tools".to_string(),
        confidence: 0.9,
        reasoning: "High confidence".to_string(),
    }));
    
    mock.add_response("medium", Ok(MockIntentDecision {
        action_type: "chat".to_string(),
        confidence: 0.6,
        reasoning: "Medium confidence".to_string(),
    }));
    
    mock.add_response("low", Ok(MockIntentDecision {
        action_type: "tools".to_string(),
        confidence: 0.3,
        reasoning: "Low confidence".to_string(),
    }));
    
    let service = create_test_service(mock);
    
    // Выполняем запросы с разной уверенностью
    service.analyze_intent(&create_test_context("high")).await?;
    service.analyze_intent(&create_test_context("medium")).await?;
    service.analyze_intent(&create_test_context("low")).await?;
    
    let stats = service.get_stats().await;
    
    assert_eq!(stats.total_requests, 3);
    assert_eq!(*stats.confidence_distribution.get("high").unwrap_or(&0), 1);
    assert_eq!(*stats.confidence_distribution.get("medium").unwrap_or(&0), 1);
    assert_eq!(*stats.confidence_distribution.get("low").unwrap_or(&0), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new();
    let service = Arc::new(create_test_service(mock));
    
    let mut handles = vec![];
    
    // Создаем 10 concurrent запросов
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let context = create_test_context(&format!("concurrent request {}", i));
            service_clone.analyze_intent(&context).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех запросов
    let mut success_count = 0;
    for handle in handles {
        if handle.await.is_ok() {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 10);
    
    let stats = service.get_stats().await;
    assert_eq!(stats.total_requests, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_reset() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new();
    let service = create_test_service(mock);
    
    // Выполняем несколько запросов
    service.analyze_intent(&create_test_context("тест 1")).await?;
    service.analyze_intent(&create_test_context("тест 2")).await?;
    
    let stats_before = service.get_stats().await;
    assert_eq!(stats_before.total_requests, 2);
    
    // Сбрасываем статистику
    service.reset_stats().await;
    
    let stats_after = service.get_stats().await;
    assert_eq!(stats_after.total_requests, 0);
    assert_eq!(stats_after.ml_analysis_success, 0);
    assert_eq!(stats_after.ml_analysis_failures, 0);
    assert_eq!(stats_after.heuristic_fallbacks, 0);
    assert_eq!(stats_after.avg_response_time_ms, 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_average_response_time_calculation() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new()
        .with_delay(Duration::from_millis(50));
    
    let service = create_test_service(mock);
    
    // Выполняем несколько запросов
    for i in 0..5 {
        let context = create_test_context(&format!("запрос {}", i));
        service.analyze_intent(&context).await?;
    }
    
    let stats = service.get_stats().await;
    assert!(stats.avg_response_time_ms > 40.0); // Учитывая задержку в 50ms
    assert_eq!(stats.total_requests, 5);
    
    Ok(())
}

#[test]
fn test_config_defaults() {
    let config = IntentAnalysisConfig::default();
    
    assert_eq!(config.ml_timeout, Duration::from_secs(10));
    assert_eq!(config.min_confidence, 0.3);
    assert_eq!(config.enable_heuristic_fallback, true);
}

#[test]
fn test_intent_decision_construction() {
    let decision = IntentDecision {
        action_type: "test".to_string(),
        confidence: 0.8,
        reasoning: "test reasoning".to_string(),
        extracted_params: HashMap::new(),
    };
    
    assert_eq!(decision.action_type, "test");
    assert_eq!(decision.confidence, 0.8);
    assert_eq!(decision.reasoning, "test reasoning");
}

#[test]
fn test_request_context_construction() {
    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), "value".to_string());
    
    let context = RequestContext {
        message: "test message".to_string(),
        user_id: Some("user123".to_string()),
        session_id: Some("session456".to_string()),
        timestamp: Utc::now(),
        metadata,
    };
    
    assert_eq!(context.message, "test message");
    assert_eq!(context.user_id, Some("user123".to_string()));
    assert_eq!(context.session_id, Some("session456".to_string()));
    assert_eq!(context.metadata.get("key"), Some(&"value".to_string()));
}

// Добавляем метод для тестирования с кастомной конфигурацией
impl DefaultIntentAnalysisService {
    #[cfg(test)]
    pub fn with_config_for_testing(mock: MockIntentAnalyzerAgent, config: IntentAnalysisConfig) -> Self {
        Self {
            intent_analyzer: TestIntentAnalyzerWrapper::new(mock),
            stats: parking_lot::RwLock::new(IntentAnalysisStats::default()),
            config,
        }
    }
}

#[tokio::test]
async fn test_heuristic_fallback_disabled() -> Result<()> {
    let mut mock = MockIntentAnalyzerAgent::new();
    mock.add_response(
        "test",
        Ok(MockIntentDecision {
            action_type: "tools".to_string(),
            confidence: 0.1, // Низкая уверенность
            reasoning: "Low confidence decision".to_string(),
        })
    );
    
    let config = IntentAnalysisConfig {
        ml_timeout: Duration::from_secs(5),
        min_confidence: 0.3,
        enable_heuristic_fallback: false, // Отключаем fallback
    };
    
    let service = DefaultIntentAnalysisService::with_config_for_testing(mock, config);
    let context = create_test_context("test");
    
    let decision = service.analyze_intent(&context).await?;
    
    // Должен принять ML результат даже с низкой уверенностью
    assert_eq!(decision.confidence, 0.1);
    assert_eq!(decision.action_type, "tools");
    
    Ok(())
}

#[tokio::test] 
async fn test_edge_cases() -> Result<()> {
    let mock = MockIntentAnalyzerAgent::new();
    let service = create_test_service(mock);
    
    // Пустое сообщение
    let empty_decision = service.quick_heuristic("");
    assert_eq!(empty_decision.action_type, "chat");
    
    // Очень длинное сообщение
    let long_message = "a".repeat(10000);
    let long_decision = service.quick_heuristic(&long_message);
    assert!(long_decision.action_type == "chat" || long_decision.action_type == "tools");
    
    // Специальные символы
    let special_decision = service.quick_heuristic("@#$%^&*()");
    assert_eq!(special_decision.action_type, "chat");
    
    // Unicode символы
    let unicode_decision = service.quick_heuristic("🚀 покажи файлы 📁");
    assert_eq!(unicode_decision.action_type, "tools");
    
    Ok(())
}