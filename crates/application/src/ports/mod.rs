//! Application Ports
//!
//! Определяет абстракции для Infrastructure layer по принципу Dependency Inversion.
//! Application layer зависит от этих abstractions, а Infrastructure layer их реализует.

pub mod embedding_provider;
pub mod llm_provider;
pub mod notification_service;
pub mod metrics_collector;
pub mod cache_provider;
pub mod search_provider;

// Re-export all port traits
pub use embedding_provider::*;
pub use llm_provider::*;
pub use notification_service::*;
pub use metrics_collector::*;
pub use cache_provider::*;
pub use search_provider::*;