#![cfg(feature = "gpu")]

use ai::{
    gpu_memory_pool::GPU_MEMORY_POOL,
    gpu_pipeline::{GpuPipelineManager, PipelineConfig},
    EmbeddingConfig,
};
use anyhow::Result;
use std::time::Instant;
use tracing_subscriber;

/// Тест GPU memory pool эффективности
#[tokio::test]
async fn test_memory_pool_efficiency() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    println!("🏊 Тестирование GPU Memory Pool");

    // Очищаем пул перед тестом
    let _ = GPU_MEMORY_POOL.clear_unused();

    // Тест множественных выделений памяти
    let sizes = vec![1024, 4096, 16384, 65536, 262144]; // От 1KB до 256KB
    let mut buffers = Vec::new();

    // Выделяем буферы
    for size in &sizes {
        for i in 0..5 {
            match GPU_MEMORY_POOL.acquire_buffer(*size) {
                Ok(buffer) => {
                    println!("✅ Выделен буфер #{} размером {} KB", i, size / 1024);
                    buffers.push(buffer);
                }
                Err(e) => {
                    println!("❌ Ошибка выделения буфера: {}", e);
                }
            }
        }
    }

    // Печатаем статистику после выделения
    println!("📊 Статистика после выделения:");
    let _ = GPU_MEMORY_POOL.print_stats();

    // Возвращаем буферы
    for buffer in buffers {
        let _ = GPU_MEMORY_POOL.release_buffer(buffer);
    }

    // Печатаем финальную статистику
    println!("📊 Финальная статистика:");
    let stats = GPU_MEMORY_POOL.get_stats()?;

    println!("  💾 Total allocations: {}", stats.allocations);
    println!("  💾 Total deallocations: {}", stats.deallocations);
    println!(
        "  📈 Hit rate: {:.1}%",
        if stats.allocations > 0 {
            (stats.hits as f64 / stats.allocations as f64) * 100.0
        } else {
            0.0
        }
    );

    assert!(stats.allocations > 0);

    Ok(())
}

/// Тест memory leak detection
#[tokio::test]
async fn test_memory_leak_detection() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    println!("🔍 Тестирование memory leak detection");

    // Получаем начальную статистику
    let initial_stats = GPU_MEMORY_POOL.get_stats()?;
    println!(
        "📊 Начальная статистика: {} буферов",
        initial_stats.current_buffers
    );

    // Симулируем work cycle
    for cycle in 0..3 {
        println!("🔄 Work cycle #{}", cycle + 1);

        // Выделяем память с помощью with_buffer
        let results: Result<Vec<Vec<u8>>> = (0..10)
            .map(|i| {
                GPU_MEMORY_POOL.with_buffer(1024 * (i + 1), |buffer| {
                    // Имитируем работу с буфером
                    buffer.extend_from_slice(&vec![0u8; 100]);
                    Ok(buffer[0..100].to_vec())
                })
            })
            .collect();

        match results {
            Ok(data) => {
                println!(
                    "✅ Cycle {} completed, processed {} buffers",
                    cycle + 1,
                    data.len()
                );
            }
            Err(e) => {
                println!("❌ Cycle {} failed: {}", cycle + 1, e);
            }
        }

        // Проверяем статистику после каждого цикла
        let cycle_stats = GPU_MEMORY_POOL.get_stats()?;
        println!(
            "  📊 После цикла {}: {} буферов",
            cycle + 1,
            cycle_stats.current_buffers
        );
    }

    // Принудительная очистка
    let _ = GPU_MEMORY_POOL.clear_unused();

    // Финальная проверка на утечки
    let final_stats = GPU_MEMORY_POOL.get_stats()?;
    println!(
        "📊 Финальная статистика: {} буферов",
        final_stats.current_buffers
    );

    // Проверяем что количество буферов не растёт неконтролируемо
    assert!(
        final_stats.current_buffers <= initial_stats.current_buffers + 10,
        "Возможная утечка памяти: {} -> {} буферов",
        initial_stats.current_buffers,
        final_stats.current_buffers
    );

    Ok(())
}

/// Тест concurrent memory access
#[tokio::test]
async fn test_concurrent_memory_access() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    println!("🔀 Тестирование concurrent memory access");

    let start_time = Instant::now();

    // Создаём несколько concurrent tasks
    let tasks: Vec<_> = (0..5)
        .map(|task_id| {
            tokio::spawn(async move {
                let mut local_results = Vec::new();

                for i in 0..10 {
                    let size = 1024 * (i % 8 + 1); // Размеры от 1KB до 8KB

                    let result = GPU_MEMORY_POOL
                        .with_buffer_async(size, |buffer| async move {
                            // Симулируем асинхронную работу
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                            // Записываем данные в буфер
                            let mut mut_buffer = buffer;
                            mut_buffer.extend_from_slice(&vec![(task_id * 10 + i) as u8; 100]);

                            Ok((format!("Task {} item {}", task_id, i), mut_buffer))
                        })
                        .await;

                    match result {
                        Ok(data) => local_results.push(data),
                        Err(e) => println!("❌ Task {} item {} failed: {}", task_id, i, e),
                    }
                }

                println!(
                    "✅ Task {} completed: {} results",
                    task_id,
                    local_results.len()
                );
                local_results
            })
        })
        .collect();

    // Ждём завершения всех задач
    let all_results: Vec<_> = futures::future::join_all(tasks).await;
    let total_results: usize = all_results
        .iter()
        .map(|result| result.as_ref().map(|r| r.len()).unwrap_or(0))
        .sum();

    let elapsed = start_time.elapsed();
    println!(
        "🏁 Concurrent test completed: {} total results in {:?}",
        total_results, elapsed
    );

    // Проверяем статистику пула
    let final_stats = GPU_MEMORY_POOL.get_stats()?;
    println!(
        "📊 Final pool stats: {}/{} hits/misses, efficiency: {:.1}%",
        final_stats.hits,
        final_stats.misses,
        if final_stats.allocations > 0 {
            (final_stats.hits as f64 / final_stats.allocations as f64) * 100.0
        } else {
            0.0
        }
    );

    assert!(total_results > 0);
    assert!(final_stats.allocations >= total_results as u64);

    Ok(())
}

/// Тест adaptive batching с memory constraints
#[tokio::test]
async fn test_adaptive_batching_memory() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    println!("📊 Тестирование adaptive batching с memory constraints");

    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        max_length: 128,
        batch_size: 16,
        use_gpu: true,
        ..Default::default()
    };

    let pipeline_config = PipelineConfig {
        max_concurrent_batches: 2,
        optimal_batch_size: 32,
        adaptive_batching: true,
        memory_pooling_enabled: true,
        ..Default::default()
    };

    // Создаём pipeline только если есть GPU
    let detector = ai::gpu_detector::GpuDetector::detect();
    if !detector.available {
        println!("⚠️ GPU недоступен, пропускаем adaptive batching тест");
        return Ok(());
    }

    let pipeline = match GpuPipelineManager::new(config, pipeline_config).await {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️ Не удалось создать pipeline: {}", e);
            return Ok(());
        }
    };

    // Тестируем с разными размерами данных
    let test_scenarios = vec![
        (10, "small batch"),
        (50, "medium batch"),
        (150, "large batch"),
    ];

    for (batch_size, scenario_name) in test_scenarios {
        println!(
            "🧪 Testing scenario: {} ({} texts)",
            scenario_name, batch_size
        );

        let test_texts: Vec<String> = (0..batch_size)
            .map(|i| format!("Adaptive batch test text #{} for {}", i, scenario_name))
            .collect();

        let start = Instant::now();
        match pipeline.process_texts_optimized(test_texts).await {
            Ok(embeddings) => {
                let elapsed = start.elapsed();
                println!(
                    "✅ {} completed: {} embeddings in {:?} ({:.1} texts/sec)",
                    scenario_name,
                    embeddings.len(),
                    elapsed,
                    embeddings.len() as f32 / elapsed.as_secs_f32()
                );
            }
            Err(e) => {
                println!("❌ {} failed: {}", scenario_name, e);
            }
        }

        // Получаем статистику pipeline
        let stats = pipeline.get_stats().await;
        println!(
            "  📊 Pipeline stats: avg_batch={:.1}, memory_efficiency={:.1}%",
            stats.avg_batch_size,
            stats.memory_pool_efficiency() * 100.0
        );
    }

    // Финальная очистка
    pipeline.cleanup().await;

    Ok(())
}

/// Stress test для memory system
#[tokio::test]
async fn test_memory_stress() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    println!("💪 Memory stress test");

    let iterations = 100;
    let max_size = 1024 * 1024; // 1MB max buffer

    let start_time = Instant::now();
    let mut successful_ops = 0;
    let mut failed_ops = 0;

    for i in 0..iterations {
        let size = 1024 + (i * 1024) % max_size; // Переменный размер

        let result = GPU_MEMORY_POOL.with_buffer(size, |buffer| {
            // Заполняем буфер данными
            let fill_value = (i % 256) as u8;
            for byte in buffer.iter_mut().take(1000) {
                *byte = fill_value;
            }

            // Проверяем что данные записались
            let check_ok = buffer.iter().take(1000).all(|&b| b == fill_value);

            if check_ok {
                Ok(buffer.len())
            } else {
                Err(anyhow::anyhow!("Data integrity check failed"))
            }
        });

        match result {
            Ok(_) => successful_ops += 1,
            Err(_) => failed_ops += 1,
        }

        // Печатаем прогресс каждые 25 операций
        if (i + 1) % 25 == 0 {
            println!(
                "  🔄 Progress: {}/{} operations ({} success, {} failed)",
                i + 1,
                iterations,
                successful_ops,
                failed_ops
            );
        }
    }

    let elapsed = start_time.elapsed();
    let ops_per_sec = iterations as f32 / elapsed.as_secs_f32();

    println!("🏁 Stress test completed:");
    println!("  ⏱️ Time: {:?}", elapsed);
    println!("  ✅ Successful ops: {}/{}", successful_ops, iterations);
    println!("  ❌ Failed ops: {}", failed_ops);
    println!("  🚀 Performance: {:.1} ops/sec", ops_per_sec);

    // Проверяем финальное состояние пула
    let final_stats = GPU_MEMORY_POOL.get_stats()?;
    println!(
        "  📊 Pool efficiency: {:.1}%",
        if final_stats.allocations > 0 {
            (final_stats.hits as f64 / final_stats.allocations as f64) * 100.0
        } else {
            0.0
        }
    );

    // Большинство операций должно быть успешным
    assert!(
        successful_ops > iterations * 80 / 100,
        "Too many failed operations: {}/{}",
        failed_ops,
        iterations
    );

    Ok(())
}
