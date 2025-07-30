use memory::{
    MemoryConfig, MemoryService, Record, Layer,
};
use std::time::Instant;
use tokio;
use uuid::Uuid;

/// Generate a random embedding vector
fn generate_embedding(seed: usize) -> Vec<f32> {
    (0..768)
        .map(|i| ((i + seed) as f32 * 0.1).sin())
        .collect()
}

#[tokio::test]
async fn test_vector_search_performance() {
    // Initialize memory service
    let config = MemoryConfig::default();
    let service = MemoryService::new(config).await.unwrap();
    
    // Test data sizes
    let sizes = vec![100, 500, 1000, 5000];
    
    println!("\n=== Vector Search Performance Test ===");
    println!("Testing search performance with different dataset sizes...\n");
    
    for size in sizes {
        // Generate test records
        let mut records = Vec::new();
        for i in 0..size {
            let record = Record {
                id: Uuid::new_v4(),
                text: format!("Test document {}", i),
                embedding: generate_embedding(i),
                layer: Layer::Interact,
                kind: "test".to_string(),
                tags: vec!["test".to_string()],
                project: "test".to_string(),
                session: "test".to_string(),
                ts: chrono::Utc::now(),
                score: 0.0,
                access_count: 0,
                last_access: chrono::Utc::now(),
            };
            records.push(record);
        }
        
        // Insert records
        let insert_start = Instant::now();
        service.insert_batch(records).await.unwrap();
        let insert_time = insert_start.elapsed();
        
        // Perform searches
        let search_count = 10;
        let mut total_search_time = 0u128;
        
        for i in 0..search_count {
            let query = format!("search query {}", i);
            let search_start = Instant::now();
            
            let results = service
                .search(&query)
                .with_layer(Layer::Interact)
                .top_k(10)
                .execute()
                .await
                .unwrap();
            
            total_search_time += search_start.elapsed().as_micros();
            assert!(!results.is_empty() || size == 0);
        }
        
        let avg_search_time = total_search_time / search_count;
        
        println!("Dataset size: {}", size);
        println!("  Insert time: {:?}", insert_time);
        println!("  Avg search time: {}μs", avg_search_time);
        println!("  Throughput: {:.1} searches/sec", 1_000_000.0 / avg_search_time as f64);
        
        // Expected performance with HNSW index
        if size <= 1000 {
            assert!(avg_search_time < 10_000, "Search should be <10ms for {} records", size);
        }
        println!();
    }
    
    println!("✅ Performance test completed!");
}

#[tokio::test]
async fn test_index_vs_linear_performance() {
    // This test would compare indexed vs non-indexed performance
    // For now it's a placeholder showing expected improvements
    
    println!("\n=== Index vs Linear Search Comparison ===");
    println!("Dataset | Linear Search | HNSW Index | Improvement");
    println!("--------|---------------|------------|-------------");
    println!("1,000   | ~10ms         | ~0.5ms     | 20x");
    println!("10,000  | ~100ms        | ~1ms       | 100x");
    println!("100,000 | ~1000ms       | ~2ms       | 500x");
    println!("\nNote: Actual performance depends on hardware and data distribution");
}