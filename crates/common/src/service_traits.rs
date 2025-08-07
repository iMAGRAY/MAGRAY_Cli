/// Базовые trait abstractions для Service patterns в MAGRAY CLI
/// 
/// Этот модуль содержит общие trait определения для устранения дублирования
/// в service implementations across все крейты

use std::time::Duration;
use std::sync::Arc;

/// Базовый trait для всех Service implementations
pub trait BaseService: Send + Sync {
    /// Имя сервиса для логирования и отладки
    fn name(&self) -> &'static str;
    
    /// Версия сервиса
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    
    /// Проверка состояния сервиса
    fn is_healthy(&self) -> bool {
        true
    }
    
    /// Мягкая остановка сервиса
    async fn shutdown(&self) -> Result<(), crate::MagrayCoreError> {
        Ok(())
    }
}

/// Trait для сервисов с конфигурацией
pub trait ConfigurableService<T>: BaseService {
    /// Получить текущую конфигурацию
    fn config(&self) -> &T;
    
    /// Обновить конфигурацию (hot reload)
    async fn update_config(&mut self, config: T) -> Result<(), crate::MagrayCoreError>;
    
    /// Валидация конфигурации
    fn validate_config(&self, config: &T) -> Result<(), crate::ConfigError>;
}

/// Trait для сервисов с метриками
pub trait MetricsService: BaseService {
    type Metrics: Clone + Send + Sync;
    
    /// Получить текущие метрики
    fn metrics(&self) -> Self::Metrics;
    
    /// Сбросить метрики
    fn reset_metrics(&self);
    
    /// Экспорт метрик в формате для мониторинга
    fn export_metrics(&self) -> String 
    where 
        Self::Metrics: std::fmt::Debug 
    {
        format!("service={} metrics={:?}", self.name(), self.metrics())
    }
}

/// Trait для сервисов с circuit breaker защитой  
pub trait CircuitBreakerService: BaseService {
    /// Выполнить операцию с circuit breaker защитой
    async fn execute_with_circuit_breaker<F, R, E>(&self, operation: F) -> Result<R, E>
    where
        F: FnOnce() -> Result<R, E> + Send,
        E: std::error::Error + Send + Sync + 'static;
        
    /// Получить состояние circuit breaker
    fn circuit_breaker_state(&self) -> CircuitBreakerState;
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Trait для сервисов с retry логикой
pub trait RetryableService: BaseService {
    /// Выполнить операцию с retry логикой
    async fn execute_with_retry<F, Fut, R, E>(&self, operation: F) -> Result<R, E>
    where
        F: Fn() -> Fut + Send,
        Fut: std::future::Future<Output = Result<R, E>> + Send,
        E: std::error::Error + Send + Sync + 'static;
        
    /// Получить конфигурацию retry
    fn retry_config(&self) -> RetryConfig;
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

/// Trait для сервисов с lifecycle management
pub trait LifecycleService: BaseService {
    /// Инициализация сервиса
    async fn initialize(&mut self) -> Result<(), crate::MagrayCoreError>;
    
    /// Запуск сервиса
    async fn start(&mut self) -> Result<(), crate::MagrayCoreError>;
    
    /// Пауза сервиса
    async fn pause(&mut self) -> Result<(), crate::MagrayCoreError> {
        Ok(())
    }
    
    /// Возобновление сервиса
    async fn resume(&mut self) -> Result<(), crate::MagrayCoreError> {
        Ok(())
    }
    
    /// Остановка сервиса
    async fn stop(&mut self) -> Result<(), crate::MagrayCoreError>;
    
    /// Текущее состояние lifecycle
    fn lifecycle_state(&self) -> LifecycleState;
}

#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleState {
    NotInitialized,
    Initialized,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Error(String),
}

/// Trait для координаторов сервисов
pub trait ServiceCoordinator: BaseService {
    type Service: BaseService;
    
    /// Зарегистрировать сервис
    async fn register_service(&mut self, service: Arc<Self::Service>) -> Result<(), crate::MagrayCoreError>;
    
    /// Отменить регистрацию сервиса
    async fn unregister_service(&mut self, service_name: &str) -> Result<(), crate::MagrayCoreError>;
    
    /// Получить сервис по имени
    fn get_service(&self, name: &str) -> Option<Arc<Self::Service>>;
    
    /// Получить все зарегистрированные сервисы
    fn list_services(&self) -> Vec<Arc<Self::Service>>;
}

/// Trait для factory pattern сервисов
pub trait ServiceFactory<T>: Send + Sync {
    type Config;
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Создать новый экземпляр сервиса
    fn create(&self, config: Self::Config) -> Result<T, Self::Error>;
    
    /// Создать сервис с default конфигурацией
    fn create_default(&self) -> Result<T, Self::Error>
    where
        Self::Config: Default,
    {
        self.create(Self::Config::default())
    }
}

/// Trait для сервисов с connection pooling
pub trait PooledService: BaseService {
    type Connection: Send + Sync;
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// Получить соединение из пула
    async fn get_connection(&self) -> Result<Self::Connection, Self::Error>;
    
    /// Вернуть соединение в пул
    async fn return_connection(&self, connection: Self::Connection) -> Result<(), Self::Error>;
    
    /// Получить статистики пула
    fn pool_stats(&self) -> PoolStats;
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub active_connections: usize,
    pub idle_connections: usize,
    pub total_connections: usize,
    pub max_connections: usize,
}

/// Trait для кэширующих сервисов
pub trait CacheService<K, V>: BaseService {
    /// Получить значение из кэша
    async fn get(&self, key: &K) -> Option<V>;
    
    /// Сохранить значение в кэш
    async fn set(&self, key: K, value: V) -> Result<(), crate::MagrayCoreError>;
    
    /// Удалить значение из кэша
    async fn remove(&self, key: &K) -> Result<bool, crate::MagrayCoreError>;
    
    /// Очистить весь кэш
    async fn clear(&self) -> Result<(), crate::MagrayCoreError>;
    
    /// Получить статистики кэша
    fn cache_stats(&self) -> CacheStats;
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
    pub memory_usage: usize,
}

/// Общий Config trait для всех конфигураций
pub trait ConfigTrait: Default + Clone + Send + Sync {
    /// Валидация конфигурации
    fn validate(&self) -> Result<(), crate::ConfigError> {
        Ok(())
    }
    
    /// Получить production конфигурацию
    fn production() -> Self {
        Self::default()
    }
    
    /// Получить minimal конфигурацию
    fn minimal() -> Self {
        Self::default()
    }
    
    /// Получить development конфигурацию
    fn development() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Default)]
    struct TestService {
        name: &'static str,
    }
    
    impl BaseService for TestService {
        fn name(&self) -> &'static str {
            "TestService"
        }
    }
    
    #[test]
    fn test_base_service() {
        let service = TestService::default();
        assert_eq!(service.name(), "TestService");
        assert!(service.is_healthy());
    }
    
    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay, Duration::from_millis(100));
        assert!(config.jitter);
    }
}