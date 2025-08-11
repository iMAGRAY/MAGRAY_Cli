use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing, rejecting requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker for LLM providers
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub last_failure_time: Option<Instant>,
    pub half_open_test_count: u32,
    pub half_open_max_tests: u32,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            failure_threshold: 5, // Open after 5 consecutive failures
            recovery_timeout: Duration::from_secs(60), // Wait 1 minute before testing
            last_failure_time: None,
            half_open_test_count: 0,
            half_open_max_tests: 3, // Max 3 test requests in half-open state
        }
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            ..Default::default()
        }
    }

    /// Check if request should be allowed
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        info!("🔄 Circuit breaker moving to HALF_OPEN state");
                        self.state = CircuitBreakerState::HalfOpen;
                        self.half_open_test_count = 0;
                        true
                    } else {
                        debug!("⭕ Circuit breaker OPEN - blocking request");
                        false
                    }
                } else {
                    false
                }
            }
            CircuitBreakerState::HalfOpen => {
                if self.half_open_test_count < self.half_open_max_tests {
                    self.half_open_test_count += 1;
                    debug!(
                        "🧪 Circuit breaker HALF_OPEN - test request {}/{}",
                        self.half_open_test_count, self.half_open_max_tests
                    );
                    true
                } else {
                    debug!("⭕ Circuit breaker HALF_OPEN - max tests reached");
                    false
                }
            }
        }
    }

    /// Record successful request
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                if self.failure_count > 0 {
                    debug!(
                        "✅ Circuit breaker - resetting failure count from {}",
                        self.failure_count
                    );
                    self.failure_count = 0;
                }
            }
            CircuitBreakerState::HalfOpen => {
                info!("✅ Circuit breaker - recovery successful, moving to CLOSED");
                self.state = CircuitBreakerState::Closed;
                self.failure_count = 0;
                self.last_failure_time = None;
                self.half_open_test_count = 0;
            }
            CircuitBreakerState::Open => {
                // This shouldn't happen, but handle gracefully
                warn!("⚠️ Circuit breaker - success recorded in OPEN state");
            }
        }
    }

    /// Record failed request
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());

        match self.state {
            CircuitBreakerState::Closed => {
                if self.failure_count >= self.failure_threshold {
                    warn!(
                        "🚨 Circuit breaker OPENING - {} consecutive failures",
                        self.failure_count
                    );
                    self.state = CircuitBreakerState::Open;
                } else {
                    debug!(
                        "❌ Circuit breaker - failure {}/{}",
                        self.failure_count, self.failure_threshold
                    );
                }
            }
            CircuitBreakerState::HalfOpen => {
                warn!("🚨 Circuit breaker - test failed, back to OPEN state");
                self.state = CircuitBreakerState::Open;
                self.half_open_test_count = 0;
            }
            CircuitBreakerState::Open => {
                debug!("❌ Circuit breaker - additional failure in OPEN state");
            }
        }
    }

    /// Get current state info
    pub fn get_state_info(&self) -> String {
        match self.state {
            CircuitBreakerState::Closed => format!("CLOSED (failures: {})", self.failure_count),
            CircuitBreakerState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    let elapsed = last_failure.elapsed();
                    let remaining = self.recovery_timeout.saturating_sub(elapsed);
                    format!("OPEN (recovery in: {:?})", remaining)
                } else {
                    "OPEN".to_string()
                }
            }
            CircuitBreakerState::HalfOpen => {
                format!(
                    "HALF_OPEN (tests: {}/{})",
                    self.half_open_test_count, self.half_open_max_tests
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_circuit_breaker_flow() {
        let mut cb = CircuitBreaker::new(3, Duration::from_millis(100));

        // Initially closed
        assert_eq!(cb.state, CircuitBreakerState::Closed);
        assert!(cb.can_execute());

        // Record failures
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state, CircuitBreakerState::Closed);

        cb.record_failure(); // Should open
        assert_eq!(cb.state, CircuitBreakerState::Open);
        assert!(!cb.can_execute());

        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(cb.can_execute()); // Should be half-open now
        assert_eq!(cb.state, CircuitBreakerState::HalfOpen);

        // Success should close
        cb.record_success();
        assert_eq!(cb.state, CircuitBreakerState::Closed);
    }
}
