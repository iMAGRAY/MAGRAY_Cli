// @component: {"k":"C","id":"cli_lib","t":"CLI interface and commands","m":{"cur":85,"tgt":95,"u":"%"},"f":["cli","interface","commands","interactive"]}
//! MAGRAY CLI Library
//! 
//! CLI components for the MAGRAY AI agent system

pub mod agent;
pub mod agent_traits;
pub mod handlers;
pub mod strategies;
pub mod unified_agent_v2;
pub mod commands;
pub mod health_checks;
pub mod progress;
// pub mod services; // Временно отключено - архитектурная несовместимость

// Re-export commonly used types
pub use agent::{UnifiedAgent, AgentResponse}; // LEGACY
pub use unified_agent_v2::UnifiedAgentV2; // NEW Clean Architecture
pub use agent_traits::*;
pub use handlers::*;
pub use strategies::*;
pub use health_checks::{HealthCheckResult, HealthStatus, HealthCheckSystem};
pub use commands::{GpuCommand, ModelsCommand};
// pub use services::{ServiceOrchestrator, create_services_container};