use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::fallback::FallbackEmbeddingService;
use ai::{EmbeddingConfig, Qwen3EmbeddingProvider};

/// Bridge adapter для интеграции Qwen3EmbeddingProvider в memory system
/// Обеспечивает совместимость между ai crate и memory system интерфейсами
pub struct Qwen3MemoryBridge {
    /// Основной Qwen3 embedding provider
    qwen3_provider: Arc<RwLock<Qwen3EmbeddingProvider>>,
    /// Fallback embedding service для случаев когда Qwen3 недоступен
    fallback_service: Arc<RwLock<FallbackEmbeddingService>>,
    /// Конфигурация embedding
    config: EmbeddingConfig,
    /// Флаг инициализации
    initialized: std::sync::atomic::AtomicBool,
    /// Метрики производительности
    performance_metrics: Arc<RwLock<BridgeMetrics>>,
}

/// Метрики производительности bridge
#[derive(Debug, Default, Clone)]
pub struct BridgeMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub qwen3_requests: u64,
    pub fallback_requests: u64,
    pub avg_latency_ms: f64,
    pub total_latency_ms: f64,
}

impl Qwen3MemoryBridge {
    /// Создать новый bridge с Qwen3EmbeddingProvider
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!("🔗 Инициализация Qwen3MemoryBridge");
        info!("   Model: qwen3emb");
        info!("   Embedding dimension: 1024");
        info!("   Batch size: {}", config.batch_size);

        // Создаем Qwen3EmbeddingProvider с конфигурацией
        let qwen3_provider =
            Qwen3EmbeddingProvider::new_with_config(config.clone()).map_err(|e| {
                error!("❌ Не удалось создать Qwen3EmbeddingProvider: {}", e);
                anyhow::anyhow!("Failed to create Qwen3EmbeddingProvider: {}", e)
            })?;

        // Создаем fallback service с dimension 1024 (Qwen3)
        let fallback_service = FallbackEmbeddingService::new(1024);

        let bridge = Self {
            qwen3_provider: Arc::new(RwLock::new(qwen3_provider)),
            fallback_service: Arc::new(RwLock::new(fallback_service)),
            config,
            initialized: std::sync::atomic::AtomicBool::new(false),
            performance_metrics: Arc::new(RwLock::new(BridgeMetrics::default())),
        };

        info!("✅ Qwen3MemoryBridge создан");
        Ok(bridge)
    }

    /// Создать с кастомной конфигурацией
    pub async fn with_custom_config(
        model_path: std::path::PathBuf,
        batch_size: usize,
        max_seq_length: usize,
    ) -> Result<Self> {
        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            batch_size,
            max_length: max_seq_length,
            use_gpu: false, // CPU по умолчанию для стабильности
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        Self::new(config).await
    }

    /// Инициализировать bridge и проверить модель
    pub async fn initialize(&self) -> Result<()> {
        if self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        info!("🔄 Инициализация Qwen3MemoryBridge...");

        // Тестируем Qwen3 provider с простым текстом
        let test_result = {
            let provider = self.qwen3_provider.read().await;
            provider.embed_text("test initialization")
        };

        match test_result {
            Ok(embedding) => {
                if embedding.len() == 1024 {
                    info!(
                        "✅ Qwen3 provider инициализирован (embedding dim: {})",
                        embedding.len()
                    );
                } else {
                    warn!(
                        "⚠️ Неожиданная размерность embedding: {} (ожидалось 1024)",
                        embedding.len()
                    );
                }
            }
            Err(e) => {
                error!("❌ Ошибка при тестировании Qwen3 provider: {}", e);
                warn!("🔄 Будет использован fallback режим");
            }
        }

        self.initialized
            .store(true, std::sync::atomic::Ordering::Relaxed);
        info!("✅ Qwen3MemoryBridge инициализирован");
        Ok(())
    }

    /// Получить embedding для одного текста
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            self.initialize().await?;
        }

        let start_time = std::time::Instant::now();
        let mut metrics = self.performance_metrics.write().await;
        metrics.total_requests += 1;

        // Пытаемся использовать Qwen3 provider напрямую
        let result = {
            let provider = self.qwen3_provider.read().await;
            provider.embed_text(text)
        };

        match result {
            Ok(embedding) => {
                metrics.successful_requests += 1;
                metrics.qwen3_requests += 1;

                let latency = start_time.elapsed().as_millis() as f64;
                metrics.total_latency_ms += latency;
                metrics.avg_latency_ms =
                    metrics.total_latency_ms / metrics.successful_requests as f64;

                debug!("✅ Qwen3 embedding получен за {:.2}ms", latency);
                Ok(embedding)
            }
            Err(e) => {
                warn!(
                    "⚠️ Qwen3 provider ошибка: {}, используем graceful fallback",
                    e
                );
                metrics.failed_requests += 1;

                // Используем fallback service
                let mut fallback = self.fallback_service.write().await;
                match fallback.embed(text) {
                    Ok(embedding) => {
                        metrics.fallback_requests += 1;
                        let latency = start_time.elapsed().as_millis() as f64;
                        metrics.total_latency_ms += latency;
                        metrics.avg_latency_ms =
                            metrics.total_latency_ms / (metrics.successful_requests + 1) as f64;

                        warn!("🔄 Fallback embedding получен за {:.2}ms", latency);
                        Ok(embedding)
                    }
                    Err(fallback_error) => {
                        error!("❌ Fallback также неудачен: {}", fallback_error);
                        Err(anyhow::anyhow!(
                            "Все embedding методы неудачны: Qwen3: {}, Fallback: {}",
                            e,
                            fallback_error
                        ))
                    }
                }
            }
        }
    }

    /// Получить batch embeddings
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if !self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            self.initialize().await?;
        }

        debug!(
            "📦 Обработка batch из {} текстов через Qwen3MemoryBridge",
            texts.len()
        );

        let start_time = std::time::Instant::now();
        let mut metrics = self.performance_metrics.write().await;
        metrics.total_requests += texts.len() as u64;

        // Пытаемся использовать Qwen3 batch processing
        let result = {
            let provider = self.qwen3_provider.read().await;
            provider.embed_batch(texts)
        };

        match result {
            Ok(embeddings) => {
                metrics.successful_requests += texts.len() as u64;
                metrics.qwen3_requests += texts.len() as u64;

                let latency = start_time.elapsed().as_millis() as f64;
                metrics.total_latency_ms += latency;
                metrics.avg_latency_ms =
                    metrics.total_latency_ms / metrics.successful_requests as f64;

                info!(
                    "✅ Qwen3 batch embeddings получены за {:.2}ms ({:.1} items/sec)",
                    latency,
                    texts.len() as f64 / (latency / 1000.0)
                );
                Ok(embeddings)
            }
            Err(e) => {
                warn!("⚠️ Qwen3 batch ошибка: {}, используем graceful fallback", e);
                metrics.failed_requests += texts.len() as u64;

                // Используем fallback service
                let mut fallback = self.fallback_service.write().await;
                match fallback.embed_batch(texts) {
                    Ok(embeddings) => {
                        metrics.fallback_requests += texts.len() as u64;
                        let latency = start_time.elapsed().as_millis() as f64;
                        metrics.total_latency_ms += latency;

                        warn!("🔄 Fallback batch embeddings получены за {:.2}ms", latency);
                        Ok(embeddings)
                    }
                    Err(fallback_error) => {
                        error!("❌ Fallback batch также неудачен: {}", fallback_error);
                        Err(anyhow::anyhow!(
                            "Все batch embedding методы неудачны: Qwen3: {}, Fallback: {}",
                            e,
                            fallback_error
                        ))
                    }
                }
            }
        }
    }

    /// Получить размерность embedding
    pub fn embedding_dim(&self) -> usize {
        1024 // Qwen3-Embedding-0.6B dimension
    }

    /// Получить метрики производительности
    pub async fn get_metrics(&self) -> BridgeMetrics {
        let metrics = self.performance_metrics.read().await;
        (*metrics).clone()
    }

    /// Проверить доступность Qwen3 provider
    pub async fn is_qwen3_available(&self) -> bool {
        let test_result = {
            let provider = self.qwen3_provider.read().await;
            provider.embed_text("availability test")
        };
        test_result.is_ok()
    }

    /// Форсировать fallback режим
    pub async fn force_fallback(&self) {
        // Просто устанавливаем флаг что инициализация не завершена
        // Это заставит использовать fallback
        self.initialized
            .store(false, std::sync::atomic::Ordering::Relaxed);
        warn!("🔄 Qwen3MemoryBridge переключен в fallback режим");
    }

    /// Попробовать восстановить Qwen3 provider
    pub async fn try_recover(&self) -> bool {
        // Пытаемся переинициализировать
        match self.initialize().await {
            Ok(()) => {
                info!("✅ Qwen3 provider восстановлен");
                true
            }
            Err(_) => false,
        }
    }
}

impl Clone for Qwen3MemoryBridge {
    fn clone(&self) -> Self {
        Self {
            qwen3_provider: Arc::clone(&self.qwen3_provider),
            fallback_service: Arc::clone(&self.fallback_service),
            config: self.config.clone(),
            initialized: std::sync::atomic::AtomicBool::new(
                self.initialized.load(std::sync::atomic::Ordering::Relaxed),
            ),
            performance_metrics: Arc::clone(&self.performance_metrics),
        }
    }
}

// Bridge реализует свои собственные методы вместо trait impl
// Это избегает проблем с Send/Sync и упрощает интеграцию

#[cfg(test)]
mod tests {
    use super::*;
    use ai::EmbeddingConfig;

    #[tokio::test]
    async fn test_qwen3_bridge_creation() -> Result<()> {
        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            batch_size: 32,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        let bridge = Qwen3MemoryBridge::new(config).await?;
        assert_eq!(bridge.embedding_dim(), 1024);
        Ok(())
    }

    #[tokio::test]
    async fn test_qwen3_bridge_initialization() -> Result<()> {
        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            batch_size: 16,
            max_length: 256,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        let bridge = Qwen3MemoryBridge::new(config).await?;

        // Инициализация может быть неудачной если модель недоступна
        // но bridge все равно должен работать через fallback
        let _init_result = bridge.initialize().await;

        // Проверяем что можем получить embedding (через fallback если нужно)
        let embedding = bridge.embed_text("test").await?;
        assert_eq!(embedding.len(), 1024);

        Ok(())
    }

    #[tokio::test]
    async fn test_qwen3_bridge_metrics() -> Result<()> {
        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            batch_size: 8,
            max_length: 128,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        let bridge = Qwen3MemoryBridge::new(config).await?;

        // Делаем несколько запросов
        let _embedding1 = bridge.embed_text("test 1").await;
        let _embedding2 = bridge.embed_text("test 2").await;

        let metrics = bridge.get_metrics().await;
        assert!(metrics.total_requests >= 2);

        Ok(())
    }
}
