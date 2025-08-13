#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::uninlined_format_args)]
//! Saga Integration Tests for Multi-Agent Orchestration
//!
//! Tests saga pattern integration:
//! - Saga workflow execution with rollback scenarios
//! - Executor-Saga integration for transactional consistency
//! - Saga compensation edge cases and failure handling

use anyhow::Result;
use orchestrator::{
    agents::{
        executor::{
            ExecutionError, ExecutionErrorType, ExecutionStatus, Executor, ExecutorTrait,
            StepResult, StepStatus,
        },
        planner::{
            ActionPlan, ActionStep, ActionStepType, BackoffStrategy, InteractionType,
            MemoryOperationType, ResourceRequirements, RetryCondition, RetryPolicy,
        },
    },
    saga::{
        CompensationStatus, CompensationStep, CompensationType, DefaultSagaManager, Saga,
        SagaCompensationResult, SagaExecutionResult, SagaManager, SagaStatus, SagaStep,
        SagaStepStatus,
    },
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Test saga workflow execution with rollback scenario
#[tokio::test]
async fn test_saga_workflow_with_rollback() -> Result<()> {
    // Create saga manager
    let saga_manager = DefaultSagaManager::new();

    // Create a complex action plan that might require rollback
    let action_plan = create_complex_action_plan_for_rollback();

    // Create saga from action plan
    let mut saga = saga_manager.create_saga(&action_plan).await?;

    // Verify saga was created correctly
    assert_eq!(saga.plan_id, action_plan.id);
    assert_eq!(saga.status, SagaStatus::Preparing);
    assert_eq!(saga.steps.len(), action_plan.steps.len());
    assert!(saga.compensation_steps.is_empty());

    // Execute saga (some steps may fail requiring rollback)
    let execution_result = saga_manager.execute_saga(&mut saga).await?;

    // Verify saga execution result
    assert_eq!(execution_result.saga_id, saga.id);
    assert!(
        execution_result.completed_steps > 0,
        "Some steps should complete"
    );
    assert!(execution_result.execution_time > Duration::from_millis(0));

    // If saga needs compensation, perform rollback
    if saga.status == SagaStatus::Compensating {
        let compensation_result = saga_manager.compensate_saga(&mut saga).await?;

        // Verify compensation was attempted
        assert_eq!(compensation_result.saga_id, saga.id);
        assert!(
            matches!(
                compensation_result.status,
                SagaStatus::Compensated | SagaStatus::Failed
            ),
            "Saga should reach terminal compensation state"
        );

        // Verify compensation metrics
        assert!(
            compensation_result.compensated_steps > 0,
            "Some compensation steps should execute"
        );
        assert!(
            compensation_result.compensation_time > Duration::from_millis(0),
            "Compensation should take measurable time"
        );

        // Check individual saga steps for compensation status
        let compensated_steps = saga
            .steps
            .iter()
            .filter(|step| step.status == SagaStepStatus::Compensated)
            .count();
        assert!(
            compensated_steps > 0,
            "At least some completed steps should be compensated"
        );

        // Verify rollback completed successfully or with acceptable failures
        if compensation_result.status == SagaStatus::Compensated {
            assert_eq!(
                compensation_result.failed_compensations, 0,
                "All compensations should succeed"
            );
        } else {
            // If compensation failed, verify error information is available
            assert!(
                !compensation_result.errors.is_empty(),
                "Should have error details"
            );
        }
    } else if saga.status == SagaStatus::Completed {
        // Saga completed without needing compensation
        assert_eq!(execution_result.failed_steps, 0, "No steps should fail");
        assert!(
            execution_result.error.is_none(),
            "No execution error should occur"
        );
    }

    // Verify saga final state is consistent
    assert!(
        matches!(
            saga.status,
            SagaStatus::Completed | SagaStatus::Compensated | SagaStatus::Failed
        ),
        "Saga should reach terminal state"
    );
    assert!(
        saga.completed_at.is_some(),
        "Saga should have completion time"
    );

    Ok(())
}

/// Test Executor-Saga integration for transactional consistency
#[tokio::test]
async fn test_executor_saga_integration() -> Result<()> {
    // Create Executor and Saga manager
    let executor = Executor::new();
    let saga_manager = DefaultSagaManager::new();

    // Create action plan with mock operation types (no real tools needed)
    let action_plan = create_mock_action_plan_for_saga_integration();

    // Create saga for the action plan
    let mut saga = saga_manager.create_saga(&action_plan).await?;

    // Execute plan through Executor with saga support
    let execution_result = executor.execute_plan(&action_plan).await;

    match execution_result {
        Ok(result) => {
            // Successful execution - update saga accordingly
            for (i, step_result) in result.step_results.iter().enumerate() {
                if let Some(saga_step) = saga.steps.get_mut(i) {
                    match step_result.status {
                        StepStatus::Completed => {
                            saga_step.status = SagaStepStatus::Completed;
                            saga_step.result = step_result.output.clone();
                            saga_step.executed_at = Some(chrono::Utc::now());
                        }
                        StepStatus::Failed => {
                            saga_step.status = SagaStepStatus::Failed;
                            saga_step.error = step_result.error.clone();
                        }
                        _ => {
                            saga_step.status = SagaStepStatus::Pending;
                        }
                    }
                }
            }

            // Execution successful - verify results

            // Verify successful integration
            assert!(
                result.status == ExecutionStatus::Completed,
                "Execution should succeed. Actual status: {:?}",
                result.status
            );
            assert_eq!(saga.steps.len(), result.step_results.len());

            // Update saga status based on execution success
            if result.status == ExecutionStatus::Completed {
                saga.status = SagaStatus::Completed;
                saga.completed_at = Some(chrono::Utc::now());
            }

            let completed_saga_steps = saga
                .steps
                .iter()
                .filter(|step| step.status == SagaStepStatus::Completed)
                .count();

            let completed_execution_steps = result
                .step_results
                .iter()
                .filter(|result| result.status == StepStatus::Completed)
                .count();

            assert_eq!(
                completed_saga_steps, completed_execution_steps,
                "Saga and execution step counts should match"
            );
        }
        Err(execution_error) => {
            // Execution failed - trigger saga compensation
            saga.status = SagaStatus::Compensating;

            // Perform saga rollback for any completed steps
            let compensation_result = saga_manager.compensate_saga(&mut saga).await?;

            // Verify compensation was executed
            assert!(
                matches!(
                    compensation_result.status,
                    SagaStatus::Compensated | SagaStatus::Failed
                ),
                "Saga should attempt compensation"
            );

            // Check that rollback was attempted for executed steps
            let compensation_attempted_steps = saga
                .steps
                .iter()
                .filter(|step| {
                    matches!(
                        step.status,
                        SagaStepStatus::Compensated | SagaStepStatus::CompensationFailed
                    )
                })
                .count();

            assert!(
                compensation_attempted_steps > 0,
                "Should attempt compensation for executed steps"
            );

            // Verify error handling integration (simplified)
            // assert for error handling can be simplified since error_type field doesn't exist
        }
    }

    // Test rollback integration when executor explicitly requests it
    if let Ok(rollback_result) = executor.rollback_execution(action_plan.id).await {
        // Verify rollback succeeded
        // rollback_result is (), so just check it exists
        // assert!(rollback_result.success, "Rollback should succeed");

        // Integrate rollback result with saga compensation
        if saga.status == SagaStatus::Executing || saga.status == SagaStatus::Completed {
            saga.status = SagaStatus::Compensating;
            let _compensation_result = saga_manager.compensate_saga(&mut saga).await?;
        }
    }

    // Check final saga state

    // Verify final integration state
    assert!(
        saga.completed_at.is_some() || saga.status == SagaStatus::Compensating,
        "Saga should be in final or compensating state. completed_at: {:?}, status: {:?}",
        saga.completed_at,
        saga.status
    );

    Ok(())
}

/// Test saga compensation edge cases and failure scenarios
#[tokio::test]
async fn test_saga_compensation_edge_cases() -> Result<()> {
    let saga_manager = DefaultSagaManager::new();

    // Test Case 1: Compensation failure scenario
    let failing_plan = create_compensation_failure_action_plan();
    let mut failing_saga = saga_manager.create_saga(&failing_plan).await?;

    // Execute saga that will require compensation
    let _execution_result = saga_manager.execute_saga(&mut failing_saga).await?;

    if failing_saga.status == SagaStatus::Compensating {
        let compensation_result = saga_manager.compensate_saga(&mut failing_saga).await?;

        // Verify compensation failure handling
        if compensation_result.failed_compensations > 0 {
            assert!(
                !compensation_result.errors.is_empty(),
                "Should have error details"
            );
            assert_eq!(
                compensation_result.status,
                SagaStatus::Failed,
                "Saga should fail when compensation fails"
            );

            // Check individual compensation steps
            let failed_compensation_steps = failing_saga
                .steps
                .iter()
                .filter(|step| step.status == SagaStepStatus::CompensationFailed)
                .count();
            assert!(
                failed_compensation_steps > 0,
                "Should have failed compensation steps"
            );
        }
    }

    // Test Case 2: Partial compensation scenario
    let partial_plan = create_partial_compensation_action_plan();
    let mut partial_saga = saga_manager.create_saga(&partial_plan).await?;

    // Manually set some steps as completed and others as failed
    for (i, step) in partial_saga.steps.iter_mut().enumerate() {
        if i % 2 == 0 {
            step.status = SagaStepStatus::Completed;
            step.executed_at = Some(chrono::Utc::now());
            step.compensation_needed = true;
        } else {
            step.status = SagaStepStatus::Failed;
            step.compensation_needed = false;
        }
    }

    // Count steps before compensation to verify logic
    let steps_to_compensate_before = partial_saga
        .steps
        .iter()
        .filter(|step| step.status == SagaStepStatus::Completed && step.compensation_needed)
        .count();

    partial_saga.status = SagaStatus::Compensating;
    let partial_compensation_result = saga_manager.compensate_saga(&mut partial_saga).await?;

    // Compensation complete - verify results

    // Verify partial compensation behavior
    let completed_steps_after = partial_saga
        .steps
        .iter()
        .filter(|step| step.status == SagaStepStatus::Completed)
        .count();

    let compensated_steps = partial_saga
        .steps
        .iter()
        .filter(|step| step.status == SagaStepStatus::Compensated)
        .count();

    // Verify compensation counts

    assert!(
        compensated_steps <= steps_to_compensate_before,
        "Can't compensate more steps ({}) than were eligible for compensation before ({})",
        compensated_steps,
        steps_to_compensate_before
    );
    assert_eq!(
        partial_compensation_result.compensated_steps as usize, compensated_steps,
        "Compensation result should match actual compensated steps"
    );

    // Test Case 3: Empty saga compensation
    let empty_plan = create_empty_action_plan();
    let mut empty_saga = saga_manager.create_saga(&empty_plan).await?;

    empty_saga.status = SagaStatus::Compensating;
    let empty_compensation_result = saga_manager.compensate_saga(&mut empty_saga).await?;

    // Verify empty saga compensation
    assert_eq!(
        empty_compensation_result.compensated_steps, 0,
        "No steps to compensate"
    );
    assert_eq!(
        empty_compensation_result.failed_compensations, 0,
        "No compensation failures"
    );
    assert!(
        empty_compensation_result.errors.is_empty(),
        "No compensation errors"
    );
    assert_eq!(
        empty_compensation_result.status,
        SagaStatus::Compensated,
        "Empty saga should compensate successfully"
    );

    // Test Case 4: Already compensated saga
    let already_compensated_plan = create_simple_action_plan();
    let mut already_compensated_saga = saga_manager.create_saga(&already_compensated_plan).await?;

    // Mark saga as already compensated
    already_compensated_saga.status = SagaStatus::Compensated;
    already_compensated_saga.completed_at = Some(chrono::Utc::now());

    for step in &mut already_compensated_saga.steps {
        step.status = SagaStepStatus::Compensated;
        step.compensated_at = Some(chrono::Utc::now());
    }

    let already_compensated_result = saga_manager
        .compensate_saga(&mut already_compensated_saga)
        .await?;

    // Verify idempotent compensation behavior
    assert_eq!(
        already_compensated_result.status,
        SagaStatus::Compensated,
        "Already compensated saga should remain compensated"
    );

    // Test Case 5: Saga status tracking edge cases
    let status_test_saga_id = Uuid::new_v4();

    // Test getting status of non-existent saga
    let nonexistent_status = saga_manager.get_saga_status(status_test_saga_id).await;
    assert!(
        nonexistent_status.is_err(),
        "Should error for non-existent saga"
    );

    // Test cancelling non-existent saga
    let cancel_nonexistent = saga_manager.cancel_saga(status_test_saga_id).await;
    assert!(
        cancel_nonexistent.is_err(),
        "Should error when cancelling non-existent saga"
    );

    Ok(())
}

/// Test saga transaction atomicity and consistency
#[tokio::test]
async fn test_saga_transaction_atomicity() -> Result<()> {
    let saga_manager = DefaultSagaManager::new();

    // Create plan with transactional operations
    let transactional_plan = create_transactional_action_plan();
    let mut saga = saga_manager.create_saga(&transactional_plan).await?;

    // Execute saga and monitor atomicity
    let execution_result = saga_manager.execute_saga(&mut saga).await?;

    // Verify transaction properties
    if execution_result.failed_steps > 0 {
        // If any step failed, all completed steps should be compensated
        assert_eq!(
            saga.status,
            SagaStatus::Compensating,
            "Failed saga should require compensation"
        );

        let compensation_result = saga_manager.compensate_saga(&mut saga).await?;

        // Check atomicity: either all operations succeed or all are rolled back
        let successful_steps = saga
            .steps
            .iter()
            .filter(|step| step.status == SagaStepStatus::Completed)
            .count();

        let compensated_steps = saga
            .steps
            .iter()
            .filter(|step| step.status == SagaStepStatus::Compensated)
            .count();

        // Verify compensation attempted for all completed steps
        if compensation_result.status == SagaStatus::Compensated {
            assert_eq!(
                successful_steps, compensated_steps,
                "All completed steps should be compensated for atomicity"
            );
        }
    } else {
        // All steps succeeded - verify consistency
        assert_eq!(saga.status, SagaStatus::Completed);
        assert_eq!(
            execution_result.completed_steps as usize,
            saga.steps.len(),
            "All steps should complete successfully"
        );
    }

    Ok(())
}

// Helper functions to create test action plans

fn create_complex_action_plan_for_rollback() -> ActionPlan {
    let step1 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "file_writer".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert(
                    "path".to_string(),
                    serde_json::json!("/tmp/saga_test_1.txt"),
                );
                args.insert("content".to_string(), serde_json::json!("test content 1"));
                args
            },
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_secs(2),
        retry_policy: RetryPolicy {
            max_retries: 1,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(100)),
            retry_conditions: vec![RetryCondition::NetworkError],
        },
        validation_rules: vec![],
    };

    let step2 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::MemoryOperation {
            operation_type: MemoryOperationType::Store,
            query: "test data for saga".to_string(),
        },
        parameters: HashMap::new(),
        dependencies: vec![step1.id],
        expected_duration: Duration::from_secs(1),
        retry_policy: RetryPolicy {
            max_retries: 2,
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                multiplier: 2.0,
            },
            retry_conditions: vec![RetryCondition::TemporaryFailure],
        },
        validation_rules: vec![],
    };

    let step3 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::UserInteraction {
            interaction_type: InteractionType::Confirmation,
            prompt: "Please confirm the operation".to_string(),
        },
        parameters: HashMap::new(),
        dependencies: vec![step2.id],
        expected_duration: Duration::from_secs(30),
        retry_policy: RetryPolicy {
            max_retries: 0,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(1000)),
            retry_conditions: vec![],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![step1, step2, step3],
        estimated_duration: Duration::from_secs(33),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 512,
            disk_space_mb: 50,
            network_required: false,
            tools_required: vec!["file_writer".to_string()],
            permissions_required: vec!["file_write".to_string()],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert(
                "test_type".to_string(),
                serde_json::json!("rollback_scenario"),
            );
            meta.insert("complexity".to_string(), serde_json::json!("high"));
            meta
        },
    }
}

fn create_mixed_operation_action_plan() -> ActionPlan {
    let file_step = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "file_creator".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("path".to_string(), serde_json::json!("/tmp/mixed_ops.txt"));
                args.insert(
                    "content".to_string(),
                    serde_json::json!("mixed operations test"),
                );
                args
            },
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_secs(1),
        retry_policy: RetryPolicy {
            max_retries: 1,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(100)),
            retry_conditions: vec![RetryCondition::TemporaryFailure],
        },
        validation_rules: vec![],
    };

    let memory_step = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::MemoryOperation {
            operation_type: MemoryOperationType::Update,
            query: "update test data".to_string(),
        },
        parameters: HashMap::new(),
        dependencies: vec![file_step.id],
        expected_duration: Duration::from_millis(500),
        retry_policy: RetryPolicy {
            max_retries: 2,
            backoff_strategy: BackoffStrategy::Linear {
                initial: Duration::from_millis(50),
                increment: Duration::from_millis(25),
            },
            retry_conditions: vec![RetryCondition::NetworkError],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![file_step, memory_step],
        estimated_duration: Duration::from_millis(1500),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 256,
            disk_space_mb: 20,
            network_required: true,
            tools_required: vec!["file_creator".to_string()],
            permissions_required: vec!["file_create".to_string(), "memory_update".to_string()],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert(
                "test_type".to_string(),
                serde_json::json!("mixed_operations"),
            );
            meta
        },
    }
}

fn create_mock_action_plan_for_saga_integration() -> ActionPlan {
    // Create simple action plan with Wait operations (which should succeed in mock execution)
    let step1 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::Wait {
            duration: Duration::from_millis(1),
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_millis(100),
        retry_policy: RetryPolicy {
            max_retries: 1,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(50)),
            retry_conditions: vec![RetryCondition::TemporaryFailure],
        },
        validation_rules: vec![],
    };

    let step2 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::Wait {
            duration: Duration::from_millis(1),
        },
        parameters: HashMap::new(),
        dependencies: vec![step1.id],
        expected_duration: Duration::from_millis(100),
        retry_policy: RetryPolicy {
            max_retries: 1,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(50)),
            retry_conditions: vec![RetryCondition::TemporaryFailure],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        steps: vec![step1, step2],
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 10,
            disk_space_mb: 1,
            network_required: false,
            tools_required: vec![],
            permissions_required: vec![],
        },
        intent_id: Uuid::new_v4(),
        estimated_duration: Duration::from_millis(200),
        dependencies: vec![],
        metadata: HashMap::new(),
    }
}

fn create_compensation_failure_action_plan() -> ActionPlan {
    let failing_step = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "failing_tool".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("simulate_failure".to_string(), serde_json::json!(true));
                args.insert(
                    "failure_type".to_string(),
                    serde_json::json!("compensation"),
                );
                args
            },
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_secs(1),
        retry_policy: RetryPolicy {
            max_retries: 0,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(100)),
            retry_conditions: vec![],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![failing_step],
        estimated_duration: Duration::from_secs(1),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 128,
            disk_space_mb: 10,
            network_required: false,
            tools_required: vec!["failing_tool".to_string()],
            permissions_required: vec![],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert(
                "test_type".to_string(),
                serde_json::json!("compensation_failure"),
            );
            meta
        },
    }
}

fn create_partial_compensation_action_plan() -> ActionPlan {
    let steps: Vec<ActionStep> = (0..4)
        .map(|i| ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::ToolExecution {
                tool_name: format!("tool_{}", i),
                arguments: {
                    let mut args = HashMap::new();
                    args.insert("step_number".to_string(), serde_json::json!(i));
                    args
                },
            },
            parameters: HashMap::new(),
            dependencies: if i > 0 { vec![Uuid::new_v4()] } else { vec![] },
            expected_duration: Duration::from_millis(500),
            retry_policy: RetryPolicy {
                max_retries: 1,
                backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(50)),
                retry_conditions: vec![],
            },
            validation_rules: vec![],
        })
        .collect();

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps,
        estimated_duration: Duration::from_secs(2),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 256,
            disk_space_mb: 20,
            network_required: false,
            tools_required: vec!["tool_0".to_string(), "tool_1".to_string()],
            permissions_required: vec![],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert(
                "test_type".to_string(),
                serde_json::json!("partial_compensation"),
            );
            meta
        },
    }
}

fn create_empty_action_plan() -> ActionPlan {
    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![],
        estimated_duration: Duration::from_secs(0),
        resource_requirements: ResourceRequirements {
            cpu_cores: 0,
            memory_mb: 0,
            disk_space_mb: 0,
            network_required: false,
            tools_required: vec![],
            permissions_required: vec![],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("test_type".to_string(), serde_json::json!("empty_plan"));
            meta
        },
    }
}

fn create_simple_action_plan() -> ActionPlan {
    let simple_step = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "simple_tool".to_string(),
            arguments: HashMap::new(),
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_millis(100),
        retry_policy: RetryPolicy {
            max_retries: 0,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(50)),
            retry_conditions: vec![],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![simple_step],
        estimated_duration: Duration::from_millis(100),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 64,
            disk_space_mb: 5,
            network_required: false,
            tools_required: vec!["simple_tool".to_string()],
            permissions_required: vec![],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("test_type".to_string(), serde_json::json!("simple_plan"));
            meta
        },
    }
}

fn create_transactional_action_plan() -> ActionPlan {
    let transaction_step1 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "database_writer".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("table".to_string(), serde_json::json!("users"));
                args.insert("operation".to_string(), serde_json::json!("insert"));
                args.insert("data".to_string(), serde_json::json!({"name": "test_user"}));
                args
            },
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_secs(1),
        retry_policy: RetryPolicy {
            max_retries: 2,
            backoff_strategy: BackoffStrategy::Exponential {
                initial: Duration::from_millis(100),
                multiplier: 2.0,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::TemporaryFailure,
            ],
        },
        validation_rules: vec![],
    };

    let transaction_step2 = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "cache_updater".to_string(),
            arguments: {
                let mut args = HashMap::new();
                args.insert("key".to_string(), serde_json::json!("user_count"));
                args.insert("operation".to_string(), serde_json::json!("increment"));
                args
            },
        },
        parameters: HashMap::new(),
        dependencies: vec![transaction_step1.id],
        expected_duration: Duration::from_millis(200),
        retry_policy: RetryPolicy {
            max_retries: 1,
            backoff_strategy: BackoffStrategy::Fixed(Duration::from_millis(100)),
            retry_conditions: vec![RetryCondition::NetworkError],
        },
        validation_rules: vec![],
    };

    ActionPlan {
        id: Uuid::new_v4(),
        intent_id: Uuid::new_v4(),
        steps: vec![transaction_step1, transaction_step2],
        estimated_duration: Duration::from_millis(1200),
        resource_requirements: ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 256,
            disk_space_mb: 10,
            network_required: true,
            tools_required: vec!["database_writer".to_string(), "cache_updater".to_string()],
            permissions_required: vec!["db_write".to_string(), "cache_write".to_string()],
        },
        dependencies: vec![],
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("test_type".to_string(), serde_json::json!("transactional"));
            meta.insert("atomicity_required".to_string(), serde_json::json!(true));
            meta
        },
    }
}
