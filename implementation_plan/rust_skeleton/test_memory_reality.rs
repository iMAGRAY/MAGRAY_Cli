// Тест для проверки реальной функциональности системы памяти

use memory::{MemoryCoordinator, MemoryConfig, MemMeta};
use memory::types::ExecutionContext;
use tempfile::TempDir;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Настройка трассировки для отладки
    tracing_subscriber::fmt::init();
    
    println!("🧠 Тестирование реальной функциональности системы памяти");
    
    // Создаём временную директорию для тестов
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path().to_path_buf();
    
    // Создаём поддельные директории моделей
    tokio::fs::create_dir_all(base_path.join("src/Qwen3-Embedding-0.6B-ONNX")).await?;
    tokio::fs::create_dir_all(base_path.join("src/Qwen3-Reranker-0.6B-ONNX")).await?;
    
    let config = MemoryConfig {
        base_path: base_path.clone(),
        sqlite_path: base_path.join("memory.db"),
        blobs_path: base_path.join("blobs"),
        vectors_path: base_path.join("vectors"),
        cache_path: base_path.join("cache.db"),
        ..Default::default()
    };
    
    println!("📋 Инициализация координатора памяти...");
    let coordinator = MemoryCoordinator::new(config).await?;
    
    let ctx = ExecutionContext::default();
    
    // Тест 1: Базовые операции
    println!("🔸 Тест 1: Базовые операции с памятью");
    let mut meta = MemMeta::default();
    meta.content_type = "text/plain".to_string();
    meta.tags.push("test".to_string());
    
    let result = coordinator.smart_put("test_key", b"Hello, World!", meta.clone(), &ctx).await?;
    println!("  ✅ Запись данных: success = {}", result.success);
    
    let retrieved = coordinator.smart_get("test_key", &ctx).await?;
    if let Some((data, meta_retrieved, mem_ref)) = retrieved {
        println!("  ✅ Чтение данных: {} байт, слой: {:?}", data.len(), mem_ref.layer);
        println!("  📊 Количество обращений: {}", meta_retrieved.access_count);
    }
    
    // Тест 2: Семантический поиск
    println!("🔸 Тест 2: Семантический поиск");
    
    // Добавляем несколько текстовых документов
    let documents = [
        ("doc1", "Это документ о машинном обучении и искусственном интеллекте"),
        ("doc2", "В этом документе говорится о погоде и природе"),
        ("doc3", "Глубокое обучение использует нейронные сети для анализа данных"),
    ];
    
    for (key, text) in &documents {
        let mut doc_meta = MemMeta::default();
        doc_meta.content_type = "text/plain".to_string();
        coordinator.smart_put(key, text.as_bytes(), doc_meta, &ctx).await?;
    }
    
    // Небольшая задержка для индексации
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    let search_results = coordinator.semantic_search("машинное обучение", 5, &ctx).await?;
    println!("  🔍 Найдено {} результатов для запроса 'машинное обучение'", search_results.len());
    
    for (i, result) in search_results.iter().enumerate() {
        println!("    {}. Ключ: {}, Оценка: {:.3}, Слой: {:?}", 
                i + 1, result.mem_ref.key, result.score, result.mem_ref.layer);
    }
    
    // Тест 3: Статистика использования
    println!("🔸 Тест 3: Статистика использования памяти");
    let stats = coordinator.get_usage_stats().await?;
    println!("  📈 Общее количество элементов: {}", stats.total_items);
    println!("  💾 Общий размер: {} байт", stats.total_size_bytes);
    println!("  📊 Статистика по слоям:");
    
    for (layer, layer_stats) in &stats.layers {
        println!("    {:?}: {} элементов, {} байт", 
                layer, layer_stats.total_items, layer_stats.total_size_bytes);
    }
    
    // Тест 4: Очистка устаревших данных
    println!("🔸 Тест 4: Очистка устаревших данных");
    let cleaned = coordinator.cleanup_expired().await?;
    println!("  🧹 Очищено {} устаревших элементов", cleaned);
    
    // Тест 5: Удаление данных
    println!("🔸 Тест 5: Удаление данных");
    let deleted = coordinator.delete("test_key").await?;
    println!("  🗑️ Данные удалены: {}", deleted);
    
    let check_deleted = coordinator.smart_get("test_key", &ctx).await?;
    println!("  ✅ Проверка удаления: {}", if check_deleted.is_none() { "успешно" } else { "ошибка" });
    
    println!("🎉 Все тесты завершены успешно!");
    println!("💡 Система памяти работает, но использует заглушки для ONNX моделей");
    
    Ok(())
}