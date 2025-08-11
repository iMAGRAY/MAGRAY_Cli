//! Contracts and interfaces for MAGRAY components

use crate::*;
use async_trait::async_trait;

/// Tool execution contract
#[async_trait]
pub trait Tool: Send + Sync {
    /// Execute tool with given arguments
    async fn execute(
        &self,
        command: &str,
        args: serde_json::Value,
        context: &TaskContext,
    ) -> Result<ToolResult>;

    /// Check if tool supports dry run
    fn supports_dry_run(&self) -> bool;

    /// Get tool specification
    fn spec(&self) -> &ToolSpec;
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub artifacts: Vec<Artifact>,
    pub side_effects: Vec<SideEffect>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    pub category: SideEffectCategory,
    pub description: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SideEffectCategory {
    FileSystem,
    Network,
    Process,
    Environment,
}

/// LLM client contract
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate completion
    async fn complete(
        &self,
        prompt: &str,
        options: CompletionOptions,
    ) -> Result<CompletionResponse>;

    /// Stream completion
    async fn stream(
        &self,
        prompt: &str,
        options: CompletionOptions,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Unpin>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub tokens_used: u32,
    pub finish_reason: String,
}

/// Vector index contract
#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// Add vector to index
    async fn add(&mut self, id: Uuid, vector: Vec<f32>, metadata: serde_json::Value) -> Result<()>;

    /// Search similar vectors
    async fn search(
        &self,
        query_vector: Vec<f32>,
        k: usize,
        filter: Option<serde_json::Value>,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Remove vector from index
    async fn remove(&mut self, id: Uuid) -> Result<bool>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: Uuid,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Document store contract
#[async_trait]
pub trait DocStore: Send + Sync {
    /// Store document
    async fn store(&mut self, doc: Document) -> Result<Uuid>;

    /// Retrieve document
    async fn get(&self, id: Uuid) -> Result<Option<Document>>;

    /// Query documents
    async fn query(&self, query: DocumentQuery) -> Result<Vec<Document>>;

    /// Delete document
    async fn delete(&mut self, id: Uuid) -> Result<bool>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentQuery {
    pub text: Option<String>,
    pub metadata_filter: Option<serde_json::Value>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Policy engine contract
#[async_trait]
pub trait Policy: Send + Sync {
    /// Evaluate if action is allowed
    async fn evaluate(
        &self,
        action: &PolicyAction,
        context: &TaskContext,
    ) -> Result<PolicyDecision>;

    /// Get risk score for action
    async fn risk_score(&self, action: &PolicyAction) -> Result<f32>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAction {
    pub tool_name: String,
    pub command: String,
    pub args: serde_json::Value,
    pub side_effects: Vec<SideEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: String,
    pub requires_confirmation: bool,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}
