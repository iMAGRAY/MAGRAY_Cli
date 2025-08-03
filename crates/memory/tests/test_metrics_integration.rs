use anyhow::Result;
use memory::{
    CacheConfig, EmbeddingCacheLRU, Layer, MemoryConfig, MemoryService, 
    MetricsCollector, PromotionConfig, Record, VectorStore
};
use tempfile::TempDir;

#[tokio::test]
async fn test_metrics_collection() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("vectors"),
        cache_path: temp_dir.path().join("cache"),
        promotion: PromotionConfig::default(),
        ai_config: ai::AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfigType::Lru(CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
        ..Default::default()
    };
    
    // Create service with metrics
    let mut service = MemoryService::new(config).await?;
    let metrics = service.enable_metrics();
    
    // Insert some test records
    for i in 0..10 {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: format!("Test document {}", i),
            embedding: vec![0.1 * i as f32; 768],
            layer: Layer::Interact,
            kind: "test".to_string(),
            project: "test".to_string(),
            tags: vec!["test".to_string()],
            session: "test-session".to_string(),
            score: 0.8,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        };
        service.insert(record).await?;
    }
    
    // Perform searches
    let results = service.search("Test document")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    assert_eq!(results.len(), 5);
    
    // Check metrics
    let snapshot = metrics.snapshot();
    assert!(snapshot.vector_inserts >= 10);
    assert!(snapshot.vector_searches >= 1);
    assert!(snapshot.cache_misses >= 1); // First embedding computation
    
    // Search again to test cache
    let _results2 = service.search("Test document")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    let snapshot2 = metrics.snapshot();
    assert!(snapshot2.cache_hits >= 1); // Should hit cache
    
    println!("Vector inserts: {}", snapshot2.vector_inserts);
    println!("Vector searches: {}", snapshot2.vector_searches);
    println!("Cache hit rate: {:.2}%", 
        (snapshot2.cache_hits as f64 / (snapshot2.cache_hits + snapshot2.cache_misses) as f64) * 100.0
    );
    
    Ok(())
}

#[tokio::test]
async fn test_promotion_metrics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("vectors"),
        cache_path: temp_dir.path().join("cache"),
        promotion: PromotionConfig {
            interact_ttl_hours: 1,
            insights_ttl_days: 1,
            promote_threshold: 0.5,
            decay_factor: 0.9,
        },
        ai_config: ai::AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfigType::Lru(memory::CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 10_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 100 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(80),
        ..Default::default()
    };
    
    let mut service = MemoryService::new(config).await?;
    let metrics = service.enable_metrics();
    
    // Insert test records with high scores
    for i in 0..5 {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: format!("Important document {}", i),
            embedding: vec![0.9; 768],
            layer: Layer::Interact,
            kind: "important".to_string(),
            project: "test".to_string(),
            tags: vec!["important".to_string()],
            session: "test-session".to_string(),
            score: 0.9,
            ts: chrono::Utc::now() - chrono::Duration::hours(2),
            last_access: chrono::Utc::now(),
            access_count: 5,
        };
        service.insert(record).await?;
    }
    
    // Run promotion cycle
    let stats = service.run_promotion_cycle().await?;
    
    // Check metrics
    let snapshot = metrics.snapshot();
    assert!(snapshot.promotions_interact_to_insights > 0);
    assert_eq!(snapshot.promotions_interact_to_insights, stats.interact_to_insights as u64);
    
    println!("Promoted {} records from Interact to Insights", stats.interact_to_insights);
    
    Ok(())
}

#[tokio::test]
async fn test_layer_metrics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Create vector store with metrics
    let store = VectorStore::new(temp_dir.path().join("vectors")).await?;
    store.init_layer(Layer::Interact).await?;
    store.init_layer(Layer::Insights).await?;
    store.init_layer(Layer::Assets).await?;
    
    let metrics = MetricsCollector::new();
    
    // Insert records into different layers
    for (layer, count) in [(Layer::Interact, 10), (Layer::Insights, 5), (Layer::Assets, 3)] {
        for i in 0..count {
            let record = Record {
                id: uuid::Uuid::new_v4(),
                text: format!("Record {} in {:?}", i, layer),
                embedding: vec![0.1 * i as f32; 768],
                layer,
                kind: "test".to_string(),
                project: "test".to_string(),
                tags: vec![],
                session: "test-session".to_string(),
                score: 0.7,
                ts: chrono::Utc::now() - chrono::Duration::days(i as i64),
                last_access: chrono::Utc::now(),
                access_count: i as u32,
            };
            store.insert(&record).await?;
        }
    }
    
    // Manually collect layer metrics
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        let iter = store.iter_layer(layer).await?;
        let mut count = 0u64;
        let mut access_sum = 0u32;
        
        for item in iter {
            if let Ok((_, value)) = item {
                count += 1;
                
                #[derive(serde::Deserialize)]
                struct StoredRecord {
                    record: Record,
                }
                if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                    access_sum += stored.record.access_count;
                }
            }
        }
        
        let layer_metrics = memory::LayerMetrics {
            record_count: count,
            total_size_bytes: count * 1000, // Approximate
            avg_embedding_size: 768.0,
            avg_access_count: if count > 0 { access_sum as f32 / count as f32 } else { 0.0 },
            oldest_record_age_hours: 24.0,
        };
        
        let layer_name = match layer {
            Layer::Interact => "interact",
            Layer::Insights => "insights",
            Layer::Assets => "assets",
        };
        metrics.update_layer_metrics(layer_name, layer_metrics);
    }
    
    // Check collected metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.layer_sizes.len(), 3);
    assert_eq!(snapshot.layer_sizes.get("Interact").unwrap().record_count, 10);
    assert_eq!(snapshot.layer_sizes.get("Insights").unwrap().record_count, 5);
    assert_eq!(snapshot.layer_sizes.get("Assets").unwrap().record_count, 3);
    
    // Test Prometheus export
    let prometheus = metrics.export_prometheus();
    assert!(prometheus.contains("memory_layer_record_count{layer=\"Interact\"} 10"));
    assert!(prometheus.contains("memory_layer_record_count{layer=\"Insights\"} 5"));
    assert!(prometheus.contains("memory_layer_record_count{layer=\"Assets\"} 3"));
    
    println!("Prometheus metrics:\n{}", prometheus);
    
    Ok(())
}

#[tokio::test]
async fn test_cache_metrics() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let cache_config = CacheConfig {
        max_size_bytes: 10_000,
        max_entries: 100,
        ttl_seconds: Some(3600),
        eviction_batch_size: 10,
    };
    
    let cache = EmbeddingCacheLRU::new(temp_dir.path().join("cache"), cache_config)?;
    let metrics = MetricsCollector::new();
    
    // Test cache operations
    for i in 0..20 {
        let text = format!("Test text {}", i);
        let embedding = vec![0.1 * i as f32; 768];
        
        // First access - miss
        if cache.get(&text, "model").is_none() {
            metrics.record_cache_miss();
            cache.insert(&text, "model", embedding)?;
        }
        
        // Second access - hit
        if cache.get(&text, "model").is_some() {
            metrics.record_cache_hit();
        }
    }
    
    let (hits, misses, total) = cache.stats();
    let size = cache.size().unwrap_or(0);
    let entries = total;
    metrics.update_cache_stats(entries as u64, size as u64);
    
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.cache_hits, 20);
    assert_eq!(snapshot.cache_misses, 20);
    assert!(snapshot.cache_entries > 0);
    
    let hit_rate = snapshot.cache_hits as f64 / (snapshot.cache_hits + snapshot.cache_misses) as f64;
    println!("Cache hit rate: {:.2}%", hit_rate * 100.0);
    println!("Cache entries: {}, size: {} bytes", snapshot.cache_entries, snapshot.cache_size_bytes);
    
    Ok(())
}

#[tokio::test]
async fn test_metrics_performance_tracking() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = VectorStore::new(temp_dir.path().join("vectors")).await?;
    store.init_layer(Layer::Interact).await?;
    
    let metrics = MetricsCollector::new();
    
    // Simulate various operation latencies
    use std::time::Duration;
    
    metrics.record_vector_search(Duration::from_millis(5));
    metrics.record_vector_search(Duration::from_millis(10));
    metrics.record_vector_search(Duration::from_millis(50));
    metrics.record_vector_search(Duration::from_millis(100));
    metrics.record_vector_search(Duration::from_millis(15));
    
    let snapshot = metrics.snapshot();
    let search_latency = &snapshot.vector_search_latency_ms;
    
    assert_eq!(search_latency.count, 5);
    assert!(search_latency.min_ms <= 5.0);
    assert!(search_latency.max_ms >= 100.0);
    assert!(search_latency.avg_ms > 0.0);
    
    println!("Search latency stats:");
    println!("  Count: {}", search_latency.count);
    println!("  Min: {:.2}ms", search_latency.min_ms);
    println!("  Max: {:.2}ms", search_latency.max_ms);
    println!("  Avg: {:.2}ms", search_latency.avg_ms);
    println!("  P50: {:.2}ms", search_latency.p50_ms);
    println!("  P90: {:.2}ms", search_latency.p90_ms);
    println!("  P99: {:.2}ms", search_latency.p99_ms);
    
    // Log summary
    metrics.log_summary();
    
    Ok(())
}