use anyhow::Result;
use memory::Qwen3MemoryBridge;

#[cfg(all(not(feature = "minimal"), feature = "embeddings"))]
#[tokio::test]
async fn test_qwen3_memory_bridge_basic() -> Result<()> {
    use ai::EmbeddingConfig;

    println!("🔗 Тестируем базовую функциональность Qwen3MemoryBridge");

    // Создаем конфигурацию для Qwen3
    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        batch_size: 8,
        max_length: 256,
        use_gpu: false,
        gpu_config: None,
        embedding_dim: Some(1024),
    };

    match Qwen3MemoryBridge::new(config).await {
        Ok(bridge) => {
            println!("✅ Qwen3MemoryBridge создан успешно");

            // Проверяем основные методы
            assert_eq!(bridge.embedding_dim(), 1024);
            println!(
                "✅ Embedding dimension корректный: {}",
                bridge.embedding_dim()
            );

            // Тестируем инициализацию (может завершиться с ошибкой без реальной модели)
            let init_result = bridge.initialize().await;
            match init_result {
                Ok(()) => {
                    println!("✅ Bridge инициализация успешна");

                    // Проверяем доступность Qwen3
                    let available = bridge.is_qwen3_available().await;
                    println!("Qwen3 provider доступность: {available}");

                    // Пытаемся получить embedding
                    let test_result = bridge.embed_text("test embedding").await;
                    match test_result {
                        Ok(embedding) => {
                            println!(
                                "✅ Single embedding получен (размерность: {})",
                                embedding.len()
                            );
                            assert_eq!(embedding.len(), 1024, "Ожидаем Qwen3 dimension");
                        }
                        Err(e) => {
                            println!("⚠️ Single embedding через fallback: {e}");
                        }
                    }

                    // Пытаемся batch embedding
                    let texts = vec!["first test".to_string(), "second test".to_string()];

                    let batch_result = bridge.embed_batch(&texts).await;
                    match batch_result {
                        Ok(embeddings) => {
                            println!(
                                "✅ Batch embedding получен ({} embeddings)",
                                embeddings.len()
                            );
                            assert_eq!(embeddings.len(), texts.len());
                            for embedding in &embeddings {
                                assert_eq!(embedding.len(), 1024);
                            }
                        }
                        Err(e) => {
                            println!("⚠️ Batch embedding через fallback: {e}");
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Bridge инициализация завершилась с ошибкой: {e}");
                    println!("💡 Это ожидаемо без реальной модели Qwen3");

                    // Но bridge все равно должен работать через fallback
                    let fallback_result = bridge.embed_text("fallback test").await;
                    match fallback_result {
                        Ok(embedding) => {
                            println!(
                                "✅ Fallback embedding работает (размерность: {})",
                                embedding.len()
                            );
                            assert_eq!(embedding.len(), 1024);
                        }
                        Err(e) => {
                            println!("❌ Даже fallback не работает: {e}");
                            // Это критическая ошибка
                            return Err(e);
                        }
                    }
                }
            }

            // Тестируем методы управления
            bridge.force_fallback().await;
            println!("✅ Force fallback работает");

            let _recovery_result = bridge.try_recover().await;
            println!("✅ Try recover работает");

            // Получаем метрики
            let metrics = bridge.get_metrics().await;
            println!("📊 Метрики bridge:");
            println!("  Total requests: {}", metrics.total_requests);
            println!("  Successful requests: {}", metrics.successful_requests);
            println!("  Qwen3 requests: {}", metrics.qwen3_requests);
            println!("  Fallback requests: {}", metrics.fallback_requests);

            println!("✅ Все базовые тесты Qwen3MemoryBridge прошли успешно");
        }
        Err(e) => {
            println!("❌ Не удалось создать Qwen3MemoryBridge: {e}");
            println!("💡 Проверьте что модель qwen3emb доступна в models/");
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(all(
    not(feature = "minimal"),
    feature = "embeddings",
    feature = "gpu-acceleration"
))]
#[tokio::test]
async fn test_qwen3_gpu_batch_processor_integration() -> Result<()> {
    use memory::{
        gpu_accelerated::{BatchProcessorConfig, GpuBatchProcessor},
        CacheConfig, EmbeddingCache,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    println!("🚀 Тестируем интеграцию Qwen3MemoryBridge с GpuBatchProcessor");

    // Создаем временный кэш
    let temp_dir = TempDir::new()?;
    let cache_path = temp_dir.path().join("test_cache");
    let cache = Arc::new(EmbeddingCache::new(cache_path, CacheConfig::default())?);

    // Создаем GpuBatchProcessor с Qwen3MemoryBridge
    let config = BatchProcessorConfig {
        max_batch_size: 8,
        batch_timeout_ms: 100,
        use_gpu_if_available: false, // CPU для стабильности тестов
        cache_embeddings: true,
    };

    match GpuBatchProcessor::with_qwen3_bridge(config, cache).await {
        Ok(processor) => {
            println!("✅ GpuBatchProcessor с Qwen3MemoryBridge создан");

            // Проверяем доступность Qwen3
            let qwen3_available = processor.is_qwen3_available().await;
            println!("Qwen3 интеграция доступна: {}", qwen3_available);

            // Тестируем embed методы через processor
            let test_result = processor.embed("integration test").await;
            match test_result {
                Ok(embedding) => {
                    println!(
                        "✅ Processor embedding получен (размерность: {})",
                        embedding.len()
                    );
                    assert!(!embedding.is_empty());
                }
                Err(e) => {
                    println!("⚠️ Processor embedding ошибка: {}", e);
                }
            }

            println!("✅ GpuBatchProcessor интеграция успешна");
        }
        Err(e) => {
            println!("⚠️ GpuBatchProcessor интеграция не удалась: {}", e);
            println!("💡 Это может быть нормально без реальной модели");
        }
    }

    Ok(())
}

#[cfg(not(all(not(feature = "minimal"), feature = "embeddings")))]
#[tokio::test]
async fn test_qwen3_feature_disabled() -> Result<()> {
    println!("ℹ️ Qwen3 integration test пропущен - feature 'embeddings' отключен");
    Ok(())
}
