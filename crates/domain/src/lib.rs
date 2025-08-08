//! Domain Layer - MAGRAY CLI Business Logic
//!
//! Содержит ТОЛЬКО чистую business logic без dependencies на:
//! - Infrastructure (databases, file systems, networks)
//! - Frameworks (web, CLI, UI)
//! - External systems (AI services, third-party APIs)
//!
//! Принципы Clean Architecture:
//! - Entities: core business objects (MemoryRecord, SearchQuery, etc.)
//! - Value Objects: immutable data structures (LayerType, ScoreThreshold)
//! - Repository Abstractions: interfaces для persistence
//! - Business Rules: чистая business logic без side effects

pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

// Re-export core domain types
pub use entities::{EmbeddingVector, MemoryRecord, RecordId, SearchQuery};
pub use errors::{DomainError, DomainResult};
pub use repositories::{EmbeddingRepository, MemoryRepository, SearchRepository};
pub use services::{MemoryDomainService, PromotionDomainService, SearchDomainService};
pub use value_objects::{AccessPattern, LayerType, PromotionCriteria, ScoreThreshold};

/// Domain-specific type aliases
pub type EmbeddingDimensions = usize;
pub type SimilarityScore = f32;
pub type RecordCount = usize;

/// Domain layer: core traits and types for orchestrator and planning

pub mod orchestrator {
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Goal {
        pub title: String,
        pub description: String,
        pub constraints: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PlanStep {
        pub id: String,
        pub description: String,
        pub tool_hint: Option<String>,
        pub deps: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Plan {
        pub steps: Vec<PlanStep>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum StepStatus {
        Pending,
        Running,
        Succeeded,
        Failed { error: String },
        Skipped,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StepResult {
        pub step_id: String,
        pub status: StepStatus,
        pub output: Option<String>,
        pub artifacts: Vec<String>,
    }

    #[async_trait]
    pub trait Planner: Send + Sync {
        async fn create_plan(&self, goal: &Goal) -> anyhow::Result<Plan>;
    }

    #[async_trait]
    pub trait Executor: Send + Sync {
        async fn execute(&self, plan: &Plan) -> anyhow::Result<Vec<StepResult>>;
    }

    #[async_trait]
    pub trait Orchestrator: Send + Sync {
        async fn plan(&self, goal: Goal) -> anyhow::Result<Plan>;
        async fn run(&self, plan: Plan) -> anyhow::Result<Vec<StepResult>>;
        async fn plan_and_run(&self, goal: Goal) -> anyhow::Result<Vec<StepResult>> {
            let plan = self.plan(goal).await?;
            self.run(plan).await
        }
    }
}
