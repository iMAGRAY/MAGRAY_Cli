use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::planner::{
    ActionPlan, ActionStep, ActionStepType, InteractionType, MemoryOperationType,
};
use crate::saga::{DefaultSagaManager, Saga, SagaManager, SagaStatus};

// Health monitoring integration
use crate::reliability::health::{HealthChecker, HealthReport, HealthStatus};

/// Result of plan execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub plan_id: Uuid,
    pub status: ExecutionStatus,
    pub step_results: Vec<StepResult>,
    pub execution_time: std::time::Duration,
    pub resource_usage: ResourceUsage,
    pub metadata: HashMap<String, serde_json::Value>,
    pub error: Option<ExecutionError>,
}

/// Status of plan execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Result of individual step execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: Uuid,
    pub status: StepStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time: std::time::Duration,
    pub retry_count: u32,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of step execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Retrying,
}

/// Resource usage during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_ms: u64,
    pub memory_peak_mb: u64,
    pub disk_reads: u64,
    pub disk_writes: u64,
    pub network_requests: u64,
    pub tool_invocations: u64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time_ms: 0,
            memory_peak_mb: 0,
            disk_reads: 0,
            disk_writes: 0,
            network_requests: 0,
            tool_invocations: 0,
        }
    }
}

/// Execution error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub error_type: ExecutionErrorType,
    pub message: String,
    pub step_id: Option<Uuid>,
    pub retryable: bool,
    pub details: HashMap<String, serde_json::Value>,
}

/// Types of execution errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionErrorType {
    ToolNotFound,
    ToolExecutionFailed,
    InvalidParameters,
    ResourceExhausted,
    NetworkError,
    TimeoutError,
    PermissionDenied,
    DependencyFailed,
    UserCancelled,
    SystemError,
}

/// Execution context for state tracking
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub plan_id: Uuid,
    pub current_step: usize,
    pub step_states: HashMap<Uuid, StepState>,
    pub shared_data: HashMap<String, serde_json::Value>,
    pub resource_limits: ResourceLimits,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

/// State of individual execution steps
#[derive(Debug, Clone)]
pub struct StepState {
    pub status: StepStatus,
    pub result: Option<serde_json::Value>,
    pub start_time: Option<std::time::Instant>,
    pub end_time: Option<std::time::Instant>,
    pub retry_count: u32,
}

/// Resource limits for execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_time_ms: u64,
    pub max_memory_mb: u64,
    pub max_disk_usage_mb: u64,
    pub max_network_requests: u64,
    pub max_execution_time: std::time::Duration,
}

/// Trait for plan execution functionality
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    /// Execute an action plan
    async fn execute_plan(&self, plan: &ActionPlan) -> Result<ExecutionResult>;

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &ActionStep,
        context: &mut ExecutionContext,
    ) -> Result<StepResult>;

    /// Cancel a running execution
    async fn cancel_execution(&self, plan_id: Uuid) -> Result<()>;

    /// Pause execution (if supported)
    async fn pause_execution(&self, plan_id: Uuid) -> Result<()>;

    /// Resume paused execution
    async fn resume_execution(&self, plan_id: Uuid) -> Result<()>;

    /// Get execution status
    async fn get_execution_status(&self, plan_id: Uuid) -> Result<ExecutionStatus>;

    /// Rollback failed execution
    async fn rollback_execution(&self, plan_id: Uuid) -> Result<()>;

    /// Execute plan with saga transaction management
    async fn execute_plan_with_saga(&self, plan: &ActionPlan) -> Result<ExecutionResult>;

    /// Get saga status for a plan
    async fn get_saga_status(&self, plan_id: Uuid) -> Result<Option<SagaStatus>>;
}

/// Plan Executor implementation
pub struct Executor {
    agent_id: Uuid,
    active_executions: dashmap::DashMap<Uuid, ExecutionContext>,
    resource_monitor: Option<sysinfo::System>,
    tool_registry: HashMap<String, Box<dyn ToolInvoker>>,
    saga_manager: DefaultSagaManager,
    active_sagas: dashmap::DashMap<Uuid, Saga>,
    // Health monitoring fields
    last_heartbeat: Arc<RwLock<Option<DateTime<Utc>>>>,
    error_count: Arc<AtomicU32>,
    start_time: Instant,
}

/// Trait for tool invocation
#[async_trait]
pub trait ToolInvoker: Send + Sync {
    async fn invoke(&self, args: HashMap<String, serde_json::Value>) -> Result<serde_json::Value>;
    fn get_name(&self) -> &str;
}

/// Mock tool invoker for testing
pub struct MockToolInvoker {
    name: String,
}

impl MockToolInvoker {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl ToolInvoker for MockToolInvoker {
    async fn invoke(&self, args: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        // Simulate tool execution
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(serde_json::json!({
            "tool": self.name,
            "result": "success",
            "args": args,
            "executed_at": chrono::Utc::now()
        }))
    }

    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Executor {
    /// Create new Executor instance
    pub fn new() -> Self {
        let mut tool_registry: HashMap<String, Box<dyn ToolInvoker>> = HashMap::new();

        // Register some mock tools for testing
        // TODO: Integrate with existing tool system from crates/tools
        tool_registry.insert(
            "file_reader".to_string(),
            Box::new(MockToolInvoker::new("file_reader".to_string())),
        );
        tool_registry.insert(
            "file_writer".to_string(),
            Box::new(MockToolInvoker::new("file_writer".to_string())),
        );
        tool_registry.insert(
            "file_search".to_string(),
            Box::new(MockToolInvoker::new("file_search".to_string())),
        );
        tool_registry.insert(
            "web_search".to_string(),
            Box::new(MockToolInvoker::new("web_search".to_string())),
        );
        tool_registry.insert(
            "shell_exec".to_string(),
            Box::new(MockToolInvoker::new("shell_exec".to_string())),
        );
        tool_registry.insert(
            "memory_search".to_string(),
            Box::new(MockToolInvoker::new("memory_search".to_string())),
        );
        tool_registry.insert(
            "memory_store".to_string(),
            Box::new(MockToolInvoker::new("memory_store".to_string())),
        );

        Self {
            agent_id: Uuid::new_v4(),
            active_executions: dashmap::DashMap::new(),
            resource_monitor: None,
            tool_registry,
            saga_manager: DefaultSagaManager::new(),
            active_sagas: dashmap::DashMap::new(),
            // Health monitoring fields
            last_heartbeat: Arc::new(RwLock::new(Some(Utc::now()))),
            error_count: Arc::new(AtomicU32::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Start automatic heartbeat loop for health monitoring
    /// This prevents timeout issues by sending heartbeat every 30 seconds
    pub fn start_heartbeat_loop(&self) {
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let agent_id = self.agent_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Update heartbeat timestamp
                {
                    let mut heartbeat = last_heartbeat.write().await;
                    *heartbeat = Some(Utc::now());
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    agent_type = "Executor",
                    "Heartbeat sent"
                );
            }
        });

        tracing::info!(
            agent_id = %self.agent_id,
            agent_type = "Executor",
            "Heartbeat loop started with 30s interval"
        );
    }

    /// Register a new tool invoker
    pub fn register_tool(&mut self, name: String, tool: Box<dyn ToolInvoker>) {
        tracing::info!("Registering tool: {}", name);
        self.tool_registry.insert(name, tool);
    }

    /// Get list of available tools
    pub fn get_available_tools(&self) -> Vec<String> {
        self.tool_registry.keys().cloned().collect()
    }

    /// Default resource limits
    fn default_resource_limits() -> ResourceLimits {
        ResourceLimits {
            max_cpu_time_ms: 300_000, // 5 minutes
            max_memory_mb: 1024,
            max_disk_usage_mb: 100,
            max_network_requests: 100,
            max_execution_time: std::time::Duration::from_secs(300),
        }
    }

    /// Create execution context for a plan
    fn create_execution_context(&self, plan: &ActionPlan) -> ExecutionContext {
        let mut step_states = HashMap::new();

        for step in &plan.steps {
            step_states.insert(
                step.id,
                StepState {
                    status: StepStatus::Pending,
                    result: None,
                    start_time: None,
                    end_time: None,
                    retry_count: 0,
                },
            );
        }

        ExecutionContext {
            plan_id: plan.id,
            current_step: 0,
            step_states,
            shared_data: HashMap::new(),
            resource_limits: Self::default_resource_limits(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    /// Execute tool invocation step
    async fn execute_tool_step(
        &self,
        tool_name: &str,
        arguments: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        if let Some(tool) = self.tool_registry.get(tool_name) {
            tracing::debug!("Executing tool: {}", tool_name);
            tool.invoke(arguments.clone()).await
        } else {
            anyhow::bail!("Tool '{}' not found in registry", tool_name)
        }
    }

    /// Execute memory operation step
    async fn execute_memory_operation(
        &self,
        operation_type: &MemoryOperationType,
        query: &str,
    ) -> Result<serde_json::Value> {
        // Simulate memory operation
        tracing::debug!("Executing memory operation: {:?}", operation_type);

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(serde_json::json!({
            "operation": format!("{:?}", operation_type),
            "query": query,
            "result": "success",
            "executed_at": chrono::Utc::now()
        }))
    }

    /// Execute user interaction step
    async fn execute_user_interaction(
        &self,
        interaction_type: &InteractionType,
        prompt: &str,
    ) -> Result<serde_json::Value> {
        // Simulate user interaction
        tracing::debug!("User interaction: {:?} - {}", interaction_type, prompt);

        // In a real implementation, this would prompt the user
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        Ok(serde_json::json!({
            "interaction_type": format!("{:?}", interaction_type),
            "prompt": prompt,
            "response": "ok", // Mock user response
            "executed_at": chrono::Utc::now()
        }))
    }

    /// Check if step dependencies are satisfied
    fn check_dependencies(&self, step: &ActionStep, context: &ExecutionContext) -> bool {
        for dep_id in &step.dependencies {
            if let Some(dep_state) = context.step_states.get(dep_id) {
                if dep_state.status != StepStatus::Completed {
                    return false;
                }
            } else {
                return false; // Dependency not found
            }
        }
        true
    }

    /// Apply retry logic for failed steps
    async fn should_retry(
        &self,
        step: &ActionStep,
        step_state: &StepState,
        error: &ExecutionError,
    ) -> bool {
        if step_state.retry_count >= step.retry_policy.max_retries {
            return false;
        }

        // Check if error is retryable
        if !error.retryable {
            return false;
        }

        // Check retry conditions
        match error.error_type {
            ExecutionErrorType::NetworkError => step
                .retry_policy
                .retry_conditions
                .iter()
                .any(|c| matches!(c, super::planner::RetryCondition::NetworkError)),
            ExecutionErrorType::ResourceExhausted => step
                .retry_policy
                .retry_conditions
                .iter()
                .any(|c| matches!(c, super::planner::RetryCondition::ResourceUnavailable)),
            _ => false,
        }
    }

    /// Calculate backoff delay for retries
    fn calculate_backoff_delay(&self, step: &ActionStep, retry_count: u32) -> std::time::Duration {
        match &step.retry_policy.backoff_strategy {
            super::planner::BackoffStrategy::Fixed(duration) => *duration,
            super::planner::BackoffStrategy::Exponential {
                initial,
                multiplier,
            } => {
                let delay_ms = initial.as_millis() as f64 * multiplier.powi(retry_count as i32);
                std::time::Duration::from_millis(delay_ms as u64)
            }
            super::planner::BackoffStrategy::Linear { initial, increment } => {
                *initial + (*increment * retry_count)
            }
        }
    }

    /// Execute rollback operation for a specific step
    async fn execute_rollback_operation(&self, step_id: Uuid) -> Result<()> {
        tracing::debug!("Executing rollback for step {}", step_id);

        // Simulate rollback operation
        // In a real implementation, this would:
        // - Undo file operations (delete created files, restore modified files)
        // - Reverse memory operations (remove stored data, restore previous state)
        // - Cancel network requests or API calls
        // - Restore system state to previous condition

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        tracing::debug!("Rollback completed for step {}", step_id);

        Ok(())
    }
}

#[async_trait]
impl ExecutorTrait for Executor {
    async fn execute_plan(&self, plan: &ActionPlan) -> Result<ExecutionResult> {
        tracing::info!("Starting execution of plan {}", plan.id);

        let start_time = std::time::Instant::now();
        let mut context = self.create_execution_context(plan);
        let mut step_results = Vec::new();
        let mut execution_error = None;

        // Store execution context
        self.active_executions.insert(plan.id, context.clone());

        let mut status = ExecutionStatus::Running;

        // Execute steps sequentially (for now - could be parallelized based on dependencies)
        for step in &plan.steps {
            // Check cancellation
            if context.cancellation_token.is_cancelled() {
                status = ExecutionStatus::Cancelled;
                break;
            }

            // Check dependencies
            if !self.check_dependencies(step, &context) {
                let error = ExecutionError {
                    error_type: ExecutionErrorType::DependencyFailed,
                    message: "Step dependencies not satisfied".to_string(),
                    step_id: Some(step.id),
                    retryable: false,
                    details: HashMap::new(),
                };

                step_results.push(StepResult {
                    step_id: step.id,
                    status: StepStatus::Failed,
                    output: None,
                    error: Some(error.message.clone()),
                    execution_time: std::time::Duration::from_millis(0),
                    retry_count: 0,
                    metadata: HashMap::new(),
                });

                execution_error = Some(error);
                status = ExecutionStatus::Failed;
                break;
            }

            // Execute step with retry logic
            match self.execute_step(step, &mut context).await {
                Ok(result) => {
                    step_results.push(result);
                }
                Err(e) => {
                    let error = ExecutionError {
                        error_type: ExecutionErrorType::SystemError,
                        message: e.to_string(),
                        step_id: Some(step.id),
                        retryable: false,
                        details: HashMap::new(),
                    };

                    step_results.push(StepResult {
                        step_id: step.id,
                        status: StepStatus::Failed,
                        output: None,
                        error: Some(error.message.clone()),
                        execution_time: std::time::Duration::from_millis(0),
                        retry_count: 0,
                        metadata: HashMap::new(),
                    });

                    execution_error = Some(error);
                    status = ExecutionStatus::Failed;
                    break;
                }
            }
        }

        if status == ExecutionStatus::Running {
            status = ExecutionStatus::Completed;
        }

        let execution_time = start_time.elapsed();

        // Remove from active executions
        self.active_executions.remove(&plan.id);

        let result = ExecutionResult {
            plan_id: plan.id,
            status,
            step_results,
            execution_time,
            resource_usage: ResourceUsage {
                cpu_time_ms: execution_time.as_millis() as u64,
                memory_peak_mb: 64, // Mock value
                disk_reads: 10,
                disk_writes: 5,
                network_requests: 0,
                tool_invocations: plan.steps.len() as u64,
            },
            metadata: HashMap::new(),
            error: execution_error,
        };

        tracing::info!(
            "Completed execution of plan {} with status: {:?}",
            plan.id,
            result.status
        );

        Ok(result)
    }

    async fn execute_step(
        &self,
        step: &ActionStep,
        context: &mut ExecutionContext,
    ) -> Result<StepResult> {
        let step_start = std::time::Instant::now();

        tracing::debug!("Executing step {}: {:?}", step.id, step.step_type);

        // Update step state to running
        if let Some(step_state) = context.step_states.get_mut(&step.id) {
            step_state.status = StepStatus::Running;
            step_state.start_time = Some(step_start);
        }

        let mut retry_count = 0;
        loop {
            let result = match &step.step_type {
                ActionStepType::ToolExecution {
                    tool_name,
                    arguments,
                } => self.execute_tool_step(tool_name, arguments).await,
                ActionStepType::MemoryOperation {
                    operation_type,
                    query,
                } => self.execute_memory_operation(operation_type, query).await,
                ActionStepType::UserInteraction {
                    interaction_type,
                    prompt,
                } => {
                    self.execute_user_interaction(interaction_type, prompt)
                        .await
                }
                ActionStepType::Wait { duration } => {
                    tokio::time::sleep(*duration).await;
                    Ok(serde_json::json!({
                        "waited": format!("{:?}", duration),
                        "executed_at": chrono::Utc::now()
                    }))
                }
                ActionStepType::Conditional {
                    condition,
                    then_steps,
                    else_steps: _,
                } => {
                    // Simplified conditional logic
                    tracing::debug!("Evaluating condition: {}", condition);

                    // For now, just execute then_steps (would need proper condition evaluation)
                    let mut conditional_results = Vec::new();
                    for sub_step in then_steps {
                        let sub_result = self.execute_step(sub_step, context).await?;
                        conditional_results.push(sub_result);
                    }

                    Ok(serde_json::json!({
                        "condition": condition,
                        "executed_branch": "then",
                        "results": conditional_results,
                        "executed_at": chrono::Utc::now()
                    }))
                }
                ActionStepType::Loop {
                    condition,
                    body_steps,
                    max_iterations,
                } => {
                    // Simplified loop logic
                    let mut loop_results = Vec::new();
                    let mut iteration = 0;

                    while iteration < *max_iterations {
                        tracing::debug!("Loop iteration {}/{}", iteration + 1, max_iterations);

                        for sub_step in body_steps {
                            let sub_result = self.execute_step(sub_step, context).await?;
                            loop_results.push(sub_result);
                        }

                        iteration += 1;

                        // Simple break condition (would need proper condition evaluation)
                        if condition == "break" {
                            break;
                        }
                    }

                    Ok(serde_json::json!({
                        "condition": condition,
                        "iterations": iteration,
                        "results": loop_results,
                        "executed_at": chrono::Utc::now()
                    }))
                }
            };

            let execution_time = step_start.elapsed();

            match result {
                Ok(output) => {
                    // Update step state to completed
                    if let Some(step_state) = context.step_states.get_mut(&step.id) {
                        step_state.status = StepStatus::Completed;
                        step_state.result = Some(output.clone());
                        step_state.end_time = Some(std::time::Instant::now());
                        step_state.retry_count = retry_count;
                    }

                    return Ok(StepResult {
                        step_id: step.id,
                        status: StepStatus::Completed,
                        output: Some(output),
                        error: None,
                        execution_time,
                        retry_count,
                        metadata: HashMap::new(),
                    });
                }
                Err(e) => {
                    let error = ExecutionError {
                        error_type: ExecutionErrorType::ToolExecutionFailed,
                        message: e.to_string(),
                        step_id: Some(step.id),
                        retryable: true,
                        details: HashMap::new(),
                    };

                    // Check if we should retry
                    if let Some(step_state) = context.step_states.get(&step.id) {
                        if self.should_retry(step, step_state, &error).await {
                            retry_count += 1;

                            // Update retry count
                            if let Some(step_state) = context.step_states.get_mut(&step.id) {
                                step_state.retry_count = retry_count;
                                step_state.status = StepStatus::Retrying;
                            }

                            let delay = self.calculate_backoff_delay(step, retry_count);
                            tracing::debug!("Retrying step {} after delay: {:?}", step.id, delay);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }

                    // No retry or max retries reached
                    if let Some(step_state) = context.step_states.get_mut(&step.id) {
                        step_state.status = StepStatus::Failed;
                        step_state.end_time = Some(std::time::Instant::now());
                        step_state.retry_count = retry_count;
                    }

                    return Ok(StepResult {
                        step_id: step.id,
                        status: StepStatus::Failed,
                        output: None,
                        error: Some(error.message),
                        execution_time,
                        retry_count,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }

    async fn cancel_execution(&self, plan_id: Uuid) -> Result<()> {
        if let Some(context) = self.active_executions.get_mut(&plan_id) {
            context.cancellation_token.cancel();
            tracing::info!("Cancelled execution of plan {}", plan_id);
            Ok(())
        } else {
            anyhow::bail!("Plan {} not found in active executions", plan_id)
        }
    }

    async fn pause_execution(&self, plan_id: Uuid) -> Result<()> {
        // Pausing is not implemented in this basic version
        tracing::warn!("Pause execution not implemented for plan {}", plan_id);
        Ok(())
    }

    async fn resume_execution(&self, plan_id: Uuid) -> Result<()> {
        // Resuming is not implemented in this basic version
        tracing::warn!("Resume execution not implemented for plan {}", plan_id);
        Ok(())
    }

    async fn get_execution_status(&self, plan_id: Uuid) -> Result<ExecutionStatus> {
        if self.active_executions.contains_key(&plan_id) {
            Ok(ExecutionStatus::Running)
        } else {
            Ok(ExecutionStatus::Completed) // Or could be Failed/Cancelled
        }
    }

    async fn rollback_execution(&self, plan_id: Uuid) -> Result<()> {
        tracing::info!("Starting rollback for plan {}", plan_id);

        // Try saga-based rollback first
        if let Some(saga) = self.active_sagas.get(&plan_id) {
            let mut saga = saga.clone();
            match self.saga_manager.compensate_saga(&mut saga).await {
                Ok(compensation_result) => {
                    self.active_sagas.insert(plan_id, saga);
                    match compensation_result.status {
                        SagaStatus::Compensated => {
                            tracing::info!("Saga-based rollback completed for plan {}", plan_id);

                            // Remove from active executions after successful saga rollback
                            self.active_executions.remove(&plan_id);
                            return Ok(());
                        }
                        _ => {
                            tracing::warn!("Saga compensation failed with status: {:?}, falling back to simple rollback", compensation_result.status);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Saga compensation failed: {}, falling back to simple rollback",
                        e
                    );
                }
            }
        }

        // Fallback to existing simple rollback if no saga or saga fails
        if let Some(context) = self.active_executions.get(&plan_id) {
            let mut rollback_operations = Vec::new();

            // Collect completed steps in reverse order for rollback
            for (step_id, step_state) in &context.step_states {
                if step_state.status == StepStatus::Completed {
                    rollback_operations.push(*step_id);
                }
            }

            rollback_operations.reverse(); // Reverse order for rollback

            for step_id in rollback_operations {
                match self.execute_rollback_operation(step_id).await {
                    Ok(_) => {
                        tracing::debug!("Simple rollback successful for step {}", step_id);
                    }
                    Err(e) => {
                        tracing::error!("Simple rollback failed for step {}: {}", step_id, e);
                        // Continue with other rollback operations
                    }
                }
            }

            // Remove from active executions after rollback
            self.active_executions.remove(&plan_id);

            tracing::info!("Simple rollback completed for plan {}", plan_id);
        } else {
            tracing::warn!("Plan {} not found for rollback", plan_id);
        }

        Ok(())
    }

    async fn execute_plan_with_saga(&self, plan: &ActionPlan) -> Result<ExecutionResult> {
        tracing::info!("Starting saga-enabled execution of plan {}", plan.id);

        // Create saga transaction for the plan
        let mut saga = self.saga_manager.create_saga(plan).await?;

        // Store saga
        self.active_sagas.insert(plan.id, saga.clone());

        // Execute the saga
        let saga_result = self.saga_manager.execute_saga(&mut saga).await?;

        // Handle saga execution result
        match saga_result.status {
            SagaStatus::Completed => {
                tracing::info!("Saga {} completed successfully", saga.id);

                // Update stored saga
                self.active_sagas.insert(plan.id, saga);

                // Return successful execution result
                Ok(ExecutionResult {
                    plan_id: plan.id,
                    status: ExecutionStatus::Completed,
                    step_results: saga_result.step_results,
                    execution_time: saga_result.execution_time,
                    resource_usage: ResourceUsage {
                        cpu_time_ms: saga_result.execution_time.as_millis() as u64,
                        memory_peak_mb: 64, // Mock value
                        disk_reads: 0,
                        disk_writes: 0,
                        network_requests: 0,
                        tool_invocations: saga_result.completed_steps as u64,
                    },
                    metadata: HashMap::new(),
                    error: None,
                })
            }
            SagaStatus::Compensating | SagaStatus::Failed => {
                tracing::warn!("Saga {} failed, starting compensation", saga.id);

                // Clone saga before compensation to avoid borrow issues
                let saga_id = saga.id;

                // Attempt compensation
                let compensation_result = self.saga_manager.compensate_saga(&mut saga).await?;

                // Update stored saga
                self.active_sagas.insert(plan.id, saga);

                let execution_status = match compensation_result.status {
                    SagaStatus::Compensated => {
                        tracing::info!("Saga {} successfully compensated", saga_id);
                        ExecutionStatus::Failed // Still failed, but compensated
                    }
                    _ => {
                        tracing::error!("Saga {} compensation failed", saga_id);
                        ExecutionStatus::Failed
                    }
                };

                Ok(ExecutionResult {
                    plan_id: plan.id,
                    status: execution_status,
                    step_results: saga_result.step_results,
                    execution_time: saga_result.execution_time
                        + compensation_result.compensation_time,
                    resource_usage: ResourceUsage {
                        cpu_time_ms: (saga_result.execution_time
                            + compensation_result.compensation_time)
                            .as_millis() as u64,
                        memory_peak_mb: 64,
                        disk_reads: 0,
                        disk_writes: 0,
                        network_requests: 0,
                        tool_invocations: saga_result.completed_steps as u64,
                    },
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("saga_id".to_string(), serde_json::json!(saga_id));
                        metadata.insert(
                            "compensation_attempted".to_string(),
                            serde_json::json!(true),
                        );
                        metadata.insert(
                            "compensation_successful".to_string(),
                            serde_json::json!(
                                compensation_result.status == SagaStatus::Compensated
                            ),
                        );
                        metadata.insert(
                            "compensated_steps".to_string(),
                            serde_json::json!(compensation_result.compensated_steps),
                        );
                        if !compensation_result.errors.is_empty() {
                            metadata.insert(
                                "compensation_errors".to_string(),
                                serde_json::json!(compensation_result.errors),
                            );
                        }
                        metadata
                    },
                    error: saga_result.error,
                })
            }
            _ => {
                // Handle other saga statuses
                let saga_id = saga.id;
                tracing::error!("Unexpected saga status: {:?}", saga_result.status);

                Ok(ExecutionResult {
                    plan_id: plan.id,
                    status: ExecutionStatus::Failed,
                    step_results: saga_result.step_results,
                    execution_time: saga_result.execution_time,
                    resource_usage: ResourceUsage::default(),
                    metadata: {
                        let mut metadata = HashMap::new();
                        metadata.insert("saga_id".to_string(), serde_json::json!(saga_id));
                        metadata.insert(
                            "saga_status".to_string(),
                            serde_json::json!(format!("{:?}", saga_result.status)),
                        );
                        metadata
                    },
                    error: saga_result.error,
                })
            }
        }
    }

    async fn get_saga_status(&self, plan_id: Uuid) -> Result<Option<SagaStatus>> {
        if let Some(saga) = self.active_sagas.get(&plan_id) {
            Ok(Some(saga.status.clone()))
        } else {
            Ok(None)
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::planner::{
        ActionPlan, ActionStep, BackoffStrategy, ResourceRequirements, RetryCondition, RetryPolicy,
        ValidationRule,
    };

    fn create_test_plan() -> ActionPlan {
        let step = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::ToolExecution {
                tool_name: "file_reader".to_string(),
                arguments: HashMap::new(),
            },
            parameters: HashMap::new(),
            dependencies: vec![],
            expected_duration: std::time::Duration::from_secs(1),
            retry_policy: RetryPolicy {
                max_retries: 2,
                backoff_strategy: BackoffStrategy::Fixed(std::time::Duration::from_millis(100)),
                retry_conditions: vec![RetryCondition::NetworkError],
            },
            validation_rules: vec![],
        };

        ActionPlan {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            steps: vec![step],
            estimated_duration: std::time::Duration::from_secs(1),
            resource_requirements: ResourceRequirements {
                cpu_cores: 1,
                memory_mb: 256,
                disk_space_mb: 10,
                network_required: false,
                tools_required: vec!["file_reader".to_string()],
                permissions_required: vec![],
            },
            dependencies: vec![],
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_execute_simple_plan() {
        let executor = Executor::new();
        let plan = create_test_plan();

        let result = executor
            .execute_plan(&plan)
            .await
            .expect("Executor operation should succeed");

        assert_eq!(result.plan_id, plan.id);
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert_eq!(result.step_results.len(), 1);
        assert_eq!(result.step_results[0].status, StepStatus::Completed);
    }

    #[tokio::test]
    async fn test_execute_step_with_tool() {
        let executor = Executor::new();
        let mut context = ExecutionContext {
            plan_id: Uuid::new_v4(),
            current_step: 0,
            step_states: HashMap::new(),
            shared_data: HashMap::new(),
            resource_limits: Executor::default_resource_limits(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };

        let step = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::ToolExecution {
                tool_name: "file_reader".to_string(),
                arguments: HashMap::new(),
            },
            parameters: HashMap::new(),
            dependencies: vec![],
            expected_duration: std::time::Duration::from_secs(1),
            retry_policy: RetryPolicy {
                max_retries: 0,
                backoff_strategy: BackoffStrategy::Fixed(std::time::Duration::from_millis(100)),
                retry_conditions: vec![],
            },
            validation_rules: vec![],
        };

        // Add step state
        context.step_states.insert(
            step.id,
            StepState {
                status: StepStatus::Pending,
                result: None,
                start_time: None,
                end_time: None,
                retry_count: 0,
            },
        );

        let result = executor
            .execute_step(&step, &mut context)
            .await
            .expect("Executor operation should succeed");

        assert_eq!(result.step_id, step.id);
        assert_eq!(result.status, StepStatus::Completed);
        assert!(result.output.is_some());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_cancel_execution() {
        let executor = Executor::new();
        let plan = create_test_plan();

        // Add a mock execution context
        let context = executor.create_execution_context(&plan);
        executor.active_executions.insert(plan.id, context);

        let result = executor.cancel_execution(plan.id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_execution_status() {
        let executor = Executor::new();
        let plan_id = Uuid::new_v4();

        // Test non-existent execution
        let status = executor
            .get_execution_status(plan_id)
            .await
            .expect("Executor operation should succeed");
        assert_eq!(status, ExecutionStatus::Completed);

        // Test active execution
        let context = ExecutionContext {
            plan_id,
            current_step: 0,
            step_states: HashMap::new(),
            shared_data: HashMap::new(),
            resource_limits: Executor::default_resource_limits(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };
        executor.active_executions.insert(plan_id, context);

        let status = executor
            .get_execution_status(plan_id)
            .await
            .expect("Executor operation should succeed");
        assert_eq!(status, ExecutionStatus::Running);
    }

    #[tokio::test]
    async fn test_rollback_execution() {
        let executor = Executor::new();
        let plan = create_test_plan();

        // Create execution context with completed step
        let mut context = executor.create_execution_context(&plan);
        let step_id = plan.steps[0].id;

        // Mark step as completed
        if let Some(step_state) = context.step_states.get_mut(&step_id) {
            step_state.status = StepStatus::Completed;
        }

        executor.active_executions.insert(plan.id, context);

        // Test rollback
        let result = executor.rollback_execution(plan.id).await;
        assert!(result.is_ok());

        // Verify execution was removed from active executions
        assert!(!executor.active_executions.contains_key(&plan.id));
    }

    #[tokio::test]
    async fn test_tool_registry_operations() {
        let mut executor = Executor::new();

        // Test initial tools
        let initial_tools = executor.get_available_tools();
        assert!(initial_tools.len() > 0);
        assert!(initial_tools.contains(&"file_reader".to_string()));

        // Register new tool
        let test_tool = Box::new(MockToolInvoker::new("test_tool".to_string()));
        executor.register_tool("test_tool".to_string(), test_tool);

        // Verify new tool is available
        let updated_tools = executor.get_available_tools();
        assert!(updated_tools.contains(&"test_tool".to_string()));
    }

    #[tokio::test]
    async fn test_execute_plan_with_saga() {
        let executor = Executor::new();
        let plan = create_test_plan();

        let result = executor
            .execute_plan_with_saga(&plan)
            .await
            .expect("Executor operation should succeed");

        assert_eq!(result.plan_id, plan.id);
        // Result could be Completed or Failed depending on saga simulation
        assert!(matches!(
            result.status,
            ExecutionStatus::Completed | ExecutionStatus::Failed
        ));

        // Check if saga was created and stored
        let saga_status = executor
            .get_saga_status(plan.id)
            .await
            .expect("Executor operation should succeed");
        assert!(saga_status.is_some());
    }

    #[tokio::test]
    async fn test_saga_status_tracking() {
        let executor = Executor::new();
        let plan = create_test_plan();

        // Initially no saga
        let initial_status = executor
            .get_saga_status(plan.id)
            .await
            .expect("Executor operation should succeed");
        assert!(initial_status.is_none());

        // Execute plan with saga
        let _result = executor
            .execute_plan_with_saga(&plan)
            .await
            .expect("Executor operation should succeed");

        // Should have saga status now
        let final_status = executor
            .get_saga_status(plan.id)
            .await
            .expect("Executor operation should succeed");
        assert!(final_status.is_some());

        let status = final_status.expect("Executor operation should succeed");
        assert!(matches!(
            status,
            SagaStatus::Completed | SagaStatus::Compensated | SagaStatus::Failed
        ));
    }

    #[tokio::test]
    async fn test_saga_compensation_metadata() {
        let executor = Executor::new();
        let plan = create_test_plan();

        let result = executor
            .execute_plan_with_saga(&plan)
            .await
            .expect("Executor operation should succeed");

        // Check if compensation metadata is present when saga fails
        if result.status == ExecutionStatus::Failed {
            assert!(result.metadata.contains_key("saga_id"));

            if result
                .metadata
                .get("compensation_attempted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                assert!(result.metadata.contains_key("compensation_successful"));
                assert!(result.metadata.contains_key("compensated_steps"));
            }
        }
    }

    #[tokio::test]
    async fn test_rollback_with_saga_integration() {
        let executor = Executor::new();
        let plan = create_test_plan();

        // Execute plan with saga first
        let _result = executor
            .execute_plan_with_saga(&plan)
            .await
            .expect("Executor operation should succeed");

        // Verify saga was created
        let saga_status = executor
            .get_saga_status(plan.id)
            .await
            .expect("Executor operation should succeed");
        assert!(saga_status.is_some());

        // Test rollback with saga integration
        let rollback_result = executor.rollback_execution(plan.id).await;
        assert!(rollback_result.is_ok());

        // Verify rollback completed (no active executions)
        assert!(!executor.active_executions.contains_key(&plan.id));

        tracing::info!("Rollback with saga integration test completed successfully");
    }

    #[tokio::test]
    async fn test_execute_plan_with_saga_error_handling() {
        let executor = Executor::new();
        let plan = create_test_plan();

        // Test that execute_plan_with_saga handles errors gracefully
        let result = executor.execute_plan_with_saga(&plan).await;
        assert!(
            result.is_ok(),
            "execute_plan_with_saga should handle errors gracefully"
        );

        let execution_result = result.expect("Executor operation should succeed");
        assert_eq!(execution_result.plan_id, plan.id);

        // Status should be either Completed or Failed (saga could succeed or fail)
        assert!(
            matches!(
                execution_result.status,
                ExecutionStatus::Completed | ExecutionStatus::Failed
            ),
            "Status should be Completed or Failed, got: {:?}",
            execution_result.status
        );

        // Verify saga tracking works
        let saga_status = executor
            .get_saga_status(plan.id)
            .await
            .expect("Executor operation should succeed");
        assert!(saga_status.is_some(), "Saga status should be tracked");

        // If saga failed, verify metadata
        if execution_result.status == ExecutionStatus::Failed {
            if execution_result
                .metadata
                .contains_key("compensation_attempted")
            {
                let compensation_attempted = execution_result
                    .metadata
                    .get("compensation_attempted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if compensation_attempted {
                    assert!(execution_result
                        .metadata
                        .contains_key("compensation_successful"));
                    assert!(execution_result.metadata.contains_key("compensated_steps"));
                }
            }
        }

        tracing::info!("execute_plan_with_saga error handling test completed successfully");
    }
}

/// HealthChecker implementation for Executor
#[async_trait]
impl HealthChecker for Executor {
    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    fn agent_name(&self) -> &str {
        "Executor"
    }

    fn agent_type(&self) -> &str {
        "Executor"
    }

    async fn check_health(&self) -> Result<HealthReport> {
        let last_heartbeat = *self.last_heartbeat.read().await;
        let error_count = self.error_count.load(Ordering::Relaxed);
        let uptime = self.start_time.elapsed().as_secs();

        // Agent-specific health checks
        let active_executions_count = self.active_executions.len();
        let active_sagas_count = self.active_sagas.len();
        let tool_registry_size = self.tool_registry.len();

        // Determine health status based on errors and resource usage
        let status = if error_count > 20 {
            HealthStatus::Unhealthy {
                reason: format!("High error count: {}", error_count),
            }
        } else if error_count > 10 || active_executions_count > 50 {
            HealthStatus::Degraded {
                reason: format!(
                    "Moderate error count: {} or high execution load: {}",
                    error_count, active_executions_count
                ),
            }
        } else {
            HealthStatus::Healthy
        };

        Ok(HealthReport {
            agent_id: self.agent_id,
            agent_name: "Executor".to_string(),
            agent_type: "Executor".to_string(),
            status,
            timestamp: Utc::now(),
            last_heartbeat,
            response_time_ms: Some(50),    // Tool execution can vary
            memory_usage_mb: Some(100),    // Estimated memory for execution context
            cpu_usage_percent: Some(15.0), // Higher CPU for tool execution
            active_tasks: active_executions_count as u32,
            error_count,
            restart_count: 0, // Track restarts in future implementation
            uptime_seconds: uptime,
            metadata: serde_json::json!({
                "active_executions": active_executions_count,
                "active_sagas": active_sagas_count,
                "available_tools": tool_registry_size,
                "resource_monitor_enabled": self.resource_monitor.is_some()
            }),
        })
    }

    async fn heartbeat(&self) -> Result<()> {
        let mut heartbeat = self.last_heartbeat.write().await;
        *heartbeat = Some(Utc::now());
        Ok(())
    }

    fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
        // Use try_read for synchronous access needed by the trait
        self.last_heartbeat.try_read().ok().and_then(|guard| *guard)
    }

    fn is_healthy(&self) -> bool {
        let error_count = self.error_count.load(Ordering::Relaxed);
        let active_executions = self.active_executions.len();
        error_count <= 20 && active_executions <= 50
    }

    async fn restart(&self) -> Result<()> {
        // Reset error count and update heartbeat
        self.error_count.store(0, Ordering::Relaxed);
        {
            let mut heartbeat = self.last_heartbeat.write().await;
            *heartbeat = Some(Utc::now());
        }

        // Cancel all active executions (graceful restart)
        let execution_ids: Vec<Uuid> = self
            .active_executions
            .iter()
            .map(|entry| *entry.key())
            .collect();
        for execution_id in execution_ids {
            if let Some((_, context)) = self.active_executions.remove(&execution_id) {
                context.cancellation_token.cancel();
            }
        }

        // Clear active sagas
        self.active_sagas.clear();

        Ok(())
    }
}

/// BaseActor implementation for Executor
#[async_trait::async_trait]
impl crate::actors::BaseActor for Executor {
    fn id(&self) -> crate::actors::ActorId {
        crate::actors::ActorId::new()
    }

    fn actor_type(&self) -> &'static str {
        "Executor"
    }

    async fn handle_message(
        &mut self,
        message: crate::actors::ActorMessage,
        _context: &crate::actors::ActorContext,
    ) -> Result<(), crate::actors::ActorError> {
        match message {
            crate::actors::ActorMessage::Agent(agent_msg) => match agent_msg {
                crate::actors::AgentMessage::ExecutePlan { plan: _, dry_run } => {
                    tracing::info!("Received plan execution request, dry_run: {}", dry_run);
                    // For BaseActor implementation, just acknowledge
                    Ok(())
                }
                _ => {
                    tracing::warn!("Unsupported agent message type for Executor");
                    Ok(())
                }
            },
            _ => {
                tracing::warn!("Unsupported message type for Executor");
                Ok(())
            }
        }
    }
}
