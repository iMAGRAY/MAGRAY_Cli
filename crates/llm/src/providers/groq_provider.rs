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
pub struct GroqProvider {
    api_key: String,
    model: String,
    client: Client,
    #[allow(dead_code)] // Будет использоваться для конфигурации HTTP timeout
    timeout: Duration,
}

impl GroqProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("Groq API key cannot be empty"));
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30)) // Groq is ultra-fast
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
            
        Ok(Self {
            api_key,
            model,
            client,
            timeout: Duration::from_secs(30),
        })
    }
    
    /// Get model-specific capabilities
    fn get_model_capabilities(&self) -> ProviderCapabilities {
        match self.model.as_str() {
            "llama-3.1-8b-instant" => ProviderCapabilities {
                max_tokens: 8192,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: false,
                context_window: 131_072,
                cost_per_1k_input: 0.00005,
                cost_per_1k_output: 0.00008,
                latency_class: LatencyClass::UltraFast, // Groq's specialty
                reliability_score: 0.93,
            },
            "llama-3.1-70b-versatile" => ProviderCapabilities {
                max_tokens: 8192,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: false,
                context_window: 131_072,
                cost_per_1k_input: 0.00059,
                cost_per_1k_output: 0.00079,
                latency_class: LatencyClass::UltraFast,
                reliability_score: 0.94,
            },
            "mixtral-8x7b-32768" => ProviderCapabilities {
                max_tokens: 32768,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: false,
                context_window: 32768,
                cost_per_1k_input: 0.00024,
                cost_per_1k_output: 0.00024,
                latency_class: LatencyClass::UltraFast,
                reliability_score: 0.92,
            },
            _ => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: false,
                supports_functions: false,
                supports_vision: false,
                context_window: 8192,
                cost_per_1k_input: 0.0001,
                cost_per_1k_output: 0.0002,
                latency_class: LatencyClass::UltraFast,
                reliability_score: 0.9,
            }
        }
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("groq", &self.model)
    }
    
    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }
    
    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();
        
        let test_request = GroqRequest {
            model: self.model.clone(),
            messages: vec![GroqMessage {
                role: "user".to_string(),
                content: "hi".to_string(),
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
        };
        
        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;
            
        match response {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                if resp.status().is_success() {
                    if elapsed > Duration::from_secs(5) {
                        info!("Groq health check: DEGRADED (slow for Groq: {:?})", elapsed);
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("Groq health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    error!("Groq health check failed: status {}", resp.status());
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("Groq health check failed: {}", e);
                Ok(ProviderHealth::Unavailable)
            }
        }
    }
    
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();
        
        self.validate_request(&request)?;
        
        let mut messages = Vec::new();
        
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(GroqMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }
        
        messages.push(GroqMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let groq_request = GroqRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        info!("🚀 Sending request to Groq: {} (ultra-fast)", 
            request.prompt.chars().take(50).collect::<String>());
        
        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&groq_request)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Groq API error: {}", error_text);
            return Err(anyhow!("Groq API error: {}", error_text));
        }
        
        let groq_response: GroqResponse = response.json().await?;
        let elapsed = start_time.elapsed();
        
        let choice = groq_response.choices.first()
            .ok_or_else(|| anyhow!("Empty response from Groq"))?;
            
        let usage = if let Some(usage) = groq_response.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = choice.message.content.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };
        
        info!("✅ Received ULTRA-FAST response from Groq ({:?}): {} tokens", 
            elapsed, usage.total_tokens);
        
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
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GroqResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}