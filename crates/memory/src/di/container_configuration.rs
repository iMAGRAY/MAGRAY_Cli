//! DI Container Configuration - централизованная конфигурация всех DI компонентов
//!
//! ЕДИНСТВЕННАЯ ОТВЕТСТВЕННОСТЬ: Управление конфигурацией DI системы
//! ПРИНЦИПЫ: Immutable configuration, preset patterns, validation

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Основная конфигурация DI контейнера
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DIContainerConfiguration {
    /// Настройки кэширования
    pub cache: CacheConfiguration,
    /// Настройки производительности
    pub performance: PerformanceConfiguration,
    /// Настройки валидации
    pub validation: ValidationConfiguration,
    /// Настройки мониторинга
    pub monitoring: MonitoringConfiguration,
}

/// Конфигурация кэширования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfiguration {
    /// Максимальный размер кэша singleton экземпляров
    pub max_singleton_cache_size: usize,
    /// Максимальный размер кэша scoped экземпляров  
    pub max_scoped_cache_size: usize,
    /// Максимальный возраст cached экземпляра
    pub max_instance_age: Duration,
    /// Максимальное время неактивности перед eviction
    pub max_idle_time: Duration,
    /// Интервал автоматической очистки
    pub cleanup_interval: Duration,
}

/// Конфигурация производительности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfiguration {
    /// Timeout для создания экземпляров
    pub instance_creation_timeout: Duration,
    /// Максимальная глубина зависимостей
    pub max_dependency_depth: u32,
    /// Включить сбор метрик производительности
    pub enable_metrics_collection: bool,
    /// Включить детальное логирование
    pub enable_detailed_logging: bool,
}

/// Конфигурация валидации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfiguration {
    /// Включить валидацию зависимостей
    pub enable_dependency_validation: bool,
    /// Включить проверку циклических зависимостей
    pub enable_cycle_detection: bool,
    /// Включить проверку типов при регистрации
    pub enable_type_validation: bool,
    /// Строгий режим валидации
    pub strict_validation: bool,
}

/// Конфигурация мониторинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    /// Включить health checks
    pub enable_health_monitoring: bool,
    /// Интервал проверки здоровья системы
    pub health_check_interval: Duration,
    /// Включить экспорт метрик
    pub enable_metrics_export: bool,
    /// Уровень детализации логирования
    pub log_level: LogLevel,
}

/// Уровень логирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for DIContainerConfiguration {
    fn default() -> Self {
        Self {
            cache: CacheConfiguration::default(),
            performance: PerformanceConfiguration::default(),
            validation: ValidationConfiguration::default(),
            monitoring: MonitoringConfiguration::default(),
        }
    }
}

impl Default for CacheConfiguration {
    fn default() -> Self {
        Self {
            max_singleton_cache_size: 1000,
            max_scoped_cache_size: 500,
            max_instance_age: Duration::from_secs(3600), // 1 hour
            max_idle_time: Duration::from_secs(600),     // 10 minutes
            cleanup_interval: Duration::from_secs(300),  // 5 minutes
        }
    }
}

impl Default for PerformanceConfiguration {
    fn default() -> Self {
        Self {
            instance_creation_timeout: Duration::from_secs(30),
            max_dependency_depth: 20,
            enable_metrics_collection: true,
            enable_detailed_logging: false,
        }
    }
}

impl Default for ValidationConfiguration {
    fn default() -> Self {
        Self {
            enable_dependency_validation: true,
            enable_cycle_detection: true,
            enable_type_validation: true,
            strict_validation: false,
        }
    }
}

impl Default for MonitoringConfiguration {
    fn default() -> Self {
        Self {
            enable_health_monitoring: true,
            health_check_interval: Duration::from_secs(60),
            enable_metrics_export: false,
            log_level: LogLevel::Info,
        }
    }
}

impl DIContainerConfiguration {
    /// Production-оптимизированная конфигурация
    pub fn production() -> Self {
        Self {
            cache: CacheConfiguration {
                max_singleton_cache_size: 5000,
                max_scoped_cache_size: 2500,
                max_instance_age: Duration::from_secs(7200), // 2 hours
                max_idle_time: Duration::from_secs(1200),    // 20 minutes
                cleanup_interval: Duration::from_secs(600),  // 10 minutes
            },
            performance: PerformanceConfiguration {
                instance_creation_timeout: Duration::from_secs(10),
                max_dependency_depth: 15,
                enable_metrics_collection: true,
                enable_detailed_logging: false,
            },
            validation: ValidationConfiguration {
                enable_dependency_validation: true,
                enable_cycle_detection: true,
                enable_type_validation: true,
                strict_validation: true,
            },
            monitoring: MonitoringConfiguration {
                enable_health_monitoring: true,
                health_check_interval: Duration::from_secs(30),
                enable_metrics_export: true,
                log_level: LogLevel::Warn,
            },
        }
    }

    /// Development-конфигурация с расширенной отладкой
    pub fn development() -> Self {
        Self {
            cache: CacheConfiguration {
                max_singleton_cache_size: 500,
                max_scoped_cache_size: 250,
                max_instance_age: Duration::from_secs(1800), // 30 minutes
                max_idle_time: Duration::from_secs(300),     // 5 minutes
                cleanup_interval: Duration::from_secs(180),  // 3 minutes
            },
            performance: PerformanceConfiguration {
                instance_creation_timeout: Duration::from_secs(60),
                max_dependency_depth: 25,
                enable_metrics_collection: true,
                enable_detailed_logging: true,
            },
            validation: ValidationConfiguration {
                enable_dependency_validation: true,
                enable_cycle_detection: true,
                enable_type_validation: true,
                strict_validation: false,
            },
            monitoring: MonitoringConfiguration {
                enable_health_monitoring: true,
                health_check_interval: Duration::from_secs(15),
                enable_metrics_export: false,
                log_level: LogLevel::Debug,
            },
        }
    }

    /// Minimal конфигурация для ресурсо-ограниченных сред
    pub fn minimal() -> Self {
        Self {
            cache: CacheConfiguration {
                max_singleton_cache_size: 100,
                max_scoped_cache_size: 50,
                max_instance_age: Duration::from_secs(900), // 15 minutes
                max_idle_time: Duration::from_secs(180),    // 3 minutes
                cleanup_interval: Duration::from_secs(120), // 2 minutes
            },
            performance: PerformanceConfiguration {
                instance_creation_timeout: Duration::from_secs(15),
                max_dependency_depth: 10,
                enable_metrics_collection: false,
                enable_detailed_logging: false,
            },
            validation: ValidationConfiguration {
                enable_dependency_validation: false,
                enable_cycle_detection: true, // Всё равно важно для корректности
                enable_type_validation: false,
                strict_validation: false,
            },
            monitoring: MonitoringConfiguration {
                enable_health_monitoring: false,
                health_check_interval: Duration::from_secs(300),
                enable_metrics_export: false,
                log_level: LogLevel::Error,
            },
        }
    }

    /// Testing конфигурация для unit/integration тестов
    pub fn testing() -> Self {
        Self {
            cache: CacheConfiguration {
                max_singleton_cache_size: 50,
                max_scoped_cache_size: 25,
                max_instance_age: Duration::from_secs(10),
                max_idle_time: Duration::from_secs(5),
                cleanup_interval: Duration::from_secs(1),
            },
            performance: PerformanceConfiguration {
                instance_creation_timeout: Duration::from_secs(5),
                max_dependency_depth: 10,
                enable_metrics_collection: true,
                enable_detailed_logging: true,
            },
            validation: ValidationConfiguration {
                enable_dependency_validation: true,
                enable_cycle_detection: true,
                enable_type_validation: true,
                strict_validation: true,
            },
            monitoring: MonitoringConfiguration {
                enable_health_monitoring: false,
                health_check_interval: Duration::from_secs(1),
                enable_metrics_export: false,
                log_level: LogLevel::Trace,
            },
        }
    }

    /// Валидация конфигурации
    pub fn validate(&self) -> Result<()> {
        // Cache validation
        if self.cache.max_singleton_cache_size == 0 {
            return Err(anyhow!("Singleton cache size must be greater than 0"));
        }
        if self.cache.max_scoped_cache_size == 0 {
            return Err(anyhow!("Scoped cache size must be greater than 0"));
        }
        if self.cache.cleanup_interval.is_zero() {
            return Err(anyhow!("Cleanup interval must be greater than 0"));
        }

        // Performance validation
        if self.performance.instance_creation_timeout.is_zero() {
            return Err(anyhow!("Instance creation timeout must be greater than 0"));
        }
        if self.performance.max_dependency_depth == 0 {
            return Err(anyhow!("Max dependency depth must be greater than 0"));
        }
        if self.performance.max_dependency_depth > 100 {
            return Err(anyhow!(
                "Max dependency depth too high (>100), potential stack overflow"
            ));
        }

        // Monitoring validation
        if self.monitoring.enable_health_monitoring
            && self.monitoring.health_check_interval.is_zero()
        {
            return Err(anyhow!(
                "Health check interval must be greater than 0 when monitoring enabled"
            ));
        }

        Ok(())
    }

    /// Создать конфигурацию из JSON
    pub fn from_json(json: &str) -> Result<Self> {
        let config: DIContainerConfiguration = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse configuration JSON: {}", e))?;

        config.validate()?;
        Ok(config)
    }

    /// Сохранить конфигурацию в JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize configuration to JSON: {}", e))
    }

    /// Создать конфигурацию из TOML
    pub fn from_toml(toml: &str) -> Result<Self> {
        let config: DIContainerConfiguration = toml::from_str(toml)
            .map_err(|e| anyhow!("Failed to parse configuration TOML: {}", e))?;

        config.validate()?;
        Ok(config)
    }

    /// Сохранить конфигурацию в TOML
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize configuration to TOML: {}", e))
    }

    /// Объединить с другой конфигурацией (self имеет приоритет)
    pub fn merge_with(self, _other: DIContainerConfiguration) -> Self {
        // При merge оставляем существующие значения, заполняем только пропущенные
        // В данном случае все поля обязательны, поэтому просто возвращаем self
        self
    }

    /// Применить environment-specific настройки
    pub fn apply_environment_overrides(mut self, env: Environment) -> Self {
        match env {
            Environment::Production => {
                self.monitoring.log_level = LogLevel::Warn;
                self.performance.enable_detailed_logging = false;
            }
            Environment::Development => {
                self.monitoring.log_level = LogLevel::Debug;
                self.performance.enable_detailed_logging = true;
            }
            Environment::Testing => {
                self.monitoring.log_level = LogLevel::Trace;
                self.performance.enable_detailed_logging = true;
                self.cache.cleanup_interval = Duration::from_millis(100);
            }
        }
        self
    }
}

/// Environment типы
#[derive(Debug, Clone, Copy)]
pub enum Environment {
    Production,
    Development,
    Testing,
}

/// Configuration builder для пошаговой настройки
pub struct DIConfigurationBuilder {
    config: DIContainerConfiguration,
}

impl DIConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            config: DIContainerConfiguration::default(),
        }
    }

    pub fn cache_size(mut self, singleton: usize, scoped: usize) -> Self {
        self.config.cache.max_singleton_cache_size = singleton;
        self.config.cache.max_scoped_cache_size = scoped;
        self
    }

    pub fn timeouts(mut self, creation: Duration, max_age: Duration) -> Self {
        self.config.performance.instance_creation_timeout = creation;
        self.config.cache.max_instance_age = max_age;
        self
    }

    pub fn validation(mut self, enabled: bool, strict: bool) -> Self {
        self.config.validation.enable_dependency_validation = enabled;
        self.config.validation.strict_validation = strict;
        self
    }

    pub fn monitoring(mut self, enabled: bool, level: LogLevel) -> Self {
        self.config.monitoring.enable_health_monitoring = enabled;
        self.config.monitoring.log_level = level;
        self
    }

    pub fn build(self) -> Result<DIContainerConfiguration> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for DIConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = DIContainerConfiguration::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_configurations() {
        assert!(DIContainerConfiguration::production().validate().is_ok());
        assert!(DIContainerConfiguration::development().validate().is_ok());
        assert!(DIContainerConfiguration::minimal().validate().is_ok());
        assert!(DIContainerConfiguration::testing().validate().is_ok());
    }

    #[test]
    fn test_configuration_builder() {
        let config = DIConfigurationBuilder::new()
            .cache_size(1000, 500)
            .timeouts(Duration::from_secs(30), Duration::from_secs(3600))
            .validation(true, false)
            .monitoring(true, LogLevel::Info)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.cache.max_singleton_cache_size, 1000);
        assert_eq!(config.cache.max_scoped_cache_size, 500);
    }

    #[test]
    fn test_json_serialization() {
        let config = DIContainerConfiguration::production();
        let json = config.to_json().unwrap();
        let deserialized = DIContainerConfiguration::from_json(&json).unwrap();

        // Проверяем ключевые поля
        assert_eq!(
            config.cache.max_singleton_cache_size,
            deserialized.cache.max_singleton_cache_size
        );
        assert_eq!(
            config.performance.max_dependency_depth,
            deserialized.performance.max_dependency_depth
        );
    }

    #[test]
    fn test_validation_errors() {
        let mut config = DIContainerConfiguration::default();

        // Test zero cache size
        config.cache.max_singleton_cache_size = 0;
        assert!(config.validate().is_err());

        config.cache.max_singleton_cache_size = 100;
        config.performance.max_dependency_depth = 0;
        assert!(config.validate().is_err());

        config.performance.max_dependency_depth = 200; // Too high
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_environment_overrides() {
        let config = DIContainerConfiguration::default()
            .apply_environment_overrides(Environment::Production);

        match config.monitoring.log_level {
            LogLevel::Warn => {}
            _ => panic!("Expected Warn log level for production"),
        }
    }
}
