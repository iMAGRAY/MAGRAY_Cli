use anyhow::Result;
use memory::{VectorIndexHnswRs, HnswRsConfig};
use std::time::Instant;

/// Тест инкрементальных обновлений HNSW без полной перестройки
#[tokio::main] 
async fn main() -> Result<()> {
    println!("🔄 Тест инкрементальных HNSW обновлений");
    println!("=====================================\n");

    let config = HnswRsConfig {
        dimension: 1024,
        max_elements: 10000, // Большой лимит для тестирования
        max_connections: 24,
        ef_construction: 200, // Уменьшаем для быстрого теста
        ef_search: 50,
        use_parallel: true,
        ..Default::default()
    };

    let index = VectorIndexHnswRs::new(config)?;
    println!("✅ HNSW индекс создан с лимитом 10000 элементов\n");

    // Этап 1: Добавляем первый батч
    println!("🔵 Этап 1: Добавление первого батча (100 векторов)");
    let mut batch1 = Vec::new();
    for i in 0..100 {
        let vector = vec![0.1 + i as f32 * 0.001; 1024];
        batch1.push((format!("doc_{}", i), vector));
    }

    let start = Instant::now();
    index.add_batch(batch1)?;
    let batch1_time = start.elapsed();
    
    println!("  ✅ Первый батч добавлен за {:?}", batch1_time);
    println!("  📊 Текущий размер индекса: {}", index.len());

    // Этап 2: Добавляем второй батч инкрементально  
    println!("\n🟢 Этап 2: Инкрементальное добавление второго батча (200 векторов)");
    let mut batch2 = Vec::new();
    for i in 100..300 {
        let vector = vec![0.2 + i as f32 * 0.001; 1024];
        batch2.push((format!("doc_{}", i), vector));
    }

    let start = Instant::now();
    index.add_batch(batch2)?;
    let batch2_time = start.elapsed();
    
    println!("  ✅ Второй батч добавлен за {:?}", batch2_time);
    println!("  📊 Текущий размер индекса: {}", index.len());

    // Этап 3: Добавляем третий батч
    println!("\n🟡 Этап 3: Добавление третьего батча (500 векторов)");
    let mut batch3 = Vec::new();
    for i in 300..800 {
        let vector = vec![0.3 + i as f32 * 0.001; 1024];
        batch3.push((format!("doc_{}", i), vector));
    }

    let start = Instant::now();
    index.add_batch(batch3)?;
    let batch3_time = start.elapsed();
    
    println!("  ✅ Третий батч добавлен за {:?}", batch3_time);
    println!("  📊 Текущий размер индекса: {}", index.len());

    // Этап 4: Тестируем поиск после инкрементальных обновлений
    println!("\n🔍 Этап 4: Тестирование поиска по всем добавленным векторам");
    
    let test_queries = vec![
        vec![0.15; 1024], // Должен найти из первого батча
        vec![0.25; 1024], // Должен найти из второго батча  
        vec![0.35; 1024], // Должен найти из третьего батча
    ];

    for (i, query) in test_queries.iter().enumerate() {
        let start = Instant::now();
        let results = index.search(query, 5)?;
        let search_time = start.elapsed();
        
        println!("  🔎 Запрос {}: найдено {} результатов за {:?}", 
                 i + 1, results.len(), search_time);
        
        if !results.is_empty() {
            let (best_id, best_score) = &results[0];
            println!("    🎯 Лучший результат: {} (score: {:.4})", best_id, best_score);
        }
    }

    // Этап 5: Проверяем статистику производительности
    println!("\n📊 Этап 5: Статистика производительности");
    let stats = index.stats();
    
    println!("  📈 Статистика HNSW:");
    println!("    Векторов в индексе: {}", index.len());
    println!("    Операций вставки: {}", stats.total_insertions.load(std::sync::atomic::Ordering::Relaxed));
    println!("    Операций поиска: {}", stats.total_searches.load(std::sync::atomic::Ordering::Relaxed));
    println!("    Среднее время вставки: {:.2} мс", stats.avg_insertion_time_ms());
    println!("    Среднее время поиска: {:.2} мс", stats.avg_search_time_ms());

    // Анализ временных затрат
    println!("\n⏱️  Анализ времени:");
    println!("  Батч 1 (100 элементов): {:?}", batch1_time);
    println!("  Батч 2 (200 элементов): {:?}", batch2_time);  
    println!("  Батч 3 (500 элементов): {:?}", batch3_time);
    
    let total_time = batch1_time + batch2_time + batch3_time;
    let avg_per_element = total_time.as_micros() / 800; // 800 элементов всего
    
    println!("  Общее время: {:?}", total_time);
    println!("  Среднее время на элемент: {} мкс", avg_per_element);

    // Проверяем, что инкрементальные обновления работают эффективно
    if batch2_time < batch1_time * 3 && batch3_time < batch1_time * 6 {
        println!("\n✅ УСПЕХ: Инкрементальные обновления работают эффективно!");
        println!("   Время растет не пропорционально (избегаем full rebuild)");
    } else {
        println!("\n⚠️  ВНИМАНИЕ: Возможно происходит full rebuild при батчевых операциях");
    }

    println!("\n🏆 Тест инкрементальных обновлений завершен!");
    Ok(())
}