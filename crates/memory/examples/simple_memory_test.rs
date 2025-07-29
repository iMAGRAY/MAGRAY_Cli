use memory::{MemoryStore, MemMeta, ExecutionContext};
use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Simple Memory Store Test ===\n");

    // Тестируем VectorStore напрямую
    let temp_dir = PathBuf::from("./test_vector_store");
    tokio::fs::create_dir_all(&temp_dir).await?;
    
    println!("1. Создаем VectorStore...");
    let store = memory::layers::VectorStore::new(
        memory::MemLayer::Short,
        temp_dir.join("short_term.json")
    ).await?;
    println!("✓ VectorStore создан\n");

    // Тестируем базовые операции
    println!("2. Тестируем базовые операции:");
    
    // Сохранение
    let key = "test_key";
    let data = b"Hello from vector store!";
    let mut meta = MemMeta::default();
    meta.content_type = "text/plain".to_string();
    
    // Добавляем mock вектор в метаданные
    let mock_vector = vec![0.1f32; 1024]; // 1024-мерный вектор
    meta.extra.insert(
        "vector".to_string(),
        serde_json::json!(mock_vector),
    );
    
    store.put(key, data, &meta).await?;
    println!("   ✓ Данные сохранены");
    
    // Чтение
    if let Some((retrieved_data, retrieved_meta)) = store.get(key).await? {
        let text = std::str::from_utf8(&retrieved_data)?;
        println!("   ✓ Данные прочитаны: '{}'", text);
        println!("   ✓ Access count: {}", retrieved_meta.access_count);
    }
    
    // Проверка существования
    if store.exists(key).await? {
        println!("   ✓ Ключ существует");
    }
    
    // Список ключей
    let keys = store.list_keys().await?;
    println!("   ✓ Всего ключей: {}", keys.len());
    
    // Статистика
    let stats = store.stats().await?;
    println!("\n3. Статистика хранилища:");
    println!("   Элементов: {}", stats.total_items);
    println!("   Размер: {} байт", stats.total_size_bytes);
    println!("   Средний access count: {:.2}", stats.avg_access_count);
    
    // Тестируем векторный поиск
    println!("\n4. Тестируем векторный поиск:");
    
    // Добавляем еще несколько записей
    for i in 1..=3 {
        let key = format!("doc_{}", i);
        let data = format!("Document number {}", i).into_bytes();
        let mut meta = MemMeta::default();
        
        // Создаем разные векторы
        let mut vector = vec![0.0f32; 1024];
        vector[i] = 1.0; // Разные позиции для разных документов
        
        meta.extra.insert(
            "vector".to_string(),
            serde_json::to_value(&vector)?,
        );
        
        store.put(&key, &data, &meta).await?;
    }
    
    // Ищем похожие на первый документ
    if let Some(query_vector) = store.get_vector("doc_1").await {
        let similar = store.search_similar(&query_vector, 3).await?;
        println!("   Найдено {} похожих документов:", similar.len());
        for (key, score) in similar {
            println!("     - {} (score: {:.3})", key, score);
        }
    }
    
    // Тестируем промоушен кандидатов
    println!("\n5. Тестируем поиск кандидатов для промоушена:");
    let candidates = store.get_promotion_candidates(0, 0).await?;
    println!("   Найдено {} кандидатов", candidates.len());
    
    println!("\n=== Тест завершен успешно! ===");
    
    // Очистка
    if temp_dir.exists() {
        tokio::fs::remove_dir_all(temp_dir).await?;
    }
    
    Ok(())
}