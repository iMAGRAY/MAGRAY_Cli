# Executor API Documentation

## Overview

The `Executor` agent provides deterministic, reliable execution of action plans with comprehensive state tracking, rollback capabilities, and tool integration.

## API Contract

### ExecutorTrait

```rust
#[async_trait]
pub trait ExecutorTrait: Send + Sync {
    /// Execute an action plan with full state tracking
    async fn execute_plan(&self, plan: &ActionPlan) -> Result<ExecutionResult>;
    
    /// Execute a single step (useful for testing and debugging)
    async fn execute_step(&self, step: &ActionStep, context: &ExecutionContext) -> Result<StepResult>;
    
    /// Cancel a running execution
    async fn cancel_execution(&self, plan_id: Uuid) -> Result<()>;
    
    /// Pause execution (can be resumed later)
    async fn pause_execution(&self, plan_id: Uuid) -> Result<()>;
    
    /// Resume a paused execution
    async fn resume_execution(&self, plan_id: Uuid) -> Result<()>;
    
    /// Get execution status and progress
    async fn get_execution_status(&self, plan_id: Uuid) -> Result<ExecutionStatus>;
    
    /// Rollback a failed or cancelled execution
    async fn rollback_execution(&self, plan_id: Uuid) -> Result<()>;
}
```

## Data Structures

### ExecutionResult

Complete result of plan execution:

```rust
pub struct ExecutionResult {
    pub plan_id: Uuid,                         // Plan identifier
    pub status: ExecutionStatus,               // Final execution status
    pub step_results: Vec<StepResult>,         // Results for each step
    pub execution_time: Duration,              // Total execution time
    pub resource_usage: ResourceUsage,         // Resource consumption
    pub metadata: HashMap<String, serde_json::Value>, // Additional data
    pub error: Option<ExecutionError>,         // Error details if failed
}
```

### ExecutionStatus

Current state of execution:

```rust
pub enum ExecutionStatus {
    Pending,        // Waiting to start
    Running,        // Currently executing
    Completed,      // Successfully finished
    Failed,         // Failed with errors
    Cancelled,      // Cancelled by user
    Paused,         // Paused (can resume)
}
```

### StepResult

Result of individual step execution:

```rust
pub struct StepResult {
    pub step_id: Uuid,                         // Step identifier
    pub status: StepStatus,                    // Step execution status
    pub output: Option<serde_json::Value>,     // Step output data
    pub error: Option<String>,                 // Error message if failed
    pub execution_time: Duration,              // Step execution time
    pub retry_count: u32,                      // Number of retries performed
    pub metadata: HashMap<String, serde_json::Value>, // Step-specific metadata
}
```

### ResourceUsage

Resource consumption tracking:

```rust
pub struct ResourceUsage {
    pub cpu_time_ms: u64,          // CPU time consumed
    pub memory_peak_mb: u64,       // Peak memory usage
    pub disk_reads: u64,           // Number of disk reads
    pub disk_writes: u64,          // Number of disk writes
    pub network_requests: u64,     // Network requests made
    pub tool_invocations: u64,     // Tools invoked
}
```

## Usage Examples

### Basic Plan Execution

```rust
use orchestrator::agents::{Executor, ExecutorTrait, ActionPlan};
use tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    // Create executor with tool registry
    let tool_registry = ToolRegistry::new();
    let executor = Executor::new()
        .with_tool_registry(tool_registry)
        .with_max_concurrent_steps(4)
        .with_resource_limits(resource_limits);
    
    // Execute the plan
    let result = executor.execute_plan(&action_plan).await?;
    
    match result.status {
        ExecutionStatus::Completed => {
            println!("Plan executed successfully in {:?}", result.execution_time);
            println!("Steps completed: {}", result.step_results.len());
        }
        ExecutionStatus::Failed => {
            if let Some(error) = result.error {
                println!("Execution failed: {}", error.message);
            }
        }
        _ => println!("Unexpected execution status: {:?}", result.status),
    }
    
    Ok(())
}
```

### Step-by-Step Execution with Monitoring

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let executor = Executor::new();
    let plan_id = action_plan.id;
    
    // Start execution in background
    let execution_handle = tokio::spawn({
        let executor = executor.clone();
        let plan = action_plan.clone();
        async move {
            executor.execute_plan(&plan).await
        }
    });
    
    // Monitor execution progress
    loop {
        let status = executor.get_execution_status(plan_id).await?;
        
        match status {
            ExecutionStatus::Running => {
                println!("Execution in progress...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            ExecutionStatus::Completed => {
                println!("Execution completed successfully!");
                break;
            }
            ExecutionStatus::Failed => {
                println!("Execution failed, attempting rollback...");
                executor.rollback_execution(plan_id).await?;
                break;
            }
            _ => break,
        }
    }
    
    let result = execution_handle.await??;
    Ok(())
}
```

### Execution with Pause/Resume

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let executor = Executor::new();
    let plan_id = action_plan.id;
    
    // Start execution
    let execution_handle = tokio::spawn({
        let executor = executor.clone();
        let plan = action_plan.clone();
        async move {
            executor.execute_plan(&plan).await
        }
    });
    
    // Pause after 5 seconds
    tokio::time::sleep(Duration::from_secs(5)).await;
    executor.pause_execution(plan_id).await?;
    println!("Execution paused");
    
    // Resume after user input
    println!("Press Enter to resume execution...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    executor.resume_execution(plan_id).await?;
    println!("Execution resumed");
    
    let result = execution_handle.await??;
    Ok(())
}
```

### Rollback on Failure

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let executor = Executor::new()
        .with_saga_manager(saga_manager)  // Enable saga-based rollback
        .with_auto_rollback(true);        // Automatic rollback on failure
    
    let result = executor.execute_plan(&action_plan).await?;
    
    if let ExecutionStatus::Failed = result.status {
        println!("Execution failed, rolling back changes...");
        
        // Manual rollback (auto-rollback also available)
        executor.rollback_execution(action_plan.id).await?;
        
        println!("Rollback completed");
        
        // Analyze failure for improvement
        if let Some(error) = result.error {
            println!("Failure reason: {}", error.message);
            if error.retryable {
                println!("This error is retryable");
            }
        }
    }
    
    Ok(())
}
```

### Tool Integration

```rust
use tools::{ToolRegistry, Tool, ToolSpec};

#[tokio::main]
async fn main() -> Result<()> {
    let mut tool_registry = ToolRegistry::new();
    
    // Register custom tool
    let custom_tool = CustomTool::new();
    tool_registry.register_tool("custom_task", Box::new(custom_tool)).await?;
    
    let executor = Executor::new()
        .with_tool_registry(tool_registry);
    
    // Create plan with tool execution step
    let step = ActionStep {
        id: Uuid::new_v4(),
        step_type: ActionStepType::ToolExecution {
            tool_name: "custom_task".to_string(),
            arguments: [
                ("input_file".to_string(), json!("data.txt")),
                ("output_format".to_string(), json!("json")),
            ].into(),
        },
        parameters: HashMap::new(),
        dependencies: vec![],
        expected_duration: Duration::from_secs(30),
        retry_policy: RetryPolicy::default(),
        validation_rules: vec![],
    };
    
    // Execute step
    let context = ExecutionContext::new();
    let result = executor.execute_step(&step, &context).await?;
    
    if let StepStatus::Completed = result.status {
        println!("Tool executed successfully");
        if let Some(output) = result.output {
            println!("Tool output: {}", output);
        }
    }
    
    Ok(())
}
```

## Configuration

### Executor Configuration

```rust
let executor = Executor::new()
    .with_tool_registry(tool_registry)          // Available tools
    .with_max_concurrent_steps(8)               // Parallel execution limit
    .with_resource_limits(resource_limits)      // Resource constraints
    .with_timeout_policy(timeout_policy)        // Step timeout configuration
    .with_retry_policy(retry_policy)            // Default retry behavior
    .with_saga_manager(saga_manager)            // Transaction management
    .with_auto_rollback(true)                   // Automatic rollback
    .with_progress_reporting(true);             // Progress updates
```

### Resource Limits

```rust
use orchestrator::agents::executor::ResourceLimits;

let resource_limits = ResourceLimits {
    max_cpu_cores: 4,
    max_memory_mb: 2048,
    max_disk_space_mb: 10000,
    max_network_requests: 100,
    max_execution_time: Duration::from_secs(3600),
    max_tool_invocations: 50,
};
```

## Saga Integration

### Transactional Execution

```rust
use orchestrator::saga::{SagaManager, DefaultSagaManager};

#[tokio::main]
async fn main() -> Result<()> {
    let saga_manager = DefaultSagaManager::new();
    
    let executor = Executor::new()
        .with_saga_manager(saga_manager);
    
    // Execution automatically creates saga for transactional consistency
    let result = executor.execute_plan(&action_plan).await?;
    
    // If execution fails, saga automatically handles compensation
    if let ExecutionStatus::Failed = result.status {
        println!("Saga compensation executed automatically");
    }
    
    Ok(())
}
```

## Error Handling

### Execution Error Types

```rust
use orchestrator::agents::executor::{ExecutionError, ExecutionErrorType};

match result.status {
    ExecutionStatus::Failed => {
        if let Some(error) = result.error {
            match error.error_type {
                ExecutionErrorType::ToolNotFound => {
                    println!("Tool not found: {}", error.message);
                }
                ExecutionErrorType::ResourceExhausted => {
                    println!("Resource limits exceeded: {}", error.message);
                }
                ExecutionErrorType::TimeoutError => {
                    println!("Execution timed out: {}", error.message);
                }
                ExecutionErrorType::NetworkError => {
                    if error.retryable {
                        println!("Network error (retryable): {}", error.message);
                    }
                }
                _ => println!("Other error: {}", error.message),
            }
        }
    }
    _ => {}
}
```

## Performance Monitoring

### Execution Metrics

```rust
let result = executor.execute_plan(&action_plan).await?;

// Analyze resource usage
let usage = result.resource_usage;
println!("Resource Usage Summary:");
println!("  CPU time: {} ms", usage.cpu_time_ms);
println!("  Peak memory: {} MB", usage.memory_peak_mb);
println!("  Disk I/O: {} reads, {} writes", usage.disk_reads, usage.disk_writes);
println!("  Network requests: {}", usage.network_requests);
println!("  Tools invoked: {}", usage.tool_invocations);

// Analyze step performance
for step_result in &result.step_results {
    println!("Step {}: {:?} in {:?}", 
        step_result.step_id, 
        step_result.status,
        step_result.execution_time
    );
    
    if step_result.retry_count > 0 {
        println!("  Retries: {}", step_result.retry_count);
    }
}
```

### Concurrent Execution

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let executor = Executor::new()
        .with_max_concurrent_steps(4);  // Execute up to 4 steps in parallel
    
    // Plans with independent steps will execute concurrently
    let result = executor.execute_plan(&action_plan).await?;
    
    println!("Plan executed with parallel steps");
    Ok(())
}
```