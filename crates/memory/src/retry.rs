use anyhow::{anyhow, Result};
use rand;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RetryConfig {
    /// Максимальное количество попыток
    pub max_attempts: usize,
    /// Базовая задержка между попытками
    pub base_delay: Duration,
    /// Максимальная задержка
    pub max_delay: Duration,
    /// Мультипликатор для exponential backoff
    pub backoff_multiplier: f64,
    /// Включить jitter для предотвращения thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Retry manager с exponential backoff и jitter
#[allow(dead_code)]
pub struct RetryManager {
    #[allow(dead_code)]
    config: RetryConfig,
}

#[allow(dead_code)]
impl RetryManager {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    #[allow(dead_code)] // Convenience constructor
    pub fn with_defaults() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Выполнить операцию с retry logic
    pub async fn retry<T, E, F, Fut>(&self, operation_name: &str, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        let mut last_error = None;

        for attempt in 1..=self.config.max_attempts {
            debug!(
                "Attempting {} (attempt {}/{})",
                operation_name, attempt, self.config.max_attempts
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!(
                            "Operation {} succeeded on attempt {}",
                            operation_name, attempt
                        );
                    }
                    return Ok(result);
                }
                Err(err) => {
                    warn!(
                        "Operation {} failed on attempt {}: {}",
                        operation_name, attempt, err
                    );
                    last_error = Some(format!("{}", err));

                    // Не ждем после последней попытки
                    if attempt < self.config.max_attempts {
                        let delay = self.calculate_delay(attempt);
                        debug!("Waiting {:?} before retry", delay);
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow!(
            "Operation {} failed after {} attempts. Last error: {}",
            operation_name,
            self.config.max_attempts,
            last_error.unwrap_or_else(|| "Unknown error".to_string())
        ))
    }

    /// Вычислить задержку с exponential backoff и jitter
    pub fn calculate_delay(&self, attempt: usize) -> Duration {
        let exponential_delay = self.config.base_delay.as_millis() as f64
            * self.config.backoff_multiplier.powi((attempt - 1) as i32);

        let capped_delay = exponential_delay.min(self.config.max_delay.as_millis() as f64);

        let final_delay = if self.config.jitter {
            // Добавляем jitter ±25% для предотвращения thundering herd
            let jitter_range = capped_delay * 0.25;
            let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
            (capped_delay + jitter).max(0.0)
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay as u64)
    }

    /// Проверить является ли ошибка retriable
    #[allow(dead_code)] // Utility function для внешнего использования
    pub fn is_retriable_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();

        // Database locking errors
        if error_str.contains("lock") || error_str.contains("busy") {
            return true;
        }

        // I/O errors (temporary)
        if error_str.contains("i/o error") || error_str.contains("connection") {
            return true;
        }

        // HNSW not initialized (temporary)
        if error_str.contains("hnsw не инициализирован")
            || error_str.contains("hnsw not initialized")
        {
            return true;
        }

        // Resource temporarily unavailable
        if error_str.contains("resource temporarily unavailable") {
            return true;
        }

        false
    }

    /// Создать retry manager для database операций
    pub fn for_database() -> Self {
        Self::new(RetryConfig {
            max_attempts: 5,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(2),
            backoff_multiplier: 1.5,
            jitter: true,
        })
    }

    /// Создать retry manager для HNSW операций
    pub fn for_hnsw() -> Self {
        Self::new(RetryConfig {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            jitter: false, // HNSW initialization не нуждается в jitter
        })
    }
}
