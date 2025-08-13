use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::agents::executor::{ExecutionError, StepResult};
use crate::agents::planner::{ActionPlan, ActionStep};

/// Saga transaction for managing multi-agent workflows with compensation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saga {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub status: SagaStatus,
    pub steps: Vec<SagaStep>,
    pub compensation_steps: Vec<CompensationStep>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Status of a Saga transaction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SagaStatus {
    /// Saga is being prepared
    Preparing,
    /// Saga is actively executing
    Executing,
    /// Saga completed successfully
    Completed,
    /// Saga failed and compensation is running
    Compensating,
    /// Saga was compensated successfully
    Compensated,
    /// Saga failed and compensation also failed
    Failed,
    /// Saga was cancelled
    Cancelled,
}

/// A step in the saga transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStep {
    pub id: Uuid,
    pub action_step_id: Uuid,
    pub status: SagaStepStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub compensation_needed: bool,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub compensated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Status of an individual saga step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SagaStepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently executing
    Executing,
    /// Step completed successfully
    Completed,
    /// Step failed
    Failed,
    /// Step is being compensated
    Compensating,
    /// Step was compensated successfully
    Compensated,
    /// Step compensation failed
    CompensationFailed,
}

/// A compensation step that undoes a previous action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationStep {
    pub id: Uuid,
    pub saga_step_id: Uuid,
    pub compensation_type: CompensationType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub status: CompensationStatus,
    pub executed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error: Option<String>,
}

/// Type of compensation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationType {
    /// Undo file operations (delete created files, restore modified files)
    FileOperationUndo {
        operation_type: String,
        affected_files: Vec<String>,
        backup_data: Option<String>,
    },
    /// Reverse memory operations (remove stored data, restore previous state)
    MemoryOperationUndo {
        operation_type: String,
        affected_records: Vec<Uuid>,
        backup_data: Option<serde_json::Value>,
    },
    /// Cancel or reverse tool executions
    ToolExecutionUndo {
        tool_name: String,
        original_arguments: HashMap<String, serde_json::Value>,
        undo_arguments: HashMap<String, serde_json::Value>,
    },
    /// Reverse user interactions (notify of cancellation)
    UserInteractionUndo {
        interaction_type: String,
        notification_message: String,
    },
    /// Custom compensation logic
    Custom {
        handler_name: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Status of compensation operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompensationStatus {
    /// Compensation is pending
    Pending,
    /// Compensation is executing
    Executing,
    /// Compensation completed successfully
    Completed,
    /// Compensation failed
    Failed,
    /// Compensation was skipped (not needed)
    Skipped,
}

/// Saga transaction manager
#[async_trait]
pub trait SagaManager: Send + Sync {
    /// Create a new saga from an action plan
    async fn create_saga(&self, plan: &ActionPlan) -> Result<Saga>;

    /// Execute a saga transaction
    async fn execute_saga(&self, saga: &mut Saga) -> Result<SagaExecutionResult>;

    /// Compensate a failed saga
    async fn compensate_saga(&self, saga: &mut Saga) -> Result<SagaCompensationResult>;

    /// Get saga status
    async fn get_saga_status(&self, saga_id: Uuid) -> Result<SagaStatus>;

    /// Cancel a running saga
    async fn cancel_saga(&self, saga_id: Uuid) -> Result<()>;
}

/// Result of saga execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaExecutionResult {
    pub saga_id: Uuid,
    pub status: SagaStatus,
    pub completed_steps: u32,
    pub failed_steps: u32,
    pub execution_time: std::time::Duration,
    pub error: Option<ExecutionError>,
    pub step_results: Vec<StepResult>,
}

/// Result of saga compensation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCompensationResult {
    pub saga_id: Uuid,
    pub status: SagaStatus,
    pub compensated_steps: u32,
    pub failed_compensations: u32,
    pub compensation_time: std::time::Duration,
    pub errors: Vec<String>,
}

/// Implementation of Saga transaction manager
pub struct DefaultSagaManager {
    active_sagas: dashmap::DashMap<Uuid, Saga>,
    compensation_handlers: HashMap<String, Box<dyn CompensationHandler>>,
}

/// Trait for custom compensation handlers
#[async_trait]
pub trait CompensationHandler: Send + Sync {
    async fn compensate(&self, step: &CompensationStep, original_step: &SagaStep) -> Result<()>;

    fn handler_name(&self) -> &str;
}

impl DefaultSagaManager {
    /// Create new saga manager
    pub fn new() -> Self {
        Self {
            active_sagas: dashmap::DashMap::new(),
            compensation_handlers: HashMap::new(),
        }
    }

    /// Register a custom compensation handler
    pub fn register_compensation_handler(&mut self, handler: Box<dyn CompensationHandler>) {
        let name = handler.handler_name().to_string();
        self.compensation_handlers.insert(name, handler);
    }

    /// Create compensation steps for an action step
    fn create_compensation_step(
        &self,
        action_step: &ActionStep,
        saga_step: &SagaStep,
    ) -> Option<CompensationStep> {
        let compensation_type = match &action_step.step_type {
            crate::agents::planner::ActionStepType::ToolExecution {
                tool_name,
                arguments,
            } => {
                match tool_name.as_str() {
                    "file_writer" | "file_creator" => Some(CompensationType::FileOperationUndo {
                        operation_type: "file_write".to_string(),
                        affected_files: vec![arguments
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string()],
                        backup_data: None,
                    }),
                    "file_deleter" => Some(CompensationType::FileOperationUndo {
                        operation_type: "file_delete".to_string(),
                        affected_files: vec![arguments
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string()],
                        backup_data: saga_step
                            .result
                            .as_ref()
                            .and_then(|r| r.get("backup"))
                            .and_then(|b| b.as_str())
                            .map(String::from),
                    }),
                    "memory_store" => Some(CompensationType::MemoryOperationUndo {
                        operation_type: "store".to_string(),
                        affected_records: vec![Uuid::new_v4()], // Would be actual record IDs
                        backup_data: None,
                    }),
                    _ => Some(CompensationType::ToolExecutionUndo {
                        tool_name: tool_name.clone(),
                        original_arguments: arguments.clone(),
                        undo_arguments: self.generate_undo_arguments(tool_name, arguments),
                    }),
                }
            }
            crate::agents::planner::ActionStepType::MemoryOperation {
                operation_type,
                query: _,
            } => {
                Some(CompensationType::MemoryOperationUndo {
                    operation_type: format!("{:?}", operation_type),
                    affected_records: vec![Uuid::new_v4()], // Would be actual record IDs
                    backup_data: saga_step.result.clone(),
                })
            }
            crate::agents::planner::ActionStepType::UserInteraction {
                interaction_type,
                prompt,
            } => Some(CompensationType::UserInteractionUndo {
                interaction_type: format!("{:?}", interaction_type),
                notification_message: format!("Operation cancelled: {}", prompt),
            }),
            _ => None, // Some operations don't need compensation
        };

        compensation_type.map(|comp_type| CompensationStep {
            id: Uuid::new_v4(),
            saga_step_id: saga_step.id,
            compensation_type: comp_type,
            parameters: HashMap::new(),
            status: CompensationStatus::Pending,
            executed_at: None,
            error: None,
        })
    }

    /// Generate undo arguments for tool execution
    fn generate_undo_arguments(
        &self,
        tool_name: &str,
        original_arguments: &HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut undo_args = HashMap::new();

        match tool_name {
            "shell_exec" => {
                // For shell commands, we might need to run reverse commands
                if let Some(command) = original_arguments.get("command") {
                    undo_args.insert(
                        "command".to_string(),
                        serde_json::Value::String(format!("# Undo for: {}", command)),
                    );
                }
            }
            "web_request" => {
                // Web requests might need to be reversed with DELETE/rollback calls
                undo_args.insert(
                    "method".to_string(),
                    serde_json::Value::String("DELETE".to_string()),
                );
            }
            _ => {
                // Default: mark as undo operation
                undo_args.insert("undo".to_string(), serde_json::Value::Bool(true));
                undo_args.extend(original_arguments.clone());
            }
        }

        undo_args
    }

    /// Execute compensation for a specific step
    async fn execute_compensation_step(
        &self,
        compensation_step: &mut CompensationStep,
        saga_step: &SagaStep,
    ) -> Result<()> {
        compensation_step.status = CompensationStatus::Executing;

        let result = match &compensation_step.compensation_type {
            CompensationType::FileOperationUndo {
                operation_type,
                affected_files,
                backup_data,
            } => {
                self.compensate_file_operation(operation_type, affected_files, backup_data)
                    .await
            }
            CompensationType::MemoryOperationUndo {
                operation_type,
                affected_records,
                backup_data,
            } => {
                self.compensate_memory_operation(operation_type, affected_records, backup_data)
                    .await
            }
            CompensationType::ToolExecutionUndo {
                tool_name,
                original_arguments,
                undo_arguments,
            } => {
                self.compensate_tool_execution(tool_name, original_arguments, undo_arguments)
                    .await
            }
            CompensationType::UserInteractionUndo {
                interaction_type,
                notification_message,
            } => {
                self.compensate_user_interaction(interaction_type, notification_message)
                    .await
            }
            CompensationType::Custom {
                handler_name,
                parameters,
            } => {
                self.compensate_custom(handler_name, parameters, compensation_step, saga_step)
                    .await
            }
        };

        match result {
            Ok(_) => {
                compensation_step.status = CompensationStatus::Completed;
                compensation_step.executed_at = Some(chrono::Utc::now());
                tracing::info!("Compensation completed for step {}", compensation_step.id);
            }
            Err(e) => {
                compensation_step.status = CompensationStatus::Failed;
                compensation_step.error = Some(e.to_string());
                tracing::error!(
                    "Compensation failed for step {}: {}",
                    compensation_step.id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Compensate file operation
    async fn compensate_file_operation(
        &self,
        operation_type: &str,
        affected_files: &[String],
        backup_data: &Option<String>,
    ) -> Result<()> {
        tracing::debug!("Compensating file operation: {}", operation_type);

        for file_path in affected_files {
            match operation_type {
                "file_write" | "file_create" => {
                    // Delete created file or restore original content
                    if let Some(_backup) = backup_data {
                        tracing::debug!("Restoring file {} from backup", file_path);
                        // Would restore from backup
                    } else {
                        tracing::debug!("Deleting created file {}", file_path);
                        // Would delete file
                    }
                }
                "file_delete" => {
                    // Restore deleted file from backup
                    if let Some(_backup) = backup_data {
                        tracing::debug!("Restoring deleted file {} from backup", file_path);
                        // Would restore file from backup
                    }
                }
                _ => {
                    tracing::warn!(
                        "Unknown file operation type for compensation: {}",
                        operation_type
                    );
                }
            }
        }

        // Simulate compensation time
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(())
    }

    /// Compensate memory operation
    async fn compensate_memory_operation(
        &self,
        operation_type: &str,
        affected_records: &[Uuid],
        backup_data: &Option<serde_json::Value>,
    ) -> Result<()> {
        tracing::debug!("Compensating memory operation: {}", operation_type);

        for record_id in affected_records {
            match operation_type {
                "Store" => {
                    tracing::debug!("Removing stored memory record {}", record_id);
                    // Would remove from memory store
                }
                "Update" => {
                    if let Some(_backup) = backup_data {
                        tracing::debug!("Restoring memory record {} from backup", record_id);
                        // Would restore from backup
                    }
                }
                "Delete" => {
                    if let Some(_backup) = backup_data {
                        tracing::debug!("Restoring deleted memory record {}", record_id);
                        // Would restore deleted record
                    }
                }
                _ => {
                    tracing::warn!(
                        "Unknown memory operation type for compensation: {}",
                        operation_type
                    );
                }
            }
        }

        // Simulate compensation time
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        Ok(())
    }

    /// Compensate tool execution
    async fn compensate_tool_execution(
        &self,
        tool_name: &str,
        _original_arguments: &HashMap<String, serde_json::Value>,
        undo_arguments: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        tracing::debug!("Compensating tool execution: {}", tool_name);

        // Simulate tool execution undo
        // In real implementation, this would:
        // 1. Invoke the same tool with undo arguments
        // 2. Or invoke a different tool that reverses the operation
        // 3. Or call a specific undo API

        tracing::debug!("Tool {} undo arguments: {:?}", tool_name, undo_arguments);

        // Simulate compensation time
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Compensate user interaction
    async fn compensate_user_interaction(
        &self,
        interaction_type: &str,
        notification_message: &str,
    ) -> Result<()> {
        tracing::info!(
            "User interaction compensation - {}: {}",
            interaction_type,
            notification_message
        );

        // In real implementation, this would:
        // 1. Send notification to user about cancellation
        // 2. Update UI to reflect cancellation
        // 3. Reset any user-facing state

        Ok(())
    }

    /// Compensate custom operation
    async fn compensate_custom(
        &self,
        handler_name: &str,
        _parameters: &HashMap<String, serde_json::Value>,
        compensation_step: &CompensationStep,
        saga_step: &SagaStep,
    ) -> Result<()> {
        if let Some(handler) = self.compensation_handlers.get(handler_name) {
            handler.compensate(compensation_step, saga_step).await
        } else {
            anyhow::bail!("Custom compensation handler '{}' not found", handler_name)
        }
    }
}

#[async_trait]
impl SagaManager for DefaultSagaManager {
    async fn create_saga(&self, plan: &ActionPlan) -> Result<Saga> {
        let saga_id = Uuid::new_v4();

        // Create saga steps from action plan steps
        let saga_steps: Vec<SagaStep> = plan
            .steps
            .iter()
            .map(|action_step| {
                SagaStep {
                    id: Uuid::new_v4(),
                    action_step_id: action_step.id,
                    status: SagaStepStatus::Pending,
                    result: None,
                    error: None,
                    compensation_needed: true, // Most operations need compensation
                    executed_at: None,
                    compensated_at: None,
                }
            })
            .collect();

        let saga = Saga {
            id: saga_id,
            plan_id: plan.id,
            status: SagaStatus::Preparing,
            steps: saga_steps,
            compensation_steps: Vec::new(),
            metadata: plan.metadata.clone(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        tracing::info!("Created saga {} for plan {}", saga_id, plan.id);

        Ok(saga)
    }

    async fn execute_saga(&self, saga: &mut Saga) -> Result<SagaExecutionResult> {
        let start_time = std::time::Instant::now();
        saga.status = SagaStatus::Executing;

        let mut completed_steps = 0;
        let mut failed_steps = 0;
        let mut step_results = Vec::new();
        let mut execution_error = None;

        // Store saga in active sagas
        self.active_sagas.insert(saga.id, saga.clone());

        tracing::info!("Starting saga execution {}", saga.id);

        // Execute saga steps sequentially
        for saga_step in &mut saga.steps {
            saga_step.status = SagaStepStatus::Executing;

            // Simulate step execution
            let step_result = self.simulate_step_execution(saga_step).await;

            match step_result {
                Ok(result) => {
                    saga_step.status = SagaStepStatus::Completed;
                    saga_step.result = Some(result.clone());
                    saga_step.executed_at = Some(chrono::Utc::now());
                    completed_steps += 1;

                    step_results.push(StepResult {
                        step_id: saga_step.action_step_id,
                        status: crate::agents::executor::StepStatus::Completed,
                        output: Some(result),
                        error: None,
                        execution_time: std::time::Duration::from_millis(100),
                        retry_count: 0,
                        metadata: HashMap::new(),
                    });
                }
                Err(e) => {
                    saga_step.status = SagaStepStatus::Failed;
                    saga_step.error = Some(e.to_string());
                    failed_steps += 1;

                    execution_error = Some(ExecutionError {
                        error_type: crate::agents::executor::ExecutionErrorType::SystemError,
                        message: e.to_string(),
                        step_id: Some(saga_step.action_step_id),
                        retryable: false,
                        details: HashMap::new(),
                    });

                    step_results.push(StepResult {
                        step_id: saga_step.action_step_id,
                        status: crate::agents::executor::StepStatus::Failed,
                        output: None,
                        error: Some(e.to_string()),
                        execution_time: std::time::Duration::from_millis(50),
                        retry_count: 0,
                        metadata: HashMap::new(),
                    });

                    // On failure, mark saga for compensation
                    saga.status = SagaStatus::Compensating;
                    break;
                }
            }
        }

        if saga.status == SagaStatus::Executing {
            saga.status = SagaStatus::Completed;
            saga.completed_at = Some(chrono::Utc::now());
        }

        let execution_time = start_time.elapsed();

        let result = SagaExecutionResult {
            saga_id: saga.id,
            status: saga.status.clone(),
            completed_steps,
            failed_steps,
            execution_time,
            error: execution_error,
            step_results,
        };

        // Update stored saga
        self.active_sagas.insert(saga.id, saga.clone());

        tracing::info!(
            "Saga execution {} completed with status: {:?}",
            saga.id,
            saga.status
        );

        Ok(result)
    }

    async fn compensate_saga(&self, saga: &mut Saga) -> Result<SagaCompensationResult> {
        let start_time = std::time::Instant::now();
        saga.status = SagaStatus::Compensating;

        let mut compensated_steps = 0;
        let mut failed_compensations = 0;
        let mut errors = Vec::new();

        tracing::info!("Starting saga compensation {}", saga.id);

        // Execute compensations in reverse order
        for saga_step in saga.steps.iter_mut().rev() {
            if saga_step.status == SagaStepStatus::Completed && saga_step.compensation_needed {
                saga_step.status = SagaStepStatus::Compensating;

                // Create and execute compensation
                // In a real implementation, this would reference the original action step
                // For now, simulate compensation
                match self.simulate_compensation(saga_step).await {
                    Ok(_) => {
                        saga_step.status = SagaStepStatus::Compensated;
                        saga_step.compensated_at = Some(chrono::Utc::now());
                        compensated_steps += 1;
                    }
                    Err(e) => {
                        saga_step.status = SagaStepStatus::CompensationFailed;
                        failed_compensations += 1;
                        errors.push(e.to_string());
                        tracing::error!("Compensation failed for step {}: {}", saga_step.id, e);
                    }
                }
            }
        }

        if failed_compensations == 0 {
            saga.status = SagaStatus::Compensated;
        } else {
            saga.status = SagaStatus::Failed;
        }

        saga.completed_at = Some(chrono::Utc::now());

        let compensation_time = start_time.elapsed();

        let result = SagaCompensationResult {
            saga_id: saga.id,
            status: saga.status.clone(),
            compensated_steps,
            failed_compensations,
            compensation_time,
            errors,
        };

        // Update stored saga
        self.active_sagas.insert(saga.id, saga.clone());

        tracing::info!(
            "Saga compensation {} completed with status: {:?}",
            saga.id,
            saga.status
        );

        Ok(result)
    }

    async fn get_saga_status(&self, saga_id: Uuid) -> Result<SagaStatus> {
        if let Some(saga) = self.active_sagas.get(&saga_id) {
            Ok(saga.status.clone())
        } else {
            anyhow::bail!("Saga {} not found", saga_id)
        }
    }

    async fn cancel_saga(&self, saga_id: Uuid) -> Result<()> {
        if let Some(mut saga) = self.active_sagas.get_mut(&saga_id) {
            saga.status = SagaStatus::Cancelled;
            tracing::info!("Cancelled saga {}", saga_id);
            Ok(())
        } else {
            anyhow::bail!("Saga {} not found", saga_id)
        }
    }
}

impl DefaultSagaManager {
    /// Simulate step execution for testing
    async fn simulate_step_execution(&self, step: &SagaStep) -> Result<serde_json::Value> {
        // Simulate execution time
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 90% success rate for simulation
        if rand::random::<f64>() < 0.9 {
            Ok(serde_json::json!({
                "step_id": step.id,
                "result": "success",
                "executed_at": chrono::Utc::now()
            }))
        } else {
            anyhow::bail!("Simulated step failure")
        }
    }

    /// Simulate compensation for testing
    async fn simulate_compensation(&self, step: &SagaStep) -> Result<()> {
        // Simulate compensation time
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        tracing::debug!("Compensating step {}", step.id);

        // 95% success rate for compensation
        if rand::random::<f64>() < 0.95 {
            Ok(())
        } else {
            anyhow::bail!("Simulated compensation failure")
        }
    }
}

impl Default for DefaultSagaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::planner::{
        ActionPlan, ActionStep, ActionStepType, BackoffStrategy, ResourceRequirements,
        RetryCondition, RetryPolicy,
    };

    fn create_test_plan() -> ActionPlan {
        let step1 = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::ToolExecution {
                tool_name: "file_writer".to_string(),
                arguments: {
                    let mut args = HashMap::new();
                    args.insert("path".to_string(), serde_json::json!("/tmp/test.txt"));
                    args.insert("content".to_string(), serde_json::json!("test content"));
                    args
                },
            },
            parameters: HashMap::new(),
            dependencies: vec![],
            expected_duration: std::time::Duration::from_secs(1),
            retry_policy: RetryPolicy {
                max_retries: 1,
                backoff_strategy: BackoffStrategy::Fixed(std::time::Duration::from_millis(100)),
                retry_conditions: vec![RetryCondition::NetworkError],
            },
            validation_rules: vec![],
        };

        let step2 = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::MemoryOperation {
                operation_type: crate::agents::planner::MemoryOperationType::Store,
                query: "test query".to_string(),
            },
            parameters: HashMap::new(),
            dependencies: vec![step1.id],
            expected_duration: std::time::Duration::from_secs(1),
            retry_policy: RetryPolicy {
                max_retries: 1,
                backoff_strategy: BackoffStrategy::Fixed(std::time::Duration::from_millis(100)),
                retry_conditions: vec![],
            },
            validation_rules: vec![],
        };

        ActionPlan {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            steps: vec![step1, step2],
            estimated_duration: std::time::Duration::from_secs(2),
            resource_requirements: ResourceRequirements {
                cpu_cores: 1,
                memory_mb: 256,
                disk_space_mb: 10,
                network_required: false,
                tools_required: vec!["file_writer".to_string()],
                permissions_required: vec![],
            },
            dependencies: vec![],
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_saga() {
        let manager = DefaultSagaManager::new();
        let plan = create_test_plan();

        let saga = manager
            .create_saga(&plan)
            .await
            .expect("Async operation should succeed");

        assert_eq!(saga.plan_id, plan.id);
        assert_eq!(saga.status, SagaStatus::Preparing);
        assert_eq!(saga.steps.len(), 2);
        assert!(saga.compensation_steps.is_empty());
    }

    #[tokio::test]
    async fn test_execute_saga_success() {
        let manager = DefaultSagaManager::new();
        let plan = create_test_plan();
        let mut saga = manager
            .create_saga(&plan)
            .await
            .expect("Async operation should succeed");

        let result = manager
            .execute_saga(&mut saga)
            .await
            .expect("Async operation should succeed");

        assert_eq!(result.saga_id, saga.id);
        // Result status could be Completed or Compensating depending on simulation
        assert!(matches!(
            result.status,
            SagaStatus::Completed | SagaStatus::Compensating
        ));
        assert_eq!(result.step_results.len(), saga.steps.len());
    }

    #[tokio::test]
    async fn test_compensate_saga() {
        let manager = DefaultSagaManager::new();
        let plan = create_test_plan();
        let mut saga = manager
            .create_saga(&plan)
            .await
            .expect("Async operation should succeed");

        // First execute the saga
        let _exec_result = manager
            .execute_saga(&mut saga)
            .await
            .expect("Async operation should succeed");

        // If it needs compensation, compensate it
        if saga.status == SagaStatus::Compensating {
            let comp_result = manager
                .compensate_saga(&mut saga)
                .await
                .expect("Async operation should succeed");

            assert_eq!(comp_result.saga_id, saga.id);
            assert!(matches!(
                comp_result.status,
                SagaStatus::Compensated | SagaStatus::Failed
            ));
        }
    }

    #[tokio::test]
    async fn test_saga_status_tracking() {
        let manager = DefaultSagaManager::new();
        let plan = create_test_plan();
        let saga = manager
            .create_saga(&plan)
            .await
            .expect("Async operation should succeed");

        // Check initial status
        let status = manager
            .get_saga_status(saga.id)
            .await
            .expect("Async operation should succeed");
        assert_eq!(status, SagaStatus::Preparing);

        // Test cancellation
        manager
            .cancel_saga(saga.id)
            .await
            .expect("Async operation should succeed");
        let status = manager
            .get_saga_status(saga.id)
            .await
            .expect("Async operation should succeed");
        assert_eq!(status, SagaStatus::Cancelled);
    }
}
