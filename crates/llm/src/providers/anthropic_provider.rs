use super::{
    LatencyClass, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth,
    ProviderId, TokenUsage,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
    timeout: Duration,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("Anthropic API key cannot be empty"));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(90)) // Anthropic tends to be slower
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            api_key,
            model,
            client,
            timeout: Duration::from_secs(90),
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

    /// Get model-specific capabilities
    fn get_model_capabilities(&self) -> ProviderCapabilities {
        match self.model.as_str() {
            "claude-3-5-sonnet-20241022" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: false, // Anthropic uses different approach
                supports_vision: true,
                context_window: 200_000,
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.98,
            },
            "claude-3-haiku-20240307" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: true,
                context_window: 200_000,
                cost_per_1k_input: 0.00025,
                cost_per_1k_output: 0.00125,
                latency_class: LatencyClass::Fast,
                reliability_score: 0.96,
            },
            "claude-3-sonnet-20240229" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: true,
                context_window: 200_000,
                cost_per_1k_input: 0.003,
                cost_per_1k_output: 0.015,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.97,
            },
            "claude-3-opus-20240229" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: true,
                context_window: 200_000,
                cost_per_1k_input: 0.015,
                cost_per_1k_output: 0.075,
                latency_class: LatencyClass::Slow,
                reliability_score: 0.99,
            },
            _ => {
                // Default capabilities for unknown models
                ProviderCapabilities {
                    max_tokens: 4096,
                    supports_streaming: false,
                    supports_functions: false,
                    supports_vision: false,
                    context_window: 100_000,
                    cost_per_1k_input: 0.003,
                    cost_per_1k_output: 0.015,
                    latency_class: LatencyClass::Standard,
                    reliability_score: 0.95,
                }
            }
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("anthropic", &self.model)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();

        let test_request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1,
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            temperature: Some(0.0),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&test_request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                if resp.status().is_success() {
                    if elapsed > Duration::from_secs(15) {
                        info!(
                            "Anthropic health check: DEGRADED (slow response: {:?})",
                            elapsed
                        );
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("Anthropic health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    error!("Anthropic health check failed: status {}", resp.status());
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("Anthropic health check failed: {}", e);
                Ok(ProviderHealth::Unavailable)
            }
        }
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();

        // Validate request first
        self.validate_request(&request)?;

        // Build messages array - Anthropic has different format
        let mut messages = Vec::new();

        // Anthropic handles system prompt separately in newer API versions
        // For simplicity, we'll include it as the first message
        if let Some(system_prompt) = &request.system_prompt {
            messages.push(AnthropicMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // Context handling can be added later if needed

        // Add main prompt
        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let anthropic_request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(1000),
            messages,
            temperature: request.temperature,
        };

        info!(
            "🚀 Sending request to Anthropic: {} (model: {})",
            request.prompt.chars().take(50).collect::<String>(),
            self.model
        );

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&anthropic_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Anthropic API error: {}", error_text);
            return Err(anyhow!("Anthropic API error: {}", error_text));
        }

        let anthropic_response: AnthropicResponse = response.json().await?;
        let elapsed = start_time.elapsed();

        let content_block = anthropic_response
            .content
            .first()
            .ok_or_else(|| anyhow!("Empty response from Anthropic"))?;

        let usage = if let Some(usage) = anthropic_response.usage {
            TokenUsage::new(usage.input_tokens, usage.output_tokens)
        } else {
            // Fallback estimation if usage not provided
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = content_block.text.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };

        info!(
            "✅ Received response from Anthropic ({:?}): {} tokens",
            elapsed, usage.total_tokens
        );

        Ok(LlmResponse {
            content: content_block.text.clone(),
            usage,
            model: self.model.clone(),
            finish_reason: anthropic_response
                .stop_reason
                .unwrap_or("end_turn".to_string()),
            response_time: elapsed,
        })
    }

    async fn complete_stream(
        &self,
        request: LlmRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<String>> {
        let capabilities = self.capabilities();
        if !capabilities.supports_streaming {
            return Err(anyhow!("Streaming not supported for model {}", self.model));
        }

        // Validate request first
        self.validate_request(&request)?;

        // Build messages array (similar to complete())
        let mut messages = Vec::new();

        if let Some(system_prompt) = &request.system_prompt {
            messages.push(AnthropicMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // Context handling can be added later if needed

        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let anthropic_request = AnthropicStreamRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(1000),
            messages,
            temperature: request.temperature,
            stream: true,
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            info!("🚀 Starting streaming request to Anthropic");

            match client
                .post("https://api.anthropic.com/v1/messages")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .json(&anthropic_request)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        error!("Anthropic streaming request failed: {}", response.status());
                        return;
                    }

                    // In a real implementation, you would parse the SSE stream
                    // For now, we'll simulate streaming by chunking the response
                    match response.text().await {
                        Ok(text) => {
                            let words: Vec<&str> = text.split_whitespace().collect();
                            for word in words {
                                if tx.send(format!("{} ", word)).await.is_err() {
                                    break;
                                }
                                tokio::time::sleep(Duration::from_millis(80)).await;
                                // Anthropic is slower
                            }
                        }
                        Err(e) => {
                            error!("Failed to read streaming response: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to send streaming request: {}", e);
                }
            }

            info!("✅ Streaming request completed");
        });

        Ok(rx)
    }
}

// Anthropic-specific request/response types
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct AnthropicStreamRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    usage: Option<AnthropicUsage>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    text: String,
    #[serde(rename = "type")]
    #[allow(dead_code)] // Может использоваться для отладки или валидации
    content_type: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new(
            "test-api-key".to_string(),
            "claude-3-haiku-20240307".to_string(),
        )
        .unwrap();

        assert_eq!(provider.id().provider_type, "anthropic");
        assert_eq!(provider.id().model, "claude-3-haiku-20240307");

        let capabilities = provider.capabilities();
        assert!(capabilities.supports_streaming);
        assert!(!capabilities.supports_functions); // Anthropic uses different approach
        assert_eq!(capabilities.cost_per_1k_input, 0.00025);
    }

    #[tokio::test]
    async fn test_anthropic_provider_validation() {
        let provider = AnthropicProvider::new(
            "test-api-key".to_string(),
            "claude-3-haiku-20240307".to_string(),
        )
        .unwrap();

        let valid_request = LlmRequest::new("Hello").with_parameters(Some(1000), Some(0.7));
        assert!(provider.validate_request(&valid_request).is_ok());

        let invalid_request = LlmRequest::new(&"x".repeat(1_000_000)) // Too long
            .with_parameters(Some(1000), Some(0.7));
        assert!(provider.validate_request(&invalid_request).is_err());
    }
}
