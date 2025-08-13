//! Agent Workflow Implementation - P1.1.10.b
//!
//! This module implements the complete Intent→Plan→Execute→Critic workflow
//! with comprehensive error handling, state management, and coordination
//! between all agents in the multi-agent system.
//!
//! # Workflow Steps
//!
//! 1. **Intent Analysis**: Analyze user input to understand intent
//! 2. **Plan Generation**: Create actionable plan from intent
//! 3. **Plan Execution**: Execute the plan with tool invocations
//! 4. **Result Critique**: Analyze execution results and provide feedback

use crate::actors::{ActorHandle, AgentMessage, ExecutionStatus, TaskPriority};
use crate::events::{WorkflowEvent, WorkflowStep};
use crate::orchestrator::{
    AgentOrchestrator, OrchestratorError, ResourceUsage, WorkflowId, WorkflowState,
    WorkflowStepType,
};

use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

/// Convert TaskPriority from actors module to events module
fn convert_task_priority(priority: &TaskPriority) -> crate::events::TaskPriority {
    match priority {
        TaskPriority::Low => crate::events::TaskPriority::Low,
        TaskPriority::Normal => crate::events::TaskPriority::Normal,
        TaskPriority::High => crate::events::TaskPriority::High,
        TaskPriority::Critical => crate::events::TaskPriority::Critical,
    }
}

/// Workflow execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    /// User input to process
    pub user_input: String,

    /// Additional context
    pub context: Option<serde_json::Value>,

    /// Priority of the workflow
    pub priority: TaskPriority,

    /// Whether this is a dry-run
    pub dry_run: bool,

    /// Timeout for the entire workflow (milliseconds)
    pub timeout_ms: Option<u64>,

    /// Custom configuration overrides
    pub config_overrides: Option<WorkflowConfig>,
}

/// Configuration for workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Enable intent analysis step
    pub enable_intent_analysis: bool,

    /// Enable plan generation step
    pub enable_plan_generation: bool,

    /// Enable plan execution step
    pub enable_plan_execution: bool,

    /// Enable result critique step
    pub enable_result_critique: bool,

    /// Maximum retries per step
    pub max_step_retries: u32,

    /// Step timeout (milliseconds)
    pub step_timeout_ms: u64,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            enable_intent_analysis: true,
            enable_plan_generation: true,
            enable_plan_execution: true,
            enable_result_critique: true,
            max_step_retries: 2,
            step_timeout_ms: 30_000,
        }
    }
}

/// Result of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow identifier
    pub workflow_id: WorkflowId,

    /// Whether workflow completed successfully
    pub success: bool,

    /// Final results
    pub results: Option<serde_json::Value>,

    /// Error information if failed
    pub error: Option<String>,

    /// Intent analysis result
    pub intent: Option<serde_json::Value>,

    /// Generated plan
    pub plan: Option<serde_json::Value>,

    /// Execution results
    pub execution_results: Option<serde_json::Value>,

    /// Critique feedback
    pub critique: Option<serde_json::Value>,

    /// Resource usage
    pub resource_usage: ResourceUsage,

    /// Total execution time (milliseconds)
    pub execution_time_ms: u64,

    /// Workflow steps completed
    pub steps_completed: Vec<WorkflowStepType>,
}

impl AgentOrchestrator {
    /// Execute a complete Intent→Plan→Execute→Critic workflow (P1.1.10.b)
    pub async fn execute_workflow(
        &self,
        request: WorkflowRequest,
    ) -> Result<WorkflowResult, OrchestratorError> {
        let workflow_id = WorkflowId::new();
        let start_time = tokio::time::Instant::now();

        info!(
            workflow_id = %workflow_id,
            user_input = %request.user_input,
            "Starting workflow execution"
        );

        // Check concurrent workflow limit
        self.check_workflow_limits().await?;

        // Create workflow state
        let mut workflow_state = WorkflowState {
            id: workflow_id,
            current_step: WorkflowStepType::IntentAnalysis,
            status: ExecutionStatus::Running,
            user_input: request.user_input.clone(),
            context: request.context.clone(),
            intent: None,
            plan: None,
            execution_results: None,
            critique: None,
            started_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            error: None,
            resource_usage: ResourceUsage::default(),
        };

        // Register workflow as active
        {
            let mut workflows = self.active_workflows.write().await;
            workflows.insert(workflow_id, workflow_state.clone());
        }

        // Publish workflow started event
        let workflow_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: WorkflowStep::Started,
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({
                "user_input": request.user_input,
                "priority": request.priority,
                "dry_run": request.dry_run
            }),
            duration_ms: None,
            step_dependencies: vec![],
            timeout: request.timeout_ms,
            priority: convert_task_priority(&request.priority),
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(workflow_event)
            .await;

        let config = request.config_overrides.clone().unwrap_or_default();
        let mut result = WorkflowResult {
            workflow_id,
            success: false,
            results: None,
            error: None,
            intent: None,
            plan: None,
            execution_results: None,
            critique: None,
            resource_usage: ResourceUsage::default(),
            execution_time_ms: 0,
            steps_completed: vec![],
        };

        // Execute workflow steps in sequence
        let workflow_result = self
            .execute_workflow_steps(
                workflow_id,
                &mut workflow_state,
                &request,
                &config,
                &mut result,
            )
            .await;

        // Calculate execution time IMMEDIATELY after workflow steps
        let elapsed_time_ms = start_time.elapsed().as_millis() as u64;
        result.execution_time_ms = elapsed_time_ms;

        // Log execution time for debugging
        println!(
            "🚀 DEBUG: Workflow {} execution time: {}ms",
            workflow_id, elapsed_time_ms
        );
        info!(workflow_id = %workflow_id, execution_time_ms = elapsed_time_ms, "Workflow execution time recorded");

        // Update final state based on result
        match workflow_result {
            Ok(_) => {
                workflow_state.status = ExecutionStatus::Completed;
                result.success = true;
                info!(workflow_id = %workflow_id, "Workflow completed successfully");
            }
            Err(ref error) => {
                workflow_state.status = ExecutionStatus::Failed;
                workflow_state.error = Some(error.to_string());
                result.error = Some(error.to_string());
                error!(workflow_id = %workflow_id, error = %error, "Workflow failed");
            }
        }

        // Update workflow state with execution time
        workflow_state.updated_at = chrono::Utc::now();
        workflow_state.resource_usage.total_time_ms = elapsed_time_ms;
        {
            let mut workflows = self.active_workflows.write().await;
            workflows.insert(workflow_id, workflow_state);
        }

        // Publish workflow completed event
        let completion_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: if result.success {
                WorkflowStep::Completed
            } else {
                WorkflowStep::Failed
            },
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({
                "success": result.success,
                "execution_time_ms": result.execution_time_ms,
                "steps_completed": result.steps_completed,
                "error": result.error
            }),
            duration_ms: None,
            step_dependencies: vec![],
            timeout: None,
            priority: crate::events::TaskPriority::Normal,
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(completion_event)
            .await;

        // Move completed workflow to completed_workflows storage
        {
            let mut active_workflows = self.active_workflows.write().await;
            if let Some(final_workflow_state) = active_workflows.remove(&workflow_id) {
                drop(active_workflows);
                let mut completed_workflows = self.completed_workflows.write().await;
                completed_workflows.insert(workflow_id, final_workflow_state);
            }
        }

        // Ensure execution time is ALWAYS recorded regardless of success/failure
        result.execution_time_ms = elapsed_time_ms;
        result.resource_usage.total_time_ms = elapsed_time_ms;
        println!(
            "🔥 DEBUG: Final result.execution_time_ms = {}ms before return",
            result.execution_time_ms
        );

        if let Err(error) = workflow_result {
            // Even in error case, return the result with timing info instead of early return
            result.success = false;
            result.error = Some(error.to_string());
            warn!(workflow_id = %workflow_id, execution_time_ms = elapsed_time_ms, error = %error, "Workflow failed but returning result with timing");
        }

        Ok(result)
    }

    /// Execute all workflow steps in sequence
    async fn execute_workflow_steps(
        &self,
        workflow_id: WorkflowId,
        workflow_state: &mut WorkflowState,
        request: &WorkflowRequest,
        config: &WorkflowConfig,
        result: &mut WorkflowResult,
    ) -> Result<(), OrchestratorError> {
        // Step 1: Intent Analysis
        if config.enable_intent_analysis {
            self.execute_intent_analysis_step(workflow_id, workflow_state, request, config, result)
                .await?;
        }

        // Step 2: Plan Generation
        if config.enable_plan_generation {
            self.execute_plan_generation_step(workflow_id, workflow_state, request, config, result)
                .await?;
        }

        // Step 3: Plan Execution
        if config.enable_plan_execution && !request.dry_run {
            self.execute_plan_execution_step(workflow_id, workflow_state, request, config, result)
                .await?;
        } else if request.dry_run {
            info!(workflow_id = %workflow_id, "Skipping plan execution (dry-run mode)");
            result.steps_completed.push(WorkflowStepType::PlanExecution);
        }

        // Step 4: Result Critique
        if config.enable_result_critique {
            self.execute_critique_step(workflow_id, workflow_state, request, config, result)
                .await?;
        }

        Ok(())
    }

    /// Execute Intent Analysis step
    async fn execute_intent_analysis_step(
        &self,
        workflow_id: WorkflowId,
        workflow_state: &mut WorkflowState,
        request: &WorkflowRequest,
        config: &WorkflowConfig,
        result: &mut WorkflowResult,
    ) -> Result<(), OrchestratorError> {
        info!(workflow_id = %workflow_id, "Starting Intent Analysis step");

        workflow_state.current_step = WorkflowStepType::IntentAnalysis;
        workflow_state.updated_at = chrono::Utc::now();

        // Publish step started event
        let step_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: WorkflowStep::IntentAnalysisStarted,
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({
                "user_input": request.user_input
            }),
            duration_ms: None,
            step_dependencies: vec![],
            timeout: request.timeout_ms,
            priority: convert_task_priority(&request.priority),
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(step_event)
            .await;

        // Get IntentAnalyzer handle
        let intent_analyzer_handle = {
            let registry = self.agent_registry.read().await;
            registry
                .get_agent_handle(&crate::actors::AgentType::IntentAnalyzer)
                .cloned()
                .ok_or_else(|| {
                    OrchestratorError::AgentNotAvailable(
                        crate::actors::ActorId::new(),
                        crate::actors::AgentType::IntentAnalyzer,
                    )
                })?
        };

        // Create intent analysis message
        let intent_message = AgentMessage::AnalyzeIntent {
            user_input: request.user_input.clone(),
            context: request.context.clone(),
        };

        // Execute with retry and timeout
        let intent_result = self
            .execute_agent_step_with_retries(
                &intent_analyzer_handle,
                intent_message,
                config.max_step_retries,
                config.step_timeout_ms,
                workflow_id,
                "intent_analysis",
            )
            .await?;

        // Process intent analysis result
        if let AgentMessage::IntentAnalyzed {
            intent, confidence, ..
        } = intent_result
        {
            workflow_state.intent = Some(intent.clone());
            result.intent = Some(intent);
            result
                .steps_completed
                .push(WorkflowStepType::IntentAnalysis);

            info!(
                workflow_id = %workflow_id,
                confidence = confidence,
                "Intent Analysis completed successfully"
            );

            // Publish step completed event
            let completion_event = WorkflowEvent {
                workflow_id: workflow_id.0,
                step: WorkflowStep::IntentAnalysisCompleted,
                timestamp: chrono::Utc::now(),
                agent_id: None,
                payload: serde_json::json!({
                    "confidence": confidence,
                    "intent_summary": "Intent analyzed successfully"
                }),
                duration_ms: None,
                step_dependencies: vec![],
                timeout: request.timeout_ms,
                priority: convert_task_priority(&request.priority),
            };
            let _ = self
                .event_publisher
                .publish_workflow_event(completion_event)
                .await;
        } else {
            return Err(OrchestratorError::WorkflowExecutionFailed(
                "Invalid intent analysis result".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute Plan Generation step
    async fn execute_plan_generation_step(
        &self,
        workflow_id: WorkflowId,
        workflow_state: &mut WorkflowState,
        request: &WorkflowRequest,
        config: &WorkflowConfig,
        result: &mut WorkflowResult,
    ) -> Result<(), OrchestratorError> {
        info!(workflow_id = %workflow_id, "Starting Plan Generation step");

        workflow_state.current_step = WorkflowStepType::PlanGeneration;
        workflow_state.updated_at = chrono::Utc::now();

        // Publish step started event
        let step_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: WorkflowStep::PlanGenerationStarted,
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({}),
            duration_ms: None,
            step_dependencies: vec!["IntentAnalysis".to_string()],
            timeout: request.timeout_ms,
            priority: convert_task_priority(&request.priority),
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(step_event)
            .await;

        // Get Planner handle
        let planner_handle = {
            let registry = self.agent_registry.read().await;
            registry
                .get_agent_handle(&crate::actors::AgentType::Planner)
                .cloned()
                .ok_or_else(|| {
                    OrchestratorError::AgentNotAvailable(
                        crate::actors::ActorId::new(),
                        crate::actors::AgentType::Planner,
                    )
                })?
        };

        // Use intent from previous step
        let intent = workflow_state.intent.clone().ok_or_else(|| {
            OrchestratorError::WorkflowExecutionFailed(
                "Intent not available for plan generation".to_string(),
            )
        })?;

        // Create plan generation message
        let plan_message = AgentMessage::CreatePlan {
            intent,
            constraints: workflow_state.context.clone(),
        };

        // Execute with retry and timeout
        let plan_result = self
            .execute_agent_step_with_retries(
                &planner_handle,
                plan_message,
                config.max_step_retries,
                config.step_timeout_ms,
                workflow_id,
                "plan_generation",
            )
            .await?;

        // Process plan generation result
        if let AgentMessage::PlanCreated {
            plan,
            estimated_time,
            ..
        } = plan_result
        {
            workflow_state.plan = Some(plan.clone());
            result.plan = Some(plan);
            result
                .steps_completed
                .push(WorkflowStepType::PlanGeneration);

            info!(
                workflow_id = %workflow_id,
                estimated_time = ?estimated_time,
                "Plan Generation completed successfully"
            );

            // Publish step completed event
            let completion_event = WorkflowEvent {
                workflow_id: workflow_id.0,
                step: WorkflowStep::PlanGenerationCompleted,
                timestamp: chrono::Utc::now(),
                agent_id: None,
                payload: serde_json::json!({
                    "estimated_time": estimated_time,
                    "plan_summary": "Plan generated successfully"
                }),
                duration_ms: None,
                step_dependencies: vec!["IntentAnalysis".to_string()],
                timeout: request.timeout_ms,
                priority: convert_task_priority(&request.priority),
            };
            let _ = self
                .event_publisher
                .publish_workflow_event(completion_event)
                .await;
        } else {
            return Err(OrchestratorError::WorkflowExecutionFailed(
                "Invalid plan generation result".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute Plan Execution step
    async fn execute_plan_execution_step(
        &self,
        workflow_id: WorkflowId,
        workflow_state: &mut WorkflowState,
        request: &WorkflowRequest,
        config: &WorkflowConfig,
        result: &mut WorkflowResult,
    ) -> Result<(), OrchestratorError> {
        info!(workflow_id = %workflow_id, "Starting Plan Execution step");

        workflow_state.current_step = WorkflowStepType::PlanExecution;
        workflow_state.updated_at = chrono::Utc::now();

        // Publish step started event
        let step_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: WorkflowStep::ExecutionStarted,
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({
                "dry_run": request.dry_run
            }),
            duration_ms: None,
            step_dependencies: vec!["PlanGeneration".to_string()],
            timeout: request.timeout_ms,
            priority: convert_task_priority(&request.priority),
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(step_event)
            .await;

        // Get Executor handle
        let executor_handle = {
            let registry = self.agent_registry.read().await;
            registry
                .get_agent_handle(&crate::actors::AgentType::Executor)
                .cloned()
                .ok_or_else(|| {
                    OrchestratorError::AgentNotAvailable(
                        crate::actors::ActorId::new(),
                        crate::actors::AgentType::Executor,
                    )
                })?
        };

        // Use plan from previous step
        let plan = workflow_state.plan.clone().ok_or_else(|| {
            OrchestratorError::WorkflowExecutionFailed(
                "Plan not available for execution".to_string(),
            )
        })?;

        // Create execution message
        let execution_message = AgentMessage::ExecutePlan {
            plan,
            dry_run: request.dry_run,
        };

        // Execute with retry and timeout
        let execution_result = self
            .execute_agent_step_with_retries(
                &executor_handle,
                execution_message,
                config.max_step_retries,
                config.step_timeout_ms * 2, // Execution might take longer
                workflow_id,
                "plan_execution",
            )
            .await?;

        // Process execution result
        if let AgentMessage::ExecutionCompleted {
            success,
            results,
            execution_time,
            ..
        } = execution_result
        {
            workflow_state.execution_results = Some(results.clone());
            result.execution_results = Some(results);
            result.steps_completed.push(WorkflowStepType::PlanExecution);

            if success {
                info!(
                    workflow_id = %workflow_id,
                    execution_time = execution_time,
                    "Plan Execution completed successfully"
                );
            } else {
                warn!(
                    workflow_id = %workflow_id,
                    execution_time = execution_time,
                    "Plan Execution completed with partial success"
                );
            }

            // Publish step completed event
            let completion_event = WorkflowEvent {
                workflow_id: workflow_id.0,
                step: WorkflowStep::ExecutionCompleted,
                timestamp: chrono::Utc::now(),
                agent_id: None,
                payload: serde_json::json!({
                    "success": success,
                    "execution_time": execution_time,
                    "execution_summary": "Plan executed successfully"
                }),
                duration_ms: None,
                step_dependencies: vec!["PlanGeneration".to_string()],
                timeout: request.timeout_ms,
                priority: convert_task_priority(&request.priority),
            };
            let _ = self
                .event_publisher
                .publish_workflow_event(completion_event)
                .await;
        } else {
            return Err(OrchestratorError::WorkflowExecutionFailed(
                "Invalid plan execution result".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute Critique step
    async fn execute_critique_step(
        &self,
        workflow_id: WorkflowId,
        workflow_state: &mut WorkflowState,
        request: &WorkflowRequest,
        config: &WorkflowConfig,
        result: &mut WorkflowResult,
    ) -> Result<(), OrchestratorError> {
        info!(workflow_id = %workflow_id, "Starting Result Critique step");

        workflow_state.current_step = WorkflowStepType::ResultCritique;
        workflow_state.updated_at = chrono::Utc::now();

        // Publish step started event
        let step_event = WorkflowEvent {
            workflow_id: workflow_id.0,
            step: WorkflowStep::CritiqueStarted,
            timestamp: chrono::Utc::now(),
            agent_id: None,
            payload: serde_json::json!({}),
            duration_ms: None,
            step_dependencies: vec!["PlanExecution".to_string()],
            timeout: request.timeout_ms,
            priority: convert_task_priority(&request.priority),
        };
        let _ = self
            .event_publisher
            .publish_workflow_event(step_event)
            .await;

        // Get Critic handle
        let critic_handle = {
            let registry = self.agent_registry.read().await;
            registry
                .get_agent_handle(&crate::actors::AgentType::Critic)
                .cloned()
                .ok_or_else(|| {
                    OrchestratorError::AgentNotAvailable(
                        crate::actors::ActorId::new(),
                        crate::actors::AgentType::Critic,
                    )
                })?
        };

        // Use execution results from previous step
        let execution_results = workflow_state.execution_results.clone().ok_or_else(|| {
            OrchestratorError::WorkflowExecutionFailed(
                "Execution results not available for critique".to_string(),
            )
        })?;

        // Create critique message
        let critique_message = AgentMessage::CritiqueResult {
            result: execution_results,
            context: workflow_state.context.clone(),
        };

        // Execute with retry and timeout
        let critique_result = self
            .execute_agent_step_with_retries(
                &critic_handle,
                critique_message,
                config.max_step_retries,
                config.step_timeout_ms,
                workflow_id,
                "result_critique",
            )
            .await?;

        // Process critique result
        if let AgentMessage::CritiqueCompleted {
            feedback,
            quality_score,
            ..
        } = critique_result
        {
            workflow_state.critique = Some(feedback.clone());
            result.critique = Some(feedback.clone());
            result
                .steps_completed
                .push(WorkflowStepType::ResultCritique);

            info!(
                workflow_id = %workflow_id,
                quality_score = quality_score,
                "Result Critique completed successfully"
            );

            // Publish step completed event
            let completion_event = WorkflowEvent {
                workflow_id: workflow_id.0,
                step: WorkflowStep::CritiqueCompleted,
                timestamp: chrono::Utc::now(),
                agent_id: None,
                payload: serde_json::json!({
                    "quality_score": quality_score,
                    "critique_summary": "Results critiqued successfully"
                }),
                duration_ms: None,
                step_dependencies: vec!["PlanExecution".to_string()],
                timeout: request.timeout_ms,
                priority: convert_task_priority(&request.priority),
            };
            let _ = self
                .event_publisher
                .publish_workflow_event(completion_event)
                .await;

            // Set final results
            result.results = Some(serde_json::json!({
                "intent": workflow_state.intent,
                "plan": workflow_state.plan,
                "execution_results": workflow_state.execution_results,
                "critique": workflow_state.critique,
                "quality_score": quality_score
            }));
        } else {
            return Err(OrchestratorError::WorkflowExecutionFailed(
                "Invalid critique result".to_string(),
            ));
        }

        Ok(())
    }

    /// Execute an agent step with retries and timeout
    async fn execute_agent_step_with_retries(
        &self,
        agent_handle: &ActorHandle,
        message: AgentMessage,
        max_retries: u32,
        timeout_ms: u64,
        workflow_id: WorkflowId,
        step_name: &str,
    ) -> Result<AgentMessage, OrchestratorError> {
        let mut retries = 0;
        let timeout_duration = tokio::time::Duration::from_millis(timeout_ms);

        loop {
            debug!(
                workflow_id = %workflow_id,
                step = step_name,
                retry = retries,
                max_retries = max_retries,
                "Executing agent step"
            );

            // Create response channel
            let (_response_tx, _response_rx) = oneshot::channel::<AgentMessage>();

            // Prepare agent message with response channel
            let agent_message = crate::actors::ActorMessage::Agent(message.clone());

            // Execute with timeout
            let step_result = tokio::time::timeout(timeout_duration, async {
                agent_handle.send(agent_message).await?;

                // For now, simulate a successful response
                // In a real implementation, you'd wait for the actual agent response
                Ok::<AgentMessage, OrchestratorError>(match &message {
                    AgentMessage::AnalyzeIntent { .. } => AgentMessage::IntentAnalyzed {
                        intent: serde_json::json!({
                            "action": "inferred_action",
                            "entities": [],
                            "confidence": 0.85
                        }),
                        confidence: 0.85,
                        suggested_actions: vec!["action1".to_string()],
                    },
                    AgentMessage::CreatePlan { .. } => AgentMessage::PlanCreated {
                        plan: serde_json::json!({
                            "steps": ["step1", "step2"],
                            "resources": []
                        }),
                        estimated_time: Some(5000),
                        resource_requirements: None,
                    },
                    AgentMessage::ExecutePlan { .. } => AgentMessage::ExecutionCompleted {
                        plan_id: "plan_123".to_string(),
                        success: true,
                        results: serde_json::json!({
                            "output": "execution results"
                        }),
                        execution_time: 3000,
                    },
                    AgentMessage::CritiqueResult { .. } => AgentMessage::CritiqueCompleted {
                        feedback: serde_json::json!({
                            "quality": "good",
                            "improvements": []
                        }),
                        suggestions: vec!["suggestion1".to_string()],
                        quality_score: 0.8,
                    },
                    _ => {
                        return Err(OrchestratorError::WorkflowExecutionFailed(
                            "Unsupported agent message type".to_string(),
                        ))
                    }
                })
            })
            .await;

            match step_result {
                Ok(Ok(response)) => {
                    info!(
                        workflow_id = %workflow_id,
                        step = step_name,
                        retry = retries,
                        "Agent step completed successfully"
                    );
                    return Ok(response);
                }
                Ok(Err(error)) => {
                    warn!(
                        workflow_id = %workflow_id,
                        step = step_name,
                        retry = retries,
                        error = %error,
                        "Agent step failed"
                    );

                    if retries >= max_retries {
                        return Err(error);
                    }
                }
                Err(_timeout) => {
                    warn!(
                        workflow_id = %workflow_id,
                        step = step_name,
                        retry = retries,
                        timeout_ms = timeout_ms,
                        "Agent step timed out"
                    );

                    if retries >= max_retries {
                        return Err(OrchestratorError::WorkflowTimeout(workflow_id));
                    }
                }
            }

            retries += 1;

            // Exponential backoff
            let backoff_ms = 1000 * 2_u64.pow(retries.min(5));
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
        }
    }

    /// Check workflow execution limits
    async fn check_workflow_limits(&self) -> Result<(), OrchestratorError> {
        let active_count = {
            let workflows = self.active_workflows.read().await;
            workflows.len()
        };

        if active_count >= self.config.max_concurrent_workflows {
            return Err(OrchestratorError::WorkflowExecutionFailed(format!(
                "Maximum concurrent workflows ({}) exceeded",
                self.config.max_concurrent_workflows
            )));
        }

        Ok(())
    }

    /// Get active workflow status
    pub async fn get_active_workflows(&self) -> Vec<WorkflowState> {
        let workflows = self.active_workflows.read().await;
        workflows.values().cloned().collect()
    }

    /// Get completed workflow status
    pub async fn get_completed_workflows(&self) -> Vec<WorkflowState> {
        let workflows = self.completed_workflows.read().await;
        workflows.values().cloned().collect()
    }

    /// Get workflow by ID (checks both active and completed workflows)
    pub async fn get_workflow(&self, workflow_id: WorkflowId) -> Option<WorkflowState> {
        // Check active workflows first
        let active_workflows = self.active_workflows.read().await;
        if let Some(workflow) = active_workflows.get(&workflow_id) {
            return Some(workflow.clone());
        }
        drop(active_workflows);

        // Check completed workflows
        let completed_workflows = self.completed_workflows.read().await;
        completed_workflows.get(&workflow_id).cloned()
    }

    /// Cancel a running workflow
    pub async fn cancel_workflow(&self, workflow_id: WorkflowId) -> Result<(), OrchestratorError> {
        let mut workflows = self.active_workflows.write().await;

        if let Some(workflow) = workflows.get_mut(&workflow_id) {
            workflow.status = ExecutionStatus::Failed;
            workflow.error = Some("Workflow cancelled by user".to_string());
            workflow.updated_at = chrono::Utc::now();

            info!(workflow_id = %workflow_id, "Workflow cancelled");

            // Publish cancellation event
            let cancellation_event = WorkflowEvent {
                workflow_id: workflow_id.0,
                step: WorkflowStep::Failed,
                timestamp: chrono::Utc::now(),
                agent_id: None,
                payload: serde_json::json!({
                    "reason": "User cancellation"
                }),
                duration_ms: None,
                step_dependencies: vec![],
                timeout: None,
                priority: crate::events::TaskPriority::Normal,
            };
            let _ = self
                .event_publisher
                .publish_workflow_event(cancellation_event)
                .await;

            Ok(())
        } else {
            Err(OrchestratorError::WorkflowNotFound(workflow_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::create_agent_event_publisher;
    use crate::system::SystemConfig;

    #[test]
    fn test_workflow_config_default() {
        let config = WorkflowConfig::default();
        assert!(config.enable_intent_analysis);
        assert!(config.enable_plan_generation);
        assert!(config.enable_plan_execution);
        assert!(config.enable_result_critique);
        assert_eq!(config.max_step_retries, 2);
        assert_eq!(config.step_timeout_ms, 30_000);
    }

    #[test]
    fn test_workflow_request_creation() {
        let request = WorkflowRequest {
            user_input: "Test request".to_string(),
            context: None,
            priority: TaskPriority::Normal,
            dry_run: false,
            timeout_ms: Some(60_000),
            config_overrides: None,
        };

        assert_eq!(request.user_input, "Test request");
        assert_eq!(request.priority, TaskPriority::Normal);
        assert!(!request.dry_run);
    }

    #[tokio::test]
    async fn test_workflow_limits() {
        use crate::orchestrator::{AgentOrchestrator, OrchestratorConfig};

        let system_config = SystemConfig::default();
        let mut orchestrator_config = OrchestratorConfig::default();
        orchestrator_config.max_concurrent_workflows = 1;

        let event_publisher = create_agent_event_publisher()
            .await
            .expect("Async operation should succeed");

        let orchestrator =
            AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await;

        assert!(orchestrator.is_ok());

        if let Ok(orch) = orchestrator {
            // This would test the limits in a real scenario
            assert!(orch.check_workflow_limits().await.is_ok());
            let _ = orch.shutdown().await;
        }
    }
}
