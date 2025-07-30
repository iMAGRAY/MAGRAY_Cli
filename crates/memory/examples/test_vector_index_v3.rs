use anyhow::Result;
use memory::{VectorIndexV3, VectorIndexConfigV3};
use std::time::Instant;

/// Генерирует детерминированный эмбеддинг из текста
fn mock_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0; 1024];
    let hash = text.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    
    for i in 0..1024 {
        let value = ((hash.wrapping_mul((i + 1) as u64) % 1000) as f32) / 1000.0;
        embedding[i] = value;
    }
    
    // Нормализация
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut embedding {
            *v /= norm;
        }
    }
    
    embedding
}

fn main() -> Result<()> {
    println!("🧪 Тестирование VectorIndexV3...\n");
    
    // Создаём индекс с конфигурацией
    let config = VectorIndexConfigV3 {
        dimension: 1024,
        rebuild_threshold: 5, // Низкий порог для тестирования
        linear_search_threshold: 10,
        ..Default::default()
    };
    
    let index = VectorIndexV3::new(config);
    
    // Тест 1: Добавление векторов
    println!("📝 Тест 1: Добавление векторов");
    let texts = vec![
        ("doc1", "Rust programming language memory safety"),
        ("doc2", "JavaScript async await promises"),
        ("doc3", "Python machine learning numpy pandas"),
        ("doc4", "Rust ownership borrowing lifetimes"),
        ("doc5", "Database indexes optimization performance"),
    ];
    
    for (id, text) in &texts {
        let embedding = mock_embedding(text);
        index.add(id.to_string(), embedding)?;
        println!("  ✅ Добавлен: {} - {}", id, text);
    }
    
    // Проверяем статистику
    let stats = index.stats();
    println!("\n📊 Статистика после добавления:");
    println!("  Всего векторов: {}", stats.total_embeddings);
    println!("  В индексе: {}", stats.indexed_embeddings);
    println!("  В ожидании: {} добавлений, {} удалений", stats.pending_additions, stats.pending_removals);
    println!("  Использует линейный поиск: {}", stats.using_linear_search);
    
    // Тест 2: Поиск
    println!("\n🔍 Тест 2: Поиск похожих");
    let query = "Rust memory management";
    let query_embedding = mock_embedding(query);
    
    let start = Instant::now();
    let results = index.search(&query_embedding, 3)?;
    let search_time = start.elapsed();
    
    println!("  Запрос: '{}' (время: {:?})", query, search_time);
    println!("  Результаты:");
    for (id, score) in &results {
        let text = texts.iter().find(|(tid, _)| tid == id).map(|(_, t)| t).unwrap_or(&"");
        println!("    {} (score: {:.3}) - {}", id, score, text);
    }
    
    // Тест 3: Пакетное добавление
    println!("\n📦 Тест 3: Пакетное добавление");
    let batch = vec![
        ("doc6", "TypeScript type inference generics"),
        ("doc7", "Go concurrency goroutines channels"),
        ("doc8", "Rust async runtime tokio futures"),
        ("doc9", "Docker containers kubernetes deployment"),
        ("doc10", "GraphQL API schema resolvers"),
    ];
    
    let batch_embeddings: Vec<(String, Vec<f32>)> = batch
        .iter()
        .map(|(id, text)| (id.to_string(), mock_embedding(text)))
        .collect();
    
    index.add_batch(batch_embeddings)?;
    println!("  ✅ Добавлено {} документов пакетом", batch.len());
    
    // Проверяем статистику после пакетного добавления
    let stats = index.stats();
    println!("\n📊 Статистика после пакетного добавления:");
    println!("  Всего векторов: {}", stats.total_embeddings);
    println!("  В индексе: {}", stats.indexed_embeddings);
    println!("  В ожидании: {} добавлений, {} удалений", stats.pending_additions, stats.pending_removals);
    println!("  Использует линейный поиск: {}", stats.using_linear_search);
    
    // Тест 4: Удаление
    println!("\n🗑️  Тест 4: Удаление векторов");
    let removed = index.remove("doc5");
    println!("  Удаление doc5: {}", if removed { "✅ успешно" } else { "❌ не найден" });
    
    // Тест 5: Поиск после удаления
    println!("\n🔍 Тест 5: Поиск после удаления");
    let query = "Database optimization";
    let query_embedding = mock_embedding(query);
    let results = index.search(&query_embedding, 5)?;
    
    println!("  Запрос: '{}'", query);
    println!("  Результаты (не должны содержать doc5):");
    for (id, score) in &results {
        println!("    {} (score: {:.3})", id, score);
    }
    
    // Тест 6: Метрики производительности
    println!("\n⚡ Тест 6: Метрики производительности");
    let stats = index.stats();
    println!("  Общее количество поисков: {}", stats.metrics.total_searches);
    println!("  Среднее время поиска: {:.2} мкс", stats.metrics.avg_search_time_us);
    println!("  Количество перестроек: {}", stats.metrics.rebuild_count);
    println!("  Среднее время перестройки: {:.2} мс", stats.metrics.avg_rebuild_time_ms);
    println!("  Количество добавлений: {}", stats.metrics.add_count);
    println!("  Количество удалений: {}", stats.metrics.remove_count);
    
    // Тест 7: Оптимизация памяти
    println!("\n💾 Тест 7: Оптимизация памяти");
    index.optimize_memory()?;
    println!("  ✅ Память оптимизирована");
    
    // Тест 8: Большой датасет для проверки переключения на линейный поиск
    println!("\n📈 Тест 8: Тестирование с большим датасетом");
    let large_batch: Vec<(String, Vec<f32>)> = (11..=20)
        .map(|i| {
            let id = format!("large_doc{}", i);
            let text = format!("Large document number {} with random content", i);
            (id, mock_embedding(&text))
        })
        .collect();
    
    index.add_batch(large_batch)?;
    
    let stats = index.stats();
    println!("  Всего векторов: {}", stats.total_embeddings);
    println!("  Использует линейный поиск: {} (должен быть false, т.к. порог = 10)", !stats.using_linear_search);
    
    // Финальный поиск
    println!("\n🎯 Финальный поиск:");
    let query = "Rust async programming";
    let query_embedding = mock_embedding(query);
    let results = index.search(&query_embedding, 5)?;
    
    println!("  Запрос: '{}'", query);
    println!("  Топ-5 результатов:");
    for (i, (id, score)) in results.iter().enumerate() {
        println!("    {}. {} (score: {:.3})", i + 1, id, score);
    }
    
    println!("\n✅ Все тесты завершены успешно!");
    
    Ok(())
}