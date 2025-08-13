#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_attributes)]
#![allow(clippy::empty_line_after_outer_attr)]
#![allow(clippy::new_without_default)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::manual_range_contains)]
//! Performance Stress Tests for Multi-Agent Orchestration
//!
//! Advanced performance and stress testing for the Multi-Agent system:
//! - High-load concurrent agent execution
//! - Memory pressure testing under extreme load
//! - Resource exhaustion and recovery testing
//! - Long-running stability tests
//! - Performance regression detection

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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{ProcessesToUpdate, System};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::{interval, sleep, timeout};
use uuid::Uuid;

/// Stress test configuration for extreme load testing
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Number of concurrent workflows for stress testing
    pub max_concurrent_workflows: usize,
    /// Total number of workflows to execute
    pub total_workflows: usize,
    /// Duration for sustained load testing (seconds)
    pub test_duration_seconds: u64,
    /// Memory pressure threshold (MB)
    pub memory_pressure_threshold_mb: u64,
    /// CPU pressure threshold (%)
    pub cpu_pressure_threshold_percent: f32,
    /// Maximum acceptable failure rate (%)
    pub max_failure_rate_percent: f64,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            max_concurrent_workflows: 25,
            total_workflows: 100,
            test_duration_seconds: 60,
            memory_pressure_threshold_mb: 512,
            cpu_pressure_threshold_percent: 85.0,
            max_failure_rate_percent: 15.0,
        }
    }
}

/// Advanced performance metrics with detailed statistics
#[derive(Debug, Clone)]
pub struct AdvancedPerformanceMetrics {
    pub workflows_completed: u64,
    pub workflows_failed: u64,
    pub workflows_timeout: u64,
    pub total_execution_time_ms: u64,
    pub min_response_time_ms: u64,
    pub max_response_time_ms: u64,
    pub median_response_time_ms: u64,
    pub p90_response_time_ms: u64,
    pub p95_response_time_ms: u64,
    pub p99_response_time_ms: u64,
    pub throughput_per_second: f64,
    pub memory_usage_mb: MemoryUsageMetrics,
    pub cpu_usage_percent: CpuUsageMetrics,
    pub error_distribution: HashMap<String, u64>,
}

/// Detailed memory usage metrics
#[derive(Debug, Clone)]
pub struct MemoryUsageMetrics {
    pub initial_mb: u64,
    pub peak_mb: u64,
    pub average_mb: f64,
    pub final_mb: u64,
    pub growth_rate_mb_per_min: f64,
    pub stability_variance: f64,
}

/// CPU usage metrics over time
#[derive(Debug, Clone)]
pub struct CpuUsageMetrics {
    pub average_percent: f32,
    pub peak_percent: f32,
    pub sustained_high_load_duration_sec: f64,
}

/// System resource monitor for comprehensive resource tracking
pub struct SystemResourceMonitor {
    system: Arc<Mutex<System>>,
    process_id: u32,
    monitoring_active: Arc<AtomicBool>,
    memory_samples: Arc<Mutex<Vec<(f64, u64)>>>, // (timestamp, memory_mb)
    cpu_samples: Arc<Mutex<Vec<(f64, f32)>>>,    // (timestamp, cpu_percent)
    start_time: Instant,
}

impl SystemResourceMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Arc::new(Mutex::new(system)),
            process_id: std::process::id(),
            monitoring_active: Arc::new(AtomicBool::new(false)),
            memory_samples: Arc::new(Mutex::new(Vec::new())),
            cpu_samples: Arc::new(Mutex::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    pub async fn start_monitoring(&self) {
        self.monitoring_active.store(true, Ordering::Relaxed);

        let system = Arc::clone(&self.system);
        let process_id = self.process_id;
        let monitoring_active = Arc::clone(&self.monitoring_active);
        let memory_samples = Arc::clone(&self.memory_samples);
        let cpu_samples = Arc::clone(&self.cpu_samples);
        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(200)); // 5 Hz sampling

            while monitoring_active.load(Ordering::Relaxed) {
                interval.tick().await;

                let mut sys = system.lock().await;
                sys.refresh_processes(ProcessesToUpdate::All);

                if let Some(process) = sys.process(sysinfo::Pid::from_u32(process_id)) {
                    let timestamp = start_time.elapsed().as_secs_f64();
                    let memory_mb = process.memory() / 1024 / 1024;
                    let cpu_percent = process.cpu_usage();

                    {
                        let mut mem_samples = memory_samples.lock().await;
                        mem_samples.push((timestamp, memory_mb));

                        // Keep only last 1000 samples to prevent unlimited growth
                        if mem_samples.len() > 1000 {
                            mem_samples.drain(0..100);
                        }
                    }

                    {
                        let mut cpu_samples = cpu_samples.lock().await;
                        cpu_samples.push((timestamp, cpu_percent));

                        if cpu_samples.len() > 1000 {
                            cpu_samples.drain(0..100);
                        }
                    }
                }
            }
        });
    }

    pub fn stop_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::Relaxed);
    }

    pub async fn get_memory_metrics(&self) -> MemoryUsageMetrics {
        let samples = self.memory_samples.lock().await;

        if samples.is_empty() {
            return MemoryUsageMetrics {
                initial_mb: 0,
                peak_mb: 0,
                average_mb: 0.0,
                final_mb: 0,
                growth_rate_mb_per_min: 0.0,
                stability_variance: 0.0,
            };
        }

        let initial_mb = samples.first().map(|(_, mem)| *mem).unwrap_or(0);
        let final_mb = samples.last().map(|(_, mem)| *mem).unwrap_or(0);
        let peak_mb = samples.iter().map(|(_, mem)| *mem).max().unwrap_or(0);
        let average_mb =
            samples.iter().map(|(_, mem)| *mem as f64).sum::<f64>() / samples.len() as f64;

        let duration_minutes = samples.last().expect("Test operation should succeed").0 / 60.0;
        let growth_rate_mb_per_min = if duration_minutes > 0.0 {
            (final_mb as f64 - initial_mb as f64) / duration_minutes
        } else {
            0.0
        };

        let variance = samples
            .iter()
            .map(|(_, mem)| {
                let diff = *mem as f64 - average_mb;
                diff * diff
            })
            .sum::<f64>()
            / samples.len() as f64;
        let stability_variance = variance.sqrt();

        MemoryUsageMetrics {
            initial_mb,
            peak_mb,
            average_mb,
            final_mb,
            growth_rate_mb_per_min,
            stability_variance,
        }
    }

    pub async fn get_cpu_metrics(&self) -> CpuUsageMetrics {
        let samples = self.cpu_samples.lock().await;

        if samples.is_empty() {
            return CpuUsageMetrics {
                average_percent: 0.0,
                peak_percent: 0.0,
                sustained_high_load_duration_sec: 0.0,
            };
        }

        let average_percent =
            samples.iter().map(|(_, cpu)| *cpu).sum::<f32>() / samples.len() as f32;
        let peak_percent = samples
            .iter()
            .map(|(_, cpu)| *cpu)
            .fold(0.0f32, |a, b| a.max(b));

        // Calculate sustained high load (>70% for >5 consecutive seconds)
        let mut sustained_duration = 0.0;
        let mut high_load_start: Option<f64> = None;

        for (timestamp, cpu) in samples.iter() {
            if *cpu > 70.0 {
                if high_load_start.is_none() {
                    high_load_start = Some(*timestamp);
                }
            } else {
                if let Some(start) = high_load_start {
                    let duration = timestamp - start;
                    if duration >= 5.0 {
                        sustained_duration += duration;
                    }
                    high_load_start = None;
                }
            }
        }

        CpuUsageMetrics {
            average_percent,
            peak_percent,
            sustained_high_load_duration_sec: sustained_duration,
        }
    }
}

// =============================================================================
// Stress Tests for Agent Performance
// =============================================================================

/// High-load concurrent agent execution stress test
/// Tests system behavior under maximum concurrent load
#[tokio::test]
async fn test_high_load_concurrent_agent_execution() -> Result<()> {
    let stress_config = StressTestConfig {
        max_concurrent_workflows: 30,
        total_workflows: 150,
        test_duration_seconds: 45,
        memory_pressure_threshold_mb: 600,
        cpu_pressure_threshold_percent: 90.0,
        max_failure_rate_percent: 20.0,
    };

    let resource_monitor = SystemResourceMonitor::new();
    resource_monitor.start_monitoring().await;

    println!("🔥 Starting High-Load Concurrent Agent Stress Test...");
    println!(
        "   Max Concurrent: {}",
        stress_config.max_concurrent_workflows
    );
    println!("   Total Workflows: {}", stress_config.total_workflows);
    println!("   Test Duration: {}s", stress_config.test_duration_seconds);

    // Setup orchestrator for high-load testing
    let system_config = SystemConfig {
        max_actors: 150, // High actor limit for stress testing
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: stress_config.max_concurrent_workflows,
        enable_resource_monitoring: true,
        health_check_interval_ms: 1000, // Less frequent during stress test
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    // Performance tracking
    let completed_workflows = Arc::new(AtomicU64::new(0));
    let failed_workflows = Arc::new(AtomicU64::new(0));
    let timeout_workflows = Arc::new(AtomicU64::new(0));
    let response_times = Arc::new(Mutex::new(Vec::new()));
    let error_counts = Arc::new(Mutex::new(HashMap::<String, u64>::new()));

    let semaphore = Arc::new(Semaphore::new(stress_config.max_concurrent_workflows));
    let test_start = Instant::now();

    // =============================================================================
    // Execute High-Load Stress Test
    // =============================================================================

    let mut workflow_handles = Vec::new();

    for workflow_id in 0..stress_config.total_workflows {
        let orchestrator_clone = Arc::clone(&orchestrator);
        let semaphore_clone = Arc::clone(&semaphore);
        let completed_clone = Arc::clone(&completed_workflows);
        let failed_clone = Arc::clone(&failed_workflows);
        let timeout_clone = Arc::clone(&timeout_workflows);
        let response_times_clone = Arc::clone(&response_times);
        let error_counts_clone = Arc::clone(&error_counts);

        let handle = tokio::spawn(async move {
            let _permit = semaphore_clone
                .acquire()
                .await
                .expect("Test operation should succeed");
            let request_start = Instant::now();

            let workflow_request = WorkflowRequest {
                user_input: format!(
                    "High-load stress test workflow #{} - process complex multi-step operation",
                    workflow_id + 1
                ),
                context: Some(serde_json::json!({
                    "stress_test": true,
                    "workflow_id": workflow_id + 1,
                    "load_type": "high_concurrent",
                    "complexity": "high",
                    "operations": {
                        "data_processing": true,
                        "analysis": true,
                        "reporting": true,
                        "validation": true
                    },
                    "resource_intensive": true
                })),
                priority: if workflow_id % 3 == 0 {
                    TaskPriority::High
                } else {
                    TaskPriority::Normal
                },
                dry_run: false,
                timeout_ms: Some(15000), // 15 second timeout for stress test
                config_overrides: Some(WorkflowConfig {
                    enable_intent_analysis: true,
                    enable_plan_generation: true,
                    enable_plan_execution: true,
                    enable_result_critique: true,
                    max_step_retries: 2, // Reduced retries for stress test
                    step_timeout_ms: 4000,
                }),
            };

            let execution_result = timeout(
                Duration::from_millis(18000), // Slightly longer than workflow timeout
                orchestrator_clone.execute_workflow(workflow_request),
            )
            .await;

            let response_time = request_start.elapsed().as_millis() as u64;

            match execution_result {
                Ok(Ok(result)) => {
                    if result.success {
                        completed_clone.fetch_add(1, Ordering::Relaxed);
                    } else {
                        failed_clone.fetch_add(1, Ordering::Relaxed);

                        // Track error types
                        if let Some(error) = result.error {
                            let mut error_counts = error_counts_clone.lock().await;
                            *error_counts.entry(error).or_insert(0) += 1;
                        }
                    }

                    {
                        let mut times = response_times_clone.lock().await;
                        times.push(response_time);
                    }
                }
                Ok(Err(e)) => {
                    failed_clone.fetch_add(1, Ordering::Relaxed);
                    let mut error_counts = error_counts_clone.lock().await;
                    *error_counts
                        .entry(format!("OrchestrationError: {}", e))
                        .or_insert(0) += 1;
                }
                Err(_) => {
                    timeout_clone.fetch_add(1, Ordering::Relaxed);
                    let mut error_counts = error_counts_clone.lock().await;
                    *error_counts.entry("TimeoutError".to_string()).or_insert(0) += 1;
                }
            }

            (workflow_id, response_time)
        });

        workflow_handles.push(handle);

        // Add small delay to prevent overwhelming the system instantly
        if workflow_id % 10 == 9 {
            sleep(Duration::from_millis(50)).await;
        }
    }

    // Wait for all workflows to complete or timeout
    for handle in workflow_handles {
        let _ = handle.await;
    }

    let total_test_duration = test_start.elapsed();
    resource_monitor.stop_monitoring();

    // =============================================================================
    // Collect and Analyze Stress Test Results
    // =============================================================================

    let completed = completed_workflows.load(Ordering::Relaxed);
    let failed = failed_workflows.load(Ordering::Relaxed);
    let timeouts = timeout_workflows.load(Ordering::Relaxed);
    let total_workflows = completed + failed + timeouts;

    let response_times_vec = {
        let mut times = response_times.lock().await;
        times.sort();
        times.clone()
    };

    let error_distribution = {
        let errors = error_counts.lock().await;
        errors.clone()
    };

    // Calculate performance metrics
    let average_response_time = if !response_times_vec.is_empty() {
        response_times_vec.iter().sum::<u64>() / response_times_vec.len() as u64
    } else {
        0
    };

    let percentile = |p: f64| -> u64 {
        if response_times_vec.is_empty() {
            return 0;
        }
        let index = ((response_times_vec.len() as f64 * p).ceil() as usize).saturating_sub(1);
        response_times_vec.get(index).copied().unwrap_or(0)
    };

    let throughput = total_workflows as f64 / total_test_duration.as_secs_f64();
    let failure_rate = (failed + timeouts) as f64 / total_workflows as f64 * 100.0;

    let memory_metrics = resource_monitor.get_memory_metrics().await;
    let cpu_metrics = resource_monitor.get_cpu_metrics().await;

    let metrics = AdvancedPerformanceMetrics {
        workflows_completed: completed,
        workflows_failed: failed,
        workflows_timeout: timeouts,
        total_execution_time_ms: total_test_duration.as_millis() as u64,
        min_response_time_ms: response_times_vec.first().copied().unwrap_or(0),
        max_response_time_ms: response_times_vec.last().copied().unwrap_or(0),
        median_response_time_ms: percentile(0.5),
        p90_response_time_ms: percentile(0.9),
        p95_response_time_ms: percentile(0.95),
        p99_response_time_ms: percentile(0.99),
        throughput_per_second: throughput,
        memory_usage_mb: memory_metrics,
        cpu_usage_percent: cpu_metrics,
        error_distribution: error_distribution.clone(),
    };

    // =============================================================================
    // Validate Stress Test Results
    // =============================================================================

    // 1. Validate minimum completion rate
    assert!(
        completed >= (stress_config.total_workflows * 80 / 100) as u64,
        "At least 80% of workflows should complete under stress: completed {}/{}",
        completed,
        stress_config.total_workflows
    );

    // 2. Validate failure rate is within acceptable limits
    assert!(
        failure_rate <= stress_config.max_failure_rate_percent,
        "Failure rate should be under {:.1}%: actual {:.1}%",
        stress_config.max_failure_rate_percent,
        failure_rate
    );

    // 3. Validate memory usage under stress
    assert!(
        metrics.memory_usage_mb.peak_mb < stress_config.memory_pressure_threshold_mb,
        "Peak memory under stress should be under {}MB: actual {}MB",
        stress_config.memory_pressure_threshold_mb,
        metrics.memory_usage_mb.peak_mb
    );

    // 4. Validate CPU usage patterns
    assert!(
        metrics.cpu_usage_percent.peak_percent < stress_config.cpu_pressure_threshold_percent,
        "Peak CPU under stress should be under {:.1}%: actual {:.1}%",
        stress_config.cpu_pressure_threshold_percent,
        metrics.cpu_usage_percent.peak_percent
    );

    // 5. Validate reasonable throughput under stress
    assert!(
        throughput >= 1.0,
        "Throughput under stress should be at least 1.0 workflows/second: actual {:.2}",
        throughput
    );

    // 6. Validate P95 response time is reasonable
    assert!(
        metrics.p95_response_time_ms < 20000,
        "P95 response time under stress should be under 20s: actual {}ms",
        metrics.p95_response_time_ms
    );

    // 7. Validate system stability (no excessive sustained high CPU)
    assert!(
        metrics.cpu_usage_percent.sustained_high_load_duration_sec
            < total_test_duration.as_secs_f64() * 0.8,
        "Sustained high CPU load should be under 80% of test duration: actual {:.1}s of {:.1}s",
        metrics.cpu_usage_percent.sustained_high_load_duration_sec,
        total_test_duration.as_secs_f64()
    );

    // Print comprehensive stress test report
    println!("⚡ High-Load Concurrent Stress Test Results:");
    println!("╭─────────────────────────────────────────╮");
    println!("│ Workflow Execution                      │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Total Workflows:       {:>11}    │", total_workflows);
    println!("│ Completed:             {:>11}    │", completed);
    println!("│ Failed:                {:>11}    │", failed);
    println!("│ Timeouts:              {:>11}    │", timeouts);
    println!(
        "│ Success Rate:          {:>8.1}%    │",
        (completed as f64 / total_workflows as f64) * 100.0
    );
    println!("│ Failure Rate:          {:>8.1}%    │", failure_rate);
    println!("├─────────────────────────────────────────┤");
    println!("│ Performance Metrics                     │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Throughput:            {:>8.2}/s    │", throughput);
    println!("│ Avg Response Time:     {:>8} ms │", average_response_time);
    println!(
        "│ Median Response Time:  {:>8} ms │",
        metrics.median_response_time_ms
    );
    println!(
        "│ P90 Response Time:     {:>8} ms │",
        metrics.p90_response_time_ms
    );
    println!(
        "│ P95 Response Time:     {:>8} ms │",
        metrics.p95_response_time_ms
    );
    println!(
        "│ P99 Response Time:     {:>8} ms │",
        metrics.p99_response_time_ms
    );
    println!(
        "│ Max Response Time:     {:>8} ms │",
        metrics.max_response_time_ms
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Resource Usage                          │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Initial Memory:        {:>8} MB │",
        metrics.memory_usage_mb.initial_mb
    );
    println!(
        "│ Peak Memory:           {:>8} MB │",
        metrics.memory_usage_mb.peak_mb
    );
    println!(
        "│ Final Memory:          {:>8} MB │",
        metrics.memory_usage_mb.final_mb
    );
    println!(
        "│ Memory Growth Rate:    {:>6.1} MB/min │",
        metrics.memory_usage_mb.growth_rate_mb_per_min
    );
    println!(
        "│ Average CPU:           {:>8.1}%    │",
        metrics.cpu_usage_percent.average_percent
    );
    println!(
        "│ Peak CPU:              {:>8.1}%    │",
        metrics.cpu_usage_percent.peak_percent
    );
    println!(
        "│ High CPU Duration:     {:>8.1}s    │",
        metrics.cpu_usage_percent.sustained_high_load_duration_sec
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Test Configuration                      │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Max Concurrent:        {:>11}    │",
        stress_config.max_concurrent_workflows
    );
    println!(
        "│ Test Duration:         {:>8.1}s    │",
        total_test_duration.as_secs_f64()
    );
    println!(
        "│ Memory Threshold:      {:>8} MB │",
        stress_config.memory_pressure_threshold_mb
    );
    println!(
        "│ CPU Threshold:         {:>8.1}%    │",
        stress_config.cpu_pressure_threshold_percent
    );
    println!("╰─────────────────────────────────────────╯");

    // Print error distribution if there are errors
    if !error_distribution.is_empty() {
        println!("🚨 Error Distribution:");
        for (error, count) in error_distribution.iter() {
            println!("   {}: {} occurrences", error, count);
        }
    }

    Ok(())
}

/// Memory pressure testing under extreme load
/// Tests system behavior when approaching memory limits
#[tokio::test]
async fn test_memory_pressure_stress() -> Result<()> {
    let resource_monitor = SystemResourceMonitor::new();
    resource_monitor.start_monitoring().await;

    println!("💾 Starting Memory Pressure Stress Test...");

    // Setup orchestrator with memory pressure configuration
    let system_config = SystemConfig {
        max_actors: 75,
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 15,
        enable_resource_monitoring: true,
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    // Get baseline memory
    sleep(Duration::from_millis(100)).await; // Allow baseline measurement
    let initial_memory = resource_monitor.get_memory_metrics().await;

    println!("   Initial Memory: {}MB", initial_memory.initial_mb);

    // =============================================================================
    // Execute Memory-Intensive Workload
    // =============================================================================

    let mut memory_pressure_handles = Vec::new();
    let completed_workflows = Arc::new(AtomicU64::new(0));
    let failed_workflows = Arc::new(AtomicU64::new(0));

    // Create memory-intensive workflows
    for batch in 0..8 {
        println!("   Executing memory pressure batch {}...", batch + 1);

        let batch_handles: Vec<_> = (0..10).map(|i| {
            let orchestrator_clone = Arc::clone(&orchestrator);
            let completed_clone = Arc::clone(&completed_workflows);
            let failed_clone = Arc::clone(&failed_workflows);
            tokio::spawn(async move {
                let workflow_request = WorkflowRequest {
                    user_input: format!("Memory-intensive data processing batch {} item {} - analyze large dataset with complex algorithms", batch + 1, i + 1),
                    context: Some(serde_json::json!({
                        "memory_pressure_test": true,
                        "batch_id": batch + 1,
                        "item_id": i + 1,
                        "data_characteristics": {
                            "size": "very_large",
                            "complexity": "high",
                            "structure": "nested_deep",
                            "processing_type": "memory_intensive"
                        },
                        "algorithms": {
                            "analysis": "comprehensive",
                            "sorting": "advanced",
                            "indexing": "full_text",
                            "aggregation": "multi_dimensional"
                        },
                        "output_requirements": {
                            "format": "detailed_report",
                            "include_raw_data": true,
                            "include_intermediate_results": true,
                            "include_metadata": true,
                            "compression": "none"
                        },
                        "resource_hints": {
                            "memory_budget": "256MB",
                            "processing_priority": "throughput_over_memory",
                            "cache_strategy": "aggressive"
                        }
                    })),
                    priority: TaskPriority::Normal,
                    dry_run: false,
                    timeout_ms: Some(12000),
                    config_overrides: Some(WorkflowConfig {
                        enable_intent_analysis: true,
                        enable_plan_generation: true,
                        enable_plan_execution: true,
                        enable_result_critique: true,
                        max_step_retries: 1, // Reduced retries to avoid memory buildup
                        step_timeout_ms: 4000,
                    }),
                };

                match orchestrator_clone.execute_workflow(workflow_request).await {
                    Ok(result) => {
                        if result.success {
                            completed_clone.fetch_add(1, Ordering::Relaxed);
                        } else {
                            failed_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        failed_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        }).collect();

        // Execute batch
        for handle in batch_handles {
            memory_pressure_handles.push(handle);
        }

        // Monitor memory during batch execution
        for _ in 0..20 {
            // Monitor for 4 seconds per batch
            sleep(Duration::from_millis(200)).await;
            let current_memory = resource_monitor.get_memory_metrics().await;

            // Log memory pressure progression
            if batch == 0 || current_memory.peak_mb > initial_memory.initial_mb + 50 {
                println!("   Memory usage: {}MB", current_memory.peak_mb);
            }
        }

        // Small stabilization pause between batches
        sleep(Duration::from_millis(500)).await;
    }

    // Wait for all memory pressure workflows to complete
    for handle in memory_pressure_handles {
        let _ = handle.await;
    }

    // Allow memory stabilization
    sleep(Duration::from_secs(3)).await;

    resource_monitor.stop_monitoring();

    // =============================================================================
    // Analyze Memory Pressure Results
    // =============================================================================

    let final_memory = resource_monitor.get_memory_metrics().await;
    let cpu_metrics = resource_monitor.get_cpu_metrics().await;

    let completed = completed_workflows.load(Ordering::Relaxed);
    let failed = failed_workflows.load(Ordering::Relaxed);
    let total_workflows = completed + failed;

    let memory_growth = final_memory
        .final_mb
        .saturating_sub(final_memory.initial_mb);
    let peak_memory_growth = final_memory.peak_mb.saturating_sub(final_memory.initial_mb);

    // =============================================================================
    // Validate Memory Pressure Behavior
    // =============================================================================

    // 1. Validate system handled memory pressure without crashes
    assert!(
        completed > 0,
        "System should complete some workflows under memory pressure: completed {}",
        completed
    );

    // 2. Validate memory usage is controlled
    assert!(
        final_memory.peak_mb < 800,
        "Peak memory under pressure should be under 800MB: actual {}MB",
        final_memory.peak_mb
    );

    // 3. Validate memory growth rate is bounded
    assert!(
        final_memory.growth_rate_mb_per_min < 100.0,
        "Memory growth rate should be under 100MB/min: actual {:.2}MB/min",
        final_memory.growth_rate_mb_per_min
    );

    // 4. Validate reasonable success rate under memory pressure
    let success_rate = completed as f64 / total_workflows as f64 * 100.0;
    assert!(
        success_rate >= 70.0,
        "Success rate under memory pressure should be at least 70%: actual {:.1}%",
        success_rate
    );

    // 5. Validate memory stabilization (final memory should not be dramatically higher)
    assert!(
        memory_growth < peak_memory_growth + 50,
        "Memory should stabilize after pressure: peak growth {}MB, final growth {}MB",
        peak_memory_growth,
        memory_growth
    );

    // 6. Validate memory variance indicates stability
    assert!(
        final_memory.stability_variance < 50.0,
        "Memory usage should be relatively stable: variance {:.2}MB",
        final_memory.stability_variance
    );

    // Print memory pressure test results
    println!("🔍 Memory Pressure Stress Test Results:");
    println!("╭─────────────────────────────────────────╮");
    println!("│ Workflow Execution                      │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Total Workflows:       {:>11}    │", total_workflows);
    println!("│ Completed:             {:>11}    │", completed);
    println!("│ Failed:                {:>11}    │", failed);
    println!("│ Success Rate:          {:>8.1}%    │", success_rate);
    println!("├─────────────────────────────────────────┤");
    println!("│ Memory Pressure Analysis                │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Initial Memory:        {:>8} MB │",
        final_memory.initial_mb
    );
    println!("│ Peak Memory:           {:>8} MB │", final_memory.peak_mb);
    println!("│ Final Memory:          {:>8} MB │", final_memory.final_mb);
    println!("│ Peak Memory Growth:    {:>8} MB │", peak_memory_growth);
    println!("│ Final Memory Growth:   {:>8} MB │", memory_growth);
    println!(
        "│ Average Memory:        {:>8.1} MB │",
        final_memory.average_mb
    );
    println!(
        "│ Growth Rate:           {:>6.1} MB/min │",
        final_memory.growth_rate_mb_per_min
    );
    println!(
        "│ Memory Stability:      {:>8.2} MB │",
        final_memory.stability_variance
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ System Performance                      │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Average CPU:           {:>8.1}%    │",
        cpu_metrics.average_percent
    );
    println!(
        "│ Peak CPU:              {:>8.1}%    │",
        cpu_metrics.peak_percent
    );
    println!(
        "│ High CPU Duration:     {:>8.1}s    │",
        cpu_metrics.sustained_high_load_duration_sec
    );
    println!("╰─────────────────────────────────────────╯");

    Ok(())
}

/// Long-running stability test to validate system endurance
/// Tests system behavior over extended periods under moderate load
#[tokio::test]
async fn test_long_running_stability() -> Result<()> {
    const TEST_DURATION_SECONDS: u64 = 120; // 2 minutes for CI-friendly test
    const WORKFLOWS_PER_MINUTE: usize = 15;

    let resource_monitor = SystemResourceMonitor::new();
    resource_monitor.start_monitoring().await;

    println!("⏱️  Starting Long-Running Stability Test...");
    println!("   Duration: {}s", TEST_DURATION_SECONDS);
    println!("   Rate: {} workflows/minute", WORKFLOWS_PER_MINUTE);

    // Setup orchestrator for stability testing
    let system_config = SystemConfig {
        max_actors: 50,
        ..Default::default()
    };
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 8,
        enable_resource_monitoring: true,
        health_check_interval_ms: 1000,
        ..Default::default()
    };
    let event_publisher = create_agent_event_publisher().await?;

    let orchestrator = Arc::new(
        AgentOrchestrator::new(system_config, orchestrator_config, event_publisher).await?,
    );

    orchestrator.initialize_agents().await?;

    // Stability tracking
    let stability_metrics = Arc::new(Mutex::new(HashMap::<String, f64>::new()));
    let workflow_counter = Arc::new(AtomicU64::new(0));
    let success_counter = Arc::new(AtomicU64::new(0));
    let error_counter = Arc::new(AtomicU64::new(0));

    let test_start = Instant::now();
    let stability_test_active = Arc::new(AtomicBool::new(true));

    // =============================================================================
    // Execute Long-Running Stability Test
    // =============================================================================

    let workflow_generator = {
        let orchestrator_clone = Arc::clone(&orchestrator);
        let counter = Arc::clone(&workflow_counter);
        let success_counter = Arc::clone(&success_counter);
        let error_counter = Arc::clone(&error_counter);
        let stability_active = Arc::clone(&stability_test_active);
        let start_time = test_start;

        tokio::spawn(async move {
            let workflow_interval = Duration::from_secs(60) / WORKFLOWS_PER_MINUTE as u32;
            let mut interval = interval(workflow_interval);

            while stability_active.load(Ordering::Relaxed)
                && start_time.elapsed() < Duration::from_secs(TEST_DURATION_SECONDS)
            {
                interval.tick().await;

                let workflow_id = counter.fetch_add(1, Ordering::Relaxed);
                let elapsed_minutes = start_time.elapsed().as_secs() / 60;

                let orchestrator_clone = Arc::clone(&orchestrator_clone);
                let success_counter = Arc::clone(&success_counter);
                let error_counter = Arc::clone(&error_counter);

                tokio::spawn(async move {
                    let workflow_request = WorkflowRequest {
                        user_input: format!("Stability test workflow #{} - minute {} - routine operation processing", workflow_id + 1, elapsed_minutes + 1),
                        context: Some(serde_json::json!({
                            "stability_test": true,
                            "workflow_id": workflow_id + 1,
                            "test_minute": elapsed_minutes + 1,
                            "operation_type": "routine",
                            "complexity": "moderate",
                            "expected_duration": "normal",
                            "resource_profile": "balanced"
                        })),
                        priority: match workflow_id % 4 {
                            0 => TaskPriority::High,
                            1 | 2 => TaskPriority::Normal,
                            _ => TaskPriority::Low,
                        },
                        dry_run: false,
                        timeout_ms: Some(8000),
                        config_overrides: Some(WorkflowConfig {
                            enable_intent_analysis: true,
                            enable_plan_generation: true,
                            enable_plan_execution: true,
                            enable_result_critique: true,
                            max_step_retries: 2,
                            step_timeout_ms: 3000,
                        }),
                    };

                    match orchestrator_clone.execute_workflow(workflow_request).await {
                        Ok(result) => {
                            if result.success {
                                success_counter.fetch_add(1, Ordering::Relaxed);
                            } else {
                                error_counter.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(_) => {
                            error_counter.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
            }
        })
    };

    // Performance monitoring during stability test
    let performance_monitor = {
        let stability_active = Arc::clone(&stability_test_active);
        let start_time = test_start;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10)); // Sample every 10 seconds
            let mut minute_reports = Vec::new();

            while stability_active.load(Ordering::Relaxed)
                && start_time.elapsed() < Duration::from_secs(TEST_DURATION_SECONDS)
            {
                interval.tick().await;

                let elapsed = start_time.elapsed();
                let minute = elapsed.as_secs() / 60;

                if minute < minute_reports.len() as u64 {
                    continue;
                }

                minute_reports.push(minute);
                println!("   Stability test: minute {} completed", minute + 1);
            }
        })
    };

    // Wait for test duration
    sleep(Duration::from_secs(TEST_DURATION_SECONDS)).await;
    stability_test_active.store(false, Ordering::Relaxed);

    // Wait for remaining workflows to complete
    sleep(Duration::from_secs(5)).await;

    resource_monitor.stop_monitoring();

    // Wait for monitors to complete
    let _ = workflow_generator.await;
    let _ = performance_monitor.await;

    let total_duration = test_start.elapsed();

    // =============================================================================
    // Analyze Stability Test Results
    // =============================================================================

    let total_workflows = workflow_counter.load(Ordering::Relaxed);
    let successful_workflows = success_counter.load(Ordering::Relaxed);
    let failed_workflows = error_counter.load(Ordering::Relaxed);

    let memory_metrics = resource_monitor.get_memory_metrics().await;
    let cpu_metrics = resource_monitor.get_cpu_metrics().await;

    let success_rate = if total_workflows > 0 {
        successful_workflows as f64 / total_workflows as f64 * 100.0
    } else {
        0.0
    };

    let actual_throughput = total_workflows as f64 / total_duration.as_secs_f64() * 60.0; // workflows per minute
    let expected_throughput = WORKFLOWS_PER_MINUTE as f64;

    // =============================================================================
    // Validate Long-Running Stability
    // =============================================================================

    // 1. Validate minimum workflow execution
    assert!(
        total_workflows
            >= (TEST_DURATION_SECONDS / 60) as u64 * WORKFLOWS_PER_MINUTE as u64 * 80 / 100,
        "Should execute at least 80% of expected workflows: expected ~{}, actual {}",
        (TEST_DURATION_SECONDS / 60) * WORKFLOWS_PER_MINUTE as u64,
        total_workflows
    );

    // 2. Validate sustained success rate
    assert!(
        success_rate >= 85.0,
        "Success rate should remain above 85% during long run: actual {:.1}%",
        success_rate
    );

    // 3. Validate memory stability (no significant leaks)
    let memory_growth = memory_metrics
        .final_mb
        .saturating_sub(memory_metrics.initial_mb);
    assert!(
        memory_growth < 100,
        "Memory growth should be under 100MB during long run: actual {}MB",
        memory_growth
    );

    // 4. Validate memory growth rate is sustainable
    assert!(
        memory_metrics.growth_rate_mb_per_min < 10.0,
        "Memory growth rate should be under 10MB/min for sustainability: actual {:.2}MB/min",
        memory_metrics.growth_rate_mb_per_min
    );

    // 5. Validate CPU usage remains reasonable
    assert!(
        cpu_metrics.average_percent < 60.0,
        "Average CPU usage should be under 60% for long-term stability: actual {:.1}%",
        cpu_metrics.average_percent
    );

    // 6. Validate throughput consistency
    let throughput_consistency = (actual_throughput / expected_throughput).abs();
    assert!(
        throughput_consistency >= 0.7 && throughput_consistency <= 1.3,
        "Throughput should be within 30% of expected: expected {:.1}, actual {:.1} workflows/min",
        expected_throughput,
        actual_throughput
    );

    // Print comprehensive stability report
    println!("📈 Long-Running Stability Test Results:");
    println!("╭─────────────────────────────────────────╮");
    println!("│ Test Execution Summary                  │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Test Duration:         {:>8.1}s    │",
        total_duration.as_secs_f64()
    );
    println!(
        "│ Target Duration:       {:>8}s    │",
        TEST_DURATION_SECONDS
    );
    println!(
        "│ Expected Workflows:    {:>11}    │",
        (TEST_DURATION_SECONDS / 60) * WORKFLOWS_PER_MINUTE as u64
    );
    println!("│ Actual Workflows:      {:>11}    │", total_workflows);
    println!(
        "│ Execution Rate:        {:>8.1}%    │",
        (total_workflows as f64
            / ((TEST_DURATION_SECONDS / 60) * WORKFLOWS_PER_MINUTE as u64) as f64)
            * 100.0
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Reliability Metrics                    │");
    println!("├─────────────────────────────────────────┤");
    println!("│ Successful:            {:>11}    │", successful_workflows);
    println!("│ Failed:                {:>11}    │", failed_workflows);
    println!("│ Success Rate:          {:>8.1}%    │", success_rate);
    println!(
        "│ Expected Throughput:   {:>6.1}/min    │",
        expected_throughput
    );
    println!(
        "│ Actual Throughput:     {:>6.1}/min    │",
        actual_throughput
    );
    println!(
        "│ Throughput Ratio:      {:>8.1}%    │",
        (actual_throughput / expected_throughput) * 100.0
    );
    println!("├─────────────────────────────────────────┤");
    println!("│ Stability Analysis                      │");
    println!("├─────────────────────────────────────────┤");
    println!(
        "│ Initial Memory:        {:>8} MB │",
        memory_metrics.initial_mb
    );
    println!(
        "│ Peak Memory:           {:>8} MB │",
        memory_metrics.peak_mb
    );
    println!(
        "│ Final Memory:          {:>8} MB │",
        memory_metrics.final_mb
    );
    println!("│ Memory Growth:         {:>8} MB │", memory_growth);
    println!(
        "│ Growth Rate:           {:>6.2} MB/min │",
        memory_metrics.growth_rate_mb_per_min
    );
    println!(
        "│ Memory Stability:      {:>8.2} MB │",
        memory_metrics.stability_variance
    );
    println!(
        "│ Average CPU:           {:>8.1}%    │",
        cpu_metrics.average_percent
    );
    println!(
        "│ Peak CPU:              {:>8.1}%    │",
        cpu_metrics.peak_percent
    );
    println!("╰─────────────────────────────────────────╯");

    println!("✅ Long-running stability test completed successfully!");
    println!(
        "   System demonstrated stable operation over {} seconds",
        TEST_DURATION_SECONDS
    );
    println!(
        "   Memory growth rate: {:.2}MB/min (sustainable)",
        memory_metrics.growth_rate_mb_per_min
    );
    println!("   Success rate: {:.1}% (high reliability)", success_rate);

    Ok(())
}
