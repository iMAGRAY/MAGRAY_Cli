# Planner API Documentation

## Overview

The `Planner` agent converts structured intents into executable action plans with ordered steps, dependencies, and resource requirements.

## API Contract

### PlannerTrait

```rust
#[async_trait]
pub trait PlannerTrait: Send + Sync {
    /// Build an action plan from the given intent
    async fn build_plan(&self, intent: &Intent) -> Result<ActionPlan>;
    
    /// Validate a plan for feasibility and correctness
    async fn validate_plan(&self, plan: &ActionPlan) -> Result<PlanValidationResult>;
    
    /// Optimize a plan for better performance or resource usage
    async fn optimize_plan(&self, plan: &ActionPlan) -> Result<ActionPlan>;
    
    /// Get planning statistics
    async fn get_statistics(&self) -> Result<PlanningStats>;
}
```

## Data Structures

### ActionPlan

Represents an executable plan with ordered steps:

```rust
pub struct ActionPlan {
    pub id: Uuid,                           // Unique plan identifier
    pub intent_id: Uuid,                    // Associated intent ID
    pub steps: Vec<ActionStep>,             // Ordered execution steps
    pub estimated_duration: Duration,       // Estimated execution time
    pub resource_requirements: ResourceRequirements, // Required resources
    pub dependencies: Vec<Uuid>,            // External dependencies
    pub metadata: HashMap<String, serde_json::Value>, // Additional metadata
}
```

### ActionStep

Individual step in an action plan:

```rust
pub struct ActionStep {
    pub id: Uuid,                           // Step identifier
    pub step_type: ActionStepType,          // Type of action to perform
    pub parameters: HashMap<String, serde_json::Value>, // Step parameters
    pub dependencies: Vec<Uuid>,            // Step dependencies
    pub expected_duration: Duration,        // Expected execution time
    pub retry_policy: RetryPolicy,          // Retry configuration
    pub validation_rules: Vec<ValidationRule>, // Validation requirements
}
```

### ActionStepType

Types of actions that can be performed:

```rust
pub enum ActionStepType {
    /// Execute a specific tool
    ToolExecution {
        tool_name: String,
        arguments: HashMap<String, serde_json::Value>,
    },
    
    /// Conditional logic step
    Conditional {
        condition: String,
        then_steps: Vec<ActionStep>,
        else_steps: Vec<ActionStep>,
    },
    
    /// Loop execution
    Loop {
        condition: String,
        body_steps: Vec<ActionStep>,
        max_iterations: u32,
    },
    
    /// Memory operation
    MemoryOperation {
        operation_type: MemoryOperationType,
        query: String,
    },
    
    /// User interaction
    UserInteraction {
        interaction_type: InteractionType,
        prompt: String,
    },
    
    /// Wait/delay step
    Wait { duration: Duration },
}
```

## Usage Examples

### Basic Plan Generation

```rust
use orchestrator::agents::{Planner, PlannerTrait, Intent, IntentType};

#[tokio::main]
async fn main() -> Result<()> {
    let planner = Planner::new()
        .with_tool_registry(tool_registry)
        .with_resource_estimator(estimator);
    
    // Create intent for file backup
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::FileOperation {
            operation: "backup".to_string(),
            path: "/home/user/project".to_string(),
        },
        parameters: [
            ("destination".to_string(), json!("/backup/project-backup")),
            ("compression".to_string(), json!(true)),
        ].into(),
        confidence: 0.9,
        context: intent_context,
    };
    
    // Generate action plan
    let plan = planner.build_plan(&intent).await?;
    
    println!("Generated plan with {} steps", plan.steps.len());
    println!("Estimated duration: {:?}", plan.estimated_duration);
    
    for (i, step) in plan.steps.iter().enumerate() {
        println!("Step {}: {:?}", i + 1, step.step_type);
    }
    
    Ok(())
}
```

### Plan Validation and Optimization

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let planner = Planner::new();
    
    // Build initial plan
    let plan = planner.build_plan(&intent).await?;
    
    // Validate plan feasibility
    let validation = planner.validate_plan(&plan).await?;
    
    if !validation.is_valid {
        println!("Plan validation failed:");
        for error in &validation.errors {
            println!("  - {}", error);
        }
        return Ok(());
    }
    
    // Optimize plan for better performance
    let optimized_plan = planner.optimize_plan(&plan).await?;
    
    println!("Original plan duration: {:?}", plan.estimated_duration);
    println!("Optimized plan duration: {:?}", optimized_plan.estimated_duration);
    
    Ok(())
}
```

### Complex Workflow Planning

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let planner = Planner::new();
    
    // Intent for complex deployment workflow
    let intent = Intent {
        id: Uuid::new_v4(),
        intent_type: IntentType::WorkflowExecution {
            workflow_name: "deployment".to_string(),
        },
        parameters: [
            ("environment".to_string(), json!("production")),
            ("rollback_enabled".to_string(), json!(true)),
            ("health_checks".to_string(), json!(true)),
        ].into(),
        confidence: 0.85,
        context: intent_context,
    };
    
    let plan = planner.build_plan(&intent).await?;
    
    // Display plan structure
    for step in &plan.steps {
        match &step.step_type {
            ActionStepType::ToolExecution { tool_name, .. } => {
                println!("Execute tool: {}", tool_name);
            }
            ActionStepType::Conditional { condition, .. } => {
                println!("Conditional: {}", condition);
            }
            ActionStepType::UserInteraction { prompt, .. } => {
                println!("User interaction: {}", prompt);
            }
            _ => println!("Other step type"),
        }
    }
    
    Ok(())
}
```

### Resource Planning

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let planner = Planner::new();
    let plan = planner.build_plan(&intent).await?;
    
    // Check resource requirements
    let resources = &plan.resource_requirements;
    
    println!("Resource Requirements:");
    println!("  CPU cores: {}", resources.cpu_cores);
    println!("  Memory: {} MB", resources.memory_mb);
    println!("  Disk space: {} MB", resources.disk_space_mb);
    println!("  Network required: {}", resources.network_required);
    
    println!("Tools required:");
    for tool in &resources.tools_required {
        println!("  - {}", tool);
    }
    
    println!("Permissions required:");
    for permission in &resources.permissions_required {
        println!("  - {}", permission);
    }
    
    Ok(())
}
```

## Configuration

### Planner Configuration

```rust
let planner = Planner::new()
    .with_tool_registry(tool_registry)          // Available tools
    .with_resource_estimator(estimator)         // Resource estimation
    .with_optimization_enabled(true)            // Enable plan optimization
    .with_max_plan_depth(10)                   // Maximum step nesting
    .with_max_parallel_steps(4)                // Parallel execution limit
    .with_timeout_policy(timeout_policy);       // Step timeout configuration
```

### Retry Policies

```rust
use orchestrator::agents::planner::{RetryPolicy, BackoffStrategy};

let retry_policy = RetryPolicy {
    max_retries: 3,
    backoff_strategy: BackoffStrategy::Exponential {
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(60),
        multiplier: 2.0,
    },
    retry_on_errors: vec![
        "NetworkError".to_string(),
        "TemporaryFailure".to_string(),
    ],
};
```

## Integration with Tools

### Tool Registry Integration

```rust
use tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    let tool_registry = ToolRegistry::new();
    
    // Register available tools
    tool_registry.register_tool("file_copy", file_copy_tool).await?;
    tool_registry.register_tool("compress", compression_tool).await?;
    tool_registry.register_tool("git_commit", git_tool).await?;
    
    let planner = Planner::new()
        .with_tool_registry(tool_registry);
    
    // Planner will now consider available tools when generating plans
    let plan = planner.build_plan(&intent).await?;
    
    Ok(())
}
```

## Plan Validation

### Validation Rules

```rust
use orchestrator::agents::planner::{ValidationRule, ValidationRuleType};

let validation_rules = vec![
    ValidationRule {
        rule_type: ValidationRuleType::ResourceLimit,
        condition: "memory_mb < 1024".to_string(),
        error_message: "Step requires too much memory".to_string(),
    },
    ValidationRule {
        rule_type: ValidationRuleType::DependencyCheck,
        condition: "tool_available('git')".to_string(),
        error_message: "Git tool not available".to_string(),
    },
];
```

### Validation Results

```rust
let validation = planner.validate_plan(&plan).await?;

if validation.is_valid {
    println!("Plan is valid and ready for execution");
} else {
    println!("Plan validation failed:");
    for error in &validation.errors {
        println!("  Error: {}", error.message);
        if let Some(step_id) = error.step_id {
            println!("    Step: {}", step_id);
        }
    }
    
    for warning in &validation.warnings {
        println!("  Warning: {}", warning);
    }
}
```

## Performance Optimization

### Plan Optimization Strategies

- **Step Parallelization**: Independent steps executed concurrently
- **Resource Optimization**: Efficient resource allocation and reuse  
- **Dependency Minimization**: Reduce unnecessary step dependencies
- **Tool Selection**: Choose optimal tools for each operation
- **Caching**: Cache intermediate results to avoid redundant work

### Monitoring Planning Performance

```rust
let stats = planner.get_statistics().await?;

println!("Planning Statistics:");
println!("  Plans generated: {}", stats.plans_generated);
println!("  Average planning time: {:?}", stats.average_planning_time);
println!("  Plan validation success rate: {:.2}%", stats.validation_success_rate);
println!("  Average plan size: {:.1} steps", stats.average_plan_size);
```

## Error Handling

```rust
match planner.build_plan(&intent).await {
    Ok(plan) => {
        // Validate before execution
        let validation = planner.validate_plan(&plan).await?;
        if validation.is_valid {
            println!("Plan ready for execution");
        } else {
            println!("Plan requires fixes before execution");
        }
    }
    Err(e) => {
        eprintln!("Planning failed: {}", e);
        // Handle planning errors (missing tools, invalid parameters, etc.)
    }
}
```