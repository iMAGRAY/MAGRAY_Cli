use anyhow::Result;
use memory::{
    VectorIndexHnswReal, VectorIndexHnswSimple, VectorIndexHnswLib,
    HnswRealConfig, HnswSimpleConfig, HnswLibConfig
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
async fn test_hnsw_comparison_small_dataset() -> Result<()> {
    println!("=== Сравнение HNSW реализаций на малом датасете (100 векторов) ===\n");
    
    let dimension = 128;
    let dataset = generate_dataset(100, dimension);
    let query = generate_test_vector(dimension, 50.5);
    
    // Тест HnswReal (простейший)
    println!("🔵 Тестируем VectorIndexHnswReal (простейший):");
    let config_real = HnswRealConfig {
        dimension,
        ..Default::default()
    };
    let index_real = VectorIndexHnswReal::new(config_real);
    
    let start = Instant::now();
    index_real.add_batch(dataset.clone())?;
    let build_time_real = start.elapsed();
    
    let start = Instant::now();
    let results_real = index_real.search(&query, 10)?;
    let search_time_real = start.elapsed();
    
    let stats_real = index_real.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_real);
    println!("  ✅ Поиск top-10: {:?}", search_time_real);
    println!("  📊 Статистика: {} векторов, {:.2} μs средний поиск", 
             stats_real.total_vectors, stats_real.avg_search_time_us);
    println!("  📝 Найдено результатов: {}", results_real.len());
    println!();
    
    // Тест HnswSimple (с оптимизациями)
    println!("🟡 Тестируем VectorIndexHnswSimple (с оптимизациями):");
    let config_simple = HnswSimpleConfig {
        dimension,
        enable_caching: true,
        linear_search_threshold: 50,
        ..Default::default()
    };
    let index_simple = VectorIndexHnswSimple::new(config_simple);
    
    let start = Instant::now();
    index_simple.add_batch(dataset.clone())?;
    let build_time_simple = start.elapsed();
    
    let start = Instant::now();
    let results_simple = index_simple.search(&query, 10)?;
    let search_time_simple = start.elapsed();
    
    let stats_simple = index_simple.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_simple);
    println!("  ✅ Поиск top-10: {:?}", search_time_simple);
    println!("  📊 Статистика: {} векторов, {:.2} μs средний поиск", 
             stats_simple.total_vectors, stats_simple.avg_search_time_us);
    println!("  📝 Найдено результатов: {}", results_simple.len());
    println!();
    
    // Тест HnswLib (профессиональная библиотека)
    println!("🟢 Тестируем VectorIndexHnswLib (hnswlib-rs профессиональная библиотека):");
    let config_lib = HnswLibConfig {
        dimension,
        max_connections: 8,
        ef_construction: 100,
        ef_search: 50,
        max_elements: 1000,
        use_parallel: false, // Для малого датасета
        ..Default::default()
    };
    let index_lib = VectorIndexHnswLib::new(config_lib)?;
    
    let start = Instant::now();
    index_lib.add_batch(dataset.clone())?;
    let build_time_lib = start.elapsed();
    
    let start = Instant::now();
    let results_lib = index_lib.search(&query, 10)?;
    let search_time_lib = start.elapsed();
    
    let stats_lib = index_lib.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_lib);
    println!("  ✅ Поиск top-10: {:?}", search_time_lib);
    println!("  📊 Статистика:");
    println!("    - Элементов: {} (активных: {}, удалённых: {})", 
             stats_lib.total_elements, stats_lib.active_elements, stats_lib.deleted_elements);
    println!("    - Соединений: {} (среднее: {:.1})", 
             stats_lib.total_connections, stats_lib.avg_connections);
    println!("    - Средний поиск: {:.2} μs", stats_lib.avg_search_time_us);
    println!("    - Средняя вставка: {:.2} μs", stats_lib.avg_add_time_us);
    println!("    - Всего поисков: {}", stats_lib.total_searches);
    println!("    - Всего вставок: {}", stats_lib.total_additions);
    println!("  📝 Найдено результатов: {}", results_lib.len());
    println!();
    
    // Сравнение результатов
    println!("📈 СРАВНЕНИЕ ПРОИЗВОДИТЕЛЬНОСТИ:");
    println!("  Построение индекса:");
    println!("    Real (линейный): {:?}", build_time_real);
    println!("    Simple (кэш+SIMD): {:?}", build_time_simple);
    println!("    HnswLib (проф. библиотека): {:?}", build_time_lib);
    println!();
    println!("  Поиск:");
    println!("    Real (O(n)): {:?}", search_time_real);
    println!("    Simple (O(n)+кэш): {:?}", search_time_simple);
    println!("    HnswLib (O(log n)): {:?}", search_time_lib);
    println!();
    
    // Определяем победителя
    println!("🏆 РЕЗУЛЬТАТЫ:");
    let lib_search_us = search_time_lib.as_micros();
    let real_search_us = search_time_real.as_micros();
    let simple_search_us = search_time_simple.as_micros();
    
    if lib_search_us < real_search_us && lib_search_us < simple_search_us {
        println!("  ✅ HnswLib - САМЫЙ БЫСТРЫЙ для поиска!");
        println!("    Быстрее Real в {:.1}x раз", real_search_us as f64 / lib_search_us as f64);
        println!("    Быстрее Simple в {:.1}x раз", simple_search_us as f64 / lib_search_us as f64);
    }
    
    // Проверяем качество результатов (все должны найти 10 результатов)
    assert_eq!(results_real.len(), 10);
    assert_eq!(results_simple.len(), 10);
    assert_eq!(results_lib.len(), 10);
    
    Ok(())
}

#[tokio::test]
async fn test_hnsw_comparison_large_dataset() -> Result<()> {
    println!("=== Сравнение HNSW реализаций на большом датасете (1000 векторов) ===\n");
    
    let dimension = 512;
    let dataset = generate_dataset(1000, dimension);
    let query = generate_test_vector(dimension, 500.5);
    
    // Тест HnswReal
    println!("🔵 Тестируем VectorIndexHnswReal:");
    let config_real = HnswRealConfig {
        dimension,
        ..Default::default()
    };
    let index_real = VectorIndexHnswReal::new(config_real);
    
    let start = Instant::now();
    index_real.add_batch(dataset.clone())?;
    let build_time_real = start.elapsed();
    
    // Несколько поисков для получения средней статистики
    let mut total_search_time = std::time::Duration::ZERO;
    for i in 0..10 {
        let query_i = generate_test_vector(dimension, 500.0 + i as f32 * 0.1);
        let start = Instant::now();
        let _results = index_real.search(&query_i, 5)?;
        total_search_time += start.elapsed();
    }
    let avg_search_time_real = total_search_time / 10;
    
    println!("  ✅ Построение индекса: {:?}", build_time_real);
    println!("  ✅ Средний поиск top-5: {:?}", avg_search_time_real);
    println!();
    
    // Тест HnswSimple
    println!("🟡 Тестируем VectorIndexHnswSimple:");
    let config_simple = HnswSimpleConfig {
        dimension,
        enable_caching: true,
        linear_search_threshold: 500,
        ..Default::default()
    };
    let index_simple = VectorIndexHnswSimple::new(config_simple);
    
    let start = Instant::now();
    index_simple.add_batch(dataset.clone())?;
    let build_time_simple = start.elapsed();
    
    let mut total_search_time = std::time::Duration::ZERO;
    for i in 0..10 {
        let query_i = generate_test_vector(dimension, 500.0 + i as f32 * 0.1);
        let start = Instant::now();
        let _results = index_simple.search(&query_i, 5)?;
        total_search_time += start.elapsed();
    }
    let avg_search_time_simple = total_search_time / 10;
    
    println!("  ✅ Построение индекса: {:?}", build_time_simple);
    println!("  ✅ Средний поиск top-5: {:?}", avg_search_time_simple);
    println!();
    
    // Тест HnswLib
    println!("🟢 Тестируем VectorIndexHnswLib:");
    let config_lib = HnswLibConfig {
        dimension,
        max_connections: 16,
        ef_construction: 200,
        ef_search: 100,
        max_elements: 2000,
        use_parallel: true,
        ..Default::default()
    };
    let index_lib = VectorIndexHnswLib::new(config_lib)?;
    
    let start = Instant::now();
    index_lib.add_batch(dataset.clone())?;
    let build_time_lib = start.elapsed();
    
    let mut total_search_time = std::time::Duration::ZERO;
    for i in 0..10 {
        let query_i = generate_test_vector(dimension, 500.0 + i as f32 * 0.1);
        let start = Instant::now();
        let _results = index_lib.search(&query_i, 5)?;
        total_search_time += start.elapsed();
    }
    let avg_search_time_lib = total_search_time / 10;
    
    let stats_lib = index_lib.stats();
    println!("  ✅ Построение индекса: {:?}", build_time_lib);
    println!("  ✅ Средний поиск top-5: {:?}", avg_search_time_lib);
    println!("  📊 HnswLib структура:");
    println!("    - Элементов: {} (активных: {})", stats_lib.total_elements, stats_lib.active_elements);
    println!("    - Среднее соединений: {:.1}", stats_lib.avg_connections);
    println!("    - Средний поиск: {:.2} μs", stats_lib.avg_search_time_us);
    println!("    - Всего поисков: {}", stats_lib.total_searches);
    println!();
    
    // Итоговое сравнение
    println!("🏆 ИТОГОВОЕ СРАВНЕНИЕ (1000 векторов):");
    println!("  Построение:");
    println!("    Real (линейный): {:?}", build_time_real);
    println!("    Simple (кэш+SIMD): {:?}", build_time_simple);
    println!("    HnswLib (профессиональный): {:?}", build_time_lib);
    println!();
    println!("  Поиск:");
    println!("    Real (O(n)): {:?}", avg_search_time_real);
    println!("    Simple (O(n)+кэш): {:?}", avg_search_time_simple);
    println!("    HnswLib (O(log n)): {:?}", avg_search_time_lib);
    println!();
    
    // На большом датасете HnswLib должен быть быстрее при поиске
    println!("💡 Эффективность:");
    let real_search_ms = avg_search_time_real.as_micros() as f64 / 1000.0;
    let simple_search_ms = avg_search_time_simple.as_micros() as f64 / 1000.0;
    let lib_search_ms = avg_search_time_lib.as_micros() as f64 / 1000.0;
    
    println!("  Real: {:.2} ms поиск", real_search_ms);
    println!("  Simple: {:.2} ms поиск", simple_search_ms);
    println!("  HnswLib: {:.2} ms поиск", lib_search_ms);
    
    if lib_search_ms < real_search_ms {
        println!("  ✅ HnswLib быстрее линейного поиска в {:.1}x раз", 
                 real_search_ms / lib_search_ms);
    }
    
    if lib_search_ms < simple_search_ms {
        println!("  ✅ HnswLib быстрее Simple в {:.1}x раз", 
                 simple_search_ms / lib_search_ms);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_hnsw_deletion_comparison() -> Result<()> {
    println!("=== Сравнение удаления в HNSW реализациях ===\n");
    
    let dimension = 64;
    let dataset = generate_dataset(50, dimension);
    
    // Тест удаления в Real
    let index_real = VectorIndexHnswReal::new(HnswRealConfig {
        dimension,
        ..Default::default()
    });
    index_real.add_batch(dataset.clone())?;
    
    println!("🔵 VectorIndexHnswReal:");
    let before_stats = index_real.stats();
    println!("  До удаления: {} векторов", before_stats.total_vectors);
    
    assert!(index_real.remove("vec_10"));
    assert!(index_real.remove("vec_20"));
    assert!(!index_real.remove("vec_999")); // не существует
    
    let after_stats = index_real.stats();
    println!("  После удаления: {} векторов", after_stats.total_vectors);
    println!("  ✅ Физическое удаление работает\n");
    
    // Тест удаления в HnswLib
    let index_lib = VectorIndexHnswLib::new(HnswLibConfig {
        dimension,
        max_elements: 100,
        ..Default::default()
    })?;
    index_lib.add_batch(dataset.clone())?;
    
    println!("🟢 VectorIndexHnswLib:");
    let before_stats = index_lib.stats();
    println!("  До удаления: {} элементов ({} активных)", 
             before_stats.total_elements, before_stats.active_elements);
    
    assert!(index_lib.remove("vec_10"));
    assert!(index_lib.remove("vec_20"));
    assert!(!index_lib.remove("vec_999")); // не существует
    
    let after_stats = index_lib.stats();
    println!("  После удаления: {} элементов ({} активных, {} удалённых)", 
             after_stats.total_elements, after_stats.active_elements, after_stats.deleted_elements);
    println!("  ✅ Логическое удаление работает");
    
    // Проверяем, что удалённые не возвращаются в поиске
    let query = generate_test_vector(dimension, 10.0);
    let results = index_lib.search(&query, 50)?;
    
    for (id, _) in results {
        assert_ne!(id, "vec_10", "Удалённый вектор не должен появляться в результатах");
        assert_ne!(id, "vec_20", "Удалённый вектор не должен появляться в результатах");
    }
    println!("  ✅ Удалённые векторы не появляются в поиске\n");
    
    Ok(())
}