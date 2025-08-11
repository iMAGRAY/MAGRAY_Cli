//! MAGRAY CLI Library
//!
//! CLI components for the MAGRAY AI agent system

pub mod agent;
pub mod agent_core;
pub mod agent_traits;
pub mod circuit_breaker_manager;
pub mod commands;
pub mod handler_registry;
pub mod handlers;
pub mod health_checks;
pub mod orchestrator;
pub mod performance_tracker;
pub mod progress;
pub mod refactored_unified_agent;
pub mod strategies;
pub mod unified_agent_v2;
pub mod util;

#[cfg(all(test, feature = "extended-tests"))]
mod agent_tests;

// Re-export commonly used types
pub use agent::UnifiedAgent; // Modern Clean Architecture - unified interface
pub use agent_core::{AgentComponent, AgentCore, AgentCoreBuilder, ComponentStats};
pub use agent_traits::*;
pub use circuit_breaker_manager::{
    CircuitBreakerConfig, CircuitBreakerManager, CircuitBreakerState, CircuitBreakerStats,
};
pub use commands::{GpuCommand, ModelsCommand};
pub use handler_registry::{
    AdaptiveStrategy, HandlerMetadata, HandlerRegistry, HandlerStats, RequestHandler, RoutingResult,
};
pub use handlers::*;
pub use health_checks::{HealthCheckResult, HealthCheckSystem, HealthStatus};
pub use performance_tracker::{
    ComponentMetrics, OperationMetric, PerformanceTracker, SystemMetrics, TrackerConfig,
    WarningThresholds,
};
pub use refactored_unified_agent::{
    RefactoredAgentConfig, RefactoredUnifiedAgent, RefactoredUnifiedAgentBuilder,
};
pub use strategies::*;
pub use unified_agent_v2::UnifiedAgentV2; // Direct access to V2 implementation
