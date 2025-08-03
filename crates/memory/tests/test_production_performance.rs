use anyhow::Result;
use chrono::Utc;
use memory::*;
use std::sync::Arc;
use std::time::Instant;
use tokio;
use uuid::Uuid;

/// Производственные performance тесты для системы памяти
#[tokio::test]
async fn test_large_scale_vector_operations() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    
    // Конфигурация для production нагрузки
    let config = MemoryConfig {
        db_path: temp_dir.path().join("production_test"),
        cache_path: temp_dir.path().join("cache"),
        promotion: PromotionConfig::default(),
        ai_config: ai::AiConfig::default(),
        health_config: HealthConfig::default(),
        cache_config: CacheConfigType::Lru(memory::CacheConfig::default()),
        resource_config: memory::ResourceConfig {
            base_max_vectors: 100_000, // 100K vectors для теста
            base_cache_size_bytes: 100 * 1024 * 1024, // 100MB cache
            target_memory_usage_percent: 80,
            ..memory::ResourceConfig::default()
        },
        #[allow(deprecated)]
        max_vectors: 100_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 100 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(80),
    };
    
    let memory_service = Arc::new(MemoryService::new(config).await?);
    
    println!("🚀 Starting large-scale performance test...");
    
    // Тест 1: Массовая вставка векторов
    let insert_start = Instant::now();
    let mut records = Vec::new();
    
    for i in 0..5000 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Performance test document {} with detailed content for realistic simulation", i),
            embedding: vec![0.1 * i as f32; 1024], // BGE-M3 размерность
            layer: if i % 3 == 0 { Layer::Interact } else if i % 3 == 1 { Layer::Insights } else { Layer::Assets },
            kind: "performance_test".to_string(),
            tags: vec!["test".to_string(), format!("batch_{}", i / 100)],
            project: "performance_benchmark".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            score: 0.8 + (i % 100) as f32 / 500.0,
            access_count: (i % 10) as u32,
            last_access: Utc::now(),
        };
        records.push(record);
    }
    
    // Batch insert performance
    memory_service.insert_batch(records).await?;
    let insert_duration = insert_start.elapsed();
    println!("✅ Inserted 5000 records in {:?} ({:.2} records/sec)", 
             insert_duration, 5000.0 / insert_duration.as_secs_f64());
    
    // Тест 2: Производительность поиска
    let search_start = Instant::now();
    let mut total_results = 0;
    
    for i in 0..100 {
        let query = format!("Performance test document {}", i * 17); // Разные запросы
        let results = memory_service
            .search(&query)
            .with_layers(&[Layer::Interact, Layer::Insights])
            .top_k(10)
            .execute()
            .await?;
        total_results += results.len();
    }
    
    let search_duration = search_start.elapsed();
    println!("✅ Executed 100 searches in {:?} ({:.2} searches/sec, {} total results)", 
             search_duration, 100.0 / search_duration.as_secs_f64(), total_results);
    
    // Тест 3: Promotion cycle performance
    let promotion_start = Instant::now();
    let promotion_stats = memory_service.run_promotion_cycle().await?;
    let promotion_duration = promotion_start.elapsed();
    
    println!("✅ Promotion cycle completed in {:?}", promotion_duration);
    println!("   Promoted: {} Interact->Insights, {} Insights->Assets", 
             promotion_stats.interact_to_insights, promotion_stats.insights_to_assets);
    
    // Тест 4: Concurrent operations stress test
    let concurrent_start = Instant::now();
    let mut handles = Vec::new();
    
    for worker_id in 0..10 {
        let service = memory_service.clone();
        let handle = tokio::spawn(async move {
            let mut results = 0;
            for i in 0..20 {
                let query = format!("Worker {} query {}", worker_id, i);
                if let Ok(search_results) = service
                    .search(&query)
                    .top_k(5)
                    .execute()
                    .await 
                {
                    results += search_results.len();
                }
            }
            results
        });
        handles.push(handle);
    }
    
    let mut concurrent_results = 0;
    for handle in handles {
        concurrent_results += handle.await?;
    }
    
    let concurrent_duration = concurrent_start.elapsed();
    println!("✅ 10 concurrent workers, 200 total operations in {:?} ({:.2} ops/sec)", 
             concurrent_duration, 200.0 / concurrent_duration.as_secs_f64());
    
    // Тест 5: Memory usage и система здоровья
    let health_status = memory_service.run_health_check().await?;
    println!("✅ System health: {:?}", health_status);
    
    let (cache_hits, cache_misses, cache_total) = memory_service.cache_stats();
    println!("✅ Cache performance: {}/{} hits ({:.2}% hit rate)", 
             cache_hits, cache_total, memory_service.cache_hit_rate() * 100.0);
    
    // Проверяем производственные требования
    assert!(insert_duration.as_secs() < 10, "Insert too slow: {:?}", insert_duration);
    assert!(search_duration.as_millis() < 5000, "Search too slow: {:?}", search_duration);
    assert!(promotion_duration.as_secs() < 5, "Promotion too slow: {:?}", promotion_duration);
    assert!(concurrent_duration.as_secs() < 15, "Concurrent ops too slow: {:?}", concurrent_duration);
    
    println!("🎉 All performance tests passed!");
    Ok(())
}

#[tokio::test]
async fn test_incremental_sync_performance() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("sync_test"),
        ..Default::default()
    };
    
    let memory_service = Arc::new(MemoryService::new(config).await?);
    println!("🔄 Testing incremental sync performance...");
    
    // Создаём базовый набор данных
    let mut base_records = Vec::new();
    for i in 0..1000 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Base record {}", i),
            embedding: vec![0.5; 1024],
            layer: Layer::Interact,
            kind: "base".to_string(),
            tags: vec!["base".to_string()],
            project: "sync_test".to_string(),
            session: "base_session".to_string(),
            ts: Utc::now(),
            score: 0.7,
            access_count: 1,
            last_access: Utc::now(),
        };
        base_records.push(record);
    }
    
    memory_service.insert_batch(base_records).await?;
    
    // Тестируем incremental добавления
    let incremental_start = Instant::now();
    
    for batch in 0..10 {
        let mut incremental_records = Vec::new();
        for i in 0..50 {
            let record = Record {
                id: Uuid::new_v4(),
                text: format!("Incremental record batch {} item {}", batch, i),
                embedding: vec![0.3 + batch as f32 * 0.1; 1024],
                layer: Layer::Interact,
                kind: "incremental".to_string(),
                tags: vec!["incremental".to_string()],
                project: "sync_test".to_string(),
                session: "incremental_session".to_string(),
                ts: Utc::now(),
                score: 0.6,
                access_count: 1,
                last_access: Utc::now(),
            };
            incremental_records.push(record);
        }
        
        memory_service.insert_batch(incremental_records).await?;
        
        // Тестируем что smart sync работает быстро
        let store = memory_service.get_store();
        let sync_start = Instant::now();
        store.smart_sync_if_needed(Layer::Interact).await?;
        let sync_duration = sync_start.elapsed();
        
        println!("Batch {} sync took: {:?}", batch, sync_duration);
        assert!(sync_duration.as_millis() < 100, "Incremental sync too slow: {:?}", sync_duration);
    }
    
    let total_incremental_time = incremental_start.elapsed();
    println!("✅ Incremental sync test completed in {:?}", total_incremental_time);
    
    Ok(())
}

#[tokio::test] 
async fn test_memory_limits_and_scaling() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("limits_test"),
        max_vectors: 1000, // Низкий лимит для тестирования
        ..Default::default()
    };
    
    let memory_service = Arc::new(MemoryService::new(config).await?);
    println!("🚧 Testing memory limits and error handling...");
    
    // Заполняем до лимита
    let mut records = Vec::new();
    for i in 0..900 { // Близко к лимиту но не превышаем
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Limit test record {}", i),
            embedding: vec![0.4; 1024],
            layer: Layer::Interact,
            kind: "limit_test".to_string(),
            tags: vec!["limit".to_string()],
            project: "limit_test".to_string(),
            session: "limit_session".to_string(),
            ts: Utc::now(),
            score: 0.5,
            access_count: 1,
            last_access: Utc::now(),
        };
        records.push(record);
    }
    
    memory_service.insert_batch(records).await?;
    
    // Проверяем статистику памяти
    let store = memory_service.get_store();
    let memory_stats = store.memory_stats();
    println!("Memory usage: {} vectors total", memory_stats.total_vectors);
    
    let capacity_usage = store.capacity_usage();
    for (layer, usage) in capacity_usage {
        println!("Layer {:?}: {:.1}% capacity used", layer, usage);
        assert!(usage < 100.0, "Layer {:?} exceeded capacity: {:.1}%", layer, usage);
    }
    
    // Пытаемся превысить лимит - должна быть ошибка
    let over_limit_record = Record {
        id: Uuid::new_v4(),
        text: "This should exceed limit".to_string(),
        embedding: vec![0.9; 1024],
        layer: Layer::Interact,
        kind: "over_limit".to_string(),
        tags: vec!["over_limit".to_string()],
        project: "limit_test".to_string(),
        session: "limit_session".to_string(),
        ts: Utc::now(),
        score: 0.9,
        access_count: 1,
        last_access: Utc::now(),
    };
    
    // Добавляем достаточно записей чтобы превысить лимит
    let mut over_limit_batch = Vec::new();
    for i in 0..200 {
        let mut record = over_limit_record.clone();
        record.id = Uuid::new_v4();
        record.text = format!("Over limit record {}", i);
        over_limit_batch.push(record);
    }
    
    let result = memory_service.insert_batch(over_limit_batch).await;
    assert!(result.is_err(), "Should have failed due to capacity limits");
    
    println!("✅ Memory limits properly enforced");
    Ok(())
}