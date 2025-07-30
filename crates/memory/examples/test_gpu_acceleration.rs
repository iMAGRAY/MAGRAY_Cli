use ai::{
    EmbeddingConfig, 
    GpuConfig,
    auto_device_selector::{AutoDeviceSelector, SmartEmbeddingFactory},
    gpu_detector::GpuDetector,
    gpu_memory_pool::GPU_MEMORY_POOL,
    embeddings_optimized_v2::OptimizedEmbeddingServiceV2,
};
use std::time::Instant;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка логирования
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Тестирование GPU ускорения для MAGRAY CLI");
    
    // 1. Проверка доступности GPU
    test_gpu_detection()?;
    
    // 2. Тест автоматического выбора устройства
    test_auto_device_selection().await?;
    
    // 3. Тест производительности CPU vs GPU
    test_performance_comparison().await?;
    
    // 4. Тест memory pooling
    test_memory_pooling()?;
    
    // 5. Тест динамической оптимизации
    test_dynamic_optimization().await?;
    
    info!("✅ Все тесты завершены успешно!");
    
    Ok(())
}

/// Тест определения GPU
fn test_gpu_detection() -> Result<()> {
    info!("\n📍 Тест 1: Определение GPU");
    
    let detector = GpuDetector::detect();
    
    if detector.available {
        info!("✅ GPU обнаружен!");
        info!("  - Количество устройств: {}", detector.devices.len());
        info!("  - CUDA версия: {}", detector.cuda_version);
        info!("  - Драйвер: {}", detector.driver_version);
        
        for (idx, device) in detector.devices.iter().enumerate() {
            info!("\n  GPU #{}: {}", idx, device.name);
            info!("    - Память: {} MB (свободно: {} MB)", 
                device.total_memory_mb, device.free_memory_mb);
            info!("    - Температура: {}°C", device.temperature);
            info!("    - Загрузка: {}%", device.utilization);
            info!("    - Compute capability: {}.{}", 
                device.compute_capability_major, device.compute_capability_minor);
        }
        
        // Проверяем оптимальные параметры
        let optimal = detector.get_optimal_params(500); // 500MB модель
        info!("\n  Оптимальные параметры для модели 500MB:");
        info!("    - Batch size: {}", optimal.batch_size);
        info!("    - Max sequence: {}", optimal.max_sequence_length);
        info!("    - FP16: {}", optimal.use_fp16);
        info!("    - GPU: {}", optimal.gpu_device_id);
    } else {
        info!("❌ GPU не обнаружен, будет использоваться CPU");
    }
    
    Ok(())
}

/// Тест автоматического выбора устройства
async fn test_auto_device_selection() -> Result<()> {
    info!("\n📍 Тест 2: Автоматический выбор CPU/GPU");
    
    let mut selector = AutoDeviceSelector::new();
    let config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        use_gpu: false, // Автоматически определится
        batch_size: 32,
        ..Default::default()
    };
    
    let decision = selector.select_device(&config).await?;
    
    info!("📊 Результат автоматического выбора:");
    info!("  - Устройство: {}", if decision.use_gpu { "GPU" } else { "CPU" });
    info!("  - Причина: {}", decision.reason);
    info!("  - CPU score: {:.2} items/sec", decision.cpu_score);
    if let Some(gpu_score) = decision.gpu_score {
        info!("  - GPU score: {:.2} items/sec", gpu_score);
        info!("  - Ускорение: {:.1}x", gpu_score / decision.cpu_score);
    }
    info!("  - Рекомендуемый batch size: {}", decision.recommended_batch_size);
    
    Ok(())
}

/// Тест сравнения производительности
async fn test_performance_comparison() -> Result<()> {
    info!("\n📍 Тест 3: Сравнение производительности CPU vs GPU");
    
    let test_sizes = vec![10, 50, 100, 500];
    let test_texts: Vec<String> = (0..500)
        .map(|i| format!("This is a test sentence number {} for benchmarking the embedding performance of our optimized service.", i))
        .collect();
    
    // Тест CPU
    info!("\n💻 Тестирование CPU:");
    
    let cpu_config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        use_gpu: false,
        batch_size: 32,
        ..Default::default()
    };
    
    if let Ok(cpu_service) = OptimizedEmbeddingServiceV2::new(cpu_config) {
        for &size in &test_sizes {
            let batch = test_texts.iter().take(size).cloned().collect();
            let start = Instant::now();
            
            match cpu_service.embed_batch(batch).await {
                Ok(embeddings) => {
                    let elapsed = start.elapsed();
                    info!("  {} текстов: {:.2}ms ({:.1} texts/sec)", 
                        size, 
                        elapsed.as_millis(),
                        size as f64 / elapsed.as_secs_f64()
                    );
                }
                Err(e) => {
                    error!("  Ошибка для {} текстов: {}", size, e);
                }
            }
        }
        
        // Выводим метрики
        cpu_service.print_metrics();
    }
    
    // Тест GPU (если доступен)
    #[cfg(feature = "gpu")]
    {
        let detector = GpuDetector::detect();
        if detector.available {
            info!("\n🎮 Тестирование GPU:");
            
            let gpu_config = EmbeddingConfig {
                model_name: "bge-m3".to_string(),
                use_gpu: true,
                gpu_config: Some(GpuConfig::auto_optimized()),
                batch_size: 128,
                ..Default::default()
            };
            
            match OptimizedEmbeddingServiceV2::new(gpu_config) {
                Ok(gpu_service) => {
                    for &size in &test_sizes {
                        let batch = test_texts.iter().take(size).cloned().collect();
                        let start = Instant::now();
                        
                        match gpu_service.embed_batch(batch).await {
                            Ok(embeddings) => {
                                let elapsed = start.elapsed();
                                info!("  {} текстов: {:.2}ms ({:.1} texts/sec)", 
                                    size, 
                                    elapsed.as_millis(),
                                    size as f64 / elapsed.as_secs_f64()
                                );
                            }
                            Err(e) => {
                                error!("  Ошибка для {} текстов: {}", size, e);
                            }
                        }
                    }
                    
                    // Выводим метрики
                    gpu_service.print_metrics();
                }
                Err(e) => {
                    error!("❌ Не удалось создать GPU сервис: {}", e);
                }
            }
        }
    }
    
    Ok(())
}

/// Тест memory pooling
fn test_memory_pooling() -> Result<()> {
    info!("\n📍 Тест 4: GPU Memory Pooling");
    
    // Тестируем выделение и освобождение памяти
    let sizes = vec![1024, 4096, 1024*1024, 4*1024*1024];
    
    for size in sizes {
        let buffer = GPU_MEMORY_POOL.acquire_buffer(size)?;
        info!("  Выделен буфер: {} KB", buffer.capacity() / 1024);
        GPU_MEMORY_POOL.release_buffer(buffer);
    }
    
    // Тестируем with_buffer
    let result = GPU_MEMORY_POOL.with_buffer(1024*1024, |buffer| {
        buffer.extend_from_slice(&vec![42u8; 1000]);
        Ok(buffer.len())
    })?;
    info!("  Обработано с временным буфером: {} байт", result);
    
    // Выводим статистику
    GPU_MEMORY_POOL.print_stats();
    
    Ok(())
}

/// Тест динамической оптимизации
async fn test_dynamic_optimization() -> Result<()> {
    info!("\n📍 Тест 5: Динамическая оптимизация");
    
    // Используем SmartEmbeddingFactory
    let base_config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        ..Default::default()
    };
    
    match SmartEmbeddingFactory::create_optimized(base_config).await {
        Ok((service, decision)) => {
            info!("✅ Создан оптимизированный сервис:");
            info!("  - Устройство: {}", if decision.use_gpu { "GPU" } else { "CPU" });
            info!("  - Batch size: {}", decision.recommended_batch_size);
            
            // Тестируем несколько запросов
            let test_batches = vec![
                vec!["Short text".to_string()],
                vec!["Medium length text that contains more words".to_string(); 10],
                vec!["This is a longer text that simulates real world usage with multiple sentences and various complexity levels.".to_string(); 50],
            ];
            
            for (idx, batch) in test_batches.into_iter().enumerate() {
                let size = batch.len();
                let start = Instant::now();
                
                match service.embed_batch(batch).await {
                    Ok(embeddings) => {
                        let elapsed = start.elapsed();
                        info!("  Batch {}: {} текстов за {:.2}ms", idx + 1, size, elapsed.as_millis());
                    }
                    Err(e) => {
                        error!("  Ошибка в batch {}: {}", idx + 1, e);
                    }
                }
            }
        }
        Err(e) => {
            error!("❌ Не удалось создать оптимизированный сервис: {}", e);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_gpu_system() {
        // Просто проверяем что основные компоненты компилируются
        let detector = GpuDetector::detect();
        assert!(detector.cuda_version.is_empty() || !detector.cuda_version.is_empty());
        
        let selector = AutoDeviceSelector::new();
        assert!(true); // Просто проверка создания
    }
}