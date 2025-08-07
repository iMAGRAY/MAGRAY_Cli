use ai::{
    auto_device_selector::SmartEmbeddingFactory, gpu_detector::GpuDetector,
    gpu_fallback::GpuFallbackManager, EmbeddingConfig, GpuConfig,
};
use anyhow::Result;
use tracing_subscriber;

/// Тест автоматического fallback с GPU на CPU
#[tokio::test]
async fn test_gpu_cpu_fallback() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 128,
        use_gpu: true,
        batch_size: 8,
        gpu_config: Some(GpuConfig::auto_optimized()),
        ..Default::default()
    };

    // Создаём fallback manager
    let manager = GpuFallbackManager::new(config).await?;

    // Тестовые тексты разной длины
    let test_texts = vec![
        "Short text".to_string(),
        "This is a medium length text for testing embedding generation".to_string(),
        "This is a much longer text that should test the embedding model's ability to handle various sequence lengths and maintain consistent performance across different input sizes".to_string(),
    ];

    // Выполняем embedding с fallback
    let embeddings = manager
        .embed_batch_with_fallback(test_texts.clone())
        .await?;

    assert_eq!(embeddings.len(), test_texts.len());
    for embedding in &embeddings {
        assert!(!embedding.is_empty());
        assert!(embedding.len() >= 512); // Минимальный размер для embedding

        // Проверяем нормализацию
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.01,
            "Embedding не нормализован: norm={}",
            norm
        );
    }

    // Получаем статистику
    let stats = manager.get_stats();
    println!(
        "📊 Fallback stats: GPU successes: {}, CPU fallbacks: {}, success rate: {:.2}%",
        stats.gpu_success_count,
        stats.cpu_fallback_count,
        stats.gpu_success_rate() * 100.0
    );

    Ok(())
}

/// Тест SmartEmbeddingFactory
#[tokio::test]
async fn test_smart_embedding_factory() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let base_config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 256,
        ..Default::default()
    };

    // Оптимизированный конфиг для системы
    let optimized_config = SmartEmbeddingFactory::optimize_config_for_system(base_config.clone());

    println!(
        "🎯 Optimized config: GPU={}, batch_size={}",
        optimized_config.use_gpu, optimized_config.batch_size
    );

    // Создаём умный сервис
    let (service, decision) = SmartEmbeddingFactory::create_optimized(optimized_config).await?;

    println!(
        "✅ Device decision: use_gpu={}, reason: {}",
        decision.use_gpu, decision.reason
    );

    // Тестируем с небольшими данными
    let test_texts = vec![
        "Test embedding generation".to_string(),
        "Another test text".to_string(),
    ];

    let embeddings = service.embed_batch(test_texts).await?;
    assert_eq!(embeddings.len(), 2);

    Ok(())
}

/// Тест high-throughput pipeline
#[tokio::test]
async fn test_high_throughput_pipeline() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let base_config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 128,
        batch_size: 32,
        ..Default::default()
    };

    // Проверяем доступность GPU
    let detector = GpuDetector::detect();
    if !detector.available {
        println!("⚠️ GPU недоступен, пропускаем pipeline тест");
        return Ok(());
    }

    // Создаём pipeline
    let pipeline =
        SmartEmbeddingFactory::create_high_throughput_pipeline(base_config, Some(2)).await?;

    // Генерируем больше тестовых данных для проверки пайплайна
    let large_batch: Vec<String> = (0..100)
        .map(|i| format!("Test text number {} for pipeline processing", i))
        .collect();

    println!("🚀 Processing {} texts through pipeline", large_batch.len());

    let start_time = std::time::Instant::now();
    let embeddings = pipeline.process_texts_optimized(large_batch).await?;
    let elapsed = start_time.elapsed();

    println!(
        "⚡ Pipeline processed {} embeddings in {:?} ({:.1} texts/sec)",
        embeddings.len(),
        elapsed,
        embeddings.len() as f32 / elapsed.as_secs_f32()
    );

    assert_eq!(embeddings.len(), 100);

    // Печатаем подробную статистику
    pipeline.print_detailed_stats().await;

    Ok(())
}

/// Тест circuit breaker functionality
#[tokio::test]
async fn test_circuit_breaker_behavior() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 64,
        use_gpu: true,
        ..Default::default()
    };

    let manager = GpuFallbackManager::new(config).await?;

    // Принудительно переключаемся в CPU режим
    manager.force_cpu_mode();

    let test_texts = vec!["Forced CPU mode test".to_string()];
    let embeddings = manager.embed_batch_with_fallback(test_texts).await?;

    assert_eq!(embeddings.len(), 1);

    let stats = manager.get_stats();
    println!(
        "🔴 Forced CPU stats: fallbacks={}",
        stats.cpu_fallback_count
    );

    // Сбрасываем circuit breaker
    manager.reset_circuit_breaker();

    let test_texts2 = vec!["After circuit breaker reset".to_string()];
    let embeddings2 = manager.embed_batch_with_fallback(test_texts2).await?;

    assert_eq!(embeddings2.len(), 1);

    Ok(())
}

/// Тест различных GPU providers
#[tokio::test]
async fn test_gpu_providers() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let detector = GpuDetector::detect();
    if !detector.available {
        println!("⚠️ GPU недоступен, пропускаем provider тесты");
        return Ok(());
    }

    // Тестируем разные типы providers
    let provider_types = vec![
        ai::gpu_config::GpuProviderType::Auto,
        ai::gpu_config::GpuProviderType::CUDA,
    ];

    // Добавляем DirectML только на Windows
    #[cfg(windows)]
    let provider_types = {
        let mut types = provider_types;
        types.push(ai::gpu_config::GpuProviderType::DirectML);
        types
    };

    for provider_type in provider_types {
        println!("🧪 Testing provider: {:?}", provider_type);

        let mut gpu_config = GpuConfig::auto_optimized();
        gpu_config.preferred_provider = provider_type.clone();

        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            max_length: 64,
            use_gpu: true,
            gpu_config: Some(gpu_config),
            batch_size: 4,
            ..Default::default()
        };

        match GpuFallbackManager::new(config).await {
            Ok(manager) => {
                let test_texts = vec![format!("Test for provider {:?}", provider_type)];
                match manager.embed_batch_with_fallback(test_texts).await {
                    Ok(embeddings) => {
                        println!(
                            "✅ Provider {:?} successful: {} embeddings",
                            provider_type,
                            embeddings.len()
                        );
                    }
                    Err(e) => {
                        println!("⚠️ Provider {:?} embedding failed: {}", provider_type, e);
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Provider {:?} unavailable: {}", provider_type, e);
            }
        }
    }

    Ok(())
}

/// Benchmark тест производительности
#[tokio::test]
async fn test_performance_benchmark() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 256,
        batch_size: 16,
        ..Default::default()
    };

    // Создаём оптимизированный сервис
    let (service, decision) = SmartEmbeddingFactory::create_optimized(config).await?;

    println!(
        "📊 Benchmark running on: {}",
        if decision.use_gpu { "GPU" } else { "CPU" }
    );

    // Тестовый датасет
    let benchmark_texts: Vec<String> = (0..50)
        .map(|i| {
            format!(
                "Benchmark text #{} with variable length content to test performance",
                i
            )
        })
        .collect();

    let start = std::time::Instant::now();
    let embeddings = service.embed_batch(benchmark_texts.clone()).await?;
    let elapsed = start.elapsed();

    let throughput = benchmark_texts.len() as f32 / elapsed.as_secs_f32();

    println!("🚀 Performance results:");
    println!("  📝 Texts processed: {}", embeddings.len());
    println!("  ⏱️ Time taken: {:?}", elapsed);
    println!("  🎯 Throughput: {:.1} texts/sec", throughput);
    println!(
        "  💾 Avg embedding size: {:.0}",
        embeddings.iter().map(|e| e.len()).sum::<usize>() as f32 / embeddings.len() as f32
    );

    assert_eq!(embeddings.len(), benchmark_texts.len());
    assert!(throughput > 0.0);

    Ok(())
}
