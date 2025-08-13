#![allow(clippy::uninlined_format_args)]
//! EventBus Agent Integration Example
//!
//! Demonstrates how multi-agent system integrates with EventBus for event-driven architecture.
//! Shows agent lifecycle events, workflow events, and inter-agent communication.

use anyhow::Result;
use orchestrator::events::DefaultAgentEventPublisher;
use orchestrator::{AgentEventPublisher, AgentStatus, SchedulerEventType, WorkflowStep};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🚀 Starting EventBus Agent Integration Demo");

    // Create multiple agents with EventBus publishers
    let intent_analyzer = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "IntentAnalyzer-001".to_string(),
        "IntentAnalyzer".to_string(),
    );

    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner-001".to_string(),
        "Planner".to_string(),
    );

    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor-001".to_string(),
        "Executor".to_string(),
    );

    let critic = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Critic-001".to_string(),
        "Critic".to_string(),
    );

    let scheduler = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Scheduler-001".to_string(),
        "Scheduler".to_string(),
    );

    // Demo 1: Agent Lifecycle Events
    println!("\n📋 Demo 1: Agent Lifecycle Events");
    demo_agent_lifecycle(&intent_analyzer).await?;
    demo_agent_lifecycle(&planner).await?;
    demo_agent_lifecycle(&executor).await?;

    // Demo 2: Multi-Agent Workflow Events
    println!("\n🔄 Demo 2: Multi-Agent Workflow Events");
    let workflow_id = Uuid::new_v4();
    demo_workflow_events(&intent_analyzer, &planner, &executor, &critic, workflow_id).await?;

    // Demo 3: Scheduler Events
    println!("\n⏰ Demo 3: Scheduler Events");
    demo_scheduler_events(&scheduler).await?;

    // Demo 4: Agent Communication Events
    println!("\n💬 Demo 4: Agent Communication Events");
    demo_agent_communication(&intent_analyzer, &planner).await?;

    println!("\n✅ EventBus Agent Integration Demo completed successfully!");

    Ok(())
}

/// Demonstrate agent lifecycle events
async fn demo_agent_lifecycle(agent: &DefaultAgentEventPublisher) -> Result<()> {
    println!("  🔵 Starting agent lifecycle for {}", agent.agent_name());

    // Agent startup
    agent
        .publish_lifecycle_event(
            AgentStatus::Started,
            serde_json::json!({
                "startup_time_ms": 150,
                "config": {
                    "max_memory_mb": 512,
                    "timeout_seconds": 30
                }
            }),
        )
        .await?;

    sleep(Duration::from_millis(100)).await;

    // Agent health check
    agent
        .publish_lifecycle_event(
            AgentStatus::Healthy,
            serde_json::json!({
                "cpu_usage_percent": 15.5,
                "memory_usage_mb": 128,
                "active_tasks": 2
            }),
        )
        .await?;

    Ok(())
}

/// Demonstrate complete multi-agent workflow
async fn demo_workflow_events(
    intent_analyzer: &DefaultAgentEventPublisher,
    planner: &DefaultAgentEventPublisher,
    executor: &DefaultAgentEventPublisher,
    critic: &DefaultAgentEventPublisher,
    workflow_id: Uuid,
) -> Result<()> {
    println!("  🔄 Running multi-agent workflow {}", workflow_id);

    // Step 1: Intent Analysis
    intent_analyzer
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::IntentAnalyzed,
            serde_json::json!({
                "user_input": "Create a new user account with email verification",
                "detected_intent": "user_creation",
                "confidence": 0.95,
                "entities": {
                    "action": "create",
                    "object": "user_account",
                    "features": ["email_verification"]
                }
            }),
            Some(250),
        )
        .await?;

    sleep(Duration::from_millis(50)).await;

    // Step 2: Plan Creation
    planner
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::PlanCreated,
            serde_json::json!({
                "plan_id": Uuid::new_v4(),
                "steps": [
                    {
                        "step_id": 1,
                        "action": "validate_email_format",
                        "tool": "email_validator"
                    },
                    {
                        "step_id": 2,
                        "action": "check_user_exists",
                        "tool": "database_query"
                    },
                    {
                        "step_id": 3,
                        "action": "create_user_record",
                        "tool": "database_insert"
                    },
                    {
                        "step_id": 4,
                        "action": "send_verification_email",
                        "tool": "email_service"
                    }
                ],
                "estimated_duration_seconds": 15,
                "risk_level": "medium"
            }),
            Some(180),
        )
        .await?;

    sleep(Duration::from_millis(50)).await;

    // Step 3: Execution Start
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionStarted,
            serde_json::json!({
                "execution_id": Uuid::new_v4(),
                "plan_id": workflow_id,
                "parallel_steps": false,
                "resources_allocated": {
                    "cpu_cores": 2,
                    "memory_mb": 256,
                    "timeout_seconds": 30
                }
            }),
            None,
        )
        .await?;

    sleep(Duration::from_millis(200)).await;

    // Step 4: Execution Completion
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionCompleted,
            serde_json::json!({
                "execution_id": workflow_id,
                "steps_completed": 4,
                "steps_failed": 0,
                "result": {
                    "user_created": true,
                    "user_id": "user_12345",
                    "verification_email_sent": true,
                    "verification_token": "token_abcd1234"
                },
                "resource_usage": {
                    "cpu_time_ms": 850,
                    "memory_peak_mb": 128,
                    "database_queries": 3,
                    "external_api_calls": 1
                }
            }),
            Some(850),
        )
        .await?;

    sleep(Duration::from_millis(50)).await;

    // Step 5: Critique Generation
    critic
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::CritiqueGenerated,
            serde_json::json!({
                "critique_id": Uuid::new_v4(),
                "workflow_id": workflow_id,
                "overall_score": 8.5,
                "quality_metrics": {
                    "efficiency": 9.0,
                    "reliability": 8.0,
                    "security": 8.5,
                    "user_experience": 8.0
                },
                "suggestions": [
                    {
                        "category": "performance",
                        "suggestion": "Consider caching email validation results",
                        "priority": "low",
                        "estimated_improvement": "5-10% faster execution"
                    },
                    {
                        "category": "security",
                        "suggestion": "Add rate limiting for account creation",
                        "priority": "medium",
                        "estimated_improvement": "Prevents abuse"
                    }
                ],
                "success_indicators": {
                    "user_created": true,
                    "email_sent": true,
                    "no_errors": true,
                    "within_time_budget": true
                }
            }),
            Some(120),
        )
        .await?;

    Ok(())
}

/// Demonstrate scheduler events
async fn demo_scheduler_events(scheduler: &DefaultAgentEventPublisher) -> Result<()> {
    println!("  ⏰ Demonstrating scheduler events");

    let job_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();

    // Job scheduled
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobScheduled {
                job_id,
                job_name: "daily_cleanup_job".to_string(),
            },
            serde_json::json!({
                "schedule": "0 2 * * *", // Daily at 2 AM
                "priority": "low",
                "estimated_duration_minutes": 15,
                "resource_requirements": {
                    "cpu": "low",
                    "memory_mb": 64,
                    "disk_space_mb": 100
                }
            }),
        )
        .await?;

    sleep(Duration::from_millis(50)).await;

    // Job started
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobStarted { job_id },
            serde_json::json!({
                "started_at": chrono::Utc::now(),
                "worker_id": "worker_001",
                "allocated_resources": {
                    "cpu_cores": 1,
                    "memory_mb": 64
                }
            }),
        )
        .await?;

    sleep(Duration::from_millis(100)).await;

    // Task scheduled
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::TaskScheduled {
                task_id,
                task_name: "delete_old_logs".to_string(),
            },
            serde_json::json!({
                "parent_job_id": job_id,
                "task_type": "filesystem_cleanup",
                "target_directory": "/var/log/old",
                "retention_days": 30
            }),
        )
        .await?;

    sleep(Duration::from_millis(150)).await;

    // Job completed
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobCompleted {
                job_id,
                duration_ms: 12500,
            },
            serde_json::json!({
                "completed_at": chrono::Utc::now(),
                "result": "success",
                "files_processed": 1247,
                "space_freed_mb": 89,
                "warnings": [],
                "performance_metrics": {
                    "average_file_process_time_ms": 10,
                    "peak_memory_usage_mb": 45
                }
            }),
        )
        .await?;

    Ok(())
}

/// Demonstrate agent-to-agent communication events
async fn demo_agent_communication(
    sender: &DefaultAgentEventPublisher,
    receiver: &DefaultAgentEventPublisher,
) -> Result<()> {
    println!("  💬 Demonstrating agent communication");

    let message_id = Uuid::new_v4();

    // Message sent
    sender
        .publish_message_event(
            Some(receiver.agent_id()), // to specific agent
            "plan_request".to_string(),
            message_id,
            serde_json::json!({
                "request_type": "create_plan",
                "user_intent": {
                    "action": "deploy_application",
                    "environment": "staging",
                    "rollback_strategy": "blue_green"
                },
                "constraints": {
                    "max_downtime_minutes": 5,
                    "budget_usd": 100
                },
                "priority": "high"
            }),
            orchestrator::MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(50)).await;

    // Message received (from receiver's perspective)
    receiver
        .publish_message_event(
            Some(sender.agent_id()), // from sender
            "plan_request".to_string(),
            message_id,
            serde_json::json!({
                "received_at": chrono::Utc::now(),
                "processing_started": true,
                "estimated_response_time_seconds": 3
            }),
            orchestrator::MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(100)).await;

    // Response sent back
    let response_message_id = Uuid::new_v4();
    receiver
        .publish_message_event(
            Some(sender.agent_id()), // back to sender
            "plan_response".to_string(),
            response_message_id,
            serde_json::json!({
                "original_message_id": message_id,
                "response_type": "plan_created",
                "plan": {
                    "plan_id": Uuid::new_v4(),
                    "deployment_steps": [
                        "backup_current_version",
                        "deploy_new_version_to_staging_slots",
                        "run_health_checks",
                        "switch_traffic_gradually",
                        "monitor_metrics"
                    ],
                    "estimated_duration_minutes": 8,
                    "rollback_plan": {
                        "trigger_conditions": ["error_rate > 1%", "response_time > 500ms"],
                        "rollback_steps": ["switch_traffic_back", "restore_backup"]
                    }
                },
                "confidence": 0.92
            }),
            orchestrator::MessageStatus::Sent,
        )
        .await?;

    Ok(())
}
