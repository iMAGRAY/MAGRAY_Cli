//! Use Cases
//!
//! Инкапсулируют бизнес-логику приложения и координируют взаимодействие
//! между Domain services и Infrastructure ports.

pub mod analyze_usage_use_case;
pub mod promote_records_use_case;
pub mod router_use_cases;
pub mod search_memory_use_case;
pub mod store_memory_use_case;
pub mod todo_use_cases;
pub mod tools_use_cases;

// AI use cases - conditional compilation based on features
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ai_use_cases;

pub use analyze_usage_use_case::*;
pub use promote_records_use_case::*;
pub use router_use_cases::*;
pub use search_memory_use_case::*;
pub use store_memory_use_case::*;
pub use todo_use_cases::*;
pub use tools_use_cases::*;

// AI use cases export - conditional compilation
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub use ai_use_cases::*;
