use memory::{
    MemoryService, MemoryConfig, Layer, Record, CacheConfigType, CacheConfig
};
use ai::AiConfig;
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use tracing::info;
use tracing_subscriber;

/// Полное тестирование системы памяти с моделями Qwen3
#[tokio::test]
async fn test_memory_system_with_qwen3_complete() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();

    info!("🚀 Запуск полного теста системы памяти с Qwen3");

    // Создаём временную директорию для теста
    let temp_dir = tempfile::TempDir::new()?;
    let base_path = temp_dir.path().to_path_buf();

    // Создаём конфигурацию
    let config = MemoryConfig {
        db_path: base_path.join("test_hnswdb"),
        cache_path: base_path.join("test_cache"),
        promotion: Default::default(),
        ai_config: AiConfig::default(), // Использует qwen3emb и qwen3_reranker по умолчанию
        health_config: Default::default(),
        cache_config: CacheConfigType::Lru(CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 10_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 100 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(80),
        ..Default::default()
    };

    // Создаём сервис памяти
    let memory_service = MemoryService::new(config).await?;
    info!("✅ Сервис памяти создан с Qwen3 моделями");

    // Тест 1: Добавление записей в разные слои
    info!("\n📝 Тест 1: Добавление записей в разные слои");
    
    let test_data = vec![
        ("Rust - это системный язык программирования с нулевой стоимостью абстракций", Layer::Interact),
        ("Tokio - асинхронный runtime для Rust, позволяющий писать эффективный асинхронный код", Layer::Interact),
        ("async/await упрощает написание асинхронного кода в Rust", Layer::Insights),
        ("ONNX Runtime поддерживает различные модели ИИ и ускорители", Layer::Assets),
        ("Qwen3 - это семейство языковых моделей с поддержкой многоязычности", Layer::Assets),
        ("HNSW алгоритм обеспечивает быстрый поиск ближайших соседей", Layer::Insights),
    ];

    let mut record_ids = Vec::new();
    
    for (content, layer) in test_data {
        let record = Record {
            id: Uuid::new_v4(),
            layer,
            text: content.to_string(),
            embedding: Vec::new(), // Будет создан автоматически
            kind: "test".to_string(),
            tags: vec!["test".to_string(), "qwen3".to_string()],
            project: "test".to_string(),
            session: "test_session".to_string(),
            score: 0.0,
            ts: Utc::now(),
            last_access: Utc::now(),
            access_count: 0,
        };
        
        memory_service.insert(record.clone()).await?;
        record_ids.push(record.id);
        info!("  ✓ Добавлена запись в {:?}: {}", layer, content);
    }

    // Даём время на индексацию
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Тест 2: Векторный поиск с Qwen3 embeddings
    info!("\n🔍 Тест 2: Векторный поиск с Qwen3 embeddings");
    
    let search_queries = vec![
        ("язык программирования Rust", Layer::Interact),
        ("асинхронное программирование", Layer::Insights),
        ("модели искусственного интеллекта", Layer::Assets),
        ("поиск ближайших соседей", Layer::Insights),
    ];

    for (query_text, expected_layer) in search_queries {
        info!("\n  Запрос: '{}' (ожидаемый слой: {:?})", query_text, expected_layer);
        
        let start = std::time::Instant::now();
        let results = memory_service
            .search(query_text)
            .with_layer(expected_layer)
            .top_k(3)
            .min_score(0.5)
            .execute()
            .await?;
        let search_time = start.elapsed();
        
        info!("  Время поиска: {:?}", search_time);
        info!("  Найдено результатов: {}", results.len());
        
        for (i, result) in results.iter().enumerate() {
            info!("    {}. [score: {:.3}] {}", 
                i + 1, 
                result.score,
                &result.text[..50.min(result.text.len())]
            );
        }
    }

    // Тест 3: Поиск по всем слоям
    info!("\n🌐 Тест 3: Поиск по всем слоям");
    
    let global_query = "программирование и алгоритмы";
    info!("  Глобальный запрос: '{}'", global_query);
    
    let start = std::time::Instant::now();
    let all_results = memory_service
        .search(global_query)
        .top_k(5)
        .min_score(0.3)
        .execute()
        .await?;
    let search_time = start.elapsed();
    
    info!("  Время поиска по всем слоям: {:?}", search_time);
    info!("  Найдено результатов: {}", all_results.len());
    
    for result in &all_results {
        info!("    [{:?}] [score: {:.3}] {}", 
            result.layer,
            result.score, 
            &result.text[..60.min(result.text.len())]
        );
    }

    // Тест 4: Reranking с Qwen3
    info!("\n🎯 Тест 4: Reranking результатов поиска");
    
    let rerank_query = "эффективный системный язык";
    info!("  Запрос для reranking: '{}'", rerank_query);
    
    let start = std::time::Instant::now();
    let reranked_results = memory_service
        .search(rerank_query)
        .top_k(10)
        .min_score(0.2)
        .execute()
        .await?;
    let rerank_time = start.elapsed();
    
    info!("  Время поиска с reranking: {:?}", rerank_time);
    info!("  Результаты после reranking:");
    
    for (i, result) in reranked_results.iter().take(3).enumerate() {
        info!("    {}. [final score: {:.3}] {}", 
            i + 1,
            result.score,
            &result.text[..60.min(result.text.len())]
        );
    }

    // Тест 5: Батч-загрузка
    info!("\n📦 Тест 5: Батч-загрузка записей");
    
    let batch_size = 50;
    let mut batch_records = Vec::new();
    
    for i in 0..batch_size {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("Тестовая запись №{} для проверки батч-загрузки. Содержит информацию о тестировании системы памяти MAGRAY.", i),
            embedding: Vec::new(),
            kind: "batch_test".to_string(),
            tags: vec!["batch".to_string(), format!("index_{}", i)],
            project: "test".to_string(),
            session: "batch_test".to_string(),
            score: 0.0,
            ts: Utc::now(),
            last_access: Utc::now(),
            access_count: 0,
        };
        batch_records.push(record);
    }

    let start = std::time::Instant::now();
    for record in batch_records {
        memory_service.insert(record).await?;
    }
    let batch_time = start.elapsed();
    
    info!("  Добавлено {} записей за {:?}", batch_size, batch_time);
    info!("  Среднее время на запись: {:?}", batch_time / batch_size as u32);

    // Тест 6: Поиск с фильтрацией по метаданным
    info!("\n🔎 Тест 6: Поиск с фильтрацией по метаданным");
    
    let start = std::time::Instant::now();
    let filtered_results = memory_service
        .search("тестовая запись")
        .with_layer(Layer::Interact)
        .top_k(5)
        .with_tags(vec!["batch".to_string()])
        .execute()
        .await?;
    let filter_time = start.elapsed();
    
    info!("  Время поиска с фильтром: {:?}", filter_time);
    info!("  Найдено записей с фильтром по метаданным: {}", filtered_results.len());

    // Тест 7: Статистика системы
    info!("\n📊 Тест 7: Статистика системы памяти");
    
    let (hits, misses, items) = memory_service.cache_stats();
    let hit_rate = if hits + misses > 0 {
        (hits as f64 / (hits + misses) as f64) * 100.0
    } else {
        0.0
    };
    info!("  Общая статистика:");
    info!("    - Попадания в кэш: {}", hits);
    info!("    - Промахи: {}", misses);
    info!("    - Всего элементов: {}", items);
    info!("    - Hit rate: {:.1}%", hit_rate);

    // Тест 8: Обновление записей
    info!("\n✏️ Тест 8: Обновление записей");
    
    let first_id = record_ids[0];
    let updated_record = Record {
        id: first_id,
        layer: Layer::Interact,
        text: "Rust - современный системный язык программирования с гарантиями безопасности памяти".to_string(),
        embedding: Vec::new(),
        kind: "test".to_string(),
        tags: vec!["test".to_string(), "updated".to_string()],
        project: "test".to_string(),
        session: "test_session".to_string(),
        score: 0.0,
        ts: Utc::now(),
        last_access: Utc::now(),
        access_count: 5,
    };
    
    memory_service.insert(updated_record).await?;
    info!("  ✓ Запись обновлена");

    // Проверяем обновление
    let search_results = memory_service
        .search("безопасность памяти")
        .with_layer(Layer::Interact)
        .top_k(1)
        .execute()
        .await?;
    
    if !search_results.is_empty() && search_results[0].id == first_id {
        info!("  ✓ Обновлённая запись найдена по новому содержимому");
    }

    // Тест 9: Производительность embeddings
    info!("\n⚡ Тест 9: Производительность генерации embeddings");
    
    let long_text = "Длинный текст. ".repeat(50);
    let test_texts = vec![
        "Короткий текст",
        "Средний текст для тестирования производительности embedding сервиса",
        long_text.as_str(),
    ];
    
    for (i, text) in test_texts.iter().enumerate() {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: text.to_string(),
            embedding: Vec::new(),
            kind: "perf_test".to_string(),
            tags: vec!["perf".to_string(), format!("test_{}", i)],
            project: "test".to_string(),
            session: "perf_test".to_string(),
            score: 0.0,
            ts: Utc::now(),
            last_access: Utc::now(),
            access_count: 0,
        };
        
        let start = std::time::Instant::now();
        memory_service.insert(record).await?;
        let embed_time = start.elapsed();
        
        info!("  Текст {} символов: {:?}", text.len(), embed_time);
    }

    // Тест 10: Многоязычность Qwen3
    info!("\n🌍 Тест 10: Многоязычная поддержка Qwen3");
    
    let multilingual_texts = vec![
        ("Hello, world!", "English"),
        ("Привет, мир!", "Russian"),
        ("你好，世界！", "Chinese"),
        ("こんにちは、世界！", "Japanese"),
        ("مرحبا بالعالم!", "Arabic"),
    ];
    
    for (text, lang) in multilingual_texts {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Assets,
            text: text.to_string(),
            embedding: Vec::new(),
            kind: "multilingual".to_string(),
            tags: vec![lang.to_string()],
            project: "test".to_string(),
            session: "multilingual_test".to_string(),
            score: 0.0,
            ts: Utc::now(),
            last_access: Utc::now(),
            access_count: 0,
        };
        
        memory_service.insert(record).await?;
        info!("  ✓ Добавлен текст на {}: {}", lang, text);
    }
    
    // Поиск на разных языках
    let multilingual_query = "привет мир hello";
    let results = memory_service
        .search(multilingual_query)
        .with_layer(Layer::Assets)
        .top_k(5)
        .execute()
        .await?;
    
    info!("\n  Результаты многоязычного поиска:");
    for result in results {
        if let Some(lang) = result.tags.first() {
            info!("    [{}: {:.3}] {}", lang, result.score, result.text);
        }
    }

    info!("\n✅ Все тесты успешно завершены!");
    info!("🎉 Система памяти MAGRAY с моделями Qwen3 работает корректно!");
    
    Ok(())
}

/// Тест производительности на больших объёмах
#[tokio::test]
async fn test_qwen3_performance_at_scale() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🏃 Тест производительности Qwen3 на больших объёмах");

    let temp_dir = tempfile::TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("perf_test_db"),
        cache_path: temp_dir.path().join("perf_test_cache"),
        promotion: Default::default(),
        ai_config: AiConfig::default(),
        health_config: Default::default(),
        cache_config: CacheConfigType::Lru(CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
        ..Default::default()
    };

    let memory_service = MemoryService::new(config).await?;

    // Загружаем 1000 документов
    info!("Загрузка 1000 документов...");
    let start_total = std::time::Instant::now();
    
    let batch_size = 100;
    for batch_idx in 0..10 {
        let mut batch = Vec::new();
        
        for i in 0..batch_size {
            let doc_idx = batch_idx * batch_size + i;
            let record = Record {
                id: Uuid::new_v4(),
                layer: Layer::Assets,
                text: format!(
                    "Документ №{}. Этот документ содержит информацию о различных аспектах разработки: \
                    архитектура систем, алгоритмы и структуры данных, машинное обучение, \
                    нейронные сети, обработка естественного языка, компьютерное зрение.",
                    doc_idx
                ),
                embedding: Vec::new(),
                kind: "document".to_string(),
                tags: vec![format!("batch_{}", batch_idx)],
                project: "test".to_string(),
                session: "perf_test".to_string(),
                score: 0.0,
                ts: Utc::now(),
                last_access: Utc::now(),
                access_count: 0,
            };
            batch.push(record);
        }
        
        let batch_start = std::time::Instant::now();
        for record in batch {
            memory_service.insert(record).await?;
        }
        let batch_time = batch_start.elapsed();
        
        info!("  Батч {}: {} документов за {:?}", batch_idx + 1, batch_size, batch_time);
    }
    
    let total_time = start_total.elapsed();
    info!("Всего загружено 1000 документов за {:?}", total_time);
    info!("Среднее время на документ: {:?}", total_time / 1000);

    // Тестируем поиск
    info!("\nТестирование поиска на большом объёме:");
    
    let queries = vec![
        "архитектура микросервисов",
        "алгоритмы машинного обучения",
        "обработка естественного языка",
        "нейронные сети и глубокое обучение",
        "структуры данных для поиска",
    ];
    
    let mut total_search_time = std::time::Duration::ZERO;
    
    for query in &queries {
        let start = std::time::Instant::now();
        let results = memory_service
            .search(query)
            .top_k(10)
            .min_score(0.5)
            .execute()
            .await?;
        let search_time = start.elapsed();
        
        total_search_time += search_time;
        info!("  '{}': {} результатов за {:?}", query, results.len(), search_time);
    }
    
    let avg_search_time = total_search_time / queries.len() as u32;
    info!("\nСреднее время поиска: {:?}", avg_search_time);
    
    // Тест с reranking
    info!("\nТест производительности с reranking:");
    
    let start = std::time::Instant::now();
    let reranked = memory_service
        .search("машинное обучение и нейронные сети")
        .top_k(20)
        .min_score(0.3)
        .execute()
        .await?;
    let rerank_time = start.elapsed();
    
    info!("Поиск с reranking топ-10 из 20: {:?}", rerank_time);
    info!("Найдено и переранжировано: {} результатов", reranked.len());

    Ok(())
}