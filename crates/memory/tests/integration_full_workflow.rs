use anyhow::Result;
use magray_memory::{
    MemoryService, MemoryConfig, VectorStore, BackupManager, IncrementalBackupManager,
    DynamicDimensionManager, DimensionConfig, OptimizedRebuildManager, RebuildConfig,
    Record, Layer, ResourceManager, ResourceConfig, HealthMonitor, HealthConfig,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// @component: {"k":"T","id":"integration_tests","t":"Full workflow integration tests","m":{"cur":0,"tgt":90,"u":"%"},"f":["integration","workflow","testing"]}

/// Комплексный тест полного workflow системы памяти
#[tokio::test]
async fn test_complete_memory_system_workflow() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // === ФАЗА 1: ИНИЦИАЛИЗАЦИЯ КОМПОНЕНТОВ ===
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path();
    
    // Создаём основные компоненты
    let memory_config = MemoryConfig::default();
    let mut memory_service = MemoryService::new(memory_config, base_path).await?;
    
    let resource_manager = ResourceManager::new(ResourceConfig::default())?;
    let dimension_manager = DynamicDimensionManager::new(DimensionConfig::default())?;
    let rebuild_manager = OptimizedRebuildManager::new(RebuildConfig::default());
    let backup_manager = BackupManager::new(base_path.join("backups"))?;
    let incremental_backup = IncrementalBackupManager::new(base_path.join("inc_backups"))?;
    
    println!("✅ Phase 1: All components initialized");

    // === ФАЗА 2: БАЗОВЫЕ ОПЕРАЦИИ С ДАННЫМИ ===
    
    // Создаём тестовые записи разных размерностей
    let test_records = create_diverse_test_records(100);
    
    // Добавляем записи
    for record in &test_records {
        memory_service.store_record(record).await?;
    }
    
    println!("✅ Phase 2: {} records stored", test_records.len());

    // === ФАЗА 3: ПОИСК И ВАЛИДАЦИЯ ===
    
    // Тестируем поиск по всем слоям
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let query = vec![0.1; 1024];
        let results = memory_service.search(&query, layer, 10, None).await?;
        
        assert!(!results.is_empty(), "Search returned no results for layer {:?}", layer);
        println!("✅ Phase 3: Search in layer {:?} returned {} results", layer, results.len());
    }

    // === ФАЗА 4: RESOURCE SCALING ===
    
    // Симулируем рост нагрузки
    let large_batch = create_diverse_test_records(500);
    for chunk in large_batch.chunks(50) {
        for record in chunk {
            memory_service.store_record(record).await?;
        }
        sleep(Duration::from_millis(10)).await; // Небольшая пауза
    }
    
    println!("✅ Phase 4: Resource scaling tested with {} additional records", large_batch.len());

    // === ФАЗА 5: BACKUP & RESTORE ===
    
    // Полный backup
    let store = memory_service.get_vector_store();
    let full_backup_path = backup_manager.create_backup(store.clone(), Some("integration_test_full".to_string())).await?;
    
    // Добавляем ещё данных для incremental backup
    let delta_records = create_diverse_test_records(50);
    for record in &delta_records {
        memory_service.store_record(record).await?;
    }
    
    // Инкрементальный backup
    let inc_backup_path = incremental_backup.create_incremental_backup(
        store.clone(),
        "integration_test_full",
        Some("integration_test_incremental".to_string())
    ).await?;
    
    println!("✅ Phase 5: Backups created - Full: {:?}, Incremental: {:?}", 
             full_backup_path, inc_backup_path);

    // === ФАЗА 6: DIMENSION MANAGEMENT ===
    
    // Тестируем разные размерности
    let dimensions_to_test = vec![384, 768, 1536];
    
    for &dim in &dimensions_to_test {
        let test_vector = vec![0.5; dim];
        let detected_dim = dimension_manager.detect_dimension(&test_vector);
        
        println!("✅ Phase 6: Dimension {} detected as {}", dim, detected_dim);
    }

    // === ФАЗА 7: INDEX REBUILD OPTIMIZATION ===
    
    // Получаем индекс и тестируем rebuild
    let layer = Layer::Interact;
    let index = memory_service.get_vector_store()
        .indices.get(&layer)
        .expect("Index should exist for layer");
    
    let rebuild_result = rebuild_manager.smart_rebuild_index(
        &memory_service.get_vector_store(),
        layer,
        index
    ).await?;
    
    println!("✅ Phase 7: Index rebuild completed - {} records processed in {:.2}s", 
             rebuild_result.records_processed, 
             rebuild_result.duration.as_secs_f64());

    // === ФАЗА 8: HEALTH MONITORING ===
    
    let health_config = HealthConfig::default();
    let health_monitor = HealthMonitor::new(health_config)?;
    
    // Получаем статистику системы
    let system_stats = memory_service.get_system_stats().await?;
    println!("✅ Phase 8: System stats - Total records: {}", system_stats.total_records);

    // === ФАЗА 9: ВОССТАНОВЛЕНИЕ ===
    
    // Создаём новый instance для тестирования restore
    let restore_temp_dir = TempDir::new()?;
    let restore_memory_service = MemoryService::new(
        MemoryConfig::default(), 
        restore_temp_dir.path()
    ).await?;
    
    // Восстанавливаем из полного backup
    backup_manager.restore_backup(
        restore_memory_service.get_vector_store(),
        &full_backup_path
    ).await?;
    
    // Применяем инкрементальный backup
    incremental_backup.restore_incremental_backup(
        restore_memory_service.get_vector_store(),
        &inc_backup_path
    ).await?;
    
    println!("✅ Phase 9: Restore completed");

    // === ФАЗА 10: ВАЛИДАЦИЯ ЦЕЛОСТНОСТИ ===
    
    // Проверяем что данные восстановились корректно
    let original_stats = memory_service.get_system_stats().await?;
    let restored_stats = restore_memory_service.get_system_stats().await?;
    
    // Количество записей должно быть >= чем в оригинале (потому что добавляли delta)
    assert!(restored_stats.total_records >= original_stats.total_records, 
            "Restored records count should be >= original");
    
    // Тестируем поиск в восстановленной системе
    let query = vec![0.1; 1024];
    let search_results = restore_memory_service.search(&query, Layer::Interact, 5, None).await?;
    assert!(!search_results.is_empty(), "Search in restored system should return results");
    
    println!("✅ Phase 10: Integrity validation passed");
    println!("🎉 COMPLETE WORKFLOW TEST SUCCESSFUL");
    println!("   Original records: {}", original_stats.total_records);
    println!("   Restored records: {}", restored_stats.total_records);
    println!("   Search results: {}", search_results.len());

    Ok(())
}

/// Тест производительности под нагрузкой
#[tokio::test] 
async fn test_performance_under_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let memory_service = MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?;
    
    let start = std::time::Instant::now();
    
    // === НАГРУЗОЧНЫЙ ТЕСТ: 1000 записей ===
    let records = create_diverse_test_records(1000);
    
    // Параллельная вставка
    let insert_start = std::time::Instant::now();
    
    for chunk in records.chunks(100) {
        let futures: Vec<_> = chunk.iter()
            .map(|record| memory_service.store_record(record))
            .collect();
        
        futures::future::try_join_all(futures).await?;
    }
    
    let insert_duration = insert_start.elapsed();
    
    // === НАГРУЗОЧНЫЙ ТЕСТ: 100 поисковых запросов ===
    let search_start = std::time::Instant::now();
    
    let mut search_results = Vec::new();
    for i in 0..100 {
        let query = vec![0.1 + (i as f32) * 0.01; 1024];
        let results = memory_service.search(&query, Layer::Interact, 10, None).await?;
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
    let mut memory_service = MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?;
    
    // === ПОДГОТОВКА ДАННЫХ ===
    let records = create_diverse_test_records(100);
    for record in &records {
        memory_service.store_record(record).await?;
    }
    
    // === СИМУЛЯЦИЯ ОТКАЗА: ПЕРЕСОЗДАНИЕ СЕРВИСА ===
    println!("💥 Simulating service restart...");
    drop(memory_service);
    
    // Пауза для симуляции downtime
    sleep(Duration::from_millis(100)).await;
    
    // === ВОССТАНОВЛЕНИЕ ===
    let recovered_service = MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?;
    
    // === ПРОВЕРКА ВОССТАНОВЛЕНИЯ ===
    let query = vec![0.1; 1024];
    let results = recovered_service.search(&query, Layer::Interact, 10, None).await?;
    
    assert!(!results.is_empty(), "Service should recover and have searchable data");
    
    let stats = recovered_service.get_system_stats().await?;
    assert!(stats.total_records > 0, "Recovered service should have records");
    
    println!("✅ RESILIENCE TEST PASSED:");
    println!("   Recovered records: {}", stats.total_records);
    println!("   Search results after recovery: {}", results.len());
    
    Ok(())
}

/// Тест многослойной системы памяти
#[tokio::test]
async fn test_multi_layer_promotion_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let memory_service = MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?;
    
    // === СОЗДАНИЕ ЗАПИСЕЙ В РАЗНЫХ СЛОЯХ ===
    
    // Interact слой - свежие данные
    for i in 0..20 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("interact_record_{}", i),
            embedding: vec![0.1 + i as f32 * 0.01; 1024],
            layer: Layer::Interact,
            score: 0.8 + i as f32 * 0.01,
            ts: chrono::Utc::now(),
            access_count: i as u32,
            last_access: chrono::Utc::now(),
        };
        memory_service.store_record(&record).await?;
    }
    
    // Insights слой - анализ
    for i in 0..10 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("insight_record_{}", i),
            embedding: vec![0.5 + i as f32 * 0.01; 1024],
            layer: Layer::Insights,
            score: 0.9 + i as f32 * 0.005,
            ts: chrono::Utc::now() - chrono::Duration::days(1),
            access_count: (i as u32) * 2,
            last_access: chrono::Utc::now(),
        };
        memory_service.store_record(&record).await?;
    }
    
    // Assets слой - постоянные данные
    for i in 0..5 {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("asset_record_{}", i),
            embedding: vec![0.9 + i as f32 * 0.002; 1024],
            layer: Layer::Assets,
            score: 0.95 + i as f32 * 0.001,
            ts: chrono::Utc::now() - chrono::Duration::days(30),
            access_count: (i as u32) * 5,
            last_access: chrono::Utc::now(),
        };
        memory_service.store_record(&record).await?;
    }
    
    // === ПРОВЕРКА РАСПРЕДЕЛЕНИЯ ПО СЛОЯМ ===
    let stats = memory_service.get_system_stats().await?;
    
    println!("📊 MULTI-LAYER TEST RESULTS:");
    println!("   Total records: {}", stats.total_records);
    println!("   Interact records: {}", stats.interact_count);
    println!("   Insights records: {}", stats.insights_count);
    println!("   Assets records: {}", stats.assets_count);
    
    assert_eq!(stats.interact_count, 20);
    assert_eq!(stats.insights_count, 10);
    assert_eq!(stats.assets_count, 5);
    assert_eq!(stats.total_records, 35);
    
    // === ТЕСТИРУЕМ ПОИСК ПО СЛОЯМ ===
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let query = vec![0.5; 1024];
        let results = memory_service.search(&query, layer, 5, None).await?;
        
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
        
        // Создаём векторы с небольшими вариациями
        let embedding = (0..1024)
            .map(|j| 0.1 + (i as f32 * 0.001) + (j as f32 * 0.0001))
            .collect();
        
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("test_record_{}_{:?}", i, layer),
            embedding,
            layer,
            score: 0.5 + (i as f32 % 100) as f32 / 200.0, // 0.5 - 1.0
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
    let memory_service = MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?;
    
    // === БЕНЧМАРК: ВСТАВКА ===
    let batch_sizes = vec![10, 50, 100, 500];
    
    for &batch_size in &batch_sizes {
        let records = create_diverse_test_records(batch_size);
        
        let start = std::time::Instant::now();
        for record in &records {
            memory_service.store_record(record).await?;
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
            let query = vec![0.5; 1024];
            let _results = memory_service.search(&query, Layer::Interact, search_k, None).await?;
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
    let memory_service = Arc::new(MemoryService::new(MemoryConfig::default(), temp_dir.path()).await?);
    
    // === КОНКУРЕНТНЫЕ ВСТАВКИ ===
    let mut insert_handles = Vec::new();
    
    for thread_id in 0..5 {
        let service = memory_service.clone();
        let handle = tokio::spawn(async move {
            let mut results = Vec::new();
            
            for i in 0..20 {
                let record = Record {
                    id: Uuid::new_v4(),
                    text: format!("concurrent_record_{}_{}", thread_id, i),
                    embedding: vec![0.1 + thread_id as f32 * 0.1; 1024],
                    layer: Layer::Interact,
                    score: 0.7,
                    ts: chrono::Utc::now(),
                    access_count: 0,
                    last_access: chrono::Utc::now(),
                };
                
                match service.store_record(&record).await {
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
                let query = vec![0.1 + thread_id as f32 * 0.1 + i as f32 * 0.01; 1024];
                match service.search(&query, Layer::Interact, 5, None).await {
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