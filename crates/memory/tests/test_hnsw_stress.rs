use anyhow::Result;
use memory::*;
use std::time::Instant;
use tokio;

/// Stress-тесты для HNSW векторного индекса
#[tokio::test]
async fn test_hnsw_scaling_performance() -> Result<()> {
    println!("🔥 HNSW Stress Test: Scaling Performance");
    
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 32,      // Более высокие параметры для production
        ef_construction: 600,     // Высокое качество построения
        ef_search: 200,           // Баланс скорость/точность
        max_elements: 50_000,     // 50K vectors
        max_layers: 16,
        use_parallel: true,
    };
    
    let index = VectorIndexHnswRs::new(config)?;
    
    // Тест 1: Последовательная вставка с измерением деградации
    println!("Phase 1: Sequential insertion performance");
    let mut insertion_times = Vec::new();
    
    for batch in 0..50 {
        let batch_start = Instant::now();
        let mut batch_vectors = Vec::new();
        
        for i in 0..100 {
            let vector_id = format!("seq_{}_{}", batch, i);
            let vector = generate_realistic_vector(1024, batch * 100 + i);
            batch_vectors.push((vector_id, vector));
        }
        
        index.add_batch(batch_vectors)?;
        let batch_duration = batch_start.elapsed();
        insertion_times.push(batch_duration.as_millis());
        
        if batch % 10 == 0 {
            println!("  Batch {}: {} vectors, {}ms (total: {})", 
                     batch, index.len(), batch_duration.as_millis(), index.len());
        }
    }
    
    // Анализ деградации производительности
    let early_avg = insertion_times[0..10].iter().sum::<u128>() / 10;
    let late_avg = insertion_times[40..50].iter().sum::<u128>() / 10;
    let degradation = (late_avg as f64 / early_avg as f64 - 1.0) * 100.0;
    
    println!("📊 Insertion performance analysis:");
    println!("  Early batches avg: {}ms", early_avg);
    println!("  Late batches avg: {}ms", late_avg);
    println!("  Performance degradation: {:.1}%", degradation);
    
    assert!(degradation < 200.0, "HNSW performance degraded too much: {:.1}%", degradation);
    
    // Тест 2: Search performance под разными нагрузками
    println!("Phase 2: Search performance scaling");
    
    let search_configurations = [
        (1, "single"),
        (10, "small_batch"),
        (50, "medium_batch"),
        (100, "large_batch"),
    ];
    
    for (batch_size, name) in search_configurations {
        let search_start = Instant::now();
        let mut total_results = 0;
        
        for _ in 0..batch_size {
            let query_vector = generate_realistic_vector(1024, rand::random::<usize>());
            let results = index.search(&query_vector, 20)?;
            total_results += results.len();
        }
        
        let search_duration = search_start.elapsed();
        let ops_per_sec = batch_size as f64 / search_duration.as_secs_f64();
        
        println!("  {}: {} searches in {:?} ({:.1} ops/sec, {} results)", 
                 name, batch_size, search_duration, ops_per_sec, total_results);
        
        // Проверяем что поиск остается быстрым даже при большом индексе
        assert!(search_duration.as_millis() < batch_size as u128 * 10, 
                "Search too slow for batch size {}: {:?}", batch_size, search_duration);
    }
    
    // Тест 3: Parallel search stress test
    println!("Phase 3: Parallel search stress test");
    
    let parallel_start = Instant::now();
    let mut handles = Vec::new();
    
    for worker_id in 0..8 {
        let index_clone = &index; // HNSW поддерживает concurrent reads
        let handle = tokio::spawn(async move {
            let mut worker_results = 0;
            for query_id in 0..25 {
                let query_vector = generate_realistic_vector(1024, worker_id * 1000 + query_id);
                if let Ok(results) = index_clone.search(&query_vector, 15) {
                    worker_results += results.len();
                }
            }
            worker_results
        });
        handles.push(handle);
    }
    
    let mut parallel_results = 0;
    for handle in handles {
        parallel_results += handle.await?;
    }
    
    let parallel_duration = parallel_start.elapsed();
    let parallel_ops_per_sec = 200.0 / parallel_duration.as_secs_f64(); // 8 workers * 25 queries
    
    println!("  8 parallel workers, 200 total searches in {:?} ({:.1} ops/sec)", 
             parallel_duration, parallel_ops_per_sec);
    
    // Проверяем что parallel search эффективен
    assert!(parallel_ops_per_sec > 50.0, "Parallel search too slow: {:.1} ops/sec", parallel_ops_per_sec);
    
    // Тест 4: Memory usage и индекс качество
    println!("Phase 4: Quality and memory analysis");
    
    let stats = index.stats();
    println!("📈 Final HNSW statistics:");
    println!("  Total vectors: {}", stats.vector_count());
    println!("  Average search time: {:.2}ms", stats.avg_search_time_ms());
    println!("  Average insertion time: {:.2}ms", stats.avg_insertion_time_ms());
    println!("  Search throughput: {:.1} ops/sec", stats.search_throughput_per_sec());
    
    // Quality test: поиск по известным векторам должен находить их в топе
    let mut quality_scores = Vec::new();
    for test_id in 0..20 {
        let known_vector = generate_realistic_vector(1024, test_id);
        let known_id = format!("seq_0_{}", test_id); // Из первого batch'а
        
        let results = index.search(&known_vector, 10)?;
        if let Some(position) = results.iter().position(|(id, _)| id == &known_id) {
            quality_scores.push(position);
        }
    }
    
    let avg_quality = quality_scores.iter().sum::<usize>() as f64 / quality_scores.len() as f64;
    println!("  Average known vector position: {:.2} (lower is better)", avg_quality);
    
    assert!(avg_quality < 2.0, "HNSW index quality too poor: {:.2}", avg_quality);
    assert!(stats.avg_search_time_ms() < 10.0, "Search too slow: {:.2}ms", stats.avg_search_time_ms());
    
    println!("🎉 HNSW stress test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_hnsw_edge_cases() -> Result<()> {
    println!("🔍 HNSW Edge Cases Test");
    
    let config = HnswRsConfig::default();
    let index = VectorIndexHnswRs::new(config)?;
    
    // Edge case 1: Identical vectors
    println!("Testing identical vectors...");
    let identical_vector = vec![0.5; 1024];
    for i in 0..10 {
        index.add(format!("identical_{}", i), identical_vector.clone())?;
    }
    
    let search_results = index.search(&identical_vector, 5)?;
    assert_eq!(search_results.len(), 5, "Should find 5 identical vectors");
    
    // Edge case 2: Very sparse vectors
    println!("Testing sparse vectors...");
    let mut sparse_vector = vec![0.0; 1024];
    sparse_vector[0] = 1.0;
    sparse_vector[512] = 1.0;
    sparse_vector[1023] = 1.0;
    
    index.add("sparse_test".to_string(), sparse_vector.clone())?;
    let sparse_results = index.search(&sparse_vector, 3)?;
    assert!(!sparse_results.is_empty(), "Should find sparse vector");
    
    // Edge case 3: Очень большие значения
    println!("Testing large magnitude vectors...");
    let large_vector = vec![1000.0; 1024];
    index.add("large_test".to_string(), large_vector.clone())?;
    let large_results = index.search(&large_vector, 3)?;
    assert!(!large_results.is_empty(), "Should find large magnitude vector");
    
    // Edge case 4: Negative values
    println!("Testing negative vectors...");
    let negative_vector = vec![-0.5; 1024];
    index.add("negative_test".to_string(), negative_vector.clone())?;
    let negative_results = index.search(&negative_vector, 3)?;
    assert!(!negative_results.is_empty(), "Should find negative vector");
    
    // Edge case 5: Mixed positive/negative
    println!("Testing mixed sign vectors...");
    let mut mixed_vector = vec![0.0; 1024];
    for i in 0..1024 {
        mixed_vector[i] = if i % 2 == 0 { 1.0 } else { -1.0 };
    }
    index.add("mixed_test".to_string(), mixed_vector.clone())?;
    let mixed_results = index.search(&mixed_vector, 3)?;
    assert!(!mixed_results.is_empty(), "Should find mixed sign vector");
    
    println!("✅ All edge cases handled correctly");
    Ok(())
}

/// Генерирует реалистичный вектор для тестирования
fn generate_realistic_vector(dim: usize, seed: usize) -> Vec<f32> {
    let mut vector = Vec::with_capacity(dim);
    let mut rng_state = seed as u64;
    
    for i in 0..dim {
        // Простой LCG для воспроизводимости
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let normalized = (rng_state as f32) / (u64::MAX as f32);
        
        // Реалистичное распределение: большинство значений около 0, некоторые выбросы
        let value = if i % 17 == 0 {
            normalized * 2.0 - 1.0  // Выбросы в диапазоне [-1, 1]
        } else {
            (normalized - 0.5) * 0.4  // Основные значения в диапазоне [-0.2, 0.2]
        };
        
        vector.push(value);
    }
    
    // Нормализация вектора для реалистичности
    let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for v in &mut vector {
            *v /= magnitude;
        }
    }
    
    vector
}