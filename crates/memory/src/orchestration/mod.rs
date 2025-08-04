// Модуль orchestration для координации компонентов memory системы

mod traits;
mod retry_handler;
mod embedding_coordinator;
mod search_coordinator;
mod health_manager;
mod promotion_coordinator;
mod resource_controller;
mod backup_coordinator;
mod memory_orchestrator;

pub use traits::{
    Coordinator, SearchCoordinator as SearchCoordinatorTrait, 
    EmbeddingCoordinator as EmbeddingCoordinatorTrait,
    PromotionCoordinator as PromotionCoordinatorTrait,
    HealthCoordinator as HealthCoordinatorTrait,
    ResourceCoordinator as ResourceCoordinatorTrait,
    BackupCoordinator as BackupCoordinatorTrait,
};

pub use retry_handler::{RetryHandler, RetryPolicy, RetryResult};
pub use embedding_coordinator::EmbeddingCoordinator;
pub use search_coordinator::SearchCoordinator;
pub use health_manager::HealthManager;
pub use promotion_coordinator::PromotionCoordinator;
pub use resource_controller::ResourceController;
pub use backup_coordinator::BackupCoordinator;
pub use memory_orchestrator::MemoryOrchestrator;