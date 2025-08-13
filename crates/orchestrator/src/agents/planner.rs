use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::intent_analyzer::{Intent, IntentType};

// Health monitoring integration
use crate::reliability::health::{HealthChecker, HealthReport, HealthStatus};

/// Represents a structured action plan with ordered steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionPlan {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub steps: Vec<ActionStep>,
    pub estimated_duration: std::time::Duration,
    pub resource_requirements: ResourceRequirements,
    pub dependencies: Vec<Uuid>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Individual step in an action plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionStep {
    pub id: Uuid,
    pub step_type: ActionStepType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<Uuid>,
    pub expected_duration: std::time::Duration,
    pub retry_policy: RetryPolicy,
    pub validation_rules: Vec<ValidationRule>,
}

/// Types of action steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    Wait { duration: std::time::Duration },
}

/// Memory operation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryOperationType {
    Store,
    Search,
    Update,
    Delete,
}

/// User interaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionType {
    Confirmation,
    Input,
    Choice,
    Information,
}

/// Resource requirements for plan execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRequirements {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_space_mb: u64,
    pub network_required: bool,
    pub tools_required: Vec<String>,
    pub permissions_required: Vec<String>,
}

/// Retry policy for action steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_strategy: BackoffStrategy,
    pub retry_conditions: Vec<RetryCondition>,
}

/// Backoff strategies for retries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackoffStrategy {
    Fixed(std::time::Duration),
    Exponential {
        initial: std::time::Duration,
        multiplier: f64,
    },
    Linear {
        initial: std::time::Duration,
        increment: std::time::Duration,
    },
}

/// Conditions that trigger retries
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetryCondition {
    NetworkError,
    TemporaryFailure,
    RateLimited,
    ResourceUnavailable,
}

/// Validation rules for step execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub description: String,
    pub required: bool,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationRuleType {
    FileExists { path: String },
    ToolAvailable { tool_name: String },
    PermissionGranted { permission: String },
    ResourceAvailable { resource: String },
    Custom { validation_function: String },
}

/// Trait for planning functionality
#[async_trait]
pub trait PlannerTrait: Send + Sync {
    /// Build an action plan from an intent
    async fn build_plan(&self, intent: &Intent) -> Result<ActionPlan>;

    /// Validate a plan before execution
    async fn validate_plan(&self, plan: &ActionPlan) -> Result<PlanValidationResult>;

    /// Optimize a plan for efficiency
    async fn optimize_plan(&mut self, plan: ActionPlan) -> Result<ActionPlan>;

    /// Check tool availability for plan
    async fn check_tool_availability(&self, plan: &ActionPlan) -> Result<ToolAvailabilityReport>;
}

/// Result of plan validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidationResult {
    pub is_valid: bool,
    pub errors: Vec<PlanValidationError>,
    pub warnings: Vec<String>,
    pub estimated_success_probability: f64,
}

/// Plan validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidationError {
    pub step_id: Option<Uuid>,
    pub error_type: PlanValidationErrorType,
    pub message: String,
}

/// Types of plan validation errors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanValidationErrorType {
    MissingTool,
    InsufficientPermissions,
    CircularDependency,
    ResourceConflict,
    InvalidParameters,
    UnreachableStep,
}

/// Tool availability report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAvailabilityReport {
    pub available_tools: Vec<String>,
    pub unavailable_tools: Vec<String>,
    pub tool_status: HashMap<String, ToolStatus>,
}

/// Status of individual tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub version: String,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub health: ToolHealth,
}

/// Tool health indicators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ToolHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Planner implementation
pub struct Planner {
    agent_id: Uuid,
    available_tools: HashMap<String, ToolStatus>,
    planning_strategies: HashMap<IntentType, PlanningStrategy>,
    // Health monitoring fields
    last_heartbeat: Arc<RwLock<Option<DateTime<Utc>>>>,
    error_count: Arc<RwLock<u32>>,
    plans_created: Arc<RwLock<u64>>,
    start_time: Instant,
}

/// Planning strategies for different intent types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlanningStrategy {
    Simple,
    Sequential,
    Parallel,
    Conditional,
    Iterative,
}

impl Planner {
    /// Create new Planner instance
    pub fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert(
            IntentType::ExecuteTool {
                tool_name: String::new(),
            },
            PlanningStrategy::Simple,
        );
        strategies.insert(
            IntentType::AskQuestion {
                question: String::new(),
            },
            PlanningStrategy::Sequential,
        );

        Self {
            agent_id: Uuid::new_v4(),
            available_tools: HashMap::new(),
            planning_strategies: strategies,
            last_heartbeat: Arc::new(RwLock::new(Some(Utc::now()))),
            error_count: Arc::new(RwLock::new(0)),
            plans_created: Arc::new(RwLock::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Start automatic heartbeat loop for health monitoring
    /// This prevents timeout issues by sending heartbeat every 30 seconds
    pub fn start_heartbeat_loop(&self) {
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let agent_id = self.agent_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Update heartbeat timestamp
                {
                    let mut heartbeat = last_heartbeat.write().await;
                    *heartbeat = Some(Utc::now());
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    agent_type = "Planner",
                    "Heartbeat sent"
                );
            }
        });

        tracing::info!(
            agent_id = %self.agent_id,
            agent_type = "Planner",
            "Heartbeat loop started with 30s interval"
        );
    }

    /// Create default retry policy
    fn default_retry_policy() -> RetryPolicy {
        RetryPolicy {
            max_retries: 3,
            backoff_strategy: BackoffStrategy::Exponential {
                initial: std::time::Duration::from_millis(100),
                multiplier: 2.0,
            },
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::TemporaryFailure,
            ],
        }
    }

    /// Create default resource requirements
    fn default_resource_requirements() -> ResourceRequirements {
        ResourceRequirements {
            cpu_cores: 1,
            memory_mb: 256,
            disk_space_mb: 10,
            network_required: false,
            tools_required: vec![],
            permissions_required: vec![],
        }
    }

    /// Plan for tool execution intent
    fn plan_tool_execution(&self, intent: &Intent, tool_name: &str) -> Result<ActionPlan> {
        let step = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::ToolExecution {
                tool_name: tool_name.to_string(),
                arguments: HashMap::new(),
            },
            parameters: HashMap::new(),
            dependencies: vec![],
            expected_duration: std::time::Duration::from_secs(10),
            retry_policy: Self::default_retry_policy(),
            validation_rules: vec![ValidationRule {
                rule_type: ValidationRuleType::ToolAvailable {
                    tool_name: tool_name.to_string(),
                },
                description: format!("Tool '{}' must be available", tool_name),
                required: true,
            }],
        };

        let mut resource_reqs = Self::default_resource_requirements();
        resource_reqs.tools_required.push(tool_name.to_string());

        Ok(ActionPlan {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            steps: vec![step],
            estimated_duration: std::time::Duration::from_secs(10),
            resource_requirements: resource_reqs,
            dependencies: vec![],
            metadata: HashMap::new(),
        })
    }

    /// Plan for question intent
    fn plan_question(&self, intent: &Intent, question: &str) -> Result<ActionPlan> {
        // Simple question answering plan
        let memory_search_step = ActionStep {
            id: Uuid::new_v4(),
            step_type: ActionStepType::MemoryOperation {
                operation_type: MemoryOperationType::Search,
                query: question.to_string(),
            },
            parameters: HashMap::new(),
            dependencies: vec![],
            expected_duration: std::time::Duration::from_secs(5),
            retry_policy: Self::default_retry_policy(),
            validation_rules: vec![],
        };

        Ok(ActionPlan {
            id: Uuid::new_v4(),
            intent_id: intent.id,
            steps: vec![memory_search_step],
            estimated_duration: std::time::Duration::from_secs(5),
            resource_requirements: Self::default_resource_requirements(),
            dependencies: vec![],
            metadata: HashMap::new(),
        })
    }
}

#[async_trait]
impl PlannerTrait for Planner {
    async fn build_plan(&self, intent: &Intent) -> Result<ActionPlan> {
        tracing::debug!("Building plan for intent: {:?}", intent.intent_type);

        let plan = match &intent.intent_type {
            IntentType::ExecuteTool { tool_name } => self.plan_tool_execution(intent, tool_name)?,
            IntentType::AskQuestion { question } => self.plan_question(intent, question)?,
            IntentType::FileOperation { operation, path } => {
                // Simple file operation plan
                let step = ActionStep {
                    id: Uuid::new_v4(),
                    step_type: ActionStepType::ToolExecution {
                        tool_name: format!("file_{}", operation),
                        arguments: HashMap::from([(
                            "path".to_string(),
                            serde_json::Value::String(path.clone()),
                        )]),
                    },
                    parameters: HashMap::new(),
                    dependencies: vec![],
                    expected_duration: std::time::Duration::from_secs(5),
                    retry_policy: Self::default_retry_policy(),
                    validation_rules: vec![],
                };

                ActionPlan {
                    id: Uuid::new_v4(),
                    intent_id: intent.id,
                    steps: vec![step],
                    estimated_duration: std::time::Duration::from_secs(5),
                    resource_requirements: Self::default_resource_requirements(),
                    dependencies: vec![],
                    metadata: HashMap::new(),
                }
            }
            IntentType::MemoryOperation { operation } => {
                let step = ActionStep {
                    id: Uuid::new_v4(),
                    step_type: ActionStepType::MemoryOperation {
                        operation_type: match operation.as_str() {
                            "store" => MemoryOperationType::Store,
                            "search" => MemoryOperationType::Search,
                            "update" => MemoryOperationType::Update,
                            "delete" => MemoryOperationType::Delete,
                            _ => MemoryOperationType::Search,
                        },
                        query: operation.clone(),
                    },
                    parameters: HashMap::new(),
                    dependencies: vec![],
                    expected_duration: std::time::Duration::from_secs(3),
                    retry_policy: Self::default_retry_policy(),
                    validation_rules: vec![],
                };

                ActionPlan {
                    id: Uuid::new_v4(),
                    intent_id: intent.id,
                    steps: vec![step],
                    estimated_duration: std::time::Duration::from_secs(3),
                    resource_requirements: Self::default_resource_requirements(),
                    dependencies: vec![],
                    metadata: HashMap::new(),
                }
            }
            _ => {
                // Default plan for unknown intents
                let step = ActionStep {
                    id: Uuid::new_v4(),
                    step_type: ActionStepType::UserInteraction {
                        interaction_type: InteractionType::Information,
                        prompt: "Unable to create plan for this intent".to_string(),
                    },
                    parameters: HashMap::new(),
                    dependencies: vec![],
                    expected_duration: std::time::Duration::from_secs(1),
                    retry_policy: Self::default_retry_policy(),
                    validation_rules: vec![],
                };

                ActionPlan {
                    id: Uuid::new_v4(),
                    intent_id: intent.id,
                    steps: vec![step],
                    estimated_duration: std::time::Duration::from_secs(1),
                    resource_requirements: Self::default_resource_requirements(),
                    dependencies: vec![],
                    metadata: HashMap::new(),
                }
            }
        };

        tracing::debug!("Created plan {} with {} steps", plan.id, plan.steps.len());

        Ok(plan)
    }

    async fn validate_plan(&self, plan: &ActionPlan) -> Result<PlanValidationResult> {
        let mut errors = vec![];
        let mut warnings = vec![];

        // Check for circular dependencies
        // TODO: Implement proper dependency cycle detection

        // Check tool availability
        for step in &plan.steps {
            if let ActionStepType::ToolExecution { tool_name, .. } = &step.step_type {
                if !self.available_tools.contains_key(tool_name) {
                    errors.push(PlanValidationError {
                        step_id: Some(step.id),
                        error_type: PlanValidationErrorType::MissingTool,
                        message: format!("Tool '{}' is not available", tool_name),
                    });
                }
            }
        }

        // Check resource requirements
        if plan.resource_requirements.memory_mb > 1024 {
            warnings.push("Plan requires significant memory usage".to_string());
        }

        let is_valid = errors.is_empty();
        let estimated_success_probability = if is_valid { 0.85 } else { 0.3 };

        Ok(PlanValidationResult {
            is_valid,
            errors,
            warnings,
            estimated_success_probability,
        })
    }

    async fn optimize_plan(&mut self, plan: ActionPlan) -> Result<ActionPlan> {
        // Basic optimization: parallel execution where possible
        // TODO: Implement proper dependency analysis and parallelization

        tracing::debug!("Optimizing plan {}", plan.id);

        // For now, just return the plan as-is
        // In a real implementation, we would:
        // - Analyze step dependencies
        // - Group independent steps for parallel execution
        // - Optimize resource usage
        // - Cache intermediate results

        Ok(plan)
    }

    async fn check_tool_availability(&self, plan: &ActionPlan) -> Result<ToolAvailabilityReport> {
        let mut available_tools = vec![];
        let mut unavailable_tools = vec![];
        let mut tool_status = HashMap::new();

        for step in &plan.steps {
            if let ActionStepType::ToolExecution { tool_name, .. } = &step.step_type {
                if self.available_tools.contains_key(tool_name) {
                    available_tools.push(tool_name.clone());
                    tool_status.insert(
                        tool_name.clone(),
                        ToolStatus {
                            available: true,
                            version: "1.0.0".to_string(),
                            last_check: chrono::Utc::now(),
                            health: ToolHealth::Healthy,
                        },
                    );
                } else {
                    unavailable_tools.push(tool_name.clone());
                    tool_status.insert(
                        tool_name.clone(),
                        ToolStatus {
                            available: false,
                            version: "unknown".to_string(),
                            last_check: chrono::Utc::now(),
                            health: ToolHealth::Unknown,
                        },
                    );
                }
            }
        }

        Ok(ToolAvailabilityReport {
            available_tools,
            unavailable_tools,
            tool_status,
        })
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

/// HealthChecker implementation for Planner
#[async_trait]
impl HealthChecker for Planner {
    async fn check_health(&self) -> Result<HealthReport> {
        let last_heartbeat = *self.last_heartbeat.read().await;
        let error_count = *self.error_count.read().await;
        let plans_created = *self.plans_created.read().await;
        let uptime = self.start_time.elapsed().as_secs();

        // Determine health status based on errors and activity
        let status = if error_count > 15 {
            HealthStatus::Unhealthy {
                reason: format!("High error count: {}", error_count),
            }
        } else if error_count > 8 {
            HealthStatus::Degraded {
                reason: format!("Moderate error count: {}", error_count),
            }
        } else {
            HealthStatus::Healthy
        };

        Ok(HealthReport {
            agent_id: self.agent_id,
            agent_name: "Planner".to_string(),
            agent_type: "Planner".to_string(),
            status,
            timestamp: Utc::now(),
            last_heartbeat,
            response_time_ms: Some(20), // Plan generation can take longer
            memory_usage_mb: Some(80),  // Estimated memory for planning logic
            cpu_usage_percent: Some(5.0), // Planning involves more CPU
            active_tasks: plans_created as u32,
            error_count,
            restart_count: 0, // Track restarts in future implementation
            uptime_seconds: uptime,
            metadata: serde_json::json!({
                "plans_created": plans_created,
                "available_tools_count": self.available_tools.len(),
                "planning_strategies_count": self.planning_strategies.len()
            }),
        })
    }

    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    fn agent_name(&self) -> &str {
        "Planner"
    }

    fn agent_type(&self) -> &str {
        "Planner"
    }

    async fn heartbeat(&self) -> Result<()> {
        let mut heartbeat = self.last_heartbeat.write().await;
        *heartbeat = Some(Utc::now());
        Ok(())
    }

    fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
        self.last_heartbeat.try_read().ok().and_then(|guard| *guard)
    }

    fn is_healthy(&self) -> bool {
        let error_count = self
            .error_count
            .try_read()
            .map(|guard| *guard)
            .unwrap_or(u32::MAX);
        error_count <= 15
    }

    async fn restart(&self) -> Result<()> {
        // Reset error count and update heartbeat
        {
            let mut error_count = self.error_count.write().await;
            *error_count = 0;
        }
        self.heartbeat().await?;
        tracing::info!("Planner restarted successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::intent_analyzer::{IntentContext, IntentType};

    fn create_test_intent(intent_type: IntentType) -> Intent {
        Intent {
            id: Uuid::new_v4(),
            intent_type,
            parameters: HashMap::new(),
            confidence: 0.8,
            context: IntentContext {
                session_id: Uuid::new_v4(),
                user_id: Some("test".to_string()),
                timestamp: chrono::Utc::now(),
                environment: HashMap::new(),
                conversation_history: vec![],
            },
        }
    }

    #[tokio::test]
    async fn test_build_tool_execution_plan() {
        let planner = Planner::new();
        let intent = create_test_intent(IntentType::ExecuteTool {
            tool_name: "test_tool".to_string(),
        });

        let plan = planner
            .build_plan(&intent)
            .await
            .expect("Async operation should succeed");

        assert_eq!(plan.intent_id, intent.id);
        assert_eq!(plan.steps.len(), 1);

        match &plan.steps[0].step_type {
            ActionStepType::ToolExecution { tool_name, .. } => {
                assert_eq!(tool_name, "test_tool");
            }
            _ => panic!("Expected ToolExecution step"),
        }
    }

    #[tokio::test]
    async fn test_build_question_plan() {
        let planner = Planner::new();
        let intent = create_test_intent(IntentType::AskQuestion {
            question: "What is the weather?".to_string(),
        });

        let plan = planner
            .build_plan(&intent)
            .await
            .expect("Async operation should succeed");

        assert_eq!(plan.steps.len(), 1);

        match &plan.steps[0].step_type {
            ActionStepType::MemoryOperation {
                operation_type,
                query,
            } => {
                assert_eq!(*operation_type, MemoryOperationType::Search);
                assert_eq!(query, "What is the weather?");
            }
            _ => panic!("Expected MemoryOperation step"),
        }
    }

    #[tokio::test]
    async fn test_validate_plan_with_missing_tool() {
        let planner = Planner::new();
        let intent = create_test_intent(IntentType::ExecuteTool {
            tool_name: "nonexistent_tool".to_string(),
        });

        let plan = planner
            .build_plan(&intent)
            .await
            .expect("Async operation should succeed");
        let validation = planner
            .validate_plan(&plan)
            .await
            .expect("Async operation should succeed");

        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
        assert_eq!(
            validation.errors[0].error_type,
            PlanValidationErrorType::MissingTool
        );
    }

    #[tokio::test]
    async fn test_check_tool_availability() {
        let planner = Planner::new();
        let intent = create_test_intent(IntentType::ExecuteTool {
            tool_name: "test_tool".to_string(),
        });

        let plan = planner
            .build_plan(&intent)
            .await
            .expect("Async operation should succeed");
        let report = planner
            .check_tool_availability(&plan)
            .await
            .expect("Async operation should succeed");

        assert_eq!(report.unavailable_tools.len(), 1);
        assert_eq!(report.unavailable_tools[0], "test_tool");
        assert!(!report.tool_status["test_tool"].available);
    }
}

/// BaseActor implementation for Planner
#[async_trait::async_trait]
impl crate::actors::BaseActor for Planner {
    fn id(&self) -> crate::actors::ActorId {
        crate::actors::ActorId::new()
    }

    fn actor_type(&self) -> &'static str {
        "Planner"
    }

    async fn handle_message(
        &mut self,
        message: crate::actors::ActorMessage,
        _context: &crate::actors::ActorContext,
    ) -> Result<(), crate::actors::ActorError> {
        match message {
            crate::actors::ActorMessage::Agent(agent_msg) => match agent_msg {
                crate::actors::AgentMessage::CreatePlan {
                    intent: _,
                    constraints: _,
                } => {
                    tracing::info!("Received plan creation request");
                    // For BaseActor implementation, just acknowledge
                    Ok(())
                }
                _ => {
                    tracing::warn!("Unsupported agent message type for Planner");
                    Ok(())
                }
            },
            _ => {
                tracing::warn!("Unsupported message type for Planner");
                Ok(())
            }
        }
    }
}
