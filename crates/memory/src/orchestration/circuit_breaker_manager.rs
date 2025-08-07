use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Circuit breaker manager для координаторов memory системы
///
/// Применяет принципы SOLID:
/// - SRP: Только управление circuit breaker логикой
/// - OCP: Расширяемость через конфигурацию
/// - LSP: Взаимозаменяемость через trait
/// - ISP: Минимальный интерфейс для circuit breaker операций
/// - DIP: Зависит от абстракций, не от конкретных типов
pub struct CircuitBreakerManager {
    /// Circuit breaker состояния для всех координаторов
    breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    /// Конфигурация по умолчанию для новых breakers
    default_config: CircuitBreakerConfig,
}

/// Конфигурация circuit breaker
#[derive(Clone, Debug)]
pub struct CircuitBreakerConfig {
    /// Количество ошибок для открытия circuit'а
    pub failure_threshold: u64,
    /// Timeout для recovery (в секундах)
    pub recovery_timeout: Duration,
}

/// Circuit breaker state для компонента
#[derive(Debug)]
struct CircuitBreakerState {
    /// Количество последовательных ошибок
    failure_count: AtomicU64,
    /// Время последней ошибки
    last_failure: Option<Instant>,
    /// Состояние circuit breaker
    state: CircuitBreakerStatus,
    /// Время recovery timeout
    recovery_timeout: Duration,
    /// Threshold для открытия circuit'а
    failure_threshold: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerStatus {
    /// Нормальная работа - все запросы проходят
    Closed,
    /// Блокировка запросов - circuit открыт
    Open,
    /// Пробная проверка восстановления - один запрос проходит
    HalfOpen,
}

/// Trait для circuit breaker управления (ISP принцип)
#[async_trait::async_trait]
pub trait CircuitBreakerManagerTrait: Send + Sync {
    /// Проверить можно ли выполнить операцию для компонента
    async fn can_execute(&self, component: &str) -> bool;

    /// Записать успешную операцию
    async fn record_success(&self, component: &str);

    /// Записать неуспешную операцию
    async fn record_failure(&self, component: &str);

    /// Получить текущий статус circuit breaker'а
    async fn get_status(&self, component: &str) -> Option<CircuitBreakerStatus>;

    /// Сбросить все circuit breaker'ы в closed состояние
    async fn reset_all(&self) -> Result<()>;

    /// Получить статистику по всем circuit breaker'ам
    async fn get_statistics(&self) -> HashMap<String, CircuitBreakerStatistics>;
}

/// Статистика circuit breaker'а
#[derive(Debug, Clone)]
pub struct CircuitBreakerStatistics {
    pub status: CircuitBreakerStatus,
    pub failure_count: u64,
    pub last_failure_seconds_ago: Option<u64>,
    pub recovery_timeout_seconds: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreakerConfig {
    /// Конфигурация для быстрых операций (search)
    pub fn fast() -> Self {
        Self {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(10),
        }
    }

    /// Конфигурация для критических операций (backup, promotion)
    pub fn critical() -> Self {
        Self {
            failure_threshold: 2,
            recovery_timeout: Duration::from_secs(300), // 5 минут
        }
    }

    /// Конфигурация для обычных операций
    pub fn standard() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

impl CircuitBreakerState {
    fn new(config: &CircuitBreakerConfig) -> Self {
        Self {
            failure_count: AtomicU64::new(0),
            last_failure: None,
            state: CircuitBreakerStatus::Closed,
            recovery_timeout: config.recovery_timeout,
            failure_threshold: config.failure_threshold,
        }
    }

    /// Записать успешную операцию
    fn record_success(&mut self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state = CircuitBreakerStatus::Closed;
        self.last_failure = None;
        debug!("Circuit breaker перешел в Closed состояние");
    }

    /// Записать ошибку
    fn record_failure(&mut self) {
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        self.last_failure = Some(Instant::now());

        if failures >= self.failure_threshold {
            self.state = CircuitBreakerStatus::Open;
            warn!("🔴 Circuit breaker ОТКРЫТ после {} ошибок", failures);
        } else {
            debug!(
                "Circuit breaker: {} ошибок из {} допустимых",
                failures, self.failure_threshold
            );
        }
    }

    /// Проверить можно ли выполнить операцию
    fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitBreakerStatus::Closed => true,
            CircuitBreakerStatus::Open => {
                // Проверяем не пора ли попробовать recovery
                if let Some(last_failure) = self.last_failure {
                    if last_failure.elapsed() >= self.recovery_timeout {
                        self.state = CircuitBreakerStatus::HalfOpen;
                        info!("🟡 Circuit breaker перешел в HalfOpen режим для recovery");
                        return true;
                    }
                }
                false
            }
            CircuitBreakerStatus::HalfOpen => true,
        }
    }

    /// Получить статистику
    fn get_statistics(&self) -> CircuitBreakerStatistics {
        CircuitBreakerStatistics {
            status: self.state.clone(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            last_failure_seconds_ago: self.last_failure.map(|t| t.elapsed().as_secs()),
            recovery_timeout_seconds: self.recovery_timeout.as_secs(),
        }
    }
}

impl CircuitBreakerManager {
    /// Создать новый circuit breaker manager с конфигурацией по умолчанию
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }

    /// Создать circuit breaker manager с кастомной конфигурацией
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
            default_config: config,
        }
    }

    /// Инициализировать circuit breakers для стандартных координаторов
    pub async fn initialize_standard_coordinators(&self) -> Result<()> {
        info!("🔧 Инициализация circuit breakers для стандартных координаторов");

        let mut breakers = self.breakers.write().await;

        // Настраиваем circuit breakers с разными конфигурациями для разных типов операций
        breakers.insert(
            "embedding".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::standard()),
        );
        breakers.insert(
            "search".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::fast()),
        );
        breakers.insert(
            "health".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::standard()),
        );
        breakers.insert(
            "promotion".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::critical()),
        );
        breakers.insert(
            "resources".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::standard()),
        );
        breakers.insert(
            "backup".to_string(),
            CircuitBreakerState::new(&CircuitBreakerConfig::critical()),
        );

        info!("✅ Инициализировано {} circuit breakers", breakers.len());
        Ok(())
    }

    /// Добавить новый circuit breaker для компонента
    pub async fn add_component(
        &self,
        component: &str,
        config: Option<CircuitBreakerConfig>,
    ) -> Result<()> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let mut breakers = self.breakers.write().await;

        if breakers.contains_key(component) {
            warn!(
                "Circuit breaker для {} уже существует, пропускаем",
                component
            );
            return Ok(());
        }

        breakers.insert(component.to_string(), CircuitBreakerState::new(&config));
        info!("➕ Добавлен circuit breaker для компонента: {}", component);
        Ok(())
    }

    /// Проверка состояния circuit breaker и возможности выполнения
    async fn ensure_component_exists(&self, component: &str) {
        let breakers = self.breakers.read().await;
        if !breakers.contains_key(component) {
            drop(breakers); // Освобождаем read lock

            // Создаем новый circuit breaker с конфигурацией по умолчанию
            if let Err(e) = self.add_component(component, None).await {
                warn!(
                    "Не удалось создать circuit breaker для {}: {}",
                    component, e
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl CircuitBreakerManagerTrait for CircuitBreakerManager {
    async fn can_execute(&self, component: &str) -> bool {
        self.ensure_component_exists(component).await;

        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(component) {
            breaker.can_execute()
        } else {
            // Если компонент не найден, разрешаем выполнение (fail-open policy)
            warn!(
                "Circuit breaker для {} не найден, разрешаем выполнение",
                component
            );
            true
        }
    }

    async fn record_success(&self, component: &str) {
        self.ensure_component_exists(component).await;

        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(component) {
            breaker.record_success();
            debug!("✅ Записан успех для circuit breaker: {}", component);
        }
    }

    async fn record_failure(&self, component: &str) {
        self.ensure_component_exists(component).await;

        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(component) {
            breaker.record_failure();
            debug!("❌ Записана ошибка для circuit breaker: {}", component);
        }
    }

    async fn get_status(&self, component: &str) -> Option<CircuitBreakerStatus> {
        let breakers = self.breakers.read().await;
        breakers.get(component).map(|breaker| breaker.state.clone())
    }

    async fn reset_all(&self) -> Result<()> {
        info!("🔄 Сброс всех circuit breakers");

        let mut breakers = self.breakers.write().await;
        for (name, breaker) in breakers.iter_mut() {
            breaker.record_success(); // Reset to closed state
            info!("✅ Circuit breaker {} сброшен в Closed состояние", name);
        }

        info!("✅ Все circuit breakers сброшены");
        Ok(())
    }

    async fn get_statistics(&self) -> HashMap<String, CircuitBreakerStatistics> {
        let breakers = self.breakers.read().await;
        let mut statistics = HashMap::new();

        for (name, breaker) in breakers.iter() {
            statistics.insert(name.clone(), breaker.get_statistics());
        }

        statistics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_lifecycle() {
        let manager = CircuitBreakerManager::new();
        manager
            .initialize_standard_coordinators()
            .await
            .expect("Test circuit breaker initialization should succeed");

        let component = "test_component";
        manager
            .add_component(
                component,
                Some(CircuitBreakerConfig {
                    failure_threshold: 2,
                    recovery_timeout: Duration::from_millis(100),
                }),
            )
            .await
            .expect("Adding test component should succeed");

        // Initially closed - should allow execution
        assert!(manager.can_execute(component).await);
        assert_eq!(
            manager.get_status(component).await,
            Some(CircuitBreakerStatus::Closed)
        );

        // Record failures until circuit opens
        manager.record_failure(component).await;
        assert!(manager.can_execute(component).await); // Still closed after 1 failure

        manager.record_failure(component).await;
        assert!(!manager.can_execute(component).await); // Should be open after 2 failures
        assert_eq!(
            manager.get_status(component).await,
            Some(CircuitBreakerStatus::Open)
        );

        // Wait for recovery timeout
        sleep(Duration::from_millis(150)).await;

        // Should allow one attempt (HalfOpen)
        assert!(manager.can_execute(component).await);
        assert_eq!(
            manager.get_status(component).await,
            Some(CircuitBreakerStatus::HalfOpen)
        );

        // Success should close circuit
        manager.record_success(component).await;
        assert_eq!(
            manager.get_status(component).await,
            Some(CircuitBreakerStatus::Closed)
        );
        assert!(manager.can_execute(component).await);
    }

    #[tokio::test]
    async fn test_circuit_breaker_statistics() {
        let manager = CircuitBreakerManager::new();
        let component = "stats_test";

        manager
            .add_component(component, None)
            .await
            .expect("Adding component for stats test should succeed");

        // Record some failures
        manager.record_failure(component).await;
        manager.record_failure(component).await;

        let stats = manager.get_statistics().await;
        let component_stats = stats
            .get(component)
            .expect("Component stats should be available after adding component");

        assert_eq!(component_stats.failure_count, 2);
        assert!(component_stats.last_failure_seconds_ago.is_some());
        assert_eq!(component_stats.status, CircuitBreakerStatus::Closed); // Still closed with threshold 5
    }

    #[tokio::test]
    async fn test_reset_all_circuit_breakers() {
        let manager = CircuitBreakerManager::new();
        manager
            .initialize_standard_coordinators()
            .await
            .expect("Test initialization should succeed");

        // Fail multiple components
        manager.record_failure("search").await;
        manager.record_failure("embedding").await;

        // Reset all
        manager.reset_all().await.expect("Reset all should succeed");

        // All should be in Closed state
        assert_eq!(
            manager.get_status("search").await,
            Some(CircuitBreakerStatus::Closed)
        );
        assert_eq!(
            manager.get_status("embedding").await,
            Some(CircuitBreakerStatus::Closed)
        );
    }

    #[tokio::test]
    async fn test_different_configurations() {
        let manager = CircuitBreakerManager::new();

        manager
            .add_component("fast", Some(CircuitBreakerConfig::fast()))
            .await
            .expect("Adding fast component should succeed");
        manager
            .add_component("critical", Some(CircuitBreakerConfig::critical()))
            .await
            .expect("Adding critical component should succeed");

        // Fast component should open after 3 failures
        for _ in 0..3 {
            manager.record_failure("fast").await;
        }
        assert!(!manager.can_execute("fast").await);

        // Critical component should open after 2 failures
        for _ in 0..2 {
            manager.record_failure("critical").await;
        }
        assert!(!manager.can_execute("critical").await);
    }
}
