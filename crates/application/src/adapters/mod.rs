//! Adapters for Integration with Memory Services
//!
//! Адаптеры для интеграции Application Layer с существующими memory services,
//! реализуя port/adapter pattern для dependency inversion.

pub mod memory_service_adapter;
pub mod embedding_service_adapter;
pub mod cache_service_adapter;
pub mod search_service_adapter;
pub mod notification_service_adapter;
pub mod metrics_collector_adapter;

// Re-export adapters
pub use memory_service_adapter::*;
pub use embedding_service_adapter::*;
pub use cache_service_adapter::*;
pub use search_service_adapter::*;
pub use notification_service_adapter::*;
pub use metrics_collector_adapter::*;