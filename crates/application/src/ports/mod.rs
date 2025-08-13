//! Application Ports
//!
//! Определяет абстракции для Infrastructure layer по принципу Dependency Inversion.
//! Application layer зависит от этих abstractions, а Infrastructure layer их реализует.

pub mod cache_provider;
pub mod embedding_provider;
pub mod llm_provider;
pub mod metrics_collector;
pub mod notification_service;
pub mod search_provider;
pub mod service_container;
pub mod storage_provider;

// Re-export all port traits
pub use cache_provider::*;
pub use embedding_provider::*;
pub use llm_provider::*;
pub use metrics_collector::*;
pub use notification_service::*;
pub use search_provider::*;
pub use service_container::*;
pub use storage_provider::*;
