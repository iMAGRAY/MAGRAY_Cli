use memory::{
    MemoryService, MemoryConfig, MemoryQuery, MemoryLayer,
    MemoryEntry, MemoryUpdate, LayerHealth, VectorStoreConfig,
    PromotionRule, PromotionConfig,
};
use ai::{EmbeddingService, RerankingService, Config as AiConfig};
use anyhow::Result;
use tokio;
use tracing::{info, warn, error};
use tracing_subscriber;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Полное тестирование системы памяти с моделями Qwen3
#[tokio::test]
async fn test_full_memory_system_with_qwen3() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    info!("🚀 Запуск полного теста системы памяти с Qwen3");

    // Создаём конфигурацию для AI сервисов
    let ai_config = AiConfig::default(); // Использует qwen3emb и qwen3_reranker по умолчанию
    
    // Создаём embedding сервис
    let embedding_service = Arc::new(EmbeddingService::new(ai_config.clone()).await?);
    
    // Создаём reranking сервис  
    let reranking_service = Arc::new(RerankingService::new(ai_config.clone()).await?);

    // Конфигурация системы памяти
    let memory_config = MemoryConfig {
        interact_ttl: Duration::from_secs(3600), // 1 час для теста
        insights_ttl: Duration::from_secs(7200), // 2 часа
        max_entries_per_layer: 1000,
        embedding_batch_size: 16,
        vector_store_config: VectorStoreConfig {
            m: 16,
            ef_construction: 200,
            ef_search: 100,
            num_threads: 4,
        },
        promotion_config: PromotionConfig {
            check_interval: Duration::from_secs(5), // Каждые 5 секунд для теста
            batch_size: 10,
            rules: vec![
                PromotionRule {
                    min_access_count: 2,
                    min_age: Duration::from_secs(3),
                    similarity_threshold: 0.75,
                    target_layer: MemoryLayer::Insights,
                },
                PromotionRule {
                    min_access_count: 5,
                    min_age: Duration::from_secs(10),
                    similarity_threshold: 0.85,
                    target_layer: MemoryLayer::Assets,
                },
            ],
        },
    };

    // Создаём сервис памяти
    let memory_service = MemoryService::new(
        memory_config,
        embedding_service.clone(),
        reranking_service.clone(),
    ).await?;

    info!("✅ Сервис памяти создан");

    // Тест 1: Добавление записей в разные слои
    info!("\n📝 Тест 1: Добавление записей в разные слои");
    
    let test_entries = vec![
        ("Rust - это системный язык программирования", MemoryLayer::Interact),
        ("Tokio - асинхронный runtime для Rust", MemoryLayer::Interact),
        ("async/await упрощает асинхронное программирование", MemoryLayer::Insights),
        ("ONNX Runtime поддерживает различные модели ИИ", MemoryLayer::Assets),
        ("Qwen3 - это семейство языковых моделей", MemoryLayer::Assets),
    ];

    let mut entry_ids = Vec::new();
    
    for (content, layer) in test_entries {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            content: content.to_string(),
            metadata: serde_json::json!({
                "test": true,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
            embedding: None, // Будет создан автоматически
            access_count: 0,
            last_accessed: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };
        
        memory_service.add_entry(layer, entry.clone()).await?;
        entry_ids.push(entry.id);
        info!("  ✓ Добавлена запись в {:?}: {}", layer, content);
    }

    // Даём время на индексацию
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Тест 2: Векторный поиск
    info!("\n🔍 Тест 2: Векторный поиск с Qwen3 embeddings");
    
    let search_queries = vec![
        "язык программирования Rust",
        "асинхронное программирование",
        "модели искусственного интеллекта",
        "Qwen модели",
    ];

    for query_text in search_queries {
        info!("\n  Запрос: '{}'", query_text);
        
        let query = MemoryQuery {
            query: Some(query_text.to_string()),
            layer: None, // Поиск по всем слоям
            limit: 3,
            similarity_threshold: Some(0.5),
            metadata_filter: None,
        };

        let start = Instant::now();
        let results = memory_service.search(query).await?;
        let search_time = start.elapsed();
        
        info!("  Время поиска: {:?}", search_time);
        info!("  Найдено результатов: {}", results.len());
        
        for (i, entry) in results.iter().enumerate() {
            if let Some(score) = entry.similarity_score {
                info!("    {}. [score: {:.3}] {}", i + 1, score, entry.content);
            }
        }
    }

    // Тест 3: Reranking
    info!("\n🎯 Тест 3: Reranking с Qwen3 моделью");
    
    let rerank_query = "системное программирование";
    info!("  Запрос для reranking: '{}'", rerank_query);

    // Сначала обычный поиск
    let query = MemoryQuery {
        query: Some(rerank_query.to_string()),
        layer: None,
        limit: 5,
        similarity_threshold: Some(0.3),
        metadata_filter: None,
    };

    let initial_results = memory_service.search(query.clone()).await?;
    info!("  Результаты до reranking:");
    for (i, entry) in initial_results.iter().enumerate() {
        if let Some(score) = entry.similarity_score {
            info!("    {}. [score: {:.3}] {}", i + 1, score, entry.content);
        }
    }

    // Reranking
    let start = Instant::now();
    let reranked_results = memory_service.search_with_reranking(
        query,
        rerank_query,
        3
    ).await?;
    let rerank_time = start.elapsed();

    info!("\n  Результаты после reranking (время: {:?}):", rerank_time);
    for (i, entry) in reranked_results.iter().enumerate() {
        if let Some(score) = entry.rerank_score {
            info!("    {}. [rerank: {:.3}] {}", i + 1, score, entry.content);
        }
    }

    // Тест 4: Обновление записей
    info!("\n✏️ Тест 4: Обновление записей и счётчиков доступа");
    
    // Обновляем первую запись несколько раз
    let first_id = &entry_ids[0];
    for i in 1..=3 {
        let update = MemoryUpdate {
            content: Some(format!("Rust - системный язык программирования (обновление {})", i)),
            metadata: Some(serde_json::json!({
                "updated": true,
                "version": i,
            })),
        };
        
        memory_service.update_entry(MemoryLayer::Interact, first_id, update).await?;
        info!("  ✓ Обновление {} выполнено", i);
        
        // Симулируем доступ
        memory_service.get_entry(MemoryLayer::Interact, first_id).await?;
    }

    // Тест 5: Продвижение между слоями
    info!("\n📈 Тест 5: Продвижение записей между слоями");
    
    // Ждём, чтобы записи могли быть продвинуты
    info!("  Ожидание продвижения (10 сек)...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Проверяем статистику слоёв
    let interact_health = memory_service.layer_health(MemoryLayer::Interact).await?;
    let insights_health = memory_service.layer_health(MemoryLayer::Insights).await?;
    let assets_health = memory_service.layer_health(MemoryLayer::Assets).await?;

    info!("\n  Статистика слоёв после продвижения:");
    info!("    Interact: {} записей", interact_health.entry_count);
    info!("    Insights: {} записей", insights_health.entry_count);
    info!("    Assets: {} записей", assets_health.entry_count);

    // Тест 6: Производительность батч-обработки
    info!("\n⚡ Тест 6: Производительность батч-обработки");
    
    let batch_size = 50;
    let mut batch_entries = Vec::new();
    
    for i in 0..batch_size {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            content: format!("Тестовая запись №{} для проверки производительности батч-обработки", i),
            metadata: serde_json::json!({"batch": true, "index": i}),
            embedding: None,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };
        batch_entries.push(entry);
    }

    let start = Instant::now();
    for entry in batch_entries {
        memory_service.add_entry(MemoryLayer::Interact, entry).await?;
    }
    let batch_time = start.elapsed();
    
    info!("  Добавлено {} записей за {:?}", batch_size, batch_time);
    info!("  Среднее время на запись: {:?}", batch_time / batch_size as u32);

    // Тест 7: Поиск с фильтрацией по метаданным
    info!("\n🔎 Тест 7: Поиск с фильтрацией по метаданным");
    
    let filtered_query = MemoryQuery {
        query: Some("тестовая запись".to_string()),
        layer: Some(MemoryLayer::Interact),
        limit: 5,
        similarity_threshold: Some(0.7),
        metadata_filter: Some(serde_json::json!({"batch": true})),
    };

    let filtered_results = memory_service.search(filtered_query).await?;
    info!("  Найдено записей с фильтром: {}", filtered_results.len());

    // Финальная статистика
    info!("\n📊 Финальная статистика системы:");
    
    let layers = vec![
        (MemoryLayer::Interact, "Interact"),
        (MemoryLayer::Insights, "Insights"),
        (MemoryLayer::Assets, "Assets"),
    ];

    for (layer, name) in layers {
        let health = memory_service.layer_health(layer).await?;
        info!("\n  Слой {}:", name);
        info!("    - Записей: {}", health.entry_count);
        info!("    - Здоровье: {:?}", health.status);
        info!("    - Использование памяти: {} байт", health.memory_usage);
        info!("    - Средний размер записи: {} байт", 
            if health.entry_count > 0 { health.memory_usage / health.entry_count } else { 0 }
        );
    }

    // Очистка
    info!("\n🧹 Очистка тестовых данных");
    memory_service.clear_layer(MemoryLayer::Interact).await?;
    
    info!("\n✅ Все тесты успешно завершены!");
    
    Ok(())
}

/// Тест производительности векторного поиска
#[tokio::test]
async fn test_vector_search_performance() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🏃 Тест производительности векторного поиска");

    let ai_config = AiConfig::default();
    let embedding_service = Arc::new(EmbeddingService::new(ai_config.clone()).await?);
    let reranking_service = Arc::new(RerankingService::new(ai_config.clone()).await?);

    let memory_config = MemoryConfig::default();
    let memory_service = MemoryService::new(
        memory_config,
        embedding_service.clone(),
        reranking_service.clone(),
    ).await?;

    // Добавляем 1000 записей
    info!("Добавление 1000 записей...");
    let start = Instant::now();
    
    for i in 0..1000 {
        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            content: format!("Документ {}. Содержит информацию о различных темах: программирование, алгоритмы, структуры данных, машинное обучение, нейронные сети.", i),
            metadata: serde_json::json!({"doc_id": i}),
            embedding: None,
            access_count: 0,
            last_accessed: chrono::Utc::now(),
            created_at: chrono::Utc::now(),
        };
        
        memory_service.add_entry(MemoryLayer::Assets, entry).await?;
        
        if (i + 1) % 100 == 0 {
            info!("  Добавлено {} записей", i + 1);
        }
    }
    
    let index_time = start.elapsed();
    info!("Индексация завершена за {:?}", index_time);

    // Выполняем серию поисковых запросов
    let search_queries = vec![
        "алгоритмы сортировки",
        "нейронные сети",
        "структуры данных",
        "машинное обучение",
        "программирование на Rust",
    ];

    info!("\nВыполнение поисковых запросов:");
    let mut total_search_time = Duration::ZERO;

    for query_text in &search_queries {
        let query = MemoryQuery {
            query: Some(query_text.to_string()),
            layer: Some(MemoryLayer::Assets),
            limit: 10,
            similarity_threshold: Some(0.5),
            metadata_filter: None,
        };

        let start = Instant::now();
        let results = memory_service.search(query).await?;
        let search_time = start.elapsed();
        
        total_search_time += search_time;
        info!("  '{}': {} результатов за {:?}", query_text, results.len(), search_time);
    }

    let avg_search_time = total_search_time / search_queries.len() as u32;
    info!("\nСреднее время поиска: {:?}", avg_search_time);

    Ok(())
}

/// Тест корректности токенизации Qwen3
#[tokio::test]
async fn test_qwen3_tokenization() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    info!("🔤 Тест токенизации Qwen3");

    let ai_config = AiConfig::default();
    let embedding_service = EmbeddingService::new(ai_config).await?;

    let test_texts = vec![
        "Hello, world!",
        "Привет, мир!",
        "你好，世界！",
        "🚀 Emoji test 🎉",
        "Mixed текст with 中文 and English",
    ];

    for text in test_texts {
        info!("\nТестирование текста: '{}'", text);
        
        let start = Instant::now();
        let embedding = embedding_service.embed_text(text).await?;
        let embed_time = start.elapsed();
        
        info!("  Размерность embedding: {}", embedding.len());
        info!("  Время создания: {:?}", embed_time);
        info!("  Первые 5 значений: {:?}", &embedding[..5.min(embedding.len())]);
    }

    Ok(())
}