# Multi-Agent System Integration Guide

## Overview

This guide provides step-by-step instructions for integrating and using the MAGRAY CLI multi-agent orchestration system. The system implements the Intent→Plan→Execute→Critic workflow pattern with comprehensive reliability and monitoring.

## Quick Start

### 1. Basic Setup

```rust
use orchestrator::{
    AgentOrchestrator,
    agents::{IntentAnalyzer, Planner, Executor, Critic, Scheduler},
    workflow::WorkflowConfig,
};
use tools::ToolRegistry;
use common::event_bus::GLOBAL_EVENT_BUS;

#[tokio::main]
async fn main() -> Result<()> {
    // Create tool registry
    let tool_registry = ToolRegistry::new();
    
    // Create and configure orchestrator
    let orchestrator = AgentOrchestrator::new()
        .with_tool_registry(tool_registry)
        .with_event_bus(&GLOBAL_EVENT_BUS)
        .with_health_monitoring(true)
        .build()
        .await?;
    
    // Start the orchestrator
    orchestrator.start().await?;
    
    println!("Multi-agent system started successfully!");
    
    Ok(())
}
```

### 2. Simple Workflow Execution

```rust
use orchestrator::workflow::WorkflowRequest;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    // Create workflow request
    let request = WorkflowRequest {
        id: Uuid::new_v4(),
        user_input: "backup my project to /backup".to_string(),
        context: Default::default(),
        options: Default::default(),
    };
    
    // Execute workflow
    let result = orchestrator.execute_workflow(request).await?;
    
    if result.success {
        println!("Workflow completed successfully!");
        println!("Steps completed: {:?}", result.steps_completed);
    } else {
        println!("Workflow failed: {:?}", result.error);
    }
    
    Ok(())
}
```

## Multi-Agent Workflow Patterns

### 1. Intent→Plan→Execute→Critic Workflow

The core workflow pattern processes user input through four stages:

```rust
use orchestrator::workflow::{WorkflowStepType, WorkflowState};

async fn execute_full_workflow(orchestrator: &AgentOrchestrator, user_input: &str) -> Result<()> {
    let request = WorkflowRequest::new(user_input);
    
    // The orchestrator automatically handles the workflow:
    // 1. IntentAnalyzer: Parse user input → Intent
    // 2. Planner: Intent → ActionPlan  
    // 3. Executor: ActionPlan → ExecutionResult
    // 4. Critic: ExecutionResult → Feedback
    
    let result = orchestrator.execute_workflow(request).await?;
    
    // Monitor workflow progress
    for step in &result.steps_completed {
        match step {
            WorkflowStepType::IntentAnalyzed => {
                println!("✓ Intent analyzed with confidence: {}", step.confidence);
            }
            WorkflowStepType::PlanCreated => {
                println!("✓ Action plan created with {} steps", step.plan_size);
            }
            WorkflowStepType::ExecutionCompleted => {
                println!("✓ Execution completed in {:?}", step.duration);
            }
            WorkflowStepType::ResultCritique => {
                println!("✓ Result critique generated");
            }
        }
    }
    
    Ok(())
}
```

### 2. Parallel Agent Execution

For complex workflows, multiple agents can work concurrently:

```rust
use std::sync::Arc;
use tokio::time::{timeout, Duration};

async fn concurrent_workflow_example() -> Result<()> {
    let orchestrator = Arc::new(create_orchestrator().await?);
    
    // Execute multiple workflows concurrently
    let workflows = vec![
        "analyze memory usage",
        "backup project files", 
        "run unit tests",
        "generate documentation",
    ];
    
    let mut handles = Vec::new();
    
    for input in workflows {
        let orchestrator_clone = Arc::clone(&orchestrator);
        let handle = tokio::spawn(async move {
            let request = WorkflowRequest::new(input);
            orchestrator_clone.execute_workflow(request).await
        });
        handles.push(handle);
    }
    
    // Wait for all workflows to complete (with timeout)
    let results = timeout(Duration::from_secs(300), async {
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await??);
        }
        Ok::<Vec<_>, anyhow::Error>(results)
    }).await??;
    
    println!("Completed {} workflows concurrently", results.len());
    
    Ok(())
}
```

### 3. Interactive Workflows with User Input

Handle workflows requiring user interaction:

```rust
use orchestrator::workflow::{InteractionType, UserInteraction};

async fn interactive_workflow_example() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    // Create workflow with potential user interactions
    let request = WorkflowRequest::new("deploy to production with confirmation");
    
    // Start workflow execution
    let mut workflow_handle = orchestrator.start_workflow(request).await?;
    
    loop {
        match workflow_handle.get_status().await? {
            WorkflowStatus::WaitingForUserInput(interaction) => {
                // Handle user interaction
                match interaction.interaction_type {
                    InteractionType::Confirmation => {
                        println!("Confirm: {}", interaction.prompt);
                        let response = get_user_confirmation();
                        workflow_handle.provide_user_input(response).await?;
                    }
                    InteractionType::Choice => {
                        println!("Choose: {}", interaction.prompt);
                        let choice = get_user_choice(&interaction.options);
                        workflow_handle.provide_user_input(choice).await?;
                    }
                    _ => {}
                }
            }
            WorkflowStatus::Completed(result) => {
                println!("Interactive workflow completed: {}", result.success);
                break;
            }
            WorkflowStatus::Failed(error) => {
                println!("Workflow failed: {}", error);
                break;
            }
            _ => {
                // Continue monitoring
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    Ok(())
}
```

## Agent Configuration and Customization

### 1. IntentAnalyzer Configuration

```rust
use orchestrator::agents::IntentAnalyzer;
use llm::AnthropicProvider;

async fn configure_intent_analyzer() -> Result<IntentAnalyzer> {
    let llm_provider = Box::new(AnthropicProvider::new("api_key"));
    
    let analyzer = IntentAnalyzer::new()
        .with_llm_provider(llm_provider)
        .with_confidence_threshold(0.75)
        .with_context_window_size(10)
        .with_intent_patterns(custom_patterns)
        .with_fallback_enabled(true);
    
    Ok(analyzer)
}
```

### 2. Planner Configuration

```rust
use orchestrator::agents::Planner;

async fn configure_planner(tool_registry: ToolRegistry) -> Result<Planner> {
    let planner = Planner::new()
        .with_tool_registry(tool_registry)
        .with_resource_estimator(resource_estimator)
        .with_optimization_enabled(true)
        .with_max_plan_depth(10)
        .with_parallel_execution_limit(4)
        .with_validation_rules(validation_rules);
    
    Ok(planner)
}
```

### 3. Executor Configuration

```rust
use orchestrator::agents::Executor;
use orchestrator::saga::DefaultSagaManager;

async fn configure_executor(tool_registry: ToolRegistry) -> Result<Executor> {
    let saga_manager = DefaultSagaManager::new();
    
    let executor = Executor::new()
        .with_tool_registry(tool_registry)
        .with_max_concurrent_steps(4)
        .with_resource_limits(resource_limits)
        .with_timeout_policy(timeout_policy)
        .with_saga_manager(saga_manager)
        .with_auto_rollback(true)
        .with_progress_reporting(true);
    
    Ok(executor)
}
```

## Error Handling and Recovery

### 1. Graceful Error Handling

```rust
use orchestrator::errors::{WorkflowError, WorkflowErrorType};

async fn handle_workflow_errors() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    let request = WorkflowRequest::new("complex operation");
    
    match orchestrator.execute_workflow(request).await {
        Ok(result) => {
            if result.success {
                println!("Workflow completed successfully");
            } else {
                println!("Workflow completed with issues: {:?}", result.error);
            }
        }
        Err(WorkflowError { error_type, message, recoverable, .. }) => {
            match error_type {
                WorkflowErrorType::IntentAnalysisFailed => {
                    println!("Failed to understand request: {}", message);
                    // Suggest alternative phrasings
                }
                WorkflowErrorType::PlanningFailed => {
                    println!("Cannot create execution plan: {}", message);
                    // Check tool availability
                }
                WorkflowErrorType::ExecutionFailed => {
                    println!("Execution failed: {}", message);
                    if recoverable {
                        println!("Attempting recovery...");
                        // Implement recovery logic
                    }
                }
                WorkflowErrorType::ResourceExhausted => {
                    println!("Resource limits exceeded: {}", message);
                    // Implement resource management
                }
                _ => println!("Other error: {}", message),
            }
        }
    }
    
    Ok(())
}
```

### 2. Automatic Recovery

```rust
async fn workflow_with_recovery() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    let request = WorkflowRequest::new("backup important files")
        .with_retry_policy(RetryPolicy::exponential(3, Duration::from_secs(1)))
        .with_fallback_enabled(true)
        .with_auto_recovery(true);
    
    let result = orchestrator.execute_workflow(request).await?;
    
    if let Some(recovery_info) = result.recovery_info {
        println!("Workflow recovered from {} failures", recovery_info.failure_count);
        println!("Recovery strategies used: {:?}", recovery_info.strategies_used);
    }
    
    Ok(())
}
```

## Health Monitoring and Diagnostics

### 1. Agent Health Monitoring

```rust
use orchestrator::health::{HealthStatus, HealthMonitor};

async fn monitor_agent_health() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    // Get overall system health
    let health = orchestrator.get_health_status().await?;
    
    println!("System Health: {:?}", health.overall_status);
    
    for (agent_name, agent_health) in &health.agent_statuses {
        match agent_health.status {
            HealthStatus::Healthy => {
                println!("✓ {}: Healthy", agent_name);
            }
            HealthStatus::Degraded => {
                println!("⚠ {}: Degraded - {}", agent_name, agent_health.message);
            }
            HealthStatus::Unhealthy => {
                println!("✗ {}: Unhealthy - {}", agent_name, agent_health.message);
            }
        }
    }
    
    Ok(())
}
```

### 2. Performance Metrics

```rust
use orchestrator::metrics::{MetricsCollector, WorkflowMetrics};

async fn collect_performance_metrics() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    // Execute workflow with metrics collection
    let request = WorkflowRequest::new("process data files");
    let result = orchestrator.execute_workflow(request).await?;
    
    // Analyze performance metrics
    let metrics = result.metrics;
    
    println!("Workflow Performance:");
    println!("  Total time: {:?}", metrics.total_duration);
    println!("  Intent analysis: {:?}", metrics.intent_analysis_time);
    println!("  Planning time: {:?}", metrics.planning_time);
    println!("  Execution time: {:?}", metrics.execution_time);
    println!("  Critique time: {:?}", metrics.critique_time);
    
    println!("Resource Usage:");
    println!("  CPU time: {} ms", metrics.cpu_time_ms);
    println!("  Peak memory: {} MB", metrics.memory_peak_mb);
    println!("  Tools invoked: {}", metrics.tool_invocations);
    
    Ok(())
}
```

## EventBus Integration

### 1. Event Subscription

```rust
use common::event_bus::{GLOBAL_EVENT_BUS, Event};

async fn setup_event_monitoring() -> Result<()> {
    // Subscribe to workflow events
    GLOBAL_EVENT_BUS.subscribe("workflow.started", |event: Event| {
        println!("Workflow started: {}", event.data["workflow_id"]);
    }).await;
    
    GLOBAL_EVENT_BUS.subscribe("workflow.step.completed", |event: Event| {
        println!("Step completed: {} - {:?}", 
            event.data["step_type"], 
            event.data["duration"]
        );
    }).await;
    
    GLOBAL_EVENT_BUS.subscribe("workflow.completed", |event: Event| {
        println!("Workflow completed: success = {}", event.data["success"]);
    }).await;
    
    // Subscribe to agent-specific events
    GLOBAL_EVENT_BUS.subscribe("agent.intent.analyzed", |event: Event| {
        println!("Intent analyzed: type = {}, confidence = {}", 
            event.data["intent_type"],
            event.data["confidence"]
        );
    }).await;
    
    GLOBAL_EVENT_BUS.subscribe("agent.plan.created", |event: Event| {
        println!("Plan created: {} steps", event.data["step_count"]);
    }).await;
    
    Ok(())
}
```

### 2. Custom Event Handlers

```rust
use orchestrator::events::{WorkflowEventHandler, WorkflowEvent};

struct CustomWorkflowHandler;

#[async_trait]
impl WorkflowEventHandler for CustomWorkflowHandler {
    async fn handle_workflow_started(&self, event: &WorkflowEvent) -> Result<()> {
        println!("Custom handler: Workflow {} started", event.workflow_id);
        // Custom logic here
        Ok(())
    }
    
    async fn handle_workflow_failed(&self, event: &WorkflowEvent) -> Result<()> {
        println!("Custom handler: Workflow {} failed", event.workflow_id);
        // Custom error handling, notifications, etc.
        Ok(())
    }
}

async fn setup_custom_handlers() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    let handler = Box::new(CustomWorkflowHandler);
    
    orchestrator.add_event_handler(handler).await?;
    
    Ok(())
}
```

## Best Practices

### 1. Resource Management

```rust
// Configure appropriate resource limits
let resource_limits = ResourceLimits {
    max_cpu_cores: num_cpus::get() / 2,     // Use half available cores
    max_memory_mb: 2048,                    // 2GB memory limit
    max_execution_time: Duration::from_secs(3600), // 1 hour timeout
    max_concurrent_workflows: 4,            // Limit concurrent workflows
};

let orchestrator = AgentOrchestrator::new()
    .with_resource_limits(resource_limits)
    .build().await?;
```

### 2. Error Recovery Strategies

```rust
// Configure comprehensive retry policies
let retry_policy = RetryPolicy {
    max_retries: 3,
    backoff_strategy: BackoffStrategy::ExponentialWithJitter {
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
        multiplier: 2.0,
        jitter: 0.1,
    },
    retry_on_errors: vec![
        "NetworkError".to_string(),
        "TemporaryFailure".to_string(),
        "ResourceTemporarilyUnavailable".to_string(),
    ],
};
```

### 3. Monitoring and Alerting

```rust
// Set up comprehensive monitoring
async fn setup_monitoring() -> Result<()> {
    let orchestrator = create_orchestrator().await?;
    
    // Monitor health every 30 seconds
    let health_monitor = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            
            if let Ok(health) = orchestrator.get_health_status().await {
                if health.overall_status != HealthStatus::Healthy {
                    // Send alert
                    send_health_alert(&health).await;
                }
            }
        }
    });
    
    // Monitor performance metrics
    let metrics_monitor = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            
            if let Ok(metrics) = orchestrator.get_performance_metrics().await {
                if metrics.average_response_time > Duration::from_secs(10) {
                    // Performance degradation detected
                    send_performance_alert(&metrics).await;
                }
            }
        }
    });
    
    Ok(())
}
```

### 4. Testing Strategies

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_workflow_execution() -> Result<()> {
        let orchestrator = create_test_orchestrator().await?;
        
        let request = WorkflowRequest::new("test operation");
        let result = orchestrator.execute_workflow(request).await?;
        
        assert!(result.success);
        assert!(!result.steps_completed.is_empty());
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_concurrent_workflows() -> Result<()> {
        let orchestrator = Arc::new(create_test_orchestrator().await?);
        
        let workflows = (0..5).map(|i| {
            let orchestrator = Arc::clone(&orchestrator);
            tokio::spawn(async move {
                let request = WorkflowRequest::new(&format!("test workflow {}", i));
                orchestrator.execute_workflow(request).await
            })
        }).collect::<Vec<_>>();
        
        let results = futures::future::try_join_all(workflows).await?;
        let success_count = results.into_iter().filter(|r| r.success).count();
        
        assert_eq!(success_count, 5);
        
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **High Memory Usage**: Increase resource limits or reduce concurrent workflows
2. **Slow Performance**: Check tool availability and network connectivity
3. **Agent Unhealthy**: Restart specific agents or check dependencies
4. **Workflow Timeouts**: Adjust timeout policies or break down complex operations

### Debug Mode

```rust
let orchestrator = AgentOrchestrator::new()
    .with_debug_mode(true)
    .with_verbose_logging(true)
    .build().await?;
```

This enables detailed logging and debug information for troubleshooting workflow issues.

## Advanced Topics

For advanced usage patterns, see:
- [Saga Pattern Implementation](./saga-pattern.md)
- [Custom Agent Development](./custom-agents.md)
- [Performance Optimization](./performance-optimization.md)
- [Security Considerations](./security.md)