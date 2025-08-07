use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// Базовые trait abstractions для Service patterns в MAGRAY CLI
///
/// Этот модуль содержит общие trait определения для устранения дублирования
/// в service implementations across все крейты

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
        Self::Metrics: std::fmt::Debug,
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

/// Универсальный trait для health checks - устраняет дублирование fn health_check
#[async_trait]
pub trait HealthCheckService: BaseService {
    type HealthData: Send + Sync + Clone;

    /// Проверка здоровья сервиса
    async fn health_check(&self) -> Result<Self::HealthData, crate::MagrayCoreError>;

    /// Простая проверка доступности
    async fn ping(&self) -> Result<(), crate::MagrayCoreError> {
        self.health_check().await.map(|_| ())
    }

    /// Детальная проверка с таймаутом
    async fn health_check_with_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Self::HealthData, crate::MagrayCoreError> {
        tokio::time::timeout(timeout, self.health_check())
            .await
            .map_err(|_| crate::MagrayCoreError::Timeout)?
    }
}

/// Trait для получения статистик - устраняет дублирование fn get_stats
pub trait StatisticsProvider: BaseService {
    type Stats: Send + Sync + Clone + std::fmt::Debug;

    /// Получить текущую статистику
    fn get_stats(&self) -> Self::Stats;

    /// Сбросить статистику
    fn reset_stats(&mut self) {
        // Default implementation - может быть переопределена
    }

    /// Экспорт статистики в JSON формате
    fn export_stats_json(&self) -> Result<String, crate::MagrayCoreError>
    where
        Self::Stats: serde::Serialize,
    {
        serde_json::to_string(&self.get_stats())
            .map_err(|e| crate::MagrayCoreError::Serialization(e.to_string()))
    }
}

/// Trait для конфигурационных профилей - устраняет дублирование fn production/minimal
pub trait ConfigurationProfile<T>: Send + Sync {
    /// Получить production профиль
    fn production() -> T;

    /// Получить minimal профиль
    fn minimal() -> T;

    /// Получить development профиль
    fn development() -> T {
        Self::minimal()
    }

    /// Получить testing профиль
    fn testing() -> T {
        Self::minimal()
    }

    /// Валидация профиля
    fn validate_profile(config: &T) -> Result<(), crate::ConfigError>;
}

/// Trait для операций инициализации - устраняет дублирование fn initialize
#[async_trait]
pub trait InitializableService: BaseService {
    type InitConfig: Send + Sync;

    /// Инициализация с конфигурацией
    async fn initialize(&mut self, config: Self::InitConfig) -> Result<(), crate::MagrayCoreError>;

    /// Инициализация с default конфигурацией
    async fn initialize_default(&mut self) -> Result<(), crate::MagrayCoreError>
    where
        Self::InitConfig: Default,
    {
        self.initialize(Self::InitConfig::default()).await
    }

    /// Проверка готовности к инициализации
    fn is_ready_for_init(&self) -> bool {
        true
    }
}

/// Trait для операций построения/сборки - устраняет дублирование fn build
pub trait BuildableService<T> {
    type BuildConfig: Send + Sync;
    type BuildError: std::error::Error + Send + Sync + 'static;

    /// Построить сервис
    fn build(config: Self::BuildConfig) -> Result<T, Self::BuildError>;

    /// Построить с default конфигурацией
    fn build_default() -> Result<T, Self::BuildError>
    where
        Self::BuildConfig: Default,
    {
        Self::build(Self::BuildConfig::default())
    }
}

/// Trait для операций очистки - устраняет дублирование fn clear
#[async_trait]
pub trait ClearableService: BaseService {
    /// Очистить состояние сервиса
    async fn clear(&mut self) -> Result<(), crate::MagrayCoreError>;

    /// Очистить с подтверждением
    async fn clear_with_confirmation(
        &mut self,
        confirm: bool,
    ) -> Result<(), crate::MagrayCoreError> {
        if confirm {
            self.clear().await
        } else {
            Err(crate::MagrayCoreError::OperationCancelled)
        }
    }

    /// Проверка перед очисткой
    async fn can_clear(&self) -> bool {
        true
    }
}

/// Trait для операций поиска - устраняет дублирование fn search
#[async_trait]
pub trait SearchableService<Q: Send + 'static, R: Clone + Send + 'static>: BaseService {
    type SearchError: std::error::Error + Send + Sync + 'static;

    /// Выполнить поиск
    async fn search(&self, query: Q) -> Result<Vec<R>, Self::SearchError>;

    /// Поиск с лимитом результатов
    async fn search_limited(&self, query: Q, limit: usize) -> Result<Vec<R>, Self::SearchError> {
        let mut results = self.search(query).await?;
        results.truncate(limit);
        Ok(results)
    }

    /// Поиск с пагинацией
    async fn search_paginated(
        &self,
        query: Q,
        page: usize,
        page_size: usize,
    ) -> Result<SearchPage<R>, Self::SearchError> {
        let results = self.search(query).await?;
        let start = page * page_size;
        let end = std::cmp::min(start + page_size, results.len());

        let page_results = if start < results.len() {
            results[start..end].to_vec()
        } else {
            vec![]
        };

        Ok(SearchPage {
            results: page_results,
            page,
            page_size,
            total_count: results.len(),
            has_next: end < results.len(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchPage<T> {
    pub results: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub has_next: bool,
}

/// Trait для операций выполнения - устраняет дублирование fn execute
#[async_trait]
pub trait ExecutableService<I: Send + 'static, O: Send + 'static>: BaseService {
    type ExecuteError: std::error::Error + Send + Sync + 'static;

    /// Выполнить операцию
    async fn execute(&self, input: I) -> Result<O, Self::ExecuteError>;

    /// Выполнить с таймаутом
    async fn execute_with_timeout(
        &self,
        input: I,
        timeout: std::time::Duration,
    ) -> Result<O, Self::ExecuteError> {
        tokio::time::timeout(timeout, self.execute(input))
            .await
            .map_err(|_| -> Self::ExecuteError {
                // Нужно будет адаптировать для каждой реализации
                panic!("Timeout error needs to be converted to ExecuteError")
            })?
    }
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
    async fn register_service(
        &mut self,
        service: Arc<Self::Service>,
    ) -> Result<(), crate::MagrayCoreError>;

    /// Отменить регистрацию сервиса
    async fn unregister_service(
        &mut self,
        service_name: &str,
    ) -> Result<(), crate::MagrayCoreError>;

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

/// Общий Config trait для всех конфигураций - обновлен для устранения дублирования
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

    /// Получить testing конфигурацию
    fn testing() -> Self {
        Self::minimal()
    }

    /// Получить debug конфигурацию
    fn debug() -> Self {
        Self::development()
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
