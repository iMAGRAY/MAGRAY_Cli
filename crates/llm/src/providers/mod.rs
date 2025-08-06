use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::Instant;

pub mod openai_provider;
pub mod anthropic_provider;
pub mod local_provider;
pub mod azure_provider;
pub mod groq_provider;

pub use openai_provider::OpenAIProvider;
pub use anthropic_provider::AnthropicProvider;
pub use local_provider::LocalProvider;
pub use azure_provider::AzureProvider;
pub use groq_provider::GroqProvider;

/// Request object for LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

impl LlmRequest {
    pub fn new(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            system_prompt: None,
            max_tokens: None,
            temperature: None,
            stream: false,
            context: None,
        }
    }
    
    pub fn with_system_prompt(mut self, system_prompt: &str) -> Self {
        self.system_prompt = Some(system_prompt.to_string());
        self
    }
    
    
    pub fn with_parameters(mut self, max_tokens: Option<u32>, temperature: Option<f32>) -> Self {
        self.max_tokens = max_tokens;
        self.temperature = temperature;
        self
    }
}

/// Response object from LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub model: String,
    pub finish_reason: String,
    pub response_time: Duration,
}

/// Chat message for conversation context
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: content.to_string(),
            timestamp: Some(Instant::now()),
        }
    }
    
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Some(Instant::now()),
        }
    }
    
    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: Some(Instant::now()),
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

/// Provider capabilities and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub max_tokens: u32,
    pub supports_streaming: bool,
    pub supports_functions: bool,
    pub supports_vision: bool,
    pub context_window: u32,
    pub cost_per_1k_input: f32,
    pub cost_per_1k_output: f32,
    pub latency_class: LatencyClass,
    pub reliability_score: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LatencyClass {
    UltraFast,  // < 500ms (Groq)
    Fast,       // 500ms - 2s
    Standard,   // 2s - 5s  
    Slow,       // > 5s
}

/// Provider health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderHealth {
    Healthy,
    Degraded,
    Unavailable,
}

/// Provider identification
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderId {
    pub provider_type: String,
    pub model: String,
    pub region: Option<String>,
}

impl ProviderId {
    pub fn new(provider_type: &str, model: &str) -> Self {
        Self {
            provider_type: provider_type.to_string(),
            model: model.to_string(),
            region: None,
        }
    }
    
    pub fn with_region(mut self, region: &str) -> Self {
        self.region = Some(region.to_string());
        self
    }
}

/// Unified LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Unique identifier for this provider instance
    fn id(&self) -> ProviderId;
    
    /// Get provider capabilities and metadata
    fn capabilities(&self) -> ProviderCapabilities;
    
    /// Check provider health status
    async fn health_check(&self) -> Result<ProviderHealth>;
    
    /// Execute completion request
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;
    
    /// Execute streaming completion (if supported)
    async fn complete_stream(&self, request: LlmRequest) -> Result<tokio::sync::mpsc::Receiver<String>> {
        // Default implementation for non-streaming providers
        let response = self.complete(request).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        
        tokio::spawn(async move {
            let _ = tx.send(response.content).await;
        });
        
        Ok(rx)
    }
    
    /// Estimate cost for a request
    fn estimate_cost(&self, request: &LlmRequest) -> f32 {
        let capabilities = self.capabilities();
        let estimated_input_tokens = request.prompt.len() as f32 / 4.0; // Rough estimation
        let estimated_output_tokens = request.max_tokens.unwrap_or(1000) as f32;
        
        (estimated_input_tokens / 1000.0 * capabilities.cost_per_1k_input) +
        (estimated_output_tokens / 1000.0 * capabilities.cost_per_1k_output)
    }
    
    /// Validate request before execution
    fn validate_request(&self, request: &LlmRequest) -> Result<()> {
        let capabilities = self.capabilities();
        
        if let Some(max_tokens) = request.max_tokens {
            if max_tokens > capabilities.max_tokens {
                return Err(anyhow::anyhow!(
                    "Requested max_tokens ({}) exceeds provider limit ({})",
                    max_tokens,
                    capabilities.max_tokens
                ));
            }
        }
        
        let estimated_prompt_tokens = request.prompt.len() as u32 / 4;
        if estimated_prompt_tokens > capabilities.context_window {
            return Err(anyhow::anyhow!(
                "Prompt too long: estimated {} tokens, max {}",
                estimated_prompt_tokens,
                capabilities.context_window
            ));
        }
        
        Ok(())
    }
    
    /// Get human-readable name
    fn name(&self) -> String {
        format!("{} ({})", self.id().provider_type, self.id().model)
    }
}

/// Enum wrapper for all provider types to enable trait objects
#[derive(Debug, Clone)]
pub enum ProviderWrapper {
    OpenAI(OpenAIProvider),
    Anthropic(AnthropicProvider),
    Local(LocalProvider),
    Azure(AzureProvider),
    Groq(GroqProvider),
}

#[async_trait]
impl LlmProvider for ProviderWrapper {
    fn id(&self) -> ProviderId {
        match self {
            ProviderWrapper::OpenAI(p) => p.id(),
            ProviderWrapper::Anthropic(p) => p.id(),
            ProviderWrapper::Local(p) => p.id(),
            ProviderWrapper::Azure(p) => p.id(),
            ProviderWrapper::Groq(p) => p.id(),
        }
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        match self {
            ProviderWrapper::OpenAI(p) => p.capabilities(),
            ProviderWrapper::Anthropic(p) => p.capabilities(),
            ProviderWrapper::Local(p) => p.capabilities(),
            ProviderWrapper::Azure(p) => p.capabilities(),
            ProviderWrapper::Groq(p) => p.capabilities(),
        }
    }
    
    async fn health_check(&self) -> Result<ProviderHealth> {
        match self {
            ProviderWrapper::OpenAI(p) => p.health_check().await,
            ProviderWrapper::Anthropic(p) => p.health_check().await,
            ProviderWrapper::Local(p) => p.health_check().await,
            ProviderWrapper::Azure(p) => p.health_check().await,
            ProviderWrapper::Groq(p) => p.health_check().await,
        }
    }
    
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        match self {
            ProviderWrapper::OpenAI(p) => p.complete(request).await,
            ProviderWrapper::Anthropic(p) => p.complete(request).await,
            ProviderWrapper::Local(p) => p.complete(request).await,
            ProviderWrapper::Azure(p) => p.complete(request).await,
            ProviderWrapper::Groq(p) => p.complete(request).await,
        }
    }
    
    async fn complete_stream(&self, request: LlmRequest) -> Result<tokio::sync::mpsc::Receiver<String>> {
        match self {
            ProviderWrapper::OpenAI(p) => p.complete_stream(request).await,
            ProviderWrapper::Anthropic(p) => p.complete_stream(request).await,
            ProviderWrapper::Local(p) => p.complete_stream(request).await,
            ProviderWrapper::Azure(p) => p.complete_stream(request).await,
            ProviderWrapper::Groq(p) => p.complete_stream(request).await,
        }
    }
}

/// Provider factory for creating instances
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create provider from configuration
    pub fn create_provider(config: &ProviderConfig) -> Result<ProviderWrapper> {
        match config.provider_type.as_str() {
            "openai" => {
                let provider = OpenAIProvider::new(
                    config.api_key.clone().unwrap_or_default(),
                    config.model.clone(),
                    config.endpoint.clone(),
                )?;
                Ok(ProviderWrapper::OpenAI(provider))
            }
            "anthropic" => {
                let provider = AnthropicProvider::new(
                    config.api_key.clone().unwrap_or_default(),
                    config.model.clone(),
                )?;
                Ok(ProviderWrapper::Anthropic(provider))
            }
            "azure" => {
                let provider = AzureProvider::new(
                    config.endpoint.clone().unwrap_or_default(),
                    config.api_key.clone().unwrap_or_default(),
                    config.model.clone(),
                )?;
                Ok(ProviderWrapper::Azure(provider))
            }
            "groq" => {
                let provider = GroqProvider::new(
                    config.api_key.clone().unwrap_or_default(),
                    config.model.clone(),
                )?;
                Ok(ProviderWrapper::Groq(provider))
            }
            "local" | "ollama" | "lmstudio" => {
                let provider = LocalProvider::new(
                    config.endpoint.clone().unwrap_or_default(),
                    config.model.clone(),
                    config.provider_type.clone(),
                )?;
                Ok(ProviderWrapper::Local(provider))
            }
            _ => Err(anyhow::anyhow!("Unknown provider type: {}", config.provider_type)),
        }
    }
}

/// Configuration for provider creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
}

impl ProviderConfig {
    pub fn new(provider_type: &str, model: &str) -> Self {
        Self {
            provider_type: provider_type.to_string(),
            model: model.to_string(),
            api_key: None,
            endpoint: None,
            region: None,
            timeout: None,
            max_retries: None,
        }
    }
    
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }
    
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}