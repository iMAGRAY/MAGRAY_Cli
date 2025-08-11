use crate::providers::{ProviderWrapper, ProviderHealth};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

/// Monitors health of all providers and maintains health metrics
pub struct HealthMonitor {
    providers: Vec<ProviderWrapper>,
    health_status: HashMap<String, ProviderHealthStatus>,
    check_interval: Duration,
    last_check: Option<Instant>,
    health_history: HashMap<String, Vec<HealthCheckResult>>,
    max_history_size: usize,
}

/// Detailed health status for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthStatus {
    pub current_health: ProviderHealth,
    pub consecutive_failures: u32,
    pub last_success: Option<Instant>,
    pub last_failure: Option<Instant>,
    pub success_rate: f64,
    pub avg_response_time: f64,
    pub uptime_percentage: f64,
    pub is_temporarily_disabled: bool,
}

impl Default for ProviderHealthStatus {
    fn default() -> Self {
        Self {
            current_health: ProviderHealth::Healthy,
            consecutive_failures: 0,
            last_success: None,
            last_failure: None,
            success_rate: 1.0,
            avg_response_time: 0.0,
            uptime_percentage: 100.0,
            is_temporarily_disabled: false,
        }
    }
}

/// Result of a single health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub timestamp: Instant,
    pub health: ProviderHealth,
    pub response_time: Duration,
    pub error_message: Option<String>,
}

impl HealthMonitor {
    pub fn new(providers: Vec<ProviderWrapper>, check_interval: Duration) -> Self {
        let mut health_status = HashMap::new();
        let mut health_history = HashMap::new();
        
        for provider in &providers {
            let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
            health_status.insert(provider_id.clone(), ProviderHealthStatus::default());
            health_history.insert(provider_id, Vec::new());
        }
        
        Self {
            providers,
            health_status,
            check_interval,
            last_check: None,
            health_history,
            max_history_size: 100, // Keep last 100 health checks per provider
        }
    }
    
    /// Check health of all providers
    pub async fn check_all_providers(&mut self) -> Result<()> {
        let start_time = Instant::now();
        info!("🏥 Starting health check for {} providers", self.providers.len());
        
        let mut futures = Vec::new();
        for provider in &self.providers {
            let provider_clone = provider.clone();
            futures.push(tokio::spawn(async move {
                Self::check_single_provider_health(provider_clone).await
            }));
        }
        
        let results = futures::future::join_all(futures).await;
        
        let mut successful_checks = 0;
        let mut total_checks = 0;
        
        for (provider, result) in self.providers.iter().zip(results.iter()) {
            total_checks += 1;
            let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
            
            match result {
                Ok(Ok((health, response_time))) => {
                    successful_checks += 1;
                    self.update_provider_health(&provider_id, health.clone(), *response_time, None);
                    
                    let health_result = HealthCheckResult {
                        timestamp: start_time,
                        health: health.clone(),
                        response_time: *response_time,
                        error_message: None,
                    };
                    self.add_to_history(&provider_id, health_result);
                    
                    debug!("✅ Health check passed for {}: {:?} ({:?})", 
                        provider_id, health, response_time);
                }
                Ok(Err(e)) => {
                    let error_msg = e.to_string();
                    self.update_provider_health(&provider_id, &ProviderHealth::Unavailable, 
                        Duration::from_secs(0), Some(error_msg.clone()));
                    
                    let health_result = HealthCheckResult {
                        timestamp: start_time,
                        health: ProviderHealth::Unavailable,
                        response_time: Duration::from_secs(0),
                        error_message: Some(error_msg),
                    };
                    self.add_to_history(&provider_id, health_result);
                    
                    warn!("❌ Health check failed for {}: {}", provider_id, e);
                }
                Err(e) => {
                    let error_msg = format!("Health check task failed: {}", e);
                    self.update_provider_health(&provider_id, &ProviderHealth::Unavailable, 
                        Duration::from_secs(0), Some(error_msg.clone()));
                    
                    error!("💥 Health check task panicked for {}: {}", provider_id, e);
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        self.last_check = Some(start_time);
        
        info!("🏥 Health check completed: {}/{} providers healthy ({:?})", 
            successful_checks, total_checks, elapsed);
        
        Ok(())
    }
    
    /// Check health of a single provider
    async fn check_single_provider_health(provider: ProviderWrapper) -> Result<(ProviderHealth, Duration)> {
        let start_time = Instant::now();
        
        // Add timeout to prevent hanging health checks
        let timeout = Duration::from_secs(10);
        let health_check_future = provider.health_check();
        
        let health = tokio::select! {
            result = health_check_future => result?,
            _ = sleep(timeout) => {
                warn!("⏰ Health check timed out for {}", provider.name());
                ProviderHealth::Degraded
            }
        };
        
        let response_time = start_time.elapsed();
        Ok((health, response_time))
    }
    
    /// Update health status for a provider
    fn update_provider_health(
        &mut self,
        provider_id: &str,
        health: &ProviderHealth,
        response_time: Duration,
        error_message: Option<String>,
    ) {
        if let Some(status) = self.health_status.get_mut(provider_id) {
            let now = Instant::now();
            
            // Update current health
            status.current_health = health.clone();
            
            // Update consecutive failures
            match health {
                ProviderHealth::Healthy => {
                    status.consecutive_failures = 0;
                    status.last_success = Some(now);
                }
                ProviderHealth::Degraded => {
                    // Don't count as failure, but don't reset counter
                    status.last_success = Some(now);
                }
                ProviderHealth::Unavailable => {
                    status.consecutive_failures += 1;
                    status.last_failure = Some(now);
                }
            }
            
            // Update response time (running average)
            if response_time > Duration::from_millis(0) {
                if status.avg_response_time == 0.0 {
                    status.avg_response_time = response_time.as_millis() as f64;
                } else {
                    // Exponential moving average with α = 0.3
                    status.avg_response_time = status.avg_response_time * 0.7 + 
                        (response_time.as_millis() as f64) * 0.3;
                }
            }
            
            // Calculate success rate from recent history
            if let Some(history) = self.health_history.get(provider_id) {
                let recent_history: Vec<_> = history.iter()
                    .rev()
                    .take(20) // Last 20 checks
                    .collect();
                
                if !recent_history.is_empty() {
                    let successful = recent_history.iter()
                        .filter(|check| matches!(check.health, ProviderHealth::Healthy | ProviderHealth::Degraded))
                        .count();
                    status.success_rate = successful as f64 / recent_history.len() as f64;
                }
                
                // Calculate uptime percentage (last 24 hours equivalent)
                let checks_in_period: Vec<_> = history.iter()
                    .filter(|check| now.duration_since(check.timestamp) < Duration::from_hours(24))
                    .collect();
                    
                if !checks_in_period.is_empty() {
                    let healthy_checks = checks_in_period.iter()
                        .filter(|check| !matches!(check.health, ProviderHealth::Unavailable))
                        .count();
                    status.uptime_percentage = (healthy_checks as f64 / checks_in_period.len() as f64) * 100.0;
                }
            }
            
            // Auto-disable providers with too many consecutive failures
            if status.consecutive_failures >= 5 {
                status.is_temporarily_disabled = true;
                warn!("🚫 Provider {} temporarily disabled due to {} consecutive failures", 
                    provider_id, status.consecutive_failures);
            } else if status.consecutive_failures == 0 && status.is_temporarily_disabled {
                status.is_temporarily_disabled = false;
                info!("✅ Provider {} re-enabled after successful health check", provider_id);
            }
        }
    }
    
    /// Add health check result to history
    fn add_to_history(&mut self, provider_id: &str, result: HealthCheckResult) {
        if let Some(history) = self.health_history.get_mut(provider_id) {
            history.push(result);
            
            // Keep only recent history
            if history.len() > self.max_history_size {
                history.remove(0);
            }
        }
    }
    
    /// Get list of healthy providers
    pub fn get_healthy_providers(&self) -> Vec<String> {
        self.health_status.iter()
            .filter_map(|(id, status)| {
                if matches!(status.current_health, ProviderHealth::Healthy | ProviderHealth::Degraded) 
                   && !status.is_temporarily_disabled {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get list of unhealthy providers
    pub fn get_unhealthy_providers(&self) -> Vec<String> {
        self.health_status.iter()
            .filter_map(|(id, status)| {
                if matches!(status.current_health, ProviderHealth::Unavailable) 
                   || status.is_temporarily_disabled {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Get health status for specific provider
    pub fn get_provider_health(&self, provider_id: &str) -> Option<&ProviderHealthStatus> {
        self.health_status.get(provider_id)
    }
    
    /// Get comprehensive health report
    pub fn get_health_report(&self) -> String {
        let mut report = String::new();
        
        let healthy = self.get_healthy_providers();
        let unhealthy = self.get_unhealthy_providers();
        let total = self.providers.len();
        
        report.push_str(&format!("Health Summary: {}/{} providers healthy\n", healthy.len(), total));
        
        for provider in &self.providers {
            let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
            
            if let Some(status) = self.health_status.get(&provider_id) {
                let health_emoji = match status.current_health {
                    ProviderHealth::Healthy => "✅",
                    ProviderHealth::Degraded => "⚠️",
                    ProviderHealth::Unavailable => "❌",
                };
                
                let disabled_indicator = if status.is_temporarily_disabled { " (DISABLED)" } else { "" };
                
                report.push_str(&format!(
                    "  {} {}: {:?} | Success: {:.1}% | Uptime: {:.1}% | Avg: {:.0}ms | Failures: {}{}\n",
                    health_emoji,
                    provider.name(),
                    status.current_health,
                    status.success_rate * 100.0,
                    status.uptime_percentage,
                    status.avg_response_time,
                    status.consecutive_failures,
                    disabled_indicator
                ));
            }
        }
        
        if let Some(last_check) = self.last_check {
            let time_since_check = Instant::now().duration_since(last_check);
            report.push_str(&format!("\nLast health check: {:?} ago\n", time_since_check));
        }
        
        report
    }
    
    /// Get detailed health history for a provider
    pub fn get_provider_history(&self, provider_id: &str, limit: Option<usize>) -> Vec<&HealthCheckResult> {
        if let Some(history) = self.health_history.get(provider_id) {
            let take_count = limit.unwrap_or(history.len());
            history.iter()
                .rev()
                .take(take_count)
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Check if provider should be considered for requests
    pub fn is_provider_available(&self, provider_id: &str) -> bool {
        if let Some(status) = self.health_status.get(provider_id) {
            !status.is_temporarily_disabled && 
            !matches!(status.current_health, ProviderHealth::Unavailable)
        } else {
            false
        }
    }
    
    /// Get overall system health score (0.0 - 1.0)
    pub fn get_system_health_score(&self) -> f64 {
        if self.health_status.is_empty() {
            return 0.0;
        }
        
        let total_score: f64 = self.health_status.values()
            .map(|status| {
                if status.is_temporarily_disabled {
                    0.0
                } else {
                    match status.current_health {
                        ProviderHealth::Healthy => 1.0,
                        ProviderHealth::Degraded => 0.5,
                        ProviderHealth::Unavailable => 0.0,
                    }
                }
            })
            .sum();
            
        total_score / self.health_status.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::{ProviderConfig, ProviderFactory};

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = ProviderConfig::new("openai", "gpt-4o-mini")
            .with_api_key("test-key".to_string());
        let provider = ProviderFactory::create_provider(&config).unwrap();
        
        let providers = vec![provider];
        let monitor = HealthMonitor::new(providers, Duration::from_secs(30));
        
        assert_eq!(monitor.providers.len(), 1);
        assert_eq!(monitor.health_status.len(), 1);
    }
    
    #[test]
    fn test_health_status_updates() {
        let config = ProviderConfig::new("openai", "gpt-4o-mini")
            .with_api_key("test-key".to_string());
        let provider = ProviderFactory::create_provider(&config).unwrap();
        let provider_id = format!("{}:{}", provider.id().provider_type, provider.id().model);
        
        let mut monitor = HealthMonitor::new(vec![provider], Duration::from_secs(30));
        
        // Initial state should be healthy
        let status = monitor.get_provider_health(&provider_id).unwrap();
        assert_eq!(status.current_health, ProviderHealth::Healthy);
        assert_eq!(status.consecutive_failures, 0);
        
        // Simulate failure
        monitor.update_provider_health(&provider_id, &ProviderHealth::Unavailable, 
            Duration::from_millis(0), Some("Test error".to_string()));
        
        let status = monitor.get_provider_health(&provider_id).unwrap();
        assert_eq!(status.current_health, ProviderHealth::Unavailable);
        assert_eq!(status.consecutive_failures, 1);
        
        // Simulate recovery
        monitor.update_provider_health(&provider_id, &ProviderHealth::Healthy, 
            Duration::from_millis(100), None);
            
        let status = monitor.get_provider_health(&provider_id).unwrap();
        assert_eq!(status.current_health, ProviderHealth::Healthy);
        assert_eq!(status.consecutive_failures, 0);
    }
    
    #[test]
    fn test_system_health_score() {
        let config1 = ProviderConfig::new("openai", "gpt-4o-mini")
            .with_api_key("test-key".to_string());
        let config2 = ProviderConfig::new("anthropic", "claude-3-haiku-20240307")
            .with_api_key("test-key".to_string());
            
        let provider1 = ProviderFactory::create_provider(&config1).unwrap();
        let provider2 = ProviderFactory::create_provider(&config2).unwrap();
        
        let monitor = HealthMonitor::new(vec![provider1, provider2], Duration::from_secs(30));
        
        // All healthy - should be 1.0
        let score = monitor.get_system_health_score();
        assert_eq!(score, 1.0);
    }
}