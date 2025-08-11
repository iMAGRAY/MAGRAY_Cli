//! # Application Layer
//!
//! Реализует Clean Architecture Application Layer с:
//! - Use Cases для бизнес-логики workflows
//! - Application Services для координации
//! - DTOs для передачи данных
//! - Ports для абстракций Infrastructure слоя
//! - Command/Query Separation (CQRS)
//!
//! ## Architecture Design Principles
//!
//! 1. **Use Cases** - инкапсулируют бизнес workflows
//! 2. **Application Services** - координируют domain services
//! 3. **DTOs** - изолируют от domain entities 
//! 4. **Ports** - абстракции для infrastructure
//! 5. **CQRS** - разделение команд и запросов
//!
//! ## Dependency Direction
//!
//! ```
//! Application Layer → Domain Layer (entities, services, repositories)
//! Infrastructure → Application Layer (implements ports)
//! ```

use anyhow::Result;
use async_trait::async_trait;
use domain::orchestrator::{Executor, Goal, Orchestrator, Plan, Planner, StepResult};

pub struct LlmPlanner;

#[async_trait]
impl Planner for LlmPlanner {
    async fn create_plan(&self, goal: &Goal) -> Result<Plan> {
        // TODO: integrate LLM to produce step list; return a minimal single-step plan for now
        Ok(Plan {
            steps: vec![domain::orchestrator::PlanStep {
                id: "step-1".into(),
                description: goal.title.clone(),
                tool_hint: Some("tool".into()),
                deps: vec![],
            }],
        })
    }
}

pub struct TodoExecutor;

#[async_trait]
impl Executor for TodoExecutor {
    async fn execute(&self, plan: &Plan) -> Result<Vec<StepResult>> {
        // TODO: connect to todo::TodoService and map steps to tasks
        Ok(plan
            .steps
            .iter()
            .map(|s| StepResult {
                step_id: s.id.clone(),
                status: domain::orchestrator::StepStatus::Succeeded,
                output: Some(format!("Executed: {}", s.description)),
                artifacts: vec![],
            })
            .collect())
    }
}

pub struct UnifiedOrchestrator<P: Planner, E: Executor> {
    planner: P,
    executor: E,
}

impl<P: Planner, E: Executor> UnifiedOrchestrator<P, E> {
    pub fn new(planner: P, executor: E) -> Self {
        Self { planner, executor }
    }
}

#[async_trait]
impl<P: Planner + Send + Sync, E: Executor + Send + Sync> Orchestrator for UnifiedOrchestrator<P, E> {
    async fn plan(&self, goal: Goal) -> Result<Plan> {
        self.planner.create_plan(&goal).await
    }

    async fn run(&self, plan: Plan) -> Result<Vec<StepResult>> {
        self.executor.execute(&plan).await
    }
}

pub mod dtos;
pub mod errors;
pub mod ports;
pub mod services;
pub mod use_cases;
pub mod cqrs;
pub mod adapters;

pub use errors::ApplicationError;

/// Application layer result type
pub type ApplicationResult<T> = Result<T, ApplicationError>;

/// Request context for tracing and metrics
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: uuid::Uuid,
    pub correlation_id: String,
    pub user_id: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub source: RequestSource,
}

#[derive(Debug, Clone)]
pub enum RequestSource {
    Cli,
    Api,
    Internal,
    System,
}

impl RequestContext {
    pub fn new(source: RequestSource) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            timestamp: std::time::SystemTime::now(),
            source,
        }
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = correlation_id;
        self
    }
}