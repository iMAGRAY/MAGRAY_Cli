use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info};

pub mod agents;
mod circuit_breaker;
mod cost_optimizer;
mod integration_test;
mod multi_provider;
pub mod providers;

pub use agents::*;
pub use circuit_breaker::*;
pub use cost_optimizer::*;
pub use multi_provider::*;
pub use providers::*;

#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI {
        api_key: String,
        model: String,
    },
    Anthropic {
        api_key: String,
        model: String,
    },
    Local {
        url: String,
        model: String,
    },
    Ollama {
        url: String,
        model: String,
    },
    LMStudio {
        url: String,
        model: String,
    },
    Azure {
        endpoint: String,
        api_key: String,
        model: String,
    },
    Groq {
        api_key: String,
        model: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Local,
    Ollama,
    LMStudio,
    Azure,
    Groq,
}

#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f32,
    pub total_cost: f32,
    pub last_error: Option<String>,
    pub circuit_breaker_state: CircuitBreakerState,
}

impl Default for ProviderStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_latency_ms: 0.0,
            total_cost: 0.0,
            last_error: None,
            circuit_breaker_state: CircuitBreakerState::Closed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskComplexity {
    pub tokens: u32,
    pub complexity: ComplexityLevel,
    pub priority: Priority,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComplexityLevel {
    Simple,  // Basic questions, simple tasks
    Medium,  // Code review, analysis
    Complex, // Architecture design, complex reasoning
    Expert,  // Advanced technical tasks
}

#[derive(Debug, Clone, PartialEq)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

impl CompletionRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            max_tokens: None,
            temperature: None,
            system_prompt: None,
        }
    }

    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }
}

#[derive(Clone)]
pub struct LlmClient {
    provider: LlmProvider,
    client: reqwest::Client,
    max_tokens: u32,
    temperature: f32,
    orchestrator: Option<Arc<MultiProviderLlmOrchestrator>>,
}

// OpenAI API types
#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIMessage,
}

// Anthropic API types
#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicContent {
    text: String,
}

impl LlmClient {
    pub fn new(provider: LlmProvider, max_tokens: u32, temperature: f32) -> Self {
        Self {
            provider,
            client: reqwest::Client::new(),
            max_tokens,
            temperature,
            orchestrator: None,
        }
    }

    /// Create a new client with multi-provider orchestration
    pub fn new_multi_provider(providers: Vec<LlmProvider>, daily_budget: Option<f32>) -> Self {
        let orchestrator = Arc::new(MultiProviderLlmOrchestrator::new(
            providers.clone(),
            daily_budget,
        ));
        Self {
            provider: providers[0].clone(), // Fallback provider
            client: reqwest::Client::new(),
            max_tokens: 1000,
            temperature: 0.7,
            orchestrator: Some(orchestrator),
        }
    }

    /// Check if multi-provider mode is enabled
    pub fn is_multi_provider(&self) -> bool {
        self.orchestrator.is_some()
    }

    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok(); // Загружаем .env если есть

        let provider_type = env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let max_tokens = env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<u32>()
            .unwrap_or(1000);
        let temperature = env::var("TEMPERATURE")
            .unwrap_or_else(|_| "0.7".to_string())
            .parse::<f32>()
            .unwrap_or(0.7);

        let provider = match provider_type.as_str() {
            "openai" => {
                let api_key = env::var("OPENAI_API_KEY")
                    .map_err(|_| anyhow!("OPENAI_API_KEY не установлен в .env"))?;
                let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
                LlmProvider::OpenAI { api_key, model }
            }
            "anthropic" => {
                let api_key = env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| anyhow!("ANTHROPIC_API_KEY не установлен в .env"))?;
                let model = env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-haiku-20240307".to_string());
                LlmProvider::Anthropic { api_key, model }
            }
            "local" => {
                let url = env::var("LOCAL_LLM_URL")
                    .unwrap_or_else(|_| "http://localhost:1234/v1".to_string());
                let model = env::var("LOCAL_LLM_MODEL")
                    .unwrap_or_else(|_| "llama-3.2-3b-instruct".to_string());
                LlmProvider::Local { url, model }
            }
            _ => return Err(anyhow!("Неподдерживаемый LLM_PROVIDER: {}", provider_type)),
        };

        Ok(Self {
            provider,
            client: reqwest::Client::new(),
            max_tokens,
            temperature,
            orchestrator: None,
        })
    }

    pub async fn complete(&self, request: CompletionRequest) -> Result<String> {
        // Use orchestrator if available
        if let Some(orchestrator) = &self.orchestrator {
            info!("🎯 Using multi-provider orchestration for request");
            return orchestrator.complete_smart(request).await;
        }

        // Fallback to single provider mode
        info!(
            "🔧 Using single provider mode: {}",
            Self::get_provider_name(&self.provider)
        );
        let message = if let Some(system) = &request.system_prompt {
            format!("{}\n\n{}", system, request.prompt)
        } else {
            request.prompt.clone()
        };

        let max_tokens = request.max_tokens.unwrap_or(self.max_tokens);
        let temperature = request.temperature.unwrap_or(self.temperature);

        // Используем значения из request
        let self_with_overrides = Self {
            provider: self.provider.clone(),
            client: self.client.clone(),
            max_tokens,
            temperature,
            orchestrator: None, // Don't pass orchestrator to avoid recursion
        };

        self_with_overrides.chat_internal(&message).await
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<String> {
        // Преобразуем сообщения в один промпт для простоты
        let prompt = messages
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        self.chat_internal(&prompt).await
    }

    // Для обратной совместимости с агентами
    pub async fn chat_simple(&self, message: &str) -> Result<String> {
        self.chat_internal(message).await
    }

    async fn chat_internal(&self, message: &str) -> Result<String> {
        match &self.provider {
            LlmProvider::OpenAI { api_key, model } => {
                self.openai_chat(api_key, model, message).await
            }
            LlmProvider::Anthropic { api_key, model } => {
                self.anthropic_chat(api_key, model, message).await
            }
            LlmProvider::Local { url, model } => self.local_chat(url, model, message).await,
            LlmProvider::Ollama { url, model } => self.local_chat(url, model, message).await,
            LlmProvider::LMStudio { url, model } => self.local_chat(url, model, message).await,
            LlmProvider::Azure {
                endpoint,
                api_key,
                model,
            } => self.azure_chat(endpoint, api_key, model, message).await,
            LlmProvider::Groq { api_key, model } => self.groq_chat(api_key, model, message).await,
        }
    }

    async fn openai_chat(&self, api_key: &str, model: &str, message: &str) -> Result<String> {
        let request = OpenAIChatRequest {
            model: model.to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };

        info!("🚀 Отправляю запрос в OpenAI: {}", model);
        debug!("Текст запроса: {}", message);

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("OpenAI API ошибка: {}", error_text);
            return Err(anyhow!("OpenAI API ошибка: {}", error_text));
        }

        let chat_response: OpenAIChatResponse = response.json().await?;

        if let Some(choice) = chat_response.choices.first() {
            info!("✅ Получен ответ от OpenAI");
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("Пустой ответ от OpenAI"))
        }
    }

    async fn anthropic_chat(&self, api_key: &str, model: &str, message: &str) -> Result<String> {
        let request = AnthropicRequest {
            model: model.to_string(),
            max_tokens: self.max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            temperature: Some(self.temperature),
        };

        info!("🚀 Отправляю запрос в Anthropic: {}", model);
        debug!("Текст запроса: {}", message);

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Anthropic API ошибка: {}", error_text);
            return Err(anyhow!("Anthropic API ошибка: {}", error_text));
        }

        let chat_response: AnthropicResponse = response.json().await?;

        if let Some(content) = chat_response.content.first() {
            info!("✅ Получен ответ от Anthropic");
            Ok(content.text.clone())
        } else {
            Err(anyhow!("Пустой ответ от Anthropic"))
        }
    }

    async fn local_chat(&self, url: &str, model: &str, message: &str) -> Result<String> {
        let request = OpenAIChatRequest {
            model: model.to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };

        info!(
            "🚀 Отправляю запрос в локальную модель: {} -> {}",
            url, model
        );
        debug!("Текст запроса: {}", message);

        // Нормализуем базовый endpoint: убираем завершающее "/" и необязательный "/v1"
        let mut base = url.trim_end_matches('/');
        if base.ends_with("/v1") {
            base = &base[..base.len() - 3]; // отрезаем "/v1"
        }

        // 1) Попытка OpenAI-совместимого эндпоинта
        let endpoint_oa = format!("{}/chat/completions", base);
        let resp_oa = self
            .client
            .post(&endpoint_oa)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if resp_oa.status().is_success() {
            let chat_response: OpenAIChatResponse = resp_oa.json().await?;
            if let Some(choice) = chat_response.choices.first() {
                info!("✅ Получен ответ от локальной модели (OpenAI совместимый)");
                return Ok(choice.message.content.clone());
            } else {
                // Приводим сообщение к формату, ожидаемому тестами (содержит "Пустой ответ")
                return Err(anyhow!("Пустой ответ от локальной модели"));
            }
        }

        // 2) Fallback: Anthropic-совместимый эндпоинт /v1/messages
        let anth_request = AnthropicRequest {
            model: model.to_string(),
            max_tokens: self.max_tokens,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            temperature: Some(self.temperature),
        };
        let endpoint_anth = format!("{}/v1/messages", base);
        let resp_anth = self
            .client
            .post(&endpoint_anth)
            .header("Content-Type", "application/json")
            .json(&anth_request)
            .send()
            .await?;

        if !resp_anth.status().is_success() {
            let error_text = resp_anth.text().await.unwrap_or_default();
            error!("Локальная LLM ошибка (fallback Anthropic): {}", error_text);
            return Err(anyhow!("Локальная LLM ошибка: {}", error_text));
        }

        let chat_response: AnthropicResponse = resp_anth.json().await?;
        if let Some(content) = chat_response.content.first() {
            info!("✅ Получен ответ от локальной модели (Anthropic совместимый)");
            Ok(content.text.clone())
        } else {
            // Сообщение совпадает по подстроке с ожидаемым в тестах
            Err(anyhow!("Пустой ответ от локальной модели"))
        }
    }

    async fn azure_chat(
        &self,
        endpoint: &str,
        api_key: &str,
        model: &str,
        message: &str,
    ) -> Result<String> {
        let request = OpenAIChatRequest {
            model: model.to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };

        info!(
            "🚀 Отправляю запрос в Azure OpenAI: {} -> {}",
            endpoint, model
        );
        debug!("Текст запроса: {}", message);

        let response = self
            .client
            .post(format!(
                "{}/openai/deployments/{}/chat/completions?api-version=2023-12-01-preview",
                endpoint, model
            ))
            .header("api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Azure OpenAI ошибка: {}", error_text);
            return Err(anyhow!("Azure OpenAI ошибка: {}", error_text));
        }

        let chat_response: OpenAIChatResponse = response.json().await?;

        if let Some(choice) = chat_response.choices.first() {
            info!("✅ Получен ответ от Azure OpenAI");
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("Пустой ответ от Azure OpenAI"))
        }
    }

    async fn groq_chat(&self, api_key: &str, model: &str, message: &str) -> Result<String> {
        let request = OpenAIChatRequest {
            model: model.to_string(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            max_tokens: Some(self.max_tokens),
            temperature: Some(self.temperature),
        };

        info!("🚀 Отправляю запрос в Groq: {}", model);
        debug!("Текст запроса: {}", message);

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Groq API ошибка: {}", error_text);
            return Err(anyhow!("Groq API ошибка: {}", error_text));
        }

        let chat_response: OpenAIChatResponse = response.json().await?;

        if let Some(choice) = chat_response.choices.first() {
            info!("✅ Получен ответ от Groq");
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("Пустой ответ от Groq"))
        }
    }

    /// Get provider name for display
    fn get_provider_name(provider: &LlmProvider) -> String {
        match provider {
            LlmProvider::OpenAI { model, .. } => format!("OpenAI ({})", model),
            LlmProvider::Anthropic { model, .. } => format!("Anthropic ({})", model),
            LlmProvider::Local { model, .. } => format!("Local ({})", model),
            LlmProvider::Ollama { model, .. } => format!("Ollama ({})", model),
            LlmProvider::LMStudio { model, .. } => format!("LM Studio ({})", model),
            LlmProvider::Azure { model, .. } => format!("Azure ({})", model),
            LlmProvider::Groq { model, .. } => format!("Groq ({})", model),
        }
    }

    /// Get status report (for multi-provider mode)
    pub async fn get_status_report(&self) -> Option<String> {
        if let Some(orchestrator) = &self.orchestrator {
            Some(orchestrator.get_status_report().await)
        } else {
            None
        }
    }

    /// Create from environment with multi-provider support
    pub fn from_env_multi() -> Result<Self> {
        dotenv::dotenv().ok();

        let mut providers = Vec::new();

        // Try to add OpenAI if configured
        if let Ok(api_key) = env::var("OPENAI_API_KEY") {
            let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
            providers.push(LlmProvider::OpenAI { api_key, model });
            info!("✅ Added OpenAI provider");
        }

        // Try to add Anthropic if configured
        if let Ok(api_key) = env::var("ANTHROPIC_API_KEY") {
            let model = env::var("ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-haiku-20240307".to_string());
            providers.push(LlmProvider::Anthropic { api_key, model });
            info!("✅ Added Anthropic provider");
        }

        // Try to add Groq if configured
        if let Ok(api_key) = env::var("GROQ_API_KEY") {
            let model =
                env::var("GROQ_MODEL").unwrap_or_else(|_| "llama-3.1-8b-instant".to_string());
            providers.push(LlmProvider::Groq { api_key, model });
            info!("✅ Added Groq provider");
        }

        // Try to add local providers
        if let Ok(url) = env::var("OLLAMA_URL") {
            let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
            providers.push(LlmProvider::Ollama { url, model });
            info!("✅ Added Ollama provider");
        }

        if let Ok(url) = env::var("LMSTUDIO_URL") {
            let model =
                env::var("LMSTUDIO_MODEL").unwrap_or_else(|_| "llama-3.2-3b-instruct".to_string());
            providers.push(LlmProvider::LMStudio { url, model });
            info!("✅ Added LM Studio provider");
        }

        if providers.is_empty() {
            return Err(anyhow!("No LLM providers configured. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, GROQ_API_KEY, OLLAMA_URL, or LMSTUDIO_URL"));
        }

        let daily_budget = env::var("LLM_DAILY_BUDGET")
            .ok()
            .and_then(|s| s.parse().ok());

        info!(
            "🏗️ Created multi-provider LLM client with {} providers",
            providers.len()
        );
        if let Some(budget) = daily_budget {
            info!("💰 Daily budget limit: ${:.2}", budget);
        }

        Ok(Self::new_multi_provider(providers, daily_budget))
    }
}
