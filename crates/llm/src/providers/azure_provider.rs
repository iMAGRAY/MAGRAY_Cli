use super::{
    LatencyClass, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth,
    ProviderId, TokenUsage,
};
use crate::retry::{RetryConfig, RetryableError};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct AzureProvider {
    endpoint: String,
    api_key: String,
    model: String,
    client: Client,
    timeout: Duration,
    retry_config: RetryConfig,
}

/// Azure OpenAI-specific error type (similar to OpenAI)
#[derive(Debug)]
struct AzureError {
    error_type: String,
    message: String,
    status_code: Option<u16>,
    is_retryable: bool,
}

impl AzureError {
    fn from_status_code(status_code: u16, message: String) -> Self {
        let (error_type, is_retryable) = match status_code {
            // Rate limiting - always retryable
            429 => ("rate_limit".to_string(), true),
            // Server errors - retryable
            500..=599 => ("server_error".to_string(), true),
            // Timeout - retryable
            408 => ("timeout".to_string(), true),
            // Client errors - generally not retryable
            400 => ("bad_request".to_string(), false),
            401 => ("unauthorized".to_string(), false),
            403 => ("forbidden".to_string(), false),
            404 => ("not_found".to_string(), false),
            _ => ("unknown".to_string(), false),
        };

        Self {
            error_type,
            message,
            status_code: Some(status_code),
            is_retryable,
        }
    }

    fn from_reqwest_error(error: reqwest::Error) -> Self {
        let is_retryable = error.is_timeout() || error.is_connect();
        Self {
            error_type: if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "network"
            } else {
                "request"
            }
            .to_string(),
            message: error.to_string(),
            status_code: error.status().map(|s| s.as_u16()),
            is_retryable,
        }
    }
}

impl RetryableError for AzureError {
    fn is_retryable(&self) -> bool {
        self.is_retryable
    }

    fn error_type(&self) -> String {
        self.error_type.clone()
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }
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
            retry_config: RetryConfig::default(),
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
            },
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

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2023-12-01-preview",
            self.endpoint.trim_end_matches('/'),
            self.model
        );

        let response = self
            .client
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
                        info!(
                            "Azure health check: DEGRADED (slow response: {:?})",
                            elapsed
                        );
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

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2023-12-01-preview",
            self.endpoint.trim_end_matches('/'),
            self.model
        );

        info!(
            "🚀 Sending request to Azure: {}",
            request.prompt.chars().take(50).collect::<String>()
        );

        let response = self
            .client
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

        let choice = azure_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("Empty response from Azure"))?;

        let usage = if let Some(usage) = azure_response.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = choice.message.content.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };

        info!(
            "✅ Received response from Azure ({:?}): {} tokens",
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
