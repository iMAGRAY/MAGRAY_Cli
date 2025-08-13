//! Health checking system for LLM providers

use super::*;
use crate::providers::{LlmProvider, LlmRequest, ProviderHealth, ProviderId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Health monitor for tracking provider health status
#[derive(Debug)]
pub struct HealthMonitor {
    /// Current health status of providers
    health_status: Arc<RwLock<HashMap<ProviderId, ProviderHealthStatus>>>,
    /// Health check configuration
    config: HealthMonitorConfig,
    /// Health check task handles
    task_handles: Arc<RwLock<HashMap<ProviderId, tokio::task::JoinHandle<()>>>>,
}

/// Detailed health status information  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthStatus {
    pub status: ProviderHealth,
    #[serde(skip, default = "Instant::now")]
    pub last_check: Instant,
    pub response_time: Option<Duration>,
    pub error_rate: f32,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub uptime_percentage: f32,
    pub circuit_breaker_state: CircuitBreakerState,
}

/// Circuit breaker states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing, requests blocked
    HalfOpen, // Testing if service recovered
}

/// Health monitoring configuration
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// Health check interval
    pub check_interval: Duration,
    /// Timeout for health checks
    pub check_timeout: Duration,
    /// Number of consecutive failures before marking as unhealthy
    pub failure_threshold: u32,
    /// Number of consecutive successes to mark as healthy again
    pub recovery_threshold: u32,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker recovery timeout
    pub circuit_breaker_timeout: Duration,
    /// Enable detailed health metrics
    pub enable_detailed_metrics: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(10),
            failure_threshold: 3,
            recovery_threshold: 2,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
            enable_detailed_metrics: true,
        }
    }
}

/// Health check result
#[derive(Debug)]
pub struct HealthCheckResult {
    pub provider_id: ProviderId,
    pub status: ProviderHealth,
    pub response_time: Duration,
    pub error: Option<String>,
    pub timestamp: Instant,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self::new_with_config(HealthMonitorConfig::default())
    }

    /// Create a new health monitor with custom configuration
    pub fn new_with_config(config: HealthMonitorConfig) -> Self {
        debug!("Creating health monitor with config: {:?}", config);

        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            config,
            task_handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start monitoring a provider
    pub async fn start_monitoring(
        &self,
        provider_id: ProviderId,
        provider: Arc<dyn LlmProvider + Send + Sync>,
    ) {
        debug!("Starting health monitoring for provider: {:?}", provider_id);

        // Initialize health status
        let initial_status = ProviderHealthStatus {
            status: ProviderHealth::Healthy,
            last_check: Instant::now(),
            response_time: None,
            error_rate: 0.0,
            consecutive_failures: 0,
            last_error: None,
            uptime_percentage: 100.0,
            circuit_breaker_state: CircuitBreakerState::Closed,
        };

        self.health_status
            .write()
            .await
            .insert(provider_id.clone(), initial_status);

        // Start background health checking task
        let health_status = Arc::clone(&self.health_status);
        let config = self.config.clone();
        let id = provider_id.clone();

        let handle = tokio::spawn(async move {
            Self::health_check_loop(id, provider, health_status, config).await;
        });

        self.task_handles.write().await.insert(provider_id, handle);
    }

    /// Stop monitoring a provider
    pub async fn stop_monitoring(&self, provider_id: &ProviderId) {
        debug!("Stopping health monitoring for provider: {:?}", provider_id);

        // Cancel the background task
        if let Some(handle) = self.task_handles.write().await.remove(provider_id) {
            handle.abort();
        }

        // Remove health status
        self.health_status.write().await.remove(provider_id);
    }

    /// Get current health status of a provider
    pub async fn get_health_status(
        &self,
        provider_id: &ProviderId,
    ) -> Option<ProviderHealthStatus> {
        let status = self.health_status.read().await;
        status.get(provider_id).cloned()
    }

    /// Get health status of all monitored providers
    pub async fn get_all_health_status(&self) -> HashMap<ProviderId, ProviderHealthStatus> {
        self.health_status.read().await.clone()
    }

    /// Check if a provider is healthy
    pub async fn is_healthy(&self, provider_id: &ProviderId) -> bool {
        if let Some(status) = self.get_health_status(provider_id).await {
            matches!(status.status, ProviderHealth::Healthy)
                && matches!(
                    status.circuit_breaker_state,
                    CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen
                )
        } else {
            false
        }
    }

    /// Check if circuit breaker allows requests
    pub async fn can_execute_request(&self, provider_id: &ProviderId) -> bool {
        if let Some(status) = self.get_health_status(provider_id).await {
            match status.circuit_breaker_state {
                CircuitBreakerState::Closed => true,
                CircuitBreakerState::HalfOpen => true, // Allow limited requests in half-open state
                CircuitBreakerState::Open => {
                    // Check if enough time has passed to try recovery
                    status.last_check.elapsed() > self.config.circuit_breaker_timeout
                }
            }
        } else {
            false
        }
    }

    /// Record a successful request
    pub async fn record_success(&self, provider_id: &ProviderId, response_time: Duration) {
        if let Some(status) = self.health_status.write().await.get_mut(provider_id) {
            status.consecutive_failures = 0;
            status.response_time = Some(response_time);
            status.last_check = Instant::now();

            // Update circuit breaker state
            if status.circuit_breaker_state == CircuitBreakerState::HalfOpen {
                // Successful request in half-open state closes the circuit breaker
                status.circuit_breaker_state = CircuitBreakerState::Closed;
                status.status = ProviderHealth::Healthy;
                info!("Circuit breaker closed for provider: {:?}", provider_id);
            }
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self, provider_id: &ProviderId, error: String) {
        if let Some(status) = self.health_status.write().await.get_mut(provider_id) {
            status.consecutive_failures += 1;
            status.last_error = Some(error);
            status.last_check = Instant::now();

            // Update health status based on consecutive failures
            if status.consecutive_failures >= self.config.failure_threshold {
                status.status = ProviderHealth::Unavailable;
            } else if status.consecutive_failures > 1 {
                status.status = ProviderHealth::Degraded;
            }

            // Update circuit breaker state
            if status.consecutive_failures >= self.config.circuit_breaker_threshold {
                match status.circuit_breaker_state {
                    CircuitBreakerState::Closed => {
                        status.circuit_breaker_state = CircuitBreakerState::Open;
                        warn!("Circuit breaker opened for provider: {:?}", provider_id);
                    }
                    CircuitBreakerState::HalfOpen => {
                        status.circuit_breaker_state = CircuitBreakerState::Open;
                        warn!(
                            "Circuit breaker returned to open state for provider: {:?}",
                            provider_id
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    /// Perform immediate health check for a provider
    pub async fn check_health_immediately(
        &self,
        provider_id: &ProviderId,
        provider: Arc<dyn LlmProvider + Send + Sync>,
    ) -> Result<HealthCheckResult> {
        debug!(
            "Performing immediate health check for provider: {:?}",
            provider_id
        );

        let start_time = Instant::now();

        // Create a simple health check request
        let _health_request = LlmRequest::new("Health check");

        let result =
            match tokio::time::timeout(self.config.check_timeout, provider.health_check()).await {
                Ok(Ok(health)) => {
                    let response_time = start_time.elapsed();
                    debug!(
                        "Health check successful for provider: {:?} in {:?}",
                        provider_id, response_time
                    );

                    HealthCheckResult {
                        provider_id: provider_id.clone(),
                        status: health,
                        response_time,
                        error: None,
                        timestamp: Instant::now(),
                    }
                }
                Ok(Err(e)) => {
                    let response_time = start_time.elapsed();
                    warn!("Health check failed for provider: {:?}: {}", provider_id, e);

                    HealthCheckResult {
                        provider_id: provider_id.clone(),
                        status: ProviderHealth::Unavailable,
                        response_time,
                        error: Some(e.to_string()),
                        timestamp: Instant::now(),
                    }
                }
                Err(_) => {
                    let response_time = start_time.elapsed();
                    warn!("Health check timeout for provider: {:?}", provider_id);

                    HealthCheckResult {
                        provider_id: provider_id.clone(),
                        status: ProviderHealth::Unavailable,
                        response_time,
                        error: Some("Health check timeout".to_string()),
                        timestamp: Instant::now(),
                    }
                }
            };

        // Update health status based on result
        match result.error.as_ref() {
            None => self.record_success(provider_id, result.response_time).await,
            Some(error) => self.record_failure(provider_id, error.clone()).await,
        }

        Ok(result)
    }

    /// Background health checking loop
    async fn health_check_loop(
        provider_id: ProviderId,
        provider: Arc<dyn LlmProvider + Send + Sync>,
        health_status: Arc<RwLock<HashMap<ProviderId, ProviderHealthStatus>>>,
        config: HealthMonitorConfig,
    ) {
        let mut interval = tokio::time::interval(config.check_interval);

        loop {
            interval.tick().await;

            debug!(
                "Running scheduled health check for provider: {:?}",
                provider_id
            );

            let start_time = Instant::now();

            let result =
                match tokio::time::timeout(config.check_timeout, provider.health_check()).await {
                    Ok(Ok(health)) => {
                        let response_time = start_time.elapsed();
                        debug!(
                            "Scheduled health check successful for provider: {:?}",
                            provider_id
                        );
                        (health, Some(response_time), None)
                    }
                    Ok(Err(e)) => {
                        let response_time = start_time.elapsed();
                        warn!(
                            "Scheduled health check failed for provider: {:?}: {}",
                            provider_id, e
                        );
                        (
                            ProviderHealth::Unavailable,
                            Some(response_time),
                            Some(e.to_string()),
                        )
                    }
                    Err(_) => {
                        let response_time = start_time.elapsed();
                        warn!(
                            "Scheduled health check timeout for provider: {:?}",
                            provider_id
                        );
                        (
                            ProviderHealth::Unavailable,
                            Some(response_time),
                            Some("Timeout".to_string()),
                        )
                    }
                };

            // Update health status
            if let Some(status) = health_status.write().await.get_mut(&provider_id) {
                status.last_check = Instant::now();
                status.response_time = result.1;

                match result.2 {
                    None => {
                        // Success
                        status.consecutive_failures = 0;
                        status.status = result.0;

                        // Update circuit breaker state
                        if matches!(status.circuit_breaker_state, CircuitBreakerState::HalfOpen) {
                            status.circuit_breaker_state = CircuitBreakerState::Closed;
                        }
                    }
                    Some(error) => {
                        // Failure
                        status.consecutive_failures += 1;
                        status.last_error = Some(error);

                        if status.consecutive_failures >= config.failure_threshold {
                            status.status = ProviderHealth::Unavailable;
                        } else if status.consecutive_failures > 1 {
                            status.status = ProviderHealth::Degraded;
                        }

                        // Update circuit breaker state
                        if status.consecutive_failures >= config.circuit_breaker_threshold {
                            match status.circuit_breaker_state {
                                CircuitBreakerState::Closed => {
                                    status.circuit_breaker_state = CircuitBreakerState::Open;
                                }
                                CircuitBreakerState::HalfOpen => {
                                    status.circuit_breaker_state = CircuitBreakerState::Open;
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Check if circuit breaker should transition to half-open
                if matches!(status.circuit_breaker_state, CircuitBreakerState::Open)
                    && status.last_check.elapsed() > config.circuit_breaker_timeout
                {
                    status.circuit_breaker_state = CircuitBreakerState::HalfOpen;
                    info!(
                        "Circuit breaker transitioned to half-open for provider: {:?}",
                        provider_id
                    );
                }
            }
        }
    }

    /// Get health monitoring statistics
    pub async fn get_statistics(&self) -> HealthMonitorStatistics {
        let status_map = self.health_status.read().await;

        let total_providers = status_map.len();
        let healthy_providers = status_map
            .values()
            .filter(|s| matches!(s.status, ProviderHealth::Healthy))
            .count();
        let degraded_providers = status_map
            .values()
            .filter(|s| matches!(s.status, ProviderHealth::Degraded))
            .count();
        let unavailable_providers = status_map
            .values()
            .filter(|s| matches!(s.status, ProviderHealth::Unavailable))
            .count();

        let open_circuit_breakers = status_map
            .values()
            .filter(|s| matches!(s.circuit_breaker_state, CircuitBreakerState::Open))
            .count();

        HealthMonitorStatistics {
            total_providers,
            healthy_providers,
            degraded_providers,
            unavailable_providers,
            open_circuit_breakers,
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Health monitoring statistics
#[derive(Debug, Clone)]
pub struct HealthMonitorStatistics {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub degraded_providers: usize,
    pub unavailable_providers: usize,
    pub open_circuit_breakers: usize,
}
