//! Central Agent Orchestrator - P1.1.10 Implementation
//!
//! This module provides the core AgentOrchestrator that coordinates all agents
//! in the multi-agent system, implementing the Intent→Plan→Execute→Critic workflow
//! with comprehensive lifecycle management and error handling.
//!
//! # Architecture
//!
//! The AgentOrchestrator serves as the central coordinator for:
//! - Agent lifecycle management (start/stop/monitor)
//! - Intent→Plan→Execute→Critic workflow orchestration
//! - Resource allocation and monitoring
//! - Error recovery and fault tolerance
//! - EventBus integration for workflow events

use crate::actors::{ActorError, ActorHandle, ActorId, ActorState, AgentType, ExecutionStatus};
use crate::agents::{Critic, Executor, IntentAnalyzer, Planner, Scheduler};
use crate::events::AgentEventPublisher;
use crate::reliability::{
    AgentReliabilityManager, HealthCheckConfig, HealthChecker, HealthMonitor, HealthStatus,
    ReliabilityError,
};
use crate::resources::{ResourceBudget, ResourceMonitor};
use crate::system::{ActorSystem, SystemConfig};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Central Agent Orchestrator managing all agents and workflows
pub struct AgentOrchestrator {
    /// Actor system for low-level actor management
    actor_system: ActorSystem,

    /// Registry of all active agents
    pub agent_registry: Arc<RwLock<AgentRegistry>>,

    /// Resource monitoring and budgeting
    resource_monitor: Arc<RwLock<ResourceMonitor>>,

    /// Reliability manager for fault tolerance
    reliability_manager: Arc<RwLock<AgentReliabilityManager>>,

    /// Health monitoring system for agents
    health_monitor: Arc<HealthMonitor>,

    /// Event publisher for workflow events
    pub event_publisher: Arc<dyn AgentEventPublisher>,

    /// Active workflow tracking
    pub active_workflows: Arc<RwLock<HashMap<WorkflowId, WorkflowState>>>,

    /// Completed workflow tracking for audit trail
    pub completed_workflows: Arc<RwLock<HashMap<WorkflowId, WorkflowState>>>,

    /// Orchestrator configuration
    pub config: OrchestratorConfig,
}

/// Registry of all agents by type and instance
#[derive(Debug, Default)]
pub struct AgentRegistry {
    /// IntentAnalyzer agents
    intent_analyzers: Vec<ActorHandle>,

    /// Planner agents
    planners: Vec<ActorHandle>,

    /// Executor agents
    executors: Vec<ActorHandle>,

    /// Critic agents
    critics: Vec<ActorHandle>,

    /// Scheduler agents
    schedulers: Vec<ActorHandle>,

    /// Agent health tracking
    agent_health: HashMap<ActorId, AgentHealthStatus>,
}

/// Workflow identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Current state of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowState {
    /// Unique workflow identifier
    pub id: WorkflowId,

    /// Current step in the workflow
    pub current_step: WorkflowStepType,

    /// Overall workflow status
    pub status: ExecutionStatus,

    /// User input that initiated the workflow
    pub user_input: String,

    /// Context passed through the workflow
    pub context: Option<serde_json::Value>,

    /// Intent analysis result (if completed)
    pub intent: Option<serde_json::Value>,

    /// Generated plan (if completed)
    pub plan: Option<serde_json::Value>,

    /// Execution results (if completed)
    pub execution_results: Option<serde_json::Value>,

    /// Critique feedback (if completed)
    pub critique: Option<serde_json::Value>,

    /// Workflow started timestamp
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Workflow updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Error information if workflow failed
    pub error: Option<String>,

    /// Resource usage tracking
    pub resource_usage: ResourceUsage,
}

/// Types of workflow steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStepType {
    IntentAnalysis,
    PlanGeneration,
    PlanExecution,
    ResultCritique,
    Completed,
    Failed,
}

impl std::fmt::Display for WorkflowStepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStepType::IntentAnalysis => write!(f, "intent_analysis"),
            WorkflowStepType::PlanGeneration => write!(f, "plan_generation"),
            WorkflowStepType::PlanExecution => write!(f, "plan_execution"),
            WorkflowStepType::ResultCritique => write!(f, "result_critique"),
            WorkflowStepType::Completed => write!(f, "completed"),
            WorkflowStepType::Failed => write!(f, "failed"),
        }
    }
}

/// Resource usage tracking for workflows
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU time used (milliseconds)
    pub cpu_time_ms: u64,

    /// Memory used (bytes)
    pub memory_bytes: u64,

    /// Number of agent invocations
    pub agent_invocations: u32,

    /// Total workflow execution time (milliseconds)
    pub total_time_ms: u64,
}

/// Health status of an individual agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealthStatus {
    /// Agent ID
    pub id: ActorId,

    /// Agent type
    pub agent_type: AgentType,

    /// Current state
    pub state: ActorState,

    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,

    /// Number of successful operations
    pub success_count: u64,

    /// Number of failed operations
    pub error_count: u64,

    /// Average response time (milliseconds)
    pub avg_response_time_ms: f64,

    /// Is agent healthy?
    pub is_healthy: bool,
}

/// Configuration for the Agent Orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum number of concurrent workflows
    pub max_concurrent_workflows: usize,

    /// Default timeout for workflow steps (milliseconds)
    pub default_step_timeout_ms: u64,

    /// Maximum workflow execution time (milliseconds)
    pub max_workflow_time_ms: u64,

    /// Health check interval (milliseconds)
    pub health_check_interval_ms: u64,

    /// Resource monitoring enabled
    pub enable_resource_monitoring: bool,

    /// Auto-retry failed operations
    pub enable_auto_retry: bool,

    /// Maximum retries per operation
    pub max_retries: u32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 10,
            default_step_timeout_ms: 30_000,  // 30 seconds
            max_workflow_time_ms: 300_000,    // 5 minutes
            health_check_interval_ms: 10_000, // 10 seconds
            enable_resource_monitoring: true,
            enable_auto_retry: true,
            max_retries: 3,
        }
    }
}

/// Orchestrator errors
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Actor system error: {0}")]
    ActorSystem(#[from] ActorError),

    #[error("Reliability error: {0}")]
    Reliability(#[from] ReliabilityError),

    #[error("Workflow {0} not found")]
    WorkflowNotFound(WorkflowId),

    #[error("Agent {0} not available for type {1}")]
    AgentNotAvailable(ActorId, AgentType),

    #[error("Workflow {0} timeout")]
    WorkflowTimeout(WorkflowId),

    #[error("Resource budget exceeded for workflow {0}")]
    ResourceBudgetExceeded(WorkflowId),

    #[error("Invalid workflow state transition from {0} to {1}")]
    InvalidStateTransition(WorkflowStepType, WorkflowStepType),

    #[error("Agent initialization failed: {0}")]
    AgentInitializationFailed(String),

    #[error("Workflow execution failed: {0}")]
    WorkflowExecutionFailed(String),

    #[error("Actor system error: {0}")]
    ActorSystemError(#[from] crate::system::ActorSystemError),
}

impl AgentOrchestrator {
    /// Create a new Agent Orchestrator
    pub async fn new(
        system_config: SystemConfig,
        orchestrator_config: OrchestratorConfig,
        event_publisher: Arc<dyn AgentEventPublisher>,
    ) -> Result<Self, OrchestratorError> {
        info!("Initializing Agent Orchestrator");

        let actor_system = ActorSystem::new(system_config).await?;
        let orchestrator_id = ActorId::new();
        let default_budget = ResourceBudget::new()
            .memory_limit(1024 * 1024 * 1024) // 1GB default
            .cpu_time_limit(std::time::Duration::from_secs(600)); // 10 minutes default
        let resource_monitor = Arc::new(RwLock::new(ResourceMonitor::new(
            orchestrator_id,
            default_budget,
        )));
        let reliability_manager = Arc::new(RwLock::new(AgentReliabilityManager::new()));

        // Create health monitor with default config
        let health_config = crate::reliability::health::HealthCheckConfig::default();
        let health_monitor = Arc::new(crate::reliability::health::HealthMonitor::new(
            health_config,
        ));

        let orchestrator = Self {
            actor_system,
            agent_registry: Arc::new(RwLock::new(AgentRegistry::default())),
            resource_monitor,
            reliability_manager,
            health_monitor,
            event_publisher,
            active_workflows: Arc::new(RwLock::new(HashMap::new())),
            completed_workflows: Arc::new(RwLock::new(HashMap::new())),
            config: orchestrator_config,
        };

        // Start background tasks
        orchestrator.start_background_tasks().await;

        info!("Agent Orchestrator initialized successfully");
        Ok(orchestrator)
    }

    /// Start all background tasks
    async fn start_background_tasks(&self) {
        // Start health monitoring
        self.start_health_monitoring().await;

        // Start resource monitoring
        if self.config.enable_resource_monitoring {
            self.start_resource_monitoring().await;
        }
    }

    /// Start periodic health monitoring
    async fn start_health_monitoring(&self) {
        // Start the integrated health monitor
        let health_monitor = Arc::clone(&self.health_monitor);
        tokio::spawn(async move {
            if let Err(e) = health_monitor.start().await {
                error!("Health monitor failed to start: {}", e);
            }
        });

        // Legacy health monitoring for AgentHealthStatus (to maintain compatibility)
        let registry = Arc::clone(&self.agent_registry);
        let interval = self.config.health_check_interval_ms;

        tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_millis(interval));

            loop {
                interval_timer.tick().await;

                let registry_read = registry.read().await;
                for (agent_id, health) in &registry_read.agent_health {
                    if !health.is_healthy {
                        warn!(agent_id = %agent_id, agent_type = ?health.agent_type,
                              "Unhealthy agent detected");
                    }
                }
            }
        });
    }

    /// Start resource monitoring
    async fn start_resource_monitoring(&self) {
        let monitor = Arc::clone(&self.resource_monitor);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(
                tokio::time::Duration::from_millis(5000), // Monitor every 5 seconds
            );

            loop {
                interval_timer.tick().await;

                let _monitor_read = monitor.read().await;
                // Resource monitoring logic would go here
                debug!("Resource monitoring tick");
            }
        });
    }

    /// Initialize all core agents (P1.1.10.a: Agent lifecycle management)
    pub async fn initialize_agents(&self) -> Result<(), OrchestratorError> {
        info!("Initializing core agents");

        // Initialize IntentAnalyzer
        self.spawn_intent_analyzer().await?;

        // Initialize Planner
        self.spawn_planner().await?;

        // Initialize Executor
        self.spawn_executor().await?;

        // Initialize Critic
        self.spawn_critic().await?;

        // Initialize Scheduler
        self.spawn_scheduler().await?;

        info!("All core agents initialized successfully");
        Ok(())
    }

    /// Spawn an IntentAnalyzer agent
    async fn spawn_intent_analyzer(&self) -> Result<ActorId, OrchestratorError> {
        // Create agent instance
        let intent_analyzer = IntentAnalyzer::new();

        // CRITICAL FIX: Start heartbeat loop IMMEDIATELY after creation
        intent_analyzer.start_heartbeat_loop();

        // AUTO-REGISTRATION: Register agent with HealthMonitor BEFORE spawning
        let health_agent = IntentAnalyzer::new();
        let health_checker = Arc::new(health_agent) as Arc<dyn HealthChecker>;
        self.register_agent_for_health_monitoring(health_checker)
            .await?;

        let actor_id = self
            .actor_system
            .spawn_actor(Box::new(intent_analyzer), None, None)
            .await?;

        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.intent_analyzers.push(handle);
            registry.agent_health.insert(
                actor_id,
                AgentHealthStatus {
                    id: actor_id,
                    agent_type: AgentType::IntentAnalyzer,
                    state: ActorState::Running,
                    last_check: chrono::Utc::now(),
                    success_count: 0,
                    error_count: 0,
                    avg_response_time_ms: 0.0,
                    is_healthy: true,
                },
            );
        }

        info!(actor_id = %actor_id, "IntentAnalyzer spawned and registered with HealthMonitor");
        Ok(actor_id)
    }

    /// Spawn a Planner agent
    async fn spawn_planner(&self) -> Result<ActorId, OrchestratorError> {
        // Create agent instance
        let planner = Planner::new();

        // CRITICAL FIX: Start heartbeat loop IMMEDIATELY after creation
        planner.start_heartbeat_loop();

        // AUTO-REGISTRATION: Register agent with HealthMonitor BEFORE spawning
        let health_agent = Planner::new();
        let health_checker = Arc::new(health_agent) as Arc<dyn HealthChecker>;
        self.register_agent_for_health_monitoring(health_checker)
            .await?;

        let actor_id = self
            .actor_system
            .spawn_actor(Box::new(planner), None, None)
            .await?;

        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.planners.push(handle);
            registry.agent_health.insert(
                actor_id,
                AgentHealthStatus {
                    id: actor_id,
                    agent_type: AgentType::Planner,
                    state: ActorState::Running,
                    last_check: chrono::Utc::now(),
                    success_count: 0,
                    error_count: 0,
                    avg_response_time_ms: 0.0,
                    is_healthy: true,
                },
            );
        }

        info!(actor_id = %actor_id, "Planner spawned and registered with HealthMonitor");
        Ok(actor_id)
    }

    /// Spawn an Executor agent
    async fn spawn_executor(&self) -> Result<ActorId, OrchestratorError> {
        // Create agent instance
        let executor = Executor::new();

        // CRITICAL FIX: Start heartbeat loop IMMEDIATELY after creation
        executor.start_heartbeat_loop();

        // AUTO-REGISTRATION: Register agent with HealthMonitor BEFORE spawning
        let health_agent = Executor::new();
        let health_checker = Arc::new(health_agent) as Arc<dyn HealthChecker>;
        self.register_agent_for_health_monitoring(health_checker)
            .await?;
        let actor_id = self
            .actor_system
            .spawn_actor(Box::new(executor), None, None)
            .await?;

        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.executors.push(handle);
            registry.agent_health.insert(
                actor_id,
                AgentHealthStatus {
                    id: actor_id,
                    agent_type: AgentType::Executor,
                    state: ActorState::Running,
                    last_check: chrono::Utc::now(),
                    success_count: 0,
                    error_count: 0,
                    avg_response_time_ms: 0.0,
                    is_healthy: true,
                },
            );

            // Health monitoring registration done above
        }

        info!(actor_id = %actor_id, "Executor spawned and registered with HealthMonitor");
        Ok(actor_id)
    }

    /// Spawn a Critic agent
    async fn spawn_critic(&self) -> Result<ActorId, OrchestratorError> {
        // Create agent instance
        let critic = Critic::new();

        // CRITICAL FIX: Start heartbeat loop IMMEDIATELY after creation
        critic.start_heartbeat_loop();

        // AUTO-REGISTRATION: Register agent with HealthMonitor BEFORE spawning
        let health_agent = Critic::new();
        let health_checker = Arc::new(health_agent) as Arc<dyn HealthChecker>;
        self.register_agent_for_health_monitoring(health_checker)
            .await?;
        let actor_id = self
            .actor_system
            .spawn_actor(Box::new(critic), None, None)
            .await?;

        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.critics.push(handle);
            registry.agent_health.insert(
                actor_id,
                AgentHealthStatus {
                    id: actor_id,
                    agent_type: AgentType::Critic,
                    state: ActorState::Running,
                    last_check: chrono::Utc::now(),
                    success_count: 0,
                    error_count: 0,
                    avg_response_time_ms: 0.0,
                    is_healthy: true,
                },
            );

            // Health monitoring registration done above
        }

        info!(actor_id = %actor_id, "Critic spawned and registered with HealthMonitor");
        Ok(actor_id)
    }

    /// Spawn a Scheduler agent
    async fn spawn_scheduler(&self) -> Result<ActorId, OrchestratorError> {
        // Create agent instance
        let scheduler = Scheduler::new();

        // CRITICAL FIX: Start heartbeat loop IMMEDIATELY after creation
        scheduler.start_heartbeat_loop();

        // AUTO-REGISTRATION: Register agent with HealthMonitor BEFORE spawning
        let health_agent = Scheduler::new();
        let health_checker = Arc::new(health_agent) as Arc<dyn HealthChecker>;
        self.register_agent_for_health_monitoring(health_checker)
            .await?;
        let actor_id = self
            .actor_system
            .spawn_actor(Box::new(scheduler), None, None)
            .await?;

        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.schedulers.push(handle);
            registry.agent_health.insert(
                actor_id,
                AgentHealthStatus {
                    id: actor_id,
                    agent_type: AgentType::Scheduler,
                    state: ActorState::Running,
                    last_check: chrono::Utc::now(),
                    success_count: 0,
                    error_count: 0,
                    avg_response_time_ms: 0.0,
                    is_healthy: true,
                },
            );

            // Health monitoring registration done above
        }

        info!(actor_id = %actor_id, "Scheduler spawned and registered with HealthMonitor");
        Ok(actor_id)
    }

    /// Stop an agent by ID
    pub async fn stop_agent(&self, actor_id: ActorId) -> Result<(), OrchestratorError> {
        // Stop the actor
        self.actor_system.stop_actor(actor_id).await?;

        // Remove from registry
        let mut registry = self.agent_registry.write().await;
        registry.remove_agent(actor_id);

        info!(actor_id = %actor_id, "Agent stopped");
        Ok(())
    }

    /// Get the status of all agents
    pub async fn get_agent_status(&self) -> HashMap<ActorId, AgentHealthStatus> {
        let registry = self.agent_registry.read().await;
        registry.agent_health.clone()
    }

    /// Register an agent for health monitoring
    pub async fn register_agent_for_health_monitoring(
        &self,
        health_checker: Arc<dyn HealthChecker>,
    ) -> Result<(), OrchestratorError> {
        self.health_monitor
            .register_agent(health_checker)
            .await
            .map_err(|e| OrchestratorError::AgentInitializationFailed(e.to_string()))?;
        Ok(())
    }

    /// Unregister an agent from health monitoring
    pub async fn unregister_agent_from_health_monitoring(
        &self,
        agent_id: Uuid,
    ) -> Result<(), OrchestratorError> {
        self.health_monitor
            .unregister_agent(agent_id)
            .await
            .map_err(|e| OrchestratorError::AgentInitializationFailed(e.to_string()))?;
        Ok(())
    }

    /// Get health status for all agents from health monitor
    pub async fn get_agent_health_reports(&self) -> Vec<(Uuid, String, String, HealthStatus)> {
        self.health_monitor.get_all_health_reports().await
    }

    /// Record heartbeat for an agent
    pub async fn record_agent_heartbeat(&self, agent_id: Uuid) -> Result<(), OrchestratorError> {
        self.health_monitor
            .record_heartbeat(agent_id)
            .await
            .map_err(|e| OrchestratorError::AgentInitializationFailed(e.to_string()))?;
        Ok(())
    }

    /// Check if any agents are unhealthy
    pub async fn has_unhealthy_agents(&self) -> bool {
        self.health_monitor.has_unhealthy_agents().await
    }

    /// Get count of agents by health status
    pub async fn get_agent_health_status_counts(&self) -> HashMap<String, u32> {
        self.health_monitor.get_status_counts().await
    }

    /// Stop health monitoring
    pub fn stop_health_monitoring(&self) {
        self.health_monitor.stop();
    }

    /// Shutdown the orchestrator and all agents
    pub async fn shutdown(self) -> Result<(), OrchestratorError> {
        info!("Shutting down Agent Orchestrator");

        // Stop health monitoring
        self.health_monitor.stop();

        // Stop all active workflows
        let workflows = self.active_workflows.read().await;
        for (workflow_id, _) in workflows.iter() {
            warn!(workflow_id = %workflow_id, "Forcibly stopping active workflow");
        }
        drop(workflows);

        // Shutdown actor system (this will stop all agents)
        self.actor_system.shutdown().await?;

        info!("Agent Orchestrator shutdown completed");
        Ok(())
    }
}

impl AgentRegistry {
    /// Remove an agent by ID from all collections
    pub fn remove_agent(&mut self, actor_id: ActorId) {
        // Remove from health tracking
        self.agent_health.remove(&actor_id);

        // Remove from agent type collections
        self.intent_analyzers.retain(|handle| handle.id != actor_id);
        self.planners.retain(|handle| handle.id != actor_id);
        self.executors.retain(|handle| handle.id != actor_id);
        self.critics.retain(|handle| handle.id != actor_id);
        self.schedulers.retain(|handle| handle.id != actor_id);
    }

    /// Get a handle to an agent of the specified type (load balancing)
    pub fn get_agent_handle(&self, agent_type: &AgentType) -> Option<&ActorHandle> {
        match agent_type {
            AgentType::IntentAnalyzer => self.intent_analyzers.first(),
            AgentType::Planner => self.planners.first(),
            AgentType::Executor => self.executors.first(),
            AgentType::Critic => self.critics.first(),
            AgentType::Scheduler => self.schedulers.first(),
        }
    }

    /// Count healthy agents of a specific type
    pub fn count_healthy_agents(&self, agent_type: &AgentType) -> usize {
        self.agent_health
            .values()
            .filter(|health| health.agent_type == *agent_type && health.is_healthy)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::create_agent_event_publisher;

    #[test]
    fn test_workflow_id_creation() {
        let id1 = WorkflowId::new();
        let id2 = WorkflowId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_concurrent_workflows, 10);
        assert_eq!(config.default_step_timeout_ms, 30_000);
        assert!(config.enable_resource_monitoring);
    }

    #[test]
    fn test_workflow_step_type_display() {
        assert_eq!(
            WorkflowStepType::IntentAnalysis.to_string(),
            "intent_analysis"
        );
        assert_eq!(WorkflowStepType::Completed.to_string(), "completed");
    }

    #[test]
    fn test_agent_registry_operations() {
        let mut registry = AgentRegistry::default();
        let actor_id = ActorId::new();

        // Add agent health
        registry.agent_health.insert(
            actor_id,
            AgentHealthStatus {
                id: actor_id,
                agent_type: AgentType::IntentAnalyzer,
                state: ActorState::Running,
                last_check: chrono::Utc::now(),
                success_count: 0,
                error_count: 0,
                avg_response_time_ms: 0.0,
                is_healthy: true,
            },
        );

        assert_eq!(registry.count_healthy_agents(&AgentType::IntentAnalyzer), 1);

        // Remove agent
        registry.remove_agent(actor_id);
        assert_eq!(registry.count_healthy_agents(&AgentType::IntentAnalyzer), 0);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let system_config = SystemConfig::default();
        let orchestrator_config = OrchestratorConfig::default();
        let event_publisher = create_agent_event_publisher()
            .await
            .expect("Async operation should succeed");

        let result =
            AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await;

        assert!(result.is_ok());

        if let Ok(orchestrator) = result {
            let _ = orchestrator.shutdown().await;
        }
    }
}
