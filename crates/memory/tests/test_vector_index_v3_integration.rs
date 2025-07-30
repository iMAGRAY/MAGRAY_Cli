use anyhow::Result;
use memory::{
    Layer, MemoryConfig, MemoryService, Record, VectorStore,
};
use tempfile::TempDir;
use tracing_subscriber;

/// Генерирует детерминированный эмбеддинг из текста (мок для тестов)
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

#[tokio::test]
async fn test_vector_store_with_v3_index() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("memory=debug")
        .try_init();

    println!("\n🧪 Тестирование VectorStore с VectorIndexV3...\n");

    // Создаём временную директорию
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    
    // Создаём VectorStore
    let store = VectorStore::new(&db_path).await?;
    
    // Инициализируем слои
    store.init_layer(Layer::Interact).await?;
    store.init_layer(Layer::Insights).await?;
    store.init_layer(Layer::Assets).await?;
    
    println!("✅ VectorStore инициализирован\n");
    
    // Тест 1: Добавление записей в разные слои
    println!("📝 Тест 1: Добавление записей");
    
    let records = vec![
        Record {
            text: "Authentication system using JWT tokens".to_string(),
            embedding: mock_embedding("Authentication system using JWT tokens"),
            layer: Layer::Interact,
            kind: "implementation".to_string(),
            tags: vec!["auth".to_string(), "jwt".to_string()],
            project: "web-api".to_string(),
            session: "session-001".to_string(),
            score: 0.95,
            ..Default::default()
        },
        Record {
            text: "Database migration script for user tables".to_string(),
            embedding: mock_embedding("Database migration script for user tables"),
            layer: Layer::Insights,
            kind: "migration".to_string(),
            tags: vec!["database".to_string(), "sql".to_string()],
            project: "backend".to_string(),
            session: "session-002".to_string(),
            score: 0.85,
            ..Default::default()
        },
        Record {
            text: "React component for user profile page".to_string(),
            embedding: mock_embedding("React component for user profile page"),
            layer: Layer::Assets,
            kind: "component".to_string(),
            tags: vec!["react".to_string(), "frontend".to_string()],
            project: "web-ui".to_string(),
            session: "session-003".to_string(),
            score: 0.90,
            ..Default::default()
        },
    ];
    
    // Добавляем записи
    for record in &records {
        store.insert(record).await?;
        println!("  ✅ Добавлено в {}: {}", 
            match record.layer {
                Layer::Interact => "Interact",
                Layer::Insights => "Insights",
                Layer::Assets => "Assets",
            },
            &record.text[..40.min(record.text.len())]
        );
    }
    
    // Тест 2: Пакетное добавление
    println!("\n📦 Тест 2: Пакетное добавление");
    
    let batch_records = vec![
        Record {
            text: "GraphQL schema definition for API".to_string(),
            embedding: mock_embedding("GraphQL schema definition for API"),
            layer: Layer::Interact,
            kind: "schema".to_string(),
            tags: vec!["graphql".to_string(), "api".to_string()],
            project: "web-api".to_string(),
            session: "session-004".to_string(),
            score: 0.88,
            ..Default::default()
        },
        Record {
            text: "Performance optimization for database queries".to_string(),
            embedding: mock_embedding("Performance optimization for database queries"),
            layer: Layer::Insights,
            kind: "optimization".to_string(),
            tags: vec!["performance".to_string(), "database".to_string()],
            project: "backend".to_string(),
            session: "session-005".to_string(),
            score: 0.92,
            ..Default::default()
        },
    ];
    
    let batch_refs: Vec<&Record> = batch_records.iter().collect();
    store.insert_batch(&batch_refs).await?;
    println!("  ✅ Добавлено {} записей пакетом", batch_records.len());
    
    // Тест 3: Поиск в каждом слое
    println!("\n🔍 Тест 3: Поиск по слоям");
    
    // Поиск в слое Interact
    let query = "authentication API";
    let query_embedding = mock_embedding(query);
    let results = store.search(&query_embedding, Layer::Interact, 3).await?;
    
    println!("\n  Поиск в Interact по запросу '{}':", query);
    for (i, record) in results.iter().enumerate() {
        println!("    {}. {} (score: {:.3})", 
            i + 1, 
            &record.text[..40.min(record.text.len())],
            record.score
        );
    }
    assert!(!results.is_empty(), "Должны быть результаты в слое Interact");
    
    // Поиск в слое Insights
    let query = "database optimization";
    let query_embedding = mock_embedding(query);
    let results = store.search(&query_embedding, Layer::Insights, 3).await?;
    
    println!("\n  Поиск в Insights по запросу '{}':", query);
    for (i, record) in results.iter().enumerate() {
        println!("    {}. {} (score: {:.3})", 
            i + 1, 
            &record.text[..40.min(record.text.len())],
            record.score
        );
    }
    assert!(!results.is_empty(), "Должны быть результаты в слое Insights");
    
    // Тест 4: Удаление записи
    println!("\n🗑️  Тест 4: Удаление записи");
    
    let id_to_delete = records[0].id;
    let deleted = store.delete_by_id(&id_to_delete, Layer::Interact).await?;
    println!("  Удаление записи {}: {}", id_to_delete, 
        if deleted { "✅ успешно" } else { "❌ не найдена" }
    );
    assert!(deleted, "Запись должна быть удалена");
    
    // Проверяем, что запись удалена
    let record = store.get_by_id(&id_to_delete, Layer::Interact).await?;
    assert!(record.is_none(), "Запись не должна существовать после удаления");
    
    // Тест 5: Обновление времени доступа
    println!("\n⏰ Тест 5: Обновление времени доступа");
    
    let id_to_update = &records[1].id.to_string();
    store.update_access(Layer::Insights, id_to_update).await?;
    println!("  ✅ Обновлено время доступа для записи в Insights");
    
    // Тест 6: Получение кандидатов для продвижения
    println!("\n📈 Тест 6: Кандидаты для продвижения");
    
    use chrono::Utc;
    let candidates = store.get_promotion_candidates(
        Layer::Insights,
        Utc::now() + chrono::Duration::hours(1), // Записи старше часа назад
        0.8, // Минимальный score
        0,   // Минимальное количество доступов
    ).await?;
    
    println!("  Найдено {} кандидатов для продвижения из Insights", candidates.len());
    
    // Тест 7: Удаление старых записей
    println!("\n🧹 Тест 7: Удаление старых записей");
    
    let deleted_count = store.delete_expired(
        Layer::Interact,
        Utc::now() + chrono::Duration::hours(1), // Удалить записи старше часа назад
    ).await?;
    
    println!("  Удалено {} старых записей из Interact", deleted_count);
    
    println!("\n✅ Все тесты интеграции VectorStore завершены успешно!");
    
    Ok(())
}

#[tokio::test] 
async fn test_memory_service_with_v3_index() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("memory=info")
        .try_init();

    println!("\n🧪 Тестирование MemoryService с VectorIndexV3...\n");

    // Создаём временные директории
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("memory_db"),
        cache_path: temp_dir.path().join("cache_db"),
        ..Default::default()
    };
    
    // Создаём MemoryService (он использует мок-эмбеддинги из AI сервиса)
    let mut service = MemoryService::new(config).await?;
    
    // Включаем метрики
    service.enable_metrics();
    
    println!("✅ MemoryService инициализирован\n");
    
    // Тест 1: Сохранение воспоминаний
    println!("💾 Тест 1: Сохранение воспоминаний");
    
    let memories = vec![
        ("Implemented OAuth2 authentication flow", Layer::Interact, vec!["auth", "oauth2"]),
        ("Optimized database queries reduced latency by 50%", Layer::Insights, vec!["performance", "database"]),
        ("Architecture decision: use microservices pattern", Layer::Assets, vec!["architecture", "design"]),
    ];
    
    for (text, layer, tags) in &memories {
        let record = Record {
            text: text.to_string(),
            layer: *layer,
            kind: "test".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            project: "test-project".to_string(),
            session: "test-session".to_string(),
            ..Default::default()
        };
        
        service.insert(record).await?;
        
        println!("  ✅ Сохранено в {:?}: {}", layer, &text[..30.min(text.len())]);
    }
    
    // Тест 2: Поиск с помощью SearchBuilder
    println!("\n🔍 Тест 2: Поиск с SearchBuilder");
    
    let results = service
        .search("authentication security")
        .with_layer(Layer::Interact)
        .with_tags(vec!["auth".to_string()])
        .top_k(5)
        .execute()
        .await?;
    
    println!("  Результаты поиска 'authentication security' в Interact:");
    for (i, record) in results.iter().enumerate() {
        println!("    {}. {} (score: {:.3})", 
            i + 1, 
            &record.text[..30.min(record.text.len())],
            record.score
        );
    }
    
    // Тест 3: Межслойный поиск
    println!("\n🔍 Тест 3: Поиск по всем слоям");
    
    let results = service
        .search("database optimization performance")
        .top_k(5)
        .execute()
        .await?;
    
    println!("  Результаты поиска 'database optimization performance' по всем слоям:");
    for (i, record) in results.iter().enumerate() {
        println!("    {}. [{:?}] {} (score: {:.3})", 
            i + 1,
            record.layer,
            &record.text[..30.min(record.text.len())],
            record.score
        );
    }
    
    // Тест 4: Получение метрик
    println!("\n📊 Тест 4: Метрики системы");
    
    // Обновляем метрики слоёв
    service.update_layer_metrics().await?;
    
    // Получаем статистику кэша
    let (cache_entries, _, _) = service.cache_stats();
    let cache_hit_rate = service.cache_hit_rate();
    
    println!("  Метрики MemoryService:");
    println!("    Записей в кэше: {}", cache_entries);
    println!("    Процент попаданий в кэш: {:.1}%", cache_hit_rate * 100.0);
    
    if let Some(metrics) = service.metrics() {
        // Выводим основные метрики
        metrics.log_summary();
    } else {
        println!("  Детальные метрики не включены");
    }
    
    println!("\n✅ Все тесты MemoryService завершены успешно!");
    
    Ok(())
}