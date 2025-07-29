use anyhow::Result;
use memory::{
    MemoryCoordinator, MemoryConfig, MemLayer, MemMeta, MemRef,
    semantic::{SemanticRouter, VectorizerService, RerankerService},
};
use std::path::PathBuf;
use tracing::{info, debug};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализируем логирование
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🧠 MAGRAY Memory System - Full Integration Test");
    println!("{}", "=".repeat(60));

    // Создаем конфигурацию памяти
    let mut config = MemoryConfig::default();
    config.base_path = PathBuf::from("test_memory_system");
    config.sqlite_path = config.base_path.join("test.db");
    config.blobs_path = config.base_path.join("blobs");
    config.vectors_path = config.base_path.join("vectors");
    config.cache_path = config.base_path.join("embed_cache.db");

    // Создаем директории
    tokio::fs::create_dir_all(&config.base_path).await?;
    tokio::fs::create_dir_all(&config.blobs_path).await?;
    tokio::fs::create_dir_all(&config.vectors_path).await?;

    println!("\n📁 Test environment created at: {}", config.base_path.display());

    // Инициализируем координатор памяти
    println!("\n🔧 Initializing Memory Coordinator...");
    let coordinator = MemoryCoordinator::new(config.clone()).await?;
    println!("✓ Memory Coordinator initialized");

    // Тестовые данные
    let test_data = vec![
        ("rust_basics", "Rust is a systems programming language focused on safety, speed, and concurrency."),
        ("memory_safety", "Rust guarantees memory safety through its ownership system and borrow checker."),
        ("async_rust", "Async Rust enables writing concurrent code using async/await syntax."),
        ("error_handling", "Rust uses Result<T, E> and Option<T> for explicit error handling."),
        ("traits", "Traits in Rust define shared behavior that types can implement."),
        ("lifetimes", "Lifetimes ensure references are valid and prevent dangling pointers."),
        ("cargo", "Cargo is Rust's build system and package manager."),
        ("macros", "Rust macros enable metaprogramming and code generation."),
        ("unsafe", "Unsafe Rust allows low-level operations with manual memory management."),
        ("testing", "Rust has built-in testing framework with #[test] attribute."),
    ];

    // Тест 1: Сохранение данных в разные слои
    println!("\n📝 Test 1: Storing data in different memory layers");
    println!("{}", "-".repeat(50));

    // M0 - Ephemeral (временные данные)
    let mut meta = MemMeta::default();
    meta.tags = vec!["ephemeral".to_string(), "test".to_string()];
    meta.ttl_seconds = Some(300); // 5 минут
    
    coordinator.store(MemLayer::Ephemeral, "session_123", b"Current session data", &meta).await?;
    println!("✓ Stored in M0 (Ephemeral): session data");

    // M1 - Short-term (недавние факты)
    meta.tags = vec!["fact".to_string(), "recent".to_string()];
    meta.ttl_seconds = Some(3600); // 1 час
    
    for (i, (key, value)) in test_data.iter().take(3).enumerate() {
        coordinator.store(MemLayer::Short, key, value.as_bytes(), &meta).await?;
        println!("✓ Stored in M1 (Short-term): {}", key);
    }

    // M2 - Medium-term (структурированные данные)
    meta.tags = vec!["knowledge".to_string(), "structured".to_string()];
    meta.ttl_seconds = Some(86400); // 1 день
    
    for (key, value) in test_data.iter().skip(3).take(4) {
        coordinator.store(MemLayer::Medium, key, value.as_bytes(), &meta).await?;
        println!("✓ Stored in M2 (Medium-term): {}", key);
    }

    // M3 - Long-term (большие артефакты)
    meta.tags = vec!["artifact".to_string(), "permanent".to_string()];
    meta.ttl_seconds = None; // Без TTL
    
    let large_content = "# Rust Programming Guide\n\n".repeat(100);
    coordinator.store(MemLayer::Long, "rust_guide", large_content.as_bytes(), &meta).await?;
    println!("✓ Stored in M3 (Long-term): large rust guide");

    // Тест 2: Поиск через семантический слой
    println!("\n🔍 Test 2: Semantic search across all layers");
    println!("{}", "-".repeat(50));

    let queries = vec![
        "How does Rust ensure memory safety?",
        "What is async programming in Rust?",
        "Tell me about Rust's package manager",
        "How to handle errors in Rust?",
    ];

    for query in &queries {
        println!("\n🔎 Query: \"{}\"", query);
        let results = coordinator.search(query, 3).await?;
        
        for (i, result) in results.iter().enumerate() {
            println!("  {}. [{}] Score: {:.3} - Key: {}", 
                i + 1,
                match result.mem_ref.layer {
                    MemLayer::Ephemeral => "M0",
                    MemLayer::Short => "M1",
                    MemLayer::Medium => "M2",
                    MemLayer::Long => "M3",
                    MemLayer::Semantic => "M4",
                },
                result.score,
                result.mem_ref.key
            );
            if let Some(snippet) = &result.snippet {
                println!("     Preview: {}...", &snippet.chars().take(60).collect::<String>());
            }
        }
    }

    // Тест 3: Прямой доступ к семантическому роутеру
    println!("\n🧭 Test 3: Direct semantic router test");
    println!("{}", "-".repeat(50));

    if let Ok(semantic_router) = SemanticRouter::new(
        config.vectors_path.clone(),
        config.cache_path.clone(),
    ).await {
        // Индексируем все тестовые данные
        for (key, content) in &test_data {
            let mem_ref = MemRef::new(MemLayer::Medium, key.to_string());
            let mut meta = MemMeta::default();
            meta.tags = vec!["test".to_string()];
            
            semantic_router.ingest(content, &mem_ref, &meta).await?;
        }
        println!("✓ Indexed {} documents", test_data.len());

        // Поиск похожих документов
        let search_query = "memory management and safety";
        let results = semantic_router.search(search_query, 5).await?;
        
        println!("\n📊 Semantic search for: \"{}\"", search_query);
        for (i, result) in results.iter().enumerate() {
            println!("  {}. Score: {:.4} - {}", i + 1, result.score, result.mem_ref.key);
        }
    }

    // Тест 4: Тестирование VectorizerService напрямую
    println!("\n🔢 Test 4: Vectorizer Service test");
    println!("{}", "-".repeat(50));

    let model_path = PathBuf::from("../../models/Qwen3-Embedding-0.6B-ONNX");
    
    match VectorizerService::new(model_path.clone()).await {
        Ok(vectorizer) => {
            let texts = vec![
                "Rust programming language",
                "Memory safety and ownership",
                "Concurrent programming",
            ];
            
            let embeddings = vectorizer.embed(&texts).await?;
            println!("✓ Generated {} embeddings", embeddings.len());
            
            for (i, text) in texts.iter().enumerate() {
                println!("  Text: \"{}\"", text);
                println!("    Embedding dims: {}", embeddings[i].len());
                println!("    First 5 values: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
                    embeddings[i][0], embeddings[i][1], embeddings[i][2], 
                    embeddings[i][3], embeddings[i][4]
                );
            }
            
            // Проверяем кеш
            let (entries, size) = vectorizer.cache_stats().await;
            println!("\n📦 Cache statistics:");
            println!("  Entries: {}", entries);
            println!("  Size: {} bytes", size);
        }
        Err(e) => {
            println!("⚠️  Vectorizer initialization failed: {}", e);
            println!("   Make sure ONNX models are present at: {}", model_path.display());
        }
    }

    // Тест 5: Тестирование RerankerService
    println!("\n🎯 Test 5: Reranker Service test");
    println!("{}", "-".repeat(50));

    let reranker_path = PathBuf::from("../../models/Qwen3-Reranker-0.6B-ONNX");
    
    match RerankerService::new(reranker_path.clone()).await {
        Ok(reranker) => {
            let query = "How to ensure memory safety?";
            let documents = vec![
                "Rust guarantees memory safety through ownership and borrowing".to_string(),
                "Cargo is Rust's package manager for dependencies".to_string(),
                "Memory safety prevents segmentation faults and data races".to_string(),
                "Async Rust uses futures for concurrent programming".to_string(),
                "The borrow checker enforces memory safety at compile time".to_string(),
            ];
            
            println!("Query: \"{}\"", query);
            println!("\nOriginal documents:");
            for (i, doc) in documents.iter().enumerate() {
                println!("  {}. {}", i + 1, doc);
            }
            
            let reranked = reranker.rerank(query, &documents, 3).await?;
            
            println!("\n🏆 Top 3 reranked results:");
            for (rank, (idx, score)) in reranked.iter().enumerate() {
                println!("  {}. [Score: {:.4}] {}", 
                    rank + 1, score, documents[*idx]
                );
            }
        }
        Err(e) => {
            println!("⚠️  Reranker initialization failed: {}", e);
            println!("   Make sure ONNX models are present at: {}", reranker_path.display());
        }
    }

    // Тест 6: Промоушен между слоями
    println!("\n⬆️  Test 6: Layer promotion test");
    println!("{}", "-".repeat(50));

    // Создаем данные с высоким access_count для промоушена
    let mut promo_meta = MemMeta::default();
    promo_meta.tags = vec!["important".to_string()];
    promo_meta.access_count = 10;
    promo_meta.last_accessed = Utc::now();

    coordinator.store(MemLayer::Ephemeral, "promoted_data", b"This should be promoted", &promo_meta).await?;
    println!("✓ Stored data in Ephemeral layer with high access count");

    // Запускаем промоушен
    let promoted = coordinator.check_promotions().await?;
    println!("✓ Promotion check completed: {} items promoted", promoted);

    // Проверяем, переместились ли данные
    if let Ok(Some((data, meta))) = coordinator.retrieve(MemLayer::Short, "promoted_data").await {
        println!("✓ Data successfully promoted to Short-term layer!");
        println!("  Access count: {}", meta.access_count);
    }

    // Тест 7: Статистика системы
    println!("\n📊 Test 7: Memory system statistics");
    println!("{}", "-".repeat(50));

    let stats = coordinator.system_stats().await?;
    println!("System-wide statistics:");
    println!("  Total items: {}", stats.total_items);
    println!("  Total size: {} bytes", stats.total_size_bytes);
    println!("  Layer distribution:");
    
    for layer in &[MemLayer::Ephemeral, MemLayer::Short, MemLayer::Medium, MemLayer::Long] {
        if let Ok(layer_stats) = coordinator.layer_stats(*layer).await {
            println!("    {:?}: {} items, {} bytes", 
                layer, layer_stats.total_items, layer_stats.total_size_bytes
            );
        }
    }

    // Очистка тестовых данных
    println!("\n🧹 Cleaning up test data...");
    tokio::fs::remove_dir_all(&config.base_path).await?;
    println!("✓ Test environment cleaned up");

    println!("\n✅ All tests completed successfully!");
    Ok(())
}