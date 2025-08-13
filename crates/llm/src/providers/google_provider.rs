use super::{
    LatencyClass, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth,
    ProviderId, TokenUsage,
};
use crate::retry::{execute_streaming_with_retry, execute_with_retry, RetryConfig, RetryableError};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct GoogleProvider {
    api_key: String,
    model: String,
    client: Client,
    timeout: Duration,
    retry_config: RetryConfig,
}

/// Google AI-specific error type with retryable classification
#[derive(Debug)]
struct GoogleError {
    error_type: String,
    message: String,
    status_code: Option<u16>,
    is_retryable: bool,
}

impl GoogleError {
    fn from_status_code(status_code: u16, message: String) -> Self {
        let (error_type, is_retryable) = match status_code {
            // Timeout - retryable
            408 => ("timeout".to_string(), true),
            // Google-specific errors (before general patterns)
            503 => ("service_unavailable".to_string(), true), // Service unavailable
            // Quota errors with different status codes
            429 => {
                let is_quota = message.contains("quota") || message.contains("QUOTA_EXCEEDED");
                if is_quota {
                    ("quota_exceeded".to_string(), true)
                } else {
                    ("rate_limit".to_string(), true)
                }
            }
            400 => {
                let is_quota = message.contains("quota") || message.contains("QUOTA_EXCEEDED");
                if is_quota {
                    ("quota_exceeded".to_string(), true)
                } else {
                    ("bad_request".to_string(), false)
                }
            }
            // Server errors - retryable (after specific cases)
            500..=599 => ("server_error".to_string(), true),
            // Client errors - generally not retryable
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

    fn from_google_api_error(message: String) -> Self {
        // Check for specific Google AI error patterns
        let is_retryable = message.contains("QUOTA_EXCEEDED")
            || message.contains("RATE_LIMIT_EXCEEDED")
            || message.contains("SERVICE_UNAVAILABLE")
            || message.contains("INTERNAL")
            || message.contains("temporarily unavailable");

        Self {
            error_type: "google_api".to_string(),
            message,
            status_code: None,
            is_retryable,
        }
    }
}

impl RetryableError for GoogleError {
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

impl GoogleProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("Google AI API key cannot be empty"));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        // Aggressive retry config for Google AI due to quota handling needs
        let retry_config = RetryConfig::new()
            .with_max_retries(4)
            .with_initial_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(30))
            .with_backoff_multiplier(1.8)
            .with_jitter(true);

        Ok(Self {
            api_key,
            model,
            client,
            timeout: Duration::from_secs(60),
            retry_config,
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

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Get model-specific capabilities for Gemini models
    fn get_model_capabilities(&self) -> ProviderCapabilities {
        match self.model.as_str() {
            "gemini-1.5-pro" => ProviderCapabilities {
                max_tokens: 8192,
                supports_streaming: true,
                supports_functions: false, // Google uses function calling differently
                supports_vision: true,
                context_window: 2_000_000, // 2M tokens context
                cost_per_1k_input: 0.00125,
                cost_per_1k_output: 0.005,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.97,
            },
            "gemini-1.5-flash" => ProviderCapabilities {
                max_tokens: 8192,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: true,
                context_window: 1_000_000, // 1M tokens context
                cost_per_1k_input: 0.000075,
                cost_per_1k_output: 0.0003,
                latency_class: LatencyClass::Fast,
                reliability_score: 0.95,
            },
            "gemini-pro" => ProviderCapabilities {
                max_tokens: 2048,
                supports_streaming: true,
                supports_functions: false,
                supports_vision: false,
                context_window: 30_720, // ~30k tokens
                cost_per_1k_input: 0.0005,
                cost_per_1k_output: 0.0015,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.96,
            },
            "gemini-pro-vision" => ProviderCapabilities {
                max_tokens: 2048,
                supports_streaming: false,
                supports_functions: false,
                supports_vision: true,
                context_window: 12_288,
                cost_per_1k_input: 0.00025,
                cost_per_1k_output: 0.0005,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.94,
            },
            _ => {
                warn!(
                    "Unknown Google AI model '{}', using default capabilities",
                    self.model
                );
                ProviderCapabilities {
                    max_tokens: 2048,
                    supports_streaming: false,
                    supports_functions: false,
                    supports_vision: false,
                    context_window: 30_720,
                    cost_per_1k_input: 0.001,
                    cost_per_1k_output: 0.002,
                    latency_class: LatencyClass::Standard,
                    reliability_score: 0.9,
                }
            }
        }
    }

    /// Get the API endpoint for the model
    fn get_api_endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        )
    }

    /// Get the streaming API endpoint for the model
    fn get_streaming_endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent",
            self.model
        )
    }

    /// Parse SSE stream event and extract content
    fn parse_sse_event(event: &str) -> Option<String> {
        for line in event.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    return Some("[DONE]".to_string());
                }

                // Try to parse JSON data
                if let Ok(chunk) = serde_json::from_str::<GoogleStreamChunk>(data) {
                    if let Some(candidate) = chunk.candidates.first() {
                        if let Some(content) = &candidate.content {
                            if let Some(part) = content.parts.first() {
                                if let Some(text) = &part.text {
                                    return Some(text.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[async_trait]
impl LlmProvider for GoogleProvider {
    fn id(&self) -> ProviderId {
        ProviderId::new("google", &self.model)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        self.get_model_capabilities()
    }

    async fn health_check(&self) -> Result<ProviderHealth> {
        let start_time = Instant::now();

        // Create a minimal test request
        let test_request = GoogleRequest {
            contents: vec![GoogleContent {
                parts: vec![GooglePart {
                    text: Some("test".to_string()),
                }],
                role: Some("user".to_string()),
            }],
            generation_config: Some(GoogleGenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(1),
                ..Default::default()
            }),
            safety_settings: Some(vec![]), // Empty safety settings for health check
        };

        let response = self
            .client
            .post(format!("{}?key={}", self.get_api_endpoint(), self.api_key))
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
                            "Google AI health check: DEGRADED (slow response: {:?})",
                            elapsed
                        );
                        Ok(ProviderHealth::Degraded)
                    } else {
                        debug!("Google AI health check: HEALTHY ({:?})", elapsed);
                        Ok(ProviderHealth::Healthy)
                    }
                } else {
                    let status = resp.status();
                    let error_text = resp.text().await.unwrap_or_default();
                    error!(
                        "Google AI health check failed: status {} - {}",
                        status, error_text
                    );
                    Ok(ProviderHealth::Unavailable)
                }
            }
            Err(e) => {
                error!("Google AI health check failed: {}", e);
                Ok(ProviderHealth::Unavailable)
            }
        }
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();

        // Validate request first
        self.validate_request(&request)?;

        // Build contents array for Google AI API
        let mut contents = Vec::new();

        // Add system prompt if provided (as a system message)
        if let Some(system_prompt) = &request.system_prompt {
            contents.push(GoogleContent {
                parts: vec![GooglePart {
                    text: Some(system_prompt.clone()),
                }],
                role: Some("system".to_string()),
            });
        }

        // Add main prompt as user message
        contents.push(GoogleContent {
            parts: vec![GooglePart {
                text: Some(request.prompt.clone()),
            }],
            role: Some("user".to_string()),
        });

        let google_request = GoogleRequest {
            contents,
            generation_config: Some(GoogleGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                ..Default::default()
            }),
            safety_settings: Some(vec![
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
        };

        info!(
            "🚀 Sending request to Google AI: {} (model: {})",
            request.prompt.chars().take(50).collect::<String>(),
            self.model
        );

        // Execute with retry logic
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let endpoint = self.get_api_endpoint();

        let google_response = execute_with_retry(&self.retry_config, || {
            let client = client.clone();
            let api_key = api_key.clone();
            let endpoint = endpoint.clone();
            let google_request = google_request.clone();

            Box::pin(async move {
                let response = client
                    .post(format!("{endpoint}?key={api_key}"))
                    .header("Content-Type", "application/json")
                    .json(&google_request)
                    .send()
                    .await
                    .map_err(GoogleError::from_reqwest_error)?;

                if !response.status().is_success() {
                    let status = response.status().as_u16();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Failed to read error response".to_string());
                    return Err(GoogleError::from_status_code(status, error_text));
                }

                let google_response: GoogleResponse =
                    response.json().await.map_err(|e| GoogleError {
                        error_type: "parse_error".to_string(),
                        message: format!("Failed to parse response: {e}"),
                        status_code: None,
                        is_retryable: false,
                    })?;

                Ok(google_response)
            })
        })
        .await?;
        let elapsed = start_time.elapsed();

        let candidate = google_response
            .candidates
            .first()
            .ok_or_else(|| anyhow!("Empty response from Google AI"))?;

        let content = candidate
            .content
            .as_ref()
            .ok_or_else(|| anyhow!("No content in Google AI response"))?;

        let part = content
            .parts
            .first()
            .ok_or_else(|| anyhow!("No parts in Google AI content"))?;

        let text = part
            .text
            .as_ref()
            .ok_or_else(|| anyhow!("No text in Google AI part"))?;

        // Calculate token usage (Google AI API doesn't always provide exact counts)
        let usage = if let Some(usage_metadata) = &google_response.usage_metadata {
            TokenUsage::new(
                usage_metadata.prompt_token_count.unwrap_or(0),
                usage_metadata.candidates_token_count.unwrap_or(0),
            )
        } else {
            // Fallback estimation
            let prompt_tokens = request.prompt.len() as u32 / 4;
            let completion_tokens = text.len() as u32 / 4;
            TokenUsage::new(prompt_tokens, completion_tokens)
        };

        let finish_reason = candidate
            .finish_reason
            .as_ref()
            .map(|r| r.to_lowercase())
            .unwrap_or_else(|| "stop".to_string());

        info!(
            "✅ Received response from Google AI ({:?}): {} tokens",
            elapsed, usage.total_tokens
        );

        Ok(LlmResponse {
            content: text.clone(),
            usage,
            model: self.model.clone(),
            finish_reason,
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

        // Build contents array (similar to complete())
        let mut contents = Vec::new();

        if let Some(system_prompt) = &request.system_prompt {
            contents.push(GoogleContent {
                parts: vec![GooglePart {
                    text: Some(system_prompt.clone()),
                }],
                role: Some("system".to_string()),
            });
        }

        contents.push(GoogleContent {
            parts: vec![GooglePart {
                text: Some(request.prompt.clone()),
            }],
            role: Some("user".to_string()),
        });

        let google_request = GoogleRequest {
            contents,
            generation_config: Some(GoogleGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                ..Default::default()
            }),
            safety_settings: Some(vec![
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_HARASSMENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
                GoogleSafetySetting {
                    category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                    threshold: "BLOCK_MEDIUM_AND_ABOVE".to_string(),
                },
            ]),
        };

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let client = self.client.clone();
        let streaming_endpoint = self.get_streaming_endpoint();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            info!("🚀 Starting streaming request to Google AI");

            // Use simplified retry for streaming
            let result = execute_streaming_with_retry(
                3,                          // Max 3 retries for Google AI streaming (quota sensitive)
                Duration::from_millis(500), // 500ms base delay
                || {
                    let client = client.clone();
                    let streaming_endpoint = streaming_endpoint.clone();
                    let api_key = api_key.clone();
                    let google_request = google_request.clone();
                    let tx = tx.clone();

                    Box::pin(async move {
                        let response = client
                            .post(format!("{streaming_endpoint}?key={api_key}"))
                            .header("Content-Type", "application/json")
                            .json(&google_request)
                            .send()
                            .await
                            .map_err(GoogleError::from_reqwest_error)?;

                        if !response.status().is_success() {
                            let status = response.status().as_u16();
                            let error_text = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Failed to read error response".to_string());
                            return Err(GoogleError::from_status_code(status, error_text));
                        }

                        // Parse SSE stream
                        let mut stream = response.bytes_stream();
                        let mut buffer = String::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    let chunk_str = String::from_utf8_lossy(&chunk);
                                    buffer.push_str(&chunk_str);

                                    // Process complete SSE events
                                    while let Some(event_end) = buffer.find("\n\n") {
                                        let event =
                                            buffer.drain(..event_end + 2).collect::<String>();

                                        if let Some(content) = Self::parse_sse_event(&event) {
                                            if content == "[DONE]" {
                                                return Ok(());
                                            }
                                            if tx.send(content).await.is_err() {
                                                // Receiver dropped, stop streaming
                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(GoogleError {
                                        error_type: "stream_error".to_string(),
                                        message: format!("Stream error: {e}"),
                                        status_code: None,
                                        is_retryable: true,
                                    });
                                }
                            }
                        }

                        Ok(())
                    })
                },
            )
            .await;

            match result {
                Ok(_) => info!("✅ Google AI streaming request completed successfully"),
                Err(e) => {
                    error!("❌ Google AI streaming request failed after retries: {}", e);
                    // Try to send error notification to receiver
                    let _ = tx.send(format!("[ERROR: {e}]")).await;
                }
            }
        });

        Ok(rx)
    }
}

// Google AI API specific request/response types
#[derive(Debug, Clone, Serialize)]
struct GoogleRequest {
    contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GoogleGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<GoogleSafetySetting>>,
}

#[derive(Debug, Clone, Serialize)]
struct GoogleContent {
    parts: Vec<GooglePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GooglePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct GoogleSafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Deserialize)]
struct GoogleResponse {
    candidates: Vec<GoogleCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GoogleUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GoogleCandidate {
    content: Option<GoogleResponseContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
    index: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GoogleResponseContent {
    parts: Vec<GoogleResponsePart>,
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleResponsePart {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

// Types for streaming responses
#[derive(Debug, Deserialize)]
struct GoogleStreamChunk {
    candidates: Vec<GoogleStreamCandidate>,
}

#[derive(Debug, Deserialize)]
struct GoogleStreamCandidate {
    content: Option<GoogleResponseContent>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_google_provider_creation() {
        let provider =
            GoogleProvider::new("test-api-key".to_string(), "gemini-1.5-pro".to_string())
                .expect("Operation failed - converted from unwrap()");

        assert_eq!(provider.id().provider_type, "google");
        assert_eq!(provider.id().model, "gemini-1.5-pro");

        let capabilities = provider.capabilities();
        assert!(capabilities.supports_streaming);
        assert!(capabilities.supports_vision);
        assert_eq!(capabilities.context_window, 2_000_000);
    }

    #[tokio::test]
    async fn test_google_provider_validation() {
        let provider =
            GoogleProvider::new("test-api-key".to_string(), "gemini-1.5-flash".to_string())
                .expect("Operation failed - converted from unwrap()");

        let valid_request = LlmRequest::new("Hello").with_parameters(Some(1000), Some(0.7));
        assert!(provider.validate_request(&valid_request).is_ok());

        let invalid_request = LlmRequest::new("Hello").with_parameters(Some(10000), Some(0.7)); // Too many tokens
        assert!(provider.validate_request(&invalid_request).is_err());
    }

    #[tokio::test]
    async fn test_google_provider_mock_response() {
        let mut server = Server::new_async().await;

        let mock = server
            .mock("POST", "/v1beta/models/gemini-pro:generateContent")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": "Hello! How can I help you today?"
                        }],
                        "role": "model"
                    },
                    "finishReason": "STOP",
                    "index": 0
                }],
                "usageMetadata": {
                    "promptTokenCount": 5,
                    "candidatesTokenCount": 9,
                    "totalTokenCount": 14
                }
            }"#,
            )
            .create_async()
            .await;

        // Note: This test would require mocking the Google AI endpoint structure
        // The actual implementation would need proper endpoint handling
    }

    #[tokio::test]
    async fn test_model_capabilities() {
        let provider_pro =
            GoogleProvider::new("test-key".to_string(), "gemini-1.5-pro".to_string())
                .expect("Operation failed - converted from unwrap()");
        let caps_pro = provider_pro.capabilities();
        assert_eq!(caps_pro.context_window, 2_000_000);
        assert!(caps_pro.supports_vision);

        let provider_flash =
            GoogleProvider::new("test-key".to_string(), "gemini-1.5-flash".to_string())
                .expect("Operation failed - converted from unwrap()");
        let caps_flash = provider_flash.capabilities();
        assert_eq!(caps_flash.context_window, 1_000_000);
        assert_eq!(caps_flash.latency_class, LatencyClass::Fast);

        let provider_basic = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string())
            .expect("Operation failed - converted from unwrap()");
        let caps_basic = provider_basic.capabilities();
        assert_eq!(caps_basic.context_window, 30_720);
        assert!(!caps_basic.supports_vision);
    }
}
