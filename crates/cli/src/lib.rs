// @component: {"k":"C","id":"cli_lib","t":"CLI interface and commands","m":{"cur":85,"tgt":95,"u":"%"},"f":["cli","interface","commands","interactive"]}
//! MAGRAY CLI Library
//! 
//! CLI components for the MAGRAY AI agent system

pub mod agent;
pub mod commands;
pub mod health_checks;
pub mod progress;

// Re-export commonly used types
pub use agent::UnifiedAgent;
pub use health_checks::{HealthCheckResult, HealthStatus, HealthCheckSystem};
pub use commands::{GpuCommand, ModelsCommand};