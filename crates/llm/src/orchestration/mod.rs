use crate::providers::{LlmProvider, ProviderWrapper, LlmRequest, LlmResponse};
use crate::{CircuitBreaker, CircuitBreakerState, CostOptimizer};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};

pub mod health_monitor;
pub mod load_balancer;
pub mod request_analyzer;

use health_monitor::HealthMonitor;
use load_balancer::{LoadBalancer, LoadBalancingStrategy};
use request_analyzer::{RequestAnalyzer, RequestComplexity, TaskPriority};

/// Smart orchestration engine for multi-provider LLM management
#[derive(Clone)]
pub struct SmartOrchestrationEngine {
    providers: Vec<ProviderWrapper>,
    health_monitor: Arc<Mutex<HealthMonitor>>,
    load_balancer: Arc<Mutex<LoadBalancer>>,
    request_analyzer: Arc<RequestAnalyzer>,
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    cost_optimizer: Arc<Mutex<CostOptimizer>>,
    config: OrchestrationConfig,
    metrics: Arc<Mutex<OrchestrationMetrics>>,
}

/// Configuration for the orchestration engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    pub health_check_interval: Duration,
    pub load_balancing_strategy: LoadBalancingStrategy,
    pub max_concurrent_requests: usize,
    pub request_timeout: Duration,
    pub retry_config: RetryConfig,
    pub cost_optimization_enabled: bool,
    pub failover_enabled: bool,
    pub circuit_breaker_enabled: bool,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(30),
            load_balancing_strategy: LoadBalancingStrategy::Weighted,
            max_concurrent_requests: 100,
            request_timeout: Duration::from_secs(30),
            retry_config: RetryConfig::default(),
            cost_optimization_enabled: true,
            failover_enabled: true,
            circuit_breaker_enabled: true,
        }
    }
}

/// Retry configuration with adaptive behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter_enabled: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter_enabled: true,
        }
    }
}

/// Comprehensive metrics for orchestration performance
#[derive(Debug, Clone, Default)]
pub struct OrchestrationMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time: f64,
    pub provider_selection_time: f64,
    pub circuit_breaker_trips: u64,
    pub failovers: u64,
    pub cost_savings: f64,
    pub provider_usage: HashMap<String, u64>,
    pub complexity_distribution: HashMap<String, u64>,
    pub error_rates_by_provider: HashMap<String, f64>,
}

impl SmartOrchestrationEngine {
    /// Create new orchestration engine with providers
    pub fn new(providers: Vec<ProviderWrapper>, config: OrchestrationConfig) -> Result<Self> {
        if providers.is_empty() {
            return Err(anyhow!("At least one provider must be configured"));
        }
        
        info!("🚀 Initializing SmartOrchestrationEngine with {} providers", providers.len());
        
        let mut circuit_breakers = HashMap::new();
        for provider in &providers {
            let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
            circuit_breakers.insert(provider_id, CircuitBreaker::default());
        }
        
        let health_monitor = Arc::new(Mutex::new(HealthMonitor::new(
            providers.clone(),
            config.health_check_interval,
        )));
        
        let load_balancer = Arc::new(Mutex::new(LoadBalancer::new(
            providers.clone(),
            config.load_balancing_strategy.clone(),
        )));
        
        let request_analyzer = Arc::new(RequestAnalyzer::new());
        
        Ok(Self {
            providers,
            health_monitor,
            load_balancer,
            request_analyzer,
            circuit_breakers: Arc::new(Mutex::new(circuit_breakers)),
            cost_optimizer: Arc::new(Mutex::new(CostOptimizer::default())),
            config,
            metrics: Arc::new(Mutex::new(OrchestrationMetrics::default())),
        })
    }
    
    /// Start background tasks (health monitoring, metrics collection)
    pub async fn start(&self) -> Result<()> {
        info!("🎯 Starting SmartOrchestrationEngine background tasks");
        
        // Start health monitoring
        let health_monitor = Arc::clone(&self.health_monitor);
        let health_interval = self.config.health_check_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(health_interval);
            loop {
                interval.tick().await;
                if let Ok(mut monitor) = health_monitor.lock() {
                    if let Err(e) = monitor.check_all_providers().await {
                        error!("Health check failed: {}", e);
                    }
                }
            }
        });
        
        // Start metrics collection
        let metrics_arc = Arc::clone(&self.metrics);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Ok(metrics) = metrics_arc.lock() {
                    debug!("📊 Orchestration metrics: {} total requests, {:.2}% success rate",
                        metrics.total_requests,
                        if metrics.total_requests > 0 {
                            (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
                        } else { 0.0 }
                    );
                }
            }
        });
        
        info!("✅ SmartOrchestrationEngine started successfully");
        Ok(())
    }
    
    /// Execute a request with intelligent provider selection and orchestration
    pub async fn execute_request(&self, request: LlmRequest) -> Result<LlmResponse> {
        let start_time = Instant::now();
        
        // Analyze request complexity and priority
        let complexity = self.request_analyzer.analyze_complexity(&request).await?;
        let priority = self.request_analyzer.analyze_priority(&request).await?;
        
        info!("🧠 Request analysis: complexity={:?}, priority={:?}", complexity, priority);
        
        // Select optimal provider based on multiple criteria
        let provider_selection_start = Instant::now();
        let selected_provider = self.select_optimal_provider(&request, &complexity, &priority).await?;
        let provider_selection_time = provider_selection_start.elapsed();
        
        info!("✅ Selected provider: {} (selection time: {:?})", 
            selected_provider.name(), provider_selection_time);
        
        // Execute with retry logic and circuit breaker protection
        let result = self.execute_with_resilience(&selected_provider, request, &complexity).await;
        
        // Update metrics
        self.update_metrics(&selected_provider, &result, start_time.elapsed(), 
                           provider_selection_time, &complexity).await;
        
        result
    }
    
    /// Select the optimal provider based on multiple criteria
    async fn select_optimal_provider(
        &self,
        request: &LlmRequest,
        complexity: &RequestComplexity,
        priority: &TaskPriority,
    ) -> Result<ProviderWrapper> {
        // Get healthy providers first
        let healthy_providers = self.get_healthy_providers().await;
        if healthy_providers.is_empty() {
            return Err(anyhow!("No healthy providers available"));
        }
        
        // Apply complexity filtering - ensure provider can handle the task
        let capable_providers = self.filter_by_capability(&healthy_providers, complexity).await;
        if capable_providers.is_empty() {
            warn!("No providers capable of handling complexity {:?}, using any healthy provider", complexity);
            return Ok(healthy_providers[0].clone());
        }
        
        // For critical priority, prefer premium providers
        if matches!(priority, TaskPriority::Critical) {
            let premium_providers = self.filter_premium_providers(&capable_providers).await;
            if !premium_providers.is_empty() {
                info!("🏆 Using premium provider for CRITICAL priority task");
                return Ok(premium_providers[0].clone());
            }
        }
        
        // Apply cost optimization if enabled
        if self.config.cost_optimization_enabled {
            if let Ok(cost_optimizer) = self.cost_optimizer.lock() {
                let cost_optimal = cost_optimizer.select_cheapest_provider(&capable_providers, request);
                if let Some(provider) = cost_optimal {
                    info!("💰 Selected cost-optimal provider");
                    return Ok(provider);
                }
            }
        }
        
        // Fallback to load balancer
        if let Ok(mut load_balancer) = self.load_balancer.lock() {
            let balanced_provider = load_balancer.select_provider(&capable_providers).await?;
            info!("⚖️ Selected load-balanced provider");
            return Ok(balanced_provider);
        }
        
        // Final fallback - first capable provider
        Ok(capable_providers[0].clone())
    }
    
    /// Get list of healthy providers (not in circuit breaker OPEN state)
    async fn get_healthy_providers(&self) -> Vec<ProviderWrapper> {
        let mut healthy = Vec::new();
        
        if let Ok(circuit_breakers) = self.circuit_breakers.lock() {
            for provider in &self.providers {
                let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
                
                if let Some(cb) = circuit_breakers.get(&provider_id) {
                    if cb.state != CircuitBreakerState::Open {
                        healthy.push(provider.clone());
                    }
                } else {
                    // Provider without circuit breaker - assume healthy
                    healthy.push(provider.clone());
                }
            }
        }
        
        debug!("🏥 Healthy providers: {}/{}", healthy.len(), self.providers.len());
        healthy
    }
    
    /// Filter providers by capability to handle request complexity
    async fn filter_by_capability(
        &self,
        providers: &[ProviderWrapper],
        complexity: &RequestComplexity,
    ) -> Vec<ProviderWrapper> {
        let mut capable = Vec::new();
        
        for provider in providers {
            let capabilities = provider.capabilities();
            
            let can_handle = match complexity {
                RequestComplexity::Simple => true, // Any provider can handle simple requests
                RequestComplexity::Medium => {
                    // Medium complexity needs decent models
                    capabilities.max_tokens >= 2048 && capabilities.context_window >= 4096
                }
                RequestComplexity::Complex | RequestComplexity::Expert => {
                    // Complex tasks need high-capacity models
                    capabilities.max_tokens >= 4096 && 
                    capabilities.context_window >= 8192 &&
                    capabilities.reliability_score >= 0.9
                }
            };
            
            if can_handle {
                capable.push(provider.clone());
            }
        }
        
        debug!("🧠 Capable providers for {:?}: {}/{}", complexity, capable.len(), providers.len());
        capable
    }
    
    /// Filter premium providers for high-priority tasks
    async fn filter_premium_providers(&self, providers: &[ProviderWrapper]) -> Vec<ProviderWrapper> {
        let mut premium = Vec::new();
        
        for provider in providers {
            let capabilities = provider.capabilities();
            let id = provider.id();
            
            // Define premium providers based on quality and capabilities
            let is_premium = match id.provider_type.as_str() {
                "openai" => id.model.contains("gpt-4") && !id.model.contains("mini"),
                "anthropic" => id.model.contains("opus") || id.model.contains("sonnet"),
                "groq" => id.model.contains("70b"),
                _ => capabilities.reliability_score >= 0.95,
            };
            
            if is_premium {
                premium.push(provider.clone());
            }
        }
        
        debug!("🏆 Premium providers: {}/{}", premium.len(), providers.len());
        premium
    }
    
    /// Execute request with resilience patterns (retry, circuit breaker, timeout)
    async fn execute_with_resilience(
        &self,
        provider: &ProviderWrapper,
        request: LlmRequest,
        _complexity: &RequestComplexity,
    ) -> Result<LlmResponse> {
        let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
        let mut last_error = None;
        
        for attempt in 0..=self.config.retry_config.max_retries {
            // Check circuit breaker
            let can_execute = if self.config.circuit_breaker_enabled {
                if let Ok(mut circuit_breakers) = self.circuit_breakers.lock() {
                    circuit_breakers.get_mut(&provider_id)
                        .map(|cb| cb.can_execute())
                        .unwrap_or(true)
                } else {
                    true
                }
            } else {
                true
            };
            
            if !can_execute {
                debug!("🚫 Circuit breaker blocked request to {}", provider_id);
                return Err(anyhow!("Circuit breaker open for provider: {}", provider_id));
            }
            
            // Execute with timeout
            let request_future = provider.complete(request.clone());
            let timeout_future = sleep(self.config.request_timeout);
            
            let result = tokio::select! {
                result = request_future => result,
                _ = timeout_future => Err(anyhow!("Request timeout after {:?}", self.config.request_timeout)),
            };
            
            match result {
                Ok(response) => {
                    // Record success in circuit breaker
                    if self.config.circuit_breaker_enabled {
                        if let Ok(mut circuit_breakers) = self.circuit_breakers.lock() {
                            if let Some(cb) = circuit_breakers.get_mut(&provider_id) {
                                cb.record_success();
                            }
                        }
                    }
                    
                    info!("✅ Request successful on attempt {} to {}", 
                        attempt + 1, provider_id);
                    return Ok(response);
                }
                Err(e) => {
                    // Record failure in circuit breaker
                    if self.config.circuit_breaker_enabled {
                        if let Ok(mut circuit_breakers) = self.circuit_breakers.lock() {
                            if let Some(cb) = circuit_breakers.get_mut(&provider_id) {
                                cb.record_failure();
                            }
                        }
                    }
                    
                    last_error = Some(e);
                    
                    if attempt < self.config.retry_config.max_retries {
                        let delay = self.calculate_retry_delay(attempt);
                        warn!("❌ Attempt {} failed for {}: {}. Retrying in {:?}", 
                            attempt + 1, provider_id, last_error.as_ref().unwrap(), delay);
                        sleep(delay).await;
                    } else {
                        error!("💥 All {} attempts failed for {}", 
                            self.config.retry_config.max_retries + 1, provider_id);
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("Max retries exceeded")))
    }
    
    /// Calculate retry delay with exponential backoff and optional jitter
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.retry_config.base_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.config.retry_config.exponential_base.powi(attempt as i32);
        
        let delay_ms = if self.config.retry_config.jitter_enabled {
            // Add ±25% jitter to avoid thundering herd
            let jitter = fastrand::f64() * 0.5 - 0.25; // -0.25 to +0.25
            exponential_delay * (1.0 + jitter)
        } else {
            exponential_delay
        };
        
        let capped_delay = delay_ms.min(self.config.retry_config.max_delay.as_millis() as f64);
        Duration::from_millis(capped_delay as u64)
    }
    
    /// Update orchestration metrics
    async fn update_metrics(
        &self,
        provider: &ProviderWrapper,
        result: &Result<LlmResponse>,
        total_time: Duration,
        selection_time: Duration,
        complexity: &RequestComplexity,
    ) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_requests += 1;
            
            let provider_name = provider.name();
            *metrics.provider_usage.entry(provider_name.clone()).or_insert(0) += 1;
            
            let complexity_key = format!("{:?}", complexity);
            *metrics.complexity_distribution.entry(complexity_key).or_insert(0) += 1;
            
            // Update response time running average
            let n = metrics.total_requests as f64;
            metrics.avg_response_time = (metrics.avg_response_time * (n - 1.0) + total_time.as_millis() as f64) / n;
            
            // Update selection time running average
            metrics.provider_selection_time = (metrics.provider_selection_time * (n - 1.0) + selection_time.as_millis() as f64) / n;
            
            match result {
                Ok(_) => {
                    metrics.successful_requests += 1;
                }
                Err(e) => {
                    metrics.failed_requests += 1;
                    
                    // Update error rate for this provider
                    let current_usage = metrics.provider_usage.get(&provider_name).copied().unwrap_or(0) as f64;
                    let current_errors = metrics.error_rates_by_provider.entry(provider_name).or_insert(0.0);
                    *current_errors = (*current_errors * (current_usage - 1.0) + 1.0) / current_usage;
                    
                    debug!("❌ Request failed: {}", e);
                }
            }
        }
    }
    
    /// Get comprehensive status report
    pub async fn get_status_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("🚀 Smart Orchestration Engine Status\n");
        report.push_str("=====================================\n\n");
        
        // Overall metrics
        if let Ok(metrics) = self.metrics.lock() {
            let success_rate = if metrics.total_requests > 0 {
                (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
            } else {
                0.0
            };
            
            report.push_str(&format!("📊 Overall Metrics:\n"));
            report.push_str(&format!("  • Total Requests: {}\n", metrics.total_requests));
            report.push_str(&format!("  • Success Rate: {:.2}%\n", success_rate));
            report.push_str(&format!("  • Avg Response Time: {:.0}ms\n", metrics.avg_response_time));
            report.push_str(&format!("  • Avg Selection Time: {:.0}ms\n", metrics.provider_selection_time));
            report.push_str(&format!("  • Circuit Breaker Trips: {}\n", metrics.circuit_breaker_trips));
            report.push_str(&format!("  • Failovers: {}\n\n", metrics.failovers));
            
            // Provider usage distribution
            report.push_str("🔌 Provider Usage:\n");
            let mut sorted_usage: Vec<_> = metrics.provider_usage.iter().collect();
            sorted_usage.sort_by(|a, b| b.1.cmp(a.1));
            
            for (provider, usage) in sorted_usage {
                let usage_percent = (*usage as f64 / metrics.total_requests as f64) * 100.0;
                let error_rate = metrics.error_rates_by_provider.get(provider).copied().unwrap_or(0.0) * 100.0;
                report.push_str(&format!("  • {}: {} requests ({:.1}%, {:.1}% errors)\n", 
                    provider, usage, usage_percent, error_rate));
            }
            
            // Complexity distribution
            report.push_str("\n🧠 Request Complexity Distribution:\n");
            let mut sorted_complexity: Vec<_> = metrics.complexity_distribution.iter().collect();
            sorted_complexity.sort_by(|a, b| b.1.cmp(a.1));
            
            for (complexity, count) in sorted_complexity {
                let percent = (*count as f64 / metrics.total_requests as f64) * 100.0;
                report.push_str(&format!("  • {}: {} requests ({:.1}%)\n", complexity, count, percent));
            }
        }
        
        // Health status
        report.push_str("\n🏥 Provider Health Status:\n");
        if let Ok(health_monitor) = self.health_monitor.lock() {
            let health_report = health_monitor.get_health_report();
            report.push_str(&health_report);
        }
        
        report
    }
}

/// Add fastrand for jitter calculation
impl Default for OrchestrationMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time: 0.0,
            provider_selection_time: 0.0,
            circuit_breaker_trips: 0,
            failovers: 0,
            cost_savings: 0.0,
            provider_usage: HashMap::new(),
            complexity_distribution: HashMap::new(),
            error_rates_by_provider: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ProviderConfig, ProviderFactory};

    #[tokio::test]
    async fn test_orchestration_engine_creation() {
        let config1 = ProviderConfig::new("openai", "gpt-4o-mini")
            .with_api_key("test-key".to_string());
        let config2 = ProviderConfig::new("anthropic", "claude-3-haiku-20240307")
            .with_api_key("test-key".to_string());
        
        let provider1 = ProviderFactory::create_provider(&config1).unwrap();
        let provider2 = ProviderFactory::create_provider(&config2).unwrap();
        
        let providers = vec![provider1, provider2];
        let config = OrchestrationConfig::default();
        
        let engine = SmartOrchestrationEngine::new(providers, config).unwrap();
        assert_eq!(engine.providers.len(), 2);
    }
}