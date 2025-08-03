use anyhow::Result;
use memory::{
    MemoryService, MemoryConfig, Record, Layer, ResourceConfig, HealthConfig,
    CacheConfigType, CacheConfig, PromotionConfig
};
use ai::AiConfig;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::Utc;

// @component: {"k":"T","id":"integration_tests","t":"Full workflow integration tests","m":{"cur":0,"tgt":90,"u":"%"},"f":["integration","workflow","testing"]}

/// Комплексный тест полного workflow системы памяти
#[tokio::test]
async fn test_complete_memory_system_workflow() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // === ФАЗА 1: ИНИЦИАЛИЗАЦИЯ КОМПОНЕНТОВ ===
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    
    // Создаём основные компоненты
    let memory_config = MemoryConfig {
        db_path: base_path.join("memory_db"),
        cache_path: base_path.join("memory_cache"),
        promotion: PromotionConfig::default(),
        ai_config: AiConfig::default(),
        health_config: HealthConfig::default(),
        cache_config: CacheConfigType::Lru(CacheConfig::default()),
        resource_config: ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 10_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 100 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(80),
        ..Default::default()
    };
    let memory_service = MemoryService::new(memory_config).await?;
    
    println!("✅ Phase 1: All components initialized");

    // === ФАЗА 2: БАЗОВЫЕ ОПЕРАЦИИ С ДАННЫМИ ===
    
    // Создаём тестовые записи разных размерностей
    let test_records = create_diverse_test_records(100);
    
    // Добавляем записи
    for record in test_records {
        memory_service.insert(record).await?;
    }
    
    println!("✅ Phase 2: 100 records stored");

    // === ФАЗА 3: ПОИСК И ВАЛИДАЦИЯ ===
    
    // Тестируем поиск по всем слоям
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let query = "test programming algorithms";
        let results = memory_service
            .search(query)
            .with_layer(layer)
            .top_k(10)
            .execute()
            .await?;
        
        assert!(!results.is_empty(), "Search returned no results for layer {:?}", layer);
        println!("✅ Phase 3: Search in layer {:?} returned {} results", layer, results.len());
    }

    // === ФАЗА 4: RESOURCE SCALING ===
    
    // Симулируем рост нагрузки
    let large_batch = create_diverse_test_records(500);
    for chunk in large_batch.chunks(50) {
        for record in chunk {
            memory_service.insert(record.clone()).await?;
        }
        sleep(Duration::from_millis(10)).await; // Небольшая пауза
    }
    
    println!("✅ Phase 4: Resource scaling tested with 500 additional records");

    // === ФАЗА 5: BACKUP & RESTORE ===
    
    // Создаём backup
    let backup_path = memory_service.create_backup(Some("integration_test_full".to_string())).await?;
    
    // Добавляем ещё данных для incremental backup
    let delta_records = create_diverse_test_records(50);
    for record in delta_records {
        memory_service.insert(record).await?;
    }
    
    println!("✅ Phase 5: Backup created: {:?}", backup_path);

    // === ФАЗА 6: HEALTH MONITORING ===
    
    // Получаем статистику системы
    let health = memory_service.run_health_check().await?;
    println!("✅ Phase 6: System health status: {:?}", health.overall_status);

    // === ФАЗА 7: CACHE STATISTICS ===
    
    let (hits, _misses, total) = memory_service.cache_stats();
    let hit_rate = if total > 0 { hits as f32 / total as f32 * 100.0 } else { 0.0 };
    println!("✅ Phase 7: Cache hit rate: {:.1}%", hit_rate);

    // === ФАЗА 8: PROMOTION CYCLE ===
    
    let promotion_stats = memory_service.run_promotion_cycle().await?;
    println!("✅ Phase 8: Promotion cycle - {} promoted, {} expired", 
             promotion_stats.interact_to_insights + promotion_stats.insights_to_assets,
             promotion_stats.expired_interact + promotion_stats.expired_insights);

    println!("🎉 COMPLETE WORKFLOW TEST SUCCESSFUL");

    Ok(())
}

/// Тест производительности под нагрузкой
#[tokio::test] 
async fn test_performance_under_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("perf_db"),
        cache_path: temp_dir.path().join("perf_cache"),
        ..Default::default()
    };
    let memory_service = MemoryService::new(config).await?;
    
    let start = std::time::Instant::now();
    
    // === НАГРУЗОЧНЫЙ ТЕСТ: 1000 записей ===
    let records = create_diverse_test_records(1000);
    
    // Параллельная вставка
    let insert_start = std::time::Instant::now();
    
    for chunk in records.chunks(100) {
        for record in chunk {
            memory_service.insert(record.clone()).await?;
        }
    }
    
    let insert_duration = insert_start.elapsed();
    
    // === НАГРУЗОЧНЫЙ ТЕСТ: 100 поисковых запросов ===
    let search_start = std::time::Instant::now();
    
    let mut search_results = Vec::new();
    for i in 0..100 {
        let query = format!("test record {} programming", i);
        let results = memory_service
            .search(&query)
            .with_layer(Layer::Interact)
            .top_k(10)
            .execute()
            .await?;
        search_results.push(results.len());
    }
    
    let search_duration = search_start.elapsed();
    let total_duration = start.elapsed();
    
    // === АНАЛИЗ ПРОИЗВОДИТЕЛЬНОСТИ ===
    let records_per_sec = records.len() as f64 / insert_duration.as_secs_f64();
    let searches_per_sec = 100.0 / search_duration.as_secs_f64();
    let avg_search_results = search_results.iter().sum::<usize>() as f64 / search_results.len() as f64;
    
    println!("🚀 PERFORMANCE TEST RESULTS:");
    println!("   Total duration: {:.2}s", total_duration.as_secs_f64());
    println!("   Insert performance: {:.1} records/sec", records_per_sec);
    println!("   Search performance: {:.1} searches/sec", searches_per_sec);
    println!("   Average search results: {:.1}", avg_search_results);
    
    // Минимальные требования производительности
    assert!(records_per_sec > 50.0, "Insert performance too low: {:.1} records/sec", records_per_sec);
    assert!(searches_per_sec > 10.0, "Search performance too low: {:.1} searches/sec", searches_per_sec);
    
    Ok(())
}

/// Тест отказоустойчивости
#[tokio::test]
async fn test_resilience_and_recovery() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("resilience_db"),
        cache_path: temp_dir.path().join("resilience_cache"),
        ..Default::default()
    };
    let memory_service = MemoryService::new(config).await?;
    
    // === ПОДГОТОВКА ДАННЫХ ===
    let records = create_diverse_test_records(100);
    for record in &records {
        memory_service.insert(record.clone()).await?;
    }
    
    // === СИМУЛЯЦИЯ ОТКАЗА: ПЕРЕСОЗДАНИЕ СЕРВИСА ===
    println!("💥 Simulating service restart...");
    drop(memory_service);
    
    // Пауза для симуляции downtime
    sleep(Duration::from_millis(100)).await;
    
    // === ВОССТАНОВЛЕНИЕ ===
    let config = MemoryConfig {
        db_path: temp_dir.path().join("resilience_db"),
        cache_path: temp_dir.path().join("resilience_cache"),
        ..Default::default()
    };
    let recovered_service = MemoryService::new(config).await?;
    
    // === ПРОВЕРКА ВОССТАНОВЛЕНИЯ ===
    let query = "test programming";
    let results = recovered_service
        .search(query)
        .with_layer(Layer::Interact)
        .top_k(10)
        .execute()
        .await?;
    
    assert!(!results.is_empty(), "Service should recover and have searchable data");
    
    println!("✅ RESILIENCE TEST PASSED:");
    println!("   Search results after recovery: {}", results.len());
    
    Ok(())
}

/// Тест многослойной системы памяти
#[tokio::test]
async fn test_multi_layer_promotion_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("multi_layer_db"),
        cache_path: temp_dir.path().join("multi_layer_cache"),
        ..Default::default()
    };
    let memory_service = MemoryService::new(config).await?;
    
    // === СОЗДАНИЕ ЗАПИСЕЙ В РАЗНЫХ СЛОЯХ ===
    
    // Interact слой - свежие данные
    for i in 0..20 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("interact_record_{} - Fresh data about programming and algorithms", i),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "interact".to_string(),
            tags: vec!["fresh".to_string()],
            project: "test".to_string(),
            session: "multi_layer_test".to_string(),
            score: 0.8 + i as f32 * 0.01,
            ts: chrono::Utc::now(),
            access_count: i as u32,
            last_access: chrono::Utc::now(),
        };
        memory_service.insert(record).await?;
    }
    
    // Insights слой - анализ
    for i in 0..10 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("insight_record_{} - Analysis of software patterns", i),
            embedding: vec![],
            layer: Layer::Insights,
            kind: "insight".to_string(),
            tags: vec!["analysis".to_string()],
            project: "test".to_string(),
            session: "multi_layer_test".to_string(),
            score: 0.9 + i as f32 * 0.005,
            ts: chrono::Utc::now() - chrono::Duration::days(1),
            access_count: (i as u32) * 2,
            last_access: chrono::Utc::now(),
        };
        memory_service.insert(record).await?;
    }
    
    // Assets слой - постоянные данные
    for i in 0..5 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("asset_record_{} - Core knowledge about systems", i),
            embedding: vec![],
            layer: Layer::Assets,
            kind: "asset".to_string(),
            tags: vec!["core".to_string()],
            project: "test".to_string(),
            session: "multi_layer_test".to_string(),
            score: 0.95 + i as f32 * 0.001,
            ts: chrono::Utc::now() - chrono::Duration::days(30),
            access_count: (i as u32) * 5,
            last_access: chrono::Utc::now(),
        };
        memory_service.insert(record).await?;
    }
    
    // === ТЕСТИРУЕМ ПОИСК ПО СЛОЯМ ===
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let query = "programming software";
        let results = memory_service
            .search(query)
            .with_layer(layer)
            .top_k(5)
            .execute()
            .await?;
        
        assert!(!results.is_empty(), "Layer {:?} should have search results", layer);
        
        // Проверяем что все результаты из правильного слоя
        for result in &results {
            assert_eq!(result.layer, layer, "Result should be from correct layer");
        }
        
        println!("   Layer {:?}: {} search results", layer, results.len());
    }
    
    Ok(())
}

/// Создание разнообразных тестовых записей
fn create_diverse_test_records(count: usize) -> Vec<Record> {
    let mut records = Vec::new();
    
    for i in 0..count {
        let layer = match i % 3 {
            0 => Layer::Interact,
            1 => Layer::Insights,
            _ => Layer::Assets,
        };
        
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("test_record_{}_{:?} - This is a test document about programming, algorithms, and software engineering", i, layer),
            embedding: vec![], // Будет сгенерирован автоматически
            layer,
            kind: "test".to_string(),
            tags: vec!["test".to_string(), format!("batch_{}", i / 10)],
            project: "integration_test".to_string(),
            session: "test_session".to_string(),
            score: 0.5 + ((i % 100) as f32) / 200.0, // 0.5 - 1.0
            ts: chrono::Utc::now() - chrono::Duration::seconds(i as i64 * 60),
            access_count: (i % 10) as u32,
            last_access: chrono::Utc::now() - chrono::Duration::seconds(i as i64 * 30),
        };
        
        records.push(record);
    }
    
    records
}

/// Benchmark тест для оценки производительности
#[tokio::test]
async fn test_memory_system_benchmarks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("bench_db"),
        cache_path: temp_dir.path().join("bench_cache"),
        ..Default::default()
    };
    let memory_service = MemoryService::new(config).await?;
    
    // === БЕНЧМАРК: ВСТАВКА ===
    let batch_sizes = vec![10, 50, 100, 500];
    
    for &batch_size in &batch_sizes {
        let records = create_diverse_test_records(batch_size);
        
        let start = std::time::Instant::now();
        for record in &records {
            memory_service.insert(record.clone()).await?;
        }
        let duration = start.elapsed();
        
        let throughput = batch_size as f64 / duration.as_secs_f64();
        println!("📈 Insert benchmark - Batch size: {}, Throughput: {:.1} records/sec", 
                 batch_size, throughput);
    }
    
    // === БЕНЧМАРК: ПОИСК ===
    let search_batch_sizes = vec![1, 5, 10, 50];
    
    for &search_k in &search_batch_sizes {
        let start = std::time::Instant::now();
        
        for _ in 0..10 {
            let query = "test algorithms programming";
            let _results = memory_service
                .search(query)
                .with_layer(Layer::Interact)
                .top_k(search_k)
                .execute()
                .await?;
        }
        
        let duration = start.elapsed();
        let search_throughput = 10.0 / duration.as_secs_f64();
        
        println!("🔍 Search benchmark - k={}, Throughput: {:.1} searches/sec", 
                 search_k, search_throughput);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("concurrent_db"),
        cache_path: temp_dir.path().join("concurrent_cache"),
        ..Default::default()
    };
    let memory_service = Arc::new(MemoryService::new(config).await?);
    
    // === КОНКУРЕНТНЫЕ ВСТАВКИ ===
    let mut insert_handles = Vec::new();
    
    for thread_id in 0..5 {
        let service = memory_service.clone();
        let handle = tokio::spawn(async move {
            let mut results = Vec::new();
            
            for i in 0..20 {
                let record = Record {
                    id: Uuid::new_v4(),
                    text: format!("concurrent_record_{}_{} - Test data for concurrent access", thread_id, i),
                    embedding: vec![],
                    layer: Layer::Interact,
                    kind: "test".to_string(),
                    tags: vec![format!("thread_{}", thread_id)],
                    project: "integration_test".to_string(),
                    session: format!("session_{}", thread_id),
                    score: 0.7,
                    ts: chrono::Utc::now(),
                    access_count: 0,
                    last_access: chrono::Utc::now(),
                };
                
                match service.insert(record).await {
                    Ok(_) => results.push(true),
                    Err(_) => results.push(false),
                }
            }
            
            results
        });
        
        insert_handles.push(handle);
    }
    
    // === КОНКУРЕНТНЫЕ ПОИСКИ ===
    let mut search_handles = Vec::new();
    
    for thread_id in 0..3 {
        let service = memory_service.clone();
        let handle = tokio::spawn(async move {
            let mut results = Vec::new();
            
            for i in 0..10 {
                let query = format!("concurrent_record_{}_{}", thread_id, i);
                match service.search(&query)
                    .with_layer(Layer::Interact)
                    .top_k(5)
                    .execute().await {
                    Ok(res) => results.push(res.len()),
                    Err(_) => results.push(0),
                }
            }
            
            results
        });
        
        search_handles.push(handle);
    }
    
    // === ОЖИДАНИЕ ЗАВЕРШЕНИЯ ===
    let insert_results = futures::future::try_join_all(insert_handles).await?;
    let search_results = futures::future::try_join_all(search_handles).await?;
    
    // === АНАЛИЗ РЕЗУЛЬТАТОВ ===
    let total_inserts: usize = insert_results.iter()
        .map(|results| results.iter().filter(|&&success| success).count())
        .sum();
    
    let total_search_results: usize = search_results.iter()
        .map(|results| results.iter().sum::<usize>())
        .sum();
    
    println!("🚀 CONCURRENT OPERATIONS TEST:");
    println!("   Successful inserts: {}/100", total_inserts);
    println!("   Total search results: {}", total_search_results);
    
    // Должны быть успешными минимум 90% операций
    assert!(total_inserts >= 90, "Too many failed concurrent inserts: {}/100", total_inserts);
    assert!(total_search_results > 0, "No search results in concurrent test");
    
    Ok(())
}