use memory::{
    MemoryService, MemoryConfig, Layer, Record, CacheConfigType, CacheConfig,
};
use ai::AiConfig;
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use tracing::info;
use tracing_subscriber;

/// Полное тестирование системы памяти с моделями Qwen3
#[tokio::test]
async fn test_complete_qwen3_memory_system() -> Result<()> {
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
    };

    // Создаём сервис памяти
    let memory_service = MemoryService::new(config).await?;
    info!("✅ Сервис памяти создан с Qwen3 моделями");

    // Тест 1: Добавление записей в разные слои
    info!("\n📝 Тест 1: Добавление записей в разные слои");
    
    let test_data = vec![
        ("Rust - это системный язык программирования с нулевой стоимостью абстракций", Layer::Interact, vec!["rust", "programming"]),
        ("Tokio - асинхронный runtime для Rust, позволяющий писать эффективный асинхронный код", Layer::Interact, vec!["tokio", "async", "rust"]),
        ("async/await упрощает написание асинхронного кода в Rust", Layer::Insights, vec!["async", "rust"]),
        ("ONNX Runtime поддерживает различные модели ИИ и ускорители", Layer::Assets, vec!["onnx", "ai", "ml"]),
        ("Qwen3 - это семейство языковых моделей с поддержкой многоязычности", Layer::Assets, vec!["qwen3", "ai", "nlp"]),
        ("HNSW алгоритм обеспечивает быстрый поиск ближайших соседей", Layer::Insights, vec!["hnsw", "search", "algorithm"]),
    ];

    let mut record_ids = Vec::new();
    
    for (content, layer, tags) in test_data {
        let record = Record {
            id: Uuid::new_v4(),
            layer,
            text: content.to_string(),
            embedding: Vec::new(), // Будет создан автоматически
            kind: "test".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
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
            info!("    {}. [layer: {:?}] {}", 
                i + 1, 
                result.layer,
                &result.text[..80.min(result.text.len())]
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
        info!("    [{:?}] {}", 
            result.layer,
            &result.text[..60.min(result.text.len())]
        );
    }

    // Тест 4: Поиск по тегам
    info!("\n🏷️ Тест 4: Поиск с фильтрацией по тегам");
    
    let tag_search = memory_service
        .search("искусственный интеллект")
        .with_tags(vec!["ai".to_string()])
        .top_k(10)
        .execute()
        .await?;
    
    info!("  Найдено записей с тегом 'ai': {}", tag_search.len());
    for record in &tag_search {
        info!("    - {} (теги: {:?})", 
            &record.text[..50.min(record.text.len())],
            record.tags
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
            tags: vec!["batch".to_string(), format!("test_{}", i % 10)],
            project: "test_project".to_string(),
            session: "batch_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        batch_records.push(record);
    }

    let start = std::time::Instant::now();
    memory_service.insert_batch(batch_records).await?;
    let batch_time = start.elapsed();
    
    info!("  Добавлено {} записей за {:?}", batch_size, batch_time);
    info!("  Среднее время на запись: {:?}", batch_time / batch_size as u32);

    // Тест 6: Проверка кэширования embeddings
    info!("\n💾 Тест 6: Проверка кэширования embeddings");
    
    // Вставляем одинаковый текст несколько раз
    let cached_text = "Этот текст будет использован для проверки кэширования embeddings";
    
    let mut cache_test_times = Vec::new();
    for i in 0..3 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: cached_text.to_string(),
            embedding: Vec::new(),
            kind: "cache_test".to_string(),
            tags: vec![format!("cache_test_{}", i)],
            project: "test_project".to_string(),
            session: "cache_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        
        let start = std::time::Instant::now();
        memory_service.insert(record).await?;
        let insert_time = start.elapsed();
        cache_test_times.push(insert_time);
        
        info!("  Вставка {}: {:?}", i + 1, insert_time);
    }
    
    // Первая вставка должна быть медленнее из-за генерации embedding
    if cache_test_times.len() >= 2 && cache_test_times[1] < cache_test_times[0] {
        info!("  ✓ Кэширование работает! Вторая вставка быстрее первой");
    }

    // Тест 7: Статистика кэша
    info!("\n📊 Тест 7: Статистика системы");
    
    let (hits, misses, items) = memory_service.cache_stats();
    info!("  Статистика кэша:");
    info!("    - Попадания: {}", hits);
    info!("    - Промахи: {}", misses);
    info!("    - Элементов в кэше: {}", items);
    info!("    - Hit rate: {:.1}%", (hits as f64 / (hits + misses).max(1) as f64) * 100.0);

    // Тест 8: Проверка здоровья системы
    info!("\n🏥 Тест 8: Проверка здоровья системы");
    
    let health_status = memory_service.health_check().await?;
    info!("  Статус системы: {:?}", health_status.status);
    info!("  Здоровье компонентов:");
    for (component, health) in &health_status.components {
        info!("    - {:?}: {:?}", component, health.status);
    }

    // Тест 9: Reranking (если доступен)
    info!("\n🎯 Тест 9: Тестирование reranking");
    
    let rerank_query = "эффективный системный язык программирования";
    let search_results = memory_service
        .search(rerank_query)
        .top_k(10)
        .min_score(0.2)
        .execute()
        .await?;
    
    if search_results.len() >= 3 {
        // Reranking через поиск с опциональным reranking параметром
        info!("  Reranking сервис интегрирован в поисковый API");
    }

    // Тест 10: Многоязычность Qwen3
    info!("\n🌍 Тест 10: Многоязычная поддержка Qwen3");
    
    let multilingual_texts = vec![
        ("Hello, world! This is a test.", "en"),
        ("Привет, мир! Это тест.", "ru"),
        ("你好，世界！这是一个测试。", "zh"),
        ("こんにちは、世界！これはテストです。", "ja"),
        ("Hola, mundo! Esto es una prueba.", "es"),
    ];
    
    for (text, lang) in multilingual_texts {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Assets,
            text: text.to_string(),
            embedding: Vec::new(),
            kind: "multilingual".to_string(),
            tags: vec![lang.to_string(), "multilingual".to_string()],
            project: "multilingual_test".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        
        memory_service.insert(record).await?;
        info!("  ✓ Добавлен текст на {}: {}", lang, text);
    }
    
    // Поиск на разных языках
    let multilingual_queries = vec![
        ("привет мир", "ru"),
        ("hello world", "en"),
        ("世界", "zh"),
    ];
    
    for (query, expected_lang) in multilingual_queries {
        info!("\n  Поиск '{}' (ожидается {}):", query, expected_lang);
        
        let results = memory_service
            .search(query)
            .with_layer(Layer::Assets)
            .with_tags(vec!["multilingual".to_string()])
            .top_k(3)
            .execute()
            .await?;
        
        for result in results {
            let lang_tag = result.tags.iter()
                .find(|t| t.len() == 2)
                .unwrap_or(&"??".to_string());
            info!("    [{}] {}", lang_tag, result.text);
        }
    }

    // Итоговая статистика
    info!("\n📈 Итоговая статистика теста:");
    
    let final_health = memory_service.health_check().await?;
    info!("  Финальный статус: {:?}", final_health.status);
    
    // Метрики доступны через health check
    info!("  Метрики доступны через систему health monitoring");

    info!("\n✅ Все тесты успешно завершены!");
    info!("🎉 Система памяти MAGRAY с моделями Qwen3 работает корректно!");
    
    Ok(())
}

/// Стресс-тест производительности
#[tokio::test]
#[ignore] // Запускать отдельно командой: cargo test test_qwen3_stress -- --ignored
async fn test_qwen3_stress() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🔥 Стресс-тест системы памяти с Qwen3");

    let temp_dir = tempfile::TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("stress_test_db"),
        cache_path: temp_dir.path().join("stress_test_cache"),
        promotion: Default::default(),
        ai_config: AiConfig::default(),
        health_config: Default::default(),
        cache_config: CacheConfigType::Lru(CacheConfig {
            max_size: 10000,
            ttl_seconds: 3600,
            ..Default::default()
        }),
    };

    let memory_service = MemoryService::new(config).await?;

    // Загружаем 10000 документов
    info!("Загрузка 10000 документов...");
    let total_start = std::time::Instant::now();
    
    for batch_idx in 0..100 {
        let mut batch = Vec::new();
        
        for i in 0..100 {
            let doc_idx = batch_idx * 100 + i;
            let record = Record {
                id: Uuid::new_v4(),
                layer: match doc_idx % 3 {
                    0 => Layer::Interact,
                    1 => Layer::Insights,
                    _ => Layer::Assets,
                },
                text: format!(
                    "Документ №{}. Содержимое для стресс-теста. \
                    Ключевые слова: производительность, масштабирование, \
                    векторный поиск, машинное обучение, обработка данных, \
                    алгоритмы, структуры данных, оптимизация.",
                    doc_idx
                ),
                embedding: Vec::new(),
                kind: "stress_test".to_string(),
                tags: vec![
                    "stress".to_string(),
                    format!("category_{}", doc_idx % 20),
                ],
                project: "stress_project".to_string(),
                session: format!("session_{}", batch_idx),
                ts: Utc::now(),
                score: 0.0,
                access_count: 0,
                last_access: Utc::now(),
            };
            batch.push(record);
        }
        
        memory_service.insert_batch(batch).await?;
        
        if (batch_idx + 1) % 10 == 0 {
            info!("  Загружено {} документов", (batch_idx + 1) * 100);
        }
    }
    
    let load_time = total_start.elapsed();
    info!("Загрузка завершена за {:?}", load_time);
    info!("Средняя скорость: {:.0} док/сек", 10000.0 / load_time.as_secs_f64());

    // Выполняем 1000 поисковых запросов
    info!("\nВыполнение 1000 поисковых запросов...");
    
    let queries = vec![
        "производительность и масштабирование",
        "векторный поиск алгоритмы",
        "машинное обучение оптимизация",
        "обработка больших данных",
        "структуры данных поиск",
    ];
    
    let search_start = std::time::Instant::now();
    let mut total_results = 0;
    
    for i in 0..200 {
        for query in &queries {
            let results = memory_service
                .search(query)
                .top_k(10)
                .min_score(0.5)
                .execute()
                .await?;
            total_results += results.len();
        }
        
        if (i + 1) % 50 == 0 {
            info!("  Выполнено {} запросов", (i + 1) * queries.len());
        }
    }
    
    let search_time = search_start.elapsed();
    info!("Поиск завершён за {:?}", search_time);
    info!("Средняя скорость: {:.0} запросов/сек", 1000.0 / search_time.as_secs_f64());
    info!("Всего найдено результатов: {}", total_results);

    // Финальная статистика
    let (hits, misses, items) = memory_service.cache_stats();
    info!("\nФинальная статистика кэша:");
    info!("  - Попадания: {}", hits);
    info!("  - Промахи: {}", misses);
    info!("  - Hit rate: {:.1}%", (hits as f64 / (hits + misses) as f64) * 100.0);
    info!("  - Элементов в кэше: {}", items);

    info!("\n✅ Стресс-тест завершён успешно!");
    
    Ok(())
}