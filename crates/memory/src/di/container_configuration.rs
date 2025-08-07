//! Container Configuration - конфигурация для DI контейнера
//!
//! Отдельный модуль для настройки контейнера, следует Single Responsibility Principle.

use std::time::Duration;

/// Конфигурация DI контейнера
#[derive(Debug, Clone)]
pub struct ContainerConfiguration {
    /// Максимальный размер кэша singleton экземпляров
    pub max_cache_size: usize,
    /// Timeout для создания экземпляров
    pub instance_creation_timeout: Duration,
    /// Включить валидацию зависимостей
    pub enable_dependency_validation: bool,
    /// Включить сбор метрик производительности
    pub enable_performance_metrics: bool,
    /// Максимальная глубина зависимостей
    pub max_dependency_depth: u32,
    /// Cache cleanup interval
    pub cache_cleanup_interval: Duration,
}

impl Default for ContainerConfiguration {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            instance_creation_timeout: Duration::from_secs(30),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 20,
            cache_cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl ContainerConfiguration {
    /// Production конфигурация с оптимизированными параметрами
    pub fn production() -> Self {
        Self {
            max_cache_size: 5000,
            instance_creation_timeout: Duration::from_secs(10),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 15,
            cache_cleanup_interval: Duration::from_secs(600), // 10 minutes
        }
    }

    /// Development конфигурация с отладкой
    pub fn development() -> Self {
        Self {
            max_cache_size: 1000,
            instance_creation_timeout: Duration::from_secs(60),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 25,
            cache_cleanup_interval: Duration::from_secs(180), // 3 minutes
        }
    }

    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            max_cache_size: 100,
            instance_creation_timeout: Duration::from_secs(5),
            enable_dependency_validation: false,
            enable_performance_metrics: false,
            max_dependency_depth: 10,
            cache_cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }

    /// Создать конфигурацию для high-performance окружения
    pub fn high_performance() -> Self {
        Self {
            max_cache_size: 10000,
            instance_creation_timeout: Duration::from_secs(5),
            enable_dependency_validation: false, // Отключено для производительности
            enable_performance_metrics: true,
            max_dependency_depth: 10,
            cache_cleanup_interval: Duration::from_secs(900), // 15 minutes
        }
    }

    /// Создать конфигурацию для debugging
    pub fn debug() -> Self {
        Self {
            max_cache_size: 500,
            instance_creation_timeout: Duration::from_secs(120), // Длинный timeout для debugging
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 50, // Разрешаем глубокие зависимости для debugging
            cache_cleanup_interval: Duration::from_secs(60),
        }
    }

    /// Проверить валидность конфигурации
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.max_cache_size == 0 {
            return Err(anyhow::anyhow!("max_cache_size должен быть больше 0"));
        }

        if self.max_cache_size > 100_000 {
            return Err(anyhow::anyhow!(
                "max_cache_size не должен превышать 100,000 для избежания проблем с памятью"
            ));
        }

        if self.instance_creation_timeout.as_millis() == 0 {
            return Err(anyhow::anyhow!(
                "instance_creation_timeout должен быть больше 0"
            ));
        }

        if self.instance_creation_timeout.as_secs() > 600 {
            return Err(anyhow::anyhow!(
                "instance_creation_timeout не должен превышать 10 минут"
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

        if self.cache_cleanup_interval.as_secs() == 0 {
            return Err(anyhow::anyhow!(
                "cache_cleanup_interval должен быть больше 0"
            ));
        }

        Ok(())
    }

    /// Получить human-readable описание конфигурации
    pub fn describe(&self) -> String {
        format!(
            "DI Container Config: cache_size={}, timeout={}s, validation={}, metrics={}, max_depth={}, cleanup={}s",
            self.max_cache_size,
            self.instance_creation_timeout.as_secs(),
            self.enable_dependency_validation,
            self.enable_performance_metrics,
            self.max_dependency_depth,
            self.cache_cleanup_interval.as_secs()
        )
    }

    /// Сравнить с другой конфигурацией и выявить различия
    pub fn diff(&self, other: &Self) -> Vec<String> {
        let mut diffs = Vec::new();

        if self.max_cache_size != other.max_cache_size {
            diffs.push(format!(
                "max_cache_size: {} -> {}",
                self.max_cache_size, other.max_cache_size
            ));
        }

        if self.instance_creation_timeout != other.instance_creation_timeout {
            diffs.push(format!(
                "instance_creation_timeout: {}s -> {}s",
                self.instance_creation_timeout.as_secs(),
                other.instance_creation_timeout.as_secs()
            ));
        }

        if self.enable_dependency_validation != other.enable_dependency_validation {
            diffs.push(format!(
                "enable_dependency_validation: {} -> {}",
                self.enable_dependency_validation, other.enable_dependency_validation
            ));
        }

        if self.enable_performance_metrics != other.enable_performance_metrics {
            diffs.push(format!(
                "enable_performance_metrics: {} -> {}",
                self.enable_performance_metrics, other.enable_performance_metrics
            ));
        }

        if self.max_dependency_depth != other.max_dependency_depth {
            diffs.push(format!(
                "max_dependency_depth: {} -> {}",
                self.max_dependency_depth, other.max_dependency_depth
            ));
        }

        if self.cache_cleanup_interval != other.cache_cleanup_interval {
            diffs.push(format!(
                "cache_cleanup_interval: {}s -> {}s",
                self.cache_cleanup_interval.as_secs(),
                other.cache_cleanup_interval.as_secs()
            ));
        }

        diffs
    }

    /// Оптимизировать конфигурацию для текущего окружения
    pub fn optimize_for_environment(&mut self) {
        // Получаем информацию о доступной памяти (приблизительно)
        if let Ok(available_memory_mb) = self.estimate_available_memory() {
            // Адаптируем размер кэша к доступной памяти
            let recommended_cache_size = (available_memory_mb / 10).min(50_000).max(100);
            if recommended_cache_size != self.max_cache_size {
                tracing::info!(
                    "Адаптируем max_cache_size с {} на {} основе доступной памяти ({}MB)",
                    self.max_cache_size,
                    recommended_cache_size,
                    available_memory_mb
                );
                self.max_cache_size = recommended_cache_size;
            }
        }

        // В debug режиме увеличиваем timeouts
        if cfg!(debug_assertions) {
            self.instance_creation_timeout = self.instance_creation_timeout.mul_f32(2.0);
            tracing::debug!(
                "Debug режим: увеличен instance_creation_timeout до {:?}",
                self.instance_creation_timeout
            );
        }
    }

    /// Приблизительная оценка доступной памяти
    fn estimate_available_memory(&self) -> Result<usize, ()> {
        // Простая эвристика - в реальном проекте можно использовать sysinfo crate
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                if let Some(line) = meminfo
                    .lines()
                    .find(|line| line.starts_with("MemAvailable:"))
                {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<usize>() {
                            return Ok(kb / 1024); // Convert KB to MB
                        }
                    }
                }
            }
        }

        // Fallback для других OS или если не удалось прочитать
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = ContainerConfiguration::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_cache_size, 1000);
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(30));
        assert!(config.enable_dependency_validation);
        assert!(config.enable_performance_metrics);
    }

    #[test]
    fn test_production_configuration() {
        let config = ContainerConfiguration::production();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_cache_size, 5000);
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(10));
        assert_eq!(config.max_dependency_depth, 15);
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = ContainerConfiguration::default();
        assert!(config.validate().is_ok());

        // Test invalid max_cache_size
        config.max_cache_size = 0;
        assert!(config.validate().is_err());

        config.max_cache_size = 200_000;
        assert!(config.validate().is_err());

        // Test invalid timeout
        config.max_cache_size = 1000;
        config.instance_creation_timeout = Duration::from_secs(0);
        assert!(config.validate().is_err());

        config.instance_creation_timeout = Duration::from_secs(700);
        assert!(config.validate().is_err());

        // Test invalid dependency depth
        config.instance_creation_timeout = Duration::from_secs(30);
        config.max_dependency_depth = 0;
        assert!(config.validate().is_err());

        config.max_dependency_depth = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_configuration_describe() {
        let config = ContainerConfiguration::production();
        let description = config.describe();
        assert!(description.contains("cache_size=5000"));
        assert!(description.contains("timeout=10s"));
        assert!(description.contains("validation=true"));
    }

    #[test]
    fn test_configuration_diff() {
        let config1 = ContainerConfiguration::default();
        let config2 = ContainerConfiguration::production();

        let diffs = config1.diff(&config2);
        assert!(!diffs.is_empty());
        assert!(diffs
            .iter()
            .any(|d| d.contains("max_cache_size: 1000 -> 5000")));
        assert!(diffs
            .iter()
            .any(|d| d.contains("instance_creation_timeout: 30s -> 10s")));
    }

    #[test]
    fn test_high_performance_configuration() {
        let config = ContainerConfiguration::high_performance();
        assert!(config.validate().is_ok());
        assert_eq!(config.max_cache_size, 10000);
        assert!(!config.enable_dependency_validation); // Отключено для производительности
        assert!(config.enable_performance_metrics);
    }

    #[test]
    fn test_debug_configuration() {
        let config = ContainerConfiguration::debug();
        assert!(config.validate().is_ok());
        assert_eq!(config.instance_creation_timeout, Duration::from_secs(120));
        assert_eq!(config.max_dependency_depth, 50);
        assert!(config.enable_dependency_validation);
    }

    #[test]
    fn test_optimize_for_environment() {
        let mut config = ContainerConfiguration::default();
        let original_timeout = config.instance_creation_timeout;

        config.optimize_for_environment();

        // В debug режиме timeout должен увеличиться
        if cfg!(debug_assertions) {
            assert!(config.instance_creation_timeout > original_timeout);
        }
    }
}
