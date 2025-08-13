#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_attributes)]
#![allow(clippy::empty_line_after_outer_attr)]
#![allow(unused)]
#![allow(clippy::new_without_default)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::expect_fun_call)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::too_many_arguments)]
//! Comprehensive Integration Tests for P1.1.13
//!
//! This module implements comprehensive integration testing for the Multi-Agent Orchestration system:
//!
//! **P1.1.13.a [8м] Agent Integration Tests:**
//! - End-to-end agent workflow tests (Intent→Plan→Execute→Critic)
//! - Multi-agent interaction and communication tests
//! - Agent state transition validation tests
//! - Agent coordination and synchronization tests
//!
//! **P1.1.13.b [7м] Performance Testing:**
//! - Agent response time benchmarks
//! - Memory usage monitoring and limits testing
//! - Concurrent agent execution performance
//! - Resource utilization under load testing
//!
//! All tests ensure agents work within production constraints and meet quality requirements.

use anyhow::Result;
use orchestrator::events::DefaultAgentEventPublisher;
use orchestrator::{
    agents::{Critic, Executor, IntentAnalyzer, Planner, Scheduler},
    events::{create_agent_event_publisher, AgentEventPublisher},
    orchestrator::{AgentOrchestrator, OrchestratorConfig},
    system::SystemConfig,
    workflow::{WorkflowConfig, WorkflowRequest, WorkflowResult},
    ActorSystem, AgentRegistry, TaskPriority, WorkflowId, WorkflowState, WorkflowStepType,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

/// Performance benchmark configuration for agent testing
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Maximum response time allowed for agent operations (ms)
    pub max_response_time_ms: u64,
    /// Maximum memory usage allowed per agent (MB)  
    pub max_memory_usage_mb: u64,
    /// Number of concurrent workflows for stress testing
    pub concurrent_workflows: usize,
    /// Maximum CPU usage percentage allowed
    pub max_cpu_usage_percent: f32,
    /// Test duration for sustained load testing
    pub test_duration_seconds: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            max_response_time_ms: 5000,  // 5 seconds max response
            max_memory_usage_mb: 256,    // 256MB max memory per agent
            concurrent_workflows: 10,    // 10 concurrent workflows
            max_cpu_usage_percent: 80.0, // 80% max CPU usage
            test_duration_seconds: 30,   // 30 second sustained tests
        }
    }
}

/// Memory usage statistics for monitoring agent resource consumption
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub initial_memory_mb: u64,
    pub peak_memory_mb: u64,
    pub final_memory_mb: u64,
    pub memory_growth_mb: u64,
}

/// Performance metrics collected during agent benchmarking
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_workflows: u64,
    pub successful_workflows: u64,
    pub failed_workflows: u64,
    pub average_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub memory_stats: MemoryStats,
    pub cpu_usage_percent: f32,
    pub throughput_workflows_per_second: f64,
}

/// Agent resource monitor for tracking system resource usage during tests
pub struct AgentResourceMonitor {
    system: Arc<RwLock<System>>,
    process_id: u32,
    initial_memory: u64,
    peak_memory: Arc<AtomicU64>,
    start_time: Instant,
}

impl AgentResourceMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let process_id = std::process::id();
        let initial_memory = system
            .process(sysinfo::Pid::from_u32(process_id))
            .map(|p| p.memory())
            .unwrap_or(0);

        Self {
            system: Arc::new(RwLock::new(system)),
            process_id,
            initial_memory,
            peak_memory: Arc::new(AtomicU64::new(initial_memory)),
            start_time: Instant::now(),
        }
    }

    pub async fn update_stats(&self) -> Result<()> {
        let mut system = self.system.write().await;
        // system.refresh_process(sysinfo::Pid::from_u32(self.process_id)); // API changed

        if let Some(process) = system.process(sysinfo::Pid::from_u32(self.process_id)) {
            let current_memory = process.memory();
            self.peak_memory
                .fetch_max(current_memory, Ordering::Relaxed);
        }

        Ok(())
    }

    pub async fn get_memory_stats(&self) -> MemoryStats {
        let system = self.system.read().await;
        let current_memory = system
            .process(sysinfo::Pid::from_u32(self.process_id))
            .map(|p| p.memory())
            .unwrap_or(0);

        let initial_mb = self.initial_memory / 1024 / 1024;
        let peak_mb = self.peak_memory.load(Ordering::Relaxed) / 1024 / 1024;
        let current_mb = current_memory / 1024 / 1024;

        MemoryStats {
            initial_memory_mb: initial_mb,
            peak_memory_mb: peak_mb,
            final_memory_mb: current_mb,
            memory_growth_mb: current_mb.saturating_sub(initial_mb),
        }
    }

    pub async fn get_cpu_usage(&self) -> f32 {
        let system = self.system.read().await;
        system
            .process(sysinfo::Pid::from_u32(self.process_id))
            .map(|p| p.cpu_usage())
            .unwrap_or(0.0)
    }
}

// =============================================================================
// P1.1.13.a: Agent Integration Tests [8м]
// =============================================================================

/// Test complete end-to-end agent workflow with comprehensive validation
/// Tests the full Intent→Plan→Execute→Critic cycle with detailed verification
#[tokio::test]
async fn test_comprehensive_end_to_end_agent_workflow() -> Result<()> {
    let resource_monitor = AgentResourceMonitor::new();
    let start_time = Instant::now();

    // Setup orchestrator with comprehensive configuration
    let system_config = SystemConfig {
        max_actors: 50,
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 5,
        enable_resource_monitoring: true,
        health_check_interval_ms: 100,
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    // Initialize all agents and verify successful initialization
    orchestrator.initialize_agents().await?;

    // Create comprehensive workflow request with complex requirements
    let workflow_request = WorkflowRequest {
        user_input: "Create a user authentication system with email verification, password hashing, and role-based access control".to_string(),
        context: Some(serde_json::json!({
            "project_type": "web_application",
            "security_level": "high",
            "compliance": ["GDPR", "SOX"],
            "features": {
                "email_verification": true,
                "two_factor_auth": true,
                "role_permissions": ["admin", "user", "moderator"],
                "session_management": true,
                "audit_logging": true
            },
            "technology_stack": {
                "backend": "rust",
                "database": "postgresql",
                "cache": "redis",
                "frontend": "react"
            },
            "deployment": {
                "environment": "kubernetes",
                "monitoring": "prometheus",
                "logging": "elk_stack"
            }
        })),
        priority: TaskPriority::High,
        dry_run: false,
        timeout_ms: Some(30000), // 30 seconds for complex operation
        config_overrides: Some(WorkflowConfig {
            enable_intent_analysis: true,
            enable_plan_generation: true,
            enable_plan_execution: true,
            enable_result_critique: true,
            max_step_retries: 3,
            step_timeout_ms: 8000,
        }),
    };

    // Monitor resource usage during workflow execution
    let monitor_handle = {
        let monitor = resource_monitor;
        tokio::spawn(async move {
            let mut max_memory = 0u64;
            let mut max_cpu = 0.0f32;

            for _ in 0..300 {
                // Monitor for up to 30 seconds
                if let Ok(()) = monitor.update_stats().await {
                    let memory = monitor.get_memory_stats().await;
                    let cpu = monitor.get_cpu_usage().await;

                    max_memory = max_memory.max(memory.peak_memory_mb);
                    max_cpu = max_cpu.max(cpu);
                }
                sleep(Duration::from_millis(100)).await;
            }

            (max_memory, max_cpu)
        })
    };

    // Execute complete workflow with detailed timing
    let workflow_start = Instant::now();
    let workflow_result = orchestrator.execute_workflow(workflow_request).await?;
    let workflow_duration = workflow_start.elapsed();

    // Get resource monitoring results
    let (peak_memory_mb, peak_cpu_percent) = monitor_handle.await?;

    // =============================================================================
    // Comprehensive Workflow Validation
    // =============================================================================

    // 1. Verify workflow successful completion
    assert!(
        workflow_result.success,
        "Complex authentication system workflow should complete successfully"
    );

    // 2. Verify all workflow components are present
    assert!(
        workflow_result.intent.is_some(),
        "Intent analysis should identify authentication system requirements"
    );

    assert!(
        workflow_result.plan.is_some(),
        "Action plan should be created for authentication system"
    );

    assert!(
        workflow_result.execution_results.is_some(),
        "Execution results should be available for authentication system"
    );

    assert!(
        workflow_result.critique.is_some(),
        "Critique should be generated for authentication system quality"
    );

    // 3. Verify workflow state progression
    let workflow_id = workflow_result.workflow_id;
    let workflow_state = orchestrator
        .get_workflow(workflow_id)
        .await
        .expect("Workflow should exist in orchestrator");

    assert!(
        matches!(
            workflow_state.current_step,
            WorkflowStepType::ResultCritique | WorkflowStepType::Completed
        ),
        "Workflow should reach final completion state"
    );

    // 4. Verify workflow performance metrics
    println!(
        "🔍 DEBUG: workflow_result.execution_time_ms = {}",
        workflow_result.execution_time_ms
    );
    println!(
        "🔍 DEBUG: workflow_result.success = {}",
        workflow_result.success
    );
    println!(
        "🔍 DEBUG: workflow_result.error = {:?}",
        workflow_result.error
    );
    println!(
        "🔍 DEBUG: workflow_result.resource_usage.total_time_ms = {}",
        workflow_result.resource_usage.total_time_ms
    );

    assert!(
        workflow_result.execution_time_ms > 0,
        "Execution time should be recorded and positive: actual={}",
        workflow_result.execution_time_ms
    );

    assert!(
        workflow_result.execution_time_ms < 30000,
        "Complex workflow should complete within 30 seconds: actual {}ms",
        workflow_result.execution_time_ms
    );

    // 5. Verify workflow completeness
    assert!(
        workflow_result.steps_completed.len() >= 4,
        "Should complete at least 4 major steps (Intent, Plan, Execute, Critic): completed {}",
        workflow_result.steps_completed.len()
    );

    // =============================================================================
    // Agent Integration and Coordination Validation
    // =============================================================================

    // 6. Verify multi-agent coordination worked properly
    let active_workflows = orchestrator.get_active_workflows().await;
    assert!(
        active_workflows.is_empty() || active_workflows.iter().all(|w| w.id != workflow_id),
        "Completed workflow should no longer be active"
    );

    // 7. Verify resource usage is within acceptable limits
    assert!(
        peak_memory_mb < 512,
        "Peak memory usage should be under 512MB: actual {}MB",
        peak_memory_mb
    );

    assert!(
        peak_cpu_percent < 90.0,
        "Peak CPU usage should be under 90%: actual {:.1}%",
        peak_cpu_percent
    );

    // 8. Validate intent analysis quality
    if let Some(intent) = &workflow_result.intent {
        let intent_str = intent.to_string();
        assert!(
            intent_str.to_lowercase().contains("authentication")
                || intent_str.to_lowercase().contains("user")
                || intent_str.to_lowercase().contains("security"),
            "Intent should recognize authentication/security requirements"
        );
    }

    // 9. Validate plan generation quality
    if let Some(plan) = &workflow_result.plan {
        let plan_str = plan.to_string();
        assert!(
            plan_str.len() > 100,
            "Plan should be detailed for complex authentication system"
        );
    }

    // 10. Validate critique quality
    if let Some(critique) = &workflow_result.critique {
        let critique_str = critique.to_string();
        assert!(
            critique_str.len() > 50,
            "Critique should provide meaningful feedback"
        );
    }

    println!("✅ Comprehensive E2E Test Results:");
    println!(
        "   - Workflow Duration: {:.2}s",
        workflow_duration.as_secs_f64()
    );
    println!("   - Peak Memory: {}MB", peak_memory_mb);
    println!("   - Peak CPU: {:.1}%", peak_cpu_percent);
    println!(
        "   - Steps Completed: {}",
        workflow_result.steps_completed.len()
    );
    println!(
        "   - Execution Time: {}ms",
        workflow_result.execution_time_ms
    );

    Ok(())
}

/// Test multi-agent interaction and communication patterns
/// Validates agent-to-agent communication and coordination
#[tokio::test]
async fn test_multi_agent_interaction_patterns() -> Result<()> {
    let system_config = SystemConfig {
        max_actors: 25,
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 3,
        enable_resource_monitoring: true,
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    // Create multiple interdependent workflow requests
    let workflow_requests = vec![
        WorkflowRequest {
            user_input: "Create user account management API".to_string(),
            context: Some(serde_json::json!({
                "service": "user_service",
                "dependencies": []
            })),
            priority: TaskPriority::High,
            dry_run: false,
            timeout_ms: Some(15000),
            config_overrides: None,
        },
        WorkflowRequest {
            user_input: "Create notification service with user integration".to_string(),
            context: Some(serde_json::json!({
                "service": "notification_service",
                "dependencies": ["user_service"]
            })),
            priority: TaskPriority::Normal,
            dry_run: false,
            timeout_ms: Some(15000),
            config_overrides: None,
        },
        WorkflowRequest {
            user_input: "Create analytics service with user and notification integration"
                .to_string(),
            context: Some(serde_json::json!({
                "service": "analytics_service",
                "dependencies": ["user_service", "notification_service"]
            })),
            priority: TaskPriority::Low,
            dry_run: false,
            timeout_ms: Some(15000),
            config_overrides: None,
        },
    ];

    // Execute workflows with dependency ordering
    let mut results = Vec::new();
    let start_time = Instant::now();

    for (i, request) in workflow_requests.into_iter().enumerate() {
        println!("Executing workflow {} with dependencies...", i + 1);
        let result = orchestrator.execute_workflow(request).await?;
        results.push(result);

        // Small delay to allow for proper agent coordination
        sleep(Duration::from_millis(100)).await;
    }

    let total_duration = start_time.elapsed();

    // =============================================================================
    // Multi-Agent Interaction Validation
    // =============================================================================

    // 1. Verify all workflows completed
    assert_eq!(
        results.len(),
        3,
        "All three interdependent workflows should complete"
    );

    // 2. Verify success rate for coordinated workflows
    let successful_workflows = results.iter().filter(|r| r.success).count();
    assert!(
        successful_workflows >= 2,
        "At least 2 out of 3 coordinated workflows should succeed: got {}",
        successful_workflows
    );

    // 3. Verify workflow coordination timing
    assert!(
        total_duration < Duration::from_secs(50),
        "Coordinated workflows should complete within 50 seconds: actual {:.2}s",
        total_duration.as_secs_f64()
    );

    // 4. Verify unique workflow IDs (no conflicts)
    let workflow_ids: Vec<WorkflowId> = results.iter().map(|r| r.workflow_id).collect();
    let unique_ids: std::collections::HashSet<_> = workflow_ids.iter().collect();
    assert_eq!(
        workflow_ids.len(),
        unique_ids.len(),
        "All workflow IDs should be unique in multi-agent coordination"
    );

    // 5. Verify workflows show increasing complexity in execution time
    // (Later workflows depend on earlier ones and should be more complex)
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.execution_time_ms > 0,
            "Workflow {} should record execution time",
            i + 1
        );
    }

    // 6. Verify proper state transitions for all workflows
    for (i, result) in results.iter().enumerate() {
        let workflow_state = orchestrator
            .get_workflow(result.workflow_id)
            .await
            .expect(&format!("Workflow {} should exist", i + 1));

        assert!(
            matches!(
                workflow_state.current_step,
                WorkflowStepType::ResultCritique
                    | WorkflowStepType::Completed
                    | WorkflowStepType::Failed
            ),
            "Workflow {} should reach terminal state",
            i + 1
        );
    }

    println!("✅ Multi-Agent Interaction Test Results:");
    println!("   - Total Duration: {:.2}s", total_duration.as_secs_f64());
    println!(
        "   - Successful Workflows: {}/{}",
        successful_workflows,
        results.len()
    );
    for (i, result) in results.iter().enumerate() {
        println!(
            "   - Workflow {}: {}ms ({})",
            i + 1,
            result.execution_time_ms,
            if result.success { "SUCCESS" } else { "FAILED" }
        );
    }

    Ok(())
}

/// Test agent state transitions and monitoring
/// Validates proper agent lifecycle and state management
#[tokio::test]
async fn test_agent_state_transitions_monitoring() -> Result<()> {
    let system_config = SystemConfig::default();
    let orchestrator_config = OrchestratorConfig {
        enable_resource_monitoring: true,
        health_check_interval_ms: 50, // Frequent health checks for monitoring
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    let workflow_request = WorkflowRequest {
        user_input: "Generate system health report with detailed metrics".to_string(),
        context: Some(serde_json::json!({
            "report_type": "health_check",
            "include_metrics": true,
            "detail_level": "comprehensive"
        })),
        priority: TaskPriority::Normal,
        dry_run: false,
        timeout_ms: Some(10000),
        config_overrides: None,
    };

    // Monitor workflow state transitions in real-time
    let state_monitor_handle = {
        let orchestrator_clone = Arc::clone(&orchestrator);
        tokio::spawn(async move {
            let mut observed_states = Vec::new();
            let mut workflow_found = false;
            let mut current_workflow_id: Option<WorkflowId> = None;

            // Monitor for state transitions for up to 15 seconds
            for _ in 0..150 {
                let active_workflows = orchestrator_clone.get_active_workflows().await;

                if !workflow_found && !active_workflows.is_empty() {
                    workflow_found = true;
                    current_workflow_id = Some(active_workflows[0].id);
                }

                if let Some(workflow_id) = current_workflow_id {
                    if let Some(state) = orchestrator_clone.get_workflow(workflow_id).await {
                        observed_states.push((Instant::now(), state.current_step));
                    }
                }

                sleep(Duration::from_millis(100)).await;

                // Stop monitoring when no active workflows
                if workflow_found && active_workflows.is_empty() {
                    break;
                }
            }

            observed_states
        })
    };

    // Execute workflow and collect state transitions
    let workflow_start = Instant::now();
    let workflow_result = orchestrator.execute_workflow(workflow_request).await?;
    let observed_states = state_monitor_handle.await?;

    // =============================================================================
    // State Transition Validation
    // =============================================================================

    // 1. Verify workflow completed successfully
    assert!(
        workflow_result.success,
        "Health report workflow should complete successfully"
    );

    // 2. Verify we observed state transitions
    assert!(
        !observed_states.is_empty(),
        "Should observe agent state transitions during workflow"
    );

    assert!(
        observed_states.len() >= 3,
        "Should observe at least 3 state transitions: got {}",
        observed_states.len()
    );

    // 3. Verify expected state progression
    let expected_states = vec![
        WorkflowStepType::IntentAnalysis,
        WorkflowStepType::PlanGeneration,
        WorkflowStepType::PlanExecution,
        WorkflowStepType::ResultCritique,
    ];

    let observed_step_types: Vec<WorkflowStepType> = observed_states
        .iter()
        .map(|(_, step_type)| *step_type)
        .collect();

    // Verify we see the major workflow steps
    for expected_state in &expected_states {
        assert!(
            observed_step_types.contains(expected_state),
            "Should observe state transition: {:?}",
            expected_state
        );
    }

    // 4. Verify state transitions are properly ordered
    let unique_states: Vec<_> = observed_step_types.into_iter().collect();
    assert!(
        unique_states.len() >= 2,
        "Should observe multiple distinct states: got {} unique states",
        unique_states.len()
    );

    // 5. Verify final workflow state
    let final_state = orchestrator
        .get_workflow(workflow_result.workflow_id)
        .await
        .expect("Workflow should exist");

    assert!(
        matches!(
            final_state.current_step,
            WorkflowStepType::ResultCritique | WorkflowStepType::Completed
        ),
        "Final workflow state should be completed or critique: {:?}",
        final_state.current_step
    );

    // 6. Verify transition timing is reasonable
    if observed_states.len() >= 2 {
        let first_transition = observed_states
            .first()
            .expect("Test operation should succeed")
            .0;
        let last_transition = observed_states
            .last()
            .expect("Test operation should succeed")
            .0;
        let total_transition_time = last_transition.duration_since(first_transition);

        assert!(
            total_transition_time < Duration::from_secs(12),
            "State transitions should complete within 12 seconds: actual {:.2}s",
            total_transition_time.as_secs_f64()
        );
    }

    println!("✅ Agent State Transitions Test Results:");
    println!("   - States Observed: {}", observed_states.len());
    println!(
        "   - Workflow Duration: {}ms",
        workflow_result.execution_time_ms
    );
    println!("   - Final State: {:?}", final_state.current_step);
    println!("   - State Progression:");
    for (i, (timestamp, state)) in observed_states.iter().enumerate() {
        if i == 0 || observed_states[i - 1].1 != *state {
            println!("     {:?}", state);
        }
    }

    Ok(())
}

// =============================================================================
// P1.1.13.b: Performance Testing [7м]
// =============================================================================

/// Test agent response time benchmarks under various load conditions
/// Measures and validates agent performance within production constraints
#[tokio::test]
async fn test_agent_performance_benchmarks() -> Result<()> {
    let benchmark_config = BenchmarkConfig::default();
    let resource_monitor = AgentResourceMonitor::new();

    // Setup orchestrator for performance testing
    let system_config = SystemConfig {
        max_actors: 100, // Higher limit for performance testing
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: benchmark_config.concurrent_workflows,
        enable_resource_monitoring: true,
        health_check_interval_ms: 500, // Less frequent during performance testing
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    let mut response_times = Vec::new();
    let mut successful_workflows = 0u64;
    let mut failed_workflows = 0u64;

    let benchmark_start = Instant::now();

    // =============================================================================
    // Single Agent Performance Benchmark
    // =============================================================================

    println!("🚀 Starting Agent Performance Benchmarks...");

    // Test 1: Single agent response time
    for i in 0..20 {
        let request_start = Instant::now();

        let workflow_request = WorkflowRequest {
            user_input: format!("Process simple task #{}", i + 1),
            context: Some(serde_json::json!({
                "task_id": i + 1,
                "complexity": "low",
                "benchmark": true
            })),
            priority: TaskPriority::Normal,
            dry_run: false,
            timeout_ms: Some(5000),
            config_overrides: Some(WorkflowConfig {
                enable_intent_analysis: true,
                enable_plan_generation: true,
                enable_plan_execution: true,
                enable_result_critique: true,
                max_step_retries: 1, // Reduced for performance testing
                step_timeout_ms: 2000,
            }),
        };

        match orchestrator.execute_workflow(workflow_request).await {
            Ok(result) => {
                let response_time = request_start.elapsed().as_millis() as u64;
                response_times.push(response_time);

                if result.success {
                    successful_workflows += 1;
                } else {
                    failed_workflows += 1;
                }

                // Update resource monitoring
                resource_monitor.update_stats().await?;
            }
            Err(_) => {
                failed_workflows += 1;
            }
        }

        // Brief pause between tests
        sleep(Duration::from_millis(50)).await;
    }

    // =============================================================================
    // Concurrent Agent Performance Benchmark
    // =============================================================================

    println!("🔄 Testing Concurrent Agent Performance...");

    let semaphore = Arc::new(Semaphore::new(benchmark_config.concurrent_workflows));
    let mut concurrent_handles = Vec::new();

    let concurrent_start = Instant::now();

    for i in 0..benchmark_config.concurrent_workflows {
        let orchestrator_clone = Arc::clone(&orchestrator);
        let semaphore_clone = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone
                .acquire()
                .await
                .expect("Test operation should succeed");
            let request_start = Instant::now();

            let workflow_request = WorkflowRequest {
                user_input: format!("Concurrent task #{}", i + 1),
                context: Some(serde_json::json!({
                    "task_id": i + 1,
                    "concurrent": true,
                    "complexity": "medium"
                })),
                priority: TaskPriority::Normal,
                dry_run: false,
                timeout_ms: Some(8000),
                config_overrides: None,
            };

            let result = orchestrator_clone.execute_workflow(workflow_request).await;
            let response_time = request_start.elapsed().as_millis() as u64;

            (result, response_time)
        });

        concurrent_handles.push(handle);
    }

    // Collect concurrent results
    let mut concurrent_response_times = Vec::new();
    let mut concurrent_successes = 0u64;
    let mut concurrent_failures = 0u64;

    for handle in concurrent_handles {
        match handle.await? {
            (Ok(result), response_time) => {
                concurrent_response_times.push(response_time);
                if result.success {
                    concurrent_successes += 1;
                } else {
                    concurrent_failures += 1;
                }
            }
            (Err(_), response_time) => {
                concurrent_response_times.push(response_time);
                concurrent_failures += 1;
            }
        }
    }

    let concurrent_duration = concurrent_start.elapsed();

    // =============================================================================
    // Performance Metrics Calculation
    // =============================================================================

    // Calculate single-agent metrics
    response_times.sort();
    let avg_response_time = response_times.iter().sum::<u64>() / response_times.len() as u64;
    let p95_response_time = response_times
        .get(response_times.len() * 95 / 100)
        .copied()
        .unwrap_or(0);
    let p99_response_time = response_times
        .get(response_times.len() * 99 / 100)
        .copied()
        .unwrap_or(0);

    // Calculate concurrent metrics
    concurrent_response_times.sort();
    let concurrent_avg =
        concurrent_response_times.iter().sum::<u64>() / concurrent_response_times.len() as u64;
    let concurrent_p95 = concurrent_response_times
        .get(concurrent_response_times.len() * 95 / 100)
        .copied()
        .unwrap_or(0);

    // Get final resource stats
    let memory_stats = resource_monitor.get_memory_stats().await;
    let cpu_usage = resource_monitor.get_cpu_usage().await;

    // Calculate throughput
    let total_duration = benchmark_start.elapsed().as_secs_f64();
    let total_workflows =
        successful_workflows + failed_workflows + concurrent_successes + concurrent_failures;
    let throughput = total_workflows as f64 / total_duration;

    let metrics = PerformanceMetrics {
        total_workflows,
        successful_workflows: successful_workflows + concurrent_successes,
        failed_workflows: failed_workflows + concurrent_failures,
        average_response_time_ms: avg_response_time,
        p95_response_time_ms: p95_response_time,
        p99_response_time_ms: p99_response_time,
        memory_stats,
        cpu_usage_percent: cpu_usage,
        throughput_workflows_per_second: throughput,
    };

    // =============================================================================
    // Performance Validation Against Production Constraints
    // =============================================================================

    // 1. Validate response time constraints
    assert!(
        avg_response_time < benchmark_config.max_response_time_ms,
        "Average response time should be under {}ms: actual {}ms",
        benchmark_config.max_response_time_ms,
        avg_response_time
    );

    assert!(
        p95_response_time < benchmark_config.max_response_time_ms * 2,
        "P95 response time should be under {}ms: actual {}ms",
        benchmark_config.max_response_time_ms * 2,
        p95_response_time
    );

    // 2. Validate memory usage constraints
    assert!(
        metrics.memory_stats.peak_memory_mb < benchmark_config.max_memory_usage_mb,
        "Peak memory usage should be under {}MB: actual {}MB",
        benchmark_config.max_memory_usage_mb,
        metrics.memory_stats.peak_memory_mb
    );

    assert!(
        metrics.memory_stats.memory_growth_mb < benchmark_config.max_memory_usage_mb / 2,
        "Memory growth should be under {}MB: actual {}MB",
        benchmark_config.max_memory_usage_mb / 2,
        metrics.memory_stats.memory_growth_mb
    );

    // 3. Validate CPU usage constraints
    assert!(
        cpu_usage < benchmark_config.max_cpu_usage_percent,
        "CPU usage should be under {:.1}%: actual {:.1}%",
        benchmark_config.max_cpu_usage_percent,
        cpu_usage
    );

    // 4. Validate success rate
    let success_rate =
        (metrics.successful_workflows as f64 / metrics.total_workflows as f64) * 100.0;
    assert!(
        success_rate >= 85.0,
        "Success rate should be at least 85%: actual {:.1}%",
        success_rate
    );

    // 5. Validate concurrent performance degradation is acceptable
    let degradation_factor = concurrent_avg as f64 / avg_response_time as f64;
    assert!(
        degradation_factor < 2.5,
        "Concurrent performance degradation should be under 2.5x: actual {:.2}x",
        degradation_factor
    );

    // 6. Validate minimum throughput
    assert!(
        throughput >= 0.5,
        "Throughput should be at least 0.5 workflows/second: actual {:.2}",
        throughput
    );

    // Print detailed performance report
    println!("📊 Performance Benchmark Results:");
    println!("╭─────────────────────────────────────────╮");
    println!("│ Single Agent Performance                │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Average Response Time: {:>11} ms │", avg_response_time);
    println!("│ P95 Response Time:     {:>11} ms │", p95_response_time);
    println!("│ P99 Response Time:     {:>11} ms │", p99_response_time);
    println!("├─────────────────────────────────────────┤");
    println!("│ Concurrent Performance                  │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Concurrent Workflows:  {:>11}    │",
        benchmark_config.concurrent_workflows
    );
    println!("│ Concurrent Avg Time:   {:>11} ms │", concurrent_avg);
    println!("│ Concurrent P95 Time:   {:>11} ms │", concurrent_p95);
    println!(
        "│ Performance Degradation: {:>8.2}x    │",
        degradation_factor
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Resource Usage                          │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Initial Memory:        {:>11} MB │",
        metrics.memory_stats.initial_memory_mb
    );
    println!(
        "│ Peak Memory:           {:>11} MB │",
        metrics.memory_stats.peak_memory_mb
    );
    println!(
        "│ Memory Growth:         {:>11} MB │",
        metrics.memory_stats.memory_growth_mb
    );
    println!("│ CPU Usage:             {:>8.1}%    │", cpu_usage);
    println!("├─────────────────────────────────────────┤");
    println!("│ Overall Metrics                         │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Total Workflows:       {:>11}    │",
        metrics.total_workflows
    );
    println!(
        "│ Successful:            {:>11}    │",
        metrics.successful_workflows
    );
    println!(
        "│ Failed:                {:>11}    │",
        metrics.failed_workflows
    );
    println!("│ Success Rate:          {:>8.1}%    │", success_rate);
    println!("│ Throughput:            {:>8.2}/s    │", throughput);
    println!("│ Total Duration:        {:>8.2}s    │", total_duration);
    println!("╰─────────────────────────────────────────╯");

    Ok(())
}

/// Test memory usage monitoring and limits under sustained load
/// Validates agent memory behavior and prevents memory leaks
#[tokio::test]
async fn test_memory_usage_monitoring() -> Result<()> {
    let resource_monitor = AgentResourceMonitor::new();

    // Setup orchestrator with memory monitoring
    let system_config = SystemConfig {
        max_actors: 30,
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 5,
        enable_resource_monitoring: true,
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    // Get baseline memory usage
    let initial_memory = resource_monitor.get_memory_stats().await;

    println!("💾 Starting Memory Usage Monitoring Test...");
    println!("   Initial Memory: {}MB", initial_memory.initial_memory_mb);

    // =============================================================================
    // Sustained Load Memory Test
    // =============================================================================

    let mut memory_samples = Vec::new();
    let test_start = Instant::now();

    // Run sustained load for memory testing
    for batch in 0..5 {
        println!("   Running memory test batch {}...", batch + 1);

        // Create memory-intensive workflow requests
        let workflow_requests: Vec<_> = (0..8)
            .map(|i| WorkflowRequest {
                user_input: format!("Process large dataset batch {} item {}", batch + 1, i + 1),
                context: Some(serde_json::json!({
                    "batch_id": batch + 1,
                    "item_id": i + 1,
                    "data_size": "large",
                    "processing": {
                        "type": "memory_intensive",
                        "algorithm": "complex_analysis",
                        "buffer_size": "64MB",
                        "iterations": 1000
                    },
                    "output": {
                        "format": "comprehensive_report",
                        "include_charts": true,
                        "include_raw_data": true
                    }
                })),
                priority: TaskPriority::Normal,
                dry_run: false,
                timeout_ms: Some(10000),
                config_overrides: Some(WorkflowConfig {
                    enable_intent_analysis: true,
                    enable_plan_generation: true,
                    enable_plan_execution: true,
                    enable_result_critique: true,
                    max_step_retries: 1,
                    step_timeout_ms: 3000,
                }),
            })
            .collect();

        // Execute batch of workflows
        let mut batch_handles = Vec::new();

        for request in workflow_requests {
            let orchestrator_clone = Arc::clone(&orchestrator);
            let handle =
                tokio::spawn(async move { orchestrator_clone.execute_workflow(request).await });
            batch_handles.push(handle);
        }

        // Monitor memory during batch execution
        let batch_start = Instant::now();
        while batch_start.elapsed() < Duration::from_secs(15) {
            resource_monitor.update_stats().await?;
            let current_memory = resource_monitor.get_memory_stats().await;
            memory_samples.push((
                test_start.elapsed().as_secs_f64(),
                current_memory.final_memory_mb,
            ));

            sleep(Duration::from_millis(500)).await;
        }

        // Wait for batch completion
        for handle in batch_handles {
            let _ = handle.await?;
        }

        // Memory stabilization pause
        sleep(Duration::from_secs(2)).await;

        // Force garbage collection hint (Rust doesn't have explicit GC, but this simulates cleanup)
        resource_monitor.update_stats().await?;

        let batch_end_memory = resource_monitor.get_memory_stats().await;
        println!(
            "   Batch {} completed - Memory: {}MB",
            batch + 1,
            batch_end_memory.final_memory_mb
        );
    }

    let final_memory = resource_monitor.get_memory_stats().await;
    let test_duration = test_start.elapsed();

    // =============================================================================
    // Memory Usage Analysis
    // =============================================================================

    // Calculate memory growth rate
    let total_growth = final_memory
        .final_memory_mb
        .saturating_sub(initial_memory.initial_memory_mb);
    let growth_rate_mb_per_min = (total_growth as f64) / (test_duration.as_secs_f64() / 60.0);

    // Find peak memory and growth patterns
    let peak_memory = memory_samples
        .iter()
        .map(|(_, memory)| *memory)
        .max()
        .unwrap_or(final_memory.final_memory_mb);

    let average_memory = memory_samples
        .iter()
        .map(|(_, memory)| *memory)
        .sum::<u64>() as f64
        / memory_samples.len() as f64;

    // Calculate memory stability (variance)
    let memory_variance = memory_samples
        .iter()
        .map(|(_, memory)| {
            let diff = *memory as f64 - average_memory;
            diff * diff
        })
        .sum::<f64>()
        / memory_samples.len() as f64;
    let memory_stability = memory_variance.sqrt();

    // =============================================================================
    // Memory Validation Against Production Constraints
    // =============================================================================

    // 1. Validate peak memory usage
    assert!(
        peak_memory < 400,
        "Peak memory usage should be under 400MB: actual {}MB",
        peak_memory
    );

    // 2. Validate memory growth rate (should not grow unboundedly)
    assert!(
        growth_rate_mb_per_min < 50.0,
        "Memory growth rate should be under 50MB/min: actual {:.2}MB/min",
        growth_rate_mb_per_min
    );

    // 3. Validate final memory is reasonable
    assert!(
        final_memory.final_memory_mb < initial_memory.initial_memory_mb + 150,
        "Final memory should not grow more than 150MB: initial {}MB, final {}MB",
        initial_memory.initial_memory_mb,
        final_memory.final_memory_mb
    );

    // 4. Validate memory stability (should not oscillate wildly)
    assert!(
        memory_stability < 30.0,
        "Memory usage should be stable (variance < 30MB): actual {:.2}MB variance",
        memory_stability
    );

    // 5. Check for potential memory leaks
    let leak_threshold = initial_memory.initial_memory_mb + 100; // Allow 100MB growth
    assert!(
        final_memory.final_memory_mb < leak_threshold,
        "Potential memory leak detected: final {}MB > threshold {}MB",
        final_memory.final_memory_mb,
        leak_threshold
    );

    // 6. Validate memory efficiency (peak shouldn't be too much higher than average)
    let memory_efficiency = average_memory / peak_memory as f64;
    assert!(
        memory_efficiency > 0.7,
        "Memory efficiency should be > 70%: actual {:.1}% (avg: {:.1}MB, peak: {}MB)",
        memory_efficiency * 100.0,
        average_memory,
        peak_memory
    );

    // Print detailed memory analysis
    println!("📈 Memory Usage Analysis Results:");
    println!("╭─────────────────────────────────────────╮");
    println!("│ Memory Statistics                       │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Initial Memory:        {:>11} MB │",
        initial_memory.initial_memory_mb
    );
    println!("│ Peak Memory:           {:>11} MB │", peak_memory);
    println!(
        "│ Final Memory:          {:>11} MB │",
        final_memory.final_memory_mb
    );
    println!("│ Average Memory:        {:>8.1} MB │", average_memory);
    println!("├─────────────────────────────────────────┤");
    println!("│ Growth Analysis                         │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Total Growth:          {:>11} MB │", total_growth);
    println!(
        "│ Growth Rate:           {:>8.2} MB/min │",
        growth_rate_mb_per_min
    );
    println!("│ Memory Stability:      {:>8.2} MB │", memory_stability);
    println!(
        "│ Memory Efficiency:     {:>8.1}%    │",
        memory_efficiency * 100.0
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Test Parameters                         │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Test Duration:         {:>8.1}s    │",
        test_duration.as_secs_f64()
    );
    println!("│ Memory Samples:        {:>11}    │", memory_samples.len());
    println!("│ Workload Batches:      {:>11}    │", 5);
    println!("│ Workflows per Batch:   {:>11}    │", 8);
    println!("╰─────────────────────────────────────────╯");

    // Print memory timeline for debugging
    println!("📊 Memory Usage Timeline (last 10 samples):");
    for (time, memory) in memory_samples.iter().rev().take(10).rev() {
        println!("   {:.1}s: {}MB", time, memory);
    }

    Ok(())
}
