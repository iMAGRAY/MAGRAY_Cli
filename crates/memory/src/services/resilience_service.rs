//! ResilienceService - отказоустойчивость и восстановление после ошибок
//!
//! Single Responsibility: только resilience логика
//! - circuit breaker implementation
//! - retry logic  
//! - failure tracking и recovery
//! - error handling patterns

use anyhow::Result;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::services::traits::{ProductionMetrics, ResilienceServiceTrait};

/// Circuit breaker состояние
#[derive(Debug, Clone)]
struct CircuitBreakerState {
    is_open: bool,
    failure_count: u32,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        Self {
            is_open: false,
            failure_count: 0,
            last_failure: None,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
        }
    }
}

/// Реализация resilience patterns
/// Отвечает ТОЛЬКО за отказоустойчивость и восстановление
#[allow(dead_code)]
pub struct ResilienceService {
    /// Circuit breaker для критических операций
    circuit_breaker: RwLock<CircuitBreakerState>,
    /// Production метрики для tracking
    production_metrics: RwLock<ProductionMetrics>,
}

impl ResilienceService {
    /// Создать новый ResilienceService
    pub fn new() -> Self {
        info!("🛡️ Создание ResilienceService для отказоустойчивости");

        Self {
            circuit_breaker: RwLock::new(CircuitBreakerState::default()),
            production_metrics: RwLock::new(ProductionMetrics::default()),
        }
    }

    /// Создать с кастомными настройками circuit breaker
    #[allow(dead_code)]
    pub fn new_with_threshold(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        info!(
            "🛡️ Создание ResilienceService с threshold={}, recovery_timeout={:?}",
            failure_threshold, recovery_timeout
        );

        let mut breaker_state = CircuitBreakerState::default();
        breaker_state.failure_threshold = failure_threshold;
        breaker_state.recovery_timeout = recovery_timeout;

        Self {
            circuit_breaker: RwLock::new(breaker_state),
            production_metrics: RwLock::new(ProductionMetrics::default()),
        }
    }

    /// Создать для production с агрессивными настройками
    #[allow(dead_code)]
    pub fn new_production() -> Self {
        Self::new_with_threshold(3, Duration::from_secs(30)) // Более агрессивный circuit breaker для production
    }

    /// Создать для тестов с мягкими настройками
    #[allow(dead_code)]
    pub fn new_for_tests() -> Self {
        Self::new_with_threshold(10, Duration::from_secs(5)) // Мягкие настройки для тестов
    }
}

#[async_trait]
impl ResilienceServiceTrait for ResilienceService {
    /// Проверить circuit breaker
    #[allow(dead_code)]
    async fn check_circuit_breaker(&self) -> Result<()> {
        let mut breaker = self.circuit_breaker.write().await;

        if breaker.is_open {
            if let Some(last_failure) = breaker.last_failure {
                if last_failure.elapsed() > breaker.recovery_timeout {
                    breaker.is_open = false;
                    breaker.failure_count = 0;
                    info!(
                        "🔄 Circuit breaker восстановлен после {:?}",
                        breaker.recovery_timeout
                    );
                    return Ok(());
                }
            }
            return Err(anyhow::anyhow!(
                "🚫 Circuit breaker открыт - операции временно недоступны"
            ));
        }

        Ok(())
    }

    /// Записать успешную операцию
    #[allow(dead_code)]
    async fn record_successful_operation(&self, duration: Duration) {
        // Обновляем circuit breaker
        {
            let mut breaker = self.circuit_breaker.write().await;
            breaker.failure_count = 0;
        }

        // Обновляем production метрики
        {
            let mut metrics = self.production_metrics.write().await;
            metrics.total_operations += 1;
            metrics.successful_operations += 1;

            // Exponential moving average для response time
            let duration_ms = duration.as_millis() as f64;
            let alpha = 0.1;
            if metrics.avg_response_time_ms == 0.0 {
                metrics.avg_response_time_ms = duration_ms;
            } else {
                metrics.avg_response_time_ms =
                    alpha * duration_ms + (1.0 - alpha) * metrics.avg_response_time_ms;
            }
        }

        debug!("✅ ResilienceService: успешная операция за {:?}", duration);
    }

    /// Записать неудачную операцию
    #[allow(dead_code)]
    async fn record_failed_operation(&self, duration: Duration) {
        // Обновляем circuit breaker
        {
            let mut breaker = self.circuit_breaker.write().await;
            breaker.failure_count += 1;
            breaker.last_failure = Some(Instant::now());

            if breaker.failure_count >= breaker.failure_threshold {
                breaker.is_open = true;
                error!(
                    "🚫 Circuit breaker открыт после {} ошибок",
                    breaker.failure_count
                );
            }
        }

        // Обновляем production метрики
        {
            let mut metrics = self.production_metrics.write().await;
            metrics.total_operations += 1;
            metrics.failed_operations += 1;

            // Увеличиваем счетчик circuit breaker trips при открытии
            if self.circuit_breaker.read().await.is_open {
                metrics.circuit_breaker_trips += 1;
            }

            // Обновляем response time даже для неудачных операций
            let duration_ms = duration.as_millis() as f64;
            let alpha = 0.1;
            if metrics.avg_response_time_ms == 0.0 {
                metrics.avg_response_time_ms = duration_ms;
            } else {
                metrics.avg_response_time_ms =
                    alpha * duration_ms + (1.0 - alpha) * metrics.avg_response_time_ms;
            }
        }

        warn!("❌ ResilienceService: неудачная операция за {:?}", duration);
    }

    /// Получить статус circuit breaker
    #[allow(dead_code)]
    async fn get_circuit_breaker_status(&self) -> bool {
        let breaker = self.circuit_breaker.read().await;
        breaker.is_open
    }

    /// Сбросить circuit breaker
    #[allow(dead_code)]
    async fn reset_circuit_breaker(&self) -> Result<()> {
        let mut breaker = self.circuit_breaker.write().await;
        breaker.is_open = false;
        breaker.failure_count = 0;
        breaker.last_failure = None;

        info!("🔄 Circuit breaker принудительно сброшен");
        Ok(())
    }

    /// Установить threshold для circuit breaker
    #[allow(dead_code)]
    async fn set_failure_threshold(&self, threshold: u32) -> Result<()> {
        let mut breaker = self.circuit_breaker.write().await;
        breaker.failure_threshold = threshold;

        info!("⚙️ Circuit breaker threshold установлен: {}", threshold);
        Ok(())
    }

    /// Получить статистику failures
    #[allow(dead_code)]
    async fn get_failure_stats(&self) -> (u32, Duration) {
        let breaker = self.circuit_breaker.read().await;
        let recovery_timeout = breaker.recovery_timeout;
        let failure_count = breaker.failure_count;

        (failure_count, recovery_timeout)
    }
}

impl ResilienceService {
    /// Получить production метрики (дополнительный helper метод)
    #[allow(dead_code)]
    pub async fn get_production_metrics(&self) -> ProductionMetrics {
        let metrics = self.production_metrics.read().await;
        metrics.clone()
    }

    /// Получить подробную статистику resilience
    #[allow(dead_code)]
    pub async fn get_resilience_stats(&self) -> ResilienceStats {
        let breaker = self.circuit_breaker.read().await;
        let metrics = self.production_metrics.read().await;

        ResilienceStats {
            circuit_breaker_open: breaker.is_open,
            failure_count: breaker.failure_count,
            failure_threshold: breaker.failure_threshold,
            recovery_timeout: breaker.recovery_timeout,
            last_failure: breaker.last_failure,
            total_operations: metrics.total_operations,
            successful_operations: metrics.successful_operations,
            failed_operations: metrics.failed_operations,
            circuit_breaker_trips: metrics.circuit_breaker_trips,
            success_rate: if metrics.total_operations > 0 {
                (metrics.successful_operations as f64 / metrics.total_operations as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

impl Default for ResilienceService {
    fn default() -> Self {
        Self::new()
    }
}

/// Подробная статистика resilience
#[derive(Debug)]
pub struct ResilienceStats {
    pub circuit_breaker_open: bool,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub last_failure: Option<Instant>,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub circuit_breaker_trips: u64,
    pub success_rate: f64,
}
