//! Container Factory - фабрика для создания DI контейнеров
//!
//! Применяет Factory Pattern и Builder Pattern для упрощения создания контейнеров.

use anyhow::Result;
use std::time::Duration;

use super::{
    container_configuration::ContainerConfiguration, unified_container::UnifiedDIContainer,
};

/// Factory для создания различных типов DI контейнеров
pub struct ContainerFactory;

impl ContainerFactory {
    /// Создать production контейнер с оптимальными настройками
    pub fn create_production() -> UnifiedDIContainer {
        let config = ContainerConfiguration {
            max_cache_size: 5000,
            instance_creation_timeout: Duration::from_secs(10),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 15,
            cache_cleanup_interval: Duration::from_secs(600),
        };

        UnifiedDIContainer::with_configuration(config)
    }

    /// Создать development контейнер с расширенным debugging
    pub fn create_development() -> UnifiedDIContainer {
        let config = ContainerConfiguration {
            max_cache_size: 1000,
            instance_creation_timeout: Duration::from_secs(60),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 25,
            cache_cleanup_interval: Duration::from_secs(180),
        };

        UnifiedDIContainer::with_configuration(config)
    }

    /// Создать minimal контейнер для unit тестов
    pub fn create_test() -> UnifiedDIContainer {
        let config = ContainerConfiguration {
            max_cache_size: 100,
            instance_creation_timeout: Duration::from_secs(5),
            enable_dependency_validation: false,
            enable_performance_metrics: false,
            max_dependency_depth: 10,
            cache_cleanup_interval: Duration::from_secs(60),
        };

        UnifiedDIContainer::with_configuration(config)
    }

    /// Создать кастомный контейнер из конфигурации
    pub fn create_with_config(config: ContainerConfiguration) -> UnifiedDIContainer {
        UnifiedDIContainer::with_configuration(config)
    }

    /// Создать контейнер из переменных окружения
    pub fn create_from_environment() -> Result<UnifiedDIContainer> {
        let config = ContainerConfiguration::from_environment()?;
        Ok(UnifiedDIContainer::with_configuration(config))
    }
}

impl ContainerConfiguration {
    /// Загрузить конфигурацию из переменных окружения
    pub fn from_environment() -> Result<Self> {
        let max_cache_size = std::env::var("DI_MAX_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let instance_timeout_secs = std::env::var("DI_INSTANCE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let enable_validation = std::env::var("DI_ENABLE_VALIDATION")
            .ok()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        let enable_metrics = std::env::var("DI_ENABLE_METRICS")
            .ok()
            .map(|s| s.to_lowercase() == "true")
            .unwrap_or(true);

        let max_depth = std::env::var("DI_MAX_DEPENDENCY_DEPTH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20);

        let cleanup_interval_secs = std::env::var("DI_CLEANUP_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        Ok(Self {
            max_cache_size,
            instance_creation_timeout: Duration::from_secs(instance_timeout_secs),
            enable_dependency_validation: enable_validation,
            enable_performance_metrics: enable_metrics,
            max_dependency_depth: max_depth,
            cache_cleanup_interval: Duration::from_secs(cleanup_interval_secs),
        })
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.max_cache_size == 0 {
            return Err(anyhow::anyhow!("max_cache_size должен быть больше 0"));
        }

        if self.instance_creation_timeout.as_secs() == 0 {
            return Err(anyhow::anyhow!(
                "instance_creation_timeout должен быть больше 0"
            ));
        }

        if self.max_dependency_depth == 0 {
            return Err(anyhow::anyhow!("max_dependency_depth должен быть больше 0"));
        }

        if self.max_dependency_depth > 100 {
            return Err(anyhow::anyhow!(
                "max_dependency_depth не должен превышать 100 (возможная циклическая зависимость)"
            ));
        }

        Ok(())
    }
}

/// Builder для пошагового создания контейнера
pub struct ContainerBuilder {
    config: ContainerConfiguration,
}

impl ContainerBuilder {
    /// Создать новый builder с default конфигурацией
    pub fn new() -> Self {
        Self {
            config: ContainerConfiguration::default(),
        }
    }

    /// Установить максимальный размер кэша
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.config.max_cache_size = size;
        self
    }

    /// Установить timeout для создания экземпляров
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.instance_creation_timeout = timeout;
        self
    }

    /// Включить/выключить валидацию зависимостей
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.config.enable_dependency_validation = enabled;
        self
    }

    /// Включить/выключить сбор метрик
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.config.enable_performance_metrics = enabled;
        self
    }

    /// Установить максимальную глубину зависимостей
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.config.max_dependency_depth = depth;
        self
    }

    /// Установить интервал очистки кэша
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.config.cache_cleanup_interval = interval;
        self
    }

    /// Применить preset конфигурацию
    pub fn with_preset(mut self, preset: ContainerPreset) -> Self {
        self.config = match preset {
            ContainerPreset::Production => ContainerConfiguration::production(),
            ContainerPreset::Development => ContainerConfiguration::development(),
            ContainerPreset::Testing => ContainerConfiguration::minimal(),
        };
        self
    }

    /// Построить контейнер
    pub fn build(self) -> Result<UnifiedDIContainer> {
        // Валидируем конфигурацию перед созданием
        self.config.validate()?;

        Ok(UnifiedDIContainer::with_configuration(self.config))
    }
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Preset конфигурации для разных окружений
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContainerPreset {
    Production,
    Development,
    Testing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_factory_production() {
        let container = ContainerFactory::create_production();
        // Проверяем что контейнер создан (тест компиляции)
        assert_eq!(container.get_configuration().max_cache_size, 5000);
    }

    #[test]
    fn test_container_factory_development() {
        let container = ContainerFactory::create_development();
        assert_eq!(container.get_configuration().max_cache_size, 1000);
        assert_eq!(
            container.get_configuration().instance_creation_timeout,
            Duration::from_secs(60)
        );
    }

    #[test]
    fn test_container_factory_test() {
        let container = ContainerFactory::create_test();
        assert_eq!(container.get_configuration().max_cache_size, 100);
        assert!(!container.get_configuration().enable_dependency_validation);
        assert!(!container.get_configuration().enable_performance_metrics);
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = ContainerConfiguration::default();
        assert!(config.validate().is_ok());

        config.max_cache_size = 0;
        assert!(config.validate().is_err());

        config.max_cache_size = 100;
        config.max_dependency_depth = 0;
        assert!(config.validate().is_err());

        config.max_dependency_depth = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_container_builder() {
        let container = ContainerBuilder::new()
            .with_cache_size(500)
            .with_timeout(Duration::from_secs(15))
            .with_validation(false)
            .with_metrics(true)
            .build()
            .expect("Should build valid container");

        let config = container.get_configuration();
        assert_eq!(config.max_cache_size, 500);
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(15));
        assert!(!config.enable_dependency_validation);
        assert!(config.enable_performance_metrics);
    }

    #[test]
    fn test_container_builder_with_preset() {
        let container = ContainerBuilder::new()
            .with_preset(ContainerPreset::Production)
            .build()
            .expect("Should build production container");

        let config = container.get_configuration();
        assert_eq!(config.max_cache_size, 5000);
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_configuration_from_environment() {
        // Устанавливаем тестовые переменные окружения
        std::env::set_var("DI_MAX_CACHE_SIZE", "2000");
        std::env::set_var("DI_INSTANCE_TIMEOUT_SECS", "45");
        std::env::set_var("DI_ENABLE_VALIDATION", "false");
        std::env::set_var("DI_ENABLE_METRICS", "true");

        let config =
            ContainerConfiguration::from_environment().expect("Should parse environment variables");

        assert_eq!(config.max_cache_size, 2000);
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(45));
        assert!(!config.enable_dependency_validation);
        assert!(config.enable_performance_metrics);

        // Очищаем переменные окружения после теста
        std::env::remove_var("DI_MAX_CACHE_SIZE");
        std::env::remove_var("DI_INSTANCE_TIMEOUT_SECS");
        std::env::remove_var("DI_ENABLE_VALIDATION");
        std::env::remove_var("DI_ENABLE_METRICS");
    }
}
