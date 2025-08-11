//! Domain models for MAGRAY following DDD principles

use crate::*;

/// Task represents a unit of work in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub intent: Option<Intent>,
    pub plan: Option<Plan>,
    pub context: TaskContext,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

/// Intent represents user intention analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub category: IntentCategory,
    pub confidence: f32,
    pub slots: std::collections::HashMap<String, serde_json::Value>,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentCategory {
    Chat,
    FileOperation,
    CodeGeneration,
    MemoryOperation,
    ToolExecution,
    Analysis,
    Planning,
}

/// Plan represents an action plan with dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub actions: Vec<Action>,
    pub dependencies: Vec<Dependency>,
    pub parallel_groups: Vec<ParallelGroup>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: Uuid,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub dry_run: bool,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: Uuid,
    pub to: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub actions: Vec<Uuid>,
    pub name: String,
}

/// ToolSpec represents a tool specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub version: String,
    pub commands: Vec<ToolCommand>,
    pub permissions: ToolPermissions,
    pub timeout_seconds: u64,
    pub side_effects: Vec<String>,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCommand {
    pub name: String,
    pub args_schema: serde_json::Value,
    pub description: String,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissions {
    pub fs_read: Vec<String>,
    pub fs_write: Vec<String>,
    pub net_access: Vec<String>,
    pub shell_access: bool,
    pub ui_access: bool,
}

/// Capability represents system capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Capability {
    FileSystem,
    Network,
    Shell,
    Gpu,
    Camera,
    Microphone,
}

/// MemoryRecord for vector memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: Uuid,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: MemoryMetadata,
    pub layer: MemoryLayer,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub recency: f32,
    pub frequency: f32,
    pub quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryLayer {
    Session,
    Semantic,
    Structured,
    KnowledgeGraph,
}

/// Task context containing environment and user info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub user_id: Option<String>,
    pub session_id: Uuid,
    pub working_directory: String,
    pub environment_vars: std::collections::HashMap<String, String>,
    pub project_context: Option<ProjectContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub name: String,
    pub language: Option<String>,
    pub repository: Option<String>,
    pub files: Vec<String>,
}
