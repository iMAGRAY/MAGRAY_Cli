//! Application Services
//!
//! Координируют выполнение use cases и управляют транзакциями.
//! Следуют принципам CQRS для разделения команд и запросов.

pub mod memory_application_service;
pub mod search_application_service;
pub mod analytics_application_service;

// Re-export services
pub use memory_application_service::*;
pub use search_application_service::*;
pub use analytics_application_service::*;