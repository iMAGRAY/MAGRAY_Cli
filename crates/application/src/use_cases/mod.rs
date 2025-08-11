//! Use Cases
//!
//! Инкапсулируют бизнес-логику приложения и координируют взаимодействие 
//! между Domain services и Infrastructure ports.

pub mod store_memory_use_case;
pub mod search_memory_use_case;
pub mod promote_records_use_case;
pub mod analyze_usage_use_case;

pub use store_memory_use_case::*;
pub use search_memory_use_case::*;
pub use promote_records_use_case::*;
pub use analyze_usage_use_case::*;