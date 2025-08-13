# Multi-Agent Integration Guide

## 📚 Overview

Complete guide для integrating MAGRAY CLI's multi-agent orchestration system into your applications. This guide covers the Intent→Plan→Execute→Critic workflow, agent lifecycle management, и production deployment patterns.

## 🏗️ Architecture Understanding

### Multi-Agent Workflow
```
User Input → IntentAnalyzer → Planner → Executor → Critic → Results
     ↓           ↓              ↓         ↓        ↓
 EventBus ←── Scheduler ←── Resource Manager ←── Health Monitor
```

### Core Components
- **IntentAnalyzer**: Natural language understanding и intent extraction
- **Planner**: Action planning и step decomposition
- **Executor**: Deterministic execution с rollback support
- **Critic**: Quality assessment и improvement suggestions
- **Scheduler**: Background task management и coordination

## 🚀 Quick Start Integration

### Basic Setup

```rust
use orchestrator::{
    AgentOrchestrator,
    OrchestratorConfig,
    SystemConfig,
    WorkflowRequest,
    TaskPriority
};
use common::event_bus::create_event_bus;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Configure the system
    let system_config = SystemConfig {
        max_concurrent_agents: 10,
        agent_timeout: std::time::Duration::from_secs(30),
        enable_health_monitoring: true,
        event_bus_capacity: 1000,
        resource_limits: Default::default(),
    };
    
    let orchestrator_config = OrchestratorConfig {
        enable_background_tasks: true,
        health_check_interval: std::time::Duration::from_secs(60),
        workflow_timeout: std::time::Duration::from_secs(300),
        enable_metrics_collection: true,
    };
    
    // 2. Create event publisher
    let event_bus = create_event_bus().await?;
    let event_publisher = event_bus.publisher("agent.events").await?;
    
    // 3. Initialize orchestrator
    let orchestrator = AgentOrchestrator::new(
        system_config,
        orchestrator_config,
        event_publisher
    ).await?;
    
    // 4. Initialize all agents
    orchestrator.initialize_agents().await?;
    
    // 5. Execute workflow
    let request = WorkflowRequest {
        user_input: "Create a user authentication system with JWT tokens".to_string(),
        context: Some("Web application backend".to_string()),
        priority: TaskPriority::Normal,
        dry_run: false,
        timeout_ms: Some(180000), // 3 minutes
        config_overrides: None,
    };
    
    let result = orchestrator.execute_workflow(request).await?;
    
    println!("Workflow completed: {}", result.success);
    println!("Steps executed: {}", result.steps_completed);
    println!("Result: {}", result.output);
    
    if let Some(critic_feedback) = result.critic_feedback {
        println!("Quality assessment: {}", critic_feedback);
    }
    
    // 6. Graceful shutdown
    orchestrator.shutdown().await?;
    
    Ok(())
}
```

### Configuration Options

```rust
use orchestrator::{SystemConfig, OrchestratorConfig, ResourceBudget};
use std::time::Duration;

// Advanced system configuration
let system_config = SystemConfig {
    // Actor system settings
    max_concurrent_agents: 20,
    agent_timeout: Duration::from_secs(60),
    actor_mailbox_size: 1000,
    
    // Health monitoring
    enable_health_monitoring: true,
    health_check_interval: Duration::from_secs(30),
    unhealthy_agent_restart_threshold: 3,
    
    // Event bus configuration
    event_bus_capacity: 5000,
    enable_event_replay: true,
    event_retention_duration: Duration::from_hours(24),
    
    // Resource management
    resource_limits: ResourceBudget::new()
        .memory_limit(1024 * 1024 * 1024) // 1GB
        .cpu_time_limit(Duration::from_secs(300))
        .max_file_handles(100),
        
    // Reliability settings
    enable_circuit_breaker: true,
    circuit_breaker_failure_threshold: 5,
    circuit_breaker_recovery_timeout: Duration::from_secs(30),
};

let orchestrator_config = OrchestratorConfig {
    // Workflow settings
    workflow_timeout: Duration::from_secs(600), // 10 minutes
    max_concurrent_workflows: 5,
    enable_workflow_history: true,
    
    // Background tasks
    enable_background_tasks: true,
    background_task_interval: Duration::from_secs(120),
    enable_automatic_cleanup: true,
    
    // Performance monitoring
    enable_metrics_collection: true,
    metrics_collection_interval: Duration::from_secs(15),
    enable_performance_profiling: true,
    
    // Agent coordination
    inter_agent_communication_timeout: Duration::from_secs(10),
    enable_agent_load_balancing: true,
    agent_failure_recovery_strategy: "restart".to_string(),
};
```

## 🔧 Advanced Integration Patterns

### Custom Agent Implementation

```rust
use orchestrator::{
    agents::{IntentAnalyzerTrait, PlannerTrait, ExecutorTrait, CriticTrait},
    AgentMessage, ExecutionStatus, TaskPriority
};
use async_trait::async_trait;

// Custom Intent Analyzer
pub struct CustomIntentAnalyzer {
    domain_knowledge: DomainKnowledgeBase,
    nlp_model: NLPModel,
}

#[async_trait]
impl IntentAnalyzerTrait for CustomIntentAnalyzer {
    async fn analyze_intent(&self, input: &str, context: Option<&str>) -> Result<Intent> {
        // Custom intent analysis logic
        let domain_context = self.domain_knowledge.get_context(context)?;
        let intent = self.nlp_model.extract_intent(input, &domain_context).await?;
        
        // Enrich with domain-specific information
        let enriched_intent = Intent {
            primary_action: intent.primary_action,
            entities: intent.entities,
            confidence: intent.confidence,
            domain_specific_data: Some(domain_context.metadata),
            user_preferences: self.load_user_preferences().await?,
            context_history: self.get_conversation_history().await?,
        };
        
        Ok(enriched_intent)
    }
    
    async fn validate_intent(&self, intent: &Intent) -> Result<ValidationResult> {
        // Custom validation logic
        let mut issues = Vec::new();
        
        // Check for ambiguous intents
        if intent.confidence < 0.7 {
            issues.push("Intent confidence below threshold".to_string());
        }
        
        // Validate against domain constraints
        if !self.domain_knowledge.supports_action(&intent.primary_action) {
            issues.push(format!("Action '{}' not supported in current domain", 
                intent.primary_action));
        }
        
        Ok(ValidationResult {
            is_valid: issues.is_empty(),
            issues,
            suggestions: self.generate_clarification_questions(intent).await?,
        })
    }
}

// Custom Planner with domain-specific strategies
pub struct DomainSpecificPlanner {
    strategy_registry: PlanningStrategyRegistry,
    resource_estimator: ResourceEstimator,
}

#[async_trait]
impl PlannerTrait for DomainSpecificPlanner {
    async fn build_plan(&self, intent: Intent) -> Result<ActionPlan> {
        // Select appropriate planning strategy
        let strategy = self.strategy_registry
            .get_strategy(&intent.primary_action)
            .await?;
        
        // Generate base plan
        let base_plan = strategy.generate_plan(intent.clone()).await?;
        
        // Optimize plan based on resources
        let resource_estimates = self.resource_estimator
            .estimate_plan_resources(&base_plan)
            .await?;
            
        let optimized_plan = if resource_estimates.exceeds_limits() {
            self.optimize_plan_for_resources(base_plan, resource_estimates).await?
        } else {
            base_plan
        };
        
        // Add error handling и rollback steps
        let robust_plan = self.add_error_handling(optimized_plan).await?;
        
        Ok(robust_plan)
    }
    
    async fn validate_plan(&self, plan: &ActionPlan) -> Result<PlanValidation> {
        // Custom plan validation
        let mut validation = PlanValidation::default();
        
        // Check for dependency cycles
        if self.has_dependency_cycles(plan)? {
            validation.add_error("Plan contains dependency cycles");
        }
        
        // Validate resource requirements
        let resource_check = self.validate_resource_requirements(plan).await?;
        validation.merge(resource_check);
        
        // Check for security implications
        let security_check = self.validate_plan_security(plan).await?;
        validation.merge(security_check);
        
        Ok(validation)
    }
}

// Register custom agents
let orchestrator = AgentOrchestrator::new(system_config, orchestrator_config, event_publisher)
    .await?
    .with_custom_intent_analyzer(Box::new(CustomIntentAnalyzer::new()))
    .with_custom_planner(Box::new(DomainSpecificPlanner::new()))
    .with_custom_executor(Box::new(DomainSpecificExecutor::new()))
    .with_custom_critic(Box::new(DomainSpecificCritic::new()));
```

### Event-Driven Integration

```rust
use common::event_bus::{EventBus, EventSubscription};
use orchestrator::events::{WorkflowEvent, AgentLifecycleEvent, SchedulerEvent};

pub struct WorkflowEventHandler {
    event_bus: EventBus,
    external_system: ExternalSystemClient,
}

impl WorkflowEventHandler {
    pub async fn setup_event_subscriptions(&self) -> Result<()> {
        // Subscribe to workflow events
        self.event_bus.subscribe("workflow.started", |event: WorkflowEvent| {
            self.handle_workflow_started(event).await
        }).await?;
        
        self.event_bus.subscribe("workflow.step.completed", |event: WorkflowEvent| {
            self.handle_workflow_step_completed(event).await
        }).await?;
        
        self.event_bus.subscribe("workflow.completed", |event: WorkflowEvent| {
            self.handle_workflow_completed(event).await
        }).await?;
        
        self.event_bus.subscribe("workflow.failed", |event: WorkflowEvent| {
            self.handle_workflow_failed(event).await
        }).await?;
        
        // Subscribe to agent lifecycle events
        self.event_bus.subscribe("agent.health.degraded", |event: AgentLifecycleEvent| {
            self.handle_agent_health_issue(event).await
        }).await?;
        
        // Subscribe to scheduler events
        self.event_bus.subscribe("scheduler.job.failed", |event: SchedulerEvent| {
            self.handle_background_job_failure(event).await
        }).await?;
        
        Ok(())
    }
    
    async fn handle_workflow_started(&self, event: WorkflowEvent) -> Result<()> {
        // Notify external systems
        self.external_system.notify_workflow_started(
            &event.workflow_id,
            &event.user_input,
            event.priority
        ).await?;
        
        // Start monitoring
        self.start_workflow_monitoring(&event.workflow_id).await?;
        
        Ok(())
    }
    
    async fn handle_workflow_completed(&self, event: WorkflowEvent) -> Result<()> {
        // Process results
        if let Some(output) = &event.output {
            self.process_workflow_results(&event.workflow_id, output).await?;
        }
        
        // Update external systems
        self.external_system.update_workflow_status(
            &event.workflow_id,
            "completed",
            event.steps_completed
        ).await?;
        
        // Collect performance metrics
        self.collect_workflow_metrics(&event).await?;
        
        Ok(())
    }
    
    async fn handle_agent_health_issue(&self, event: AgentLifecycleEvent) -> Result<()> {
        // Alert operations team
        self.send_alert(&format!(
            "Agent {} health degraded: {}",
            event.agent_id,
            event.health_status
        )).await?;
        
        // Attempt automated recovery
        if event.health_status.is_recoverable() {
            self.trigger_agent_recovery(&event.agent_id).await?;
        }
        
        Ok(())
    }
}
```

### Production Deployment Integration

```rust
use orchestrator::{
    reliability::{CircuitBreaker, RetryPolicy, TimeoutManager},
    resources::{ResourceMonitor, ResourceBudget},
};

pub struct ProductionOrchestrator {
    orchestrator: AgentOrchestrator,
    circuit_breaker: CircuitBreaker,
    resource_monitor: ResourceMonitor,
    metrics_collector: MetricsCollector,
}

impl ProductionOrchestrator {
    pub async fn new() -> Result<Self> {
        // Configure circuit breaker for reliability
        let circuit_breaker = CircuitBreaker::new(
            CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 3,
                timeout: Duration::from_secs(60),
                half_open_max_calls: 2,
            }
        );
        
        // Configure resource monitoring
        let resource_budget = ResourceBudget::new()
            .memory_limit(2 * 1024 * 1024 * 1024) // 2GB
            .cpu_time_limit(Duration::from_secs(600)) // 10 minutes
            .max_file_handles(500)
            .max_network_connections(100);
            
        let resource_monitor = ResourceMonitor::new(resource_budget);
        
        // Configure metrics collection
        let metrics_collector = MetricsCollector::new()
            .with_prometheus_endpoint("0.0.0.0:9090")
            .with_collection_interval(Duration::from_secs(15));
        
        // Initialize orchestrator with production settings
        let orchestrator = AgentOrchestrator::new(
            Self::production_system_config(),
            Self::production_orchestrator_config(),
            event_publisher
        ).await?;
        
        Ok(Self {
            orchestrator,
            circuit_breaker,
            resource_monitor,
            metrics_collector,
        })
    }
    
    pub async fn execute_workflow_with_protection(
        &self,
        request: WorkflowRequest
    ) -> Result<WorkflowResult> {
        // Check resource availability
        self.resource_monitor.check_availability().await?;
        
        // Execute through circuit breaker
        self.circuit_breaker.execute(|| {
            self.orchestrator.execute_workflow(request.clone())
        }).await
    }
    
    pub async fn start_monitoring(&self) -> Result<()> {
        // Start resource monitoring
        tokio::spawn({
            let monitor = self.resource_monitor.clone();
            async move {
                monitor.start_monitoring().await
            }
        });
        
        // Start metrics collection
        tokio::spawn({
            let collector = self.metrics_collector.clone();
            async move {
                collector.start_collection().await
            }
        });
        
        // Start health checks
        tokio::spawn({
            let orchestrator = self.orchestrator.clone();
            async move {
                loop {
                    let health = orchestrator.get_system_health().await;
                    if health.is_critical() {
                        // Trigger alerts
                        eprintln!("🚨 Critical system health issue: {:?}", health);
                    }
                    
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        });
        
        Ok(())
    }
    
    fn production_system_config() -> SystemConfig {
        SystemConfig {
            max_concurrent_agents: 50,
            agent_timeout: Duration::from_secs(120),
            enable_health_monitoring: true,
            health_check_interval: Duration::from_secs(15),
            event_bus_capacity: 10000,
            enable_circuit_breaker: true,
            circuit_breaker_failure_threshold: 3,
            enable_metrics_collection: true,
            log_level: "info".to_string(),
            enable_profiling: false, // Disable in production for performance
            ..Default::default()
        }
    }
    
    fn production_orchestrator_config() -> OrchestratorConfig {
        OrchestratorConfig {
            workflow_timeout: Duration::from_secs(1800), // 30 minutes
            max_concurrent_workflows: 10,
            enable_background_tasks: true,
            background_task_interval: Duration::from_secs(60),
            enable_metrics_collection: true,
            enable_workflow_history: true,
            workflow_history_retention: Duration::from_days(7),
            agent_failure_recovery_strategy: "circuit_breaker".to_string(),
            enable_distributed_coordination: true, // For multi-instance
            ..Default::default()
        }
    }
}
```

## 🔧 Configuration Management

### Environment-based Configuration

```rust
use orchestrator::{SystemConfig, OrchestratorConfig};
use std::env;

pub fn load_config_from_environment() -> Result<(SystemConfig, OrchestratorConfig)> {
    let system_config = SystemConfig {
        max_concurrent_agents: env::var("MAGRAY_MAX_AGENTS")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10),
            
        agent_timeout: Duration::from_secs(
            env::var("MAGRAY_AGENT_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30)
        ),
        
        enable_health_monitoring: env::var("MAGRAY_HEALTH_MONITORING")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
            
        event_bus_capacity: env::var("MAGRAY_EVENT_BUS_SIZE")
            .unwrap_or_else(|_| "1000".to_string())
            .parse()
            .unwrap_or(1000),
            
        // Resource limits from environment
        resource_limits: ResourceBudget::new()
            .memory_limit(
                env::var("MAGRAY_MEMORY_LIMIT")
                    .unwrap_or_else(|_| "1073741824".to_string()) // 1GB
                    .parse()
                    .unwrap_or(1073741824)
            )
            .cpu_time_limit(Duration::from_secs(
                env::var("MAGRAY_CPU_TIMEOUT")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300)
            )),
            
        ..Default::default()
    };
    
    let orchestrator_config = OrchestratorConfig {
        workflow_timeout: Duration::from_secs(
            env::var("MAGRAY_WORKFLOW_TIMEOUT")
                .unwrap_or_else(|_| "600".to_string())
                .parse()
                .unwrap_or(600)
        ),
        
        max_concurrent_workflows: env::var("MAGRAY_MAX_WORKFLOWS")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5),
            
        enable_background_tasks: env::var("MAGRAY_BACKGROUND_TASKS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
            
        enable_metrics_collection: env::var("MAGRAY_METRICS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true),
            
        ..Default::default()
    };
    
    Ok((system_config, orchestrator_config))
}

// Configuration validation
pub fn validate_configuration(
    system_config: &SystemConfig,
    orchestrator_config: &OrchestratorConfig
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    
    // Validate system configuration
    if system_config.max_concurrent_agents > 100 {
        warnings.push("High concurrent agent count may impact performance".to_string());
    }
    
    if system_config.agent_timeout < Duration::from_secs(10) {
        warnings.push("Very short agent timeout may cause premature failures".to_string());
    }
    
    // Validate orchestrator configuration
    if orchestrator_config.workflow_timeout > Duration::from_secs(3600) {
        warnings.push("Very long workflow timeout may consume excessive resources".to_string());
    }
    
    if orchestrator_config.max_concurrent_workflows > 20 {
        warnings.push("High concurrent workflow count may impact system stability".to_string());
    }
    
    Ok(warnings)
}
```

### Configuration Files

```toml
# /etc/magray/agent-config.toml

[system]
max_concurrent_agents = 20
agent_timeout_seconds = 60
enable_health_monitoring = true
health_check_interval_seconds = 30
event_bus_capacity = 5000
log_level = "info"

[system.resource_limits]
memory_limit_bytes = 2147483648  # 2GB
cpu_time_limit_seconds = 600     # 10 minutes
max_file_handles = 500
max_network_connections = 100

[orchestrator]
workflow_timeout_seconds = 1200  # 20 minutes
max_concurrent_workflows = 10
enable_background_tasks = true
background_task_interval_seconds = 120
enable_metrics_collection = true
enable_workflow_history = true
workflow_history_retention_days = 7

[agents.intent_analyzer]
model_type = "local"  # local | remote | custom
confidence_threshold = 0.7
max_input_length = 10000
enable_context_memory = true

[agents.planner]
planning_strategy = "hierarchical"  # linear | hierarchical | adaptive
max_plan_depth = 10
enable_plan_optimization = true
resource_estimation = true

[agents.executor]
execution_strategy = "sequential"  # sequential | parallel | adaptive
enable_rollback = true
max_rollback_depth = 5
enable_saga_pattern = true

[agents.critic]
evaluation_criteria = ["correctness", "completeness", "efficiency"]
min_quality_threshold = 0.8
enable_improvement_suggestions = true

[agents.scheduler]
max_background_jobs = 50
job_timeout_seconds = 300
enable_job_persistence = true
cleanup_interval_seconds = 3600

[reliability]
enable_circuit_breaker = true
circuit_breaker_failure_threshold = 5
circuit_breaker_recovery_timeout_seconds = 60
retry_policy = "exponential_backoff"
max_retry_attempts = 3

[monitoring]
enable_prometheus_metrics = true
prometheus_port = 9090
metrics_collection_interval_seconds = 15
enable_health_endpoint = true
health_endpoint_port = 8080
```

## 📊 Monitoring & Observability

### Metrics Collection

```rust
use orchestrator::monitoring::{MetricsCollector, MetricType};
use prometheus::{Counter, Histogram, Gauge, Registry};

pub struct AgentMetricsCollector {
    workflow_counter: Counter,
    workflow_duration: Histogram,
    active_agents: Gauge,
    agent_health: Gauge,
    registry: Registry,
}

impl AgentMetricsCollector {
    pub fn new() -> Result<Self> {
        let registry = Registry::new();
        
        let workflow_counter = Counter::new(
            "magray_workflows_total",
            "Total number of workflows executed"
        )?;
        registry.register(Box::new(workflow_counter.clone()))?;
        
        let workflow_duration = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "magray_workflow_duration_seconds",
                "Workflow execution duration"
            ).buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0])
        )?;
        registry.register(Box::new(workflow_duration.clone()))?;
        
        let active_agents = Gauge::new(
            "magray_active_agents",
            "Number of currently active agents"
        )?;
        registry.register(Box::new(active_agents.clone()))?;
        
        let agent_health = Gauge::new(
            "magray_agent_health_score",
            "Agent health score (0-1)"
        )?;
        registry.register(Box::new(agent_health.clone()))?;
        
        Ok(Self {
            workflow_counter,
            workflow_duration,
            active_agents,
            agent_health,
            registry,
        })
    }
    
    pub async fn collect_workflow_metrics(&self, event: &WorkflowEvent) {
        match event.event_type.as_str() {
            "workflow.started" => {
                self.workflow_counter.inc();
            },
            "workflow.completed" => {
                if let Some(duration) = event.duration {
                    self.workflow_duration.observe(duration.as_secs_f64());
                }
            },
            _ => {}
        }
    }
    
    pub async fn collect_agent_metrics(&self, orchestrator: &AgentOrchestrator) {
        let agent_stats = orchestrator.get_agent_stats().await;
        
        self.active_agents.set(agent_stats.active_count as f64);
        
        let avg_health = agent_stats.agents.iter()
            .map(|agent| agent.health_score)
            .sum::<f32>() / agent_stats.agents.len() as f32;
            
        self.agent_health.set(avg_health as f64);
    }
    
    pub fn export_metrics(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap()
    }
}

// Usage with HTTP server
pub async fn start_metrics_server(collector: AgentMetricsCollector) -> Result<()> {
    use warp::Filter;
    
    let metrics = warp::path("metrics")
        .map(move || {
            warp::reply::with_header(
                collector.export_metrics(),
                "content-type",
                "text/plain; version=0.0.4; charset=utf-8"
            )
        });
    
    let health = warp::path("health")
        .map(|| warp::reply::json(&serde_json::json!({"status": "healthy"})));
    
    let routes = metrics.or(health);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 9090))
        .await;
        
    Ok(())
}
```

### Health Monitoring

```rust
use orchestrator::{AgentOrchestrator, AgentHealthStatus};
use std::time::{Duration, Instant};

pub struct HealthMonitor {
    orchestrator: AgentOrchestrator,
    check_interval: Duration,
    alert_thresholds: HealthThresholds,
}

#[derive(Debug)]
pub struct HealthThresholds {
    critical_response_time: Duration,
    warning_response_time: Duration,
    max_failure_rate: f32,
    min_success_rate: f32,
}

impl HealthMonitor {
    pub async fn start_monitoring(self) -> Result<()> {
        let mut interval = tokio::time::interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            let health_report = self.perform_health_check().await?;
            
            if health_report.is_critical() {
                self.handle_critical_health(&health_report).await?;
            } else if health_report.has_warnings() {
                self.handle_health_warnings(&health_report).await?;
            }
            
            self.log_health_metrics(&health_report).await?;
        }
    }
    
    async fn perform_health_check(&self) -> Result<HealthReport> {
        let start_time = Instant::now();
        
        // Test basic orchestrator responsiveness
        let ping_result = self.orchestrator.ping().await;
        let response_time = start_time.elapsed();
        
        // Get detailed agent health
        let agent_health = self.orchestrator.get_detailed_health().await?;
        
        // Check resource usage
        let resource_usage = self.orchestrator.get_resource_usage().await?;
        
        // Test workflow execution
        let workflow_test = self.test_sample_workflow().await;
        
        Ok(HealthReport {
            timestamp: chrono::Utc::now(),
            overall_status: self.determine_overall_status(&ping_result, &agent_health, &resource_usage),
            response_time,
            agent_health,
            resource_usage,
            workflow_test_result: workflow_test,
            recommendations: self.generate_health_recommendations(&agent_health, &resource_usage),
        })
    }
    
    async fn test_sample_workflow(&self) -> WorkflowTestResult {
        let test_request = WorkflowRequest {
            user_input: "health_check_test_workflow".to_string(),
            context: Some("system_health_monitoring".to_string()),
            priority: TaskPriority::Low,
            dry_run: true,
            timeout_ms: Some(5000), // 5 seconds
            config_overrides: None,
        };
        
        let start_time = Instant::now();
        let result = self.orchestrator.execute_workflow(test_request).await;
        let duration = start_time.elapsed();
        
        WorkflowTestResult {
            success: result.is_ok(),
            duration,
            error: result.err().map(|e| e.to_string()),
        }
    }
    
    async fn handle_critical_health(&self, report: &HealthReport) -> Result<()> {
        // Log critical alert
        log::error!("🚨 CRITICAL: Multi-agent system health critical: {:?}", report);
        
        // Send external alerts
        self.send_critical_alert(report).await?;
        
        // Attempt automatic recovery
        if report.agent_health.has_failed_agents() {
            log::info!("Attempting automatic agent recovery...");
            
            for agent_id in report.agent_health.failed_agents() {
                match self.orchestrator.restart_agent(&agent_id).await {
                    Ok(_) => log::info!("✅ Successfully restarted agent: {}", agent_id),
                    Err(e) => log::error!("❌ Failed to restart agent {}: {}", agent_id, e),
                }
            }
        }
        
        // If resource exhaustion, trigger cleanup
        if report.resource_usage.is_exhausted() {
            log::info!("Triggering emergency resource cleanup...");
            self.orchestrator.emergency_cleanup().await?;
        }
        
        Ok(())
    }
    
    async fn send_critical_alert(&self, report: &HealthReport) -> Result<()> {
        // Implementation depends on your alerting system
        // Examples: PagerDuty, Slack, email, SMS, etc.
        
        let alert = CriticalAlert {
            title: "MAGRAY CLI Multi-Agent System Critical Health Issue".to_string(),
            description: format!("Critical health issue detected: {:?}", report),
            severity: "critical".to_string(),
            timestamp: report.timestamp,
            affected_components: report.agent_health.unhealthy_agents(),
            recommended_actions: report.recommendations.clone(),
        };
        
        // Send to alerting system
        // self.alert_client.send_alert(alert).await?;
        
        Ok(())
    }
}
```

## 🧪 Testing Integration

### Integration Testing

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use orchestrator::testing::{TestAgentOrchestrator, MockEventBus};
    
    #[tokio::test]
    async fn test_basic_workflow_execution() {
        let (event_bus, _) = MockEventBus::new().await;
        let orchestrator = TestAgentOrchestrator::new(event_bus).await.unwrap();
        
        let request = WorkflowRequest {
            user_input: "Create a simple file with test content".to_string(),
            context: None,
            priority: TaskPriority::Normal,
            dry_run: true,
            timeout_ms: Some(30000),
            config_overrides: None,
        };
        
        let result = orchestrator.execute_workflow(request).await;
        
        assert!(result.is_ok());
        let workflow_result = result.unwrap();
        assert!(workflow_result.success);
        assert!(workflow_result.steps_completed > 0);
    }
    
    #[tokio::test]
    async fn test_workflow_error_handling() {
        let orchestrator = TestAgentOrchestrator::new_with_faulty_agents().await.unwrap();
        
        let request = WorkflowRequest {
            user_input: "Execute impossible task".to_string(),
            context: None,
            priority: TaskPriority::Normal,
            dry_run: false,
            timeout_ms: Some(10000),
            config_overrides: None,
        };
        
        let result = orchestrator.execute_workflow(request).await;
        
        // Should handle errors gracefully
        match result {
            Ok(workflow_result) => {
                assert!(!workflow_result.success);
                assert!(workflow_result.error_message.is_some());
            },
            Err(e) => {
                // Error should be descriptive
                assert!(!e.to_string().is_empty());
            }
        }
    }
    
    #[tokio::test]
    async fn test_concurrent_workflow_execution() {
        let orchestrator = TestAgentOrchestrator::new_with_concurrency().await.unwrap();
        
        let requests = vec![
            WorkflowRequest {
                user_input: "Task 1".to_string(),
                context: None,
                priority: TaskPriority::Normal,
                dry_run: true,
                timeout_ms: Some(15000),
                config_overrides: None,
            },
            WorkflowRequest {
                user_input: "Task 2".to_string(),
                context: None,
                priority: TaskPriority::High,
                dry_run: true,
                timeout_ms: Some(15000),
                config_overrides: None,
            },
            WorkflowRequest {
                user_input: "Task 3".to_string(),
                context: None,
                priority: TaskPriority::Low,
                dry_run: true,
                timeout_ms: Some(15000),
                config_overrides: None,
            },
        ];
        
        // Execute workflows concurrently
        let results = futures::future::join_all(
            requests.into_iter().map(|req| orchestrator.execute_workflow(req))
        ).await;
        
        // All workflows should complete
        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_ok());
        }
    }
    
    #[tokio::test]
    async fn test_agent_recovery() {
        let orchestrator = TestAgentOrchestrator::new().await.unwrap();
        
        // Simulate agent failure
        orchestrator.simulate_agent_failure("intent_analyzer").await.unwrap();
        
        // Check system detects failure
        let health = orchestrator.get_system_health().await.unwrap();
        assert!(health.has_unhealthy_agents());
        
        // Trigger recovery
        orchestrator.recover_failed_agents().await.unwrap();
        
        // Verify recovery
        let health_after_recovery = orchestrator.get_system_health().await.unwrap();
        assert!(!health_after_recovery.has_unhealthy_agents());
    }
}
```

### Performance Testing

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::{Duration, Instant};
    
    #[tokio::test]
    async fn test_workflow_latency_under_load() {
        let orchestrator = TestAgentOrchestrator::new_with_performance_config().await.unwrap();
        
        let num_workflows = 100;
        let concurrent_limit = 10;
        
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrent_limit));
        let latencies = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        
        let mut tasks = Vec::new();
        
        for i in 0..num_workflows {
            let orchestrator = orchestrator.clone();
            let semaphore = semaphore.clone();
            let latencies = latencies.clone();
            
            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                
                let request = WorkflowRequest {
                    user_input: format!("Performance test workflow {}", i),
                    context: None,
                    priority: TaskPriority::Normal,
                    dry_run: true,
                    timeout_ms: Some(30000),
                    config_overrides: None,
                };
                
                let start = Instant::now();
                let result = orchestrator.execute_workflow(request).await;
                let latency = start.elapsed();
                
                if result.is_ok() {
                    latencies.lock().await.push(latency);
                }
                
                result
            });
            
            tasks.push(task);
        }
        
        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;
        
        // Analyze results
        let successful_workflows = results.iter()
            .filter(|r| r.as_ref().unwrap().is_ok())
            .count();
            
        let latencies = latencies.lock().await;
        
        assert!(successful_workflows >= num_workflows * 95 / 100); // 95% success rate
        
        if !latencies.is_empty() {
            let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
            let max_latency = *latencies.iter().max().unwrap();
            let min_latency = *latencies.iter().min().unwrap();
            
            println!("Performance Results:");
            println!("  Successful workflows: {}/{}", successful_workflows, num_workflows);
            println!("  Average latency: {:?}", avg_latency);
            println!("  Min latency: {:?}", min_latency);
            println!("  Max latency: {:?}", max_latency);
            
            // Performance assertions
            assert!(avg_latency < Duration::from_secs(5)); // Average under 5 seconds
            assert!(max_latency < Duration::from_secs(15)); // Max under 15 seconds
        }
    }
    
    #[tokio::test]
    async fn test_memory_usage_stability() {
        let orchestrator = TestAgentOrchestrator::new().await.unwrap();
        
        // Baseline memory usage
        let initial_memory = get_process_memory_usage();
        
        // Execute many workflows
        for i in 0..50 {
            let request = WorkflowRequest {
                user_input: format!("Memory test workflow {}", i),
                context: Some("Large context data with repeated content".repeat(100)),
                priority: TaskPriority::Normal,
                dry_run: true,
                timeout_ms: Some(10000),
                config_overrides: None,
            };
            
            let _ = orchestrator.execute_workflow(request).await;
            
            // Periodic memory checks
            if i % 10 == 0 {
                let current_memory = get_process_memory_usage();
                let memory_growth = current_memory - initial_memory;
                
                // Memory growth should be reasonable
                assert!(memory_growth < 500 * 1024 * 1024); // Less than 500MB growth
            }
        }
        
        // Force garbage collection and check final memory
        orchestrator.cleanup_resources().await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let final_memory = get_process_memory_usage();
        let total_growth = final_memory - initial_memory;
        
        // Memory should not grow excessively
        assert!(total_growth < 200 * 1024 * 1024); // Less than 200MB final growth
    }
    
    fn get_process_memory_usage() -> usize {
        // Implementation depends on your system
        // This is a placeholder
        0
    }
}
```

## 🔗 Related Documentation

- [Agent API Documentation](../api/agents/README.md) - Detailed agent API reference
- [Event-Driven Architecture](event-driven-architecture.md) - EventBus integration patterns
- [Performance Tuning](performance-tuning.md) - Performance optimization guide  
- [Security Configuration](security-configuration.md) - Security integration
- [Production Deployment](production-deployment.md) - Production deployment guide

---

**Guide Version**: 1.0  
**Multi-Agent System Version**: P1.1 (95% Complete)  
**Production Ready**: ✅ Yes  
**Last Updated**: 2025-08-13