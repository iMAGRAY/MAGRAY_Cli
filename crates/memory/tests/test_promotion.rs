#![cfg(all(feature = "extended-tests", feature = "persistence", feature = "legacy-tests"))]

use anyhow::Result;
use chrono::{Duration, Utc};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use ai::AiConfig;
use memory::{
    promotion::PromotionEngine, storage::VectorStore, Layer, MemoryContext, PromotionConfig, Record,
};

#[tokio::test]
async fn test_promotion_engine() -> Result<()> {
    // Create temporary directories
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let cache_path = temp_dir.path().join("test_cache");

    // Configure with short TTLs for testing
    let config = MemoryConfig {
        db_path: db_path.clone(),
        cache_path: cache_path.clone(),
        promotion: PromotionConfig {
            interact_ttl_hours: 1,  // 1 hour for testing
            insights_ttl_days: 1,   // 1 day for testing
            promote_threshold: 0.5, // Lower threshold for testing
            decay_factor: 0.9,
        },
        ai_config: AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfig::default(),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
        ..Default::default()
    };

    // Initialize memory service
    let memory_service = MemoryService::new(config).await?;

    // Insert test records with different ages and access patterns
    let now = Utc::now();

    // Old, frequently accessed record (should be promoted)
    let old_popular = Record {
        id: Uuid::new_v4(),
        text: "Popular old content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["popular".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.1; 768],    // Mock embedding
        ts: now - Duration::hours(2), // 2 hours old
        last_access: now - Duration::minutes(10),
        access_count: 10,
        score: 0.8,
    };

    // Old, rarely accessed record (should expire)
    let old_unpopular = Record {
        id: Uuid::new_v4(),
        text: "Unpopular old content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["unpopular".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.2; 768],
        ts: now - Duration::hours(3), // 3 hours old
        last_access: now - Duration::hours(3),
        access_count: 1,
        score: 0.3,
    };

    // New record (should stay in Interact)
    let new_record = Record {
        id: Uuid::new_v4(),
        text: "Fresh content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["new".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.3; 768],
        ts: now - Duration::minutes(30), // 30 minutes old
        last_access: now,
        access_count: 5,
        score: 0.9,
    };

    // Insert all records
    memory_service.insert(old_popular.clone()).await?;
    memory_service.insert(old_unpopular.clone()).await?;
    memory_service.insert(new_record.clone()).await?;

    println!("✅ Inserted 3 test records");

    // Run promotion cycle
    let stats = memory_service.run_promotion_cycle().await?;

    println!("\n📊 Promotion Stats:");
    println!("  Interact → Insights: {}", stats.interact_to_insights);
    println!("  Insights → Assets: {}", stats.insights_to_assets);
    println!("  Expired Interact: {}", stats.expired_interact);
    println!("  Expired Insights: {}", stats.expired_insights);

    // Verify old popular record was promoted to Insights
    let promoted = memory_service
        .get_by_id(&old_popular.id, Layer::Insights)
        .await?;
    assert!(
        promoted.is_some(),
        "Popular record should be promoted to Insights"
    );

    // Verify it was removed from Interact
    let in_interact = memory_service
        .get_by_id(&old_popular.id, Layer::Interact)
        .await?;
    assert!(
        in_interact.is_none(),
        "Promoted record should be removed from Interact"
    );

    // Verify new record stays in Interact
    let still_new = memory_service
        .get_by_id(&new_record.id, Layer::Interact)
        .await?;
    assert!(still_new.is_some(), "New record should remain in Interact");

    // Search in Insights layer
    let insights_results = memory_service
        .search("popular")
        .with_layer(Layer::Insights)
        .execute()
        .await?;

    assert_eq!(
        insights_results.len(),
        1,
        "Should find promoted record in Insights"
    );
    assert_eq!(insights_results[0].id, old_popular.id);

    println!("\n✅ All promotion tests passed!");

    Ok(())
}

#[tokio::test]
async fn test_layer_ttl_expiration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let cache_path = temp_dir.path().join("test_cache");

    let config = MemoryConfig {
        db_path,
        cache_path,
        promotion: PromotionConfig {
            interact_ttl_hours: 1,
            insights_ttl_days: 1,
            promote_threshold: 0.7,
            decay_factor: 0.9,
        },
        ai_config: AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfig::default(),
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
    let now = Utc::now();

    // Insert very old record that should be expired
    let ancient_record = Record {
        id: Uuid::new_v4(),
        text: "Ancient content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["ancient".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.4; 768],
        ts: now - Duration::hours(10), // 10 hours old (way past TTL)
        last_access: now - Duration::hours(10),
        access_count: 0,
        score: 0.1,
    };

    memory_service.insert(ancient_record.clone()).await?;

    // Run promotion cycle
    let stats = memory_service.run_promotion_cycle().await?;

    // Should have expired the ancient record
    assert!(stats.expired_interact > 0, "Should expire old records");

    // Verify it's gone
    let gone = memory_service
        .get_by_id(&ancient_record.id, Layer::Interact)
        .await?;
    assert!(gone.is_none(), "Ancient record should be expired");

    println!("✅ TTL expiration test passed!");

    Ok(())
}

#[tokio::test]
async fn test_time_based_indices_performance() -> Result<()> {
    // Создаем временную директорию для БД
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");

    // Создаем VectorStore напрямую для теста индексов
    let store = Arc::new(VectorStore::new(&db_path).await?);

    // Создаем PromotionEngine с конфигурацией
    let config = PromotionConfig {
        interact_ttl_hours: 24,
        insights_ttl_days: 7,
        promote_threshold: 0.7,
        decay_factor: 0.9,
    };

    let sled_db = sled::open(&db_path)?;
    let engine = PromotionEngine::new(store.clone(), config, Arc::new(sled_db)).await?;

    // Создаем тестовые записи с разными временными метками
    let now = Utc::now();
    let mut records = Vec::new();

    // Старые записи (кандидаты на promotion)
    for i in 0..100 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("Old record {}", i),
            kind: "test".to_string(),
            tags: vec!["old".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.1; 768],
            ts: now - Duration::hours(30 + i as i64), // 30+ часов назад
            last_access: now - Duration::hours(1),
            access_count: 3 + (i % 5) as u32,
            score: 0.8 + (i as f32 / 1000.0),
        };
        records.push(record);
    }

    // Новые записи (не должны быть promoted)
    for i in 0..100 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("New record {}", i),
            kind: "test".to_string(),
            tags: vec!["new".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.2; 768],
            ts: now - Duration::hours(5), // 5 часов назад
            last_access: now,
            access_count: 1,
            score: 0.9,
        };
        records.push(record);
    }

    // Вставляем все записи
    println!("Вставка {} тестовых записей...", records.len());
    let start_insert = std::time::Instant::now();
    let record_refs: Vec<&Record> = records.iter().collect();
    store.insert_batch(&record_refs).await?;
    println!("Вставка завершена за {:?}", start_insert.elapsed());

    // Измеряем время поиска кандидатов с time-based индексами
    println!("\nТестирование производительности time-based индексов...");
    let start_search = std::time::Instant::now();
    let stats = engine.run_promotion_cycle().await?;
    let search_duration = start_search.elapsed();

    println!("Результаты promotion цикла:");
    println!(
        "  - Promoted Interact->Insights: {}",
        stats.interact_to_insights
    );
    println!("  - Expired Interact: {}", stats.expired_interact);
    println!("  - Общее время: {}ms", stats.total_time_ms);
    println!(
        "  - Время обновления индексов: {}ms",
        stats.index_update_time_ms
    );
    println!("  - Время promotion: {}ms", stats.promotion_time_ms);
    println!("  - Время cleanup: {}ms", stats.cleanup_time_ms);

    // Проверяем что поиск быстрый
    assert!(
        search_duration.as_millis() < 1000,
        "Поиск слишком медленный: {:?}",
        search_duration
    );

    // Проверяем что promoted правильное количество
    assert!(
        stats.interact_to_insights > 50,
        "Слишком мало записей promoted"
    );
    assert!(
        stats.interact_to_insights < 150,
        "Слишком много записей promoted"
    );

    // Получаем статистику производительности
    let perf_stats = engine.get_performance_stats().await?;
    println!("\nСтатистика индексов:");
    println!(
        "  - Interact time index: {} записей",
        perf_stats.interact_time_index_size
    );
    println!(
        "  - Interact score index: {} записей",
        perf_stats.interact_score_index_size
    );
    println!(
        "  - Insights time index: {} записей",
        perf_stats.insights_time_index_size
    );

    // Второй цикл должен быть еще быстрее (инкрементальное обновление)
    println!("\nТестирование инкрементального обновления индексов...");
    let start_incremental = std::time::Instant::now();
    let stats2 = engine.run_promotion_cycle().await?;
    let _incremental_duration = start_incremental.elapsed();

    println!(
        "Инкрементальный цикл завершен за {}ms",
        stats2.total_time_ms
    );
    assert!(
        stats2.total_time_ms < stats.total_time_ms / 2,
        "Инкрементальное обновление должно быть намного быстрее"
    );

    println!("\n✅ Все тесты time-based индексов пройдены!");

    Ok(())
}
