//! Actor System Core Components
//!
//! This module provides the fundamental building blocks for the Actor System:
//! - BaseActor trait for all actors
//! - ActorHandle for communication
//! - ActorId for unique identification
//! - Error types and lifecycle management

pub mod actor_system;
pub mod base_actor;

pub use actor_system::{ActorSystemManager, AgentCommunicationConfig, AgentSystemStats, AgentType};
pub use base_actor::{ActorContext, ActorLifecycle, BaseActor};

// Export the new agent message types (define them in this module)
// pub use self::{AgentMessage, ExecutionStatus, TaskPriority};

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for actors in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorId(pub Uuid);

impl ActorId {
    /// Create a new random ActorId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create ActorId from a UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }
}

impl fmt::Display for ActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ActorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for communicating with an actor
#[derive(Debug, Clone)]
pub struct ActorHandle {
    pub id: ActorId,
    pub sender: tokio::sync::mpsc::UnboundedSender<ActorMessage>,
}

impl ActorHandle {
    /// Send a message to this actor
    pub async fn send(&self, message: ActorMessage) -> Result<(), ActorError> {
        self.sender
            .send(message)
            .map_err(|_| ActorError::MessageSendFailed(self.id))
    }

    /// Get the actor's ID
    pub fn id(&self) -> ActorId {
        self.id
    }
}

/// Messages that can be sent to actors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorMessage {
    /// Start the actor
    Start,

    /// Stop the actor gracefully
    Stop,

    /// Restart the actor
    Restart,

    /// Ping for health check
    Ping,

    /// Custom message with payload
    Custom {
        message_type: String,
        payload: serde_json::Value,
    },

    /// System message from supervisor
    System { command: SystemCommand },

    /// Agent-specific messages for multi-agent orchestration
    Agent(AgentMessage),
}

/// Specialized messages for multi-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    /// Intent analysis request
    AnalyzeIntent {
        user_input: String,
        context: Option<serde_json::Value>,
    },

    /// Intent analysis result
    IntentAnalyzed {
        intent: serde_json::Value,
        confidence: f64,
        suggested_actions: Vec<String>,
    },

    /// Plan creation request
    CreatePlan {
        intent: serde_json::Value,
        constraints: Option<serde_json::Value>,
    },

    /// Plan creation result
    PlanCreated {
        plan: serde_json::Value,
        estimated_time: Option<u64>,
        resource_requirements: Option<serde_json::Value>,
    },

    /// Plan execution request
    ExecutePlan {
        plan: serde_json::Value,
        dry_run: bool,
    },

    /// Plan execution progress
    ExecutionProgress {
        step_index: usize,
        step_name: String,
        status: ExecutionStatus,
        result: Option<serde_json::Value>,
    },

    /// Plan execution completed
    ExecutionCompleted {
        plan_id: String,
        success: bool,
        results: serde_json::Value,
        execution_time: u64,
    },

    /// Critique/analysis request
    CritiqueResult {
        result: serde_json::Value,
        context: Option<serde_json::Value>,
    },

    /// Critique/analysis response
    CritiqueCompleted {
        feedback: serde_json::Value,
        suggestions: Vec<String>,
        quality_score: f64,
    },

    /// Schedule task request
    ScheduleTask {
        task: serde_json::Value,
        priority: TaskPriority,
        delay: Option<u64>,
    },

    /// Task scheduled confirmation
    TaskScheduled {
        task_id: String,
        scheduled_at: chrono::DateTime<chrono::Utc>,
    },

    /// Generic request/response for extensibility
    Request {
        request_id: String,
        request_type: String,
        payload: serde_json::Value,
    },

    /// Generic response
    Response {
        request_id: String,
        response_type: String,
        payload: serde_json::Value,
        success: bool,
        error: Option<String>,
    },
}

/// Execution status for plan steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Task priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// System commands from supervisors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemCommand {
    /// Force immediate shutdown
    Shutdown,

    /// Update resource budget
    UpdateBudget(crate::resources::ResourceBudget),

    /// Report health status
    ReportHealth,

    /// Pause processing
    Pause,

    /// Resume processing
    Resume,
}

/// Actor-specific errors
#[derive(Debug, thiserror::Error)]
pub enum ActorError {
    #[error("Actor {0} not found")]
    NotFound(ActorId),

    #[error("Actor {0} already exists")]
    AlreadyExists(ActorId),

    #[error("Failed to send message to actor {0}")]
    MessageSendFailed(ActorId),

    #[error("Actor {0} failed to start: {1}")]
    StartupFailed(ActorId, String),

    #[error("Actor {0} crashed: {1}")]
    Crashed(ActorId, String),

    #[error("Actor {0} timeout: {1}")]
    Timeout(ActorId, String),

    #[error("Resource budget violated for actor {0}: {1}")]
    BudgetViolation(ActorId, String),

    #[error("Actor {0} initialization failed: {1}")]
    InitializationFailed(ActorId, String),

    #[error("Actor {0} message handling failed: {1}")]
    MessageHandlingFailed(ActorId, String),

    #[error("System error: {0}")]
    SystemError(String),
}

/// Current state of an actor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActorState {
    /// Actor is being initialized
    Initializing,

    /// Actor is running normally
    Running,

    /// Actor is paused
    Paused,

    /// Actor is stopping
    Stopping,

    /// Actor has stopped
    Stopped,

    /// Actor has crashed
    Crashed,

    /// Actor is being restarted
    Restarting,
}

impl fmt::Display for ActorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActorState::Initializing => write!(f, "initializing"),
            ActorState::Running => write!(f, "running"),
            ActorState::Paused => write!(f, "paused"),
            ActorState::Stopping => write!(f, "stopping"),
            ActorState::Stopped => write!(f, "stopped"),
            ActorState::Crashed => write!(f, "crashed"),
            ActorState::Restarting => write!(f, "restarting"),
        }
    }
}

/// Health status of an actor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorHealth {
    pub id: ActorId,
    pub state: ActorState,
    pub uptime: chrono::Duration,
    pub messages_processed: u64,
    pub last_error: Option<String>,
    pub resource_usage: crate::resources::ResourceUsage,
    pub restart_count: u32,
}

impl ActorHealth {
    pub fn new(id: ActorId) -> Self {
        Self {
            id,
            state: ActorState::Initializing,
            uptime: chrono::Duration::zero(),
            messages_processed: 0,
            last_error: None,
            resource_usage: Default::default(),
            restart_count: 0,
        }
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.state, ActorState::Running | ActorState::Paused)
    }

    pub fn is_crashed(&self) -> bool {
        matches!(self.state, ActorState::Crashed)
    }
}

/// Convert from common errors to ActorError
impl From<anyhow::Error> for ActorError {
    fn from(error: anyhow::Error) -> Self {
        ActorError::SystemError(format!("Error: {}", error))
    }
}

impl From<tokio::task::JoinError> for ActorError {
    fn from(error: tokio::task::JoinError) -> Self {
        ActorError::SystemError(format!("Task join error: {}", error))
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for ActorError {
    fn from(error: tokio::sync::oneshot::error::RecvError) -> Self {
        ActorError::SystemError(format!("Channel receive error: {}", error))
    }
}
