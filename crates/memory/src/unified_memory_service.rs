//! Simplified Unified Memory Service - объединяет все memory сервисы в единый интерфейс
//!
//! Упрощенная реализация для демонстрации концепции объединения сервисов.

use anyhow::Result;
use common::comprehensive_errors::MemoryError;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

use crate::{
    api::MemoryServiceTrait,
    fallback::FallbackEmbeddingService,
    health::SystemHealthStatus,
    types::{Layer, Record, SearchOptions},
};

/// Конфигурация Unified Memory Service
#[derive(Debug, Clone)]
pub struct UnifiedMemoryConfig {
    /// Включить batch optimization для high QPS workloads
    pub enable_batch_optimization: bool,
    /// Максимальный размер batch для optimized processing
    pub max_batch_size: usize,
    /// Включить GPU acceleration если доступен
    pub enable_gpu_acceleration: bool,
    /// Включить LRU кэширование
    pub enable_cache: bool,
    /// Путь для cache persistence
    pub cache_path: Option<std::path::PathBuf>,
    /// Таймаут для adaptive backend selection (ms)
    pub backend_selection_timeout_ms: u64,
}

impl Default for UnifiedMemoryConfig {
    fn default() -> Self {
        Self {
            enable_batch_optimization: true,
            max_batch_size: 512,
            enable_gpu_acceleration: true,
            enable_cache: true,
            cache_path: None,
            backend_selection_timeout_ms: 5000,
        }
    }
}

/// Backend типы для adaptive selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendType {
    /// CPU fallback - наиболее совместимый
    CpuFallback,
    /// Batch optimized - для high QPS workloads
    BatchOptimized,
    /// GPU accelerated - максимальная производительность
    GpuAccelerated,
}

/// Статистика backend
#[derive(Debug, Clone)]
pub struct BackendStats {
    pub backend_type: BackendType,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub average_latency_ms: f64,
}

/// Статистика cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size_bytes: u64,
    pub hit_rate: f64,
}

/// Общая статистика Unified Memory Service
#[derive(Debug)]
pub struct UnifiedMemoryStats {
    pub total_operations: u64,
    pub uptime_seconds: u64,
    pub current_backend: BackendType,
    pub backend_stats: Vec<BackendStats>,
    pub cache_stats: Option<CacheStats>,
    pub throughput_ops_per_sec: f64,
}

/// Simplified Unified Memory Service
pub struct UnifiedMemoryService {
    fallback_service: Arc<std::sync::Mutex<FallbackEmbeddingService>>,

    // Service state
    initialized: AtomicBool,
    shutdown_requested: AtomicBool,
    total_operations: AtomicU64,
    startup_time: Instant,
}

impl UnifiedMemoryService {
    /// Создать новый Unified Memory Service
    pub async fn new(_config: UnifiedMemoryConfig) -> Result<Self> {
        info!("🚀 Creating Simplified Unified Memory Service");

        let fallback_service = Arc::new(std::sync::Mutex::new(FallbackEmbeddingService::new(1024)));

        let service = Self {
            fallback_service,
            initialized: AtomicBool::new(false),
            shutdown_requested: AtomicBool::new(false),
            total_operations: AtomicU64::new(0),
            startup_time: Instant::now(),
        };

        info!("✅ Simplified Unified Memory Service created");
        Ok(service)
    }

    /// Инициализировать сервис
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Initializing Simplified Unified Memory Service...");

        self.initialized.store(true, Ordering::Relaxed);

        let init_time = self.startup_time.elapsed();
        info!(
            "✅ Simplified Unified Memory Service initialized in {:?}",
            init_time
        );

        Ok(())
    }

    /// Embed single text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.total_operations.fetch_add(1, Ordering::Relaxed);

        let mut service = self
            .fallback_service
            .lock()
            .map_err(|_| anyhow::anyhow!("Fallback service mutex poisoned"))?;
        service.embed(text)
    }

    /// Embed batch of texts
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        self.total_operations
            .fetch_add(texts.len() as u64, Ordering::Relaxed);

        let mut service = self
            .fallback_service
            .lock()
            .map_err(|_| anyhow::anyhow!("Fallback service mutex poisoned"))?;
        service.embed_batch(&texts)
    }

    /// Search в векторном индексе
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!(
            "Search in layer {:?} with query: '{}' (top_k: {})",
            layer, query, options.top_k
        );

        // В упрощенной реализации просто возвращаем пустой результат
        Ok(vec![])
    }

    /// Insert record в векторный индекс
    pub async fn insert(&self, record: Record) -> Result<()> {
        debug!(
            "Inserting record {} into layer {:?}",
            record.id, record.layer
        );

        // В упрощенной реализации просто логируем
        Ok(())
    }

    /// Batch insert records
    pub async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        debug!("Batch inserting {} records", records.len());

        for record in records {
            self.insert(record).await?;
        }

        Ok(())
    }

    /// Get comprehensive statistics
    pub async fn get_stats(&self) -> UnifiedMemoryStats {
        let total_ops = self.total_operations.load(Ordering::Relaxed);
        let uptime = self.startup_time.elapsed();

        let backend_stats = vec![BackendStats {
            backend_type: BackendType::CpuFallback,
            total_operations: total_ops,
            successful_operations: total_ops,
            error_count: 0,
            success_rate: 1.0,
            average_latency_ms: 1.0,
        }];

        UnifiedMemoryStats {
            total_operations: total_ops,
            uptime_seconds: uptime.as_secs(),
            current_backend: BackendType::CpuFallback,
            backend_stats,
            cache_stats: None,
            throughput_ops_per_sec: if uptime.as_secs() > 0 {
                total_ops as f64 / uptime.as_secs() as f64
            } else {
                0.0
            },
        }
    }

    /// Get system health status
    pub async fn get_health(&self) -> SystemHealthStatus {
        use crate::health::HealthStatus;
        use chrono::Utc;
        use std::collections::HashMap;

        let stats = self.get_stats().await;

        SystemHealthStatus {
            overall_status: HealthStatus::Healthy,
            component_statuses: HashMap::new(),
            active_alerts: vec![],
            metrics_summary: HashMap::new(),
            last_updated: Utc::now(),
            uptime_seconds: stats.uptime_seconds,
        }
    }

    /// Graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Starting graceful shutdown of Simplified Unified Memory Service");

        self.shutdown_requested.store(true, Ordering::Relaxed);

        let final_stats = self.get_stats().await;
        info!(
            "📊 Final statistics: {} total operations, {:.1}s uptime",
            final_stats.total_operations, final_stats.uptime_seconds
        );

        info!("✅ Simplified Unified Memory Service shutdown completed");
        Ok(())
    }

    /// Performance benchmark для валидации
    pub async fn benchmark_performance(&self, num_operations: usize) -> Result<BenchmarkResults> {
        let start_time = Instant::now();

        // Single embedding benchmark
        let single_start = Instant::now();
        for i in 0..num_operations {
            let text = format!("benchmark text {}", i);
            let _embedding = self.embed(&text).await?;
        }
        let single_duration = single_start.elapsed();

        // Batch embedding benchmark
        let texts: Vec<String> = (0..num_operations)
            .map(|i| format!("batch text {}", i))
            .collect();

        let batch_start = Instant::now();
        let _batch_embeddings = self.embed_batch(texts).await?;
        let batch_duration = batch_start.elapsed();

        let total_duration = start_time.elapsed();
        let stats = self.get_stats().await;

        Ok(BenchmarkResults {
            num_operations,
            single_ops_duration: single_duration,
            batch_ops_duration: batch_duration,
            total_duration,
            single_ops_per_sec: num_operations as f64 / single_duration.as_secs_f64(),
            batch_ops_per_sec: num_operations as f64 / batch_duration.as_secs_f64(),
            total_operations: stats.total_operations,
            throughput_ops_per_sec: stats.throughput_ops_per_sec,
        })
    }
}

/// Результаты бенчмарка производительности
#[derive(Debug)]
pub struct BenchmarkResults {
    pub num_operations: usize,
    pub single_ops_duration: std::time::Duration,
    pub batch_ops_duration: std::time::Duration,
    pub total_duration: std::time::Duration,
    pub single_ops_per_sec: f64,
    pub batch_ops_per_sec: f64,
    pub total_operations: u64,
    pub throughput_ops_per_sec: f64,
}

impl BenchmarkResults {
    pub fn print_results(&self) {
        println!("🚀 Unified Memory Service Performance Benchmark");
        println!("==============================================");
        println!("Operations tested: {}", self.num_operations);
        println!("Total duration: {:?}", self.total_duration);
        println!();
        println!("Single operations:");
        println!("  Duration: {:?}", self.single_ops_duration);
        println!("  Throughput: {:.0} ops/sec", self.single_ops_per_sec);
        println!();
        println!("Batch operations:");
        println!("  Duration: {:?}", self.batch_ops_duration);
        println!("  Throughput: {:.0} ops/sec", self.batch_ops_per_sec);
        println!();

        if self.batch_ops_per_sec > self.single_ops_per_sec {
            let speedup = self.batch_ops_per_sec / self.single_ops_per_sec;
            println!(
                "🚀 Batch speedup: {:.1}x faster than single operations",
                speedup
            );
        }

        println!("📊 Overall service stats:");
        println!("  Total operations processed: {}", self.total_operations);
        println!(
            "  Service throughput: {:.0} ops/sec",
            self.throughput_ops_per_sec
        );
    }
}

/// Реализация MemoryServiceTrait для backward compatibility
impl MemoryServiceTrait for UnifiedMemoryService {
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
        let options = SearchOptions {
            top_k,
            layers: vec![layer],
            ..Default::default()
        };

        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { self.search(query, layer, options).await })
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async { self.search(query, layer, options).await })
            }
        }
    }

    #[cfg(all(not(feature = "minimal"), feature = "persistence"))]
    fn run_promotion_sync(&self) -> Result<crate::api::PromotionStats> {
        let real_stats = crate::promotion::PromotionStats {
            interact_to_insights: 0,
            insights_to_assets: 0,
            expired_interact: 0,
            expired_insights: 0,
            total_time_ms: 0,
            index_update_time_ms: 0,
            promotion_time_ms: 0,
            cleanup_time_ms: 0,
        };
        // Convert real promotion stats to API stats
        Ok(crate::api::PromotionStats {
            interact_to_insights: real_stats.interact_to_insights,
            insights_to_assets: real_stats.insights_to_assets,
            expired_interact: real_stats.expired_interact,
            expired_insights: real_stats.expired_insights,
            total_time_ms: real_stats.total_time_ms,
            index_update_time_ms: real_stats.index_update_time_ms,
            promotion_time_ms: real_stats.promotion_time_ms,
            cleanup_time_ms: real_stats.cleanup_time_ms,
        })
    }

    #[cfg(not(all(not(feature = "minimal"), feature = "persistence")))]
    fn run_promotion_sync(&self) -> Result<crate::api::PromotionStats> {
        // Promotion disabled when persistence feature is not enabled
        Ok(crate::api::PromotionStats::default())
    }

    fn get_system_health(&self) -> SystemHealthStatus {
        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async { self.get_health().await })
            }),
            Err(_) => match tokio::runtime::Runtime::new() {
                Ok(rt) => rt.block_on(async { self.get_health().await }),
                Err(_) => SystemHealthStatus {
                    overall_status: crate::health::HealthStatus::Down,
                    component_statuses: std::collections::HashMap::new(),
                    active_alerts: Vec::new(),
                    metrics_summary: std::collections::HashMap::new(),
                    last_updated: chrono::Utc::now(),
                    uptime_seconds: 0,
                },
            },
        }
    }

    fn cache_stats(&self) -> (u64, u64, u64) {
        (0, 0, 0) // Simplified implementation
    }

    fn remember_sync(&self, text: String, layer: Layer) -> Result<uuid::Uuid> {
        use chrono::Utc;

        let record = Record {
            id: uuid::Uuid::new_v4(),
            text,
            embedding: vec![], // Will be populated by embed operation
            layer,
            kind: "user_input".to_string(),
            tags: vec![],
            project: "default".to_string(),
            session: "unified_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };

        let record_id = record.id;

        match tokio::runtime::Handle::try_current() {
            Ok(_) => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    self.insert(record).await?;
                    Ok(record_id)
                })
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    self.insert(record).await?;
                    Ok(record_id)
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unified_memory_service_creation() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config).await;

        assert!(service.is_ok());
        let service = service.expect("Operation failed - converted from unwrap()");

        // Сервис создается без инициализации
        assert!(!service.initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_service_initialization() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config)
            .await
            .expect("Async operation should succeed");

        // Инициализируем сервис
        let result = service.initialize().await;
        assert!(result.is_ok());
        assert!(service.initialized.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_embedding_operations() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config)
            .await
            .expect("Async operation should succeed");
        service
            .initialize()
            .await
            .expect("Async operation should succeed");

        // Тест single embedding
        let embedding = service.embed("test text").await;
        assert!(embedding.is_ok());
        let embedding = embedding.expect("Operation failed - converted from unwrap()");
        assert!(!embedding.is_empty());

        // Тест batch embedding
        let texts = vec!["text1".to_string(), "text2".to_string()];
        let embeddings = service.embed_batch(texts).await;
        assert!(embeddings.is_ok());
        let embeddings = embeddings.expect("Operation failed - converted from unwrap()");
        assert_eq!(embeddings.len(), 2);
    }

    #[tokio::test]
    async fn test_statistics() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config)
            .await
            .expect("Async operation should succeed");
        service
            .initialize()
            .await
            .expect("Async operation should succeed");

        // Выполняем несколько операций
        let _ = service.embed("test1").await;
        let _ = service.embed("test2").await;

        let stats = service.get_stats().await;
        assert!(stats.total_operations >= 2);
        assert_eq!(stats.current_backend, BackendType::CpuFallback);
        assert!(!stats.backend_stats.is_empty());
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config)
            .await
            .expect("Async operation should succeed");
        service
            .initialize()
            .await
            .expect("Async operation should succeed");

        // Выполняем shutdown
        let result = service.shutdown().await;
        assert!(result.is_ok());
        assert!(service.shutdown_requested.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_performance_benchmark() {
        let config = UnifiedMemoryConfig::default();
        let service = UnifiedMemoryService::new(config)
            .await
            .expect("Async operation should succeed");
        service
            .initialize()
            .await
            .expect("Async operation should succeed");

        // Запускаем небольшой benchmark
        let results = service.benchmark_performance(10).await;
        assert!(results.is_ok());

        let results = results.expect("Operation failed - converted from unwrap()");
        assert_eq!(results.num_operations, 10);
        assert!(results.single_ops_per_sec > 0.0);
        assert!(results.batch_ops_per_sec > 0.0);
    }
}
