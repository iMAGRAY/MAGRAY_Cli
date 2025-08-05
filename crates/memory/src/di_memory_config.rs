use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug};

#[allow(unused_imports)]
use crate::{
    di_container::{DIContainer, DIContainerBuilder},
    EmbeddingCache,
    CacheConfig,
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
// @component: {"k":"C","id":"memory_di_config","t":"DI configuration for memory system","m":{"cur":60,"tgt":100,"u":"%"},"f":["di","config","memory"]}
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
            .configure_monitoring_layer(&config).await?  // Мониторинг до processing layer
            .configure_processing_layer(&config).await?
            .configure_backup_layer(&config).await?
            .build()?;

        // Создаем async компоненты после базового контейнера
        Self::configure_async_components(&container, &config).await?;

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

    /// Настроить async компоненты после базового контейнера
    pub async fn configure_async_components(container: &DIContainer, config: &MemoryConfig) -> Result<()> {
        info!("🔧 Настройка async компонентов");
        
        // Создаем PromotionEngine
        let store = container.resolve::<VectorStore>()
            .map_err(|e| anyhow::anyhow!("Failed to resolve VectorStore for PromotionEngine: {}", e))?;
        let promotion_config = PromotionConfig::default();
        // PromotionEngine требует db: Arc<Db>, создаем временную базу для тестов
        let temp_db = Arc::new(sled::open(std::env::temp_dir().join("promotion_db"))
            .map_err(|e| anyhow::anyhow!("Failed to create temp db: {}", e))?);
        
        info!("Создание PromotionEngine");
        let promotion_engine = PromotionEngine::new(store, promotion_config, temp_db).await
            .map_err(|e| anyhow::anyhow!("Failed to create PromotionEngine: {}", e))?;
        
        container.register_instance(promotion_engine)?;

        // Создаем MLPromotionEngine
        let ml_config = MLPromotionConfig::default();
        let store_for_ml = container.resolve::<VectorStore>()
            .map_err(|e| anyhow::anyhow!("Failed to resolve VectorStore for MLPromotionEngine: {}", e))?;
        
        info!("Создание MLPromotionEngine");
        let ml_engine = MLPromotionEngine::new(store_for_ml, ml_config).await
            .map_err(|e| anyhow::anyhow!("Failed to create MLPromotionEngine: {}", e))?;
        
        container.register_instance(ml_engine)?;

        // GPU Processor (опционально)
        if config.ai_config.embedding.use_gpu {
            let gpu_config = BatchProcessorConfig::default();
            let embedding_config = container.resolve::<EmbeddingConfig>()
                .map_err(|e| anyhow::anyhow!("Failed to resolve EmbeddingConfig: {}", e))?;
            let cache = container.resolve::<Arc<dyn EmbeddingCacheInterface>>()
                .map_err(|e| anyhow::anyhow!("Failed to resolve cache for GPU: {}", e))?;
            
            info!("Создание GpuBatchProcessor");
            let processor = GpuBatchProcessor::new(gpu_config, (*embedding_config).clone(), (*cache).clone()).await
                .map_err(|e| anyhow::anyhow!("Failed to create GpuBatchProcessor: {}", e))?;
            
            container.register_instance(processor)?;
        }

        info!("✅ Async компоненты настроены");
        Ok(())
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
                let ai_config = container.resolve::<AiConfig>()?;
                Ok(Arc::new(ModelLoader::new(&ai_config.models_dir)))
            })?;

        debug!("✓ Core dependencies настроены");
        Ok(final_self)
    }

    /// Слой хранения данных (VectorStore)
    async fn configure_storage_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка storage layer");

        let db_path = config.db_path.clone();
        
        // Создаем VectorStore заранее в async контексте
        info!("Создание VectorStore по пути: {:?}", db_path);
        let store = VectorStore::new(&db_path).await
            .map_err(|e| anyhow::anyhow!("Failed to create VectorStore: {}", e))?;
        
        let final_self = self
            .register_instance(store)?;

        debug!("✓ Storage layer настроен");
        Ok(final_self)
    }

    /// Слой кэширования (EmbeddingCache)
    async fn configure_cache_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка cache layer");

        let cache_config_clone = config.cache_config.clone();
        let cache_path = config.cache_path.clone();
        
        let final_self = self.register_singleton(move |_container| {
            info!("Настройка cache layer");
            // Только LRU cache (унифицировано)
            info!("Создание LRU cache: max_entries={}, ttl={}s", 
                  cache_config_clone.max_entries, 
                  cache_config_clone.ttl_seconds.unwrap_or(3600));
            let cache_interface: Arc<dyn EmbeddingCacheInterface> = Arc::new(
                EmbeddingCache::new(&cache_path, cache_config_clone.clone())?
            );
            Ok(cache_interface)
        })?;

        debug!("✓ Cache layer настроен");
        Ok(final_self)
    }

    /// Слой обработки (GPU, Batch, Promotion)
    async fn configure_processing_layer(self, config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка processing layer");

        let mut builder = self;

        // Batch Manager
        builder = builder
            .register_singleton(|container| {
                info!("Создание BatchOperationManager");
                let store = container.resolve::<VectorStore>()?;
                let batch_config = BatchConfig::default();
                let metrics = container.try_resolve::<Arc<MetricsCollector>>()
                    .map(|arc_arc| arc_arc.as_ref().clone());
                let manager = BatchOperationManager::new(store, batch_config, metrics);
                Ok(Arc::new(manager))
            })?;

        // GPU Processor временно отключен - требует complex async setup
        if config.ai_config.embedding.use_gpu {
            debug!("GPU processor временно отключен для упрощения");
        }

        debug!("✓ Processing layer настроен");
        Ok(builder)
    }

    /// Слой мониторинга (HealthMonitor, Metrics)
    async fn configure_monitoring_layer(self, _config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка monitoring layer");

        let mut builder = self;

        // Health Monitor
        builder = builder
            .register_singleton(|_container| {
                info!("Создание HealthMonitor");
                let health_config = HealthConfig::default();
                let monitor = HealthMonitor::new(health_config);
                Ok(Arc::new(monitor))
            })?;

        // Metrics Collector
        builder = builder
            .register_singleton(|_container| {
                info!("Создание MetricsCollector");
                let collector = MetricsCollector::new();
                Ok(Arc::new(collector))
            })?;

        // Notification Manager
        builder = builder
            .register_singleton(|_container| {
                info!("Создание NotificationManager");
                let notification_config = crate::notifications::NotificationConfig::default();
                let manager = NotificationManager::new(notification_config)?;
                Ok(Arc::new(manager))
            })?;

        debug!("✓ Monitoring layer настроен");
        Ok(builder)
    }

    /// Слой резервного копирования
    async fn configure_backup_layer(self, _config: &MemoryConfig) -> Result<Self> {
        debug!("Настройка backup layer");

        let final_self = self
            .register_singleton(|_container| {
                info!("Создание BackupManager");
                let manager = BackupManager::new(std::path::Path::new("./backups"));
                Ok(Arc::new(manager))
            })?;

        debug!("✓ Backup layer настроен");
        Ok(final_self)
    }
}

/// Test helpers для создания конфигураций
#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use ai::{AiConfig, EmbeddingConfig as AiEmbeddingConfig, RerankingConfig};

    pub fn create_test_config() -> Result<MemoryConfig> {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("test_magray_db");
        let cache_path = temp_dir.join("test_cache");
        
        // Создаем директории если не существуют
        std::fs::create_dir_all(&db_path)?;
        std::fs::create_dir_all(&cache_path)?;
        
        let ai_config = AiConfig {
            models_dir: temp_dir.join("models"),
            embedding: AiEmbeddingConfig {
                model_name: "test-model".to_string(),
                batch_size: 32,
                max_length: 512,
                embedding_dim: Some(1024),
                use_gpu: false,
                gpu_config: None,
            },
            reranking: RerankingConfig {
                model_name: "test-reranker".to_string(),
                batch_size: 16,
                max_length: 512,
                use_gpu: false,
                gpu_config: None,
            },
        };
        
        let cache_config = CacheConfig::default();
        
        Ok(MemoryConfig {
            db_path,
            cache_path,
            promotion: PromotionConfig::default(),
            ml_promotion: None,
            streaming_config: None,
            ai_config,
            cache_config,
            health_enabled: true,
            health_config: HealthConfig::default(),
            resource_config: ResourceConfig::default(),
            notification_config: crate::notifications::NotificationConfig::default(),
            batch_config: BatchConfig::default(),
        })
    }
}
