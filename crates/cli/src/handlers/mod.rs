//! Специализированные обработчики для Clean Architecture UnifiedAgent
//!
//! Каждый handler реализует Single Responsibility Principle
//! и интегрируется через Dependency Injection

pub mod admin_handler;
pub mod chat_handler;
pub mod memory_handler;
pub mod performance_monitor;
pub mod tools_handler;

pub use admin_handler::*;
pub use chat_handler::*;
pub use memory_handler::*;
pub use performance_monitor::*;
pub use tools_handler::*;
