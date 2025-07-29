use memory::{
    MemoryCoordinator, MemoryConfig, CodeSearchAPI, CodeQueryBuilder,
};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализируем логирование
    tracing_subscriber::fmt::init();

    println!("=== MAGRAY Memory System Demo ===\n");

    // Создаем конфигурацию памяти
    let base_path = PathBuf::from("./test_memory");
    let config = MemoryConfig {
        base_path: base_path.clone(),
        sqlite_path: base_path.join("test.db"),
        blobs_path: base_path.join("blobs"),
        vectors_path: base_path.join("vectors"),
        cache_path: base_path.join("cache.db"),
        ..Default::default()
    };

    // Модели находятся в C:/Users/1/Documents/GitHub/MAGRAY_Cli/models/

    println!("1. Инициализация системы памяти...");
    let memory = Arc::new(MemoryCoordinator::new(config).await?);
    println!("✓ Система памяти инициализирована\n");

    // Создаем API для поиска кода
    let mut code_search = CodeSearchAPI::new(Arc::clone(&memory));

    // Демонстрация индексации
    println!("2. Индексация проекта...");
    let project_path = PathBuf::from("./crates/memory/src");
    
    if project_path.exists() {
        code_search.index_directory(&project_path).await?;
        println!("✓ Проект проиндексирован\n");
    } else {
        println!("! Директория {} не найдена, пропускаем индексацию\n", project_path.display());
    }

    // Демонстрация различных типов поиска
    println!("3. Примеры поиска кода:\n");

    // Простой семантический поиск
    println!("a) Семантический поиск: 'vector store implementation'");
    let results = code_search.search_code("vector store implementation", 5, true).await?;
    for (i, result) in results.iter().enumerate() {
        println!("   {}. {} ({}:{})", 
            i + 1, 
            result.file_path, 
            result.line_start, 
            result.relevance_score
        );
        if let Some(ref entity) = result.entity_name {
            println!("      Entity: {} ({})", entity, result.entity_type);
        }
    }
    println!();

    // Поиск определения
    println!("b) Поиск определения: 'VectorStore'");
    let definitions = code_search.find_definition("VectorStore", Some("struct")).await?;
    for def in &definitions {
        println!("   Найдено в {}:{}", def.file_path, def.line_start);
        println!("   Код:");
        for line in def.code_snippet.lines().take(5) {
            println!("      {}", line);
        }
        if def.code_snippet.lines().count() > 5 {
            println!("      ...");
        }
    }
    println!();

    // Использование построителя запросов
    println!("c) Построитель запросов: async функции в Rust");
    let query = CodeQueryBuilder::new()
        .with_text("async function implementation")
        .language("rust")
        .entity_type("function")
        .build();
    
    println!("   Построенный запрос: {}", query);
    let results = code_search.search_code(&query, 3, false).await?;
    for result in &results {
        println!("   - {} в {}", 
            result.entity_name.as_ref().unwrap_or(&"<anonymous>".to_string()),
            result.file_path
        );
    }
    println!();

    // Демонстрация работы с памятью напрямую
    println!("4. Прямая работа с системой памяти:\n");
    
    // Сохранение данных
    let key = "example_code_snippet";
    let code_data = r#"
fn calculate_fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),
    }
}
"#;
    
    let mut meta = memory::MemMeta::default();
    meta.content_type = "text/rust".to_string();
    meta.tags.push("example".to_string());
    
    let ctx = memory::ExecutionContext::default();
    let result = memory.smart_put(key, code_data.as_bytes(), meta, &ctx).await?;
    println!("   ✓ Сохранено: {} байт в слое {:?}", 
        result.bytes_processed,
        result.mem_ref.as_ref().unwrap().layer
    );

    // Семантическая индексация
    if let Some(ref mem_ref) = result.mem_ref {
        memory.semantic_index(code_data, mem_ref, &memory::MemMeta::default()).await?;
        println!("   ✓ Проиндексировано семантически");
    }

    // Поиск по содержимому
    let search_results = memory.semantic_search("fibonacci recursive", 5, &ctx).await?;
    println!("   ✓ Найдено {} результатов для 'fibonacci recursive'", search_results.len());
    
    for (i, result) in search_results.iter().enumerate() {
        println!("      {}. Score: {:.3}, Key: {}", 
            i + 1, 
            result.score, 
            result.mem_ref.key
        );
    }
    println!();

    // Статистика использования
    println!("5. Статистика системы памяти:");
    let stats = memory.get_usage_stats().await?;
    println!("   Всего элементов: {}", stats.total_items);
    println!("   Размер данных: {} байт", stats.total_size_bytes);
    println!("   Статистика по слоям:");
    for (layer, layer_stats) in &stats.layers {
        println!("      {:?}: {} элементов, {} байт", 
            layer, 
            layer_stats.total_items, 
            layer_stats.total_size_bytes
        );
    }

    println!("\n=== Демонстрация завершена ===");

    // Очистка
    if base_path.exists() {
        tokio::fs::remove_dir_all(base_path).await?;
    }

    Ok(())
}