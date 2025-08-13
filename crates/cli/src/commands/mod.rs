pub mod agent;
pub mod config;
pub mod gpu;
#[cfg(not(feature = "minimal"))]
pub mod memory;
#[cfg(feature = "minimal")]
pub mod memory_stub;
pub mod models;
pub mod orchestrator;
pub mod router;
pub mod smart;
pub mod tasks;
pub mod tools;

// AI commands - available when CPU or GPU features are enabled
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ai;

pub use gpu::GpuCommand;
#[cfg(not(feature = "minimal"))]
pub use memory::MemoryCommand;
#[cfg(feature = "minimal")]
pub use memory_stub::MemoryCommand;
pub use models::ModelsCommand;
pub use orchestrator::OrchestratorCommand;
pub use smart::SmartCommand;
pub use tasks::TasksCommand;
pub use tools::ToolsCommand;

// AI commands export - conditional compilation
