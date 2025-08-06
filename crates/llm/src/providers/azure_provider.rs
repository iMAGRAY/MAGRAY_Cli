use super::{
    LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth, ProviderId, 
    TokenUsage, LatencyClass,
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, debug, error};

#[derive(Debug, Clone)]
pub struct AzureProvider {
    endpoint: String,
    api_key: String,
    model: String,
    client: Client,
    #[allow(dead_code)] // Будет использоваться для конфигурации HTTP timeout
    timeout: Duration,
}

impl AzureProvider {
    pub fn new(endpoint: String, api_key: String, model: String) -> Result<Self> {
        if endpoint.is_empty() || api_key.is_empty() {
            return Err(anyhow!("Azure endpoint and API key cannot be empty"));
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
            
        Ok(Self {
            endpoint,
            api_key,
            model,
            client,
            timeout: Duration::from_secs(60),
        })
    }
    
    /// Get model-specific capabilities (similar to OpenAI)
    fn get_model_capabilities(&self) -> ProviderCapabilities {
        // Azure OpenAI models have similar capabilities to OpenAI
        match self.model.as_str() {
            "gpt-4o" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: true,
                supports_vision: true,
                context_window: 128_000,
                cost_per_1k_input: 0.0025,
                cost_per_1k_output: 0.01,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.98,
            },
            "gpt-4o-mini" => ProviderCapabilities {
                max_tokens: 16384,
                supports_streaming: true,
                supports_functions: true,
                supports_vision: true,
                context_window: 128_000,
                cost_per_1k_input: 0.00015,
                cost_per_1k_output: 0.0006,
                latency_class: LatencyClass::Fast,
                reliability_score: 0.97,
            },
            _ => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: false,
                supports_functions: false,
                supports_vision: false,
                context_window: 8192,
                cost_per_1k_input: 0.0025,
                cost_per_1k_output: 0.01,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.96,
            }
        }
    }
}

#[async_trait]
impl LlmProvider for AzureProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("azure", &self.model)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }
    
    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();
        
        let test_request = AzureRequest {
            messages: vec![AzureMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
        };
        
        let url = format!("{}/openai/deployments/{}/chat/completions?api-version=2023-12-01-preview", 
            self.endpoint.trim_end_matches('/'), self.model);
        
        let response = self.client
            .post(&url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                if resp.status().is_success() {
                    if elapsed > Duration::from_secs(10) {
                        info!("Azure health check: DEGRADED (slow response: {:?})", elapsed);
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("Azure health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    error!("Azure health check failed: status {}", resp.status());
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("Azure health check failed: {}", e);
                Ok(ProviderHealth::Unavailable)
            }
        }
    }
    
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();
        
        self.validate_request(&request)?;
        
        let mut messages = Vec::new();
        
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(AzureMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }
        
        messages.push(AzureMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let azure_request = AzureRequest {
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        let url = format!("{}/openai/deployments/{}/chat/completions?api-version=2023-12-01-preview", 
            self.endpoint.trim_end_matches('/'), self.model);
        
        info!("🚀 Sending request to Azure: {}", request.prompt.chars().take(50).collect::<String>());
        
        let response = self.client
            .post(&url)
            .header("api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&azure_request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Azure API error: {}", error_text);
            return Err(anyhow!("Azure API error: {}", error_text));
        }
        
        let azure_response: AzureResponse = response.json().await?;
        let elapsed = start_time.elapsed();
        
        let choice = azure_response.choices.first()
            .ok_or_else(|| anyhow!("Empty response from Azure"))?;
            
        let usage = if let Some(usage) = azure_response.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = choice.message.content.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };
        
        info!("✅ Received response from Azure ({:?}): {} tokens", elapsed, usage.total_tokens);
        
        Ok(LlmResponse {
            content: choice.message.content.clone(),
            usage,
            model: self.model.clone(),
            finish_reason: choice.finish_reason.clone().unwrap_or("stop".to_string()),
            response_time: elapsed,
        })
    }
}

#[derive(Debug, Serialize)]
struct AzureRequest {
    messages: Vec<AzureMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AzureMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AzureResponse {
    choices: Vec<AzureChoice>,
    usage: Option<AzureUsage>,
}

#[derive(Debug, Deserialize)]
struct AzureChoice {
    message: AzureResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct AzureUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}