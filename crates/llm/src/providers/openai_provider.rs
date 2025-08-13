use super::{
    LatencyClass, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, ProviderHealth,
    ProviderId, TokenUsage,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    model: String,
    endpoint: String,
    client: Client,
    timeout: Duration,
    retry_config: RetryConfig,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

#[derive(Debug)]
struct OpenAIError {
    error_type: String,
    message: String,
    status_code: Option<u16>,
    is_retryable: bool,
}

impl OpenAIError {
    fn from_status_code(status_code: u16, message: String) -> Self {
        let (error_type, is_retryable) = match status_code {
            429 => ("rate_limit".to_string(), true),
            500..=599 => ("server_error".to_string(), true),
            408 => ("timeout".to_string(), true),
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
            } else {
                "network"
            }
            .to_string(),
            message: error.to_string(),
            status_code: error.status().map(|s| s.as_u16()),
            is_retryable,
        }
    }
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
            retry_config: RetryConfig::default(),
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

    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<T, OpenAIError>> + Send + 'static>,
            > + Send
            + Sync,
    {
        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt == self.retry_config.max_retries || !error.is_retryable {
                        last_error = Some(error);
                        break;
                    }

                    let delay = self.calculate_backoff_delay(attempt);
                    warn!(
                        "OpenAI request failed (attempt {}/{}): {} - retrying in {:?}",
                        attempt + 1,
                        self.retry_config.max_retries + 1,
                        error.message,
                        delay
                    );

                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        if let Some(error) = last_error {
            Err(anyhow!(
                "OpenAI request failed after {} attempts: {}",
                self.retry_config.max_retries + 1,
                error.message
            ))
        } else {
            Err(anyhow!("Unexpected error in retry logic"))
        }
    }

    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.retry_config.initial_delay.as_millis() as f64;
        let exponential_delay =
            base_delay * self.retry_config.backoff_multiplier.powi(attempt as i32);

        let mut delay = Duration::from_millis(
            exponential_delay.min(self.retry_config.max_delay.as_millis() as f64) as u64,
        );

        if self.retry_config.jitter {
            let jitter_range = delay.as_millis() as f64 * 0.1;
            let jitter = rand::thread_rng().gen_range(-jitter_range..jitter_range);
            let jittered_delay = (delay.as_millis() as f64 + jitter).max(0.0) as u64;
            delay = Duration::from_millis(jittered_delay);
        }

        delay
    }

    /// Parse SSE event and extract content
    fn parse_sse_event(event: &str) -> Option<String> {
        for line in event.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Some("[DONE]".to_string());
                }

                // Try to parse JSON data
                if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            return Some(content.clone());
                        }
                    }
                }
            }
        }
        None
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
            _ => ProviderCapabilities {
                max_tokens: 4096,
                supports_streaming: false,
                supports_functions: false,
                supports_vision: false,
                context_window: 8192,
                cost_per_1k_input: 0.001,
                cost_per_1k_output: 0.003,
                latency_class: LatencyClass::Standard,
                reliability_score: 0.9,
            },
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
            .post(format!("{}/chat/completions", self.endpoint))
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

        if let Some(system_prompt) = &request.system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

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

        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let api_key = self.api_key.clone();

        let result = self
            .execute_with_retry(|| {
                let client = client.clone();
                let endpoint = endpoint.clone();
                let api_key = api_key.clone();
                let openai_request = openai_request.clone();

                Box::pin(async move {
                    let response = client
                        .post(format!("{endpoint}/chat/completions"))
                        .header("Authorization", format!("Bearer {api_key}"))
                        .header("Content-Type", "application/json")
                        .json(&openai_request)
                        .send()
                        .await
                        .map_err(OpenAIError::from_reqwest_error)?;

                    if !response.status().is_success() {
                        let status = response.status().as_u16();
                        let error_text = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Failed to read error response".to_string());
                        return Err(OpenAIError::from_status_code(status, error_text));
                    }

                    let openai_response: OpenAIResponse =
                        response.json().await.map_err(|e| OpenAIError {
                            error_type: "parse_error".to_string(),
                            message: format!("Failed to parse response: {e}"),
                            status_code: None,
                            is_retryable: false,
                        })?;

                    Ok(openai_response)
                })
            })
            .await?;

        let elapsed = start_time.elapsed();

        let choice = result
            .choices
            .first()
            .ok_or_else(|| anyhow!("Empty response from OpenAI"))?;

        let usage = if let Some(usage) = result.usage {
            TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
        } else {
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

        // Build messages array
        let mut messages = Vec::new();

        if let Some(system_prompt) = &request.system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

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
        let retry_config = self.retry_config.clone();

        tokio::spawn(async move {
            info!("🚀 Starting streaming request to OpenAI");

            // Streaming requests get simplified retry (no exponential backoff due to nature of streaming)
            let mut retries = 0;

            loop {
                match client
                    .post(format!("{endpoint}/chat/completions"))
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("Content-Type", "application/json")
                    .json(&openai_request)
                    .send()
                    .await
                {
                    Ok(response) => {
                        if !response.status().is_success() {
                            let status = response.status();
                            let error_text = response.text().await.unwrap_or_default();

                            let openai_error =
                                OpenAIError::from_status_code(status.as_u16(), error_text);

                            if openai_error.is_retryable && retries < retry_config.max_retries {
                                retries += 1;
                                warn!("OpenAI streaming request failed (attempt {}/{}): {} - retrying", 
                                     retries, retry_config.max_retries + 1, openai_error.message);
                                tokio::time::sleep(retry_config.initial_delay).await;
                                continue;
                            } else {
                                error!("OpenAI streaming request failed: {}", openai_error.message);
                                return;
                            }
                        }

                        // Process successful response
                        let mut stream = response.bytes_stream();
                        let mut buffer = String::new();

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    if let Ok(text) = std::str::from_utf8(&chunk) {
                                        buffer.push_str(text);

                                        // Process complete SSE events
                                        while let Some(event_end) = buffer.find("\n\n") {
                                            let event = buffer[..event_end].to_string();
                                            buffer = buffer[event_end + 2..].to_string();

                                            if let Some(content) =
                                                OpenAIProvider::parse_sse_event(&event)
                                            {
                                                if content == "[DONE]" {
                                                    info!("✅ Streaming completed");
                                                    return;
                                                }

                                                if tx.send(content).await.is_err() {
                                                    debug!("Stream receiver closed");
                                                    return;
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Error reading stream chunk: {}", e);
                                    break;
                                }
                            }
                        }
                        break; // Success, exit retry loop
                    }
                    Err(e) => {
                        let openai_error = OpenAIError::from_reqwest_error(e);

                        if openai_error.is_retryable && retries < retry_config.max_retries {
                            retries += 1;
                            warn!(
                                "OpenAI streaming connection failed (attempt {}/{}): {} - retrying",
                                retries,
                                retry_config.max_retries + 1,
                                openai_error.message
                            );
                            tokio::time::sleep(retry_config.initial_delay).await;
                            continue;
                        } else {
                            error!("Failed to send streaming request: {}", openai_error.message);
                            break;
                        }
                    }
                }
            }

            info!("✅ Streaming request completed");
        });

        Ok(rx)
    }
}

// OpenAI-specific request/response types
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

// Types for streaming responses
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    role: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use std::time::Duration;

    #[tokio::test]
    async fn test_openai_provider_creation() {
        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .expect("Operation failed - converted from unwrap()");

        assert_eq!(provider.id().provider_type, "openai");
        assert_eq!(provider.id().model, "gpt-4o-mini");

        let capabilities = provider.capabilities();
        assert!(capabilities.supports_streaming);
        assert!(capabilities.supports_functions);
        assert_eq!(capabilities.cost_per_1k_input, 0.00015);
    }

    #[tokio::test]
    async fn test_retry_config_creation() {
        let custom_retry_config = RetryConfig {
            max_retries: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 1.5,
            jitter: false,
        };

        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .expect("Operation failed - converted from unwrap()")
                .with_retry_config(custom_retry_config.clone());

        assert_eq!(provider.retry_config.max_retries, 5);
        assert_eq!(
            provider.retry_config.initial_delay,
            Duration::from_millis(200)
        );
        assert!(!provider.retry_config.jitter);
    }

    #[tokio::test]
    async fn test_backoff_delay_calculation() {
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .expect("Operation failed - converted from unwrap()")
                .with_retry_config(retry_config);

        let delay_0 = provider.calculate_backoff_delay(0);
        let delay_1 = provider.calculate_backoff_delay(1);
        let delay_2 = provider.calculate_backoff_delay(2);

        assert_eq!(delay_0, Duration::from_millis(100));
        assert_eq!(delay_1, Duration::from_millis(200));
        assert_eq!(delay_2, Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_openai_error_classification() {
        // Test rate limit error (retryable)
        let rate_limit_error =
            OpenAIError::from_status_code(429, "Rate limit exceeded".to_string());
        assert!(rate_limit_error.is_retryable);
        assert_eq!(rate_limit_error.error_type, "rate_limit");

        // Test server error (retryable)
        let server_error = OpenAIError::from_status_code(500, "Internal server error".to_string());
        assert!(server_error.is_retryable);
        assert_eq!(server_error.error_type, "server_error");

        // Test auth error (not retryable)
        let auth_error = OpenAIError::from_status_code(401, "Unauthorized".to_string());
        assert!(!auth_error.is_retryable);
        assert_eq!(auth_error.error_type, "unauthorized");

        // Test bad request (not retryable)
        let bad_request = OpenAIError::from_status_code(400, "Bad request".to_string());
        assert!(!bad_request.is_retryable);
        assert_eq!(bad_request.error_type, "bad_request");
    }

    #[tokio::test]
    async fn test_openai_provider_validation() {
        let provider =
            OpenAIProvider::new("test-api-key".to_string(), "gpt-4o-mini".to_string(), None)
                .expect("Operation failed - converted from unwrap()");

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
        .expect("Operation failed - converted from unwrap()");

        let request = LlmRequest::new("Hello");
        let response = provider
            .complete(request)
            .await
            .expect("Async operation should succeed");

        assert_eq!(response.content, "Hello! How can I help you today?");
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 8);
        assert_eq!(response.finish_reason, "stop");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_retry_on_server_error() {
        let mut server = Server::new_async().await;

        // First two requests fail with 500, third succeeds
        let error_mock = server
            .mock("POST", "/chat/completions")
            .with_status(500)
            .with_header("content-type", "text/plain")
            .with_body("Internal Server Error")
            .expect(2)
            .create_async()
            .await;

        let success_mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": "Success after retry!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 5,
                    "completion_tokens": 3
                }
            }"#,
            )
            .create_async()
            .await;

        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10), // Fast for testing
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 1.5,
            jitter: false,
        };

        let provider = OpenAIProvider::new(
            "test-api-key".to_string(),
            "gpt-4o-mini".to_string(),
            Some(server.url()),
        )
        .expect("Operation failed - converted from unwrap()")
        .with_retry_config(retry_config);

        let request = LlmRequest::new("Test retry");
        let response = provider
            .complete(request)
            .await
            .expect("Async operation should succeed");

        assert_eq!(response.content, "Success after retry!");
        assert_eq!(response.usage.prompt_tokens, 5);
        assert_eq!(response.usage.completion_tokens, 3);

        error_mock.assert_async().await;
        success_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_no_retry_on_auth_error() {
        let mut server = Server::new_async().await;

        // Auth error should not be retried
        let auth_error_mock = server
            .mock("POST", "/chat/completions")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": {"message": "Invalid API key"}}"#)
            .expect(1) // Should only be called once (no retries)
            .create_async()
            .await;

        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let provider = OpenAIProvider::new(
            "invalid-api-key".to_string(),
            "gpt-4o-mini".to_string(),
            Some(server.url()),
        )
        .expect("Operation failed - converted from unwrap()")
        .with_retry_config(retry_config);

        let request = LlmRequest::new("Test no retry");
        let result = provider.complete(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid API key"));

        auth_error_mock.assert_async().await;
    }
}
