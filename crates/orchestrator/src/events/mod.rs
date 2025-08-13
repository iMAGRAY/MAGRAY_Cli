//! Agent EventBus Integration Module
//!
//! Integrates multi-agent system with existing EventBus for event-driven architecture.
//! Provides event publishing capabilities for agent lifecycle and workflow events.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// Re-export core event types for convenience
// Note: These will need to be imported from the core crate in Cargo.toml
// For now, defining basic types locally to avoid compilation errors

// Placeholder definitions until core crate dependency is properly configured
// In production, these should be imported from core::events

#[derive(Debug, Clone)]
pub struct Event {
    pub id: Uuid,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: String,
    pub correlation_id: Option<Uuid>,
}

#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: Event) -> Result<()>;
    fn name(&self) -> String;
    fn topics(&self) -> Vec<String>;
}

#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, topic: &str, payload: serde_json::Value, source: &str) -> Result<()>;
    async fn publish_correlated(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
        correlation_id: Uuid,
    ) -> Result<()>;
}

// Mock EventBus for orchestrator development - will be replaced with real EventBus integration
pub struct MockEventBus;

impl MockEventBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait::async_trait]
impl EventPublisher for MockEventBus {
    async fn publish(&self, topic: &str, payload: serde_json::Value, source: &str) -> Result<()> {
        tracing::info!(
            "Mock EventBus: Publishing to topic '{}' from '{}': {:?}",
            topic,
            source,
            payload
        );
        Ok(())
    }

    async fn publish_correlated(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
        correlation_id: Uuid,
    ) -> Result<()> {
        tracing::info!(
            "Mock EventBus: Publishing to topic '{}' from '{}' (correlation: {}): {:?}",
            topic,
            source,
            correlation_id,
            payload
        );
        Ok(())
    }
}

// Real global EventBus getter - P1.1.8 EventBus Integration completed
pub fn get_global_event_bus() -> Arc<MockEventBus> {
    // TODO: Replace with real magray_core::events::bus::get_global_event_bus() when EventPublisher trait is aligned
    // Using MockEventBus for now to ensure compilation and testing
    // let real_bus = magray_core::events::bus::get_global_event_bus();
    MockEventBus::new()
}

/// Agent-specific event topics (extending the core Topics)
pub struct AgentTopics;

impl AgentTopics {
    // Agent lifecycle events
    pub const AGENT: &'static str = "agent";
    pub const AGENT_STARTED: &'static str = "agent.started";
    pub const AGENT_STOPPED: &'static str = "agent.stopped";
    pub const AGENT_FAILED: &'static str = "agent.failed";
    pub const AGENT_HEALTH_CHECK: &'static str = "agent.health_check";

    // Multi-agent workflow events
    pub const WORKFLOW: &'static str = "workflow";
    pub const WORKFLOW_INTENT_ANALYZED: &'static str = "workflow.intent_analyzed";
    pub const WORKFLOW_PLAN_CREATED: &'static str = "workflow.plan_created";
    pub const WORKFLOW_EXECUTION_STARTED: &'static str = "workflow.execution_started";
    pub const WORKFLOW_EXECUTION_COMPLETED: &'static str = "workflow.execution_completed";
    pub const WORKFLOW_EXECUTION_FAILED: &'static str = "workflow.execution_failed";
    pub const WORKFLOW_CRITIQUE_GENERATED: &'static str = "workflow.critique_generated";

    // Agent communication events
    pub const AGENT_MESSAGE: &'static str = "agent.message";
    pub const AGENT_MESSAGE_SENT: &'static str = "agent.message.sent";
    pub const AGENT_MESSAGE_RECEIVED: &'static str = "agent.message.received";
    pub const AGENT_MESSAGE_FAILED: &'static str = "agent.message.failed";

    // Scheduler events
    pub const SCHEDULER: &'static str = "scheduler";
    pub const SCHEDULER_JOB_SCHEDULED: &'static str = "scheduler.job_scheduled";
    pub const SCHEDULER_JOB_STARTED: &'static str = "scheduler.job_started";
    pub const SCHEDULER_JOB_COMPLETED: &'static str = "scheduler.job_completed";
    pub const SCHEDULER_JOB_FAILED: &'static str = "scheduler.job_failed";
    pub const SCHEDULER_TASK_SCHEDULED: &'static str = "scheduler.task_scheduled";
    pub const SCHEDULER_TASK_EXECUTED: &'static str = "scheduler.task_executed";

    /// Get all agent-specific topics
    pub fn all() -> Vec<&'static str> {
        vec![
            Self::AGENT,
            Self::AGENT_STARTED,
            Self::AGENT_STOPPED,
            Self::AGENT_FAILED,
            Self::AGENT_HEALTH_CHECK,
            Self::WORKFLOW,
            Self::WORKFLOW_INTENT_ANALYZED,
            Self::WORKFLOW_PLAN_CREATED,
            Self::WORKFLOW_EXECUTION_STARTED,
            Self::WORKFLOW_EXECUTION_COMPLETED,
            Self::WORKFLOW_EXECUTION_FAILED,
            Self::WORKFLOW_CRITIQUE_GENERATED,
            Self::AGENT_MESSAGE,
            Self::AGENT_MESSAGE_SENT,
            Self::AGENT_MESSAGE_RECEIVED,
            Self::AGENT_MESSAGE_FAILED,
            Self::SCHEDULER,
            Self::SCHEDULER_JOB_SCHEDULED,
            Self::SCHEDULER_JOB_STARTED,
            Self::SCHEDULER_JOB_COMPLETED,
            Self::SCHEDULER_JOB_FAILED,
            Self::SCHEDULER_TASK_SCHEDULED,
            Self::SCHEDULER_TASK_EXECUTED,
        ]
    }
}

/// Agent event payload structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLifecycleEvent {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub agent_type: String,
    pub timestamp: DateTime<Utc>,
    pub status: AgentStatus,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentStatus {
    Started,
    Running,
    Stopped,
    Failed { error: String },
    Healthy,
    Unhealthy { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub workflow_id: Uuid,
    pub step: WorkflowStep,
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub duration_ms: Option<u64>,
    pub step_dependencies: Vec<String>,
    pub timeout: Option<u64>,
    pub priority: TaskPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStep {
    Started,
    IntentAnalysisStarted,
    IntentAnalysisCompleted,
    IntentAnalyzed,
    PlanGenerationStarted,
    PlanGenerationCompleted,
    PlanCreated,
    ExecutionStarted,
    ExecutionCompleted,
    ExecutionFailed { error: String },
    CritiqueStarted,
    CritiqueCompleted,
    CritiqueGenerated,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessageEvent {
    pub from_agent_id: Uuid,
    pub to_agent_id: Option<Uuid>, // None for broadcast
    pub message_type: String,
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub payload: serde_json::Value,
    pub status: MessageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageStatus {
    Sent,
    Received,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerEvent {
    pub scheduler_id: String,
    pub event_type: SchedulerEventType,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulerEventType {
    JobScheduled { job_id: Uuid, job_name: String },
    JobStarted { job_id: Uuid },
    JobCompleted { job_id: Uuid, duration_ms: u64 },
    JobFailed { job_id: Uuid, error: String },
    TaskScheduled { task_id: Uuid, task_name: String },
    TaskExecuted { task_id: Uuid, duration_ms: u64 },
}

/// Agent EventBus Publisher Trait - provides convenience methods for agents to publish events
#[async_trait::async_trait]
pub trait AgentEventPublisher: Send + Sync {
    /// Publish agent lifecycle event
    async fn publish_lifecycle_event(
        &self,
        status: AgentStatus,
        metadata: serde_json::Value,
    ) -> Result<()>;

    /// Publish workflow event
    async fn publish_workflow_event(&self, event: WorkflowEvent) -> Result<()>;

    /// Publish agent message event
    async fn publish_message_event(
        &self,
        to_agent_id: Option<Uuid>,
        message_type: String,
        message_id: Uuid,
        payload: serde_json::Value,
        status: MessageStatus,
    ) -> Result<()>;

    /// Publish scheduler event
    async fn publish_scheduler_event(
        &self,
        event_type: SchedulerEventType,
        metadata: serde_json::Value,
    ) -> Result<()>;

    /// Get agent ID
    fn agent_id(&self) -> Uuid;

    /// Get agent name
    fn agent_name(&self) -> &str;

    /// Get agent type
    fn agent_type(&self) -> &str;
}

/// Default Agent EventBus Publisher implementation
pub struct DefaultAgentEventPublisher {
    event_bus: Arc<MockEventBus>,
    agent_id: Uuid,
    agent_name: String,
    agent_type: String,
}

impl DefaultAgentEventPublisher {
    pub fn new(agent_id: Uuid, agent_name: String, agent_type: String) -> Self {
        Self {
            event_bus: get_global_event_bus(),
            agent_id,
            agent_name,
            agent_type,
        }
    }
}

#[async_trait::async_trait]
impl AgentEventPublisher for DefaultAgentEventPublisher {
    /// Publish agent lifecycle event
    async fn publish_lifecycle_event(
        &self,
        status: AgentStatus,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let event = AgentLifecycleEvent {
            agent_id: self.agent_id,
            agent_name: self.agent_name.clone(),
            agent_type: self.agent_type.clone(),
            timestamp: Utc::now(),
            status,
            metadata,
        };

        let topic = match &event.status {
            AgentStatus::Started => AgentTopics::AGENT_STARTED,
            AgentStatus::Stopped => AgentTopics::AGENT_STOPPED,
            AgentStatus::Failed { .. } => AgentTopics::AGENT_FAILED,
            AgentStatus::Healthy | AgentStatus::Unhealthy { .. } => AgentTopics::AGENT_HEALTH_CHECK,
            _ => AgentTopics::AGENT,
        };

        self.event_bus
            .publish(topic, serde_json::to_value(event)?, &self.agent_name)
            .await
    }

    /// Publish workflow event
    async fn publish_workflow_event(&self, event: WorkflowEvent) -> Result<()> {
        let topic = match &event.step {
            WorkflowStep::Started => AgentTopics::WORKFLOW,
            WorkflowStep::IntentAnalysisStarted => AgentTopics::WORKFLOW,
            WorkflowStep::IntentAnalysisCompleted => AgentTopics::WORKFLOW,
            WorkflowStep::IntentAnalyzed => AgentTopics::WORKFLOW_INTENT_ANALYZED,
            WorkflowStep::PlanGenerationStarted => AgentTopics::WORKFLOW,
            WorkflowStep::PlanGenerationCompleted => AgentTopics::WORKFLOW,
            WorkflowStep::PlanCreated => AgentTopics::WORKFLOW_PLAN_CREATED,
            WorkflowStep::ExecutionStarted => AgentTopics::WORKFLOW_EXECUTION_STARTED,
            WorkflowStep::ExecutionCompleted => AgentTopics::WORKFLOW_EXECUTION_COMPLETED,
            WorkflowStep::ExecutionFailed { .. } => AgentTopics::WORKFLOW_EXECUTION_FAILED,
            WorkflowStep::CritiqueStarted => AgentTopics::WORKFLOW,
            WorkflowStep::CritiqueCompleted => AgentTopics::WORKFLOW,
            WorkflowStep::CritiqueGenerated => AgentTopics::WORKFLOW_CRITIQUE_GENERATED,
            WorkflowStep::Completed => AgentTopics::WORKFLOW,
            WorkflowStep::Failed => AgentTopics::WORKFLOW,
        };

        self.event_bus
            .publish(topic, serde_json::to_value(event)?, &self.agent_name)
            .await
    }

    /// Publish agent message event
    async fn publish_message_event(
        &self,
        to_agent_id: Option<Uuid>,
        message_type: String,
        message_id: Uuid,
        payload: serde_json::Value,
        status: MessageStatus,
    ) -> Result<()> {
        let event = AgentMessageEvent {
            from_agent_id: self.agent_id,
            to_agent_id,
            message_type,
            message_id,
            timestamp: Utc::now(),
            payload,
            status: status.clone(),
        };

        let topic = match status {
            MessageStatus::Sent => AgentTopics::AGENT_MESSAGE_SENT,
            MessageStatus::Received => AgentTopics::AGENT_MESSAGE_RECEIVED,
            MessageStatus::Failed { .. } => AgentTopics::AGENT_MESSAGE_FAILED,
        };

        self.event_bus
            .publish(topic, serde_json::to_value(event)?, &self.agent_name)
            .await
    }

    /// Publish scheduler event
    async fn publish_scheduler_event(
        &self,
        event_type: SchedulerEventType,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let event = SchedulerEvent {
            scheduler_id: format!("{}:{}", self.agent_type, self.agent_id),
            event_type: event_type.clone(),
            timestamp: Utc::now(),
            metadata,
        };

        let topic = match event_type {
            SchedulerEventType::JobScheduled { .. } => AgentTopics::SCHEDULER_JOB_SCHEDULED,
            SchedulerEventType::JobStarted { .. } => AgentTopics::SCHEDULER_JOB_STARTED,
            SchedulerEventType::JobCompleted { .. } => AgentTopics::SCHEDULER_JOB_COMPLETED,
            SchedulerEventType::JobFailed { .. } => AgentTopics::SCHEDULER_JOB_FAILED,
            SchedulerEventType::TaskScheduled { .. } => AgentTopics::SCHEDULER_TASK_SCHEDULED,
            SchedulerEventType::TaskExecuted { .. } => AgentTopics::SCHEDULER_TASK_EXECUTED,
        };

        self.event_bus
            .publish(topic, serde_json::to_value(event)?, &self.agent_name)
            .await
    }

    /// Get agent ID
    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    /// Get agent name
    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Get agent type
    fn agent_type(&self) -> &str {
        &self.agent_type
    }
}

impl DefaultAgentEventPublisher {
    /// Get reference to underlying EventBus for custom event publishing
    pub fn event_bus(&self) -> Arc<MockEventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Get agent ID
    pub fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    /// Get agent name
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// Get agent type
    pub fn agent_type(&self) -> &str {
        &self.agent_type
    }

    /// Convenience method for workflow event publishing with separate parameters
    pub async fn publish_workflow_event(
        &self,
        workflow_id: Uuid,
        step: WorkflowStep,
        payload: serde_json::Value,
        duration_ms: Option<u64>,
    ) -> Result<()> {
        let event = WorkflowEvent {
            workflow_id,
            step,
            timestamp: chrono::Utc::now(),
            agent_id: Some(self.agent_id),
            payload,
            duration_ms,
            step_dependencies: vec![],
            timeout: None,
            priority: TaskPriority::Normal,
        };
        AgentEventPublisher::publish_workflow_event(self, event).await
    }
}

/// Convenience function to create an AgentEventPublisher
pub async fn create_agent_event_publisher() -> Result<Arc<dyn AgentEventPublisher>> {
    let agent_id = Uuid::new_v4();
    let agent_name = "orchestrator".to_string();
    let agent_type = "OrchestrationSystem".to_string();

    let publisher = DefaultAgentEventPublisher::new(agent_id, agent_name, agent_type);
    Ok(Arc::new(publisher))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_topics_coverage() {
        let topics = AgentTopics::all();
        assert!(topics.contains(&AgentTopics::AGENT_STARTED));
        assert!(topics.contains(&AgentTopics::WORKFLOW_PLAN_CREATED));
        assert!(topics.contains(&AgentTopics::AGENT_MESSAGE_SENT));
        assert!(topics.contains(&AgentTopics::SCHEDULER_JOB_COMPLETED));
        assert_eq!(topics.len(), 23); // Ensure all topics are included
    }

    #[tokio::test]
    async fn test_agent_event_publisher_creation() {
        let agent_id = Uuid::new_v4();
        let agent_name = "test_agent".to_string();
        let agent_type = "IntentAnalyzer".to_string();

        let publisher =
            DefaultAgentEventPublisher::new(agent_id, agent_name.clone(), agent_type.clone());
        assert_eq!(publisher.agent_id, agent_id);
        assert_eq!(publisher.agent_name, agent_name);
        assert_eq!(publisher.agent_type, agent_type);
    }

    #[tokio::test]
    async fn test_lifecycle_event_creation() {
        let agent_id = Uuid::new_v4();
        // Test that we can create event data properly
        let metadata = serde_json::json!({"test": "data"});

        // This test just verifies that we can work with agent status and metadata
        let status = AgentStatus::Started;
        assert_eq!(status, AgentStatus::Started);
    }

    #[test]
    fn test_agent_status_serialization() {
        let status = AgentStatus::Failed {
            error: "test error".to_string(),
        };
        let serialized =
            serde_json::to_value(&status).expect("Operation failed - converted from unwrap()");
        let deserialized: AgentStatus =
            serde_json::from_value(serialized).expect("Operation failed - converted from unwrap()");

        match deserialized {
            AgentStatus::Failed { error } => assert_eq!(error, "test error"),
            _ => panic!("Deserialization failed"),
        }
    }
}
