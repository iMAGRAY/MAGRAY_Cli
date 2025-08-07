//! LLM Provider Port
//!
//! Абстракция для Language Model services для reranking, анализа и генерации.

use async_trait::async_trait;
use crate::ApplicationResult;
use serde::{Deserialize, Serialize};

/// Trait для LLM services
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate text completion/chat response
    async fn generate_response(&self, request: &LlmRequest) -> ApplicationResult<LlmResponse>;
    
    /// Batch text generation
    async fn generate_batch_responses(&self, requests: &[LlmRequest]) -> ApplicationResult<Vec<LlmResponse>>;
    
    /// Rerank search results using LLM
    async fn rerank_results(&self, query: &str, results: &[RerankItem]) -> ApplicationResult<Vec<RerankResult>>;
    
    /// Analyze text intent and extract entities
    async fn analyze_intent(&self, text: &str) -> ApplicationResult<IntentAnalysis>;
    
    /// Extract structured information from text
    async fn extract_information(&self, text: &str, schema: &ExtractionSchema) -> ApplicationResult<serde_json::Value>;
    
    /// Health check for LLM service
    async fn health_check(&self) -> ApplicationResult<LlmHealth>;
    
    /// Get provider capabilities
    fn get_capabilities(&self) -> LlmCapabilities;
    
    /// Get provider model information
    fn model_info(&self) -> ModelInfo;
}

/// LLM request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub parameters: LlmParameters,
    pub context: Option<RequestContext>,
}

/// Message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

/// Message roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant, 
    System,
    Tool,
}

/// LLM generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmParameters {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
}

/// Request context for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub source: String,
}

/// LLM response structure  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
    pub model: String,
    pub processing_time_ms: u64,
    pub metadata: ResponseMetadata,
}

/// Reason why generation finished
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCall,
    Error,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub model_version: String,
    pub cached: bool,
    pub latency_ms: u64,
    pub warnings: Vec<String>,
}

/// Item to rerank
#[derive(Debug, Clone)]
pub struct RerankItem {
    pub id: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub original_score: f32,
}

/// Reranking result
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub id: String,
    pub rerank_score: f32,
    pub combined_score: f32,
    pub explanation: Option<String>,
    pub confidence: f32,
}

/// Intent analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnalysis {
    pub primary_intent: Intent,
    pub confidence: f32,
    pub secondary_intents: Vec<(Intent, f32)>,
    pub entities: Vec<Entity>,
    pub sentiment: Sentiment,
    pub complexity: QueryComplexity,
}

/// Detected intent types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Question,
    Command,
    Search,
    Analysis,
    Comparison,
    Explanation,
    Troubleshooting,
    Creative,
    Unknown,
}

/// Extracted entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
    pub span: (usize, usize), // Character positions
}

/// Sentiment analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentiment {
    pub polarity: SentimentPolarity,
    pub score: f32,
    pub confidence: f32,
}

/// Sentiment polarity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentimentPolarity {
    Positive,
    Negative,
    Neutral,
    Mixed,
}

/// Query complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Information extraction schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSchema {
    pub schema_name: String,
    pub fields: Vec<SchemaField>,
    pub required_fields: Vec<String>,
    pub validation_rules: Vec<ValidationRule>,
}

/// Schema field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    pub name: String,
    pub field_type: FieldType,
    pub description: String,
    pub examples: Vec<String>,
}

/// Field types for extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Date,
    Array(Box<FieldType>),
    Object(Vec<SchemaField>),
}

/// Validation rules for extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_type: ValidationType,
    pub parameters: serde_json::Value,
    pub error_message: String,
}

/// Validation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationType {
    Required,
    MinLength,
    MaxLength,
    Pattern,
    Range,
    Custom(String),
}

/// LLM health status
#[derive(Debug, Clone)]
pub struct LlmHealth {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_rate: f32,
    pub rate_limit_remaining: Option<u32>,
    pub last_error: Option<String>,
    pub model_availability: ModelAvailability,
}

/// Model availability status
#[derive(Debug, Clone)]
pub enum ModelAvailability {
    Available,
    Degraded,
    Unavailable,
    RateLimited,
    MaintenanceMode,
}

/// LLM provider capabilities
#[derive(Debug, Clone)]
pub struct LlmCapabilities {
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub supports_vision: bool,
    pub supports_reranking: bool,
    pub max_context_length: u32,
    pub max_output_length: u32,
    pub supported_formats: Vec<String>,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: LlmProvider,
    pub model_name: String,
    pub model_version: String,
    pub context_window: u32,
    pub training_cutoff: Option<chrono::DateTime<chrono::Utc>>,
    pub capabilities: Vec<String>,
}

/// LLM provider types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmProviderType {
    OpenAI,
    Anthropic,
    Azure,
    Groq,
    Local,
    Custom(String),
}

impl Default for LlmParameters {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(1000),
            top_p: Some(0.9),
            frequency_penalty: Some(0.0),
            presence_penalty: Some(0.0),
            stop_sequences: vec![],
            stream: false,
        }
    }
}

impl LlmRequest {
    pub fn new(content: &str) -> Self {
        Self {
            messages: vec![Message {
                role: MessageRole::User,
                content: content.to_string(),
                metadata: None,
            }],
            parameters: LlmParameters::default(),
            context: None,
        }
    }
    
    pub fn with_system_message(mut self, system_content: &str) -> Self {
        self.messages.insert(0, Message {
            role: MessageRole::System,
            content: system_content.to_string(),
            metadata: None,
        });
        self
    }
    
    pub fn with_parameters(mut self, parameters: LlmParameters) -> Self {
        self.parameters = parameters;
        self
    }
}

/// Mock LLM provider for testing
#[cfg(feature = "test-utils")]
pub struct MockLlmProvider {
    pub responses: std::sync::Mutex<std::collections::VecDeque<ApplicationResult<LlmResponse>>>,
    pub model_info: ModelInfo,
}

#[cfg(feature = "test-utils")]
impl MockLlmProvider {
    pub fn new() -> Self {
        Self {
            responses: std::sync::Mutex::new(std::collections::VecDeque::new()),
            model_info: ModelInfo {
                provider: LlmProviderType::Custom("mock".to_string()),
                model_name: "mock-llm".to_string(),
                model_version: "1.0".to_string(),
                context_window: 4096,
                training_cutoff: None,
                capabilities: vec!["text-generation".to_string()],
            },
        }
    }
    
    pub fn add_response(&self, response: ApplicationResult<LlmResponse>) {
        self.responses.lock().unwrap().push_back(response);
    }
}

#[cfg(feature = "test-utils")]
#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate_response(&self, _request: &LlmRequest) -> ApplicationResult<LlmResponse> {
        if let Some(response) = self.responses.lock().unwrap().pop_front() {
            response
        } else {
            Ok(LlmResponse {
                content: "Mock response".to_string(),
                finish_reason: FinishReason::Stop,
                usage: TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                },
                model: "mock-llm".to_string(),
                processing_time_ms: 100,
                metadata: ResponseMetadata {
                    model_version: "1.0".to_string(),
                    cached: false,
                    latency_ms: 100,
                    warnings: vec![],
                },
            })
        }
    }
    
    async fn generate_batch_responses(&self, requests: &[LlmRequest]) -> ApplicationResult<Vec<LlmResponse>> {
        let mut responses = Vec::new();
        for request in requests {
            responses.push(self.generate_response(request).await?);
        }
        Ok(responses)
    }
    
    async fn rerank_results(&self, _query: &str, results: &[RerankItem]) -> ApplicationResult<Vec<RerankResult>> {
        Ok(results.iter().enumerate().map(|(i, item)| RerankResult {
            id: item.id.clone(),
            rerank_score: 1.0 - (i as f32 * 0.1),
            combined_score: item.original_score * (1.0 - (i as f32 * 0.1)),
            explanation: Some("Mock reranking".to_string()),
            confidence: 0.8,
        }).collect())
    }
    
    async fn analyze_intent(&self, _text: &str) -> ApplicationResult<IntentAnalysis> {
        Ok(IntentAnalysis {
            primary_intent: Intent::Question,
            confidence: 0.8,
            secondary_intents: vec![(Intent::Search, 0.2)],
            entities: vec![],
            sentiment: Sentiment {
                polarity: SentimentPolarity::Neutral,
                score: 0.0,
                confidence: 0.5,
            },
            complexity: QueryComplexity::Simple,
        })
    }
    
    async fn extract_information(&self, _text: &str, _schema: &ExtractionSchema) -> ApplicationResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }
    
    async fn health_check(&self) -> ApplicationResult<LlmHealth> {
        Ok(LlmHealth {
            is_healthy: true,
            response_time_ms: 50,
            error_rate: 0.0,
            rate_limit_remaining: Some(1000),
            last_error: None,
            model_availability: ModelAvailability::Available,
        })
    }
    
    fn get_capabilities(&self) -> LlmCapabilities {
        LlmCapabilities {
            supports_streaming: false,
            supports_function_calling: false,
            supports_vision: false,
            supports_reranking: true,
            max_context_length: 4096,
            max_output_length: 1000,
            supported_formats: vec!["text/plain".to_string()],
        }
    }
    
    fn model_info(&self) -> ModelInfo {
        self.model_info.clone()
    }
}