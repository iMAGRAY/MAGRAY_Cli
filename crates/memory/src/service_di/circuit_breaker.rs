//! Circuit Breaker Module - Single Responsibility для resilience patterns
//!
//! Этот модуль отвечает ТОЛЬКО за circuit breaker логику и управление отказоустойчивостью.
//! Применяет Single Responsibility и State pattern.

use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Состояние Circuit Breaker
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    /// Закрыт - операции проходят нормально
    Closed,
    /// Открыт - операции блокируются
    Open,
    /// Полуоткрыт - пробные операции разрешены
    HalfOpen,
}

impl Default for CircuitBreakerState {
    fn default() -> Self {
        CircuitBreakerState::Closed
    }
}

/// Конфигурация Circuit Breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Порог неудачных операций для открытия
    pub failure_threshold: u32,
    /// Таймаут для перехода в HalfOpen
    pub recovery_timeout: Duration,
    /// Максимальное время ожидания операции
    pub operation_timeout: Duration,
    /// Количество пробных операций в HalfOpen состоянии
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            operation_timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

impl CircuitBreakerConfig {
    pub fn production() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            operation_timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }

    pub fn minimal() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(10),
            operation_timeout: Duration::from_secs(5),
            half_open_max_calls: 2,
        }
    }

    pub fn tolerant() -> Self {
        Self {
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(120),
            operation_timeout: Duration::from_secs(60),
            half_open_max_calls: 5,
        }
    }
}

/// Внутреннее состояние Circuit Breaker
#[derive(Debug, Clone)]
struct CircuitBreakerInternalState {
    state: CircuitBreakerState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    last_success_time: Option<Instant>,
    half_open_calls: u32,
}

impl Default for CircuitBreakerInternalState {
    fn default() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_success_time: None,
            half_open_calls: 0,
        }
    }
}

/// Статистика Circuit Breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitBreakerState,
    pub total_failures: u32,
    pub total_successes: u32,
    pub state_transitions: u32,
    pub last_failure_time: Option<Instant>,
    pub last_success_time: Option<Instant>,
    pub time_in_current_state: Duration,
}

/// Production-ready Circuit Breaker
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    internal_state: RwLock<CircuitBreakerInternalState>,
    stats: RwLock<CircuitBreakerStats>,
    state_entered_at: RwLock<Instant>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let now = Instant::now();
        Self {
            config,
            internal_state: RwLock::new(CircuitBreakerInternalState::default()),
            stats: RwLock::new(CircuitBreakerStats {
                state: CircuitBreakerState::Closed,
                total_failures: 0,
                total_successes: 0,
                state_transitions: 0,
                last_failure_time: None,
                last_success_time: None,
                time_in_current_state: Duration::from_secs(0),
            }),
            state_entered_at: RwLock::new(now),
        }
    }

    pub fn with_production_config() -> Self {
        Self::new(CircuitBreakerConfig::production())
    }

    pub fn with_minimal_config() -> Self {
        Self::new(CircuitBreakerConfig::minimal())
    }

    /// Alias для with_production_config() для совместимости
    pub fn new_production() -> Self {
        Self::with_production_config()
    }

    /// Alias для with_minimal_config() для совместимости
    pub fn new_minimal() -> Self {
        Self::with_minimal_config()
    }

    /// Проверить состояние и разрешить/заблокировать операцию
    pub async fn check_and_allow_operation(&self) -> Result<()> {
        let mut state = self.internal_state.write().await;

        match state.state {
            CircuitBreakerState::Closed => {
                // Операции разрешены
                debug!("Circuit breaker закрыт - операция разрешена");
                Ok(())
            }
            CircuitBreakerState::Open => {
                // Проверяем не пора ли перейти в HalfOpen
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() >= self.config.recovery_timeout {
                        self.transition_to_half_open(&mut state).await;
                        debug!("Circuit breaker переходит в HalfOpen - пробная операция разрешена");
                        Ok(())
                    } else {
                        error!("🚫 Circuit breaker открыт - операция заблокирована");
                        Err(anyhow::anyhow!(
                            "Circuit breaker открыт - операции временно недоступны"
                        ))
                    }
                } else {
                    // Нет времени последней ошибки - переходим в HalfOpen
                    self.transition_to_half_open(&mut state).await;
                    Ok(())
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Разрешаем ограниченное количество пробных операций
                if state.half_open_calls < self.config.half_open_max_calls {
                    state.half_open_calls += 1;
                    debug!(
                        "Circuit breaker в HalfOpen - пробная операция {} разрешена",
                        state.half_open_calls
                    );
                    Ok(())
                } else {
                    warn!("Circuit breaker в HalfOpen - лимит пробных операций исчерпан");
                    Err(anyhow::anyhow!(
                        "Circuit breaker в половинном состоянии - лимит операций исчерпан"
                    ))
                }
            }
        }
    }

    /// Записать успешную операцию
    pub async fn record_success(&self) {
        let mut state = self.internal_state.write().await;
        let mut stats = self.stats.write().await;

        state.failure_count = 0;
        state.success_count += 1;
        state.last_success_time = Some(Instant::now());

        stats.total_successes += 1;
        stats.last_success_time = state.last_success_time;

        match state.state {
            CircuitBreakerState::HalfOpen => {
                // Достаточно успехов для закрытия?
                if state.success_count >= self.config.half_open_max_calls {
                    self.transition_to_closed(&mut state, &mut stats).await;
                    info!("🔄 Circuit breaker закрыт после успешных пробных операций");
                }
            }
            CircuitBreakerState::Open => {
                // Это не должно происходить, но если произошло - закрываем
                warn!("Неожиданный успех в состоянии Open - закрываем circuit breaker");
                self.transition_to_closed(&mut state, &mut stats).await;
            }
            CircuitBreakerState::Closed => {
                debug!("Circuit breaker остается закрытым после успешной операции");
            }
        }
    }

    /// Записать неудачную операцию
    pub async fn record_failure(&self) {
        let mut state = self.internal_state.write().await;
        let mut stats = self.stats.write().await;

        state.failure_count += 1;
        state.success_count = 0;
        state.last_failure_time = Some(Instant::now());

        stats.total_failures += 1;
        stats.last_failure_time = state.last_failure_time;

        match state.state {
            CircuitBreakerState::Closed => {
                if state.failure_count >= self.config.failure_threshold {
                    self.transition_to_open(&mut state, &mut stats).await;
                    error!(
                        "🚫 Circuit breaker открыт после {} неудач",
                        state.failure_count
                    );
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Любая неудача в HalfOpen ведет к Open
                self.transition_to_open(&mut state, &mut stats).await;
                error!("🚫 Circuit breaker открыт после неудачи в HalfOpen состоянии");
            }
            CircuitBreakerState::Open => {
                debug!("Circuit breaker остается открытым после очередной неудачи");
            }
        }
    }

    /// Получить текущее состояние
    pub async fn get_state(&self) -> CircuitBreakerState {
        let state = self.internal_state.read().await;
        state.state.clone()
    }

    /// Получить статистику
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let mut stats = self.stats.write().await;
        let state_entered_at = self.state_entered_at.read().await;
        stats.time_in_current_state = state_entered_at.elapsed();
        stats.clone()
    }

    /// Принудительно сбросить circuit breaker (для тестов)
    pub async fn reset(&self) {
        let mut state = self.internal_state.write().await;
        let mut stats = self.stats.write().await;

        *state = CircuitBreakerInternalState::default();
        stats.state = CircuitBreakerState::Closed;
        stats.state_transitions += 1;

        *self.state_entered_at.write().await = Instant::now();

        info!("Circuit breaker сброшен в состояние Closed");
    }

    /// Принудительно открыть (для экстренных случаев)
    pub async fn force_open(&self) {
        let mut state = self.internal_state.write().await;
        let mut stats = self.stats.write().await;

        self.transition_to_open(&mut state, &mut stats).await;
        error!("🚨 Circuit breaker принудительно открыт!");
    }

    // === Private helper methods ===

    async fn transition_to_closed(
        &self,
        state: &mut CircuitBreakerInternalState,
        stats: &mut CircuitBreakerStats,
    ) {
        state.state = CircuitBreakerState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.half_open_calls = 0;

        stats.state = CircuitBreakerState::Closed;
        stats.state_transitions += 1;

        *self.state_entered_at.write().await = Instant::now();
    }

    async fn transition_to_open(
        &self,
        state: &mut CircuitBreakerInternalState,
        stats: &mut CircuitBreakerStats,
    ) {
        state.state = CircuitBreakerState::Open;
        state.success_count = 0;
        state.half_open_calls = 0;

        stats.state = CircuitBreakerState::Open;
        stats.state_transitions += 1;

        *self.state_entered_at.write().await = Instant::now();
    }

    async fn transition_to_half_open(&self, state: &mut CircuitBreakerInternalState) {
        state.state = CircuitBreakerState::HalfOpen;
        state.half_open_calls = 0;
        state.success_count = 0;

        let mut stats = self.stats.write().await;
        stats.state = CircuitBreakerState::HalfOpen;
        stats.state_transitions += 1;

        *self.state_entered_at.write().await = Instant::now();
    }

    /// Получить человекочитаемое описание состояния
    pub async fn get_state_description(&self) -> String {
        let state = self.internal_state.read().await;
        let stats = self.get_stats().await;

        match state.state {
            CircuitBreakerState::Closed => {
                format!(
                    "✅ ЗАКРЫТ: {} неудач из {} допустимых",
                    state.failure_count, self.config.failure_threshold
                )
            }
            CircuitBreakerState::Open => {
                let time_until_recovery = self
                    .config
                    .recovery_timeout
                    .saturating_sub(stats.time_in_current_state);
                format!("🚫 ОТКРЫТ: восстановление через {:?}", time_until_recovery)
            }
            CircuitBreakerState::HalfOpen => {
                format!(
                    "🔄 ПОЛУОТКРЫТ: {} из {} пробных операций",
                    state.half_open_calls, self.config.half_open_max_calls
                )
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::with_minimal_config();

        // В закрытом состоянии операции должны проходить
        assert!(cb.check_and_allow_operation().await.is_ok());
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_failure_threshold() {
        let cb = CircuitBreaker::with_minimal_config(); // threshold = 3

        // Записываем неудачи, но не превышаем порог
        cb.record_failure().await;
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);

        // Превышаем порог - должен открыться
        cb.record_failure().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let mut config = CircuitBreakerConfig::minimal();
        config.recovery_timeout = Duration::from_millis(100);
        let cb = CircuitBreaker::new(config);

        // Открываем circuit breaker
        for _ in 0..3 {
            cb.record_failure().await;
        }
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);

        // Операции блокируются
        assert!(cb.check_and_allow_operation().await.is_err());

        // Ждем recovery timeout
        sleep(Duration::from_millis(150)).await;

        // Теперь должен перейти в HalfOpen
        assert!(cb.check_and_allow_operation().await.is_ok());
        assert_eq!(cb.get_state().await, CircuitBreakerState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_success() {
        let cb = CircuitBreaker::with_minimal_config();

        // Открываем circuit breaker
        for _ in 0..3 {
            cb.record_failure().await;
        }

        // Принудительно переводим в HalfOpen для теста
        cb.reset().await;
        let mut state = cb.internal_state.write().await;
        state.state = CircuitBreakerState::HalfOpen;
        drop(state);

        // Записываем достаточно успехов для закрытия
        for _ in 0..2 {
            // half_open_max_calls = 2 для minimal config
            cb.record_success().await;
        }

        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_stats() {
        let cb = CircuitBreaker::with_minimal_config();

        cb.record_success().await;
        cb.record_failure().await;
        cb.record_success().await;

        let stats = cb.get_stats().await;
        assert_eq!(stats.total_successes, 2);
        assert_eq!(stats.total_failures, 1);
        assert!(stats.last_success_time.is_some());
        assert!(stats.last_failure_time.is_some());
    }

    #[tokio::test]
    async fn test_circuit_breaker_force_operations() {
        let cb = CircuitBreaker::with_minimal_config();

        // Принудительно открываем
        cb.force_open().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);

        // Сбрасываем
        cb.reset().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_state_description() {
        let cb = CircuitBreaker::with_minimal_config();

        let desc_closed = cb.get_state_description().await;
        assert!(desc_closed.contains("ЗАКРЫТ"));

        cb.force_open().await;
        let desc_open = cb.get_state_description().await;
        assert!(desc_open.contains("ОТКРЫТ"));
    }
}
