//! Intent Decision Strategies - различные стратегии для определения намерений пользователя
//!
//! Реализует Strategy pattern для декомпозиции логики принятия решений

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::agent_traits::{
    IntentDecision, IntentDecisionStrategy, LlmServiceTrait, RequestContext,
};

// ============================================================================
// ОСНОВНЫЕ INTENT STRATEGIES
// ============================================================================

/// Эвристическая стратегия на основе ключевых слов
pub struct HeuristicIntentStrategy {
    confidence_threshold: f32,
}

impl HeuristicIntentStrategy {
    pub fn new(confidence_threshold: f32) -> Self {
        Self {
            confidence_threshold,
        }
    }

    /// Анализ на основе ключевых слов с весами
    fn analyze_keywords(&self, message: &str) -> (String, f32, HashMap<String, String>) {
        let message_lower = message.to_lowercase();
        let mut scores = HashMap::new();

        // Весовые коэффициенты для разных типов индикаторов
        let chat_patterns = [
            ("привет", 0.9),
            ("здравствуй", 0.9),
            ("как дела", 0.8),
            ("что нового", 0.7),
            ("расскажи", 0.6),
            ("объясни", 0.5),
            ("как", 0.4),
            ("что", 0.3),
            ("почему", 0.5),
            ("зачем", 0.5),
        ];

        let tools_patterns = [
            ("файл", 1.0),
            ("file", 1.0),
            ("папка", 0.9),
            ("folder", 0.9),
            ("git", 1.0),
            ("commit", 0.9),
            ("status", 0.8),
            ("создай", 0.9),
            ("create", 0.9),
            ("прочитай", 0.9),
            ("read", 0.9),
            ("запиши", 0.9),
            ("write", 0.9),
            ("выполни", 0.8),
            ("execute", 0.8),
            ("команда", 0.8),
            ("command", 0.8),
            ("shell", 0.8),
        ];

        let memory_patterns = [
            ("память", 1.0),
            ("memory", 1.0),
            ("сохрани", 0.9),
            ("save", 0.9),
            ("найди", 0.8),
            ("search", 0.8),
            ("запомни", 0.9),
            ("remember", 0.9),
            ("promotion", 0.9),
            ("слои", 0.8),
            ("layers", 0.8),
        ];

        let admin_patterns = [
            ("статистика", 1.0),
            ("stats", 1.0),
            ("здоровье", 0.9),
            ("health", 0.9),
            ("метрики", 1.0),
            ("metrics", 1.0),
            ("система", 0.7),
            ("system", 0.7),
            ("производительность", 0.9),
            ("performance", 0.9),
        ];

        // Подсчет очков для каждого типа
        scores.insert(
            "chat".to_string(),
            self.calculate_score(&message_lower, &chat_patterns),
        );
        scores.insert(
            "tools".to_string(),
            self.calculate_score(&message_lower, &tools_patterns),
        );
        scores.insert(
            "memory".to_string(),
            self.calculate_score(&message_lower, &memory_patterns),
        );
        scores.insert(
            "admin".to_string(),
            self.calculate_score(&message_lower, &admin_patterns),
        );

        // Находим лучший матч
        let (best_type, best_score) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.clone(), *v))
            .unwrap_or(("chat".to_string(), 0.0));

        let mut context = HashMap::new();
        context.insert(
            "analysis_method".to_string(),
            "heuristic_keywords".to_string(),
        );
        context.insert("all_scores".to_string(), format!("{:?}", scores));

        (best_type, best_score, context)
    }

    fn calculate_score(&self, message: &str, patterns: &[(&str, f32)]) -> f32 {
        let mut total_score = 0.0;
        let mut matches = 0;

        for (pattern, weight) in patterns {
            if message.contains(pattern) {
                total_score += weight;
                matches += 1;
            }
        }

        // Нормализуем счет с учетом количества совпадений
        if matches > 0 {
            total_score / patterns.len() as f32 * (1.0 + (matches as f32 * 0.1))
        } else {
            0.0
        }
    }
}

#[async_trait]
impl IntentDecisionStrategy for HeuristicIntentStrategy {
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision> {
        debug!(
            "HeuristicIntentStrategy: анализ намерения для '{}'",
            context.message
        );

        let (action_type, confidence, analysis_context) = self.analyze_keywords(&context.message);

        // Применяем пороговое значение
        let final_confidence = if confidence >= self.confidence_threshold {
            confidence
        } else {
            // Если уверенность низкая, по умолчанию считаем chat
            0.3
        };

        let final_action_type = if confidence >= self.confidence_threshold {
            action_type
        } else {
            "chat".to_string()
        };

        info!(
            "HeuristicIntentStrategy: результат '{}' с уверенностью {:.2}",
            final_action_type, final_confidence
        );

        Ok(IntentDecision {
            action_type: final_action_type,
            confidence: final_confidence,
            context: Some(analysis_context),
        })
    }

    fn strategy_name(&self) -> &'static str {
        "heuristic_keywords"
    }
}

/// LLM-based стратегия с использованием языковой модели
pub struct LlmIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    llm_service: L,
    system_prompt: String,
}

impl<L> LlmIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    pub fn new(llm_service: L) -> Self {
        let system_prompt = r#"
Ты - классификатор намерений пользователя. Проанализируй сообщение и определи тип намерения.

Доступные типы:
- "chat": обычный разговор, вопросы, объяснения
- "tools": выполнение команд, работа с файлами, git операции
- "memory": операции с памятью (сохранение, поиск, promotion)
- "admin": административные запросы (статистика, здоровье системы, метрики)

Ответь ТОЛЬКО в формате JSON:
{
  "action_type": "тип",
  "confidence": 0.95,
  "reasoning": "краткое объяснение"
}
        "#
        .to_string();

        Self {
            llm_service,
            system_prompt,
        }
    }
}

#[async_trait]
impl<L> IntentDecisionStrategy for LlmIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision> {
        debug!(
            "LlmIntentStrategy: анализ намерения через LLM для '{}'",
            context.message
        );

        let prompt = format!(
            "{}\n\nСообщение пользователя: {}",
            self.system_prompt, context.message
        );

        let response = self.llm_service.chat(&prompt).await?;

        // Парсим JSON ответ
        let parsed: serde_json::Value = serde_json::from_str(&response)
            .map_err(|e| anyhow::anyhow!("Ошибка парсинга LLM ответа: {}", e))?;

        let action_type = parsed
            .get("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("chat")
            .to_string();

        let confidence = parsed
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        let reasoning = parsed
            .get("reasoning")
            .and_then(|v| v.as_str())
            .unwrap_or("No reasoning provided")
            .to_string();

        let mut analysis_context = HashMap::new();
        analysis_context.insert(
            "analysis_method".to_string(),
            "llm_classification".to_string(),
        );
        analysis_context.insert("reasoning".to_string(), reasoning);

        info!(
            "LlmIntentStrategy: результат '{}' с уверенностью {:.2}",
            action_type, confidence
        );

        Ok(IntentDecision {
            action_type,
            confidence,
            context: Some(analysis_context),
        })
    }

    fn strategy_name(&self) -> &'static str {
        "llm_classification"
    }
}

/// Комбинированная стратегия - сочетает эвристику и LLM
pub struct HybridIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    heuristic: HeuristicIntentStrategy,
    llm: LlmIntentStrategy<L>,
    heuristic_threshold: f32,
}

impl<L> HybridIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    pub fn new(llm_service: L, heuristic_threshold: f32) -> Self {
        Self {
            heuristic: HeuristicIntentStrategy::new(0.7),
            llm: LlmIntentStrategy::new(llm_service),
            heuristic_threshold,
        }
    }
}

#[async_trait]
impl<L> IntentDecisionStrategy for HybridIntentStrategy<L>
where
    L: LlmServiceTrait,
{
    async fn analyze_intent(&self, context: &RequestContext) -> Result<IntentDecision> {
        debug!("HybridIntentStrategy: анализ намерения через гибридную стратегию");

        // Сначала пробуем эвристику
        let heuristic_result = self.heuristic.analyze_intent(context).await?;

        // Если эвристика дает высокую уверенность, используем её
        if heuristic_result.confidence >= self.heuristic_threshold {
            info!(
                "HybridIntentStrategy: используем результат эвристики (уверенность: {:.2})",
                heuristic_result.confidence
            );
            return Ok(heuristic_result);
        }

        // Иначе обращаемся к LLM для более точного анализа
        debug!(
            "HybridIntentStrategy: эвристика дала низкую уверенность ({:.2}), используем LLM",
            heuristic_result.confidence
        );

        let mut llm_result = self.llm.analyze_intent(context).await?;

        // Добавляем информацию о том, что использовали оба метода
        if let Some(ref mut ctx) = llm_result.context {
            ctx.insert("hybrid_analysis".to_string(), "true".to_string());
            ctx.insert(
                "heuristic_confidence".to_string(),
                heuristic_result.confidence.to_string(),
            );
            ctx.insert("heuristic_result".to_string(), heuristic_result.action_type);
        }

        info!(
            "HybridIntentStrategy: финальный результат '{}' с уверенностью {:.2}",
            llm_result.action_type, llm_result.confidence
        );

        Ok(llm_result)
    }

    fn strategy_name(&self) -> &'static str {
        "hybrid_heuristic_llm"
    }
}

#[cfg(all(test, not(feature = "minimal")))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Mock LLM service для тестирования
    struct MockLlmService;

    #[async_trait]
    impl LlmServiceTrait for MockLlmService {
        async fn chat(&self, message: &str) -> Result<String> {
            if message.contains("tools") || message.contains("git") {
                Ok("tools:0.9".to_string())
            } else {
                Ok("chat:0.7".to_string())
            }
        }

        async fn chat_with_context(
            &self,
            message: &str,
            _context: &HashMap<String, String>,
        ) -> Result<String> {
            self.chat(message).await
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
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
    async fn test_heuristic_strategy_tools() {
        let strategy = HeuristicIntentStrategy::new(0.5);
        let context = create_test_context("Прочитай файл test.txt");

        let result = strategy.analyze_intent(&context).await.unwrap();
        assert_eq!(result.action_type, "tools");
        assert!(result.confidence > 0.5);
    }

    #[tokio::test]
    async fn test_heuristic_strategy_chat() {
        let strategy = HeuristicIntentStrategy::new(0.5);
        let context = create_test_context("Привет, как дела?");

        let result = strategy.analyze_intent(&context).await.unwrap();
        assert_eq!(result.action_type, "chat");
    }

    #[tokio::test]
    async fn test_llm_strategy() {
        let strategy = LlmIntentStrategy::new(MockLlmService);
        let context = create_test_context("Test message");

        let result = strategy.analyze_intent(&context).await.unwrap();
        assert_eq!(result.action_type, "tools");
        assert_eq!(result.confidence, 0.9);
    }

    #[tokio::test]
    async fn test_hybrid_strategy_high_confidence() {
        let strategy = HybridIntentStrategy::new(MockLlmService, 0.6);
        let context = create_test_context("git status"); // High confidence for heuristic

        let result = strategy.analyze_intent(&context).await.unwrap();
        assert_eq!(result.action_type, "tools");
        // Должен использовать эвристику
        assert!(result
            .context
            .as_ref()
            .unwrap()
            .get("analysis_method")
            .unwrap()
            .contains("heuristic"));
    }

    #[tokio::test]
    async fn test_hybrid_strategy_low_confidence() {
        let strategy = HybridIntentStrategy::new(MockLlmService, 0.9); // Высокий порог
        let context = create_test_context("Неопределенное сообщение");

        let result = strategy.analyze_intent(&context).await.unwrap();
        // Должен использовать LLM из-за низкой уверенности эвристики
        assert!(result
            .context
            .as_ref()
            .unwrap()
            .contains_key("hybrid_analysis"));
    }
}
