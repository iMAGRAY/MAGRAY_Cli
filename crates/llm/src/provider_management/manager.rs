//! Provider Manager - Central component for dynamic LLM provider management

use super::*;
use crate::providers::{
    LlmProvider, LlmRequest, LlmResponse, ProviderConfig, ProviderFactory, ProviderId,
    ProviderWrapper,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

/// Central provider management system
pub struct ProviderManager {
    /// Provider registry
    registry: Arc<ProviderRegistry>,
    /// Health monitor
    health_monitor: Arc<HealthMonitor>,
    /// Metrics collector
    metrics_collector: Arc<MetricsCollector>,
    /// Available selection strategies
    strategies: Arc<RwLock<HashMap<String, Box<dyn SelectionStrategy>>>>,
    /// Available fallback policies
    policies: Arc<RwLock<HashMap<String, Box<dyn FallbackPolicy>>>>,
    /// Configuration
    config: ProviderManagementConfig,
    /// Current load tracking
    current_load: Arc<RwLock<HashMap<ProviderId, f32>>>,
    /// Background task handles
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

impl std::fmt::Debug for ProviderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderManager")
            .field("registry", &self.registry)
            .field("health_monitor", &"<HealthMonitor>")
            .field("metrics_collector", &"<MetricsCollector>")
            .field("config", &self.config)
            .field(
                "strategies",
                &format!(
                    "{} strategies",
                    self.strategies.try_read().map_or(0, |s| s.len())
                ),
            )
            .field(
                "policies",
                &format!(
                    "{} policies",
                    self.policies.try_read().map_or(0, |p| p.len())
                ),
            )
            .finish()
    }
}

impl ProviderManager {
    /// Create a new provider manager
    pub fn new() -> Self {
        Self::new_with_config(ProviderManagementConfig::default())
    }

    /// Create a new provider manager with custom configuration
    pub fn new_with_config(config: ProviderManagementConfig) -> Self {
        info!("Creating ProviderManager with config: {:?}", config);

        let registry = Arc::new(ProviderRegistry::new());
        let health_monitor = Arc::new(HealthMonitor::new());
        let metrics_collector = Arc::new(MetricsCollector::new());

        let mut strategies: HashMap<String, Box<dyn SelectionStrategy>> = HashMap::new();
        strategies.insert(
            "name_based".to_string(),
            Box::new(NameBasedStrategy::default()),
        );
        strategies.insert(
            "performance_based".to_string(),
            Box::new(PerformanceBasedStrategy::default()),
        );
        strategies.insert(
            "availability_based".to_string(),
            Box::new(AvailabilityBasedStrategy::default()),
        );
        strategies.insert(
            "cost_based".to_string(),
            Box::new(CostBasedStrategy::default()),
        );
        strategies.insert(
            "load_balanced".to_string(),
            Box::new(LoadBalancedStrategy::default()),
        );

        let mut policies: HashMap<String, Box<dyn FallbackPolicy>> = HashMap::new();
        policies.insert("cascade".to_string(), Box::new(CascadePolicy::default()));
        policies.insert(
            "round_robin".to_string(),
            Box::new(RoundRobinPolicy::default()),
        );
        policies.insert("priority".to_string(), Box::new(PriorityPolicy::default()));
        policies.insert("adaptive".to_string(), Box::new(AdaptivePolicy::default()));
        policies.insert(
            "circuit_breaker".to_string(),
            Box::new(CircuitBreakerPolicy::default()),
        );

        Self {
            registry,
            health_monitor,
            metrics_collector,
            strategies: Arc::new(RwLock::new(strategies)),
            policies: Arc::new(RwLock::new(policies)),
            config,
            current_load: Arc::new(RwLock::new(HashMap::new())),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize the provider manager
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing ProviderManager");

        // Start background monitoring tasks
        self.start_background_tasks().await?;

        info!("ProviderManager initialized successfully");
        Ok(())
    }

    /// Register a new provider
    #[instrument(skip(self, provider))]
    pub async fn register_provider(
        &self,
        provider: Arc<ProviderWrapper>,
        tags: Vec<String>,
        priority: i32,
    ) -> Result<()> {
        let provider_id = provider.id();
        info!("Registering provider: {:?}", provider_id);

        // Register with registry
        self.registry
            .register_provider(Arc::clone(&provider), tags, priority)
            .await?;

        // Initialize metrics
        self.metrics_collector
            .initialize_provider(provider_id.clone())
            .await;

        // Start health monitoring
        self.health_monitor
            .start_monitoring(
                provider_id.clone(),
                provider as Arc<dyn LlmProvider + Send + Sync>,
            )
            .await;

        // Initialize load tracking
        self.current_load
            .write()
            .await
            .insert(provider_id.clone(), 0.0);

        info!("Successfully registered provider: {:?}", provider_id);
        Ok(())
    }

    /// Register multiple providers from configuration
    #[instrument(skip(self, configs))]
    pub async fn register_providers_from_configs(
        &self,
        configs: Vec<ProviderConfig>,
    ) -> Result<()> {
        info!("Registering {} providers from configuration", configs.len());

        for config in configs {
            match ProviderFactory::create_provider(&config) {
                Ok(provider) => {
                    let provider_arc = Arc::new(provider);
                    self.register_provider(provider_arc, vec![config.provider_type.clone()], 0)
                        .await?;
                }
                Err(e) => {
                    error!(
                        "Failed to create provider from config: {:?}, error: {}",
                        config, e
                    );
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Deregister a provider
    #[instrument(skip(self))]
    pub async fn deregister_provider(&self, provider_id: &ProviderId) -> Result<()> {
        info!("Deregistering provider: {:?}", provider_id);

        // Stop health monitoring
        self.health_monitor.stop_monitoring(provider_id).await;

        // Remove from registry (will wait for active requests)
        self.registry.deregister_provider(provider_id).await?;

        // Clean up load tracking
        self.current_load.write().await.remove(provider_id);

        info!("Successfully deregistered provider: {:?}", provider_id);
        Ok(())
    }

    /// Execute a request with automatic provider selection and fallback
    #[instrument(skip(self, request))]
    pub async fn execute_request(
        &self,
        request: LlmRequest,
        criteria: Option<SelectionCriteria>,
    ) -> Result<LlmResponse> {
        let criteria = criteria.unwrap_or_default();
        debug!("Executing request with criteria: {:?}", criteria);

        // Build selection context
        let context = self.build_selection_context(&criteria).await?;

        // Select strategy
        let strategy_name = criteria
            .model_preference
            .as_ref()
            .and_then(|m| {
                if m.contains("strategy:") {
                    Some(
                        m.strip_prefix("strategy:")
                            .unwrap_or(&self.config.default_strategy),
                    )
                } else {
                    None
                }
            })
            .unwrap_or(&self.config.default_strategy);

        // Get strategy reference
        let strategies_lock = self.strategies.read().await;
        let strategy = strategies_lock
            .get(strategy_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown selection strategy: {}", strategy_name))?;

        // Select provider
        let selection = strategy.select_provider(&context).await?;
        info!(
            "Selected provider: {:?} (confidence: {:.2}, reason: {})",
            selection.provider_id, selection.confidence, selection.reasoning
        );

        // Execute with fallback support
        self.execute_with_fallback(request, selection, &criteria)
            .await
    }

    /// Execute request with fallback handling
    #[instrument(skip(self, request, selection, criteria))]
    async fn execute_with_fallback(
        &self,
        request: LlmRequest,
        selection: SelectionResult,
        criteria: &SelectionCriteria,
    ) -> Result<LlmResponse> {
        let mut current_provider = selection.provider_id.clone();
        let mut fallback_chain = selection.fallback_chain;
        let mut retry_count = 0;
        let mut failed_providers = Vec::new();

        // Get fallback policy
        let policy_name = &self.config.default_fallback_policy;
        // Get policy reference
        let policies_lock = self.policies.read().await;
        let policy = policies_lock
            .get(policy_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown fallback policy: {}", policy_name))?;

        loop {
            // Get provider from registry
            let provider = self
                .registry
                .get_provider(&current_provider)
                .await
                .ok_or_else(|| anyhow::anyhow!("Provider not found: {:?}", current_provider))?;

            // Check if provider is healthy and available
            if !self
                .health_monitor
                .can_execute_request(&current_provider)
                .await
            {
                warn!(
                    "Provider {:?} failed health check, trying fallback",
                    current_provider
                );
                failed_providers
                    .push((current_provider.clone(), "Health check failed".to_string()));

                if fallback_chain.is_empty() {
                    return Err(anyhow::anyhow!("No healthy providers available"));
                }

                current_provider = fallback_chain.remove(0);
                continue;
            }

            // Track request start
            let request_metrics = self
                .metrics_collector
                .start_request(&current_provider)
                .await;

            // Update load
            self.increment_load(&current_provider).await;

            // Execute request
            match self
                .execute_single_request(&*provider, &request, &current_provider)
                .await
            {
                Ok(response) => {
                    // Success path
                    info!(
                        "Request completed successfully with provider: {:?}",
                        current_provider
                    );

                    // Record success metrics
                    self.metrics_collector
                        .complete_request(request_metrics, Some(&response))
                        .await;
                    self.health_monitor
                        .record_success(&current_provider, response.response_time)
                        .await;

                    // Update load
                    self.decrement_load(&current_provider).await;

                    return Ok(response);
                }
                Err(error) => {
                    // Failure path
                    warn!(
                        "Request failed with provider: {:?}: {}",
                        current_provider, error
                    );

                    // Record failure metrics
                    self.metrics_collector
                        .record_failure(request_metrics, error.to_string())
                        .await;
                    self.health_monitor
                        .record_failure(&current_provider, error.to_string())
                        .await;

                    // Update load
                    self.decrement_load(&current_provider).await;

                    // Build fallback context
                    let available_providers = self.get_available_providers_map().await;
                    let mut fallback_context = FallbackContext {
                        request: request.clone(),
                        primary_provider: current_provider.clone(),
                        fallback_chain: fallback_chain.clone(),
                        retry_count,
                        max_retries: self.config.max_retries,
                        failed_providers: failed_providers.clone(),
                        execution_context: ExecutionContext {
                            provider_id: current_provider.clone(),
                            request: request.clone(),
                            timeout: self.config.request_timeout,
                            retry_count,
                            started_at: Instant::now(),
                        },
                        available_providers,
                    };

                    // Handle failure with policy
                    match policy.handle_failure(&mut fallback_context, error).await {
                        Ok(FallbackAction::Retry {
                            delay,
                            modified_request,
                        }) => {
                            info!(
                                "Retrying same provider: {:?} after {:?}",
                                current_provider, delay
                            );
                            retry_count += 1;

                            // Apply delay
                            if delay > Duration::from_millis(0) {
                                tokio::time::sleep(delay).await;
                            }

                            // Use modified request if provided
                            if let Some(modified) = modified_request {
                                return Box::pin(self.execute_with_fallback(
                                    modified,
                                    SelectionResult {
                                        provider_id: current_provider,
                                        confidence: 0.5,
                                        reasoning: "Retry with modified request".to_string(),
                                        fallback_chain: fallback_context.fallback_chain,
                                    },
                                    criteria,
                                ))
                                .await;
                            }
                        }
                        Ok(FallbackAction::Switch {
                            provider_id,
                            delay,
                            modified_request,
                        }) => {
                            info!("Switching to provider: {:?} after {:?}", provider_id, delay);

                            current_provider = provider_id;
                            fallback_chain = fallback_context.fallback_chain;
                            failed_providers = fallback_context.failed_providers;
                            retry_count = fallback_context.retry_count;

                            // Apply delay
                            if delay > Duration::from_millis(0) {
                                tokio::time::sleep(delay).await;
                            }

                            // Use modified request if provided
                            if let Some(modified) = modified_request {
                                return Box::pin(self.execute_with_fallback(
                                    modified,
                                    SelectionResult {
                                        provider_id: current_provider,
                                        confidence: 0.5,
                                        reasoning: "Switch with modified request".to_string(),
                                        fallback_chain,
                                    },
                                    criteria,
                                ))
                                .await;
                            }
                        }
                        Ok(FallbackAction::Fail { reason }) => {
                            error!("Fallback policy determined to fail: {}", reason);
                            return Err(anyhow::anyhow!(
                                "Request failed after fallback: {}",
                                reason
                            ));
                        }
                        Ok(FallbackAction::CustomRecovery { action: _ }) => {
                            // Custom recovery not implemented in this basic version
                            warn!("Custom recovery not implemented, treating as failure");
                            return Err(anyhow::anyhow!("Custom recovery not supported"));
                        }
                        Err(policy_error) => {
                            error!("Fallback policy failed: {}", policy_error);
                            return Err(anyhow::anyhow!("Fallback policy error: {}", policy_error));
                        }
                    }
                }
            }
        }
    }

    /// Execute a single request with a specific provider
    #[instrument(skip(self, provider, request))]
    async fn execute_single_request(
        &self,
        provider: &dyn LlmProvider,
        request: &LlmRequest,
        provider_id: &ProviderId,
    ) -> Result<LlmResponse> {
        debug!("Executing single request with provider: {:?}", provider_id);

        // Validate request
        provider.validate_request(request)?;

        // Execute with timeout
        let result = tokio::time::timeout(
            self.config.request_timeout,
            provider.complete(request.clone()),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                debug!("Request completed successfully");
                Ok(response)
            }
            Ok(Err(error)) => {
                debug!("Request failed: {}", error);
                Err(error)
            }
            Err(_) => {
                warn!("Request timed out after {:?}", self.config.request_timeout);
                Err(anyhow::anyhow!("Request timeout"))
            }
        }
    }

    /// Build selection context for strategy evaluation
    async fn build_selection_context(
        &self,
        criteria: &SelectionCriteria,
    ) -> Result<SelectionContext> {
        let available_providers = self.registry.list_enabled_providers().await;

        if available_providers.is_empty() {
            return Err(anyhow::anyhow!("No providers available"));
        }

        let provider_metrics = self.metrics_collector.get_all_metrics().await;
        let provider_health = self.health_monitor.get_all_health_status().await;
        let current_load = self.current_load.read().await.clone();

        Ok(SelectionContext {
            criteria: criteria.clone(),
            available_providers,
            provider_metrics,
            provider_health,
            current_load,
        })
    }

    /// Get map of available providers for fallback context
    async fn get_available_providers_map(
        &self,
    ) -> HashMap<ProviderId, Arc<dyn LlmProvider + Send + Sync>> {
        let provider_ids = self.registry.list_enabled_providers().await;
        let mut providers = HashMap::new();

        for id in provider_ids {
            if let Some(provider) = self.registry.get_provider(&id).await {
                providers.insert(id, provider as Arc<dyn LlmProvider + Send + Sync>);
            }
        }

        providers
    }

    /// Increment load counter for a provider
    async fn increment_load(&self, provider_id: &ProviderId) {
        let mut load = self.current_load.write().await;
        let current = load.get(provider_id).copied().unwrap_or(0.0);
        load.insert(provider_id.clone(), current + 1.0);
    }

    /// Decrement load counter for a provider
    async fn decrement_load(&self, provider_id: &ProviderId) {
        let mut load = self.current_load.write().await;
        let current = load.get(provider_id).copied().unwrap_or(0.0);
        load.insert(provider_id.clone(), (current - 1.0).max(0.0));
    }

    /// Start background monitoring tasks
    async fn start_background_tasks(&self) -> Result<()> {
        debug!("Starting background tasks");

        // Cleanup task
        let registry = Arc::clone(&self.registry);
        let cleanup_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                registry.cleanup().await;
            }
        });

        self.background_tasks.write().await.push(cleanup_handle);

        info!("Background tasks started");
        Ok(())
    }

    /// Get comprehensive status report
    pub async fn get_status_report(&self) -> ProviderManagerStatus {
        let registry_stats = self.registry.get_statistics().await;
        let health_stats = self.health_monitor.get_statistics().await;
        let comparative_metrics = self.metrics_collector.get_comparative_metrics().await;
        let current_load = self.current_load.read().await.clone();

        ProviderManagerStatus {
            total_providers: registry_stats.total_providers,
            enabled_providers: registry_stats.enabled_providers,
            healthy_providers: health_stats.healthy_providers,
            active_requests: registry_stats.active_requests,
            total_requests: registry_stats.total_requests,
            current_load,
            comparative_metrics,
            uptime: registry_stats.registry_uptime,
        }
    }

    /// Add a new selection strategy
    pub async fn add_strategy(&self, name: String, strategy: Box<dyn SelectionStrategy>) {
        info!("Adding selection strategy: {}", name);
        self.strategies.write().await.insert(name, strategy);
    }

    /// Add a new fallback policy
    pub async fn add_policy(&self, name: String, policy: Box<dyn FallbackPolicy>) {
        info!("Adding fallback policy: {}", name);
        self.policies.write().await.insert(name, policy);
    }

    /// List available strategies
    pub async fn list_strategies(&self) -> Vec<String> {
        self.strategies.read().await.keys().cloned().collect()
    }

    /// List available policies
    pub async fn list_policies(&self) -> Vec<String> {
        self.policies.read().await.keys().cloned().collect()
    }

    /// Get provider health status
    pub async fn get_provider_health(
        &self,
        provider_id: &ProviderId,
    ) -> Option<ProviderHealthStatus> {
        self.health_monitor.get_health_status(provider_id).await
    }

    /// Get provider metrics
    pub async fn get_provider_metrics(&self, provider_id: &ProviderId) -> Option<ProviderMetrics> {
        self.metrics_collector
            .get_provider_metrics(provider_id)
            .await
    }

    /// Enable/disable a provider
    pub async fn set_provider_enabled(
        &self,
        provider_id: &ProviderId,
        enabled: bool,
    ) -> Result<()> {
        self.registry
            .set_provider_enabled(provider_id, enabled)
            .await
    }

    /// Update provider priority
    pub async fn set_provider_priority(
        &self,
        provider_id: &ProviderId,
        priority: i32,
    ) -> Result<()> {
        self.registry
            .set_provider_priority(provider_id, priority)
            .await
    }

    /// Shutdown the provider manager
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down ProviderManager");

        // Cancel background tasks
        let mut tasks = self.background_tasks.write().await;
        for handle in tasks.drain(..) {
            handle.abort();
        }

        // Stop health monitoring for all providers
        let provider_ids = self.registry.list_providers().await;
        for provider_id in provider_ids {
            self.health_monitor.stop_monitoring(&provider_id).await;
        }

        info!("ProviderManager shutdown complete");
        Ok(())
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider manager status information
#[derive(Debug, Clone)]
pub struct ProviderManagerStatus {
    pub total_providers: usize,
    pub enabled_providers: usize,
    pub healthy_providers: usize,
    pub active_requests: u32,
    pub total_requests: u64,
    pub current_load: HashMap<ProviderId, f32>,
    pub comparative_metrics: ComparativeMetrics,
    pub uptime: Instant,
}
