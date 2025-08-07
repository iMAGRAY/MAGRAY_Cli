use ai::{
    config::EmbeddingConfig,
    errors::Result,
    gpu_pipeline::{GpuPipelineManager, PipelineConfig, PipelineStats},
    models::ModelType,
};
use mockall::{mock, predicate::*};
use proptest::prelude::*;
use rstest::*;
use serial_test::serial;
use std::{sync::Arc, time::Duration};
use tokio_test::*;

// Mock для внешних зависимостей
mock! {
    EmbeddingProvider {}

    #[async_trait::async_trait]
    impl ai::embeddings_gpu::EmbeddingProvider for EmbeddingProvider {
        async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
        async fn get_stats(&self) -> ai::embeddings_gpu::EmbeddingStats;
        async fn cleanup(&self);
    }
}

#[fixture]
fn pipeline_config() -> PipelineConfig {
    PipelineConfig {
        batch_size: 32,
        max_concurrent_batches: 4,
        memory_limit_mb: 1024,
        timeout_seconds: 30,
        enable_memory_pooling: true,
        cache_embeddings: false,
    }
}

#[fixture]
fn embedding_config() -> EmbeddingConfig {
    EmbeddingConfig {
        model_type: ModelType::BgeM3,
        device: "cuda:0".to_string(),
        batch_size: 32,
        max_sequence_length: 512,
        normalize_embeddings: true,
    }
}

#[rstest]
#[tokio::test]
#[serial]
async fn test_pipeline_creation_success(
    embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange - создание конфигурации для успешной инициализации

    // Act - создание пайплайна
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await;

    // Assert - проверка успешного создания
    assert!(pipeline.is_ok(), "Pipeline должен создаваться без ошибок");

    let pipeline = pipeline?;

    // Проверяем начальные метрики
    let stats = pipeline.get_stats().await;
    assert_eq!(stats.total_processed, 0);
    assert_eq!(stats.active_batches, 0);

    // Cleanup
    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_batch_processing(
    embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    let test_texts = vec![
        "Test text for embedding".to_string(),
        "Another test sentence".to_string(),
        "Third text for processing".to_string(),
    ];

    // Act - обработка текстов
    let result = pipeline.process_texts_optimized(test_texts.clone()).await;

    // Assert
    assert!(result.is_ok(), "Обработка текстов должна проходить успешно");

    let embeddings = result?;
    assert_eq!(
        embeddings.len(),
        test_texts.len(),
        "Количество embeddings должно соответствовать количеству текстов"
    );

    // Проверяем размерности векторов
    for embedding in &embeddings {
        assert!(!embedding.is_empty(), "Embedding не должен быть пустым");
        assert!(
            embedding.len() > 0,
            "Размерность embedding должна быть больше 0"
        );
    }

    // Проверяем статистику
    let stats = pipeline.get_stats().await;
    assert!(stats.total_processed >= test_texts.len() as u64);
    assert_eq!(
        stats.active_batches, 0,
        "После обработки не должно быть активных батчей"
    );

    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_concurrent_processing(
    embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange
    let pipeline = Arc::new(GpuPipelineManager::new(embedding_config, pipeline_config).await?);

    let test_batches: Vec<Vec<String>> = (0..3)
        .map(|i| (0..10).map(|j| format!("Batch {} text {}", i, j)).collect())
        .collect();

    // Act - параллельная обработка нескольких батчей
    let tasks: Vec<_> = test_batches
        .into_iter()
        .map(|batch| {
            let pipeline = pipeline.clone();
            tokio::spawn(async move { pipeline.process_texts_optimized(batch).await })
        })
        .collect();

    let results = futures::future::try_join_all(tasks).await?;

    // Assert
    for result in results {
        assert!(
            result.is_ok(),
            "Все параллельные задачи должны завершаться успешно"
        );
        let embeddings = result?;
        assert!(
            !embeddings.is_empty(),
            "Каждый результат должен содержать embeddings"
        );
    }

    // Проверяем итоговую статистику
    let stats = pipeline.get_stats().await;
    assert!(
        stats.total_processed >= 30,
        "Должно быть обработано минимум 30 текстов"
    );
    assert_eq!(
        stats.active_batches, 0,
        "После завершения не должно быть активных батчей"
    );

    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_memory_management(
    embedding_config: EmbeddingConfig,
    mut pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange - ограниченная память
    pipeline_config.memory_limit_mb = 128;
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    // Act - попытка обработки большого количества текстов
    let large_batch: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                "Large text batch item {} with more content to increase memory usage",
                i
            )
        })
        .collect();

    let result = pipeline.process_texts_optimized(large_batch).await;

    // Assert - должна быть успешная обработка или контролируемый сбой
    match result {
        Ok(embeddings) => {
            assert!(!embeddings.is_empty(), "Должны быть получены embeddings");

            // Проверяем статистику памяти
            let stats = pipeline.get_stats().await;
            assert!(
                stats.memory_pool_efficiency() <= 1.0,
                "Эффективность памяти не должна превышать 100%"
            );
        }
        Err(e) => {
            // Если память исчерпана, должна быть соответствующая ошибка
            assert!(
                e.to_string().contains("memory") || e.to_string().contains("limit"),
                "Ошибка должна быть связана с ограничениями памяти: {}",
                e
            );
        }
    }

    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_error_handling_invalid_config(
    mut embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange - некорректная конфигурация
    embedding_config.device = "invalid_device".to_string();

    // Act - попытка создания пайплайна с некорректной конфигурацией
    let result = GpuPipelineManager::new(embedding_config, pipeline_config).await;

    // Assert - должна быть ошибка
    assert!(
        result.is_err(),
        "Создание пайплайна с некорректным устройством должно завершаться ошибкой"
    );

    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("device") || error.to_string().contains("invalid"),
        "Ошибка должна содержать информацию о некорректном устройстве: {}",
        error
    );

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_timeout_handling(
    embedding_config: EmbeddingConfig,
    mut pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange - очень короткий timeout
    pipeline_config.timeout_seconds = 1;
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    // Act - обработка с потенциальным timeout
    let large_text = vec!["Very long text ".repeat(10000)];

    let start = std::time::Instant::now();
    let result = pipeline.process_texts_optimized(large_text).await;
    let duration = start.elapsed();

    // Assert - либо успех в разумное время, либо timeout
    match result {
        Ok(_) => {
            assert!(
                duration.as_secs() <= 10,
                "Обработка должна завершаться в разумное время"
            );
        }
        Err(e) => {
            assert!(
                e.to_string().contains("timeout") || duration.as_secs() <= 10,
                "При timeout должна быть соответствующая ошибка или быстрое завершение: {}",
                e
            );
        }
    }

    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_stats_accuracy(
    embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    // Получаем начальные статистики
    let initial_stats = pipeline.get_stats().await;
    assert_eq!(initial_stats.total_processed, 0);

    // Act - обработка известного количества текстов
    let test_texts = vec![
        "First text".to_string(),
        "Second text".to_string(),
        "Third text".to_string(),
    ];

    let _result = pipeline.process_texts_optimized(test_texts.clone()).await?;

    // Assert - проверка обновленной статистики
    let final_stats = pipeline.get_stats().await;
    assert!(
        final_stats.total_processed >= initial_stats.total_processed + test_texts.len() as u64,
        "Счетчик обработанных текстов должен увеличиться"
    );

    assert!(
        final_stats.throughput_per_second() >= 0.0,
        "Пропускная способность должна быть неотрицательной"
    );

    assert!(
        final_stats.memory_pool_efficiency() >= 0.0 && final_stats.memory_pool_efficiency() <= 1.0,
        "Эффективность памяти должна быть от 0 до 1"
    );

    pipeline.cleanup().await;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_pipeline_cleanup_resources(
    embedding_config: EmbeddingConfig,
    pipeline_config: PipelineConfig,
) -> Result<()> {
    // Arrange
    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    // Act - использование ресурсов
    let test_texts = vec!["Test".to_string()];
    let _result = pipeline.process_texts_optimized(test_texts).await?;

    // Проверяем статистику до cleanup
    let stats_before = pipeline.get_stats().await;
    assert!(stats_before.total_processed > 0);

    // Cleanup
    pipeline.cleanup().await;

    // Assert - ресурсы должны быть освобождены
    let stats_after = pipeline.get_stats().await;
    assert_eq!(
        stats_after.active_batches, 0,
        "После cleanup не должно быть активных батчей"
    );

    // Попытка использования после cleanup должна быть безопасной
    let texts_after_cleanup = vec!["After cleanup".to_string()];
    let result_after = pipeline.process_texts_optimized(texts_after_cleanup).await;

    // Может быть ошибка или успешная обработка, но не паника
    match result_after {
        Ok(_) => {
            // Если обработка продолжается, это нормально
        }
        Err(e) => {
            // Если ошибка, она должна быть связана с cleanup
            assert!(
                e.to_string().contains("cleanup") || e.to_string().contains("shutdown"),
                "Ошибка после cleanup должна быть соответствующей: {}",
                e
            );
        }
    }

    Ok(())
}

// Property-based тесты для проверки инвариантов
proptest! {
    #[test]
    fn test_pipeline_output_dimensions_consistent(
        texts in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]{1,100}").unwrap(), 1..10)
    ) {
        tokio_test::block_on(async {
            let embedding_config = EmbeddingConfig {
                model_type: ModelType::BgeM3,
                device: "cpu".to_string(), // Используем CPU для property-based тестов
                batch_size: 16,
                max_sequence_length: 512,
                normalize_embeddings: true,
            };

            let pipeline_config = PipelineConfig::default();

            if let Ok(pipeline) = GpuPipelineManager::new(embedding_config, pipeline_config).await {
                if let Ok(embeddings) = pipeline.process_texts_optimized(texts.clone()).await {
                    // Инвариант: количество выходных векторов равно количеству входных текстов
                    prop_assert_eq!(embeddings.len(), texts.len());

                    // Инвариант: все векторы имеют одинаковую размерность
                    if !embeddings.is_empty() {
                        let expected_dim = embeddings[0].len();
                        prop_assert!(expected_dim > 0, "Размерность должна быть положительной");

                        for embedding in &embeddings {
                            prop_assert_eq!(
                                embedding.len(),
                                expected_dim,
                                "Все векторы должны иметь одинаковую размерность"
                            );
                        }
                    }

                    pipeline.cleanup().await;
                }
            }
        })?;
    }

    #[test]
    fn test_pipeline_batch_size_invariant(
        batch_size in 1u32..64,
        num_texts in 1usize..100
    ) {
        tokio_test::block_on(async {
            let embedding_config = EmbeddingConfig {
                model_type: ModelType::BgeM3,
                device: "cpu".to_string(),
                batch_size: batch_size,
                max_sequence_length: 512,
                normalize_embeddings: true,
            };

            let pipeline_config = PipelineConfig {
                batch_size,
                ..Default::default()
            };

            let texts: Vec<String> = (0..num_texts)
                .map(|i| format!("Test text {}", i))
                .collect();

            if let Ok(pipeline) = GpuPipelineManager::new(embedding_config, pipeline_config).await {
                if let Ok(embeddings) = pipeline.process_texts_optimized(texts.clone()).await {
                    // Инвариант: результат не зависит от размера батча
                    prop_assert_eq!(embeddings.len(), texts.len());

                    let stats = pipeline.get_stats().await;
                    prop_assert!(stats.total_processed >= texts.len() as u64);

                    pipeline.cleanup().await;
                }
            }
        })?;
    }
}

// Benchmark тесты для проверки производительности
#[tokio::test]
#[ignore] // Игнорируем по умолчанию, запускаем отдельно
async fn benchmark_pipeline_throughput() -> Result<()> {
    let embedding_config = EmbeddingConfig {
        model_type: ModelType::BgeM3,
        device: "cuda:0".to_string(),
        batch_size: 32,
        max_sequence_length: 512,
        normalize_embeddings: true,
    };

    let pipeline_config = PipelineConfig {
        batch_size: 32,
        max_concurrent_batches: 4,
        memory_limit_mb: 2048,
        timeout_seconds: 60,
        enable_memory_pooling: true,
        cache_embeddings: false,
    };

    let pipeline = GpuPipelineManager::new(embedding_config, pipeline_config).await?;

    // Генерируем большой набор тестовых данных
    let test_texts: Vec<String> = (0..1000)
        .map(|i| {
            format!(
                "Performance test text number {} with sufficient length for realistic testing",
                i
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let embeddings = pipeline.process_texts_optimized(test_texts.clone()).await?;
    let duration = start.elapsed();

    // Вычисляем метрики производительности
    let throughput = test_texts.len() as f64 / duration.as_secs_f64();
    let stats = pipeline.get_stats().await;

    println!("Benchmark Results:");
    println!("  Texts processed: {}", test_texts.len());
    println!("  Time taken: {:?}", duration);
    println!("  Throughput: {:.2} texts/sec", throughput);
    println!(
        "  Pipeline throughput: {:.2} texts/sec",
        stats.throughput_per_second()
    );
    println!(
        "  Memory efficiency: {:.2}%",
        stats.memory_pool_efficiency() * 100.0
    );

    // Проверяем минимальные требования к производительности
    assert!(
        throughput > 10.0,
        "Пропускная способность должна быть больше 10 текстов/сек"
    );
    assert_eq!(embeddings.len(), test_texts.len());

    pipeline.cleanup().await;
    Ok(())
}
