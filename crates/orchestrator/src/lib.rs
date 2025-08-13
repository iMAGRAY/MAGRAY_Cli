//! Multi-Agent Orchestration System

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::items_after_test_module)]
#![allow(clippy::len_zero)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::new_without_default)]
#![allow(clippy::question_mark)]
#![allow(clippy::should_implement_trait)]
//!
//! This crate provides the core Actor System for MAGRAY CLI's multi-agent orchestration,
//! implementing a robust, fault-tolerant actor model with supervision, resource budgets,
//! and message passing between agents.
//!
//! # Architecture
//!
//! - **BaseActor**: Core actor trait with lifecycle management
//! - **ActorSystem**: Main system for actor registration and message routing  
//! - **ActorSystemManager**: Multi-agent orchestration manager with typed message passing
//! - **AgentMessage**: Typed messages for Intent→Plan→Execute→Critic workflow
//! - **Supervisor**: Fault tolerance with restart strategies and backoff
//! - **ResourceBudget**: CPU/memory/time limits with monitoring
//! - **EventBus**: Message passing infrastructure with backpressure
//!
//! # Usage
//!
//! ## Basic Actor System
//! ```no_run
//! use orchestrator::{ActorSystem, ResourceBudget, SystemConfig};
//! use tokio::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = SystemConfig::default();
//!     let system = ActorSystem::new(config).await?;
//!     
//!     // Register and start actors...
//!     
//!     system.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Multi-Agent Orchestration with AgentOrchestrator
//! ```no_run
//! use orchestrator::{AgentOrchestrator, OrchestratorConfig, SystemConfig, WorkflowRequest, TaskPriority};
//! use orchestrator::events::create_agent_event_publisher;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let system_config = SystemConfig::default();
//!     let orchestrator_config = OrchestratorConfig::default();
//!     let event_publisher = create_agent_event_publisher().await?;
//!     
//!     // Create orchestrator
//!     let orchestrator = AgentOrchestrator::new(
//!         system_config,
//!         orchestrator_config,
//!         event_publisher
//!     ).await?;
//!
//!     // Initialize all agents (IntentAnalyzer, Planner, Executor, Critic, Scheduler)
//!     orchestrator.initialize_agents().await?;
//!
//!     // Execute complete Intent→Plan→Execute→Critic workflow
//!     let request = WorkflowRequest {
//!         user_input: "Create a new user account".to_string(),
//!         context: None,
//!         priority: TaskPriority::Normal,
//!         dry_run: false,
//!         timeout_ms: None,
//!         config_overrides: None,
//!     };
//!
//!     let result = orchestrator.execute_workflow(request).await?;
//!     println!("Workflow completed: {}", result.success);
//!
//!     orchestrator.shutdown().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Lower-Level Actor System Management
//! ```no_run
//! use orchestrator::{ActorSystemManager, AgentCommunicationConfig, AgentType, AgentMessage, SystemConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let system_config = SystemConfig::default();
//!     let comm_config = AgentCommunicationConfig::default();
//!     let manager = ActorSystemManager::new(system_config, comm_config).await?;
//!
//!     // Send message to intent analyzer
//!     let message = AgentMessage::AnalyzeIntent {
//!         user_input: "Create a new user account".to_string(),
//!         context: None,
//!     };
//!     manager.send_to_agent_type(AgentType::IntentAnalyzer, message).await?;
//!
//!     manager.shutdown().await?;
//!     Ok(())
//! }
//! ```

pub mod actors;
pub mod agents;
pub mod events;
pub mod orchestrator;
pub mod reliability;
pub mod resources;
pub mod saga;
pub mod system;
pub mod workflow;

pub use actors::{
    ActorError, ActorHandle, ActorHealth, ActorId, ActorMessage, ActorState, ActorSystemManager,
    AgentCommunicationConfig, AgentMessage, AgentSystemStats, AgentType, BaseActor,
    ExecutionStatus, TaskPriority,
};
pub use agents::{
    Critic, CriticTrait, Executor, ExecutorTrait, IntentAnalyzer, IntentAnalyzerTrait, Planner,
    PlannerTrait, Scheduler, SchedulerTrait,
};
pub use events::{
    create_agent_event_publisher, AgentEventPublisher, AgentLifecycleEvent, AgentMessageEvent,
    AgentStatus, AgentTopics, MessageStatus, SchedulerEvent, SchedulerEventType, WorkflowEvent,
    WorkflowStep,
};
pub use reliability::{
    AgentReliabilityConfig, AgentReliabilityManager, AgentReliabilityStats, BackoffStrategy,
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerState,
    OperationTimeoutError, ReliabilityError, RetryConfig, RetryError, RetryPolicy, TimeoutConfig,
    TimeoutManager,
};
pub use resources::{
    BudgetViolation, EnforcementPolicy, ResourceBudget, ResourceLimits, ResourceMonitor,
};
pub use system::{ActorSystem, ActorSystemError, RestartStrategy, Supervisor, SystemConfig};

// Re-export orchestrator and workflow types
pub use orchestrator::{
    AgentHealthStatus, AgentOrchestrator, AgentRegistry, OrchestratorConfig, OrchestratorError,
    ResourceUsage, WorkflowId, WorkflowState, WorkflowStepType,
};
pub use saga::{
    CompensationHandler, CompensationStatus, CompensationStep, CompensationType,
    DefaultSagaManager, Saga, SagaCompensationResult, SagaExecutionResult, SagaManager, SagaStatus,
    SagaStep, SagaStepStatus,
};
pub use workflow::{WorkflowConfig, WorkflowRequest, WorkflowResult};

/// Result type for Actor System operations
pub type ActorResult<T> = Result<T, ActorError>;

/// System-wide error types
#[derive(Debug, thiserror::Error)]
pub enum OrchestrationError {
    #[error("Actor error: {0}")]
    Actor(#[from] ActorError),

    #[error("Resource budget violated: {0}")]
    BudgetViolation(#[from] BudgetViolation),

    #[error("System initialization failed: {0}")]
    SystemInit(String),

    #[error("Message routing failed: {0}")]
    MessageRouting(String),

    #[error("Supervisor error: {0}")]
    Supervisor(String),

    #[error("Actor System error: {0}")]
    System(#[from] ActorSystemError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_id_creation() {
        let id1 = ActorId::new();
        let id2 = ActorId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_resource_budget_creation() {
        let budget = ResourceBudget::new()
            .memory_limit(1024 * 1024)
            .cpu_time_limit(std::time::Duration::from_secs(30));

        assert!(budget.limits.max_memory.is_some());
        assert!(budget.limits.max_cpu_time.is_some());
    }

    #[tokio::test]
    async fn test_actor_system_creation() {
        let config = SystemConfig::default();
        let result = ActorSystem::new(config).await;
        assert!(result.is_ok());

        if let Ok(system) = result {
            let _ = system.shutdown().await;
        }
    }
}
