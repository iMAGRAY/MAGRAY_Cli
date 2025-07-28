use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, debug, error};

#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI { api_key: String, model: String },
    Anthropic { api_key: String, model: String },
    Local { url: String, model: String },
}

#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Clone)]
pub struct LlmClient {
    provider: LlmProvider,
    client: reqwest::Client,
    max_tokens: u32,
    temperature: f32,
}

impl LlmClient {
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
                let model = env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-3-haiku-20240307".to_string());
                LlmProvider::Anthropic { api_key, model }
            }
            "local" => {
                let url = env::var("LOCAL_LLM_URL").unwrap_or_else(|_| "http://localhost:1234/v1".to_string());
                let model = env::var("LOCAL_LLM_MODEL").unwrap_or_else(|_| "llama-3.2-3b-instruct".to_string());
                LlmProvider::Local { url, model }
            }
            _ => return Err(anyhow!("Неподдерживаемый LLM_PROVIDER: {}", provider_type)),
        };

        Ok(Self {
            provider,
            client: reqwest::Client::new(),
            max_tokens,
            temperature,
        })
    }

    pub async fn chat(&self, message: &str) -> Result<String> {
        match &self.provider {
            LlmProvider::OpenAI { api_key, model } => {
                self.openai_chat(api_key, model, message).await
            }
            LlmProvider::Anthropic { api_key, model } => {
                self.anthropic_chat(api_key, model, message).await
            }
            LlmProvider::Local { url, model } => {
                self.local_chat(url, model, message).await
            }
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
            .header("Authorization", format!("Bearer {}", api_key))
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
            .header("Authorization", format!("Bearer {}", api_key))
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

        info!("🚀 Отправляю запрос в локальную модель: {} -> {}", url, model);
        debug!("Текст запроса: {}", message);

        let endpoint = format!("{}/chat/completions", url.trim_end_matches('/'));
        
        let response = self
            .client
            .post(&endpoint)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Локальная LLM ошибка: {}", error_text);
            return Err(anyhow!("Локальная LLM ошибка: {}", error_text));
        }

        let chat_response: OpenAIChatResponse = response.json().await?;
        
        if let Some(choice) = chat_response.choices.first() {
            info!("✅ Получен ответ от локальной модели");
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("Пустой ответ от локальной модели"))
        }
    }
}
