//! MAGRAY CLI Library
//! 
//! CLI components for the MAGRAY AI agent system

pub mod agent;
pub mod agent_traits;
pub mod handlers;
pub mod strategies;
pub mod orchestrator;
pub mod unified_agent_v2;
pub mod legacy_bridge; // Bridge для backward compatibility
pub mod commands;
pub mod health_checks;
pub mod progress;
// pub mod services; // Временно отключено - архитектурная несовместимость

#[cfg(test)]
mod agent_tests;

// Re-export commonly used types
pub use agent::UnifiedAgent; // LEGACY Bridge - обеспечивает backward compatibility
pub use unified_agent_v2::UnifiedAgentV2; // NEW Clean Architecture - для новых проектов
pub use legacy_bridge::LegacyUnifiedAgent; // Bridge implementation
pub use agent_traits::*;
pub use handlers::*;
pub use strategies::*;
pub use health_checks::{HealthCheckResult, HealthStatus, HealthCheckSystem};
pub use commands::{GpuCommand, ModelsCommand};
// pub use services::{ServiceOrchestrator, create_services_container};