use anyhow::Result;
use memory::{
    VectorIndexHnswRs, HnswRsConfig
};
use std::time::Instant;

fn generate_test_vector(dim: usize, seed: f32) -> Vec<f32> {
    (0..dim).map(|i| ((i as f32 + seed) * 0.1).sin()).collect()
}

fn generate_dataset(count: usize, dim: usize) -> Vec<(String, Vec<f32>)> {
    (0..count)
        .map(|i| (format!("vec_{}", i), generate_test_vector(dim, i as f32)))
        .collect()
}

#[tokio::test]
async fn test_hnsw_performance() -> Result<()> {
    println!("=== Тестирование производительности HNSW индекса ===\n");
    
    let dimension = 1024;
    let dataset = generate_dataset(100, dimension);
    let query = generate_test_vector(dimension, 50.5);
    
    // Тест VectorIndexHnswRs с разными конфигурациями
    println!("🔵 Тестируем VectorIndexHnswRs (default config):");
    let config_default = HnswRsConfig::default();
    let index_default = VectorIndexHnswRs::new(config_default)?;
    
    let start = Instant::now();
    index_default.add_batch(dataset.clone())?;
    let build_time_default = start.elapsed();
    
    let start = Instant::now();
    let results_default = index_default.search(&query, 10)?;
    let search_time_default = start.elapsed();
    
    let stats_default = index_default.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_default);
    println!("  ✅ Поиск top-10: {:?}", search_time_default);
    println!("  📊 Статистика: {} векторов, использование памяти: {} KB", 
             stats_default.vector_count(), stats_default.memory_usage_kb());
    println!("  📝 Найдено результатов: {}", results_default.len());
    println!();
    
    // Тест с оптимизированной конфигурацией
    println!("🟡 Тестируем VectorIndexHnswRs (optimized config):");
    let config_optimized = HnswRsConfig {
        max_elements: 10000,
        max_connections: 32,  // Больше связей для лучшего качества
        ef_construction: 400,  // Лучше строить граф
        use_parallel: true,
        ..Default::default()
    };
    let index_optimized = VectorIndexHnswRs::new(config_optimized)?;
    
    let start = Instant::now();
    index_optimized.add_batch(dataset.clone())?;
    let build_time_optimized = start.elapsed();
    
    let start = Instant::now();
    let results_optimized = index_optimized.search(&query, 10)?;
    let search_time_optimized = start.elapsed();
    
    let stats_optimized = index_optimized.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_optimized);
    println!("  ✅ Поиск top-10: {:?}", search_time_optimized);
    println!("  📊 Статистика: {} векторов, использование памяти: {} KB", 
             stats_optimized.vector_count(), stats_optimized.memory_usage_kb());
    println!("  📝 Найдено результатов: {}", results_optimized.len());
    println!();
    
    // Сравнение результатов
    println!("📊 Сравнение производительности:");
    println!("  Построение: default {:?} vs optimized {:?} ({:.1}x)", 
             build_time_default, build_time_optimized,
             build_time_default.as_secs_f64() / build_time_optimized.as_secs_f64());
    println!("  Поиск: default {:?} vs optimized {:?} ({:.1}x)", 
             search_time_default, search_time_optimized,
             search_time_default.as_secs_f64() / search_time_optimized.as_secs_f64());
    
    // Проверяем качество результатов
    let mut same_results = 0;
    for (i, (id_def, _)) in results_default.iter().enumerate() {
        if i < results_optimized.len() {
            let (id_opt, _) = &results_optimized[i];
            if id_def == id_opt {
                same_results += 1;
            }
        }
    }
    println!("  Совпадение top-10: {}/10 ({:.0}%)", same_results, same_results as f32 * 10.0);
    
    Ok(())
}

#[tokio::test]
async fn test_hnsw_large_dataset() -> Result<()> {
    println!("=== Тестирование HNSW на большом датасете (10K векторов) ===\n");
    
    let dimension = 1024;  // Размерность для тестов
    let dataset = generate_dataset(10_000, dimension);
    let queries: Vec<_> = (0..100).map(|i| generate_test_vector(dimension, i as f32 * 100.0)).collect();
    
    let config = HnswRsConfig {
        max_elements: 15000,
        max_connections: 16,
        ef_construction: 200,
        use_parallel: true,
        ..Default::default()
    };
    let index = VectorIndexHnswRs::new(config)?;
    
    // Построение индекса
    let start = Instant::now();
    index.add_batch(dataset)?;
    let build_time = start.elapsed();
    println!("✅ Построение индекса 10K векторов: {:?}", build_time);
    
    // Batch поиск
    let start = Instant::now();
    let mut total_results = 0;
    for query in &queries {
        let results = index.search(query, 10)?;
        total_results += results.len();
    }
    let search_time = start.elapsed();
    
    println!("✅ Поиск 100 запросов: {:?} (средний: {:.2} ms)", 
             search_time, search_time.as_millis() as f64 / 100.0);
    println!("📊 Всего найдено результатов: {}", total_results);
    
    let stats = index.stats();
    println!("📈 Финальная статистика:");
    println!("  - Векторов в индексе: {}", stats.vector_count());
    println!("  - Использование памяти: {:.1} MB", stats.memory_usage_kb() as f64 / 1024.0);
    println!("  - Среднее время поиска: {:.1} μs", 
             search_time.as_micros() as f64 / queries.len() as f64);
    
    Ok(())
}