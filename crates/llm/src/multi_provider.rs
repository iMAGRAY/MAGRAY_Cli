use crate::{
    LlmProvider, ProviderType, ProviderStats, CircuitBreaker, CircuitBreakerState, 
    CostOptimizer, TaskComplexity, ComplexityLevel, Priority, CompletionRequest,
};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, debug, warn, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Advanced multi-provider LLM orchestrator with failover, load balancing, and cost optimization
pub struct MultiProviderLlmOrchestrator {
    providers: Vec<LlmProvider>,
    provider_stats: Arc<Mutex<HashMap<String, ProviderStats>>>,
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    cost_optimizer: Arc<Mutex<CostOptimizer>>,
    client: Client,
    current_provider_index: Arc<Mutex<usize>>, // For round-robin load balancing
    retry_config: RetryConfig,
    performance_monitor: Arc<Mutex<PerformanceMonitor>>,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            exponential_base: 2.0,
        }
    }
}

#[derive(Debug, Default)]
struct PerformanceMonitor {
    total_requests: u64,
    total_successful: u64,
    total_failed: u64,
    provider_usage: HashMap<String, u64>,
    avg_response_time: f64,
}

impl MultiProviderLlmOrchestrator {
    pub fn new(providers: Vec<LlmProvider>, daily_budget: Option<f32>) -> Self {
        info!("🏗️ Инициализация MultiProviderLlmOrchestrator с {} провайдерами", providers.len());
        
        let mut provider_stats = HashMap::new();
        let mut circuit_breakers = HashMap::new();
        
        for provider in &providers {
            let provider_id = Self::get_provider_id(provider);
            provider_stats.insert(provider_id.clone(), ProviderStats::default());
            circuit_breakers.insert(provider_id.clone(), CircuitBreaker::default());
        }
        
        Self {
            providers,
            provider_stats: Arc::new(Mutex::new(provider_stats)),
            circuit_breakers: Arc::new(Mutex::new(circuit_breakers)),
            cost_optimizer: Arc::new(Mutex::new(CostOptimizer::new(daily_budget))),
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
            current_provider_index: Arc::new(Mutex::new(0)),
            retry_config: RetryConfig::default(),
            performance_monitor: Arc::new(Mutex::new(PerformanceMonitor::default())),
        }
    }
    
    /// Smart completion with automatic provider selection
    pub async fn complete_smart(&self, request: CompletionRequest) -> Result<String> {
        let task_complexity = self.analyze_task_complexity(&request);
        info!("🎯 Smart completion: complexity={:?}, priority={:?}", 
            task_complexity.complexity, task_complexity.priority);
        
        let available_providers = self.get_available_providers().await;
        
        if available_providers.is_empty() {
            return Err(anyhow!("No available providers (all circuit breakers open)"));
        }
        
        // Select optimal provider based on cost and complexity
        let selected_provider = {
            let optimizer = self.cost_optimizer.lock().unwrap();
            optimizer.select_optimal_provider(&available_providers, &task_complexity)
                .ok_or_else(|| anyhow!("No suitable provider found for task"))?
        };
        
        info!("✅ Selected provider: {}", Self::get_provider_name(&selected_provider));
        
        // Execute with retry logic
        self.execute_with_retry(&selected_provider, &request, &task_complexity).await
    }
    
    /// Get providers that are available (circuit breaker not open)
    async fn get_available_providers(&self) -> Vec<LlmProvider> {
        let circuit_breakers = self.circuit_breakers.lock().unwrap();
        let mut available = Vec::new();
        
        for provider in &self.providers {
            let provider_id = Self::get_provider_id(provider);
            if let Some(cb) = circuit_breakers.get(&provider_id) {
                if cb.state != CircuitBreakerState::Open {
                    available.push(provider.clone());
                }
            } else {
                available.push(provider.clone());
            }
        }
        
        debug!("📊 Available providers: {}/{}", available.len(), self.providers.len());
        available
    }
    
    /// Execute request with exponential backoff retry
    async fn execute_with_retry(
        &self, 
        provider: &LlmProvider, 
        request: &CompletionRequest,
        task_complexity: &TaskComplexity
    ) -> Result<String> {
        let provider_id = Self::get_provider_id(provider);
        let mut last_error = None;
        
        for attempt in 0..=self.retry_config.max_retries {
            // Check circuit breaker
            let can_execute = {
                let mut circuit_breakers = self.circuit_breakers.lock().unwrap();
                circuit_breakers.get_mut(&provider_id)
                    .map(|cb| cb.can_execute())
                    .unwrap_or(true)
            };
            
            if !can_execute {
                debug!("🚫 Circuit breaker blocked request to {}", provider_id);
                return Err(anyhow!("Circuit breaker open for provider: {}", provider_id));
            }
            
            let start_time = Instant::now();
            
            match self.execute_single_request(provider, request).await {
                Ok(response) => {
                    let latency = start_time.elapsed();
                    
                    // Record success
                    self.record_success(&provider_id, latency, task_complexity).await;
                    
                    info!("✅ Request successful on attempt {} ({}ms)", 
                        attempt + 1, latency.as_millis());
                    return Ok(response);
                }
                Err(e) => {
                    let latency = start_time.elapsed();
                    last_error = Some(anyhow::anyhow!(e.to_string()));
                    
                    // Record failure
                    self.record_failure(&provider_id, latency, &e.to_string()).await;
                    
                    if attempt < self.retry_config.max_retries {
                        let delay = self.calculate_retry_delay(attempt);
                        warn!("❌ Attempt {} failed for {}: {}. Retrying in {:?}", 
                            attempt + 1, provider_id, e, delay);
                        sleep(delay).await;
                    } else {
                        error!("💥 All {} attempts failed for {}: {}", 
                            self.retry_config.max_retries + 1, provider_id, e);
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Max retries exceeded")))
    }
    
    /// Execute single request to provider
    async fn execute_single_request(&self, provider: &LlmProvider, request: &CompletionRequest) -> Result<String> {
        match provider {
            LlmProvider::OpenAI { api_key, model } => {
                self.openai_request(api_key, model, request).await
            }
            LlmProvider::Anthropic { api_key, model } => {
                self.anthropic_request(api_key, model, request).await
            }
            LlmProvider::Local { url, model } | 
            LlmProvider::Ollama { url, model } | 
            LlmProvider::LMStudio { url, model } => {
                self.local_request(url, model, request).await
            }
            LlmProvider::Azure { endpoint, api_key, model } => {
                self.azure_request(endpoint, api_key, model, request).await
            }
            LlmProvider::Groq { api_key, model } => {
                self.groq_request(api_key, model, request).await
            }
        }
    }
    
    /// Analyze task complexity from request content
    pub fn analyze_task_complexity(&self, request: &CompletionRequest) -> TaskComplexity {
        let text = format!("{} {}", 
            request.system_prompt.as_deref().unwrap_or(""), 
            request.prompt);
        
        let tokens = self.estimate_tokens(&text);
        
        let complexity = if text.contains("architecture") || text.contains("complex") || tokens > 2000 {
            ComplexityLevel::Expert
        } else if text.contains("code") || text.contains("analyze") || tokens > 1000 {
            ComplexityLevel::Complex
        } else if tokens > 500 {
            ComplexityLevel::Medium
        } else {
            ComplexityLevel::Simple
        };
        
        let priority = if text.contains("critical") || text.contains("urgent") {
            Priority::Critical
        } else if text.contains("important") {
            Priority::High
        } else {
            Priority::Normal
        };
        
        TaskComplexity { tokens, complexity, priority }
    }
    
    /// Simple token estimation
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Rough estimation: 1 token ≈ 4 characters for English
        (text.len() as f32 / 4.0) as u32
    }
    
    /// Calculate exponential backoff delay
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let delay = self.retry_config.base_delay.as_millis() as f64 * 
                   self.retry_config.exponential_base.powi(attempt as i32);
        
        Duration::from_millis(delay.min(self.retry_config.max_delay.as_millis() as f64) as u64)
    }
    
    /// Record successful request
    async fn record_success(&self, provider_id: &str, latency: Duration, task_complexity: &TaskComplexity) {
        // Update provider stats
        {
            let mut stats = self.provider_stats.lock().unwrap();
            if let Some(provider_stats) = stats.get_mut(provider_id) {
                provider_stats.total_requests += 1;
                provider_stats.successful_requests += 1;
                
                let n = provider_stats.successful_requests as f32;
                provider_stats.avg_latency_ms = 
                    (provider_stats.avg_latency_ms * (n - 1.0) + latency.as_millis() as f32) / n;
            }
        }
        
        // Update circuit breaker
        {
            let mut circuit_breakers = self.circuit_breakers.lock().unwrap();
            if let Some(cb) = circuit_breakers.get_mut(provider_id) {
                cb.record_success();
            }
        }
        
        // Record cost
        {
            let mut optimizer = self.cost_optimizer.lock().unwrap();
            let estimated_cost = optimizer.cost_table.estimate_cost(
                &self.get_provider_type_by_id(provider_id),
                &self.get_model_by_provider_id(provider_id),
                task_complexity.tokens,
                task_complexity.tokens / 2, // Estimate output tokens
            );
            optimizer.record_cost(estimated_cost);
        }
        
        // Update performance monitor
        {
            let mut monitor = self.performance_monitor.lock().unwrap();
            monitor.total_requests += 1;
            monitor.total_successful += 1;
            *monitor.provider_usage.entry(provider_id.to_string()).or_insert(0) += 1;
            
            let n = monitor.total_successful as f64;
            monitor.avg_response_time = (monitor.avg_response_time * (n - 1.0) + latency.as_millis() as f64) / n;
        }
    }
    
    /// Record failed request
    async fn record_failure(&self, provider_id: &str, _latency: Duration, error: &str) {
        // Update provider stats
        {
            let mut stats = self.provider_stats.lock().unwrap();
            if let Some(provider_stats) = stats.get_mut(provider_id) {
                provider_stats.total_requests += 1;
                provider_stats.failed_requests += 1;
                provider_stats.last_error = Some(error.to_string());
            }
        }
        
        // Update circuit breaker
        {
            let mut circuit_breakers = self.circuit_breakers.lock().unwrap();
            if let Some(cb) = circuit_breakers.get_mut(provider_id) {
                cb.record_failure();
            }
        }
        
        // Update performance monitor
        {
            let mut monitor = self.performance_monitor.lock().unwrap();
            monitor.total_requests += 1;
            monitor.total_failed += 1;
        }
    }
    
    // API implementation methods
    async fn openai_request(&self, api_key: &str, model: &str, request: &CompletionRequest) -> Result<String> {
        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<OpenAIMessage>,
            max_tokens: Option<u32>,
            temperature: Option<f32>,
        }
        
        #[derive(Serialize)]
        struct OpenAIMessage {
            role: String,
            content: String,
        }
        
        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
        }
        
        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIResponseMessage,
        }
        
        #[derive(Deserialize)]
        struct OpenAIResponseMessage {
            content: String,
        }
        
        let mut messages = Vec::new();
        
        if let Some(system) = &request.system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
        
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let req = OpenAIRequest {
            model: model.to_string(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }
        
        let chat_response: OpenAIResponse = response.json().await?;
        
        chat_response.choices.first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("Empty response from OpenAI"))
    }
    
    async fn anthropic_request(&self, api_key: &str, model: &str, request: &CompletionRequest) -> Result<String> {
        #[derive(Serialize)]
        struct AnthropicRequest {
            model: String,
            max_tokens: u32,
            messages: Vec<AnthropicMessage>,
            temperature: Option<f32>,
        }
        
        #[derive(Serialize)]
        struct AnthropicMessage {
            role: String,
            content: String,
        }
        
        #[derive(Deserialize)]
        struct AnthropicResponse {
            content: Vec<AnthropicContent>,
        }
        
        #[derive(Deserialize)]
        struct AnthropicContent {
            text: String,
        }
        
        let mut messages = Vec::new();
        
        if let Some(system) = &request.system_prompt {
            messages.push(AnthropicMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
        
        messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let req = AnthropicRequest {
            model: model.to_string(),
            max_tokens: request.max_tokens.unwrap_or(1000),
            messages,
            temperature: request.temperature,
        };
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Anthropic API error: {}", error_text));
        }
        
        let chat_response: AnthropicResponse = response.json().await?;
        
        chat_response.content.first()
            .map(|content| content.text.clone())
            .ok_or_else(|| anyhow!("Empty response from Anthropic"))
    }
    
    async fn local_request(&self, url: &str, model: &str, request: &CompletionRequest) -> Result<String> {
        // Use OpenAI-compatible format for local models
        self.openai_compatible_request(url, model, request, None).await
    }
    
    async fn azure_request(&self, endpoint: &str, api_key: &str, model: &str, request: &CompletionRequest) -> Result<String> {
        self.openai_compatible_request(endpoint, model, request, Some(api_key)).await
    }
    
    async fn groq_request(&self, api_key: &str, model: &str, request: &CompletionRequest) -> Result<String> {
        self.openai_compatible_request("https://api.groq.com/openai/v1", model, request, Some(api_key)).await
    }
    
    async fn openai_compatible_request(&self, url: &str, model: &str, request: &CompletionRequest, api_key: Option<&str>) -> Result<String> {
        // Implement OpenAI-compatible request for local/alternative endpoints
        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<OpenAIMessage>,
            max_tokens: Option<u32>,
            temperature: Option<f32>,
        }
        
        #[derive(Serialize)]
        struct OpenAIMessage {
            role: String,
            content: String,
        }
        
        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
        }
        
        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIResponseMessage,
        }
        
        #[derive(Deserialize)]
        struct OpenAIResponseMessage {
            content: String,
        }
        
        let mut messages = Vec::new();
        
        if let Some(system) = &request.system_prompt {
            messages.push(OpenAIMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }
        
        messages.push(OpenAIMessage {
            role: "user".to_string(),
            content: request.prompt.clone(),
        });
        
        let req = OpenAIRequest {
            model: model.to_string(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        let mut request_builder = self.client
            .post(&format!("{}/chat/completions", url.trim_end_matches('/')))
            .header("Content-Type", "application/json");
            
        if let Some(key) = api_key {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request_builder
            .json(&req)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API error: {}", error_text));
        }
        
        let chat_response: OpenAIResponse = response.json().await?;
        
        chat_response.choices.first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("Empty response"))
    }
    
    // Utility methods
    fn get_provider_id(provider: &LlmProvider) -> String {
        match provider {
            LlmProvider::OpenAI { model, .. } => format!("openai_{}", model),
            LlmProvider::Anthropic { model, .. } => format!("anthropic_{}", model),
            LlmProvider::Local { model, .. } => format!("local_{}", model),
            LlmProvider::Ollama { model, .. } => format!("ollama_{}", model),
            LlmProvider::LMStudio { model, .. } => format!("lmstudio_{}", model),
            LlmProvider::Azure { model, .. } => format!("azure_{}", model),
            LlmProvider::Groq { model, .. } => format!("groq_{}", model),
        }
    }
    
    fn get_provider_name(provider: &LlmProvider) -> String {
        match provider {
            LlmProvider::OpenAI { model, .. } => format!("OpenAI ({})", model),
            LlmProvider::Anthropic { model, .. } => format!("Anthropic ({})", model),
            LlmProvider::Local { model, .. } => format!("Local ({})", model),
            LlmProvider::Ollama { model, .. } => format!("Ollama ({})", model),
            LlmProvider::LMStudio { model, .. } => format!("LM Studio ({})", model),
            LlmProvider::Azure { model, .. } => format!("Azure ({})", model),
            LlmProvider::Groq { model, .. } => format!("Groq ({})", model),
        }
    }
    
    fn get_provider_type_by_id(&self, provider_id: &str) -> ProviderType {
        if provider_id.starts_with("openai_") {
            ProviderType::OpenAI
        } else if provider_id.starts_with("anthropic_") {
            ProviderType::Anthropic
        } else if provider_id.starts_with("local_") {
            ProviderType::Local
        } else if provider_id.starts_with("ollama_") {
            ProviderType::Ollama
        } else if provider_id.starts_with("lmstudio_") {
            ProviderType::LMStudio
        } else if provider_id.starts_with("azure_") {
            ProviderType::Azure
        } else if provider_id.starts_with("groq_") {
            ProviderType::Groq
        } else {
            ProviderType::Local // Default fallback
        }
    }
    
    fn get_model_by_provider_id(&self, provider_id: &str) -> String {
        provider_id.split('_').nth(1).unwrap_or("unknown").to_string()
    }
    
    /// Get comprehensive status report
    pub async fn get_status_report(&self) -> String {
        let stats = self.provider_stats.lock().unwrap();
        let circuit_breakers = self.circuit_breakers.lock().unwrap();
        let optimizer = self.cost_optimizer.lock().unwrap();
        let monitor = self.performance_monitor.lock().unwrap();
        
        let mut report = String::new();
        report.push_str("🏗️ Multi-Provider LLM Orchestrator Status\n");
        report.push_str(&format!("📊 Overall: {} requests ({} successful, {} failed)\n", 
            monitor.total_requests, monitor.total_successful, monitor.total_failed));
        report.push_str(&format!("⏱️ Average response time: {:.1}ms\n", monitor.avg_response_time));
        report.push_str(&format!("💰 {}\n", optimizer.get_spending_summary()));
        report.push_str("\n🔌 Provider Status:\n");
        
        for provider in &self.providers {
            let provider_id = Self::get_provider_id(provider);
            let provider_name = Self::get_provider_name(provider);
            
            if let (Some(stat), Some(cb)) = (stats.get(&provider_id), circuit_breakers.get(&provider_id)) {
                report.push_str(&format!("  • {}: {} requests ({:.1}% success, {:.0}ms avg) - {}\n",
                    provider_name,
                    stat.total_requests,
                    if stat.total_requests > 0 { 
                        (stat.successful_requests as f32 / stat.total_requests as f32) * 100.0 
                    } else { 0.0 },
                    stat.avg_latency_ms,
                    cb.get_state_info()
                ));
                
                if let Some(error) = &stat.last_error {
                    report.push_str(&format!("    Last error: {}\n", error));
                }
            }
        }
        
        report
    }
}