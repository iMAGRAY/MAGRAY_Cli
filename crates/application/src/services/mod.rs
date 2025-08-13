//! Application Services
//!
//! Координируют выполнение use cases и управляют транзакциями.
//! Следуют принципам CQRS для разделения команд и запросов.

pub mod memory_application_service;
pub mod router_application_service;
pub mod tools_application_service;
// Temporarily commented out missing services
// pub mod search_application_service;
// pub mod analytics_application_service;

// AI application service - conditional compilation based on features
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ai_application_service;

// Re-export services
pub use memory_application_service::*;
pub use tools_application_service::*;
// pub use search_application_service::*;
// pub use analytics_application_service::*;

// AI service export - conditional compilation
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub use ai_application_service::*;
