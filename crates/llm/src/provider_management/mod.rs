//! Provider Management System
//!
//! This module provides a comprehensive system for dynamic LLM provider switching in runtime.
//! It includes provider registration, selection strategies, fallback policies, health monitoring,
//! and performance metrics collection.

pub mod health;
pub mod manager;
pub mod metrics;
pub mod policies;
pub mod registry;
pub mod strategies;

pub use health::*;
pub use manager::ProviderManager;
pub use metrics::*;
pub use policies::*;
pub use registry::ProviderRegistry;
pub use strategies::*;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::providers::{LlmRequest, ProviderId};

/// Runtime provider management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderManagementConfig {
    /// Default selection strategy
    pub default_strategy: String,
    /// Default fallback policy
    pub default_fallback_policy: String,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Maximum number of retries for failed requests
    pub max_retries: u32,
    /// Timeout for individual requests
    pub request_timeout: Duration,
    /// Enable performance metrics collection
    pub enable_metrics: bool,
    /// Circuit breaker threshold for failures
    pub circuit_breaker_threshold: u32,
}

impl Default for ProviderManagementConfig {
    fn default() -> Self {
        Self {
            default_strategy: "performance_based".to_string(),
            default_fallback_policy: "cascade".to_string(),
            health_check_interval: 30,
            max_retries: 3,
            request_timeout: Duration::from_secs(30),
            enable_metrics: true,
            circuit_breaker_threshold: 5,
        }
    }
}

/// Provider selection criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionCriteria {
    /// Preferred provider types
    pub preferred_providers: Option<Vec<String>>,
    /// Maximum acceptable latency
    pub max_latency: Option<Duration>,
    /// Maximum acceptable cost
    pub max_cost_per_token: Option<f32>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Model preference
    pub model_preference: Option<String>,
    /// Priority level
    pub priority: RequestPriority,
}

/// Request priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequestPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            preferred_providers: None,
            max_latency: None,
            max_cost_per_token: None,
            required_capabilities: vec![],
            model_preference: None,
            priority: RequestPriority::Normal,
        }
    }
}

/// Provider selection context
#[derive(Debug)]
pub struct SelectionContext {
    pub criteria: SelectionCriteria,
    pub available_providers: Vec<ProviderId>,
    pub provider_metrics: HashMap<ProviderId, ProviderMetrics>,
    pub provider_health: HashMap<ProviderId, ProviderHealthStatus>,
    pub current_load: HashMap<ProviderId, f32>,
}

/// Execution context for provider operations
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub provider_id: ProviderId,
    pub request: LlmRequest,
    pub timeout: Duration,
    pub retry_count: u32,
    pub started_at: Instant,
}

/// Result of provider selection
#[derive(Debug)]
pub struct SelectionResult {
    pub provider_id: ProviderId,
    pub confidence: f32,
    pub reasoning: String,
    pub fallback_chain: Vec<ProviderId>,
}
