//! Provider selection strategies for dynamic runtime switching

use super::*;
use crate::providers::{ProviderHealth, ProviderId};
use async_trait::async_trait;
use tracing::debug;

/// Trait for provider selection strategies
#[async_trait]
pub trait SelectionStrategy: Send + Sync {
    /// Select the best provider based on the given criteria and context
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult>;

    /// Get strategy name for logging and debugging
    fn name(&self) -> &'static str;

    /// Get strategy description
    fn description(&self) -> &'static str;

    /// Validate that the strategy can work with given context
    async fn validate_context(&self, context: &SelectionContext) -> Result<()> {
        if context.available_providers.is_empty() {
            return Err(anyhow::anyhow!("No providers available for selection"));
        }
        Ok(())
    }
}

/// Name-based provider selection strategy
#[derive(Debug, Default)]
pub struct NameBasedStrategy {
    /// Preferred provider order
    pub provider_preferences: Vec<String>,
}

impl NameBasedStrategy {
    pub fn new(preferences: Vec<String>) -> Self {
        Self {
            provider_preferences: preferences,
        }
    }
}

#[async_trait]
impl SelectionStrategy for NameBasedStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!(
            "Selecting provider using name-based strategy with preferences: {:?}",
            self.provider_preferences
        );

        // First try to match criteria preferences
        if let Some(ref preferred) = context.criteria.preferred_providers {
            for pref in preferred {
                for provider_id in &context.available_providers {
                    if provider_id.provider_type == *pref || provider_id.model.contains(pref) {
                        if let Some(health) = context.provider_health.get(provider_id) {
                            if matches!(
                                health.status,
                                ProviderHealth::Healthy | ProviderHealth::Degraded
                            ) {
                                return Ok(SelectionResult {
                                    provider_id: provider_id.clone(),
                                    confidence: 0.9,
                                    reasoning: format!("Selected preferred provider: {pref}"),
                                    fallback_chain: self.build_fallback_chain(context, provider_id),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Fall back to strategy's own preferences
        for pref in &self.provider_preferences {
            for provider_id in &context.available_providers {
                if provider_id.provider_type == *pref || provider_id.model.contains(pref) {
                    if let Some(health) = context.provider_health.get(provider_id) {
                        if matches!(
                            health.status,
                            ProviderHealth::Healthy | ProviderHealth::Degraded
                        ) {
                            return Ok(SelectionResult {
                                provider_id: provider_id.clone(),
                                confidence: 0.7,
                                reasoning: format!("Selected from strategy preferences: {pref}"),
                                fallback_chain: self.build_fallback_chain(context, provider_id),
                            });
                        }
                    }
                }
            }
        }

        // If no preferences match, select the first healthy provider
        for provider_id in &context.available_providers {
            if let Some(health) = context.provider_health.get(provider_id) {
                if matches!(health.status, ProviderHealth::Healthy) {
                    return Ok(SelectionResult {
                        provider_id: provider_id.clone(),
                        confidence: 0.5,
                        reasoning: "Selected first available healthy provider".to_string(),
                        fallback_chain: self.build_fallback_chain(context, provider_id),
                    });
                }
            }
        }

        Err(anyhow::anyhow!(
            "No healthy providers available for name-based selection"
        ))
    }

    fn name(&self) -> &'static str {
        "name_based"
    }

    fn description(&self) -> &'static str {
        "Selects providers based on name/model preferences with fallback to availability"
    }
}

impl NameBasedStrategy {
    fn build_fallback_chain(
        &self,
        context: &SelectionContext,
        selected: &ProviderId,
    ) -> Vec<ProviderId> {
        context
            .available_providers
            .iter()
            .filter(|id| *id != selected)
            .filter(|id| {
                if let Some(health) = context.provider_health.get(id) {
                    !matches!(health.status, ProviderHealth::Unavailable)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }
}

/// Performance-based provider selection strategy
#[derive(Debug)]
pub struct PerformanceBasedStrategy {
    /// Weight for latency in scoring (0.0 - 1.0)
    pub latency_weight: f32,
    /// Weight for throughput in scoring (0.0 - 1.0)
    pub throughput_weight: f32,
    /// Weight for reliability in scoring (0.0 - 1.0)
    pub reliability_weight: f32,
}

impl Default for PerformanceBasedStrategy {
    fn default() -> Self {
        Self {
            latency_weight: 0.4,
            throughput_weight: 0.3,
            reliability_weight: 0.3,
        }
    }
}

#[async_trait]
impl SelectionStrategy for PerformanceBasedStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!("Selecting provider using performance-based strategy");

        let mut scored_providers: Vec<(ProviderId, f32, String)> = Vec::new();

        for provider_id in &context.available_providers {
            // Skip unavailable providers
            if let Some(health) = context.provider_health.get(provider_id) {
                if matches!(health.status, ProviderHealth::Unavailable) {
                    continue;
                }
            }

            let score = self.calculate_performance_score(provider_id, context).await;
            let reasoning = format!("Performance score: {score:.3}");
            scored_providers.push((provider_id.clone(), score, reasoning));
        }

        if scored_providers.is_empty() {
            return Err(anyhow::anyhow!(
                "No healthy providers available for performance-based selection"
            ));
        }

        // Sort by score (highest first)
        scored_providers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Performance score comparison failed - invalid float values")
        });

        let (best_provider, best_score, reasoning) = scored_providers
            .into_iter()
            .next()
            .expect("No scored providers available after sorting");
        let confidence = (best_score * 0.8).clamp(0.1, 0.95);

        Ok(SelectionResult {
            provider_id: best_provider.clone(),
            confidence,
            reasoning,
            fallback_chain: self.build_fallback_chain(context, &best_provider),
        })
    }

    fn name(&self) -> &'static str {
        "performance_based"
    }

    fn description(&self) -> &'static str {
        "Selects providers based on weighted performance metrics (latency, throughput, reliability)"
    }
}

impl PerformanceBasedStrategy {
    pub fn new(latency_weight: f32, throughput_weight: f32, reliability_weight: f32) -> Self {
        Self {
            latency_weight,
            throughput_weight,
            reliability_weight,
        }
    }

    async fn calculate_performance_score(
        &self,
        provider_id: &ProviderId,
        context: &SelectionContext,
    ) -> f32 {
        let mut score = 0.0;

        // Latency score (lower is better)
        if let Some(metrics) = context.provider_metrics.get(provider_id) {
            let latency_ms = metrics.avg_latency.as_millis() as f32;
            let latency_score = if latency_ms > 0.0 {
                (1.0 / (1.0 + latency_ms / 1000.0)).min(1.0)
            } else {
                1.0
            };
            score += latency_score * self.latency_weight;

            // Throughput score (higher is better)
            let throughput_score = (metrics.throughput / 100.0).min(1.0);
            score += throughput_score * self.throughput_weight;

            // Reliability score (higher is better)
            let reliability_score = 1.0 - (metrics.error_rate / 100.0).min(1.0);
            score += reliability_score * self.reliability_weight;
        } else {
            // Default score for providers without metrics
            score = 0.5;
        }

        score
    }

    fn build_fallback_chain(
        &self,
        context: &SelectionContext,
        selected: &ProviderId,
    ) -> Vec<ProviderId> {
        let mut providers_with_scores: Vec<(ProviderId, f32)> = Vec::new();

        for provider_id in &context.available_providers {
            if provider_id != selected {
                if let Some(health) = context.provider_health.get(provider_id) {
                    if !matches!(health.status, ProviderHealth::Unavailable) {
                        let score = futures::executor::block_on(
                            self.calculate_performance_score(provider_id, context),
                        );
                        providers_with_scores.push((provider_id.clone(), score));
                    }
                }
            }
        }

        // Sort by score (highest first)
        providers_with_scores.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Fallback chain score comparison failed - invalid float values")
        });
        providers_with_scores
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }
}

/// Availability-based provider selection strategy
#[derive(Debug, Default)]
pub struct AvailabilityBasedStrategy {
    /// Prefer providers with lower current load
    pub prefer_low_load: bool,
}

#[async_trait]
impl SelectionStrategy for AvailabilityBasedStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!("Selecting provider using availability-based strategy");

        let mut available_providers: Vec<(ProviderId, f32, String)> = Vec::new();

        for provider_id in &context.available_providers {
            if let Some(health) = context.provider_health.get(provider_id) {
                let (availability_score, reasoning) = match health.status {
                    ProviderHealth::Healthy => {
                        let load = context
                            .current_load
                            .get(provider_id)
                            .copied()
                            .unwrap_or(0.0);
                        let load_score = if self.prefer_low_load {
                            1.0 - load.min(1.0)
                        } else {
                            0.8
                        };
                        (load_score, format!("Healthy with load: {load:.2}"))
                    }
                    ProviderHealth::Degraded => {
                        (0.5, "Provider degraded but available".to_string())
                    }
                    ProviderHealth::Unavailable => continue,
                };

                // Check circuit breaker state
                let circuit_breaker_penalty = match health.circuit_breaker_state {
                    CircuitBreakerState::Closed => 0.0,
                    CircuitBreakerState::HalfOpen => 0.2,
                    CircuitBreakerState::Open => continue, // Skip open circuit breakers
                };

                let final_score = availability_score - circuit_breaker_penalty;
                available_providers.push((provider_id.clone(), final_score, reasoning));
            }
        }

        if available_providers.is_empty() {
            return Err(anyhow::anyhow!("No available providers found"));
        }

        // Sort by availability score (highest first)
        available_providers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Availability score comparison failed - invalid float values")
        });

        let (best_provider, score, reasoning) = available_providers
            .into_iter()
            .next()
            .expect("No available providers after sorting");
        let confidence = score.clamp(0.3, 0.95);

        Ok(SelectionResult {
            provider_id: best_provider.clone(),
            confidence,
            reasoning,
            fallback_chain: self.build_fallback_chain(context, &best_provider),
        })
    }

    fn name(&self) -> &'static str {
        "availability_based"
    }

    fn description(&self) -> &'static str {
        "Selects providers based on health status and current load"
    }
}

impl AvailabilityBasedStrategy {
    fn build_fallback_chain(
        &self,
        context: &SelectionContext,
        selected: &ProviderId,
    ) -> Vec<ProviderId> {
        let mut providers: Vec<(ProviderId, f32)> = Vec::new();

        for provider_id in &context.available_providers {
            if provider_id != selected {
                if let Some(health) = context.provider_health.get(provider_id) {
                    let score = match health.status {
                        ProviderHealth::Healthy => 1.0,
                        ProviderHealth::Degraded => 0.5,
                        ProviderHealth::Unavailable => continue,
                    };

                    let load = context
                        .current_load
                        .get(provider_id)
                        .copied()
                        .unwrap_or(0.0);
                    let adjusted_score = score - load * 0.3;
                    providers.push((provider_id.clone(), adjusted_score));
                }
            }
        }

        providers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Availability-based fallback chain score comparison failed")
        });
        providers.into_iter().map(|(id, _)| id).collect()
    }
}

/// Cost-based provider selection strategy
#[derive(Debug)]
pub struct CostBasedStrategy {
    /// Maximum cost per 1K tokens
    pub max_cost_per_1k_tokens: Option<f32>,
    /// Weight for cost optimization vs performance
    pub cost_weight: f32,
}

impl Default for CostBasedStrategy {
    fn default() -> Self {
        Self {
            max_cost_per_1k_tokens: None,
            cost_weight: 0.8,
        }
    }
}

#[async_trait]
impl SelectionStrategy for CostBasedStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!("Selecting provider using cost-based strategy");

        let mut cost_providers: Vec<(ProviderId, f32, String)> = Vec::new();

        for provider_id in &context.available_providers {
            // Skip unavailable providers
            if let Some(health) = context.provider_health.get(provider_id) {
                if matches!(health.status, ProviderHealth::Unavailable) {
                    continue;
                }
            }

            if let Some(metrics) = context.provider_metrics.get(provider_id) {
                let cost_per_token = if metrics.total_tokens_processed > 0 {
                    metrics.total_cost / metrics.total_tokens_processed as f32
                } else {
                    // Use provider capabilities for cost estimation
                    0.001 // Default cost estimate
                };

                // Apply cost filter if specified
                if let Some(max_cost) = self.max_cost_per_1k_tokens {
                    if cost_per_token * 1000.0 > max_cost {
                        continue;
                    }
                }

                // Calculate cost score (lower cost = higher score)
                let cost_score = if cost_per_token > 0.0 {
                    1.0 / (1.0 + cost_per_token * 10000.0)
                } else {
                    1.0
                };

                // Consider performance as well
                let performance_score =
                    if metrics.error_rate < 5.0 && metrics.avg_latency.as_millis() < 5000 {
                        0.8
                    } else {
                        0.5
                    };

                let combined_score =
                    cost_score * self.cost_weight + performance_score * (1.0 - self.cost_weight);
                let reasoning = format!(
                    "Cost per 1K tokens: ${:.4}, Combined score: {:.3}",
                    cost_per_token * 1000.0,
                    combined_score
                );

                cost_providers.push((provider_id.clone(), combined_score, reasoning));
            } else {
                // Provider without metrics - assign default score
                cost_providers.push((
                    provider_id.clone(),
                    0.5,
                    "No cost data available".to_string(),
                ));
            }
        }

        if cost_providers.is_empty() {
            return Err(anyhow::anyhow!("No cost-effective providers available"));
        }

        // Sort by combined score (highest first)
        cost_providers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Cost-based score comparison failed - invalid float values")
        });

        let (best_provider, score, reasoning) = cost_providers
            .into_iter()
            .next()
            .expect("No cost-effective providers after sorting");
        let confidence = score.clamp(0.2, 0.95);

        Ok(SelectionResult {
            provider_id: best_provider.clone(),
            confidence,
            reasoning,
            fallback_chain: self.build_fallback_chain(context, &best_provider),
        })
    }

    fn name(&self) -> &'static str {
        "cost_based"
    }

    fn description(&self) -> &'static str {
        "Selects providers based on cost optimization with performance consideration"
    }
}

impl CostBasedStrategy {
    pub fn new(max_cost_per_1k_tokens: Option<f32>, cost_weight: f32) -> Self {
        Self {
            max_cost_per_1k_tokens,
            cost_weight: cost_weight.clamp(0.0, 1.0),
        }
    }

    fn build_fallback_chain(
        &self,
        context: &SelectionContext,
        selected: &ProviderId,
    ) -> Vec<ProviderId> {
        let mut providers: Vec<(ProviderId, f32)> = Vec::new();

        for provider_id in &context.available_providers {
            if provider_id != selected {
                if let Some(health) = context.provider_health.get(provider_id) {
                    if matches!(health.status, ProviderHealth::Unavailable) {
                        continue;
                    }
                }

                let score = if let Some(metrics) = context.provider_metrics.get(provider_id) {
                    let cost_per_token = if metrics.total_tokens_processed > 0 {
                        metrics.total_cost / metrics.total_tokens_processed as f32
                    } else {
                        0.001
                    };

                    if let Some(max_cost) = self.max_cost_per_1k_tokens {
                        if cost_per_token * 1000.0 > max_cost {
                            continue;
                        }
                    }

                    1.0 / (1.0 + cost_per_token * 10000.0)
                } else {
                    0.3
                };

                providers.push((provider_id.clone(), score));
            }
        }

        providers.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .expect("Cost-based fallback chain score comparison failed")
        });
        providers.into_iter().map(|(id, _)| id).collect()
    }
}

/// Load-balanced provider selection strategy
#[derive(Debug, Default)]
pub struct LoadBalancedStrategy {
    /// Round-robin counter for load balancing
    pub round_robin_counter: std::sync::atomic::AtomicUsize,
}

#[async_trait]
impl SelectionStrategy for LoadBalancedStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!("Selecting provider using load-balanced strategy");

        // Filter healthy providers
        let healthy_providers: Vec<&ProviderId> = context
            .available_providers
            .iter()
            .filter(|provider_id| {
                if let Some(health) = context.provider_health.get(provider_id) {
                    !matches!(health.status, ProviderHealth::Unavailable)
                        && !matches!(health.circuit_breaker_state, CircuitBreakerState::Open)
                } else {
                    true
                }
            })
            .collect();

        if healthy_providers.is_empty() {
            return Err(anyhow::anyhow!(
                "No healthy providers available for load balancing"
            ));
        }

        // Use round-robin selection
        let counter = self
            .round_robin_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let selected_index = counter % healthy_providers.len();
        let selected_provider = healthy_providers[selected_index].clone();

        let current_load = context
            .current_load
            .get(&selected_provider)
            .copied()
            .unwrap_or(0.0);
        let reasoning = format!(
            "Round-robin selection (index: {selected_index}), current load: {current_load:.2}"
        );

        Ok(SelectionResult {
            provider_id: selected_provider.clone(),
            confidence: 0.7,
            reasoning,
            fallback_chain: self.build_fallback_chain(context, &selected_provider),
        })
    }

    fn name(&self) -> &'static str {
        "load_balanced"
    }

    fn description(&self) -> &'static str {
        "Distributes requests evenly across healthy providers using round-robin"
    }
}

impl LoadBalancedStrategy {
    fn build_fallback_chain(
        &self,
        context: &SelectionContext,
        selected: &ProviderId,
    ) -> Vec<ProviderId> {
        context
            .available_providers
            .iter()
            .filter(|id| *id != selected)
            .filter(|id| {
                if let Some(health) = context.provider_health.get(id) {
                    !matches!(health.status, ProviderHealth::Unavailable)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }
}

/// Composite strategy that combines multiple strategies
pub struct CompositeStrategy {
    /// Primary strategy
    pub primary: Box<dyn SelectionStrategy>,
    /// Fallback strategy if primary fails
    pub fallback: Box<dyn SelectionStrategy>,
    /// Weight for primary strategy (0.0 - 1.0)
    pub primary_weight: f32,
}

impl std::fmt::Debug for CompositeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeStrategy")
            .field("primary", &self.primary.name())
            .field("fallback", &self.fallback.name())
            .field("primary_weight", &self.primary_weight)
            .finish()
    }
}

#[async_trait]
impl SelectionStrategy for CompositeStrategy {
    async fn select_provider(&self, context: &SelectionContext) -> Result<SelectionResult> {
        self.validate_context(context).await?;

        debug!(
            "Selecting provider using composite strategy (primary: {}, fallback: {})",
            self.primary.name(),
            self.fallback.name()
        );

        // Try primary strategy first
        match self.primary.select_provider(context).await {
            Ok(mut result) => {
                result.confidence *= self.primary_weight;
                result.reasoning =
                    format!("Primary ({}): {}", self.primary.name(), result.reasoning);
                Ok(result)
            }
            Err(primary_error) => {
                debug!(
                    "Primary strategy failed: {}, trying fallback",
                    primary_error
                );

                match self.fallback.select_provider(context).await {
                    Ok(mut result) => {
                        result.confidence *= 1.0 - self.primary_weight;
                        result.reasoning =
                            format!("Fallback ({}): {}", self.fallback.name(), result.reasoning);
                        Ok(result)
                    }
                    Err(fallback_error) => Err(anyhow::anyhow!(
                        "Both strategies failed: primary: {}, fallback: {}",
                        primary_error,
                        fallback_error
                    )),
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "composite"
    }

    fn description(&self) -> &'static str {
        "Combines multiple selection strategies with fallback mechanism"
    }
}

impl CompositeStrategy {
    pub fn new(
        primary: Box<dyn SelectionStrategy>,
        fallback: Box<dyn SelectionStrategy>,
        primary_weight: f32,
    ) -> Self {
        Self {
            primary,
            fallback,
            primary_weight: primary_weight.clamp(0.0, 1.0),
        }
    }
}
