use memory::{
    MemoryService, MemoryError, Layer, MemoryConfig, CacheConfig, CacheConfigType,
    types::Record,
    cache::CacheStats,
    default_config
};
use std::sync::Arc;
use tokio::test;

// @component: {"k":"T","id":"integration_comprehensive","t":"Comprehensive integration tests","m":{"cur":100,"tgt":100,"u":"%"},"f":["test","integration","comprehensive"]}

const TEST_DIMENSIONS: usize = 768;

fn create_test_record(id: &str, content: &str, layer: Layer) -> Record {
    let embedding: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| (i as f32) * 0.001).collect();
    Record {
        id: uuid::Uuid::new_v4(),
        text: content.to_string(),
        embedding,
        layer,
        ts: chrono::Utc::now(),
        access_count: 1,
        last_access: chrono::Utc::now(),
        score: 0.0,
    }
}

#[test]
async fn test_full_memory_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = MemoryConfig {
        db_path: temp_dir.path().join("test_memory.db"),
        cache_path: temp_dir.path().join("test_cache"),
        ..default_config().unwrap()
    };
    
    let service = MemoryService::new(config).await.unwrap();
    
    // 1. Store initial records in different layers
    let interact_records = vec![
        create_test_record("interact_1", "User asked about weather", Layer::Interact),
        create_test_record("interact_2", "User asked about news", Layer::Interact),
    ];
    
    let insights_records = vec![
        create_test_record("insight_1", "User prefers morning weather updates", Layer::Insights),
    ];
    
    let assets_records = vec![
        create_test_record("asset_1", "Weather API documentation", Layer::Assets),
    ];
    
    // Store records
    let interact_refs: Vec<_> = interact_records.iter().collect();
    let insights_refs: Vec<_> = insights_records.iter().collect();
    let assets_refs: Vec<_> = assets_records.iter().collect();
    
    service.store_batch(&interact_refs).await.unwrap();
    service.store_batch(&insights_refs).await.unwrap();
    service.store_batch(&assets_refs).await.unwrap();
    
    // 2. Test search across layers
    let query_embedding: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| i as f32 * 0.001).collect();
    
    let interact_results = service.search(&query_embedding, Layer::Interact, 10).await.unwrap();
    assert_eq!(interact_results.len(), 2);
    
    let insights_results = service.search(&query_embedding, Layer::Insights, 10).await.unwrap();
    assert_eq!(insights_results.len(), 1);
    
    let assets_results = service.search(&query_embedding, Layer::Assets, 10).await.unwrap();
    assert_eq!(assets_results.len(), 1);
    
    // 3. Test batch operations
    let large_batch: Vec<_> = (0..1000)
        .map(|i| create_test_record(&format!("batch_{}", i), &format!("Batch content {}", i), Layer::Interact))
        .collect();
    let batch_refs: Vec<_> = large_batch.iter().collect();
    
    service.store_batch(&batch_refs).await.unwrap();
    
    let batch_search_results = service.search(&query_embedding, Layer::Interact, 50).await.unwrap();
    assert!(batch_search_results.len() >= 50);
    
    // 4. Test promotion workflow
    service.promote_memories().await.unwrap();
    
    // 5. Test cache integration
    let cache_stats = service.get_cache_stats().await.unwrap();
    assert!(cache_stats.total_entries > 0);
    
    println!("✅ Full memory lifecycle test completed successfully");
}

#[test]
async fn test_concurrent_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = MemoryConfig {
        db_path: temp_dir.path().join("concurrent_test.db"),
        cache_path: temp_dir.path().join("concurrent_cache"),
        ..default_config().unwrap()
    };
    
    let service = Arc::new(MemoryService::new(config).await.unwrap());
    
    // Spawn concurrent insert tasks
    let mut insert_handles = vec![];
    for i in 0..10 {
        let service_clone = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let records: Vec<_> = (0..100)
                .map(|j| create_test_record(&format!("concurrent_{}_{}", i, j), &format!("Content {} {}", i, j), Layer::Interact))
                .collect();
            let refs: Vec<_> = records.iter().collect();
            service_clone.store_batch(&refs).await.unwrap();
        });
        insert_handles.push(handle);
    }
    
    // Spawn concurrent search tasks
    let mut search_handles = vec![];
    for _i in 0..5 {
        let service_clone = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let query: Vec<f32> = (0..TEST_DIMENSIONS).map(|j| j as f32 * 0.001).collect();
            for _attempt in 0..20 {
                let _results = service_clone.search(&query, Layer::Interact, 10).await.unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
        search_handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in insert_handles {
        handle.await.unwrap();
    }
    
    for handle in search_handles {
        handle.await.unwrap();
    }
    
    // Verify final state
    let query: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| i as f32 * 0.001).collect();
    let final_results = service.search(&query, Layer::Interact, 100).await.unwrap();
    assert!(final_results.len() >= 100); // Should have at least 100 results from concurrent inserts
    
    println!("✅ Concurrent operations test completed successfully");
}

#[test]
async fn test_error_handling_and_recovery() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = MemoryConfig {
        db_path: temp_dir.path().join("error_test.db"),
        cache_path: temp_dir.path().join("error_cache"),
        ..default_config().unwrap()
    };
    
    let service = MemoryService::new(config).await.unwrap();
    
    // Test invalid embedding dimensions
    let invalid_record = Record {
        id: uuid::Uuid::new_v4(),
        text: "Invalid embedding size".to_string(),
        embedding: vec![1.0, 2.0], // Wrong size
        layer: Layer::Interact,
        ts: chrono::Utc::now(),
        access_count: 1,
        last_access: chrono::Utc::now(),
        score: 0.0,
    };
    
    let result = service.store_batch(&[&invalid_record]).await;
    assert!(result.is_err()); // Should fail due to dimension mismatch
    
    // Test empty search query
    let empty_query = vec![];
    let result = service.search(&empty_query, Layer::Interact, 10).await;
    assert!(result.is_err()); // Should fail due to empty query
    
    // Test search with wrong dimensions
    let wrong_dim_query: Vec<f32> = vec![1.0, 2.0]; // Wrong size
    let result = service.search(&wrong_dim_query, Layer::Interact, 10).await;
    assert!(result.is_err()); // Should fail due to dimension mismatch
    
    // Test that service can still handle valid operations after errors
    let valid_record = create_test_record("valid_after_error", "This should work", Layer::Interact);
    let result = service.store_batch(&[&valid_record]).await;
    assert!(result.is_ok()); // Should work fine
    
    let valid_query: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| i as f32 * 0.001).collect();
    let result = service.search(&valid_query, Layer::Interact, 10).await;
    assert!(result.is_ok()); // Should work fine
    
    println!("✅ Error handling and recovery test completed successfully");
}

#[test]
async fn test_memory_pressure_handling() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut config = default_config().unwrap();
    config.db_path = temp_dir.path().join("pressure_test.db");
    config.cache_path = temp_dir.path().join("pressure_cache");
    
    // Configure smaller limits for testing
    config.resource_config.max_vectors_per_layer = 5000;
    config.cache_config = CacheConfigType::Lru(CacheConfig {
        max_size_bytes: 1024 * 1024, // 1MB limit
        max_entries: 1000,
        ttl_seconds: Some(3600),
        eviction_batch_size: 100,
    });
    
    let service = MemoryService::new(config).await.unwrap();
    
    // Try to insert more records than the configured limit
    let large_batch: Vec<_> = (0..10000) // More than max_vectors_per_layer
        .map(|i| create_test_record(&format!("pressure_{}", i), &format!("Content under pressure {}", i), Layer::Interact))
        .collect();
    
    // Insert in smaller chunks to test gradual pressure
    for chunk in large_batch.chunks(1000) {
        let refs: Vec<_> = chunk.iter().collect();
        let result = service.store_batch(&refs).await;
        
        // Should either succeed or fail gracefully
        if result.is_err() {
            println!("⚠️  Memory pressure detected, operation failed gracefully");
            break;
        }
    }
    
    // Service should still be responsive for searches
    let query: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| i as f32 * 0.001).collect();
    let result = service.search(&query, Layer::Interact, 10).await;
    assert!(result.is_ok());
    
    // Test cache eviction behavior
    let cache_stats = service.get_cache_stats().await.unwrap();
    println!("Cache stats under pressure: entries={}, hit_rate={:.2}%", 
             cache_stats.total_entries, cache_stats.hit_rate * 100.0);
    
    println!("✅ Memory pressure handling test completed successfully");
}

#[test]
async fn test_performance_benchmarks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config = MemoryConfig {
        db_path: temp_dir.path().join("perf_test.db"),
        cache_path: temp_dir.path().join("perf_cache"),
        ..default_config().unwrap()
    };
    
    let service = MemoryService::new(config).await.unwrap();
    
    // Benchmark batch insertion
    let large_batch: Vec<_> = (0..1000)
        .map(|i| create_test_record(&format!("perf_{}", i), &format!("Performance test {}", i), Layer::Interact))
        .collect();
    let refs: Vec<_> = large_batch.iter().collect();
    
    let start = std::time::Instant::now();
    service.store_batch(&refs).await.unwrap();
    let insert_duration = start.elapsed();
    
    println!("📊 Batch insert (1000 records): {:.2}ms ({:.0} records/sec)", 
             insert_duration.as_millis(),
             1000.0 / insert_duration.as_secs_f64());
    
    // Benchmark search performance
    let query: Vec<f32> = (0..TEST_DIMENSIONS).map(|i| i as f32 * 0.001).collect();
    let mut total_search_time = std::time::Duration::ZERO;
    
    for _i in 0..100 {
        let start = std::time::Instant::now();
        let _results = service.search(&query, Layer::Interact, 10).await.unwrap();
        total_search_time += start.elapsed();
    }
    
    let avg_search_time = total_search_time / 100;
    println!("📊 Average search time (100 searches): {:.2}ms", avg_search_time.as_millis());
    
    // Performance assertions
    assert!(insert_duration.as_millis() < 5000, "Batch insert should complete in <5s");
    assert!(avg_search_time.as_millis() < 50, "Average search should complete in <50ms");
    
    println!("✅ Performance benchmarks test completed successfully");
}