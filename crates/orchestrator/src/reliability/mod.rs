//! Agent Reliability Module
//!
//! Provides retry logic and timeout management for agents in the multi-agent system.
//! Ensures production-ready reliability with exponential backoff, circuit breakers,
//! and graceful degradation patterns.

pub mod circuit_breaker;
pub mod health;
pub mod retry_policy;
pub mod timeout_manager;

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerState,
};
pub use health::{HealthCheckConfig, HealthChecker, HealthMonitor, HealthReport, HealthStatus};
pub use retry_policy::{BackoffStrategy, RetryConfig, RetryError, RetryPolicy};
pub use timeout_manager::{OperationTimeoutError, TimeoutConfig, TimeoutManager};

use crate::actors::ActorId;
use std::time::Duration;
use thiserror::Error;

/// Comprehensive reliability errors for agent operations
#[derive(Debug, Error)]
pub enum ReliabilityError {
    #[error("Retry policy failed for actor {actor_id}: {source}")]
    RetryFailed {
        actor_id: ActorId,
        #[source]
        source: RetryError,
    },

    #[error("Operation timeout for actor {actor_id}: {source}")]
    Timeout {
        actor_id: ActorId,
        #[source]
        source: OperationTimeoutError,
    },

    #[error("Circuit breaker open for actor {actor_id}: {source}")]
    CircuitBreakerOpen {
        actor_id: ActorId,
        #[source]
        source: CircuitBreakerError,
    },

    #[error("Agent operation failed permanently for actor {actor_id}: {error}")]
    PermanentFailure { actor_id: ActorId, error: String },
}

/// Configuration for agent reliability features
#[derive(Debug, Clone)]
pub struct AgentReliabilityConfig {
    /// Retry policy configuration
    pub retry_config: RetryConfig,

    /// Timeout configuration  
    pub timeout_config: TimeoutConfig,

    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,

    /// Enable graceful degradation
    pub enable_graceful_degradation: bool,

    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for AgentReliabilityConfig {
    fn default() -> Self {
        Self {
            retry_config: RetryConfig::default(),
            timeout_config: TimeoutConfig::default(),
            circuit_breaker_config: CircuitBreakerConfig::default(),
            enable_graceful_degradation: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

/// Agent reliability manager that orchestrates retry, timeout, and circuit breaker patterns
#[derive(Debug)]
pub struct AgentReliabilityManager {
    config: AgentReliabilityConfig,
    retry_policies: std::collections::HashMap<ActorId, RetryPolicy>,
    timeout_managers: std::collections::HashMap<ActorId, TimeoutManager>,
    circuit_breakers: std::collections::HashMap<ActorId, CircuitBreaker>,
}

impl AgentReliabilityManager {
    /// Create a new reliability manager with default configuration
    pub fn new() -> Self {
        Self::with_config(AgentReliabilityConfig::default())
    }

    /// Create a reliability manager with custom configuration
    pub fn with_config(config: AgentReliabilityConfig) -> Self {
        Self {
            config,
            retry_policies: std::collections::HashMap::new(),
            timeout_managers: std::collections::HashMap::new(),
            circuit_breakers: std::collections::HashMap::new(),
        }
    }

    /// Register an agent for reliability management
    pub fn register_agent(&mut self, actor_id: ActorId) {
        let retry_policy = RetryPolicy::new(self.config.retry_config.clone());
        let timeout_manager = TimeoutManager::new(self.config.timeout_config.clone());
        let circuit_breaker = CircuitBreaker::new(self.config.circuit_breaker_config.clone());

        self.retry_policies.insert(actor_id, retry_policy);
        self.timeout_managers.insert(actor_id, timeout_manager);
        self.circuit_breakers.insert(actor_id, circuit_breaker);

        tracing::debug!(actor_id = %actor_id, "Agent registered for reliability management");
    }

    /// Unregister an agent from reliability management
    pub fn unregister_agent(&mut self, actor_id: ActorId) {
        self.retry_policies.remove(&actor_id);
        self.timeout_managers.remove(&actor_id);
        self.circuit_breakers.remove(&actor_id);

        tracing::debug!(actor_id = %actor_id, "Agent unregistered from reliability management");
    }

    /// Get retry policy for an agent
    pub fn get_retry_policy(&self, actor_id: ActorId) -> Option<&RetryPolicy> {
        self.retry_policies.get(&actor_id)
    }

    /// Get timeout manager for an agent
    pub fn get_timeout_manager(&self, actor_id: ActorId) -> Option<&TimeoutManager> {
        self.timeout_managers.get(&actor_id)
    }

    /// Get circuit breaker for an agent
    pub fn get_circuit_breaker(&self, actor_id: ActorId) -> Option<&CircuitBreaker> {
        self.circuit_breakers.get(&actor_id)
    }

    /// Get mutable circuit breaker for an agent
    pub fn get_circuit_breaker_mut(&mut self, actor_id: ActorId) -> Option<&mut CircuitBreaker> {
        self.circuit_breakers.get_mut(&actor_id)
    }

    /// Check if agent operations should be allowed based on circuit breaker state
    pub fn should_allow_operation(&mut self, actor_id: ActorId) -> bool {
        self.circuit_breakers
            .get_mut(&actor_id)
            .map(|cb| cb.can_execute())
            .unwrap_or(true) // Allow if no circuit breaker registered
    }

    /// Record success for circuit breaker
    pub fn record_success(&mut self, actor_id: ActorId) {
        if let Some(circuit_breaker) = self.circuit_breakers.get_mut(&actor_id) {
            circuit_breaker.record_success();
        }
    }

    /// Record failure for circuit breaker
    pub fn record_failure(&mut self, actor_id: ActorId) {
        if let Some(circuit_breaker) = self.circuit_breakers.get_mut(&actor_id) {
            circuit_breaker.record_failure();
        }
    }

    /// Get agent reliability statistics
    pub fn get_agent_stats(&self, actor_id: ActorId) -> Option<AgentReliabilityStats> {
        let retry_stats = self.retry_policies.get(&actor_id)?.get_stats();
        let timeout_stats = self.timeout_managers.get(&actor_id)?.get_stats();
        let circuit_stats = self.circuit_breakers.get(&actor_id)?.get_stats();

        Some(AgentReliabilityStats {
            actor_id,
            retry_stats,
            timeout_stats,
            circuit_stats,
        })
    }

    /// Get statistics for all registered agents
    pub fn get_all_stats(&self) -> Vec<AgentReliabilityStats> {
        self.retry_policies
            .keys()
            .filter_map(|&actor_id| self.get_agent_stats(actor_id))
            .collect()
    }
}

/// Statistics for agent reliability
#[derive(Debug, Clone)]
pub struct AgentReliabilityStats {
    pub actor_id: ActorId,
    pub retry_stats: retry_policy::RetryStats,
    pub timeout_stats: timeout_manager::TimeoutStats,
    pub circuit_stats: circuit_breaker::CircuitBreakerStats,
}

impl Default for AgentReliabilityManager {
    fn default() -> Self {
        Self::new()
    }
}
