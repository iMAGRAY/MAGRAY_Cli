//! Fallback policies for provider management system

use super::*;
use crate::providers::{LlmProvider, LlmRequest, LlmResponse, ProviderId};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Trait for defining fallback policies when primary provider fails
#[async_trait]
pub trait FallbackPolicy: Send + Sync {
    /// Execute fallback logic when a provider fails
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction>;

    /// Get policy name for logging and debugging
    fn name(&self) -> &'static str;

    /// Get policy description
    fn description(&self) -> &'static str;

    /// Check if the policy can handle the current failure
    async fn can_handle(&self, context: &FallbackContext, _error: &anyhow::Error) -> bool {
        // Default implementation - can handle any error if there are fallback providers
        !context.fallback_chain.is_empty() && context.retry_count < context.max_retries
    }
}

/// Context information for fallback execution
#[derive(Clone)]
pub struct FallbackContext {
    /// Original request
    pub request: LlmRequest,
    /// Primary provider that failed
    pub primary_provider: ProviderId,
    /// Available fallback providers (ordered by preference)
    pub fallback_chain: Vec<ProviderId>,
    /// Current retry count
    pub retry_count: u32,
    /// Maximum allowed retries
    pub max_retries: u32,
    /// Providers that have already been tried and failed
    pub failed_providers: Vec<(ProviderId, String)>,
    /// Additional execution context
    pub execution_context: ExecutionContext,
    /// Provider registry for accessing providers
    pub available_providers: HashMap<ProviderId, Arc<dyn LlmProvider + Send + Sync>>,
}

impl std::fmt::Debug for FallbackContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackContext")
            .field("request", &self.request)
            .field("primary_provider", &self.primary_provider)
            .field("fallback_chain", &self.fallback_chain)
            .field("retry_count", &self.retry_count)
            .field("max_retries", &self.max_retries)
            .field("failed_providers", &self.failed_providers)
            .field("execution_context", &self.execution_context)
            .field(
                "available_providers",
                &format!("{} providers", self.available_providers.len()),
            )
            .finish()
    }
}

/// Actions that can be taken by fallback policies
pub enum FallbackAction {
    /// Retry with the same provider
    Retry {
        delay: Duration,
        modified_request: Option<LlmRequest>,
    },
    /// Switch to a different provider
    Switch {
        provider_id: ProviderId,
        delay: Duration,
        modified_request: Option<LlmRequest>,
    },
    /// Fail immediately without further attempts
    Fail { reason: String },
    /// Execute custom recovery logic
    CustomRecovery {
        action: Box<dyn Fn() -> tokio::task::JoinHandle<Result<LlmResponse>> + Send + Sync>,
    },
}

impl std::fmt::Debug for FallbackAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FallbackAction::Retry {
                delay,
                modified_request,
            } => f
                .debug_struct("Retry")
                .field("delay", delay)
                .field("modified_request", modified_request)
                .finish(),
            FallbackAction::Switch {
                provider_id,
                delay,
                modified_request,
            } => f
                .debug_struct("Switch")
                .field("provider_id", provider_id)
                .field("delay", delay)
                .field("modified_request", modified_request)
                .finish(),
            FallbackAction::Fail { reason } => {
                f.debug_struct("Fail").field("reason", reason).finish()
            }
            FallbackAction::CustomRecovery { .. } => f
                .debug_struct("CustomRecovery")
                .field("action", &"<closure>")
                .finish(),
        }
    }
}

/// Cascade fallback policy - tries providers in sequence
#[derive(Debug)]
pub struct CascadePolicy {
    /// Delay between attempts
    pub retry_delay: Duration,
    /// Whether to exponentially increase delay
    pub exponential_backoff: bool,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Factor for exponential backoff
    pub backoff_factor: f32,
}

impl Default for CascadePolicy {
    fn default() -> Self {
        Self {
            retry_delay: Duration::from_millis(500),
            exponential_backoff: true,
            max_delay: Duration::from_secs(30),
            backoff_factor: 2.0,
        }
    }
}

#[async_trait]
impl FallbackPolicy for CascadePolicy {
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction> {
        debug!("Handling failure with cascade policy: {}", error);

        // Record the failure
        context
            .failed_providers
            .push((context.primary_provider.clone(), error.to_string()));

        // Check if we have any fallback providers left
        if context.fallback_chain.is_empty() {
            return Ok(FallbackAction::Fail {
                reason: format!(
                    "No more fallback providers available. Tried {} providers.",
                    context.failed_providers.len()
                ),
            });
        }

        // Check retry limits
        if context.retry_count >= context.max_retries {
            return Ok(FallbackAction::Fail {
                reason: format!(
                    "Maximum retry limit reached: {}/{}",
                    context.retry_count, context.max_retries
                ),
            });
        }

        // Get next provider from chain
        let next_provider = context.fallback_chain.remove(0);

        // Calculate delay with optional exponential backoff
        let delay = if self.exponential_backoff {
            let exponential_delay = self.retry_delay.as_millis() as f32
                * self.backoff_factor.powi(context.retry_count as i32);
            Duration::from_millis(exponential_delay.min(self.max_delay.as_millis() as f32) as u64)
        } else {
            self.retry_delay
        };

        info!(
            "Cascading to provider: {:?} after {:?} delay",
            next_provider, delay
        );

        context.retry_count += 1;

        Ok(FallbackAction::Switch {
            provider_id: next_provider,
            delay,
            modified_request: None,
        })
    }

    fn name(&self) -> &'static str {
        "cascade"
    }

    fn description(&self) -> &'static str {
        "Sequentially tries fallback providers with optional exponential backoff"
    }
}

impl CascadePolicy {
    pub fn new(
        retry_delay: Duration,
        exponential_backoff: bool,
        max_delay: Duration,
        backoff_factor: f32,
    ) -> Self {
        Self {
            retry_delay,
            exponential_backoff,
            max_delay,
            backoff_factor,
        }
    }
}

/// Round-robin fallback policy - cycles through providers
#[derive(Debug)]
pub struct RoundRobinPolicy {
    /// Delay between attempts
    pub retry_delay: Duration,
    /// Maximum number of full cycles through all providers
    pub max_cycles: u32,
    /// Current cycle counter
    cycle_counter: std::sync::atomic::AtomicU32,
}

impl Default for RoundRobinPolicy {
    fn default() -> Self {
        Self {
            retry_delay: Duration::from_millis(1000),
            max_cycles: 3,
            cycle_counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl FallbackPolicy for RoundRobinPolicy {
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction> {
        debug!("Handling failure with round-robin policy: {}", error);

        context
            .failed_providers
            .push((context.primary_provider.clone(), error.to_string()));

        // Check if we have any providers left
        if context.fallback_chain.is_empty() {
            // Check if we can start a new cycle
            let current_cycle = self
                .cycle_counter
                .load(std::sync::atomic::Ordering::Relaxed);
            if current_cycle >= self.max_cycles {
                return Ok(FallbackAction::Fail {
                    reason: format!(
                        "Maximum cycles reached: {}/{}",
                        current_cycle, self.max_cycles
                    ),
                });
            }

            // Start new cycle - rebuild fallback chain from failed providers
            context.fallback_chain = context
                .failed_providers
                .iter()
                .map(|(id, _)| id.clone())
                .collect();
            context.failed_providers.clear();

            self.cycle_counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if context.fallback_chain.is_empty() {
                return Ok(FallbackAction::Fail {
                    reason: "No providers available for round-robin cycling".to_string(),
                });
            }
        }

        // Get next provider
        let next_provider = context.fallback_chain.remove(0);
        context.retry_count += 1;

        info!("Round-robin switching to provider: {:?}", next_provider);

        Ok(FallbackAction::Switch {
            provider_id: next_provider,
            delay: self.retry_delay,
            modified_request: None,
        })
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }

    fn description(&self) -> &'static str {
        "Cycles through providers in round-robin fashion for multiple cycles"
    }
}

impl RoundRobinPolicy {
    pub fn new(retry_delay: Duration, max_cycles: u32) -> Self {
        Self {
            retry_delay,
            max_cycles,
            cycle_counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

/// Priority-based fallback policy - uses provider priorities
#[derive(Debug, Default)]
pub struct PriorityPolicy {
    /// Delay between attempts
    pub retry_delay: Duration,
    /// Whether to allow retries on same provider
    pub allow_same_provider_retry: bool,
    /// Maximum retries per provider
    pub max_retries_per_provider: u32,
}

#[async_trait]
impl FallbackPolicy for PriorityPolicy {
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction> {
        debug!("Handling failure with priority policy: {}", error);

        let failed_provider = &context.primary_provider;
        let failures_for_provider = context
            .failed_providers
            .iter()
            .filter(|(id, _)| id == failed_provider)
            .count() as u32;

        context
            .failed_providers
            .push((failed_provider.clone(), error.to_string()));

        // Check if we can retry the same provider
        if self.allow_same_provider_retry
            && failures_for_provider < self.max_retries_per_provider
            && context.retry_count < context.max_retries
        {
            info!(
                "Retrying same provider: {:?} (attempt {}/{})",
                failed_provider,
                failures_for_provider + 1,
                self.max_retries_per_provider
            );

            context.retry_count += 1;

            return Ok(FallbackAction::Retry {
                delay: self.retry_delay,
                modified_request: None,
            });
        }

        // Sort fallback providers by priority (if available)
        // Note: In a real implementation, you'd get priorities from provider metadata
        context.fallback_chain.sort_by(|a, b| {
            // For now, use simple string comparison as priority
            // In real implementation, this would use actual priority values
            a.provider_type.cmp(&b.provider_type)
        });

        if context.fallback_chain.is_empty() {
            return Ok(FallbackAction::Fail {
                reason: "No fallback providers available".to_string(),
            });
        }

        let next_provider = context.fallback_chain.remove(0);
        context.retry_count += 1;

        info!("Priority-based switch to provider: {:?}", next_provider);

        Ok(FallbackAction::Switch {
            provider_id: next_provider,
            delay: self.retry_delay,
            modified_request: None,
        })
    }

    fn name(&self) -> &'static str {
        "priority"
    }

    fn description(&self) -> &'static str {
        "Selects fallback providers based on priority with optional same-provider retries"
    }
}

impl PriorityPolicy {
    pub fn new(
        retry_delay: Duration,
        allow_same_provider_retry: bool,
        max_retries_per_provider: u32,
    ) -> Self {
        Self {
            retry_delay,
            allow_same_provider_retry,
            max_retries_per_provider,
        }
    }
}

/// Adaptive fallback policy - learns from failures and adapts behavior
#[derive(Debug)]
pub struct AdaptivePolicy {
    /// Base retry delay
    pub base_delay: Duration,
    /// Learning rate for adaptation
    pub learning_rate: f32,
    /// Failure history for learning
    failure_history: std::sync::Arc<tokio::sync::RwLock<HashMap<ProviderId, FailurePattern>>>,
    /// Adaptation configuration
    pub adaptation_config: AdaptationConfig,
}

#[derive(Debug, Clone)]
pub struct FailurePattern {
    pub total_failures: u32,
    pub recent_failures: u32,
    pub failure_types: HashMap<String, u32>,
    pub avg_recovery_time: Duration,
    pub last_failure: Instant,
    pub success_rate: f32,
}

#[derive(Debug, Clone)]
pub struct AdaptationConfig {
    pub min_delay: Duration,
    pub max_delay: Duration,
    pub failure_penalty_factor: f32,
    pub success_bonus_factor: f32,
    pub pattern_weight: f32,
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            min_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            failure_penalty_factor: 1.5,
            success_bonus_factor: 0.8,
            pattern_weight: 0.3,
        }
    }
}

impl Default for AdaptivePolicy {
    fn default() -> Self {
        Self {
            base_delay: Duration::from_millis(500),
            learning_rate: 0.1,
            failure_history: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            adaptation_config: AdaptationConfig::default(),
        }
    }
}

#[async_trait]
impl FallbackPolicy for AdaptivePolicy {
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction> {
        debug!("Handling failure with adaptive policy: {}", error);

        let failed_provider = &context.primary_provider;

        // Update failure history
        self.update_failure_history(failed_provider, &error).await;
        context
            .failed_providers
            .push((failed_provider.clone(), error.to_string()));

        // Calculate adaptive delay based on failure patterns
        let adaptive_delay = self.calculate_adaptive_delay(failed_provider).await;

        // Decide on next action based on learning
        let action = self.decide_next_action(context, adaptive_delay).await?;

        context.retry_count += 1;

        Ok(action)
    }

    fn name(&self) -> &'static str {
        "adaptive"
    }

    fn description(&self) -> &'static str {
        "Learns from failure patterns and adapts fallback behavior dynamically"
    }
}

impl AdaptivePolicy {
    pub fn new(
        base_delay: Duration,
        learning_rate: f32,
        adaptation_config: AdaptationConfig,
    ) -> Self {
        Self {
            base_delay,
            learning_rate: learning_rate.clamp(0.0, 1.0),
            failure_history: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            adaptation_config,
        }
    }

    async fn update_failure_history(&self, provider_id: &ProviderId, error: &anyhow::Error) {
        let mut history = self.failure_history.write().await;
        let pattern = history
            .entry(provider_id.clone())
            .or_insert_with(|| FailurePattern {
                total_failures: 0,
                recent_failures: 0,
                failure_types: HashMap::new(),
                avg_recovery_time: Duration::from_secs(0),
                last_failure: Instant::now(),
                success_rate: 1.0,
            });

        pattern.total_failures += 1;
        pattern.recent_failures += 1;
        pattern.last_failure = Instant::now();

        // Categorize error type
        let error_type = Self::categorize_error(error);
        *pattern.failure_types.entry(error_type).or_insert(0) += 1;

        // Update success rate using exponential smoothing
        pattern.success_rate *= 1.0 - self.learning_rate;
    }

    async fn calculate_adaptive_delay(&self, provider_id: &ProviderId) -> Duration {
        let history = self.failure_history.read().await;

        if let Some(pattern) = history.get(provider_id) {
            let failure_factor = (pattern.recent_failures as f32).ln() + 1.0;
            let success_factor = pattern.success_rate.max(0.1);

            let adaptive_multiplier =
                failure_factor * self.adaptation_config.failure_penalty_factor / success_factor;
            let delay_ms = (self.base_delay.as_millis() as f32 * adaptive_multiplier) as u64;

            Duration::from_millis(
                delay_ms
                    .max(self.adaptation_config.min_delay.as_millis() as u64)
                    .min(self.adaptation_config.max_delay.as_millis() as u64),
            )
        } else {
            self.base_delay
        }
    }

    async fn decide_next_action(
        &self,
        context: &FallbackContext,
        delay: Duration,
    ) -> Result<FallbackAction> {
        // Analyze failure patterns to decide best action
        let history = self.failure_history.read().await;

        if context.fallback_chain.is_empty() {
            return Ok(FallbackAction::Fail {
                reason: "No fallback providers available".to_string(),
            });
        }

        // Find the best provider based on learned patterns
        let mut best_provider = context.fallback_chain[0].clone();
        let mut best_score = 0.0;

        for provider_id in &context.fallback_chain {
            let score = if let Some(pattern) = history.get(provider_id) {
                // Score based on success rate and recent performance
                let recency_factor = if pattern.last_failure.elapsed() > Duration::from_secs(300) {
                    1.2 // Bonus for providers that haven't failed recently
                } else {
                    0.8
                };
                pattern.success_rate * recency_factor
            } else {
                1.0 // New providers get default score
            };

            if score > best_score {
                best_score = score;
                best_provider = provider_id.clone();
            }
        }

        info!(
            "Adaptive policy selected provider: {:?} (score: {:.2})",
            best_provider, best_score
        );

        Ok(FallbackAction::Switch {
            provider_id: best_provider,
            delay,
            modified_request: None,
        })
    }

    fn categorize_error(error: &anyhow::Error) -> String {
        let error_msg = error.to_string().to_lowercase();

        if error_msg.contains("timeout") || error_msg.contains("deadline") {
            "timeout".to_string()
        } else if error_msg.contains("rate limit") || error_msg.contains("quota") {
            "rate_limit".to_string()
        } else if error_msg.contains("auth") || error_msg.contains("unauthorized") {
            "authentication".to_string()
        } else if error_msg.contains("network") || error_msg.contains("connection") {
            "network".to_string()
        } else if error_msg.contains("internal") || error_msg.contains("server error") {
            "server_error".to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// Record a successful operation for learning
    pub async fn record_success(&self, provider_id: &ProviderId) {
        let mut history = self.failure_history.write().await;
        if let Some(pattern) = history.get_mut(provider_id) {
            // Improve success rate using exponential smoothing
            pattern.success_rate =
                pattern.success_rate * (1.0 - self.learning_rate) + self.learning_rate;
            pattern.recent_failures = pattern.recent_failures * 3 / 4; // Decay recent failures
        }
    }

    /// Get learned patterns for analysis
    pub async fn get_failure_patterns(&self) -> HashMap<ProviderId, FailurePattern> {
        self.failure_history.read().await.clone()
    }

    /// Reset learning history
    pub async fn reset_history(&self) {
        self.failure_history.write().await.clear();
    }
}

/// Circuit breaker fallback policy - implements circuit breaker pattern
#[derive(Debug)]
pub struct CircuitBreakerPolicy {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Timeout before trying half-open
    pub timeout: Duration,
    /// Success threshold to close circuit in half-open state
    pub success_threshold: u32,
    /// Circuit states per provider
    circuit_states: std::sync::Arc<tokio::sync::RwLock<HashMap<ProviderId, CircuitState>>>,
}

#[derive(Debug, Clone)]
pub struct CircuitState {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
    pub last_success_time: Option<Instant>,
}

impl Default for CircuitBreakerPolicy {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 3,
            circuit_states: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl FallbackPolicy for CircuitBreakerPolicy {
    async fn handle_failure(
        &self,
        context: &mut FallbackContext,
        error: anyhow::Error,
    ) -> Result<FallbackAction> {
        debug!("Handling failure with circuit breaker policy: {}", error);

        let failed_provider = &context.primary_provider;

        // Update circuit state
        self.record_failure(failed_provider).await;
        context
            .failed_providers
            .push((failed_provider.clone(), error.to_string()));

        let circuit_state = self.get_circuit_state(failed_provider).await;

        match circuit_state.state {
            CircuitBreakerState::Open => {
                // Circuit is open, don't try this provider
                warn!(
                    "Circuit breaker is OPEN for provider: {:?}",
                    failed_provider
                );

                // Try next available provider
                if context.fallback_chain.is_empty() {
                    return Ok(FallbackAction::Fail {
                        reason: "All circuit breakers are open".to_string(),
                    });
                }

                let next_provider = context.fallback_chain.remove(0);
                context.retry_count += 1;

                Ok(FallbackAction::Switch {
                    provider_id: next_provider,
                    delay: Duration::from_millis(100),
                    modified_request: None,
                })
            }
            CircuitBreakerState::HalfOpen => {
                // Allow limited retries in half-open state
                info!(
                    "Circuit breaker is HALF-OPEN for provider: {:?}, allowing limited retry",
                    failed_provider
                );

                context.retry_count += 1;

                Ok(FallbackAction::Retry {
                    delay: Duration::from_secs(1),
                    modified_request: None,
                })
            }
            CircuitBreakerState::Closed => {
                // Normal operation, try fallback
                if context.fallback_chain.is_empty() {
                    return Ok(FallbackAction::Fail {
                        reason: "No fallback providers available".to_string(),
                    });
                }

                let next_provider = context.fallback_chain.remove(0);
                context.retry_count += 1;

                Ok(FallbackAction::Switch {
                    provider_id: next_provider,
                    delay: Duration::from_millis(500),
                    modified_request: None,
                })
            }
        }
    }

    fn name(&self) -> &'static str {
        "circuit_breaker"
    }

    fn description(&self) -> &'static str {
        "Implements circuit breaker pattern to prevent cascade failures"
    }
}

impl CircuitBreakerPolicy {
    pub fn new(failure_threshold: u32, timeout: Duration, success_threshold: u32) -> Self {
        Self {
            failure_threshold,
            timeout,
            success_threshold,
            circuit_states: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    async fn record_failure(&self, provider_id: &ProviderId) {
        let mut states = self.circuit_states.write().await;
        let state = states
            .entry(provider_id.clone())
            .or_insert_with(|| CircuitState {
                state: CircuitBreakerState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_success_time: None,
            });

        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());

        // Check if we should open the circuit
        if state.failure_count >= self.failure_threshold {
            state.state = CircuitBreakerState::Open;
            info!("Circuit breaker OPENED for provider: {:?}", provider_id);
        }
    }

    async fn record_success(&self, provider_id: &ProviderId) {
        let mut states = self.circuit_states.write().await;
        if let Some(state) = states.get_mut(provider_id) {
            state.success_count += 1;
            state.last_success_time = Some(Instant::now());

            match state.state {
                CircuitBreakerState::HalfOpen => {
                    if state.success_count >= self.success_threshold {
                        state.state = CircuitBreakerState::Closed;
                        state.failure_count = 0;
                        state.success_count = 0;
                        info!("Circuit breaker CLOSED for provider: {:?}", provider_id);
                    }
                }
                _ => {
                    state.failure_count = 0; // Reset failure count on success
                }
            }
        }
    }

    async fn get_circuit_state(&self, provider_id: &ProviderId) -> CircuitState {
        let mut states = self.circuit_states.write().await;
        let state = states
            .entry(provider_id.clone())
            .or_insert_with(|| CircuitState {
                state: CircuitBreakerState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                last_success_time: None,
            });

        // Check if we should transition from open to half-open
        if matches!(state.state, CircuitBreakerState::Open) {
            if let Some(last_failure) = state.last_failure_time {
                if last_failure.elapsed() >= self.timeout {
                    state.state = CircuitBreakerState::HalfOpen;
                    state.success_count = 0;
                    info!(
                        "Circuit breaker transitioned to HALF-OPEN for provider: {:?}",
                        provider_id
                    );
                }
            }
        }

        state.clone()
    }

    /// Get all circuit states for monitoring
    pub async fn get_all_circuit_states(&self) -> HashMap<ProviderId, CircuitState> {
        self.circuit_states.read().await.clone()
    }
}
