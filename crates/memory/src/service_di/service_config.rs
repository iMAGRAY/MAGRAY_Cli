//! Service Configuration Module - Single Responsibility для конфигурации
//!
//! Этот модуль отвечает ТОЛЬКО за конфигурацию DIMemoryService.
//! Применяет Single Responsibility и Dependency Inversion принципы.

use ai::AiConfig;
use anyhow::Result;
use common::service_traits::ConfigurationProfile;
use std::path::PathBuf;

use crate::{
    cache_lru::CacheConfig,
    resource_manager::ResourceConfig,
};
#[cfg(all(not(feature = "minimal"), feature = "gpu-acceleration"))]
use crate::gpu_accelerated::GpuDeviceManager;
#[cfg(not(feature = "minimal"))]
use crate::notifications::NotificationConfig;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
use crate::{ml_promotion::MLPromotionConfig, promotion::PromotionConfig as MLPromotionCfg};
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
use crate::batch_manager::BatchConfig;
use crate::{health::HealthMonitorConfig, streaming::StreamingConfig, types::PromotionConfig, CacheConfigType};

// Fallback BatchConfig when persistence is disabled
#[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub flush_interval_ms: u64,
}

#[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
impl Default for BatchConfig {
    fn default() -> Self {
        Self { max_batch_size: 64, flush_interval_ms: 100 }
    }
}

#[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
impl BatchConfig {
    pub fn production() -> Self { Self { max_batch_size: 512, flush_interval_ms: 50 } }
    pub fn minimal() -> Self { Self { max_batch_size: 16, flush_interval_ms: 250 } }
}

/// Типы конфигурации Memory Service
#[derive(Debug, Clone)]
pub enum ServiceConfigType {
    /// Production конфигурация со всеми координаторами
    Production,
    /// Минимальная конфигурация для тестов
    Minimal,
    /// CPU-only конфигурация без GPU
    CpuOnly,
}

/// Trait для создания конфигураций (Open/Closed Principle)
pub trait ServiceConfigFactory {
    fn create_config(&self, config_type: ServiceConfigType) -> Result<MemoryServiceConfig>;
}

/// Главная конфигурация Memory Service
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryServiceConfig {
    pub db_path: PathBuf,
    pub cache_path: PathBuf,
    pub promotion: PromotionConfig,
    #[cfg(all(not(feature = "minimal"), feature = "persistence"))]
    pub ml_promotion: Option<MLPromotionConfig>,
    #[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
    pub ml_promotion: Option<PromotionConfig>,
    pub streaming_config: Option<StreamingConfig>,
    pub ai_config: AiConfig,
    pub cache_config: CacheConfigType,
    pub health_enabled: bool,
    pub health_config: HealthMonitorConfig,
    pub resource_config: ResourceConfig,
    #[cfg(not(feature = "minimal"))]
    pub notification_config: NotificationConfig,
    pub batch_config: BatchConfig,
}

impl Default for MemoryServiceConfig {
    fn default() -> Self {
        Self::create_default().expect("Не удалось создать конфигурацию по умолчанию")
    }
}

impl MemoryServiceConfig {
    /// Создать production конфигурацию
    pub fn create_production() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
            .join("magray");

        Ok(Self {
            db_path: cache_dir.join("memory.db"),
            cache_path: cache_dir.join("embeddings_cache"),
            promotion: PromotionConfig::production(),
            #[cfg(all(not(feature = "minimal"), feature = "persistence"))]
            ml_promotion: Some(MLPromotionConfig::production()),
            #[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
            ml_promotion: None,
            streaming_config: Some(StreamingConfig::production()),
            ai_config: AiConfig::production(),
            cache_config: CacheConfigType::production(),
            health_enabled: true,
            health_config: HealthMonitorConfig::production(),
            resource_config: ResourceConfig::production(),
            #[cfg(not(feature = "minimal"))]
            notification_config: NotificationConfig::production(),
            batch_config: BatchConfig::production(),
        })
    }

    /// Создать минимальную конфигурацию для тестов
    pub fn create_minimal() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("magray_test");

        Ok(Self {
            db_path: temp_dir.join("test_memory.db"),
            cache_path: temp_dir.join("test_cache"),
            promotion: PromotionConfig::minimal(),
            ml_promotion: None,
            streaming_config: None,
            ai_config: AiConfig::minimal(),
            cache_config: CacheConfigType::minimal(),
            health_enabled: false,
            health_config: HealthMonitorConfig::minimal(),
            resource_config: ResourceConfig::minimal(),
            #[cfg(not(feature = "minimal"))]
            notification_config: NotificationConfig::minimal(),
            batch_config: BatchConfig::minimal(),
        })
    }

    /// Создать CPU-only конфигурацию
    pub fn create_cpu_only() -> Result<Self> {
        let mut config = Self::create_production()?;

        // Отключаем GPU acceleration
        config.ai_config.embedding.use_gpu = false;
        config.ai_config.reranking.use_gpu = false;

        Ok(config)
    }

    /// Создать конфигурацию по умолчанию (backward compatibility)
    pub fn create_default() -> Result<Self> {
        Self::create_production()
    }

    /// Валидация конфигурации
    pub fn validate(&self) -> Result<()> {
        // Проверяем пути
        if self.db_path.to_str().is_none() {
            return Err(anyhow::anyhow!("Некорректный путь к базе данных"));
        }

        if self.cache_path.to_str().is_none() {
            return Err(anyhow::anyhow!("Некорректный путь к кешу"));
        }

        // Проверяем AI конфигурацию
        if self.ai_config.embedding.model_name.is_empty() {
            return Err(anyhow::anyhow!("Не указано имя embedding модели"));
        }

        // Проверяем batch конфигурацию
        if self.batch_config.max_batch_size == 0 {
            return Err(anyhow::anyhow!("Размер batch не может быть 0"));
        }

        Ok(())
    }

    /// Создать директории если не существуют
    pub fn ensure_directories(&self) -> Result<()> {
        // Создаем директорию для базы данных
        if let Some(db_parent) = self.db_path.parent() {
            std::fs::create_dir_all(db_parent)?;
        }

        // Создаем директорию для кеша
        std::fs::create_dir_all(&self.cache_path)?;

        Ok(())
    }

    /// Получить human-readable описание конфигурации
    pub fn get_description(&self) -> String {
        format!(
            "MemoryServiceConfig: db={}, cache={}, health={}, gpu={}",
            self.db_path.display(),
            self.cache_path.display(),
            self.health_enabled,
            self.ai_config.embedding.use_gpu
        )
    }
}

/// Builder pattern для создания кастомных конфигураций (Open/Closed)
pub struct MemoryServiceConfigBuilder {
    config: MemoryServiceConfig,
}

impl MemoryServiceConfigBuilder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: MemoryServiceConfig::create_minimal()?,
        })
    }

    pub fn with_db_path(mut self, path: PathBuf) -> Self {
        self.config.db_path = path;
        self
    }

    pub fn with_cache_path(mut self, path: PathBuf) -> Self {
        self.config.cache_path = path;
        self
    }

    pub fn with_health_enabled(mut self, enabled: bool) -> Self {
        self.config.health_enabled = enabled;
        self
    }

    pub fn with_ai_config(mut self, ai_config: AiConfig) -> Self {
        self.config.ai_config = ai_config;
        self
    }

    pub fn production_ready(mut self) -> Self {
        self.config.health_enabled = true;
        #[cfg(all(not(feature = "minimal"), feature = "persistence"))]
        {
            self.config.ml_promotion = Some(MLPromotionConfig::production());
        }
        self.config.streaming_config = Some(StreamingConfig::production());
        self
    }

    pub fn build(self) -> Result<MemoryServiceConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Default factory implementation (Dependency Inversion)
pub struct DefaultServiceConfigFactory;

impl ServiceConfigFactory for DefaultServiceConfigFactory {
    fn create_config(&self, config_type: ServiceConfigType) -> Result<MemoryServiceConfig> {
        match config_type {
            ServiceConfigType::Production => MemoryServiceConfig::create_production(),
            ServiceConfigType::Minimal => MemoryServiceConfig::create_minimal(),
            ServiceConfigType::CpuOnly => MemoryServiceConfig::create_cpu_only(),
        }
    }
}

/// Backward compatibility функция
pub fn default_config() -> Result<MemoryServiceConfig> {
    MemoryServiceConfig::create_default()
}

/// Re-export для обратной совместимости
pub type MemoryConfig = MemoryServiceConfig;

#[cfg(all(test, feature = "extended-tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() -> Result<()> {
        let config = MemoryServiceConfig::create_minimal()?;
        assert!(!config.health_enabled);
        assert!(config.ml_promotion.is_none());

        let prod_config = MemoryServiceConfig::create_production()?;
        assert!(prod_config.health_enabled);
        assert!(prod_config.ml_promotion.is_some());

        Ok(())
    }

    #[test]
    fn test_config_validation() -> Result<()> {
        let config = MemoryServiceConfig::create_minimal()?;
        config.validate()?;

        Ok(())
    }

    #[test]
    fn test_builder_pattern() -> Result<()> {
        let config = MemoryServiceConfigBuilder::new()?
            .with_health_enabled(true)
            .production_ready()
            .build()?;

        assert!(config.health_enabled);
        assert!(config.ml_promotion.is_some());

        Ok(())
    }

    #[test]
    fn test_config_factory() -> Result<()> {
        let factory = DefaultServiceConfigFactory;

        let min_config = factory.create_config(ServiceConfigType::Minimal)?;
        assert!(!min_config.health_enabled);

        let prod_config = factory.create_config(ServiceConfigType::Production)?;
        assert!(prod_config.health_enabled);

        Ok(())
    }
}
