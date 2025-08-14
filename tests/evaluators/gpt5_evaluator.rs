use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use tokio::time::{timeout, Duration};
use super::super::integration::human_like_testing::{TestResult, TestScenario};

/// GPT-5 nano API Evaluator - оценивает качество ответов MAGRAY CLI через OpenAI API
pub struct Gpt5Evaluator {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    timeout_duration: Duration,
}

/// Результат оценки GPT-5 nano
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub scenario_id: String,
    pub overall_score: f64,
    pub scores: EvaluationScores,
    pub feedback: EvaluationFeedback,
    pub metadata: EvaluationMetadata,
}

/// Детальные оценки по критериям
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationScores {
    pub relevance: f64,           // Релевантность ответа (1-10)
    pub technical_accuracy: f64,  // Техническая точность (1-10)
    pub completeness: f64,        // Полнота решения (1-10)
    pub practicality: f64,        // Практичность рекомендаций (1-10)
    pub overall_quality: f64,     // Общее качество (1-10)
}

/// Развернутая обратная связь от GPT-5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationFeedback {
    pub strengths: Vec<String>,    // Сильные стороны ответа
    pub weaknesses: Vec<String>,   // Слабые места
    pub suggestions: Vec<String>,  // Предложения по улучшению
    pub summary: String,           // Краткое резюме оценки
}

/// Метаданные оценки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationMetadata {
    pub evaluation_time_ms: u64,
    pub model_used: String,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub timestamp: String,
}

/// Структура для API запроса к OpenAI
#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f64,
    max_tokens: u32,
    response_format: OpenAIResponseFormat,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAIResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

/// Структура ответа от OpenAI API
#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Структура для парсинга JSON ответа от GPT-5
#[derive(Deserialize)]
struct GPTEvaluationResponse {
    relevance: f64,
    technical_accuracy: f64,
    completeness: f64,
    practicality: f64,
    overall_quality: f64,
    strengths: Vec<String>,
    weaknesses: Vec<String>,
    suggestions: Vec<String>,
    summary: String,
}

impl Gpt5Evaluator {
    /// Создает новый GPT-5 Evaluator
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "gpt-4".to_string(), // Используем GPT-4 как лучшую доступную модель
            base_url: "https://api.openai.com/v1/chat/completions".to_string(),
            timeout_duration: Duration::from_secs(60),
        }
    }

    /// Загружает API ключ из переменных окружения
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY environment variable not found")?;
        
        Ok(Self::new(api_key))
    }

    /// Загружает API ключ из .env файла
    pub fn from_env_file<P: AsRef<std::path::Path>>(env_path: P) -> Result<Self> {
        let content = std::fs::read_to_string(env_path)?;
        
        for line in content.lines() {
            if line.starts_with("OPENAI_API_KEY=") {
                let api_key = line.trim_start_matches("OPENAI_API_KEY=").trim();
                return Ok(Self::new(api_key.to_string()));
            }
        }
        
        Err(anyhow::anyhow!("OPENAI_API_KEY not found in .env file"))
    }

    /// Устанавливает модель для использования
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Устанавливает timeout для API запросов
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    /// Оценивает результат тестирования с помощью GPT-5 nano
    pub async fn evaluate_response(
        &self, 
        scenario: &TestScenario, 
        test_result: &TestResult
    ) -> Result<EvaluationResult> {
        
        println!("🤖 Evaluating response for scenario: {}", scenario.name);
        
        let start_time = std::time::Instant::now();
        
        // Создаем промпт для оценки
        let evaluation_prompt = self.create_evaluation_prompt(scenario, test_result);
        
        // Выполняем API запрос к OpenAI
        let gpt_response = self.call_openai_api(&evaluation_prompt).await?;
        
        // Парсим JSON ответ
        let evaluation_data: GPTEvaluationResponse = serde_json::from_str(&gpt_response)
            .context("Failed to parse GPT evaluation response as JSON")?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Вычисляем общую оценку как среднее арифметическое
        let overall_score = (
            evaluation_data.relevance +
            evaluation_data.technical_accuracy +
            evaluation_data.completeness +
            evaluation_data.practicality +
            evaluation_data.overall_quality
        ) / 5.0;

        let result = EvaluationResult {
            scenario_id: scenario.id.clone(),
            overall_score,
            scores: EvaluationScores {
                relevance: evaluation_data.relevance,
                technical_accuracy: evaluation_data.technical_accuracy,
                completeness: evaluation_data.completeness,
                practicality: evaluation_data.practicality,
                overall_quality: evaluation_data.overall_quality,
            },
            feedback: EvaluationFeedback {
                strengths: evaluation_data.strengths,
                weaknesses: evaluation_data.weaknesses,
                suggestions: evaluation_data.suggestions,
                summary: evaluation_data.summary,
            },
            metadata: EvaluationMetadata {
                evaluation_time_ms: execution_time,
                model_used: self.model.clone(),
                prompt_tokens: None, // TODO: extract from API response
                completion_tokens: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        };

        println!("✅ Evaluation completed. Overall score: {:.1}/10", overall_score);
        
        Ok(result)
    }

    /// Создает промпт для оценки ответа
    fn create_evaluation_prompt(&self, scenario: &TestScenario, test_result: &TestResult) -> String {
        format!(r#"
Вы являетесь экспертом по оценке качества AI-ассистентов. Оцените ответ MAGRAY CLI на пользовательский запрос.

## КОНТЕКСТ ТЕСТИРОВАНИЯ:
Сценарий: {scenario_name}
Тип задачи: {scenario_type}
Критерии оценки: {criteria}

## ПОЛЬЗОВАТЕЛЬСКИЙ ЗАПРОС:
{user_input}

## ОТВЕТ MAGRAY CLI:
{cli_output}

## РЕЗУЛЬТАТ ВЫПОЛНЕНИЯ:
Успешно: {success}
Время выполнения: {execution_time}мс
{error_info}

## ИНСТРУКЦИИ ПО ОЦЕНКЕ:

Оцените ответ по следующим критериям (шкала 1-10, где 10 - отлично):

1. **Релевантность** (relevance): Насколько ответ соответствует запросу пользователя
2. **Техническая точность** (technical_accuracy): Корректность технических деталей и рекомендаций
3. **Полнота решения** (completeness): Покрывает ли ответ все аспекты запроса
4. **Практичность** (practicality): Можно ли применить предложенные решения на практике
5. **Общее качество** (overall_quality): Общее впечатление от качества ответа

Также предоставьте:
- **Сильные стороны** (strengths): Что сделано хорошо
- **Слабые места** (weaknesses): Что можно улучшить
- **Предложения** (suggestions): Конкретные рекомендации по улучшению
- **Резюме** (summary): Краткое заключение на русском языке

Ответьте строго в формате JSON:
{{
    "relevance": 0.0,
    "technical_accuracy": 0.0,
    "completeness": 0.0,
    "practicality": 0.0,
    "overall_quality": 0.0,
    "strengths": ["сильная сторона 1", "сильная сторона 2"],
    "weaknesses": ["слабость 1", "слабость 2"],
    "suggestions": ["предложение 1", "предложение 2"],
    "summary": "Краткое резюме оценки на русском языке"
}}
"#,
            scenario_name = scenario.name,
            scenario_type = scenario.expected_type,
            criteria = scenario.evaluation_criteria.join(", "),
            user_input = scenario.input,
            cli_output = test_result.output,
            success = test_result.success,
            execution_time = test_result.execution_time_ms,
            error_info = test_result.error_message.as_ref()
                .map(|err| format!("Ошибка: {}", err))
                .unwrap_or_default()
        )
    }

    /// Выполняет API запрос к OpenAI
    async fn call_openai_api(&self, prompt: &str) -> Result<String> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: "Вы эксперт по оценке качества AI-ассистентов. Отвечайте только в формате JSON.".to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: 0.3,
            max_tokens: 2000,
            response_format: OpenAIResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let response = timeout(
            self.timeout_duration,
            self.client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
        ).await
            .context("OpenAI API request timeout")?
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI API error: {}", error_text));
        }

        let api_response: OpenAIResponse = response.json().await
            .context("Failed to parse OpenAI API response")?;

        if api_response.choices.is_empty() {
            return Err(anyhow::anyhow!("No choices in OpenAI API response"));
        }

        Ok(api_response.choices[0].message.content.clone())
    }

    /// Оценивает результаты батча тестов
    pub async fn evaluate_test_batch(
        &self,
        scenarios: &[TestScenario],
        test_results: &[TestResult]
    ) -> Result<Vec<EvaluationResult>> {
        
        println!("🔍 Starting batch evaluation of {} test results", test_results.len());
        
        let mut evaluation_results = Vec::new();
        
        for (i, test_result) in test_results.iter().enumerate() {
            // Находим соответствующий сценарий
            let scenario = scenarios.iter()
                .find(|s| s.id == test_result.scenario_id)
                .context(format!("Scenario not found for result: {}", test_result.scenario_id))?;

            println!("📊 Evaluating {}/{}: {}", i + 1, test_results.len(), scenario.name);
            
            match self.evaluate_response(scenario, test_result).await {
                Ok(evaluation) => {
                    evaluation_results.push(evaluation);
                }
                Err(e) => {
                    eprintln!("❌ Failed to evaluate scenario '{}': {}", scenario.name, e);
                    // Создаем fallback результат при ошибке
                    evaluation_results.push(EvaluationResult {
                        scenario_id: scenario.id.clone(),
                        overall_score: 0.0,
                        scores: EvaluationScores {
                            relevance: 0.0,
                            technical_accuracy: 0.0,
                            completeness: 0.0,
                            practicality: 0.0,
                            overall_quality: 0.0,
                        },
                        feedback: EvaluationFeedback {
                            strengths: vec![],
                            weaknesses: vec!["Evaluation failed due to technical error".to_string()],
                            suggestions: vec!["Review evaluation system configuration".to_string()],
                            summary: format!("Evaluation failed: {}", e),
                        },
                        metadata: EvaluationMetadata {
                            evaluation_time_ms: 0,
                            model_used: self.model.clone(),
                            prompt_tokens: None,
                            completion_tokens: None,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        },
                    });
                }
            }

            // Небольшая пауза между запросами для соблюдения rate limits
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        println!("✅ Batch evaluation completed: {}/{} successful", 
               evaluation_results.iter().filter(|r| r.overall_score > 0.0).count(),
               evaluation_results.len());

        Ok(evaluation_results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluator_creation() {
        let evaluator = Gpt5Evaluator::new("test_key".to_string());
        assert_eq!(evaluator.api_key, "test_key");
        assert_eq!(evaluator.model, "gpt-4");
    }

    #[test]
    fn test_prompt_creation() {
        let evaluator = Gpt5Evaluator::new("test_key".to_string());
        
        let scenario = TestScenario {
            id: "test".to_string(),
            name: "Test Scenario".to_string(),
            input: "test input".to_string(),
            expected_type: "simple_response".to_string(),
            timeout_seconds: 30,
            evaluation_criteria: vec!["test_criterion".to_string()],
        };
        
        let test_result = TestResult {
            scenario_id: "test".to_string(),
            input: "test input".to_string(),
            output: "test output".to_string(),
            execution_time_ms: 1000,
            success: true,
            error_message: None,
            timestamp: "2025-01-01T00:00:00Z".to_string(),
        };
        
        let prompt = evaluator.create_evaluation_prompt(&scenario, &test_result);
        assert!(prompt.contains("test input"));
        assert!(prompt.contains("test output"));
        assert!(prompt.contains("Test Scenario"));
    }
}