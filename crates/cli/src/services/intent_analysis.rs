//! Intent Analysis Service - анализ намерений пользователя
//!
//! Выделенный сервис для анализа того, что пользователь хочет сделать:
//! - Чат с LLM
//! - Выполнение инструментов (tools)
//! - Смешанные операции
//!
//! Использует как ML-based анализ через IntentAnalyzerAgent,
//! так и fallback эвристики для надёжности.

use super::types::{IntentDecision, RequestContext};
use anyhow::Result;
use llm::IntentAnalyzerAgent;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Trait для сервиса анализа намерений
#[async_trait::async_trait]
pub trait IntentAnalysisService: Send + Sync {
    /// Анализировать намерение пользователя из сообщения
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision>;

    /// Быстрая эвристическая проверка без ML (для fallback)
    fn quick_heuristic(&self, message: &str) -> IntentDecision;

    /// Получить статистику работы сервиса
    async fn get_stats(&self) -> IntentAnalysisStats;

    /// Сбросить статистику (для тестов)
    async fn reset_stats(&self);
}

/// Статистика работы сервиса анализа намерений
#[derive(Debug, Clone)]
pub struct IntentAnalysisStats {
    pub total_requests: u64,
    pub ml_analysis_success: u64,
    pub ml_analysis_failures: u64,
    pub heuristic_fallbacks: u64,
    pub avg_response_time_ms: f64,
    pub confidence_distribution: std::collections::HashMap<String, u64>, // "high", "medium", "low"
}

/// Реализация сервиса анализа намерений по умолчанию
pub struct DefaultIntentAnalysisService {
    /// ML-based агент для анализа намерений
    intent_analyzer: IntentAnalyzerAgent,

    /// Статистика работы
    stats: parking_lot::RwLock<IntentAnalysisStats>,

    /// Конфигурация таймаутов
    config: IntentAnalysisConfig,
}

/// Конфигурация сервиса анализа намерений
#[derive(Debug, Clone)]
pub struct IntentAnalysisConfig {
    /// Таймаут для ML анализа
    pub ml_timeout: Duration,

    /// Минимальная уверенность для принятия ML решения
    pub min_confidence: f64,

    /// Включить ли fallback на эвристики
    pub enable_heuristic_fallback: bool,
}

impl Default for IntentAnalysisConfig {
    fn default() -> Self {
        Self {
            ml_timeout: Duration::from_secs(10),
            min_confidence: 0.3,
            enable_heuristic_fallback: true,
        }
    }
}

impl DefaultIntentAnalysisService {
    /// Создать новый экземпляр сервиса
    pub fn new(intent_analyzer: IntentAnalyzerAgent) -> Self {
        Self::with_config(intent_analyzer, IntentAnalysisConfig::default())
    }

    /// Создать экземпляр с кастомной конфигурацией
    pub fn with_config(intent_analyzer: IntentAnalyzerAgent, config: IntentAnalysisConfig) -> Self {
        Self {
            intent_analyzer,
            stats: parking_lot::RwLock::new(IntentAnalysisStats::default()),
            config,
        }
    }

    /// Простая эвристика для определения типа действия
    fn simple_heuristic_internal(&self, message: &str) -> bool {
        let message_lower = message.to_lowercase();
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
            "установи",
            "install",
            "запусти",
            "run",
            "выполни",
            "execute",
        ];

        tool_indicators
            .iter()
            .any(|&indicator| message_lower.contains(indicator))
    }

    /// Обновить статистику
    fn update_stats(&self, success: bool, duration: Duration, confidence: f64, from_ml: bool) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;

        if from_ml {
            if success {
                stats.ml_analysis_success += 1;
            } else {
                stats.ml_analysis_failures += 1;
                stats.heuristic_fallbacks += 1;
            }
        } else {
            stats.heuristic_fallbacks += 1;
        }

        // Обновляем среднее время
        let duration_ms = duration.as_millis() as f64;
        let total = stats.total_requests as f64;
        stats.avg_response_time_ms =
            ((stats.avg_response_time_ms * (total - 1.0)) + duration_ms) / total;

        // Распределение уверенности
        let confidence_bucket = if confidence >= 0.8 {
            "high"
        } else if confidence >= 0.5 {
            "medium"
        } else {
            "low"
        };

        *stats
            .confidence_distribution
            .entry(confidence_bucket.to_string())
            .or_insert(0) += 1;
    }
}

#[async_trait::async_trait]
impl IntentAnalysisService for DefaultIntentAnalysisService {
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision> {
        use std::time::Instant;
        let start_time = Instant::now();

        debug!("🔍 Анализ намерения для сообщения: {}", context.message);

        // Пытаемся ML анализ с timeout
        let ml_future = self.intent_analyzer.analyze_intent(&context.message);
        let ml_result = timeout(self.config.ml_timeout, ml_future).await;

        let decision = match ml_result {
            Ok(Ok(llm_decision)) => {
                // Конвертируем из llm::IntentDecision в наш IntentDecision
                let decision = IntentDecision {
                    action_type: llm_decision.action_type,
                    confidence: llm_decision.confidence as f64,
                    reasoning: llm_decision.reasoning,
                    extracted_params: std::collections::HashMap::new(), // TODO: извлечение параметров
                };

                debug!(
                    "✅ ML анализ успешен: {} (confidence: {:.2})",
                    decision.action_type, decision.confidence
                );

                self.update_stats(true, start_time.elapsed(), decision.confidence, true);
                decision
            }
            Ok(Err(e)) => {
                warn!("⚠️ ML анализ failed: {}, используем эвристику", e);
                let decision = self.quick_heuristic(&context.message);
                self.update_stats(false, start_time.elapsed(), decision.confidence, false);
                decision
            }
            Err(_) => {
                warn!("⚠️ ML анализ timeout, используем эвристику");
                let decision = self.quick_heuristic(&context.message);
                self.update_stats(false, start_time.elapsed(), decision.confidence, false);
                decision
            }
        };

        // Проверяем минимальную уверенность
        if decision.confidence < self.config.min_confidence && self.config.enable_heuristic_fallback
        {
            warn!(
                "⚠️ Низкая уверенность ({:.2}), используем эвристику",
                decision.confidence
            );
            let heuristic_decision = self.quick_heuristic(&context.message);
            return Ok(heuristic_decision);
        }

        Ok(decision)
    }

    fn quick_heuristic(&self, message: &str) -> IntentDecision {
        let is_tool_request = self.simple_heuristic_internal(message);

        IntentDecision {
            action_type: if is_tool_request {
                "tools".to_string()
            } else {
                "chat".to_string()
            },
            confidence: 0.6, // Средняя уверенность для эвристики
            reasoning: "Heuristic-based decision".to_string(),
            extracted_params: std::collections::HashMap::new(),
        }
    }

    async fn get_stats(&self) -> IntentAnalysisStats {
        let stats = self.stats.read();
        stats.clone()
    }

    async fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = IntentAnalysisStats::default();
        debug!("🔄 Intent analysis stats reset");
    }
}

impl Default for IntentAnalysisStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            ml_analysis_success: 0,
            ml_analysis_failures: 0,
            heuristic_fallbacks: 0,
            avg_response_time_ms: 0.0,
            confidence_distribution: std::collections::HashMap::new(),
        }
    }
}

/// Factory функция для DI контейнера
pub fn create_intent_analysis_service(
    intent_analyzer: IntentAnalyzerAgent,
) -> Arc<dyn IntentAnalysisService> {
    Arc::new(DefaultIntentAnalysisService::new(intent_analyzer))
}

/// Factory функция с кастомной конфигурацией
pub fn create_intent_analysis_service_with_config(
    intent_analyzer: IntentAnalyzerAgent,
    config: IntentAnalysisConfig,
) -> Arc<dyn IntentAnalysisService> {
    Arc::new(DefaultIntentAnalysisService::with_config(
        intent_analyzer,
        config,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // Mock IntentAnalyzerAgent для тестов
    struct MockIntentAnalyzer;

    impl MockIntentAnalyzer {
        fn create() -> IntentAnalyzerAgent {
            // В реальных тестах это был бы mock
            // TODO: создать правильный mock
            unimplemented!("Mock implementation needed")
        }
    }

    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            user_id: Some("test_user".to_string()),
            session_id: Some("test_session".to_string()),
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_heuristic_tool_detection() {
        // Создаём минимальный экземпляр для тестирования эвристики
        let service = DefaultIntentAnalysisService {
            intent_analyzer: MockIntentAnalyzer::create(), // Не будет использоваться в этом тесте
            stats: parking_lot::RwLock::new(IntentAnalysisStats::default()),
            config: IntentAnalysisConfig::default(),
        };

        // Тестируем определение инструментальных запросов
        assert!(service.simple_heuristic_internal("прочитай файл test.rs"));
        assert!(service.simple_heuristic_internal("покажи status git"));
        assert!(service.simple_heuristic_internal("создай папку docs"));

        // Тестируем определение чат запросов
        assert!(!service.simple_heuristic_internal("привет, как дела?"));
        assert!(!service.simple_heuristic_internal("объясни концепцию dependency injection"));
        assert!(!service.simple_heuristic_internal("что такое Rust?"));
    }

    #[test]
    fn test_quick_heuristic() {
        let service = DefaultIntentAnalysisService {
            intent_analyzer: MockIntentAnalyzer::create(),
            stats: parking_lot::RwLock::new(IntentAnalysisStats::default()),
            config: IntentAnalysisConfig::default(),
        };

        let tool_decision = service.quick_heuristic("покажи список файлов");
        assert_eq!(tool_decision.action_type, "tools");
        assert_eq!(tool_decision.confidence, 0.6);

        let chat_decision = service.quick_heuristic("привет мир");
        assert_eq!(chat_decision.action_type, "chat");
        assert_eq!(chat_decision.confidence, 0.6);
    }
}
