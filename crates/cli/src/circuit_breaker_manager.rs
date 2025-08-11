//! CircuitBreakerManager - управление circuit breakers для всех компонентов
//!
//! Реализует Single Responsibility Principle для управления состоянием
//! circuit breakers и их конфигурацией.

use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Состояния circuit breaker
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CircuitBreakerState {
    Closed,   // Нормальная работа
    Open,     // Блокировка запросов
    HalfOpen, // Пробная проверка восстановления
}

/// Конфигурация circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Максимальное количество ошибок перед открытием
    pub failure_threshold: u64,
    /// Время восстановления после открытия
    pub recovery_timeout: Duration,
    /// Минимальное количество запросов для расчета статистики
    pub min_request_threshold: u64,
    /// Процент ошибок для открытия (0.0 - 1.0)
    pub error_rate_threshold: f64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            min_request_threshold: 10,
            error_rate_threshold: 0.5,
        }
    }
}

/// Статистика circuit breaker
#[derive(Debug, Clone, Serialize)]
pub struct CircuitBreakerStats {
    pub name: String,
    pub state: CircuitBreakerState,
    pub failure_count: u64,
    pub success_count: u64,
    pub total_requests: u64,
    pub error_rate: f64,
    #[serde(skip)]
    pub last_failure_time: Option<Instant>,
    pub trip_count: u64,
    pub recovery_attempts: u64,
}

/// Внутреннее состояние circuit breaker
struct CircuitBreakerData {
    config: CircuitBreakerConfig,
    state: CircuitBreakerState,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    last_failure_time: Option<Instant>,
    trip_count: AtomicU64,
    recovery_attempts: AtomicU64,
    last_state_change: Instant,
}

impl CircuitBreakerData {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: CircuitBreakerState::Closed,
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            last_failure_time: None,
            trip_count: AtomicU64::new(0),
            recovery_attempts: AtomicU64::new(0),
            last_state_change: Instant::now(),
        }
    }

    /// Проверить можно ли выполнить операцию
    fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Проверяем не пора ли перейти в HalfOpen
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() >= self.config.recovery_timeout {
                        self.transition_to_half_open();
                        return true;
                    }
                }
                false
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Записать успешную операцию
    fn record_success(&mut self) {
        let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

        match self.state {
            CircuitBreakerState::HalfOpen => {
                // В HalfOpen успех переводит в Closed
                self.transition_to_closed();
                debug!(
                    "Circuit breaker восстановлен после {} успешных операций",
                    success_count
                );
            }
            CircuitBreakerState::Open => {
                // Не должно происходить, но на всякий случай
                warn!("Получен успех в состоянии Open");
            }
            CircuitBreakerState::Closed => {
                // Нормальная ситуация
            }
        }
    }

    /// Записать ошибку
    fn record_failure(&mut self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure_time = Some(Instant::now());

        let total_requests = failure_count + self.success_count.load(Ordering::Relaxed);

        match self.state {
            CircuitBreakerState::Closed => {
                // Проверяем нужно ли открыть circuit
                if self.should_open(failure_count, total_requests) {
                    self.transition_to_open();
                }
            }
            CircuitBreakerState::HalfOpen => {
                // В HalfOpen любая ошибка переводит в Open
                self.transition_to_open();
            }
            CircuitBreakerState::Open => {
                // Уже открыт, просто обновляем статистику
            }
        }
    }

    /// Проверить нужно ли открыть circuit
    fn should_open(&self, failure_count: u64, total_requests: u64) -> bool {
        // Простая проверка по количеству ошибок
        if failure_count >= self.config.failure_threshold {
            return true;
        }

        // Проверка по проценту ошибок (если есть минимальное количество запросов)
        if total_requests >= self.config.min_request_threshold {
            let error_rate = failure_count as f64 / total_requests as f64;
            return error_rate >= self.config.error_rate_threshold;
        }

        false
    }

    /// Переход в состояние Closed
    fn transition_to_closed(&mut self) {
        self.state = CircuitBreakerState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        self.last_failure_time = None;
        self.last_state_change = Instant::now();
    }

    /// Переход в состояние Open
    fn transition_to_open(&mut self) {
        self.state = CircuitBreakerState::Open;
        self.trip_count.fetch_add(1, Ordering::Relaxed);
        self.last_state_change = Instant::now();
    }

    /// Переход в состояние HalfOpen
    fn transition_to_half_open(&mut self) {
        self.state = CircuitBreakerState::HalfOpen;
        self.recovery_attempts.fetch_add(1, Ordering::Relaxed);
        self.last_state_change = Instant::now();
    }

    /// Получить статистику
    fn get_stats(&self, name: &str) -> CircuitBreakerStats {
        let failure_count = self.failure_count.load(Ordering::Relaxed);
        let success_count = self.success_count.load(Ordering::Relaxed);
        let total_requests = failure_count + success_count;

        let error_rate = if total_requests > 0 {
            failure_count as f64 / total_requests as f64
        } else {
            0.0
        };

        CircuitBreakerStats {
            name: name.to_string(),
            state: self.state.clone(),
            failure_count,
            success_count,
            total_requests,
            error_rate,
            last_failure_time: self.last_failure_time,
            trip_count: self.trip_count.load(Ordering::Relaxed),
            recovery_attempts: self.recovery_attempts.load(Ordering::Relaxed),
        }
    }
}

/// Менеджер circuit breakers
pub struct CircuitBreakerManager {
    breakers: Arc<RwLock<HashMap<String, CircuitBreakerData>>>,
}

impl CircuitBreakerManager {
    /// Создать новый менеджер
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Зарегистрировать circuit breaker
    pub async fn register_circuit_breaker(&self, name: String, config: CircuitBreakerConfig) {
        info!(
            "Регистрация circuit breaker: {} с threshold: {}",
            name, config.failure_threshold
        );

        let mut breakers = self.breakers.write().await;
        breakers.insert(name, CircuitBreakerData::new(config));
    }

    /// Проверить можно ли выполнить операцию
    pub async fn can_execute(&self, name: &str) -> Result<bool> {
        let mut breakers = self.breakers.write().await;

        match breakers.get_mut(name) {
            Some(breaker) => Ok(breaker.can_execute()),
            None => {
                warn!("Circuit breaker '{}' не найден, разрешаем выполнение", name);
                Ok(true) // Если breaker не найден, разрешаем выполнение
            }
        }
    }

    /// Записать успешную операцию
    pub async fn record_success(&self, name: &str) -> Result<()> {
        let mut breakers = self.breakers.write().await;

        if let Some(breaker) = breakers.get_mut(name) {
            breaker.record_success();
            debug!("Circuit breaker '{}' записал успех", name);
        } else {
            warn!("Circuit breaker '{}' не найден для записи успеха", name);
        }

        Ok(())
    }

    /// Записать ошибку
    pub async fn record_failure(&self, name: &str) -> Result<()> {
        let mut breakers = self.breakers.write().await;

        if let Some(breaker) = breakers.get_mut(name) {
            let old_state = breaker.state.clone();
            breaker.record_failure();

            // Логируем изменение состояния
            if breaker.state != old_state {
                warn!(
                    "Circuit breaker '{}' изменил состояние: {:?} -> {:?}",
                    name, old_state, breaker.state
                );
            }
        } else {
            warn!("Circuit breaker '{}' не найден для записи ошибки", name);
        }

        Ok(())
    }

    /// Получить статистику circuit breaker
    pub async fn get_stats(&self, name: &str) -> Option<CircuitBreakerStats> {
        let breakers = self.breakers.read().await;
        breakers.get(name).map(|breaker| breaker.get_stats(name))
    }

    /// Получить статистику всех circuit breakers
    pub async fn get_all_stats(&self) -> HashMap<String, CircuitBreakerStats> {
        let breakers = self.breakers.read().await;
        let mut stats = HashMap::new();

        for (name, breaker) in breakers.iter() {
            stats.insert(name.clone(), breaker.get_stats(name));
        }

        stats
    }

    /// Принудительно сбросить circuit breaker
    pub async fn reset_circuit_breaker(&self, name: &str) -> Result<()> {
        let mut breakers = self.breakers.write().await;

        if let Some(breaker) = breakers.get_mut(name) {
            breaker.transition_to_closed();
            info!("Circuit breaker '{}' принудительно сброшен", name);
        } else {
            return Err(anyhow::anyhow!("Circuit breaker '{}' не найден", name));
        }

        Ok(())
    }

    /// Принудительно сбросить все circuit breakers
    pub async fn reset_all_circuit_breakers(&self) -> Result<()> {
        let mut breakers = self.breakers.write().await;

        for (name, breaker) in breakers.iter_mut() {
            breaker.transition_to_closed();
            info!("Circuit breaker '{}' сброшен", name);
        }

        info!("Все circuit breakers сброшены");
        Ok(())
    }

    /// Получить состояния всех circuit breakers
    pub async fn get_states(&self) -> HashMap<String, CircuitBreakerState> {
        let breakers = self.breakers.read().await;
        let mut states = HashMap::new();

        for (name, breaker) in breakers.iter() {
            states.insert(name.clone(), breaker.state.clone());
        }

        states
    }

    /// Выполнить операцию с защитой circuit breaker
    pub async fn execute_with_breaker<F, R>(&self, name: &str, operation: F) -> Result<R>
    where
        F: std::future::Future<Output = Result<R>>,
    {
        // Проверяем можно ли выполнить операцию
        if !self.can_execute(name).await? {
            return Err(anyhow::anyhow!(
                "Circuit breaker '{}' заблокировал операцию",
                name
            ));
        }

        // Выполняем операцию
        let start_time = Instant::now();
        match operation.await {
            Ok(result) => {
                self.record_success(name).await?;
                debug!(
                    "Операция '{}' выполнена успешно за {:?}",
                    name,
                    start_time.elapsed()
                );
                Ok(result)
            }
            Err(e) => {
                self.record_failure(name).await?;
                debug!(
                    "Операция '{}' завершилась ошибкой за {:?}: {}",
                    name,
                    start_time.elapsed(),
                    e
                );
                Err(e)
            }
        }
    }

    /// Количество зарегистрированных circuit breakers
    pub async fn circuit_breaker_count(&self) -> usize {
        let breakers = self.breakers.read().await;
        breakers.len()
    }

    /// Получить список имен circuit breakers
    pub async fn circuit_breaker_names(&self) -> Vec<String> {
        let breakers = self.breakers.read().await;
        breakers.keys().cloned().collect()
    }
}

impl Default for CircuitBreakerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder для создания настроенных circuit breakers
pub struct CircuitBreakerBuilder {
    manager: CircuitBreakerManager,
}

impl CircuitBreakerBuilder {
    pub fn new() -> Self {
        Self {
            manager: CircuitBreakerManager::new(),
        }
    }

    /// Добавить circuit breaker с конфигурацией по умолчанию
    pub async fn with_default_breaker(self, name: String) -> Self {
        self.manager
            .register_circuit_breaker(name, CircuitBreakerConfig::default())
            .await;
        self
    }

    /// Добавить circuit breaker с custom конфигурацией
    pub async fn with_custom_breaker(self, name: String, config: CircuitBreakerConfig) -> Self {
        self.manager.register_circuit_breaker(name, config).await;
        self
    }

    /// Добавить быстрый circuit breaker (для latency-sensitive операций)
    pub async fn with_fast_breaker(self, name: String) -> Self {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(10),
            min_request_threshold: 5,
            error_rate_threshold: 0.3,
        };
        self.manager.register_circuit_breaker(name, config).await;
        self
    }

    /// Добавить медленный circuit breaker (для heavy операций)
    pub async fn with_slow_breaker(self, name: String) -> Self {
        let config = CircuitBreakerConfig {
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(60),
            min_request_threshold: 20,
            error_rate_threshold: 0.7,
        };
        self.manager.register_circuit_breaker(name, config).await;
        self
    }

    pub fn build(self) -> CircuitBreakerManager {
        self.manager
    }
}

impl Default for CircuitBreakerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_basic_functionality() {
        let manager = CircuitBreakerManager::new();

        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(100),
            min_request_threshold: 1,
            error_rate_threshold: 0.5,
        };

        manager
            .register_circuit_breaker("test".to_string(), config)
            .await;

        // Initially closed
        assert!(manager.can_execute("test").await.unwrap());

        // Record failures to open circuit
        manager.record_failure("test").await.unwrap();
        manager.record_failure("test").await.unwrap();

        // Should be open now
        let stats = manager.get_stats("test").await.unwrap();
        assert_eq!(stats.state, CircuitBreakerState::Open);
        assert!(!manager.can_execute("test").await.unwrap());

        sleep(Duration::from_millis(150)).await;

        // Should allow one attempt (HalfOpen)
        assert!(manager.can_execute("test").await.unwrap());

        // Record success to close
        manager.record_success("test").await.unwrap();
        let stats = manager.get_stats("test").await.unwrap();
        assert_eq!(stats.state, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_execute_with_protection() {
        let manager = CircuitBreakerManager::new();
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout: Duration::from_millis(50),
            ..Default::default()
        };

        manager
            .register_circuit_breaker("operation".to_string(), config)
            .await;

        // Successful operation
        let result = manager
            .execute_with_breaker("operation", async { Ok::<i32, anyhow::Error>(42) })
            .await;
        assert_eq!(result.unwrap(), 42);

        // Failed operation that should open circuit
        let _result = manager
            .execute_with_breaker("operation", async {
                Err::<i32, anyhow::Error>(anyhow::anyhow!("test error"))
            })
            .await;

        // Circuit should be open now
        let stats = manager.get_stats("operation").await.unwrap();
        assert_eq!(stats.state, CircuitBreakerState::Open);

        // Next operation should be blocked
        let result = manager
            .execute_with_breaker("operation", async { Ok::<i32, anyhow::Error>(100) })
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_builder() {
        let manager = CircuitBreakerBuilder::new()
            .with_default_breaker("default".to_string())
            .await
            .with_fast_breaker("fast".to_string())
            .await
            .with_slow_breaker("slow".to_string())
            .await
            .build();

        assert_eq!(manager.circuit_breaker_count().await, 3);

        let names = manager.circuit_breaker_names().await;
        assert!(names.contains(&"default".to_string()));
        assert!(names.contains(&"fast".to_string()));
        assert!(names.contains(&"slow".to_string()));
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset_functionality() {
        let manager = CircuitBreakerManager::new();
        manager
            .register_circuit_breaker("reset_test".to_string(), CircuitBreakerConfig::default())
            .await;

        // Open the circuit
        for _ in 0..5 {
            manager.record_failure("reset_test").await.unwrap();
        }

        let stats = manager.get_stats("reset_test").await.unwrap();
        assert_eq!(stats.state, CircuitBreakerState::Open);

        // Reset circuit breaker
        manager.reset_circuit_breaker("reset_test").await.unwrap();

        let stats = manager.get_stats("reset_test").await.unwrap();
        assert_eq!(stats.state, CircuitBreakerState::Closed);
        assert!(manager.can_execute("reset_test").await.unwrap());
    }
}
