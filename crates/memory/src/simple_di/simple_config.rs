//! Упрощенная конфигурация для DI системы
//!
//! ЗАМЕНЯЕТ ВСЕ СЛОЖНЫЕ КОНФИГУРАЦИИ:
//! - unified_config.rs (300+ строк сложной конфигурации)
//! - container_configuration.rs (200+ строк)
//! - config_validation.rs, config_presets.rs и др.
//!
//! ПРИНЦИПЫ УПРОЩЕНИЯ:
//! - Только самые необходимые настройки
//! - Default values для всего
//! - Никаких сложных validation rules
//! - Простая структура без вложенности

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Простая конфигурация DI контейнера
/// Заменяет все сложные конфигурационные файлы из di/ папки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleConfig {
    /// Максимальное количество сервисов (защита от утечек памяти)
    pub max_services: usize,

    /// Включить отладочные логи
    pub debug_logging: bool,

    /// Тайм-аут для создания сервисов
    pub service_creation_timeout: Duration,
}

impl SimpleConfig {
    /// Создать конфигурацию по умолчанию
    pub fn new() -> Self {
        Self {
            max_services: 1000,
            debug_logging: false,
            service_creation_timeout: Duration::from_secs(30),
        }
    }

    /// Конфигурация для разработки
    pub fn development() -> Self {
        Self {
            max_services: 500,
            debug_logging: true,
            service_creation_timeout: Duration::from_secs(10),
        }
    }

    /// Конфигурация для production
    pub fn production() -> Self {
        Self {
            max_services: 2000,
            debug_logging: false,
            service_creation_timeout: Duration::from_secs(60),
        }
    }

    /// Минимальная конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            max_services: 50,
            debug_logging: false,
            service_creation_timeout: Duration::from_secs(5),
        }
    }

    /// Проверить валидность конфигурации (минимальная валидация)
    pub fn validate(&self) -> Result<(), String> {
        if self.max_services == 0 {
            return Err("max_services must be greater than 0".to_string());
        }

        if self.service_creation_timeout.is_zero() {
            return Err("service_creation_timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}

impl Default for SimpleConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder для конфигурации (для удобства)
pub struct ConfigBuilder {
    config: SimpleConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SimpleConfig::new(),
        }
    }

    pub fn max_services(mut self, max: usize) -> Self {
        self.config.max_services = max;
        self
    }

    pub fn debug_logging(mut self, enabled: bool) -> Self {
        self.config.debug_logging = enabled;
        self
    }

    pub fn service_timeout(mut self, timeout: Duration) -> Self {
        self.config.service_creation_timeout = timeout;
        self
    }

    pub fn build(self) -> SimpleConfig {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SimpleConfig::default();

        assert_eq!(config.max_services, 1000);
        assert!(!config.debug_logging);
        assert_eq!(config.service_creation_timeout, Duration::from_secs(30));

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_preset_configs() {
        let dev = SimpleConfig::development();
        assert!(dev.debug_logging);
        assert_eq!(dev.max_services, 500);

        let prod = SimpleConfig::production();
        assert!(!prod.debug_logging);
        assert_eq!(prod.max_services, 2000);

        let minimal = SimpleConfig::minimal();
        assert_eq!(minimal.max_services, 50);
    }

    #[test]
    fn test_config_validation() {
        let mut config = SimpleConfig::new();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid max_services
        config.max_services = 0;
        assert!(config.validate().is_err());

        // Invalid timeout
        config.max_services = 100;
        config.service_creation_timeout = Duration::ZERO;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .max_services(123)
            .debug_logging(true)
            .service_timeout(Duration::from_secs(15))
            .build();

        assert_eq!(config.max_services, 123);
        assert!(config.debug_logging);
        assert_eq!(config.service_creation_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_config_serialization() {
        let config = SimpleConfig::development();

        // Test serialization
        let json = serde_json::to_string(&config).expect("Operation failed - converted from unwrap()");
        assert!(!json.is_empty());

        // Test deserialization
        let deserialized: SimpleConfig = serde_json::from_str(&json).expect("Operation failed - converted from unwrap()");
        assert_eq!(config.max_services, deserialized.max_services);
        assert_eq!(config.debug_logging, deserialized.debug_logging);
    }
}
