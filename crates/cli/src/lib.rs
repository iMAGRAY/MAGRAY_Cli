//! MAGRAY CLI Library
//! 
//! CLI components for the MAGRAY AI agent system

pub mod agent;
pub mod commands;
pub mod health_checks;
pub mod progress;

// Re-export commonly used types
pub use agent::{UnifiedAgent, AgentConfig, AgentResponseInfo, AgentMetrics, MemoryConfig, AgentContext};
pub use health_checks::{HealthCheckResult, HealthStatus, HealthCheckSystem};
pub use progress::{ProgressBar, Spinner, ProgressStyle};
pub use commands::{GpuCommand, ModelsCommand};