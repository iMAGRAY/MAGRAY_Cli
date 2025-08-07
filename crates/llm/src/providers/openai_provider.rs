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
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    endpoint: String,
    client: Client,
    timeout: Duration,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String, endpoint: Option<String>) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("OpenAI API key cannot be empty"));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            api_key,
            model,
            endpoint: endpoint.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client,
            timeout: Duration::from_secs(60),
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
            "gpt-4-turbo" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: true,
                supports_vision: true,
                context_window: 128_000,
                cost_per_1k_input: 0.01,
                cost_per_1k_output: 0.03,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.98,
            },
            "gpt-3.5-turbo" => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: true,
                supports_functions: true,
                supports_vision: false,
                context_window: 16_385,
                cost_per_1k_input: 0.0005,
                cost_per_1k_output: 0.0015,
                latency_class: LatencyClass::Fast,
                reliability_score: 0.95,
            },
            _ => {
                // Default capabilities for unknown models
                ProviderCapabilities {
                    max_tokens: 4096,
                    supports_streaming: false,
                    supports_functions: false,
                    supports_vision: false,
                    context_window: 8192,
                    cost_per_1k_input: 0.001,
                    cost_per_1k_output: 0.003,
                    latency_class: LatencyClass::Standard,
                    reliability_score: 0.9,
                }
            }
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("openai", &self.model)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();

        let test_request = OpenAIRequest {
            model: self.model.clone(),
            messages: vec![OpenAIMessage {
                role: "user".to_string(),
                content: "test".to_string(),
            }],
            max_tokens: Some(1),
            temperature: Some(0.0),
            stream: Some(false),
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&test_request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let elapsed = start_time.elapsed();
                if resp.status().is_success() {
                    if elapsed > Duration::from_secs(10) {
                        info!(
                            "OpenAI health check: DEGRADED (slow response: {:?})",
                            elapsed
                        );
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("OpenAI health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    error!("OpenAI health check failed: status {}", resp.status());
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("OpenAI health check failed: {}", e);
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
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // Context handling can be added later if needed

        // Add main prompt
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let openai_request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: Some(false),
        };

        info!(
            "🚀 Sending request to OpenAI: {} (model: {})",
            request.prompt.chars().take(50).collect::<String>(),
            self.model
        );

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("OpenAI API error: {}", error_text);
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        let elapsed = start_time.elapsed();

        let choice = openai_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("Empty response from OpenAI"))?;

        let usage = if let Some(usage) = openai_response.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
            // Fallback estimation if usage not provided
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = choice.message.content.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };

        info!(
            "✅ Received response from OpenAI ({:?}): {} tokens",
            elapsed, usage.total_tokens
        );

        Ok(LlmResponse {
            content: choice.message.content.clone(),
            usage,
            model: self.model.clone(),
            finish_reason: choice.finish_reason.clone().unwrap_or("stop".to_string()),
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
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // Context handling can be added later if needed

        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });

        let openai_request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: Some(true),
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            info!("🚀 Starting streaming request to OpenAI");

            match client
                .post(&format!("{}/chat/completions", endpoint))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&openai_request)
                .send()
                .await
            {
                Ok(response) => {
                    if !response.status().is_success() {
                        error!("OpenAI streaming request failed: {}", response.status());
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
                                tokio::time::sleep(Duration::from_millis(50)).await;
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

// OpenAI-specific request/response types
#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_openai_provider_creation() {
        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .unwrap();

        assert_eq!(provider.id().provider_type, "openai");
        assert_eq!(provider.id().model, "gpt-4o-mini");

        let capabilities = provider.capabilities();
        assert!(capabilities.supports_streaming);
        assert!(capabilities.supports_functions);
        assert_eq!(capabilities.cost_per_1k_input, 0.00015);
    }

    #[tokio::test]
    async fn test_openai_provider_validation() {
        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .unwrap();

        let valid_request = LlmRequest::new("Hello").with_parameters(Some(1000), Some(0.7));
        assert!(provider.validate_request(&valid_request).is_ok());

        let invalid_request = LlmRequest::new("Hello").with_parameters(Some(50000), Some(0.7)); // Too many tokens
        assert!(provider.validate_request(&invalid_request).is_err());
    }

    #[tokio::test]
    async fn test_openai_provider_mock_response() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 8
                }
            }"#,
            )
            .create_async()
            .await;

        let provider = OpenAIProvider::new(
            "test-api-key".to_string(),
            "gpt-4o-mini".to_string(),
            Some(server.url()),
        )
        .unwrap();

        let request = LlmRequest::new("Hello");
        let response = provider.complete(request).await.unwrap();

        assert_eq!(response.content, "Hello! How can I help you today?");
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 8);
        assert_eq!(response.finish_reason, "stop");

        mock.assert_async().await;
    }
}
