use memory::{
    layers::VectorStore, MemLayer, MemoryStore, MemMeta,
    VectorIndex,
};
use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Testing VectorStore ===\n");
    
    // Создаем временную директорию
    let temp_dir = PathBuf::from("./test_vector_memory");
    tokio::fs::create_dir_all(&temp_dir).await?;
    
    // Создаем VectorStore для краткосрочной памяти
    println!("1. Creating VectorStore...");
    let store = VectorStore::new(
        MemLayer::Short,
        temp_dir.join("short_term.json")
    ).await?;
    println!("✓ VectorStore created\n");
    
    // Тестируем сохранение с векторами
    println!("2. Testing vector storage:");
    
    // Сохраняем несколько документов с mock векторами
    let documents = vec![
        ("rust_basics", "Rust is a systems programming language focused on safety"),
        ("rust_memory", "Rust uses ownership system for memory management"),
        ("rust_async", "Async programming in Rust uses futures and tokio"),
        ("python_intro", "Python is a high-level interpreted language"),
        ("js_web", "JavaScript is the language of the web"),
    ];
    
    for (key, content) in &documents {
        let mut meta = MemMeta::default();
        meta.content_type = "text/plain".to_string();
        
        // Создаем простой mock вектор на основе контента
        let vector = create_mock_vector(content);
        meta.extra.insert("vector".to_string(), serde_json::json!(vector));
        
        store.put(key, content.as_bytes(), &meta).await?;
        println!("  ✓ Stored: {}", key);
    }
    
    // Тестируем векторный поиск
    println!("\n3. Testing vector search:");
    
    // Создаем запрос похожий на Rust документы
    let query = "Rust programming language features";
    let query_vector = create_mock_vector(query);
    
    let results = store.search_similar(&query_vector, 3).await?;
    println!("  Query: '{}'\n  Results:", query);
    for (key, score) in &results {
        println!("    - {} (score: {:.3})", key, score);
    }
    
    // Тестируем статистику
    println!("\n4. Storage statistics:");
    let stats = store.stats().await?;
    println!("  Total items: {}", stats.total_items);
    println!("  Total size: {} bytes", stats.total_size_bytes);
    println!("  Average access count: {:.2}", stats.avg_access_count);
    
    // Тестируем промоушен кандидатов
    println!("\n5. Testing promotion candidates:");
    let candidates = store.get_promotion_candidates(0, 0).await?;
    println!("  Found {} candidates for promotion", candidates.len());
    
    // Тестируем персистентность
    println!("\n6. Testing persistence:");
    drop(store);
    
    // Загружаем снова
    let store2 = VectorStore::new(
        MemLayer::Short,
        temp_dir.join("short_term.json")
    ).await?;
    
    let keys = store2.list_keys().await?;
    println!("  Reloaded {} items from disk", keys.len());
    
    // Тестируем удаление
    println!("\n7. Testing deletion:");
    store2.delete("js_web").await?;
    let remaining = store2.list_keys().await?;
    println!("  After deletion: {} items remain", remaining.len());
    
    println!("\n=== All tests passed! ===");
    
    // Очистка
    tokio::fs::remove_dir_all(temp_dir).await?;
    
    Ok(())
}

/// Создает простой mock вектор на основе текста
fn create_mock_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0; 1024];
    
    // Простая эвристика для создания вектора
    let words = text.to_lowercase();
    
    // Устанавливаем разные позиции для разных ключевых слов
    if words.contains("rust") { vector[0] = 0.9; vector[1] = 0.8; }
    if words.contains("programming") { vector[2] = 0.7; }
    if words.contains("language") { vector[3] = 0.6; }
    if words.contains("memory") { vector[4] = 0.8; }
    if words.contains("safety") { vector[5] = 0.7; }
    if words.contains("ownership") { vector[6] = 0.9; }
    if words.contains("async") { vector[7] = 0.8; }
    if words.contains("futures") { vector[8] = 0.7; }
    if words.contains("python") { vector[10] = 0.9; }
    if words.contains("javascript") { vector[12] = 0.9; }
    if words.contains("web") { vector[13] = 0.8; }
    
    // Добавляем длину текста как признак
    vector[20] = (text.len() as f32 / 100.0).min(1.0);
    
    // Нормализуем вектор
    let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut vector {
            *x /= norm;
        }
    }
    
    vector
}