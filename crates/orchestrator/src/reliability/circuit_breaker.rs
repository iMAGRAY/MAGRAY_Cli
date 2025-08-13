//! Circuit Breaker Implementation
//!
//! Provides circuit breaker pattern for failing agents to prevent cascading failures
//! and enable graceful degradation in production environments.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

/// Circuit breaker errors
#[derive(Debug, Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open - operation blocked")]
    Open,

    #[error("Circuit breaker configuration invalid: {reason}")]
    ConfigurationError { reason: String },

    #[error("Circuit breaker state transition failed: {reason}")]
    StateTransitionFailed { reason: String },
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit is closed, requests flow through normally
    Closed,

    /// Circuit is open, requests are blocked
    Open,

    /// Circuit is half-open, testing if service has recovered
    HalfOpen,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self::Closed
    }
}

impl std::fmt::Display for CircuitBreakerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerState::Closed => write!(f, "closed"),
            CircuitBreakerState::Open => write!(f, "open"),
            CircuitBreakerState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Configuration for circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before circuit opens
    pub failure_threshold: u32,

    /// Minimum number of requests before evaluating failure rate
    pub minimum_request_threshold: u32,

    /// Time window for failure rate calculation
    pub failure_rate_window: Duration,

    /// Time to wait before attempting recovery (half-open state)
    pub recovery_timeout: Duration,

    /// Number of successful requests needed to close circuit in half-open state
    pub success_threshold_half_open: u32,

    /// Maximum number of requests allowed in half-open state
    pub max_half_open_requests: u32,

    /// Failure rate threshold (0.0 to 1.0) to open circuit
    pub failure_rate_threshold: f64,

    /// Enable automatic recovery attempts
    pub enable_automatic_recovery: bool,

    /// Reset failure counts after successful recovery
    pub reset_on_recovery: bool,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            minimum_request_threshold: 10,
            failure_rate_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(30),
            success_threshold_half_open: 3,
            max_half_open_requests: 5,
            failure_rate_threshold: 0.5, // 50% failure rate
            enable_automatic_recovery: true,
            reset_on_recovery: true,
        }
    }
}

/// Statistics for circuit breaker operations
#[derive(Debug, Clone, Default)]
pub struct CircuitBreakerStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub blocked_requests: u64,
    pub state_transitions: u64,
    pub time_in_open_state: Duration,
    pub time_in_half_open_state: Duration,
    pub recovery_attempts: u32,
    pub successful_recoveries: u32,
    pub current_state: CircuitBreakerState,
    pub failure_rate: f64,
}

impl CircuitBreakerStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    pub fn block_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.blocked_requests as f64 / self.total_requests as f64
        }
    }
}

/// Request outcome for circuit breaker tracking
#[derive(Debug, Clone, Copy)]
struct RequestOutcome {
    timestamp: Instant,
    success: bool,
}

/// Circuit breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<std::sync::Mutex<CircuitBreakerState>>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    total_requests: AtomicU64,
    blocked_requests: AtomicU64,
    last_failure_time: Arc<std::sync::Mutex<Option<Instant>>>,
    last_state_change: Arc<std::sync::Mutex<Instant>>,
    recent_requests: Arc<std::sync::Mutex<Vec<RequestOutcome>>>,
    half_open_requests: AtomicU32,
    state_transitions: AtomicU64,
    recovery_attempts: AtomicU32,
    successful_recoveries: AtomicU32,
}

impl CircuitBreaker {
    /// Create new circuit breaker with configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(CircuitBreakerState::Closed)),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            total_requests: AtomicU64::new(0),
            blocked_requests: AtomicU64::new(0),
            last_failure_time: Arc::new(std::sync::Mutex::new(None)),
            last_state_change: Arc::new(std::sync::Mutex::new(Instant::now())),
            recent_requests: Arc::new(std::sync::Mutex::new(Vec::new())),
            half_open_requests: AtomicU32::new(0),
            state_transitions: AtomicU64::new(0),
            recovery_attempts: AtomicU32::new(0),
            successful_recoveries: AtomicU32::new(0),
        }
    }

    /// Create circuit breaker with default configuration
    pub fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// Check if circuit breaker allows execution
    pub fn can_execute(&mut self) -> bool {
        let current_state = self.get_current_state();

        match current_state {
            CircuitBreakerState::Closed => {
                self.total_requests.fetch_add(1, Ordering::SeqCst);
                true
            }
            CircuitBreakerState::Open => {
                // Check if recovery timeout has passed
                if self.config.enable_automatic_recovery && self.should_attempt_recovery() {
                    self.transition_to_half_open();
                    self.can_execute() // Recursive call to handle half-open state
                } else {
                    self.total_requests.fetch_add(1, Ordering::SeqCst);
                    self.blocked_requests.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }
            CircuitBreakerState::HalfOpen => {
                let current_half_open = self.half_open_requests.load(Ordering::SeqCst);
                if current_half_open < self.config.max_half_open_requests {
                    self.half_open_requests.fetch_add(1, Ordering::SeqCst);
                    self.total_requests.fetch_add(1, Ordering::SeqCst);
                    true
                } else {
                    self.total_requests.fetch_add(1, Ordering::SeqCst);
                    self.blocked_requests.fetch_add(1, Ordering::SeqCst);
                    false
                }
            }
        }
    }

    /// Record successful operation
    pub fn record_success(&mut self) {
        let current_state = self.get_current_state();
        self.success_count.fetch_add(1, Ordering::SeqCst);

        // Record in recent requests for failure rate calculation
        {
            let mut recent = self
                .recent_requests
                .lock()
                .expect("Circuit breaker operation should succeed");
            recent.push(RequestOutcome {
                timestamp: Instant::now(),
                success: true,
            });
            self.cleanup_old_requests(&mut recent);
        }

        debug!(state = %current_state, "Recorded successful operation");

        match current_state {
            CircuitBreakerState::Closed => {
                // No state change needed
            }
            CircuitBreakerState::HalfOpen => {
                let successes = self.success_count.load(Ordering::SeqCst);
                if successes >= self.config.success_threshold_half_open {
                    self.transition_to_closed();
                }
            }
            CircuitBreakerState::Open => {
                // Shouldn't happen if can_execute() is used correctly
                warn!("Recorded success while circuit is open");
            }
        }
    }

    /// Record failed operation
    pub fn record_failure(&mut self) {
        let current_state = self.get_current_state();
        self.failure_count.fetch_add(1, Ordering::SeqCst);

        // Record in recent requests for failure rate calculation
        {
            let mut recent = self
                .recent_requests
                .lock()
                .expect("Circuit breaker operation should succeed");
            recent.push(RequestOutcome {
                timestamp: Instant::now(),
                success: false,
            });
            self.cleanup_old_requests(&mut recent);

            *self
                .last_failure_time
                .lock()
                .expect("Circuit breaker operation should succeed") = Some(Instant::now());
        }

        debug!(state = %current_state, "Recorded failed operation");

        match current_state {
            CircuitBreakerState::Closed => {
                if self.should_open_circuit() {
                    self.transition_to_open();
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Any failure in half-open state should open the circuit
                self.transition_to_open();
            }
            CircuitBreakerState::Open => {
                // Already open, just record the failure
            }
        }
    }

    /// Get current circuit breaker state
    pub fn get_current_state(&self) -> CircuitBreakerState {
        *self
            .state
            .lock()
            .expect("Circuit breaker operation should succeed")
    }

    /// Force circuit breaker to open state
    pub fn force_open(&mut self) {
        info!("Forcing circuit breaker to open state");
        self.transition_to_open();
    }

    /// Force circuit breaker to closed state
    pub fn force_close(&mut self) {
        info!("Forcing circuit breaker to closed state");
        self.transition_to_closed();
    }

    /// Reset circuit breaker statistics
    pub fn reset(&mut self) {
        debug!("Resetting circuit breaker statistics");

        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        self.total_requests.store(0, Ordering::SeqCst);
        self.blocked_requests.store(0, Ordering::SeqCst);
        self.half_open_requests.store(0, Ordering::SeqCst);

        *self
            .last_failure_time
            .lock()
            .expect("Circuit breaker operation should succeed") = None;
        *self
            .last_state_change
            .lock()
            .expect("Circuit breaker operation should succeed") = Instant::now();
        self.recent_requests
            .lock()
            .expect("Circuit breaker operation should succeed")
            .clear();

        // Transition to closed state
        *self
            .state
            .lock()
            .expect("Circuit breaker operation should succeed") = CircuitBreakerState::Closed;
    }

    /// Check if circuit should open based on failure conditions
    fn should_open_circuit(&self) -> bool {
        let failures = self.failure_count.load(Ordering::SeqCst);
        let total = self.total_requests.load(Ordering::SeqCst);

        // Check simple failure threshold
        if failures >= self.config.failure_threshold {
            return true;
        }

        // Check failure rate within time window
        if total >= self.config.minimum_request_threshold as u64 {
            let failure_rate = self.calculate_failure_rate();
            if failure_rate >= self.config.failure_rate_threshold {
                return true;
            }
        }

        false
    }

    /// Calculate failure rate within the configured time window
    fn calculate_failure_rate(&self) -> f64 {
        let recent = self
            .recent_requests
            .lock()
            .expect("Circuit breaker operation should succeed");
        if recent.is_empty() {
            return 0.0;
        }

        let now = Instant::now();
        let window_start = now - self.config.failure_rate_window;

        let recent_in_window: Vec<_> = recent
            .iter()
            .filter(|outcome| outcome.timestamp >= window_start)
            .collect();

        if recent_in_window.is_empty() {
            return 0.0;
        }

        let failures = recent_in_window
            .iter()
            .filter(|outcome| !outcome.success)
            .count();
        failures as f64 / recent_in_window.len() as f64
    }

    /// Check if should attempt recovery (transition to half-open)
    fn should_attempt_recovery(&self) -> bool {
        let last_change = *self
            .last_state_change
            .lock()
            .expect("Circuit breaker operation should succeed");
        last_change.elapsed() >= self.config.recovery_timeout
    }

    /// Transition to open state
    fn transition_to_open(&mut self) {
        let mut state = self
            .state
            .lock()
            .expect("Circuit breaker operation should succeed");
        if *state != CircuitBreakerState::Open {
            warn!("Circuit breaker opening - blocking requests");
            *state = CircuitBreakerState::Open;
            *self
                .last_state_change
                .lock()
                .expect("Circuit breaker operation should succeed") = Instant::now();
            self.state_transitions.fetch_add(1, Ordering::SeqCst);
            self.half_open_requests.store(0, Ordering::SeqCst);
        }
    }

    /// Transition to half-open state
    fn transition_to_half_open(&mut self) {
        let mut state = self
            .state
            .lock()
            .expect("Circuit breaker operation should succeed");
        if *state != CircuitBreakerState::HalfOpen {
            info!("Circuit breaker transitioning to half-open - testing recovery");
            *state = CircuitBreakerState::HalfOpen;
            *self
                .last_state_change
                .lock()
                .expect("Circuit breaker operation should succeed") = Instant::now();
            self.state_transitions.fetch_add(1, Ordering::SeqCst);
            self.half_open_requests.store(0, Ordering::SeqCst);
            self.recovery_attempts.fetch_add(1, Ordering::SeqCst);

            if self.config.reset_on_recovery {
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Transition to closed state
    fn transition_to_closed(&mut self) {
        let mut state = self
            .state
            .lock()
            .expect("Circuit breaker operation should succeed");
        if *state != CircuitBreakerState::Closed {
            info!("Circuit breaker closing - service recovered");
            *state = CircuitBreakerState::Closed;
            *self
                .last_state_change
                .lock()
                .expect("Circuit breaker operation should succeed") = Instant::now();
            self.state_transitions.fetch_add(1, Ordering::SeqCst);
            self.half_open_requests.store(0, Ordering::SeqCst);
            self.successful_recoveries.fetch_add(1, Ordering::SeqCst);

            if self.config.reset_on_recovery {
                self.failure_count.store(0, Ordering::SeqCst);
                self.success_count.store(0, Ordering::SeqCst);
            }
        }
    }

    /// Clean up old request records outside the time window
    fn cleanup_old_requests(&self, recent: &mut Vec<RequestOutcome>) {
        let now = Instant::now();
        let cutoff = now - self.config.failure_rate_window;
        recent.retain(|outcome| outcome.timestamp >= cutoff);
    }

    /// Get circuit breaker statistics
    pub fn get_stats(&self) -> CircuitBreakerStats {
        let current_state = self.get_current_state();
        let failure_rate = self.calculate_failure_rate();

        // Calculate time in states (simplified)
        let last_change = *self
            .last_state_change
            .lock()
            .expect("Circuit breaker operation should succeed");
        let time_in_current_state = last_change.elapsed();

        CircuitBreakerStats {
            total_requests: self.total_requests.load(Ordering::SeqCst),
            successful_requests: self.success_count.load(Ordering::SeqCst) as u64,
            failed_requests: self.failure_count.load(Ordering::SeqCst) as u64,
            blocked_requests: self.blocked_requests.load(Ordering::SeqCst),
            state_transitions: self.state_transitions.load(Ordering::SeqCst),
            time_in_open_state: if current_state == CircuitBreakerState::Open {
                time_in_current_state
            } else {
                Duration::ZERO
            },
            time_in_half_open_state: if current_state == CircuitBreakerState::HalfOpen {
                time_in_current_state
            } else {
                Duration::ZERO
            },
            recovery_attempts: self.recovery_attempts.load(Ordering::SeqCst),
            successful_recoveries: self.successful_recoveries.load(Ordering::SeqCst),
            current_state,
            failure_rate,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_breaker_initial_state() {
        let mut cb = CircuitBreaker::default();
        assert_eq!(cb.get_current_state(), CircuitBreakerState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_opens_on_failures() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        });

        // Record failures to trigger opening
        for _ in 0..3 {
            cb.record_failure();
        }

        assert_eq!(cb.get_current_state(), CircuitBreakerState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_recovery_to_half_open() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            ..Default::default()
        });

        // Force circuit to open
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_current_state(), CircuitBreakerState::Open);

        // Wait for recovery timeout
        thread::sleep(Duration::from_millis(60));

        // Next can_execute should transition to half-open
        assert!(cb.can_execute());
        assert_eq!(cb.get_current_state(), CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_circuit_closes_after_successful_recovery() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold_half_open: 2,
            recovery_timeout: Duration::from_millis(10),
            ..Default::default()
        });

        // Force to open
        cb.record_failure();
        cb.record_failure();

        // Wait and transition to half-open
        thread::sleep(Duration::from_millis(20));
        cb.can_execute();

        // Record successful operations to close
        cb.record_success();
        cb.record_success();

        assert_eq!(cb.get_current_state(), CircuitBreakerState::Closed);
    }

    #[test]
    fn test_failure_rate_calculation() {
        let mut cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_rate_threshold: 0.5,
            minimum_request_threshold: 4,
            ..Default::default()
        });

        // 2 successes, 2 failures = 50% failure rate
        cb.record_success();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();

        // Need to meet minimum threshold and rate
        for _ in 0..4 {
            cb.can_execute();
        }

        // The circuit should consider opening based on failure rate
        let failure_rate = cb.calculate_failure_rate();
        assert!((failure_rate - 0.5).abs() < 0.1); // Approximately 50%
    }

    #[test]
    fn test_circuit_breaker_stats() {
        let mut cb = CircuitBreaker::default();

        cb.record_success();
        cb.record_failure();
        cb.can_execute();

        let stats = cb.get_stats();
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.current_state, CircuitBreakerState::Closed);
    }
}
