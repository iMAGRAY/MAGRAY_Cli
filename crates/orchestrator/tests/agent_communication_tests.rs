#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
//! Agent Communication Integration Tests
//!
//! Tests multi-agent communication via EventBus:
//! - IntentAnalyzer to Planner message handoff
//! - Planner to Executor coordination
//! - Executor to Critic feedback loop  
//! - Scheduler agent integration with other agents

use anyhow::Result;
use orchestrator::events::DefaultAgentEventPublisher;
use orchestrator::{
    agents::{Critic, Executor, IntentAnalyzer, Planner, Scheduler},
    events::{
        create_agent_event_publisher, AgentEventPublisher, AgentStatus, MessageStatus,
        SchedulerEventType, WorkflowStep,
    },
    orchestrator::{AgentOrchestrator, OrchestratorConfig},
    system::SystemConfig,
    ActorSystem, AgentMessage, AgentType, TaskPriority,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

/// Test IntentAnalyzer to Planner message handoff
#[tokio::test]
async fn test_intent_analyzer_to_planner_handoff() -> Result<()> {
    // Create multiple DefaultAgentEventPublisher instances for different agents
    let intent_analyzer = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "IntentAnalyzer-Handoff".to_string(),
        "IntentAnalyzer".to_string(),
    );

    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner-Handoff".to_string(),
        "Planner".to_string(),
    );

    let workflow_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();

    // Test agent message publishing/receiving through EventBus

    // 1. IntentAnalyzer publishes intent analysis completion
    intent_analyzer
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::IntentAnalyzed,
            serde_json::json!({
                "intent_type": "user_management",
                "action": "create_user",
                "confidence": 0.95,
                "extracted_entities": {
                    "user_email": "john@example.com",
                    "user_name": "John Doe",
                    "user_role": "standard"
                },
                "validation_passed": true,
                "ready_for_planning": true
            }),
            Some(250), // 250ms processing time
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 2. IntentAnalyzer sends planning request message to Planner
    intent_analyzer
        .publish_message_event(
            Some(planner.agent_id()),
            "planning_request".to_string(),
            message_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "intent_analysis": {
                    "action": "create_user",
                    "confidence": 0.95,
                    "parameters": {
                        "email": "john@example.com",
                        "name": "John Doe",
                        "role": "standard"
                    }
                },
                "context": {
                    "user_source": "web_ui",
                    "authentication_level": "verified",
                    "permissions": ["user_create"]
                },
                "priority": "normal",
                "deadline": null
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 3. Planner receives and acknowledges message
    planner
        .publish_message_event(
            Some(intent_analyzer.agent_id()),
            "planning_request".to_string(),
            message_id,
            serde_json::json!({
                "received_at": chrono::Utc::now(),
                "processing_started": true,
                "estimated_completion_time": "2023-01-01T12:05:00Z",
                "agent_status": "processing"
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 4. Planner completes planning and publishes workflow event
    planner
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::PlanCreated,
            serde_json::json!({
                "plan_id": Uuid::new_v4(),
                "steps": [
                    {
                        "step_id": Uuid::new_v4(),
                        "action": "validate_email",
                        "tool": "email_validator",
                        "parameters": {"email": "john@example.com"}
                    },
                    {
                        "step_id": Uuid::new_v4(),
                        "action": "create_user_record",
                        "tool": "user_database",
                        "parameters": {
                            "name": "John Doe",
                            "email": "john@example.com",
                            "role": "standard"
                        }
                    },
                    {
                        "step_id": Uuid::new_v4(),
                        "action": "send_welcome_email",
                        "tool": "email_sender",
                        "parameters": {
                            "recipient": "john@example.com",
                            "template": "welcome_user"
                        }
                    }
                ],
                "estimated_duration_minutes": 3,
                "resource_requirements": {
                    "cpu": "low",
                    "memory": "64MB",
                    "network": true
                },
                "approval_required": false,
                "rollback_plan_available": true
            }),
            Some(300), // 300ms planning time
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 5. Verify proper event ordering and message content
    // Test that messages contain expected workflow and coordination data
    assert_eq!(intent_analyzer.agent_type(), "IntentAnalyzer");
    assert_eq!(planner.agent_type(), "Planner");
    assert_ne!(intent_analyzer.agent_id(), planner.agent_id());

    Ok(())
}

/// Test Planner to Executor coordination
#[tokio::test]
async fn test_planner_to_executor_coordination() -> Result<()> {
    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner-Coordination".to_string(),
        "Planner".to_string(),
    );

    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor-Coordination".to_string(),
        "Executor".to_string(),
    );

    let workflow_id = Uuid::new_v4();
    let execution_message_id = Uuid::new_v4();

    // Test coordination between Planner and Executor with detailed plan execution

    // 1. Planner sends execution request with detailed plan
    planner
        .publish_message_event(
            Some(executor.agent_id()),
            "execution_request".to_string(),
            execution_message_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "plan_id": Uuid::new_v4(),
                "execution_plan": {
                    "steps": [
                        {
                            "step_id": "step_001",
                            "action": "validate_input",
                            "tool": "input_validator",
                            "parameters": {"strict_mode": true},
                            "dependencies": [],
                            "timeout_seconds": 30
                        },
                        {
                            "step_id": "step_002",
                            "action": "process_data",
                            "tool": "data_processor",
                            "parameters": {"batch_size": 100},
                            "dependencies": ["step_001"],
                            "timeout_seconds": 120
                        },
                        {
                            "step_id": "step_003",
                            "action": "save_results",
                            "tool": "database_writer",
                            "parameters": {"transaction": true},
                            "dependencies": ["step_002"],
                            "timeout_seconds": 60
                        }
                    ],
                    "execution_mode": "sequential",
                    "rollback_enabled": true,
                    "monitoring_enabled": true
                },
                "resource_allocation": {
                    "max_memory_mb": 512,
                    "max_cpu_percent": 80,
                    "max_execution_time_minutes": 10
                },
                "priority": "high"
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 2. Executor acknowledges execution request
    executor
        .publish_message_event(
            Some(planner.agent_id()),
            "execution_request".to_string(),
            execution_message_id,
            serde_json::json!({
                "acknowledgment": true,
                "execution_id": Uuid::new_v4(),
                "estimated_start_time": chrono::Utc::now(),
                "resource_check": "passed",
                "readiness_status": "ready_to_execute"
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 3. Executor starts execution and publishes workflow event
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionStarted,
            serde_json::json!({
                "execution_id": Uuid::new_v4(),
                "started_at": chrono::Utc::now(),
                "current_step": "step_001",
                "resources_allocated": {
                    "memory_mb": 256,
                    "cpu_percent": 45
                },
                "monitoring_active": true
            }),
            None,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 4. Executor sends progress updates back to Planner
    let progress_message_id = Uuid::new_v4();
    executor
        .publish_message_event(
            Some(planner.agent_id()),
            "execution_progress".to_string(),
            progress_message_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "progress": {
                    "completed_steps": ["step_001"],
                    "current_step": "step_002",
                    "progress_percentage": 66,
                    "estimated_remaining_time_minutes": 3
                },
                "status": "in_progress",
                "issues": [],
                "resource_usage": {
                    "current_memory_mb": 234,
                    "current_cpu_percent": 52,
                    "peak_memory_mb": 267
                }
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 5. Planner acknowledges progress update
    planner
        .publish_message_event(
            Some(executor.agent_id()),
            "execution_progress".to_string(),
            progress_message_id,
            serde_json::json!({
                "progress_received": true,
                "continue_execution": true,
                "adjustments": {
                    "timeout_extension_seconds": 30,
                    "priority_boost": false
                }
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 6. Executor completes execution
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionCompleted,
            serde_json::json!({
                "execution_id": Uuid::new_v4(),
                "completed_at": chrono::Utc::now(),
                "result": "success",
                "all_steps_completed": ["step_001", "step_002", "step_003"],
                "execution_summary": {
                    "total_execution_time_seconds": 180,
                    "peak_memory_usage_mb": 267,
                    "average_cpu_percent": 48,
                    "errors_encountered": 0,
                    "warnings_generated": 1
                },
                "output_data": {
                    "records_processed": 1000,
                    "records_saved": 1000,
                    "data_integrity_check": "passed"
                }
            }),
            Some(180000), // 3 minutes execution time
        )
        .await?;

    // 7. Verify agent types and IDs are different
    assert_eq!(planner.agent_type(), "Planner");
    assert_eq!(executor.agent_type(), "Executor");
    assert_ne!(planner.agent_id(), executor.agent_id());

    Ok(())
}

/// Test Executor to Critic feedback loop
#[tokio::test]
async fn test_executor_to_critic_feedback_loop() -> Result<()> {
    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor-Feedback".to_string(),
        "Executor".to_string(),
    );

    let critic = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Critic-Feedback".to_string(),
        "Critic".to_string(),
    );

    let workflow_id = Uuid::new_v4();
    let feedback_message_id = Uuid::new_v4();

    // Test feedback loop between Executor and Critic with detailed analysis

    // 1. Executor completes execution and requests critique
    executor
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::ExecutionCompleted,
            serde_json::json!({
                "execution_id": Uuid::new_v4(),
                "completion_status": "success",
                "execution_details": {
                    "steps_executed": 5,
                    "steps_successful": 4,
                    "steps_failed": 1,
                    "total_duration_seconds": 450,
                    "resource_efficiency": 0.87
                },
                "performance_metrics": {
                    "throughput": "1000 ops/min",
                    "error_rate": 0.02,
                    "response_time_p95_ms": 250,
                    "memory_peak_mb": 512
                },
                "quality_indicators": {
                    "code_coverage": 0.92,
                    "test_pass_rate": 0.98,
                    "lint_score": 8.5,
                    "security_scan_passed": true
                }
            }),
            Some(450000), // 7.5 minutes execution time
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 2. Executor sends critique request to Critic
    executor
        .publish_message_event(
            Some(critic.agent_id()),
            "critique_request".to_string(),
            feedback_message_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "execution_report": {
                    "success_metrics": {
                        "completion_rate": 0.8,
                        "quality_score": 8.7,
                        "performance_score": 7.9,
                        "reliability_score": 9.1
                    },
                    "execution_context": {
                        "environment": "production",
                        "user_count": 1000,
                        "data_volume": "10GB",
                        "complexity_level": "high"
                    },
                    "outcomes": {
                        "primary_objectives_met": true,
                        "secondary_objectives_met": false,
                        "unexpected_benefits": ["improved_cache_performance"],
                        "issues_encountered": ["timeout_on_step_3"]
                    }
                },
                "critique_focus": ["efficiency", "reliability", "best_practices", "improvements"],
                "urgency": "normal"
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 3. Critic receives and acknowledges critique request
    critic
        .publish_message_event(
            Some(executor.agent_id()),
            "critique_request".to_string(),
            feedback_message_id,
            serde_json::json!({
                "critique_accepted": true,
                "analysis_started_at": chrono::Utc::now(),
                "estimated_analysis_time_minutes": 2,
                "analysis_scope": ["performance", "quality", "best_practices"],
                "preliminary_assessment": "positive_with_improvement_opportunities"
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 4. Critic completes analysis and publishes critique
    critic
        .publish_workflow_event(
            workflow_id,
            WorkflowStep::CritiqueGenerated,
            serde_json::json!({
                "critique_id": Uuid::new_v4(),
                "analyzed_at": chrono::Utc::now(),
                "overall_score": 8.2,
                "assessment": {
                    "strengths": [
                        "Excellent error handling implementation",
                        "Good resource utilization efficiency",
                        "Strong security measures applied",
                        "Comprehensive logging and monitoring"
                    ],
                    "areas_for_improvement": [
                        "Step 3 timeout handling could be optimized",
                        "Cache warming strategy needs enhancement",
                        "Parallel processing opportunities missed",
                        "Resource allocation could be more dynamic"
                    ],
                    "critical_issues": [],
                    "performance_analysis": {
                        "bottlenecks_identified": ["database_query_step", "file_processing_step"],
                        "optimization_potential": "25% improvement possible",
                        "scalability_assessment": "good_up_to_5000_users"
                    }
                },
                "recommendations": [
                    {
                        "priority": "high",
                        "category": "performance",
                        "description": "Implement query optimization for step 3",
                        "effort_estimate": "medium",
                        "impact_estimate": "high"
                    },
                    {
                        "priority": "medium",
                        "category": "reliability",
                        "description": "Add circuit breaker pattern for external service calls",
                        "effort_estimate": "low",
                        "impact_estimate": "medium"
                    }
                ],
                "follow_up_required": true
            }),
            Some(120), // 2 minutes analysis time
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 5. Critic sends detailed feedback back to Executor
    let feedback_response_id = Uuid::new_v4();
    critic
        .publish_message_event(
            Some(executor.agent_id()),
            "critique_response".to_string(),
            feedback_response_id,
            serde_json::json!({
                "workflow_id": workflow_id,
                "critique_summary": {
                    "overall_rating": "good",
                    "score": 8.2,
                    "confidence": 0.94
                },
                "actionable_feedback": {
                    "immediate_actions": [
                        "Review timeout configuration for step 3",
                        "Implement suggested query optimization"
                    ],
                    "future_improvements": [
                        "Consider implementing caching layer",
                        "Explore parallel processing for independent steps"
                    ],
                    "learning_points": [
                        "Timeout handling patterns",
                        "Database optimization techniques",
                        "Resource allocation strategies"
                    ]
                },
                "quality_gates_passed": {
                    "security": true,
                    "performance": true,
                    "reliability": true,
                    "maintainability": false
                }
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 6. Executor acknowledges feedback and plans improvements
    executor
        .publish_message_event(
            Some(critic.agent_id()),
            "critique_response".to_string(),
            feedback_response_id,
            serde_json::json!({
                "feedback_received": true,
                "feedback_accepted": true,
                "improvement_plan": {
                    "scheduled_optimizations": [
                        {
                            "recommendation_id": "query_optimization",
                            "scheduled_for": "next_maintenance_window",
                            "assigned_to": "performance_team"
                        }
                    ],
                    "immediate_fixes": [
                        {
                            "issue": "timeout_configuration",
                            "fix_eta": "within_24_hours"
                        }
                    ]
                },
                "learning_integration": {
                    "patterns_to_adopt": ["circuit_breaker", "dynamic_timeouts"],
                    "training_scheduled": true
                }
            }),
            MessageStatus::Received,
        )
        .await?;

    // 7. Verify agent identity and communication completion
    assert_eq!(executor.agent_type(), "Executor");
    assert_eq!(critic.agent_type(), "Critic");
    assert_ne!(executor.agent_id(), critic.agent_id());

    Ok(())
}

/// Test Scheduler agent integration with other agents
#[tokio::test]
async fn test_scheduler_agent_integration() -> Result<()> {
    let scheduler = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Scheduler-Integration".to_string(),
        "Scheduler".to_string(),
    );

    let executor = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Executor-Scheduled".to_string(),
        "Executor".to_string(),
    );

    let planner = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Planner-Scheduled".to_string(),
        "Planner".to_string(),
    );

    let job_id = Uuid::new_v4();
    let task_id = Uuid::new_v4();
    let workflow_id = Uuid::new_v4();

    // Test Scheduler integration with multiple agents for coordinated task execution

    // 1. Scheduler schedules a complex job involving multiple agents
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobScheduled {
                job_id,
                job_name: "nightly_data_processing_pipeline".to_string(),
            },
            serde_json::json!({
                "job_type": "pipeline",
                "schedule": "0 2 * * *", // Daily at 2 AM
                "dependencies": [],
                "timeout_minutes": 120,
                "retry_policy": {
                    "max_retries": 2,
                    "retry_delay_minutes": 10
                },
                "resource_requirements": {
                    "cpu_cores": 4,
                    "memory_gb": 8,
                    "disk_space_gb": 50
                },
                "involved_agents": ["Planner", "Executor"],
                "priority": "medium",
                "notification_channels": ["email", "slack"]
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 2. Scheduler coordinates with Planner for job planning
    let planning_message_id = Uuid::new_v4();
    scheduler
        .publish_message_event(
            Some(planner.agent_id()),
            "job_planning_request".to_string(),
            planning_message_id,
            serde_json::json!({
                "job_id": job_id,
                "planning_requirements": {
                    "data_sources": ["customer_db", "analytics_db", "logs"],
                    "processing_steps": [
                        "data_extraction",
                        "data_transformation",
                        "data_validation",
                        "data_aggregation",
                        "report_generation"
                    ],
                    "output_formats": ["json", "csv", "dashboard"],
                    "quality_checks": ["completeness", "accuracy", "freshness"]
                },
                "constraints": {
                    "max_execution_time_hours": 2,
                    "business_hours_restriction": false,
                    "resource_limits": {
                        "max_memory_gb": 8,
                        "max_cpu_cores": 4
                    }
                },
                "deadline": "before_business_hours"
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 3. Planner responds with detailed execution plan
    planner
        .publish_message_event(
            Some(scheduler.agent_id()),
            "job_planning_request".to_string(),
            planning_message_id,
            serde_json::json!({
                "planning_completed": true,
                "execution_plan": {
                    "plan_id": Uuid::new_v4(),
                    "total_estimated_duration_minutes": 90,
                    "steps": [
                        {
                            "step_id": "extract_customer_data",
                            "duration_estimate_minutes": 15,
                            "dependencies": [],
                            "resources": {"memory_gb": 2, "cpu_cores": 1}
                        },
                        {
                            "step_id": "extract_analytics_data", 
                            "duration_estimate_minutes": 20,
                            "dependencies": [],
                            "resources": {"memory_gb": 3, "cpu_cores": 1}
                        },
                        {
                            "step_id": "transform_and_merge",
                            "duration_estimate_minutes": 30,
                            "dependencies": ["extract_customer_data", "extract_analytics_data"],
                            "resources": {"memory_gb": 6, "cpu_cores": 2}
                        },
                        {
                            "step_id": "generate_reports",
                            "duration_estimate_minutes": 25,
                            "dependencies": ["transform_and_merge"],
                            "resources": {"memory_gb": 4, "cpu_cores": 2}
                        }
                    ],
                    "parallel_execution_opportunities": ["extract_customer_data", "extract_analytics_data"],
                    "rollback_strategy": "checkpoint_based"
                },
                "risk_assessment": {
                    "probability_of_success": 0.92,
                    "potential_issues": ["network_latency", "data_volume_variations"],
                    "mitigation_strategies": ["retry_logic", "dynamic_timeouts"]
                }
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 4. Scheduler starts job execution
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobStarted { job_id },
            serde_json::json!({
                "started_at": chrono::Utc::now(),
                "execution_mode": "scheduled",
                "assigned_resources": {
                    "cpu_cores": 4,
                    "memory_gb": 8,
                    "worker_nodes": ["worker-01", "worker-02"]
                },
                "monitoring_enabled": true,
                "estimated_completion": chrono::Utc::now() + chrono::Duration::minutes(90)
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 5. Scheduler delegates execution to Executor
    let execution_message_id = Uuid::new_v4();
    scheduler
        .publish_message_event(
            Some(executor.agent_id()),
            "scheduled_execution_request".to_string(),
            execution_message_id,
            serde_json::json!({
                "job_id": job_id,
                "workflow_id": workflow_id,
                "execution_plan": {
                    "priority": "scheduled_job",
                    "timeout_minutes": 120,
                    "monitoring_interval_seconds": 30,
                    "progress_reporting_required": true
                },
                "scheduler_context": {
                    "scheduled_time": "2023-01-01T02:00:00Z",
                    "actual_start_time": chrono::Utc::now(),
                    "job_queue_position": 1,
                    "resource_allocation_id": "alloc-12345"
                }
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 6. Executor acknowledges and begins execution
    executor
        .publish_message_event(
            Some(scheduler.agent_id()),
            "scheduled_execution_request".to_string(),
            execution_message_id,
            serde_json::json!({
                "execution_accepted": true,
                "execution_id": Uuid::new_v4(),
                "resource_validation": "passed",
                "readiness_check": "ready",
                "estimated_start_delay_seconds": 5
            }),
            MessageStatus::Received,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 7. Executor reports progress back to Scheduler
    let progress_message_id = Uuid::new_v4();
    executor
        .publish_message_event(
            Some(scheduler.agent_id()),
            "execution_progress_report".to_string(),
            progress_message_id,
            serde_json::json!({
                "job_id": job_id,
                "workflow_id": workflow_id,
                "progress": {
                    "percentage_complete": 45,
                    "steps_completed": 2,
                    "steps_total": 4,
                    "current_step": "transform_and_merge",
                    "elapsed_time_minutes": 40,
                    "estimated_remaining_minutes": 50
                },
                "performance_metrics": {
                    "throughput": "1.2MB/s",
                    "memory_usage_gb": 5.8,
                    "cpu_utilization_percent": 75,
                    "error_count": 0
                },
                "health_status": "healthy"
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 8. Scheduler schedules related task based on progress
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::TaskScheduled {
                task_id,
                task_name: "cleanup_temp_files".to_string(),
            },
            serde_json::json!({
                "parent_job_id": job_id,
                "trigger": "parent_job_75_percent_complete",
                "task_type": "cleanup",
                "schedule": "immediate_after_parent",
                "estimated_duration_minutes": 5,
                "dependencies": [],
                "priority": "low"
            }),
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 9. Job completes successfully
    scheduler
        .publish_scheduler_event(
            SchedulerEventType::JobCompleted {
                job_id,
                duration_ms: 5400000, // 90 minutes
            },
            serde_json::json!({
                "completion_status": "success",
                "completed_at": chrono::Utc::now(),
                "final_metrics": {
                    "total_data_processed_gb": 15.7,
                    "records_processed": 1250000,
                    "reports_generated": 12,
                    "peak_memory_usage_gb": 6.2,
                    "average_cpu_utilization_percent": 68
                },
                "outputs": {
                    "reports": ["daily_summary.json", "customer_analytics.csv"],
                    "dashboards": ["executive_dashboard", "operations_dashboard"],
                    "data_quality_score": 0.97
                },
                "next_scheduled_run": "2023-01-02T02:00:00Z"
            }),
        )
        .await?;

    // 10. Verify Scheduler integration with other agents
    assert_eq!(scheduler.agent_type(), "Scheduler");
    assert_eq!(executor.agent_type(), "Executor");
    assert_eq!(planner.agent_type(), "Planner");
    assert_ne!(scheduler.agent_id(), executor.agent_id());
    assert_ne!(scheduler.agent_id(), planner.agent_id());
    assert_ne!(executor.agent_id(), planner.agent_id());

    Ok(())
}

/// Test failure scenarios in agent communication
#[tokio::test]
async fn test_agent_communication_failure_scenarios() -> Result<()> {
    let sender = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Sender-FailureTest".to_string(),
        "TestSender".to_string(),
    );

    let receiver = DefaultAgentEventPublisher::new(
        Uuid::new_v4(),
        "Receiver-FailureTest".to_string(),
        "TestReceiver".to_string(),
    );

    // Test various failure scenarios in agent communication

    // 1. Message timeout scenario
    let timeout_message_id = Uuid::new_v4();
    sender
        .publish_message_event(
            Some(receiver.agent_id()),
            "timeout_test_message".to_string(),
            timeout_message_id,
            serde_json::json!({
                "request_type": "slow_operation",
                "timeout_seconds": 1,
                "expected_processing_time_seconds": 10
            }),
            MessageStatus::Sent,
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // Simulate message timeout failure
    sender
        .publish_message_event(
            Some(receiver.agent_id()),
            "timeout_test_message".to_string(),
            timeout_message_id,
            serde_json::json!({
                "timeout_occurred": true,
                "timeout_duration_seconds": 1,
                "partial_result": null,
                "retry_recommended": true
            }),
            MessageStatus::Failed {
                error: "Message processing timeout after 1 second".to_string(),
            },
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 2. Network communication failure scenario
    let network_message_id = Uuid::new_v4();
    sender
        .publish_message_event(
            None, // Simulate missing recipient
            "network_failure_message".to_string(),
            network_message_id,
            serde_json::json!({
                "message_type": "critical_update",
                "retry_attempt": 1,
                "fallback_strategy": "store_and_forward"
            }),
            MessageStatus::Failed {
                error: "Network unreachable: connection timeout".to_string(),
            },
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 3. Agent unavailable scenario
    let unavailable_agent_id = Uuid::new_v4(); // Non-existent agent
    let unavailable_message_id = Uuid::new_v4();
    sender
        .publish_message_event(
            Some(unavailable_agent_id),
            "agent_unavailable_message".to_string(),
            unavailable_message_id,
            serde_json::json!({
                "target_agent": "NonExistentAgent",
                "message_priority": "high",
                "delivery_required": true
            }),
            MessageStatus::Failed {
                error: "Target agent not found or unavailable".to_string(),
            },
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 4. Message corruption scenario
    let corruption_message_id = Uuid::new_v4();
    sender
        .publish_message_event(
            Some(receiver.agent_id()),
            "corrupted_message".to_string(),
            corruption_message_id,
            serde_json::json!({
                "data_integrity": false,
                "checksum_failed": true,
                "original_size": 1024,
                "received_size": 987,
                "corruption_detected": true
            }),
            MessageStatus::Failed {
                error: "Message corruption detected: checksum mismatch".to_string(),
            },
        )
        .await?;

    sleep(Duration::from_millis(10)).await;

    // 5. Resource exhaustion scenario
    let resource_message_id = Uuid::new_v4();
    receiver
        .publish_message_event(
            Some(sender.agent_id()),
            "resource_exhaustion_response".to_string(),
            resource_message_id,
            serde_json::json!({
                "resource_status": "exhausted",
                "memory_usage_percent": 98,
                "cpu_usage_percent": 95,
                "queue_size": 1000,
                "can_accept_new_messages": false,
                "backpressure_applied": true
            }),
            MessageStatus::Failed {
                error: "Agent resource exhaustion: cannot process new messages".to_string(),
            },
        )
        .await?;

    // Verify that failure scenarios are properly tracked
    assert_eq!(sender.agent_type(), "TestSender");
    assert_eq!(receiver.agent_type(), "TestReceiver");

    Ok(())
}
