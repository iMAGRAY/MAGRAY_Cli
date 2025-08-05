use anyhow::Result;
use memory::*;
use std::time::Instant;
use std::sync::Arc;
use std::path::PathBuf;
use uuid::Uuid;

fn create_test_record(idx: usize) -> Record {
    let embedding: Vec<f32> = (0..1024).map(|i| ((i + idx) as f32) * 0.001).collect();
    Record {
        id: Uuid::new_v4(),
        text: format!("Test record {}", idx),
        embedding,
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["perf_test".to_string()],
        project: "performance".to_string(),
        session: "test_session".to_string(),
        ts: chrono::Utc::now(),
        access_count: 1,
        last_access: chrono::Utc::now(),
        score: 0.5,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("memory=info")
        .init();

    println!("🚀 Memory System Performance Test");
    println!("=================================\n");

    // 1. Test VectorStore with HNSW
    println!("📊 Testing VectorStore with HNSW...");
    let db_path = PathBuf::from("./test_memory_perf_db");
    let store = Arc::new(VectorStore::new(&db_path).await?);
    
    // Batch insert test
    let batch_sizes = [10, 100, 1000];
    for &size in &batch_sizes {
        let records: Vec<Record> = (0..size).map(create_test_record).collect();
        
        let start = Instant::now();
        for record in &records {
            store.insert(record).await?;
        }
        let insert_time = start.elapsed();
        
        println!("  ✅ Inserted {} records in {:?} ({:.2} records/sec)", 
            size, insert_time, size as f64 / insert_time.as_secs_f64());
        
        // Search test
        let query = "test record 50";
        let start = Instant::now();
        // Need to get embeddings first for vector search
        let query_embedding = vec![0.5; 1024]; // Mock embedding
        let results = store.search(&query_embedding, Layer::Interact, 10).await?;
        let search_time = start.elapsed();
        
        println!("  🔍 Search completed in {:?} (found {} results)", 
            search_time, results.len());
    }
    
    // 2. Test LRU Cache
    println!("\n📊 Testing LRU Cache...");
    let cache_path = PathBuf::from("./test_memory_perf_cache");
    let cache_config = CacheConfig::default();
    let cache = EmbeddingCacheLRU::new(&cache_path, cache_config)?;
    
    let test_size = 1000;
    let start = Instant::now();
    for i in 0..test_size {
        let text = format!("cache test {}", i);
        let embedding: Vec<f32> = vec![i as f32; 1024];
        let _ = cache.insert(&text, "test_model", embedding);
    }
    let cache_insert_time = start.elapsed();
    
    println!("  ✅ Cached {} items in {:?} ({:.2} items/sec)", 
        test_size, cache_insert_time, test_size as f64 / cache_insert_time.as_secs_f64());
    
    // Cache hit test
    let mut hits = 0;
    let start = Instant::now();
    for i in 0..100 {
        let text = format!("cache test {}", i);
        if cache.get(&text, "test_model").is_some() {
            hits += 1;
        }
    }
    let cache_lookup_time = start.elapsed();
    
    println!("  🎯 Cache hit rate: {}% (lookup time: {:?})", hits, cache_lookup_time);
    
    // 3. Test ML Promotion Engine
    println!("\n📊 Testing ML Promotion Engine...");
    let ml_config = MLPromotionConfig::default();
    let mut engine = MLPromotionEngine::new(store.clone(), ml_config).await?;
    
    let start = Instant::now();
    let stats = engine.run_ml_promotion_cycle().await?;
    let promotion_time = start.elapsed();
    
    println!("  ✅ ML Promotion cycle completed in {:?}", promotion_time);
    println!("  📈 Analyzed: {} records", stats.total_analyzed);
    println!("  ⬆️  Promoted to Insights: {}", stats.promoted_interact_to_insights);
    println!("  ⬆️  Promoted to Assets: {}", stats.promoted_insights_to_assets);
    println!("  🎯 Model accuracy: {:.1}%", stats.model_accuracy * 100.0);
    
    // 4. Test Memory Service (full integration)
    println!("\n📊 Testing Memory Service Integration...");
    let config = default_config()?;
    let service = Arc::new(MemoryService::new(config).await?);
    
    // Concurrent operations test
    let concurrent_ops = 100;
    let start = Instant::now();
    
    let mut handles = vec![];
    for i in 0..concurrent_ops {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let record = create_test_record(i);
            service_clone.insert(record).await
        });
        handles.push(handle);
    }
    
    // Wait for all operations
    for handle in handles {
        handle.await??;
    }
    
    let concurrent_time = start.elapsed();
    println!("  ✅ {} concurrent operations in {:?} ({:.2} ops/sec)", 
        concurrent_ops, concurrent_time, concurrent_ops as f64 / concurrent_time.as_secs_f64());
    
    // Final search test
    let start = Instant::now();
    let results = service.search("test")
        .with_layer(Layer::Interact)
        .top_k(10)
        .execute()
        .await?;
    let final_search_time = start.elapsed();
    
    println!("  🔍 Final search: {} results in {:?}", results.len(), final_search_time);
    
    println!("\n✅ All performance tests completed successfully!");
    
    Ok(())
}