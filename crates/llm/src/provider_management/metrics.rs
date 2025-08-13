//! Performance metrics collection system for LLM providers

#![allow(clippy::field_reassign_with_default)]

use super::*;
use crate::providers::{LlmResponse, ProviderId};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// Performance metrics collector
#[derive(Debug)]
pub struct MetricsCollector {
    /// Provider metrics storage
    metrics: Arc<RwLock<HashMap<ProviderId, ProviderMetrics>>>,
    /// Historical performance data
    history: Arc<RwLock<HashMap<ProviderId, PerformanceHistory>>>,
    /// Metrics configuration
    config: MetricsConfig,
}

/// Comprehensive metrics for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetrics {
    pub provider_id: ProviderId,
    pub request_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_latency: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub avg_latency: Duration,
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
    pub total_tokens_processed: u64,
    pub total_cost: f32,
    pub throughput: f32, // requests per second
    pub error_rate: f32, // percentage
    pub uptime_percentage: f32,
    #[serde(skip)]
    pub last_request_time: Option<Instant>,
    pub circuit_breaker_trips: u32,
    pub retry_count: u64,
    pub first_byte_time: Duration, // Time to first response byte
    pub quality_score: f32,        // Overall quality metric (0-1)
}

impl Default for ProviderMetrics {
    fn default() -> Self {
        Self {
            provider_id: ProviderId::new("unknown", "unknown"),
            request_count: 0,
            success_count: 0,
            failure_count: 0,
            total_latency: Duration::from_secs(0),
            min_latency: Duration::from_secs(u64::MAX),
            max_latency: Duration::from_secs(0),
            avg_latency: Duration::from_secs(0),
            p50_latency: Duration::from_secs(0),
            p95_latency: Duration::from_secs(0),
            p99_latency: Duration::from_secs(0),
            total_tokens_processed: 0,
            total_cost: 0.0,
            throughput: 0.0,
            error_rate: 0.0,
            uptime_percentage: 100.0,
            last_request_time: None,
            circuit_breaker_trips: 0,
            retry_count: 0,
            first_byte_time: Duration::from_secs(0),
            quality_score: 1.0,
        }
    }
}

/// Historical performance data
#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    /// Sliding window of latency measurements
    pub latency_samples: VecDeque<Duration>,
    /// Sliding window of request timestamps
    pub request_timestamps: VecDeque<Instant>,
    /// Sliding window of error events
    pub error_events: VecDeque<ErrorEvent>,
    /// Cost tracking over time
    pub cost_history: VecDeque<(Instant, f32)>,
    /// Throughput measurements over time
    pub throughput_history: VecDeque<(Instant, f32)>,
}

/// Error event for tracking patterns
#[derive(Debug, Clone)]
pub struct ErrorEvent {
    pub timestamp: Instant,
    pub error_type: String,
    pub error_message: String,
    pub request_id: Option<String>,
}

/// Metrics collection configuration
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Maximum number of samples to keep in history
    pub max_history_size: usize,
    /// How often to calculate derived metrics
    pub calculation_interval: Duration,
    /// Enable detailed request tracing
    pub enable_request_tracing: bool,
    /// Enable cost tracking
    pub enable_cost_tracking: bool,
    /// Time window for throughput calculation
    pub throughput_window: Duration,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            max_history_size: 1000,
            calculation_interval: Duration::from_secs(60),
            enable_request_tracing: true,
            enable_cost_tracking: true,
            throughput_window: Duration::from_secs(60),
        }
    }
}

/// Request execution metrics
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub provider_id: ProviderId,
    pub request_id: String,
    pub started_at: Instant,
    pub completed_at: Option<Instant>,
    pub latency: Option<Duration>,
    pub first_byte_time: Option<Duration>,
    pub success: bool,
    pub error: Option<String>,
    pub tokens_used: Option<u64>,
    pub cost: Option<f32>,
    pub retry_count: u32,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self::new_with_config(MetricsConfig::default())
    }

    /// Create a new metrics collector with custom configuration
    pub fn new_with_config(config: MetricsConfig) -> Self {
        debug!("Creating metrics collector with config: {:?}", config);

        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Initialize metrics for a provider
    pub async fn initialize_provider(&self, provider_id: ProviderId) {
        debug!("Initializing metrics for provider: {:?}", provider_id);

        let mut metrics = ProviderMetrics::default();
        metrics.provider_id = provider_id.clone();

        self.metrics
            .write()
            .await
            .insert(provider_id.clone(), metrics);
        self.history.write().await.insert(
            provider_id,
            PerformanceHistory {
                latency_samples: VecDeque::new(),
                request_timestamps: VecDeque::new(),
                error_events: VecDeque::new(),
                cost_history: VecDeque::new(),
                throughput_history: VecDeque::new(),
            },
        );
    }

    /// Record the start of a request
    pub async fn start_request(&self, provider_id: &ProviderId) -> RequestMetrics {
        let request_id = uuid::Uuid::new_v4().to_string();

        trace!(
            "Starting request tracking: {} for provider: {:?}",
            request_id,
            provider_id
        );

        RequestMetrics {
            provider_id: provider_id.clone(),
            request_id,
            started_at: Instant::now(),
            completed_at: None,
            latency: None,
            first_byte_time: None,
            success: false,
            error: None,
            tokens_used: None,
            cost: None,
            retry_count: 0,
        }
    }

    /// Record the completion of a request
    pub async fn complete_request(
        &self,
        mut request_metrics: RequestMetrics,
        response: Option<&LlmResponse>,
    ) {
        let completed_at = Instant::now();
        let latency = completed_at.duration_since(request_metrics.started_at);

        request_metrics.completed_at = Some(completed_at);
        request_metrics.latency = Some(latency);
        request_metrics.success = response.is_some();

        if let Some(resp) = response {
            request_metrics.tokens_used = Some(resp.usage.total_tokens as u64);
            // Cost calculation would be done here based on provider pricing
        }

        debug!(
            "Completed request {} for provider: {:?} in {:?}",
            request_metrics.request_id, request_metrics.provider_id, latency
        );

        self.record_request_metrics(request_metrics).await;
    }

    /// Record a failed request
    pub async fn record_failure(&self, mut request_metrics: RequestMetrics, error: String) {
        let completed_at = Instant::now();
        let latency = completed_at.duration_since(request_metrics.started_at);

        request_metrics.completed_at = Some(completed_at);
        request_metrics.latency = Some(latency);
        request_metrics.success = false;
        request_metrics.error = Some(error.clone());

        debug!(
            "Failed request {} for provider: {:?}: {}",
            request_metrics.request_id, request_metrics.provider_id, error
        );

        self.record_request_metrics(request_metrics).await;
    }

    /// Record retry attempt
    pub async fn record_retry(&self, request_metrics: &mut RequestMetrics) {
        request_metrics.retry_count += 1;

        trace!(
            "Recording retry #{} for request {} on provider: {:?}",
            request_metrics.retry_count,
            request_metrics.request_id,
            request_metrics.provider_id
        );
    }

    /// Record circuit breaker trip
    pub async fn record_circuit_breaker_trip(&self, provider_id: &ProviderId) {
        debug!(
            "Recording circuit breaker trip for provider: {:?}",
            provider_id
        );

        if let Some(metrics) = self.metrics.write().await.get_mut(provider_id) {
            metrics.circuit_breaker_trips += 1;
        }
    }

    /// Internal method to record request metrics
    async fn record_request_metrics(&self, request_metrics: RequestMetrics) {
        let provider_id = &request_metrics.provider_id;

        // Update provider metrics
        {
            let mut metrics = self.metrics.write().await;
            let provider_metrics = metrics.entry(provider_id.clone()).or_insert_with(|| {
                let mut m = ProviderMetrics::default();
                m.provider_id = provider_id.clone();
                m
            });

            // Update counters
            provider_metrics.request_count += 1;
            provider_metrics.retry_count += request_metrics.retry_count as u64;
            provider_metrics.last_request_time = Some(request_metrics.started_at);

            if let Some(latency) = request_metrics.latency {
                provider_metrics.total_latency += latency;

                // Update min/max latency
                if latency < provider_metrics.min_latency {
                    provider_metrics.min_latency = latency;
                }
                if latency > provider_metrics.max_latency {
                    provider_metrics.max_latency = latency;
                }
            }

            if request_metrics.success {
                provider_metrics.success_count += 1;
            } else {
                provider_metrics.failure_count += 1;
            }

            if let Some(tokens) = request_metrics.tokens_used {
                provider_metrics.total_tokens_processed += tokens;
            }

            if let Some(cost) = request_metrics.cost {
                provider_metrics.total_cost += cost;
            }

            // Calculate derived metrics
            if provider_metrics.request_count > 0 {
                provider_metrics.avg_latency =
                    provider_metrics.total_latency / provider_metrics.request_count as u32;
                provider_metrics.error_rate = (provider_metrics.failure_count as f32
                    / provider_metrics.request_count as f32)
                    * 100.0;
            }
        }

        // Update historical data
        {
            let mut history = self.history.write().await;
            let provider_history =
                history
                    .entry(provider_id.clone())
                    .or_insert_with(|| PerformanceHistory {
                        latency_samples: VecDeque::new(),
                        request_timestamps: VecDeque::new(),
                        error_events: VecDeque::new(),
                        cost_history: VecDeque::new(),
                        throughput_history: VecDeque::new(),
                    });

            // Add latency sample
            if let Some(latency) = request_metrics.latency {
                provider_history.latency_samples.push_back(latency);
                if provider_history.latency_samples.len() > self.config.max_history_size {
                    provider_history.latency_samples.pop_front();
                }
            }

            // Add request timestamp
            provider_history
                .request_timestamps
                .push_back(request_metrics.started_at);
            if provider_history.request_timestamps.len() > self.config.max_history_size {
                provider_history.request_timestamps.pop_front();
            }

            // Add error event if applicable
            if !request_metrics.success {
                let error_event = ErrorEvent {
                    timestamp: request_metrics.started_at,
                    error_type: "request_failure".to_string(),
                    error_message: request_metrics.error.unwrap_or("Unknown error".to_string()),
                    request_id: Some(request_metrics.request_id),
                };

                provider_history.error_events.push_back(error_event);
                if provider_history.error_events.len() > self.config.max_history_size {
                    provider_history.error_events.pop_front();
                }
            }

            // Add cost data if available
            if let Some(cost) = request_metrics.cost {
                provider_history
                    .cost_history
                    .push_back((request_metrics.started_at, cost));
                if provider_history.cost_history.len() > self.config.max_history_size {
                    provider_history.cost_history.pop_front();
                }
            }
        }

        // Update derived metrics periodically
        self.update_derived_metrics(provider_id).await;
    }

    /// Update derived metrics like percentiles and throughput
    async fn update_derived_metrics(&self, provider_id: &ProviderId) {
        let history = self.history.read().await;
        let mut metrics = self.metrics.write().await;

        if let (Some(provider_history), Some(provider_metrics)) =
            (history.get(provider_id), metrics.get_mut(provider_id))
        {
            // Calculate latency percentiles
            if !provider_history.latency_samples.is_empty() {
                let mut samples: Vec<Duration> =
                    provider_history.latency_samples.iter().cloned().collect();
                samples.sort();

                let len = samples.len();
                provider_metrics.p50_latency = samples[len * 50 / 100];
                provider_metrics.p95_latency = samples[len * 95 / 100];
                provider_metrics.p99_latency = samples[len * 99 / 100];
            }

            // Calculate throughput (requests per second)
            let now = Instant::now();
            let window_start = now - self.config.throughput_window;
            let requests_in_window = provider_history
                .request_timestamps
                .iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count();

            provider_metrics.throughput =
                requests_in_window as f32 / self.config.throughput_window.as_secs() as f32;

            // Calculate quality score (weighted combination of latency, error rate, uptime)
            let latency_score =
                1.0 - (provider_metrics.avg_latency.as_millis() as f32 / 10000.0).min(1.0);
            let reliability_score = 1.0 - (provider_metrics.error_rate / 100.0);
            let uptime_score = provider_metrics.uptime_percentage / 100.0;

            provider_metrics.quality_score =
                (latency_score * 0.3 + reliability_score * 0.5 + uptime_score * 0.2)
                    .clamp(0.0, 1.0);
        }
    }

    /// Get metrics for a specific provider
    pub async fn get_provider_metrics(&self, provider_id: &ProviderId) -> Option<ProviderMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(provider_id).cloned()
    }

    /// Get metrics for all providers
    pub async fn get_all_metrics(&self) -> HashMap<ProviderId, ProviderMetrics> {
        self.metrics.read().await.clone()
    }

    /// Get performance history for a provider
    pub async fn get_performance_history(
        &self,
        provider_id: &ProviderId,
    ) -> Option<PerformanceHistory> {
        let history = self.history.read().await;
        history.get(provider_id).cloned()
    }

    /// Get comparative metrics between providers
    pub async fn get_comparative_metrics(&self) -> ComparativeMetrics {
        let metrics = self.metrics.read().await;

        if metrics.is_empty() {
            return ComparativeMetrics::default();
        }

        let fastest_provider = metrics
            .iter()
            .min_by_key(|(_, m)| m.avg_latency)
            .map(|(id, _)| id.clone());

        let most_reliable_provider = metrics
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.error_rate
                    .partial_cmp(&b.error_rate)
                    .expect("Operation failed - converted from unwrap()")
            })
            .map(|(id, _)| id.clone());

        let cheapest_provider = metrics
            .iter()
            .filter(|(_, m)| m.total_tokens_processed > 0)
            .min_by(|(_, a), (_, b)| {
                let cost_per_token_a = a.total_cost / a.total_tokens_processed as f32;
                let cost_per_token_b = b.total_cost / b.total_tokens_processed as f32;
                cost_per_token_a
                    .partial_cmp(&cost_per_token_b)
                    .expect("Operation failed - converted from unwrap()")
            })
            .map(|(id, _)| id.clone());

        let highest_quality_provider = metrics
            .iter()
            .max_by(|(_, a), (_, b)| {
                a.quality_score
                    .partial_cmp(&b.quality_score)
                    .expect("Operation failed - converted from unwrap()")
            })
            .map(|(id, _)| id.clone());

        ComparativeMetrics {
            fastest_provider,
            most_reliable_provider,
            cheapest_provider,
            highest_quality_provider,
            total_providers: metrics.len(),
            total_requests: metrics.values().map(|m| m.request_count).sum(),
            total_cost: metrics.values().map(|m| m.total_cost).sum(),
        }
    }

    /// Reset metrics for a provider
    pub async fn reset_provider_metrics(&self, provider_id: &ProviderId) {
        debug!("Resetting metrics for provider: {:?}", provider_id);

        {
            let mut metrics = self.metrics.write().await;
            if let Some(provider_metrics) = metrics.get_mut(provider_id) {
                *provider_metrics = ProviderMetrics::default();
                provider_metrics.provider_id = provider_id.clone();
            }
        }

        {
            let mut history = self.history.write().await;
            if let Some(provider_history) = history.get_mut(provider_id) {
                provider_history.latency_samples.clear();
                provider_history.request_timestamps.clear();
                provider_history.error_events.clear();
                provider_history.cost_history.clear();
                provider_history.throughput_history.clear();
            }
        }
    }

    /// Export metrics to JSON
    pub async fn export_metrics(&self) -> Result<String> {
        let metrics = self.get_all_metrics().await;
        serde_json::to_string_pretty(&metrics)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metrics: {}", e))
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Comparative metrics across providers
#[derive(Debug, Clone, Default)]
pub struct ComparativeMetrics {
    pub fastest_provider: Option<ProviderId>,
    pub most_reliable_provider: Option<ProviderId>,
    pub cheapest_provider: Option<ProviderId>,
    pub highest_quality_provider: Option<ProviderId>,
    pub total_providers: usize,
    pub total_requests: u64,
    pub total_cost: f32,
}
