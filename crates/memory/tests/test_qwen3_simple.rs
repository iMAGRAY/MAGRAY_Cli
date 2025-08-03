use memory::{MemoryService, MemoryConfig, Layer, Record, CacheConfigType, CacheConfig};
use ai::AiConfig;
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use tracing::info;
use tracing_subscriber;

/// Простой тест системы памяти с моделями Qwen3
#[tokio::test]
async fn test_qwen3_memory_basic() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,memory=debug,ai=debug")
        .try_init();

    info!("🚀 Запуск теста системы памяти с Qwen3");

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
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
    };

    // Создаём сервис памяти
    info!("Создание сервиса памяти...");
    let memory_service = MemoryService::new(config).await?;
    info!("✅ Сервис памяти создан");

    // Тест 1: Добавление записей
    info!("\n📝 Тест 1: Добавление записей");
    
    let test_texts = vec![
        ("Rust - это системный язык программирования", Layer::Interact),
        ("Tokio - асинхронный runtime для Rust", Layer::Interact),
        ("ONNX Runtime поддерживает различные модели", Layer::Assets),
        ("Qwen3 - семейство языковых моделей", Layer::Assets),
    ];

    for (text, layer) in test_texts {
        let record = Record {
            id: Uuid::new_v4(),
            layer,
            text: text.to_string(),
            embedding: Vec::new(),
            kind: "test".to_string(),
            tags: vec!["test".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        
        info!("  Добавление: {}", text);
        let start = std::time::Instant::now();
        memory_service.insert(record).await?;
        let insert_time = start.elapsed();
        info!("  ✓ Добавлено за {:?}", insert_time);
    }

    // Даём время на индексацию
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Тест 2: Поиск
    info!("\n🔍 Тест 2: Поиск с Qwen3 embeddings");
    
    let query = "язык программирования";
    info!("  Запрос: '{}'", query);
    
    let start = std::time::Instant::now();
    let results = memory_service
        .search(query)
        .top_k(3)
        .min_score(0.5)
        .execute()
        .await?;
    let search_time = start.elapsed();
    
    info!("  Время поиска: {:?}", search_time);
    info!("  Найдено результатов: {}", results.len());
    
    for (i, result) in results.iter().enumerate() {
        info!("    {}. [{:?}] {}", i + 1, result.layer, result.text);
    }

    // Тест 3: Батч-загрузка
    info!("\n📦 Тест 3: Батч-загрузка");
    
    let mut batch = Vec::new();
    for i in 0..10 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("Тестовая запись №{} для батч-загрузки", i),
            embedding: Vec::new(),
            kind: "batch".to_string(),
            tags: vec!["batch".to_string()],
            project: "test_project".to_string(),
            session: "batch_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        batch.push(record);
    }

    let start = std::time::Instant::now();
    for record in batch {
        memory_service.insert(record).await?;
    }
    let batch_time = start.elapsed();
    
    info!("  Добавлено 10 записей за {:?}", batch_time);

    // Тест 4: Статистика кэша
    info!("\n📊 Тест 4: Статистика");
    
    let (hits, misses, items) = memory_service.cache_stats();
    let hit_rate = if hits + misses > 0 {
        (hits as f64 / (hits + misses) as f64) * 100.0
    } else {
        0.0
    };
    
    info!("  Статистика кэша:");
    info!("    - Попадания: {}", hits);
    info!("    - Промахи: {}", misses);
    info!("    - Элементов: {}", items);
    info!("    - Hit rate: {:.1}%", hit_rate);

    // Тест 5: Проверка здоровья системы
    info!("\n🏥 Тест 5: Здоровье системы");
    
    let health = memory_service.run_health_check().await?;
    info!("  Статус системы: {:?}", health.overall_status);
    info!("  Компоненты:");
    for (component, status) in &health.component_statuses {
        info!("    - {:?}: {:?}", component, status);
    }

    // Тест 6: Многоязычность
    info!("\n🌍 Тест 6: Многоязычная поддержка Qwen3");
    
    let multilingual_texts = vec![
        ("Hello, world!", "en"),
        ("Привет, мир!", "ru"),
        ("你好，世界！", "zh"),
    ];
    
    for (text, lang) in multilingual_texts {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Assets,
            text: text.to_string(),
            embedding: Vec::new(),
            kind: "multilingual".to_string(),
            tags: vec![lang.to_string()],
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
    let query = "привет мир";
    info!("\n  Поиск '{}' среди многоязычных текстов:", query);
    
    let results = memory_service
        .search(query)
        .with_layer(Layer::Assets)
        .with_tags(vec!["ru".to_string(), "en".to_string(), "zh".to_string()])
        .top_k(3)
        .execute()
        .await?;
    
    for result in results {
        let default_lang = "?".to_string();
        let lang = result.tags.first().unwrap_or(&default_lang);
        info!("    [{}] {}", lang, result.text);
    }

    info!("\n✅ Тест успешно завершён!");
    info!("🎉 Система памяти MAGRAY с моделями Qwen3 работает!");
    
    Ok(())
}