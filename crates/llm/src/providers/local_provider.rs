use super::{
    LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth, ProviderId, 
    TokenUsage, LatencyClass, MessageRole,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, debug, error};

#[derive(Debug, Clone)]
pub struct LocalProvider {
    endpoint: String,
    model: String,
    provider_type: String, // "local", "ollama", "lmstudio"
    client: Client,
    timeout: Duration,
}

impl LocalProvider {
    pub fn new(endpoint: String, model: String, provider_type: String) -> Result<Self> {
        if endpoint.is_empty() {
            return Err(anyhow!("Local provider endpoint cannot be empty"));
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(120)) // Local models can be slow
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
            
        Ok(Self {
            endpoint,
            model,
            provider_type,
            client,
            timeout: Duration::from_secs(120),
        })
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to rebuild HTTP client with timeout");
        self
    }
    
    /// Get generic capabilities for local models
    fn get_model_capabilities(&self) -> ProviderCapabilities {
        // Local models are free but typically slower and less capable
        ProviderCapabilities {
            max_tokens: 4096,
            supports_streaming: false, // Most local setups don't support streaming
            supports_functions: false,
            supports_vision: false,
            context_window: 8192,
            cost_per_1k_input: 0.0,   // Free
            cost_per_1k_output: 0.0,  // Free
            latency_class: LatencyClass::Slow,
            reliability_score: 0.85,
        }
    }
}

#[async_trait]
impl LlmProvider for LocalProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new(&self.provider_type, &self.model)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }
    
    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();
        
        // Try to ping the health endpoint first
        let health_response = self.client
            .get(&format!("{}/health", self.endpoint.trim_end_matches('/')))
            .send()
            .await;
            
        if let Ok(resp) = health_response {
            if resp.status().is_success() {
                let elapsed = start_time.elapsed();
                debug!("Local provider health check: HEALTHY ({:?})", elapsed);
                return Ok(ProviderHealth::Healthy);
            }
        }
        
        // Fallback to a minimal completion request
        let test_request = LocalRequest {
            model: self.model.clone(),
            messages: vec![LocalMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
        };
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.endpoint.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                if resp.status().is_success() {
                    if elapsed > Duration::from_secs(30) {
                        info!("Local provider health check: DEGRADED (slow response: {:?})", elapsed);
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("Local provider health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    error!("Local provider health check failed: status {}", resp.status());
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("Local provider health check failed: {}", e);
                Ok(ProviderHealth::Unavailable)
            }
        }
    }
    
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();
        
        // Validate request first
        self.validate_request(&request)?;
        
        // Build messages array
        let mut messages = Vec::new();
        
        // Add system prompt if provided
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(LocalMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }
        
        // Add context messages if provided
        if let Some(context) = &request.context {
            for msg in context {
                let role = match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                };
                messages.push(LocalMessage {
                    role: role.to_string(),
                    content: msg.content.clone(),
                });
            }
        }
        
        // Add main prompt
        messages.push(LocalMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let local_request = LocalRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        info!("🚀 Sending request to {} provider: {} (model: {})", 
            self.provider_type,
            request.prompt.chars().take(50).collect::<String>(), 
            self.model
        );
        
        let response = self.client
            .post(&format!("{}/chat/completions", self.endpoint.trim_end_matches('/')))
            .header("Content-Type", "application/json")
            .json(&local_request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("{} provider API error: {}", self.provider_type, error_text);
            return Err(anyhow!("{} provider API error: {}", self.provider_type, error_text));
        }
        
        let local_response: LocalResponse = response.json().await?;
        let elapsed = start_time.elapsed();
        
        let choice = local_response.choices.first()
            .ok_or_else(|| anyhow!("Empty response from {} provider", self.provider_type))?;
            
        // Most local providers don't return usage statistics
        let usage = if let Some(usage) = local_response.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
            // Fallback estimation
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = choice.message.content.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };
        
        info!("✅ Received response from {} provider ({:?}): {} tokens", 
            self.provider_type, elapsed, usage.total_tokens);
        
        Ok(LlmResponse {
            content: choice.message.content.clone(),
            usage,
            model: self.model.clone(),
            finish_reason: choice.finish_reason.clone().unwrap_or("stop".to_string()),
            response_time: elapsed,
        })
    }
}

// Local provider request/response types (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct LocalRequest {
    model: String,
    messages: Vec<LocalMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct LocalMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LocalResponse {
    choices: Vec<LocalChoice>,
    usage: Option<LocalUsage>,
}

#[derive(Debug, Deserialize)]
struct LocalChoice {
    message: LocalResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct LocalUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_creation() {
        let provider = LocalProvider::new(
            "http://localhost:1234".to_string(),
            "llama-3.2-3b".to_string(),
            "local".to_string(),
        ).unwrap();
        
        assert_eq!(provider.id().provider_type, "local");
        assert_eq!(provider.id().model, "llama-3.2-3b");
        
        let capabilities = provider.capabilities();
        assert!(!capabilities.supports_streaming);
        assert_eq!(capabilities.cost_per_1k_input, 0.0); // Free
    }
}