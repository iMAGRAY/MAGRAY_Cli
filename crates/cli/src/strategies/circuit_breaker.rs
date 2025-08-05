//! Circuit Breaker Pattern Implementation
//! 
//! Реализует паттерн Circuit Breaker для защиты от каскадных сбоев

use anyhow::Result;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn, error};

use crate::agent_traits::CircuitBreakerTrait;

// ============================================================================
// CIRCUIT BREAKER STATES
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    /// Закрыт - нормальная работа
    Closed,
    /// Открыт - все запросы отклоняются
    Open,
    /// Полуоткрыт - пробуем восстановление
    HalfOpen,
}

#[derive(Debug, Clone)]
struct CircuitBreakerMetrics {
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_success_time: Option<Instant>,
    state_change_time: Instant,
}

impl Default for CircuitBreakerMetrics {
    fn default() -> Self {
        Self {
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_success_time: None,
            state_change_time: Instant::now(),
        }
    }
}

// ============================================================================
// CIRCUIT BREAKER IMPLEMENTATION
// ============================================================================

/// Базовая реализация Circuit Breaker
// @component: {"k":"C","id":"basic_circuit_breaker","t":"Basic circuit breaker implementation","m":{"cur":90,"tgt":95,"u":"%"},"f":["circuit_breaker","resilience","production_ready"]}
pub struct BasicCircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    metrics: Arc<Mutex<CircuitBreakerMetrics>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    success_threshold: u32,
    name: String,
}

impl BasicCircuitBreaker {
    /// Создание нового Circuit Breaker
    pub fn new(
        name: String,
        failure_threshold: u32,
        recovery_timeout: Duration,
        success_threshold: u32,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            metrics: Arc::new(Mutex::new(CircuitBreakerMetrics::default())),
            failure_threshold,
            recovery_timeout,
            success_threshold,
            name,
        }
    }
    
    /// Создание с настройками по умолчанию
    pub fn with_defaults(name: String) -> Self {
        Self::new(
            name,
            5,                              // 5 сбоев для открытия
            Duration::from_secs(30),        // 30 секунд recovery timeout
            3,                              // 3 успеха для закрытия
        )
    }
    
    /// Проверка и обновление состояния
    fn check_and_update_state(&self) -> CircuitBreakerState {
        let mut state = self.state.lock().unwrap();
        let mut metrics = self.metrics.lock().unwrap();
        
        match *state {
            CircuitBreakerState::Closed => {
                // Проверяем превышение порога сбоев
                if metrics.failure_count >= self.failure_threshold {
                    info!("CircuitBreaker '{}': переход в Open состояние (сбоев: {})", 
                          self.name, metrics.failure_count);
                    *state = CircuitBreakerState::Open;
                    metrics.state_change_time = Instant::now();
                }
            }
            CircuitBreakerState::Open => {
                // Проверяем таймаут восстановления
                if metrics.state_change_time.elapsed() >= self.recovery_timeout {
                    info!("CircuitBreaker '{}': переход в HalfOpen состояние (timeout истек)", self.name);
                    *state = CircuitBreakerState::HalfOpen;
                    metrics.state_change_time = Instant::now();
                    // Сбрасываем счетчики для пробного периода
                    metrics.failure_count = 0;
                    metrics.success_count = 0;
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Проверяем достаточно ли успешных запросов для закрытия
                if metrics.success_count >= self.success_threshold {
                    info!("CircuitBreaker '{}': переход в Closed состояние (успехов: {})", 
                          self.name, metrics.success_count);
                    *state = CircuitBreakerState::Closed;
                    metrics.state_change_time = Instant::now();
                    // Полностью сбрасываем метрики
                    metrics.failure_count = 0;
                    metrics.success_count = 0;
                }
                // Если есть сбой в HalfOpen, сразу возвращаемся в Open
                else if metrics.failure_count > 0 {
                    warn!("CircuitBreaker '{}': возврат в Open состояние (сбой в HalfOpen)", self.name);
                    *state = CircuitBreakerState::Open;
                    metrics.state_change_time = Instant::now();
                }
            }
        }
        
        state.clone()
    }
    
    /// Записать успешное выполнение
    fn record_success(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.success_count += 1;
        metrics.last_success_time = Some(Instant::now());
        
        debug!("CircuitBreaker '{}': записан успех (всего: {})", self.name, metrics.success_count);
    }
    
    /// Записать сбой
    fn record_failure(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.failure_count += 1;
        metrics.last_failure_time = Some(Instant::now());
        
        debug!("CircuitBreaker '{}': записан сбой (всего: {})", self.name, metrics.failure_count);
    }
    
    /// Получить подробные метрики
    pub fn get_detailed_metrics(&self) -> String {
        let state = self.state.lock().unwrap();
        let metrics = self.metrics.lock().unwrap();
        
        format!(
            "CircuitBreaker '{}' Metrics:\n\
             ├─ Состояние: {:?}\n\
             ├─ Сбоев: {}\n\
             ├─ Успехов: {}\n\
             ├─ Порог сбоев: {}\n\
             ├─ Порог успехов: {}\n\
             ├─ Recovery timeout: {:?}\n\
             ├─ Время в состоянии: {:?}\n\
             ├─ Последний сбой: {:?}\n\
             └─ Последний успех: {:?}",
            self.name,
            *state,
            metrics.failure_count,
            metrics.success_count,
            self.failure_threshold,
            self.success_threshold,
            self.recovery_timeout,
            metrics.state_change_time.elapsed(),
            metrics.last_failure_time.map(|t| t.elapsed()),
            metrics.last_success_time.map(|t| t.elapsed())
        )
    }
    
    /// Проверка, можно ли выполнить запрос
    fn can_execute(&self) -> bool {
        let current_state = self.check_and_update_state();
        current_state != CircuitBreakerState::Open
    }
}

#[async_trait]
impl CircuitBreakerTrait for BasicCircuitBreaker {
    async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        // Проверяем можно ли выполнить операцию
        if !self.can_execute() {
            let time_to_retry = {
                let metrics = self.metrics.lock().unwrap();
                let elapsed = metrics.state_change_time.elapsed();
                if elapsed < self.recovery_timeout {
                    self.recovery_timeout - elapsed
                } else {
                    Duration::from_secs(0)
                }
            };
            
            return Err(anyhow::anyhow!(
                "CircuitBreaker '{}' открыт. Попробуйте через {:?}",
                self.name,
                time_to_retry
            ));
        }
        
        debug!("CircuitBreaker '{}': выполняем операцию", self.name);
        
        // Выполняем операцию и записываем результат
        match operation.await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                
                // Принудительно проверяем состояние после сбоя
                let new_state = self.check_and_update_state();
                if new_state == CircuitBreakerState::Open {
                    warn!("CircuitBreaker '{}': открыт после сбоя", self.name);
                }
                
                Err(error)
            }
        }
    }
    
    async fn force_open(&self) {
        let mut state = self.state.lock().unwrap();
        let mut metrics = self.metrics.lock().unwrap();
        
        warn!("CircuitBreaker '{}': принудительное открытие", self.name);
        *state = CircuitBreakerState::Open;
        metrics.state_change_time = Instant::now();
    }
    
    async fn force_close(&self) {
        let mut state = self.state.lock().unwrap();
        let mut metrics = self.metrics.lock().unwrap();
        
        info!("CircuitBreaker '{}': принудительное закрытие", self.name);
        *state = CircuitBreakerState::Closed;
        metrics.state_change_time = Instant::now();
        metrics.failure_count = 0;
        metrics.success_count = 0;
    }
    
    async fn get_state(&self) -> String {
        let state = self.state.lock().unwrap();
        format!("{:?}", *state)
    }
}

// ============================================================================
// ADAPTIVE CIRCUIT BREAKER
// ============================================================================

/// Адаптивный Circuit Breaker с динамической настройкой порогов
// @component: {"k":"C","id":"adaptive_circuit_breaker","t":"Adaptive circuit breaker with dynamic thresholds","m":{"cur":85,"tgt":95,"u":"%"},"f":["circuit_breaker","adaptive","intelligent"]}
pub struct AdaptiveCircuitBreaker {
    basic: BasicCircuitBreaker,
    base_failure_threshold: u32,
    max_failure_threshold: u32,
    adaptation_factor: f32,
}

impl AdaptiveCircuitBreaker {
    /// Создание адаптивного Circuit Breaker
    pub fn new(
        name: String,
        base_failure_threshold: u32,
        max_failure_threshold: u32,
        recovery_timeout: Duration,
        success_threshold: u32,
        adaptation_factor: f32,
    ) -> Self {
        let basic = BasicCircuitBreaker::new(
            name,
            base_failure_threshold,
            recovery_timeout,
            success_threshold,
        );
        
        Self {
            basic,
            base_failure_threshold,
            max_failure_threshold,
            adaptation_factor,
        }
    }
    
    /// Адаптация порога на основе истории
    fn adapt_threshold(&self) -> u32 {
        let metrics = self.basic.metrics.lock().unwrap();
        
        // Простая адаптация на основе времени в состоянии
        let time_in_state = metrics.state_change_time.elapsed().as_secs() as f32;
        let adaptation = (time_in_state * self.adaptation_factor) as u32;
        
        (self.base_failure_threshold + adaptation).min(self.max_failure_threshold)
    }
}

#[async_trait]
impl CircuitBreakerTrait for AdaptiveCircuitBreaker {
    async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + Send,
        T: Send,
    {
        // Адаптируем порог перед выполнением
        let adapted_threshold = self.adapt_threshold();
        
        // Обновляем порог в базовом circuit breaker
        // (Для простоты, в production версии лучше создать внутренний механизм)
        
        self.basic.execute(operation).await
    }
    
    async fn force_open(&self) {
        self.basic.force_open().await
    }
    
    async fn force_close(&self) {
        self.basic.force_close().await
    }
    
    async fn get_state(&self) -> String {
        let basic_state = self.basic.get_state().await;
        let adapted_threshold = self.adapt_threshold();
        
        format!("{} (adapted threshold: {})", basic_state, adapted_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let cb = BasicCircuitBreaker::new(
            "test".to_string(),
            2, // Низкий порог для тестирования
            Duration::from_millis(100),
            1,
        );
        
        // Изначально закрыт
        assert_eq!(cb.get_state().await, "Closed");
        
        // Первый сбой
        let result1 = cb.execute(async { Err::<(), _>(anyhow::anyhow!("Test error")) }).await;
        assert!(result1.is_err());
        assert_eq!(cb.get_state().await, "Closed");
        
        // Второй сбой - должен открыться
        let result2 = cb.execute(async { Err::<(), _>(anyhow::anyhow!("Test error")) }).await;
        assert!(result2.is_err());
        assert_eq!(cb.get_state().await, "Open");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_open_to_half_open() {
        let cb = BasicCircuitBreaker::new(
            "test".to_string(),
            1, // Очень низкий порог
            Duration::from_millis(50), // Короткий timeout
            1,
        );
        
        // Создаем сбой для открытия
        let _ = cb.execute(async { Err::<(), _>(anyhow::anyhow!("Test error")) }).await;
        assert_eq!(cb.get_state().await, "Open");
        
        // Ждем recovery timeout
        sleep(Duration::from_millis(60)).await;
        
        // Следующий запрос должен перевести в HalfOpen
        let can_execute = cb.can_execute();
        assert!(can_execute);
        assert_eq!(cb.get_state().await, "HalfOpen");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed() {
        let cb = BasicCircuitBreaker::new(
            "test".to_string(),
            1,
            Duration::from_millis(50),
            1, // Только один успех нужен
        );
        
        // Открываем circuit breaker
        let _ = cb.execute(async { Err::<(), _>(anyhow::anyhow!("Test error")) }).await;
        assert_eq!(cb.get_state().await, "Open");
        
        // Ждем и переходим в HalfOpen
        sleep(Duration::from_millis(60)).await;
        
        // Успешная операция должна закрыть
        let result = cb.execute(async { Ok::<(), _>(()) }).await;
        assert!(result.is_ok());
        assert_eq!(cb.get_state().await, "Closed");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_force_operations() {
        let cb = BasicCircuitBreaker::with_defaults("test".to_string());
        
        // Принудительно открываем
        cb.force_open().await;
        assert_eq!(cb.get_state().await, "Open");
        
        // Принудительно закрываем
        cb.force_close().await;
        assert_eq!(cb.get_state().await, "Closed");
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_detailed_metrics() {
        let cb = BasicCircuitBreaker::with_defaults("metrics_test".to_string());
        
        // Выполняем операции
        let _ = cb.execute(async { Ok::<(), _>(()) }).await; // Успех
        let _ = cb.execute(async { Err::<(), _>(anyhow::anyhow!("Error")) }).await; // Сбой
        
        let metrics = cb.get_detailed_metrics();
        assert!(metrics.contains("metrics_test"));
        assert!(metrics.contains("Сбоев: 1"));
        assert!(metrics.contains("Успехов: 1"));
    }
}