//! EventBus Integration Tests
//!
//! Tests to verify P1.1.8 EventBus Integration implementation:
//! - Agent lifecycle events
//! - Multi-agent workflow events
//! - Agent communication events
//! - Scheduler events
//! - Event-driven architecture functionality

use anyhow::Result;
use orchestrator::events::DefaultAgentEventPublisher;
use orchestrator::{
    AgentEventPublisher, AgentStatus, MessageStatus, SchedulerEventType, WorkflowStep,
};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Test agent lifecycle event publishing
#[tokio::test]
async fn test_agent_lifecycle_events() -> Result<()> {
    let agent = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "TestAgent".to_string(),
        "LifecycleTest".to_string(),
    );

    // Test agent started event
    agent
        .publish_lifecycle_event(
            AgentStatus::Started,
            serde_json::json!({
                "startup_time_ms": 100,
                "version": "1.0.0"
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test agent health check event
    agent
        .publish_lifecycle_event(
            AgentStatus::Healthy,
            serde_json::json!({
                "cpu_usage": 25.5,
                "memory_mb": 128
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test agent failure event
    agent
        .publish_lifecycle_event(
            AgentStatus::Failed {
                error: "Connection timeout".to_string(),
            },
            serde_json::json!({
                "error_code": "CONN_TIMEOUT",
                "retry_count": 3
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test agent stopped event
    agent
        .publish_lifecycle_event(
            AgentStatus::Stopped,
            serde_json::json!({
                "shutdown_reason": "graceful",
                "uptime_seconds": 3600
            }),
        )
        .await?;

    Ok(())
}

/// Test multi-agent workflow events
#[tokio::test]
async fn test_workflow_events() -> Result<()> {
    let intent_analyzer = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "IntentAnalyzer".to_string(),
        "WorkflowTest".to_string(),
    );

    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner".to_string(),
        "WorkflowTest".to_string(),
    );

    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor".to_string(),
        "WorkflowTest".to_string(),
    );

    let workflow_id = Uuid::new_v4();

    // Step 1: Intent analyzed
    intent_analyzer
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::IntentAnalyzed,
            serde_json::json!({
                "intent": "create_user",
                "confidence": 0.95
            }),
            Some(150),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Step 2: Plan created
    planner
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::PlanCreated,
            serde_json::json!({
                "plan_id": Uuid::new_v4(),
                "steps": 3,
                "estimated_duration": 10
            }),
            Some(200),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Step 3: Execution started
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionStarted,
            serde_json::json!({
                "execution_id": Uuid::new_v4(),
                "resources_allocated": true
            }),
            None,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Step 4: Execution completed
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionCompleted,
            serde_json::json!({
                "result": "success",
                "user_id": "user_12345"
            }),
            Some(500),
        )
        .await?;

    Ok(())
}

/// Test agent communication events
#[tokio::test]
async fn test_agent_communication_events() -> Result<()> {
    let sender = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "SenderAgent".to_string(),
        "CommunicationTest".to_string(),
    );

    let receiver = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "ReceiverAgent".to_string(),
        "CommunicationTest".to_string(),
    );

    let message_id = Uuid::new_v4();

    // Test message sent
    sender
        .publish_message_event(
            Some(receiver.agent_id()),
            "test_message".to_string(),
            message_id,
            serde_json::json!({
                "content": "Hello, receiver!",
                "priority": "high"
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test message received
    receiver
        .publish_message_event(
            Some(sender.agent_id()),
            "test_message".to_string(),
            message_id,
            serde_json::json!({
                "acknowledged": true,
                "processing_started": true
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test message failed
    let failed_message_id = Uuid::new_v4();
    sender
        .publish_message_event(
            Some(receiver.agent_id()),
            "failed_message".to_string(),
            failed_message_id,
            serde_json::json!({
                "content": "This will fail",
                "retry_attempt": 1
            }),
            MessageStatus::Failed {
                error: "Network timeout".to_string(),
            },
        )
        .await?;

    Ok(())
}

/// Test scheduler events
#[tokio::test]
async fn test_scheduler_events() -> Result<()> {
    let scheduler = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "TestScheduler".to_string(),
        "SchedulerTest".to_string(),
    );

    let job_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    // Test job scheduled event
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobScheduled {
                job_id,
                job_name: "test_job".to_string(),
            },
            serde_json::json!({
                "priority": "medium",
                "schedule": "0 */6 * * *"
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test job started event
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobStarted { job_id },
            serde_json::json!({
                "worker_id": "worker_001",
                "started_at": chrono::Utc::now()
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test task scheduled event
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::TaskScheduled {
                task_id,
                task_name: "cleanup_task".to_string(),
            },
            serde_json::json!({
                "parent_job_id": job_id,
                "task_type": "cleanup"
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test task executed event
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::TaskExecuted {
                task_id,
                duration_ms: 1500,
            },
            serde_json::json!({
                "result": "success",
                "items_processed": 100
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test job completed event
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobCompleted {
                job_id,
                duration_ms: 5000,
            },
            serde_json::json!({
                "result": "success",
                "total_tasks": 3,
                "performance": {
                    "avg_task_duration_ms": 1667
                }
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Test job failed event
    let failed_job_id = Uuid::new_v4();
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobFailed {
                job_id: failed_job_id,
                error: "Database connection failed".to_string(),
            },
            serde_json::json!({
                "retry_count": 2,
                "next_retry_at": chrono::Utc::now() + chrono::Duration::minutes(5)
            }),
        )
        .await?;

    Ok(())
}

/// Test AgentEventPublisher getters
#[test]
fn test_agent_event_publisher_getters() {
    let agent_id = Uuid::new_v4();
    let agent_name = "TestAgent".to_string();
    let agent_type = "TestType".to_string();

    let publisher =
        DefaultAgentEventPublisher::new(agent_id, agent_name.clone(), agent_type.clone());

    assert_eq!(publisher.agent_id(), agent_id);
    assert_eq!(publisher.agent_name(), &agent_name);
    assert_eq!(publisher.agent_type(), &agent_type);
}

/// Test complex multi-agent scenario
#[tokio::test]
async fn test_complex_multi_agent_scenario() -> Result<()> {
    // Create multiple agents
    let intent_analyzer = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "IntentAnalyzer-Complex".to_string(),
        "IntentAnalyzer".to_string(),
    );

    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner-Complex".to_string(),
        "Planner".to_string(),
    );

    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor-Complex".to_string(),
        "Executor".to_string(),
    );

    let critic = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Critic-Complex".to_string(),
        "Critic".to_string(),
    );

    let scheduler = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Scheduler-Complex".to_string(),
        "Scheduler".to_string(),
    );

    // Simulate complex workflow
    let workflow_id = Uuid::new_v4();

    // 1. All agents start
    for agent in &[&intent_analyzer, &planner, &executor, &critic, &scheduler] {
        agent
            .publish_lifecycle_event(
                AgentStatus::Started,
                serde_json::json!({
                    "agent_type": agent.agent_type(),
                    "startup_time_ms": 100
                }),
            )
            .await?;
    }

    sleep(Duration::from_millis(10)).await;

    // 2. Intent analysis workflow
    intent_analyzer
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::IntentAnalyzed,
            serde_json::json!({
                "user_intent": "deploy application to production",
                "complexity": "high",
                "requires_approval": true
            }),
            Some(300),
        )
        .await?;

    // 3. Inter-agent communication: Intent → Planner
    let message_id = Uuid::new_v4();
    intent_analyzer
        .publish_message_event(
            Some(planner.agent_id()),
            "planning_request".to_string(),
            message_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "intent_data": {
                    "action": "deploy",
                    "target": "production",
                    "risk_level": "high"
                }
            }),
            MessageStatus::Sent,
        )
        .await?;

    // 4. Planner receives and processes
    planner
        .publish_message_event(
            Some(intent_analyzer.agent_id()),
            "planning_request".to_string(),
            message_id,
            serde_json::json!({
                "received_at": chrono::Utc::now(),
                "processing_started": true
            }),
            MessageStatus::Received,
        )
        .await?;

    planner
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::PlanCreated,
            serde_json::json!({
                "plan_id": Uuid::new_v4(),
                "deployment_strategy": "blue_green",
                "approval_required": true,
                "estimated_duration_minutes": 45
            }),
            Some(500),
        )
        .await?;

    // 5. Scheduler schedules deployment job
    let job_id = Uuid::new_v4();
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobScheduled {
                job_id,
                job_name: "production_deployment".to_string(),
            },
            serde_json::json!({
                "workflow_id": workflow_id,
                "scheduled_for": chrono::Utc::now() + chrono::Duration::minutes(10),
                "priority": "critical"
            }),
        )
        .await?;

    // 6. Execution starts
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobStarted { job_id },
            serde_json::json!({
                "workflow_id": workflow_id,
                "assigned_executor": executor.agent_id()
            }),
        )
        .await?;

    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionStarted,
            serde_json::json!({
                "job_id": job_id,
                "deployment_strategy": "blue_green",
                "target_environment": "production"
            }),
            None,
        )
        .await?;

    // 7. Execution completes successfully
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionCompleted,
            serde_json::json!({
                "job_id": job_id,
                "deployment_result": "success",
                "new_version": "v2.1.0",
                "rollback_available": true,
                "health_checks_passed": true
            }),
            Some(2700000), // 45 minutes in ms
        )
        .await?;

    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobCompleted {
                job_id,
                duration_ms: 2700000,
            },
            serde_json::json!({
                "workflow_id": workflow_id,
                "final_result": "deployment_successful",
                "performance_metrics": {
                    "downtime_seconds": 0,
                    "rollout_percentage": 100
                }
            }),
        )
        .await?;

    // 8. Critic provides feedback
    critic
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::CritiqueGenerated,
            serde_json::json!({
                "workflow_id": workflow_id,
                "overall_score": 9.2,
                "evaluation": {
                    "efficiency": 9.0,
                    "reliability": 9.5,
                    "security": 9.0,
                    "best_practices": 9.2
                },
                "recommendations": [
                    {
                        "area": "monitoring",
                        "suggestion": "Add post-deployment monitoring alerts",
                        "priority": "medium"
                    }
                ]
            }),
            Some(180),
        )
        .await?;

    // 9. All agents report healthy status
    for agent in &[&intent_analyzer, &planner, &executor, &critic, &scheduler] {
        agent
            .publish_lifecycle_event(
                AgentStatus::Healthy,
                serde_json::json!({
                    "workflow_completed": workflow_id,
                    "resource_usage": {
                        "cpu_percent": 12.5,
                        "memory_mb": 256
                    },
                    "performance": "nominal"
                }),
            )
            .await?;
    }

    Ok(())
}

/// Test agent status serialization and deserialization
#[test]
fn test_agent_status_serialization() -> Result<()> {
    // Test various AgentStatus variants
    let statuses = vec![
        AgentStatus::Started,
        AgentStatus::Running,
        AgentStatus::Stopped,
        AgentStatus::Failed {
            error: "Test error".to_string(),
        },
        AgentStatus::Healthy,
        AgentStatus::Unhealthy {
            reason: "Memory limit exceeded".to_string(),
        },
    ];

    for status in statuses {
        let serialized = serde_json::to_value(&status)?;
        let deserialized: AgentStatus = serde_json::from_value(serialized)?;

        match (&status, &deserialized) {
            (AgentStatus::Started, AgentStatus::Started) => {}
            (AgentStatus::Running, AgentStatus::Running) => {}
            (AgentStatus::Stopped, AgentStatus::Stopped) => {}
            (AgentStatus::Failed { error: e1 }, AgentStatus::Failed { error: e2 }) => {
                assert_eq!(e1, e2);
            }
            (AgentStatus::Healthy, AgentStatus::Healthy) => {}
            (AgentStatus::Unhealthy { reason: r1 }, AgentStatus::Unhealthy { reason: r2 }) => {
                assert_eq!(r1, r2);
            }
            _ => panic!("Status serialization/deserialization mismatch"),
        }
    }

    Ok(())
}
