use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, error};

/// Политика повторных попыток
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Максимальное количество попыток
    pub max_attempts: u32,
    /// Начальная задержка между попытками
    pub initial_delay: Duration,
    /// Максимальная задержка
    pub max_delay: Duration,
    /// Множитель для экспоненциального backoff
    pub backoff_multiplier: f32,
    /// Добавить случайный jitter
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Быстрая политика для операций с низкой латентностью
    pub fn fast() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 1.5,
            jitter: true,
        }
    }
    
    /// Агрессивная политика для критически важных операций
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.5,
            jitter: true,
        }
    }
    
    /// Без повторов
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::from_millis(0),
            max_delay: Duration::from_millis(0),
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }
}

/// Результат retry операции
#[derive(Debug)]
pub enum RetryResult<T> {
    /// Успех с количеством попыток
    Success(T, u32),
    /// Все попытки исчерпаны
    ExhaustedRetries(anyhow::Error),
    /// Операция не retriable
    NonRetriable(anyhow::Error),
}

impl<T> RetryResult<T> {
    pub fn into_result(self) -> Result<T> {
        match self {
            Self::Success(value, _) => Ok(value),
            Self::ExhaustedRetries(e) => Err(e),
            Self::NonRetriable(e) => Err(e),
        }
    }
    
    pub fn attempts(&self) -> u32 {
        match self {
            Self::Success(_, attempts) => *attempts,
            _ => 0,
        }
    }
}

/// Обработчик повторных попыток
pub struct RetryHandler {
    policy: RetryPolicy,
}

impl RetryHandler {
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }
    
    /// Выполнить операцию с повторными попытками
    pub async fn execute<F, Fut, T>(&self, operation: F) -> RetryResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay = self.policy.initial_delay;
        
        loop {
            attempt += 1;
            debug!("Попытка {}/{}", attempt, self.policy.max_attempts);
            
            match operation().await {
                Ok(result) => {
                    if attempt > 1 {
                        debug!("Операция успешна после {} попыток", attempt);
                    }
                    return RetryResult::Success(result, attempt);
                }
                Err(e) => {
                    // Проверяем является ли ошибка retriable
                    if !self.is_retriable(&e) {
                        warn!("Ошибка не подлежит повтору: {}", e);
                        return RetryResult::NonRetriable(e);
                    }
                    
                    if attempt >= self.policy.max_attempts {
                        error!("Все {} попыток исчерпаны: {}", self.policy.max_attempts, e);
                        return RetryResult::ExhaustedRetries(
                            anyhow::anyhow!("Exhausted {} retries: {}", self.policy.max_attempts, e)
                        );
                    }
                    
                    warn!("Попытка {} не удалась: {}, повтор через {:?}", attempt, e, delay);
                    
                    // Ждем перед следующей попыткой
                    sleep(self.calculate_delay(delay)).await;
                    
                    // Увеличиваем задержку
                    delay = self.next_delay(delay);
                }
            }
        }
    }
    
    /// Выполнить операцию с fallback
    pub async fn execute_with_fallback<F, Fut, T, Fallback, FallbackFut>(
        &self,
        operation: F,
        fallback: Fallback,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        Fallback: FnOnce() -> FallbackFut,
        FallbackFut: std::future::Future<Output = Result<T>>,
    {
        match self.execute(operation).await {
            RetryResult::Success(result, _) => Ok(result),
            RetryResult::ExhaustedRetries(e) | RetryResult::NonRetriable(e) => {
                warn!("Основная операция не удалась: {}, используем fallback", e);
                fallback().await
            }
        }
    }
    
    /// Проверить является ли ошибка retriable
    fn is_retriable(&self, error: &anyhow::Error) -> bool {
        // Проверяем сообщение об ошибке на известные retriable паттерны
        let error_msg = error.to_string().to_lowercase();
        
        // Network и timeout ошибки - retriable
        if error_msg.contains("timeout") || 
           error_msg.contains("connection") ||
           error_msg.contains("network") ||
           error_msg.contains("temporary") {
            return true;
        }
        
        // Database lock - retriable
        if error_msg.contains("database locked") ||
           error_msg.contains("database is locked") {
            return true;
        }
        
        // Resource exhaustion - retriable после ожидания
        if error_msg.contains("out of memory") ||
           error_msg.contains("too many open files") {
            return true;
        }
        
        // Все остальные ошибки не retriable по умолчанию
        false
    }
    
    /// Вычислить задержку с jitter
    fn calculate_delay(&self, base_delay: Duration) -> Duration {
        if !self.policy.jitter {
            return base_delay;
        }
        
        // Добавляем случайный jitter ±25%
        let jitter_range = base_delay.as_millis() as f64 * 0.25;
        let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
        let final_delay = (base_delay.as_millis() as f64 + jitter).max(0.0) as u64;
        
        Duration::from_millis(final_delay)
    }
    
    /// Вычислить следующую задержку
    fn next_delay(&self, current: Duration) -> Duration {
        let next = current.as_secs_f32() * self.policy.backoff_multiplier;
        let next_duration = Duration::from_secs_f32(next);
        
        if next_duration > self.policy.max_delay {
            self.policy.max_delay
        } else {
            next_duration
        }
    }
}

/// Builder для создания RetryHandler с кастомной конфигурацией
pub struct RetryHandlerBuilder {
    policy: RetryPolicy,
}

impl RetryHandlerBuilder {
    pub fn new() -> Self {
        Self {
            policy: RetryPolicy::default(),
        }
    }
    
    pub fn max_attempts(mut self, attempts: u32) -> Self {
        self.policy.max_attempts = attempts;
        self
    }
    
    pub fn initial_delay(mut self, delay: Duration) -> Self {
        self.policy.initial_delay = delay;
        self
    }
    
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.policy.max_delay = delay;
        self
    }
    
    pub fn backoff_multiplier(mut self, multiplier: f32) -> Self {
        self.policy.backoff_multiplier = multiplier;
        self
    }
    
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.policy.jitter = jitter;
        self
    }
    
    pub fn build(self) -> RetryHandler {
        RetryHandler::new(self.policy)
    }
}

impl Default for RetryHandlerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let handler = RetryHandler::new(RetryPolicy::default());
        
        let result = handler.execute(|| async {
            Ok::<_, anyhow::Error>(42)
        }).await;
        
        match result {
            RetryResult::Success(value, attempts) => {
                assert_eq!(value, 42);
                assert_eq!(attempts, 1);
            }
            _ => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let handler = RetryHandler::new(RetryPolicy::fast());
        let counter = std::sync::atomic::AtomicU32::new(0);
        
        let result = handler.execute(|| async {
            let count = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if count < 1 {
                Err(anyhow::anyhow!("Temporary failure"))
            } else {
                Ok(42)
            }
        }).await;
        
        match result {
            RetryResult::Success(value, attempts) => {
                assert_eq!(value, 42);
                assert_eq!(attempts, 2);
            }
            _ => panic!("Expected success"),
        }
    }
    
    #[tokio::test]
    async fn test_retry_exhausted() {
        let handler = RetryHandler::new(RetryPolicy {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        });
        
        let result = handler.execute(|| async {
            Err::<i32, _>(anyhow::anyhow!("Network timeout"))
        }).await;
        
        assert!(matches!(result, RetryResult::ExhaustedRetries(_)));
    }
    
    #[tokio::test]
    async fn test_non_retriable_error() {
        let handler = RetryHandler::new(RetryPolicy::default());
        
        let result = handler.execute(|| async {
            Err::<i32, _>(anyhow::anyhow!("Invalid argument"))
        }).await;
        
        assert!(matches!(result, RetryResult::NonRetriable(_)));
    }
}