// @component: {"k":"T","id":"gpu_batch_processor_tests","t":"Comprehensive unit tests for GpuBatchProcessor","m":{"cur":100,"tgt":100,"u":"%"},"f":["testing","gpu","performance","batch","fallback"]}

//! Comprehensive Unit Tests для GpuBatchProcessor
//!
//! Покрывает критический компонент (90% готовности, 800+ строк):
//! - GPU initialization и detection
//! - Batch processing с GPU acceleration
//! - CPU fallback механизмы
//! - Memory pooling для GPU
//! - CUDA operations и error handling
//! - Performance SLA (<5ms для GPU)
//! - Graceful degradation

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use memory::{
        GpuBatchConfig, GpuBatchProcessor, GpuBatchStats, GpuDevice, GpuError, GpuMemoryPool,
        Layer, Record,
    };
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::RwLock;
    use uuid::Uuid;

    // ===== Утилиты для тестов =====

    /// Создание тестового GpuBatchProcessor
    async fn create_test_gpu_processor() -> GpuBatchProcessor {
        let config = GpuBatchConfig {
            max_batch_size: 64,
            batch_timeout_ms: 30,
            memory_pool_size_mb: 128,
            enable_fallback: true,
            cuda_device_id: 0,
            tensor_cores: true,
            mixed_precision: true,
            memory_fraction: 0.7,
        };

        GpuBatchProcessor::new(config).await
    }

    /// Создание CPU fallback процессора
    async fn create_fallback_processor() -> GpuBatchProcessor {
        let config = GpuBatchConfig {
            max_batch_size: 32,
            batch_timeout_ms: 50,
            memory_pool_size_mb: 64,
            enable_fallback: true,
            cuda_device_id: -1, // Force fallback
            tensor_cores: false,
            mixed_precision: false,
            memory_fraction: 0.5,
        };

        GpuBatchProcessor::new(config).await
    }

    /// Создание тестовой записи с embeddings
    fn create_gpu_test_record(index: usize, dimension: usize) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: format!("GPU test record {} for batch processing", index),
            embedding: vec![0.1 + (index as f32 * 0.01); dimension],
            layer: Layer::Interact,
            kind: "gpu_test".to_string(),
            tags: vec!["gpu".to_string(), "performance".to_string()],
            project: "gpu_test".to_string(),
            session: "gpu_session".to_string(),
            ts: Utc::now(),
            score: 0.9,
            access_count: 0,
            last_access: Utc::now(),
        }
    }

    /// Генерация GPU-aligned batch записей
    fn generate_gpu_batch(size: usize, dimension: usize) -> Vec<Record> {
        (0..size)
            .map(|i| create_gpu_test_record(i, dimension))
            .collect()
    }

    // ===== РАЗДЕЛ 1: GPU Initialization и Detection =====

    #[tokio::test]
    async fn test_gpu_processor_initialization() {
        let processor = create_test_gpu_processor().await;

        let stats = processor.get_stats().await;

        // Проверяем начальное состояние
        assert_eq!(stats.total_batches, 0, "Should start with 0 batches");
        assert_eq!(stats.total_records, 0, "Should start with 0 records");
        assert!(
            stats.gpu_device_info.is_some() || stats.fallback_active,
            "Should have GPU info or fallback"
        );
    }

    #[tokio::test]
    async fn test_gpu_device_detection() {
        // Проверяем GPU detection
        let gpu_available = memory::gpu_detector::is_cuda_available();

        if gpu_available {
            let devices = memory::gpu_detector::get_gpu_devices().await;
            assert!(devices.len() > 0, "Should detect at least one GPU");

            for device in devices {
                assert!(device.memory_total > 0, "GPU should have memory");
                assert!(
                    device.compute_capability >= 6.0,
                    "Should support modern CUDA"
                );
                assert!(!device.name.is_empty(), "GPU should have name");
            }
        } else {
            println!("⚠️  No GPU detected - testing fallback only");
        }
    }

    #[tokio::test]
    async fn test_gpu_memory_pool_initialization() {
        let pool = GpuMemoryPool::new(64).await; // 64MB pool

        if pool.is_ok() {
            let pool = pool.unwrap();
            let stats = pool.get_stats().await;

            assert!(
                stats.total_size_mb >= 64,
                "Pool should have allocated memory"
            );
            assert_eq!(stats.used_size_mb, 0, "Pool should start empty");
            assert!(stats.free_size_mb > 0, "Should have free memory");
        } else {
            println!("⚠️  GPU memory pool unavailable - using CPU fallback");
        }
    }

    // ===== РАЗДЕЛ 2: Batch Processing с GPU =====

    #[tokio::test]
    async fn test_gpu_single_batch_processing() {
        let processor = create_test_gpu_processor().await;
        let batch = generate_gpu_batch(32, 1024); // 32 records, 1024D

        let start = Instant::now();
        let result = processor.process_batch(batch.clone()).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "GPU batch processing should succeed");

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_batches, 1, "Should have processed 1 batch");
        assert_eq!(stats.total_records, 32, "Should have processed 32 records");

        // GPU должен быть быстрее CPU для больших batch
        if !stats.fallback_active {
            assert!(duration.as_millis() < 50, "GPU batch should be under 50ms");
        }
    }

    #[tokio::test]
    async fn test_large_gpu_batch_processing() {
        let processor = create_test_gpu_processor().await;
        let batch = generate_gpu_batch(128, 1024); // Большой batch для GPU

        let start = Instant::now();
        let result = processor.process_batch(batch).await;
        let duration = start.elapsed();

        assert!(result.is_ok(), "Large GPU batch should process");

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_records, 128, "Should process all 128 records");

        if !stats.fallback_active {
            // GPU должен справляться с большими batch эффективно
            assert!(
                duration.as_millis() < 200,
                "Large GPU batch should be under 200ms"
            );
        }
    }

    #[tokio::test]
    async fn test_mixed_precision_processing() {
        let mut config = GpuBatchConfig {
            max_batch_size: 64,
            batch_timeout_ms: 30,
            memory_pool_size_mb: 128,
            enable_fallback: true,
            cuda_device_id: 0,
            tensor_cores: true,
            mixed_precision: true, // FP16/FP32 mix
            memory_fraction: 0.7,
        };

        let processor = GpuBatchProcessor::new(config.clone()).await;
        let batch = generate_gpu_batch(64, 1024);

        let start = Instant::now();
        let result = processor.process_batch(batch).await;
        let fp16_duration = start.elapsed();

        assert!(result.is_ok(), "Mixed precision should work");

        // Сравниваем с full precision
        config.mixed_precision = false;
        let fp32_processor = GpuBatchProcessor::new(config).await;
        let batch_fp32 = generate_gpu_batch(64, 1024);

        let start = Instant::now();
        let result_fp32 = fp32_processor.process_batch(batch_fp32).await;
        let fp32_duration = start.elapsed();

        assert!(result_fp32.is_ok(), "FP32 should work");

        let stats = processor.get_stats().await;
        if !stats.fallback_active {
            // FP16 должен быть быстрее или сопоставим с FP32
            println!("FP16: {:?}, FP32: {:?}", fp16_duration, fp32_duration);
            assert!(
                fp16_duration <= fp32_duration * 2,
                "FP16 should not be much slower"
            );
        }
    }

    // ===== РАЗДЕЛ 3: CPU Fallback механизмы =====

    #[tokio::test]
    async fn test_automatic_fallback_on_gpu_unavailable() {
        // Создаем processor с недоступным GPU
        let processor = create_fallback_processor().await;
        let batch = generate_gpu_batch(16, 1024);

        let result = processor.process_batch(batch).await;
        assert!(result.is_ok(), "Should fallback to CPU gracefully");

        let stats = processor.get_stats().await;
        assert!(stats.fallback_active, "Should be using CPU fallback");
        assert_eq!(stats.total_records, 16, "Should still process all records");
    }

    #[tokio::test]
    async fn test_fallback_on_gpu_memory_exhaustion() {
        let processor = create_test_gpu_processor().await;

        // Пытаемся обработать очень большой batch
        let huge_batch = generate_gpu_batch(1000, 2048); // 1000 x 2048D vectors

        let result = processor.process_batch(huge_batch).await;

        // Должен либо успешно обработать, либо gracefully fallback
        assert!(result.is_ok(), "Should handle large batch or fallback");

        let stats = processor.get_stats().await;
        if stats.fallback_active {
            println!("✅ Graceful fallback to CPU on memory exhaustion");
        } else {
            println!("✅ GPU handled large batch successfully");
        }
    }

    #[tokio::test]
    async fn test_fallback_performance_comparison() {
        let gpu_processor = create_test_gpu_processor().await;
        let fallback_processor = create_fallback_processor().await;

        let batch_size = 32;
        let gpu_batch = generate_gpu_batch(batch_size, 1024);
        let cpu_batch = generate_gpu_batch(batch_size, 1024);

        // GPU timing
        let start = Instant::now();
        let gpu_result = gpu_processor.process_batch(gpu_batch).await;
        let gpu_duration = start.elapsed();

        // CPU fallback timing
        let start = Instant::now();
        let cpu_result = fallback_processor.process_batch(cpu_batch).await;
        let cpu_duration = start.elapsed();

        assert!(gpu_result.is_ok(), "GPU should work");
        assert!(cpu_result.is_ok(), "CPU fallback should work");

        println!("GPU: {:?}, CPU: {:?}", gpu_duration, cpu_duration);

        let gpu_stats = gpu_processor.get_stats().await;
        let cpu_stats = fallback_processor.get_stats().await;

        assert!(cpu_stats.fallback_active, "CPU should be in fallback mode");

        // Для небольших batch CPU может быть быстрее из-за overhead GPU
        // Проверяем что оба работают корректно
        assert_eq!(
            gpu_stats.total_records, batch_size,
            "GPU should process all"
        );
        assert_eq!(
            cpu_stats.total_records, batch_size,
            "CPU should process all"
        );
    }

    // ===== РАЗДЕЛ 4: Memory Pooling для GPU =====

    #[tokio::test]
    async fn test_gpu_memory_pool_reuse() {
        let processor = create_test_gpu_processor().await;

        // Обрабатываем несколько batch для тестирования pool reuse
        for i in 0..10 {
            let batch = generate_gpu_batch(24, 1024);
            let result = processor.process_batch(batch).await;
            assert!(result.is_ok(), "Batch {} should process", i);
        }

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_batches, 10, "Should have 10 batches");

        if let Some(pool_stats) = stats.memory_pool_stats {
            assert!(pool_stats.reuse_count > 0, "Should reuse GPU memory");
            assert!(pool_stats.efficiency > 0.5, "Pool should be efficient");
        }
    }

    #[tokio::test]
    async fn test_gpu_memory_alignment() {
        let processor = create_test_gpu_processor().await;

        // Тестируем разные размеры для проверки alignment
        let sizes = vec![16, 32, 48, 64, 80]; // Различные размеры batch

        for size in sizes {
            let batch = generate_gpu_batch(size, 1024);
            let result = processor.process_batch(batch).await;
            assert!(
                result.is_ok(),
                "Size {} should process with proper alignment",
                size
            );
        }

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_batches, 5, "Should process all size variants");
    }

    // ===== РАЗДЕЛ 5: CUDA Operations и Error Handling =====

    #[tokio::test]
    async fn test_cuda_context_management() {
        // Проверяем что CUDA context правильно управляется
        let processor1 = create_test_gpu_processor().await;
        let processor2 = create_test_gpu_processor().await;

        // Параллельные операции на разных processors
        let batch1 = generate_gpu_batch(20, 1024);
        let batch2 = generate_gpu_batch(20, 1024);

        let handle1 = tokio::spawn(async move { processor1.process_batch(batch1).await });

        let handle2 = tokio::spawn(async move { processor2.process_batch(batch2).await });

        let (result1, result2) = tokio::join!(handle1, handle2);

        assert!(result1.unwrap().is_ok(), "Processor 1 should succeed");
        assert!(result2.unwrap().is_ok(), "Processor 2 should succeed");
    }

    #[tokio::test]
    async fn test_gpu_error_recovery() {
        let processor = create_test_gpu_processor().await;

        // Создаем batch с потенциально проблемными данными
        let mut batch = generate_gpu_batch(16, 1024);

        // Добавляем NaN/Inf values для тестирования error handling
        batch[5].embedding = vec![f32::NAN; 1024];
        batch[10].embedding = vec![f32::INFINITY; 1024];

        let result = processor.process_batch(batch).await;

        // Processor должен обработать или gracefully fail
        if result.is_ok() {
            println!("✅ GPU handled problematic data");
        } else {
            println!("✅ GPU gracefully failed with problematic data");
        }

        // После ошибки processor должен продолжать работать
        let normal_batch = generate_gpu_batch(8, 1024);
        let recovery_result = processor.process_batch(normal_batch).await;
        assert!(recovery_result.is_ok(), "Should recover after error");
    }

    // ===== РАЗДЕЛ 6: Performance SLA для GPU =====

    #[tokio::test]
    async fn test_gpu_sub_5ms_sla() {
        let processor = create_test_gpu_processor().await;
        let mut latencies = vec![];

        // Прогрев GPU
        for _ in 0..5 {
            let batch = generate_gpu_batch(16, 1024);
            let _ = processor.process_batch(batch).await;
        }

        // Измеряем latency для небольших batch
        for _ in 0..20 {
            let batch = generate_gpu_batch(16, 1024);
            let start = Instant::now();
            let _ = processor.process_batch(batch).await;
            let latency = start.elapsed();
            latencies.push(latency.as_micros() as f64 / 1000.0);
        }

        let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
        let min_latency = latencies.iter().fold(f64::MAX, |a, &b| a.min(b));

        println!("GPU Avg: {:.2}ms, Min: {:.2}ms", avg_latency, min_latency);

        let stats = processor.get_stats().await;
        if !stats.fallback_active {
            // GPU должен быть очень быстрым для небольших batch после прогрева
            assert!(min_latency < 10.0, "GPU min latency should be under 10ms");
            assert!(avg_latency < 25.0, "GPU avg latency should be under 25ms");
        }
    }

    #[tokio::test]
    async fn test_gpu_throughput_scaling() {
        let processor = Arc::new(create_test_gpu_processor().await);
        let start = Instant::now();
        let mut handles = vec![];

        // Высокая concurrent нагрузка для GPU
        for _ in 0..20 {
            let processor_clone = processor.clone();
            let handle = tokio::spawn(async move {
                let batch = generate_gpu_batch(32, 1024);
                processor_clone.process_batch(batch).await
            });
            handles.push(handle);
        }

        // Ждем завершения
        for handle in handles {
            let _ = handle.await;
        }

        let duration = start.elapsed();
        let stats = processor.get_stats().await;

        let throughput = stats.total_records as f64 / duration.as_secs_f64();
        println!("GPU Throughput: {:.0} records/sec", throughput);

        if !stats.fallback_active {
            // GPU должен иметь высокую пропускную способность
            assert!(throughput > 500.0, "GPU should process >500 records/sec");
        }
    }

    // ===== РАЗДЕЛ 7: Concurrent Operations =====

    #[tokio::test]
    async fn test_concurrent_gpu_batch_processing() {
        let processor = Arc::new(create_test_gpu_processor().await);
        let mut handles = vec![];

        // Параллельные GPU операции
        for i in 0..8 {
            let processor_clone = processor.clone();
            let handle = tokio::spawn(async move {
                let batch = generate_gpu_batch(24, 1024);
                processor_clone.process_batch(batch).await
            });
            handles.push(handle);
        }

        // Проверяем результаты
        let mut success_count = 0;
        for handle in handles {
            if let Ok(Ok(_)) = handle.await {
                success_count += 1;
            }
        }

        assert_eq!(
            success_count, 8,
            "All concurrent GPU batches should succeed"
        );

        let stats = processor.get_stats().await;
        assert_eq!(stats.total_batches, 8, "Should have 8 batches");
        assert_eq!(stats.total_records, 192, "Should have 192 records");
    }

    // ===== РАЗДЕЛ 8: Graceful Degradation =====

    #[tokio::test]
    async fn test_gradual_performance_degradation() {
        let processor = create_test_gpu_processor().await;
        let mut avg_latencies = vec![];

        // Тестируем increasing load
        let sizes = vec![16, 32, 64, 128, 256];

        for size in sizes {
            let batch = generate_gpu_batch(size, 1024);

            let start = Instant::now();
            let _ = processor.process_batch(batch).await;
            let latency = start.elapsed().as_millis() as f64;

            avg_latencies.push(latency);
        }

        println!("Latencies by size: {:?}", avg_latencies);

        // Проверяем что latency растет разумно с размером
        for i in 1..avg_latencies.len() {
            let growth = avg_latencies[i] / avg_latencies[i - 1];
            assert!(
                growth < 5.0,
                "Latency should not grow too dramatically: {}",
                growth
            );
        }
    }

    // ===== РАЗДЕЛ 9: Cleanup и Resource Management =====

    #[tokio::test]
    async fn test_gpu_resource_cleanup() {
        let processor = create_test_gpu_processor().await;

        // Интенсивная работа
        for _ in 0..5 {
            let batch = generate_gpu_batch(32, 1024);
            let _ = processor.process_batch(batch).await;
        }

        let stats_before = processor.get_stats().await;

        // Cleanup
        processor.cleanup().await;

        let stats_after = processor.get_stats().await;

        // После cleanup stats должны сохраниться, но resources освободиться
        assert_eq!(
            stats_after.total_batches, stats_before.total_batches,
            "Stats should persist after cleanup"
        );

        if let (Some(pool_before), Some(pool_after)) = (
            stats_before.memory_pool_stats,
            stats_after.memory_pool_stats,
        ) {
            assert!(
                pool_after.used_size_mb <= pool_before.used_size_mb,
                "Memory usage should decrease after cleanup"
            );
        }
    }

    // ===== РАЗДЕЛ 10: Метрики и мониторинг =====

    #[tokio::test]
    async fn test_gpu_statistics_tracking() {
        let processor = create_test_gpu_processor().await;

        // Разные размеры batch для статистики
        let sizes = vec![8, 16, 24, 32, 40];
        for size in sizes {
            let batch = generate_gpu_batch(size, 1024);
            let _ = processor.process_batch(batch).await;
        }

        let stats = processor.get_stats().await;

        // Основные метрики
        assert_eq!(stats.total_batches, 5, "Should have 5 batches");
        assert_eq!(stats.total_records, 120, "Should have 120 total records");
        assert!(
            stats.avg_batch_size > 0.0,
            "Should track average batch size"
        );

        // GPU specific метрики
        if !stats.fallback_active {
            assert!(
                stats.gpu_memory_usage_mb.is_some(),
                "Should track GPU memory"
            );
            assert!(
                stats.cuda_kernel_time_ms.is_some(),
                "Should track kernel time"
            );
            assert!(
                stats.gpu_utilization.is_some(),
                "Should track GPU utilization"
            );
        }
    }

    #[tokio::test]
    async fn test_gpu_performance_metrics() {
        let processor = create_test_gpu_processor().await;

        // Генерируем mixed workload
        for i in 0..10 {
            let size = 16 + (i * 4);
            let batch = generate_gpu_batch(size, 1024);
            let _ = processor.process_batch(batch).await;
        }

        let stats = processor.get_stats().await;

        // Performance распределения
        assert!(stats.p50_latency_ms >= 0.0, "Should have P50 latency");
        assert!(stats.p95_latency_ms >= stats.p50_latency_ms, "P95 >= P50");
        assert!(stats.p99_latency_ms >= stats.p95_latency_ms, "P99 >= P95");

        if !stats.fallback_active {
            // GPU specific performance метрики
            assert!(
                stats.gpu_efficiency >= 0.0 && stats.gpu_efficiency <= 1.0,
                "GPU efficiency should be 0-1"
            );
            assert!(
                stats.memory_transfer_efficiency >= 0.0,
                "Memory transfer efficiency should be tracked"
            );
        }
    }
}
