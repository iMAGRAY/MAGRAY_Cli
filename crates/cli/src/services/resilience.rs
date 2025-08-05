//! Resilience Service - обработка ошибок и retry логика
//! 
//! Сервис предоставляет централизованную обработку ошибок,
//! retry логику с exponential backoff, circuit breaker pattern
//! и мониторинг отказоустойчивости системы.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};
use super::types::OperationResult;

/// Trait для сервиса отказоустойчивости
// @component: {"k":"C","id":"resilience_service","t":"Resilience service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","resilience","circuit_breaker","retry","clean_architecture"]}
#[async_trait::async_trait]
pub trait ResilienceService: Send + Sync {
    /// Выполнить операцию с retry логикой
    async fn execute_with_retry<T, F, Fut>(
        &self,
        operation: F,
        config: &RetryConfig,
    ) -> Result<OperationResult<T>>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send + 'static;
    
    /// Получить статистику отказоустойчивости
    async fn get_resilience_stats(&self) -> ResilienceStats;
    
    /// Сбросить статистику
    async fn reset_stats(&self);
}

/// Конфигурация retry логики
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Статистика отказоустойчивости
#[derive(Debug, Clone)]
pub struct ResilienceStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub retried_operations: u64,
    pub avg_retry_count: f64,
}

/// Реализация сервиса отказоустойчивости по умолчанию
// @component: {"k":"C","id":"default_resilience_service","t":"Default resilience service implementation","m":{"cur":85,"tgt":95,"u":"%"},"f":["service","resilience","exponential_backoff","jitter","metrics"]}
pub struct DefaultResilienceService {
    stats: parking_lot::RwLock<ResilienceStats>,
}

impl DefaultResilienceService {
    pub fn new() -> Self {
        Self {
            stats: parking_lot::RwLock::new(ResilienceStats::default()),
        }
    }
    
    fn calculate_delay(&self, attempt: u32, config: &RetryConfig) -> Duration {
        let mut delay = config.base_delay.as_millis() as f64 
            * config.backoff_multiplier.powi(attempt as i32 - 1);
        
        if config.jitter {
            use rand::Rng;
            let jitter_factor = rand::thread_rng().gen_range(0.5..1.5);
            delay *= jitter_factor;
        }
        
        Duration::from_millis(delay.min(config.max_delay.as_millis() as f64) as u64)
    }
    
    fn update_stats(&self, success: bool, retries: u32) {
        let mut stats = self.stats.write();
        stats.total_operations += 1;
        
        if success {
            stats.successful_operations += 1;
        } else {
            stats.failed_operations += 1;
        }
        
        if retries > 0 {
            stats.retried_operations += 1;
            let total_retries = stats.avg_retry_count * (stats.retried_operations - 1) as f64 + retries as f64;
            stats.avg_retry_count = total_retries / stats.retried_operations as f64;
        }
    }
}

#[async_trait::async_trait]
impl ResilienceService for DefaultResilienceService {
    async fn execute_with_retry<T, F, Fut>(
        &self,
        operation: F,
        config: &RetryConfig,
    ) -> Result<OperationResult<T>>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send + 'static,
    {
        use std::time::Instant;
        let start_time = Instant::now();
        
        for attempt in 1..=config.max_attempts {
            match operation().await {
                Ok(result) => {
                    let duration = start_time.elapsed();
                    self.update_stats(true, attempt - 1);
                    return Ok(OperationResult {
                        result: Ok(result),
                        duration,
                        retries: attempt - 1,
                        from_cache: false,
                    });
                }
                Err(e) => {
                    if attempt == config.max_attempts {
                        warn!("🔴 Операция failed после {} попыток: {}", config.max_attempts, e);
                        let duration = start_time.elapsed();
                        self.update_stats(false, attempt - 1);
                        return Ok(OperationResult {
                            result: Err(e),
                            duration,
                            retries: attempt - 1,
                            from_cache: false,
                        });
                    }
                    
                    let delay = self.calculate_delay(attempt, config);
                    debug!("⚠️ Попытка {} failed, retry через {:?}: {}", attempt, delay, e);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        unreachable!()
    }
    
    async fn get_resilience_stats(&self) -> ResilienceStats {
        let stats = self.stats.read();
        stats.clone()
    }
    
    async fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = ResilienceStats::default();
        debug!("🔄 Resilience stats reset");
    }
}

impl Default for ResilienceStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            retried_operations: 0,
            avg_retry_count: 0.0,
        }
    }
}

/// Factory функция для DI контейнера
pub fn create_resilience_service() -> Arc<dyn ResilienceService> {
    Arc::new(DefaultResilienceService::new())
}