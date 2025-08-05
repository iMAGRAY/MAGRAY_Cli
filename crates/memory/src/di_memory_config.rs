use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug};

#[allow(unused_imports)]
use crate::{
    di_container::{DIContainer, DIContainerBuilder},
    cache::EmbeddingCache,
    cache_lru::EmbeddingCacheLRU,
    cache_interface::EmbeddingCacheInterface,
    health::{HealthMonitor, HealthMonitorConfig as HealthConfig},
    metrics::MetricsCollector,
    notifications::NotificationManager,
    promotion::PromotionEngine,
    ml_promotion::{MLPromotionEngine, MLPromotionConfig},
    storage::VectorStore,
    types::PromotionConfig,
    gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig},
    backup::BackupManager,
    resource_manager::{ResourceManager, ResourceConfig},
    batch_manager::{BatchOperationManager, BatchConfig},
    CacheConfigType, MemoryConfig,
};
use ai::{AiConfig, EmbeddingConfig, ModelLoader};

/// Конфигуратор DI для memory системы
// @component: {"k":"C","id":"memory_di_configurator","t":"DI configuration for memory system","m":{"cur":0,"tgt":90,"u":"%"},"f":["di","config","memory"]}
pub struct MemoryDIConfigurator;

impl MemoryDIConfigurator {
    /// Настроить полный DI контейнер для memory системы
    pub async fn configure_full(config: MemoryConfig) -> Result<DIContainer> {
        info!("🔧 Настройка полного DI контейнера для memory системы");

        let builder = DIContainerBuilder::new();

        let container = builder
            .configure_core_dependencies(&config).await?
            .configure_storage_layer(&config).await?
            .configure_cache_layer(&config).await?
            .configure_processing_layer(&config).await?
            .configure_monitoring_layer(&config).await?
            .configure_backup_layer(&config).await?
            .build()?;

        info!("✅ DI контейнер настроен с {} зависимостями", 
              container.stats().total_types);

        Ok(container)
    }

    /// Настроить минимальный контейнер для тестов
    pub async fn configure_minimal(config: MemoryConfig) -> Result<DIContainer> {
        info!("🔧 Настройка минимального DI контейнера");

        let container = DIContainerBuilder::new()
            .configure_core_dependencies(&config).await?
            .configure_storage_layer(&config).await?
            .configure_cache_layer(&config).await?
            .build()?;

        info!("✅ Минимальный DI контейнер настроен");
        Ok(container)
    }

    /// Настроить только CPU зависимости (без GPU)
    pub async fn configure_cpu_only(config: MemoryConfig) -> Result<DIContainer> {
        info!("🔧 Настройка CPU-only DI контейнера");

        let mut cpu_config = config;
        cpu_config.ai_config.embedding.use_gpu = false;
        cpu_config.ai_config.reranking.use_gpu = false;

        let container = DIContainerBuilder::new()
            .configure_core_dependencies(&cpu_config).await?
            .configure_storage_layer(&cpu_config).await?
            .configure_cache_layer(&cpu_config).await?
            .configure_monitoring_layer(&cpu_config).await?
            .build()?;

        info!("✅ CPU-only DI контейнер настроен");
        Ok(container)
    }
}

/// Extension trait для удобной настройки
trait MemoryDIExtensions {
    async fn configure_core_dependencies(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
    
    async fn configure_storage_layer(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
    
    async fn configure_cache_layer(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
    
    async fn configure_processing_layer(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
    
    async fn configure_monitoring_layer(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
    
    async fn configure_backup_layer(self, config: &MemoryConfig) -> Result<Self>
    where 
        Self: Sized;
}

impl MemoryDIExtensions for DIContainerBuilder {
    /// Основные зависимости (конфигурация, AI сервисы)
    async fn configure_core_dependencies(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка core dependencies");

        let config_clone = (*config).clone();
        let self_with_config = self
            .register_instance(config_clone)?;

        // AI конфигурация
        let ai_config = config.ai_config.clone();
        let self_with_ai = self_with_config
            .register_instance(ai_config)?;

        // Embedding конфигурация из AI config  
        let embedding_config = EmbeddingConfig {
            model_name: config.ai_config.embedding.model_name.clone(),
            batch_size: 32,
            max_length: 512,
            use_gpu: config.ai_config.embedding.use_gpu,
            gpu_config: None,
            embedding_dim: Some(1024), // Qwen3 standard dimension
        };

        let self_with_embedding = self_with_ai
            .register_instance(embedding_config)?;

        // Model Loader (singleton для переиспользования)
        let final_self = self_with_embedding
            .register_singleton(|container| {
                let ai_config = container.resolve::<Arc<AiConfig>>()?;
                Ok(ModelLoader::new(&ai_config.models_dir))
            })?;

        debug!("✓ Core dependencies настроены");
        Ok(final_self)
    }

    /// Слой векторного хранилища
    async fn configure_storage_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка storage layer");

        // Создаем VectorStore заранее в async контексте
        let store = Arc::new(VectorStore::new(&config.db_path).await?);
        
        let self_with_storage = self
            .register_instance(store)?;

        debug!("✓ Storage layer настроен");
        Ok(self_with_storage)
    }

    /// Слой кэширования
    async fn configure_cache_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка cache layer");

        let cache_path = config.cache_path.clone();
        let cache_config_type = config.cache_config.clone();

        let self_with_cache = self
            .register_singleton(move |_container| {
                debug!("Создание cache по пути: {:?}", cache_path);
                
                let cache: Arc<dyn EmbeddingCacheInterface> = match &cache_config_type {
                    CacheConfigType::Simple => {
                        let simple_cache = EmbeddingCache::new(&cache_path)?;
                        Arc::new(simple_cache) as Arc<dyn EmbeddingCacheInterface>
                    },
                    CacheConfigType::Lru(lru_config) => {
                        let lru_cache = EmbeddingCacheLRU::new(&cache_path, lru_config.clone())?;
                        Arc::new(lru_cache) as Arc<dyn EmbeddingCacheInterface>
                    }
                };
                
                Ok(cache)
            })?;

        debug!("✓ Cache layer настроен");
        Ok(self_with_cache)
    }

    /// Слой обработки (GPU, batching, promotion)
    async fn configure_processing_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка processing layer");

        let batch_config = BatchProcessorConfig::default();
        let promotion_config = config.promotion.clone();
        let ml_promotion_config = config.ml_promotion.clone();

        // GPU Batch Processor
        let self_with_gpu = self
            .register_singleton({
                let batch_config_clone = batch_config.clone();
                move |container| {
                let embedding_config = container.resolve::<Arc<EmbeddingConfig>>()?;
                let cache = container.resolve::<Arc<dyn EmbeddingCacheInterface>>()?;
                
                let batch_processor_config = BatchProcessorConfig {
                    max_batch_size: batch_config_clone.max_batch_size,
                    batch_timeout_ms: batch_config_clone.batch_timeout_ms,
                    use_gpu_if_available: embedding_config.use_gpu,
                    cache_embeddings: true,
                };
                
                let processor = match tokio::runtime::Handle::try_current() {
                    Ok(rt) => {
                        rt.block_on(async {
                            GpuBatchProcessor::new(
                                batch_processor_config,
                                (**embedding_config).clone(),
                                (*cache).clone(),
                            ).await
                        })?
                    }
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new()
                            .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
                        rt.block_on(async {
                            GpuBatchProcessor::new(
                                batch_processor_config,
                                (**embedding_config).clone(),
                                (*cache).clone(),
                            ).await
                        })?
                    }
                };
                
                Ok(processor)
                }
            })?;

        // Batch Operation Manager
        let self_with_batch = self_with_gpu
            .register_singleton(move |container| {
                let store = container.resolve::<Arc<VectorStore>>()?;
                let metrics = container.try_resolve::<Arc<MetricsCollector>>()
                    .map(|m| (*m).clone());
                
                let batch_manager_config = BatchConfig::default();
                Ok(BatchOperationManager::new((*store).clone(), batch_manager_config, metrics))
            })?;

        // Promotion Engine
        let promotion_config_clone = promotion_config.clone();
        let db_path_clone = config.db_path.clone();
        let self_with_promotion = self_with_batch
            .register_singleton(move |container| {
                let store = container.resolve::<Arc<VectorStore>>()?;
                
                // Создаем БД для promotion engine
                let promotion_db_path = db_path_clone.with_extension("promotion.db");
                let db = Arc::new(sled::open(&promotion_db_path)
                    .map_err(|e| anyhow::anyhow!("Failed to open promotion DB: {}", e))?);
                
                match tokio::runtime::Handle::try_current() {
                    Ok(rt) => {
                        let engine = rt.block_on(async {
                            PromotionEngine::new((*store).clone(), promotion_config_clone.clone(), db).await
                        })?;
                        Ok(engine)
                    }
                    Err(_) => {
                        let rt = tokio::runtime::Runtime::new()
                            .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
                        let engine = rt.block_on(async {
                            PromotionEngine::new((*store).clone(), promotion_config_clone.clone(), db).await
                        })?;
                        Ok(engine)
                    }
                }
            })?;

        // ML Promotion Engine (опциональный)
        let self_with_ml_promotion = if let Some(ml_config) = ml_promotion_config {
            let ml_config_clone = ml_config.clone();
            self_with_promotion
                .register_singleton(move |container| {
                    let store = container.resolve::<Arc<VectorStore>>()?;
                    
                    match tokio::runtime::Handle::try_current() {
                        Ok(rt) => {
                            let engine = rt.block_on(async {
                                MLPromotionEngine::new((*store).clone(), ml_config_clone.clone()).await
                            })?;
                            Ok(parking_lot::RwLock::new(engine))
                        }
                        Err(_) => {
                            let rt = tokio::runtime::Runtime::new()
                                .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;
                            let engine = rt.block_on(async {
                                MLPromotionEngine::new((*store).clone(), ml_config_clone.clone()).await
                            })?;
                            Ok(parking_lot::RwLock::new(engine))
                        }
                    }
                })?
        } else {
            self_with_promotion
        };

        debug!("✓ Processing layer настроен");
        Ok(self_with_ml_promotion)
    }

    /// Слой мониторинга (health, metrics, notifications)
    async fn configure_monitoring_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка monitoring layer");

        let health_config = config.health_config.clone();
        let notification_config = crate::notifications::NotificationConfig::default();

        // Metrics Collector
        let self_with_metrics = self
            .register_singleton(|_container| {
                Ok(MetricsCollector::new())
            })?;

        // Health Monitor
        let self_with_health = self_with_metrics
            .register_singleton({
                let health_config_clone = health_config.clone();
                move |_container| {
                    Ok(Arc::new(HealthMonitor::new(health_config_clone.clone())))
                }
            })?;

        // Notification Manager (опциональный)
        let self_with_notifications = self_with_health
            .register_singleton({
                let notification_config_clone = notification_config.clone();
                move |_container| {
                    Ok(NotificationManager::new(notification_config_clone.clone()))
                }
            })?;

        debug!("✓ Monitoring layer настроен");
        Ok(self_with_notifications)
    }

    /// Слой резервного копирования
    async fn configure_backup_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка backup layer");

        let db_path = config.db_path.clone();
        let self_with_backup = self
            .register_singleton({
                let db_path_clone = db_path.clone();
                move |_container| {
                    let backup_path = db_path_clone.with_extension("backup");
                    Ok(Arc::new(BackupManager::new(backup_path)?))
                }
            })?;

        // Resource Manager
        let resource_config = ResourceConfig::default();
        let self_with_resource = self_with_backup
            .register_singleton({
                let resource_config_clone = resource_config.clone();
                move |_container| {
                    Ok(parking_lot::RwLock::new(ResourceManager::new(resource_config_clone.clone())))
                }
            })?;

        debug!("✓ Backup layer настроен");
        Ok(self_with_resource)
    }
}

/// Вспомогательные функции для тестирования
#[cfg(test)]
#[allow(unused_imports)]
pub mod test_helpers {
    use super::*;
    use tempfile::TempDir;

    pub fn create_test_config() -> Result<MemoryConfig> {
        let _temp_dir = TempDir::new()?;
        let base_path = _temp_dir.path().to_path_buf();

        Ok(MemoryConfig {
            db_path: base_path.join("test.db"),
            cache_path: base_path.join("cache"),
            promotion: PromotionConfig::default(),
            ml_promotion: None, // Отключаем для тестов
            streaming_config: None,
            ai_config: AiConfig {
                models_dir: base_path.join("models"),
                embedding: ai::EmbeddingConfig {
                    model_name: "test-model".to_string(),
                    use_gpu: false, // CPU-only для тестов
                    batch_size: 16,
                    max_length: 512,
                    gpu_config: None,
                    embedding_dim: Some(1024),
                },
                reranking: ai::RerankingConfig {
                    model_name: "test-reranker".to_string(),
                    use_gpu: false,
                    batch_size: 8,
                    max_length: 512,
                    gpu_config: None,
                },
            },
            health_config: HealthConfig::default(),
            notification_config: crate::notifications::NotificationConfig::default(),
            cache_config: CacheConfigType::Simple,
            batch_config: BatchConfig {
                max_batch_size: 10, // Маленький batch для тестов
                ..Default::default()
            },
            resource_config: ResourceConfig::default(),
            // Legacy поля
            #[allow(deprecated)]
            max_vectors: 1000,
            #[allow(deprecated)]
            max_cache_size_bytes: 1024 * 1024,
            #[allow(deprecated)]
            max_memory_usage_percent: Some(50),
        })
    }

    pub async fn create_test_container() -> Result<DIContainer> {
        let config = create_test_config()?;
        MemoryDIConfigurator::configure_minimal(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_minimal_di_configuration() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let container = MemoryDIConfigurator::configure_minimal(config).await?;

        // Проверяем основные зависимости
        assert!(container.is_registered::<MemoryConfig>());
        assert!(container.is_registered::<Arc<VectorStore>>());
        assert!(container.is_registered::<Arc<dyn EmbeddingCacheInterface>>());

        let stats = container.stats();
        assert!(stats.total_types >= 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_cpu_only_configuration() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let container = MemoryDIConfigurator::configure_cpu_only(config).await?;

        // Должен содержать все CPU компоненты
        assert!(container.is_registered::<Arc<VectorStore>>());
        assert!(container.is_registered::<Arc<dyn EmbeddingCacheInterface>>());
        assert!(container.is_registered::<Arc<HealthMonitor>>());

        Ok(())
    }

    #[tokio::test]
    async fn test_dependency_resolution() -> Result<()> {
        let container = test_helpers::create_test_container().await?;

        // Тестируем разрешение зависимостей
        let store = container.resolve::<Arc<VectorStore>>()?;
        assert!(!(store.as_ref() as *const _ == std::ptr::null()));

        let cache = container.resolve::<Arc<dyn EmbeddingCacheInterface>>()?;
        assert!(!cache.as_ref().is_null_check());

        Ok(())
    }
}