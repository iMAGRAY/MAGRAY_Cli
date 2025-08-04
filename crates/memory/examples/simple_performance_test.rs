use memory::{VectorIndexHnswRs, HnswRsConfig, VectorStore};
use std::time::Instant;
use tempfile::TempDir;
use uuid::Uuid;

// Простой тест производительности
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 MAGRAY Memory System Performance Test");
    println!("========================================");

    // === ТЕСТ 1: HNSW Index Performance ===
    println!("\n📊 Test 1: HNSW Index Performance");
    
    let config = HnswRsConfig {
        dimension: 768,
        max_connections: 16,
        ef_construction: 200,
        ef_search: 50,
        max_elements: 10000,
        max_layers: 16,
        use_parallel: true,
    };
    
    let index = VectorIndexHnswRs::new(config)?;
    
    // Вставка векторов
    let insert_start = Instant::now();
    for i in 0..1000 {
        let vector: Vec<f32> = (0..768).map(|j| (i as f32 + j as f32) * 0.001).collect();
        index.add(format!("vec_{}", i), vector)?;
    }
    let insert_duration = insert_start.elapsed();
    
    println!("  ✅ Inserted 1000 vectors in {:.2}ms", insert_duration.as_millis());
    println!("  📈 Insert rate: {:.1} vectors/sec", 1000.0 / insert_duration.as_secs_f64());
    
    // Поиск
    let search_start = Instant::now();
    let query: Vec<f32> = (0..768).map(|i| i as f32 * 0.001).collect();
    let results = index.search(&query, 10)?;
    let search_duration = search_start.elapsed();
    
    println!("  🔍 Search completed in {:.2}ms", search_duration.as_micros() as f64 / 1000.0);
    println!("  📈 Found {} results", results.len());
    
    // Статистика HNSW
    let stats = index.stats();
    println!("  📊 HNSW Stats:");
    println!("     - Total vectors: {}", stats.vector_count());
    println!("     - Avg insert time: {:.3}ms", stats.avg_insertion_time_ms());
    println!("     - Avg search time: {:.3}ms", stats.avg_search_time_ms());
    
    // === ТЕСТ 2: VectorStore Performance ===
    println!("\n📊 Test 2: VectorStore Performance");
    
    let temp_dir = TempDir::new()?;
    let store = VectorStore::new(temp_dir.path()).await?;
    
    // Создаём тестовые записи
    let mut test_records = Vec::new();
    for i in 0..500 {
        let embedding: Vec<f32> = (0..1024).map(|j| (i as f32 + j as f32) * 0.001).collect();
        
        let record = memory::Record {
            id: Uuid::new_v4(),
            text: format!("Test record {}", i),
            embedding,
            layer: memory::Layer::Interact,
            score: 0.8,
            ts: chrono::Utc::now(),
            access_count: 0,
            last_access: chrono::Utc::now(),
            kind: "test".to_string(),
            project: "benchmark".to_string(),
            session: Uuid::new_v4().to_string(),
            tags: vec!["test".to_string(), "benchmark".to_string()],
        };
        test_records.push(record);
    }
    
    // Batch insert
    let batch_start = Instant::now();
    let refs: Vec<&memory::Record> = test_records.iter().collect();
    store.insert_batch(&refs).await?;
    let batch_duration = batch_start.elapsed();
    
    println!("  ✅ Batch inserted 500 records in {:.2}ms", batch_duration.as_millis());
    println!("  📈 Batch rate: {:.1} records/sec", 500.0 / batch_duration.as_secs_f64());
    
    // Поиск в VectorStore
    let vs_search_start = Instant::now();
    let query: Vec<f32> = (0..1024).map(|i| i as f32 * 0.001).collect();
    let vs_results = store.search(&query, memory::Layer::Interact, 10).await?;
    let vs_search_duration = vs_search_start.elapsed();
    
    println!("  🔍 VectorStore search in {:.2}ms", vs_search_duration.as_micros() as f64 / 1000.0);
    println!("  📈 Found {} results", vs_results.len());
    
    // === ТЕСТ 3: Parallel Performance ===
    println!("\n📊 Test 3: Parallel Operations");
    
    let parallel_start = Instant::now();
    
    // Параллельный поиск
    let queries: Vec<Vec<f32>> = (0..10)
        .map(|i| (0..768).map(|j| (i as f32 + j as f32) * 0.001).collect())
        .collect();
    
    let parallel_results = index.parallel_search(&queries, 5)?;
    let parallel_duration = parallel_start.elapsed();
    
    println!("  ⚡ Parallel search (10 queries) in {:.2}ms", parallel_duration.as_millis());
    println!("  📈 Parallel rate: {:.1} queries/sec", 10.0 / parallel_duration.as_secs_f64());
    
    let total_results: usize = parallel_results.iter().map(|r| r.len()).sum();
    println!("  📊 Total results: {}", total_results);
    
    // === ИТОГОВАЯ СТАТИСТИКА ===
    println!("\n🎯 Performance Summary");
    println!("=====================");
    println!("• HNSW Insert: {:.1} vectors/sec", 1000.0 / insert_duration.as_secs_f64());
    println!("• HNSW Search: {:.3}ms avg", search_duration.as_micros() as f64 / 1000.0);
    println!("• VectorStore Batch: {:.1} records/sec", 500.0 / batch_duration.as_secs_f64());
    println!("• VectorStore Search: {:.3}ms", vs_search_duration.as_micros() as f64 / 1000.0);
    println!("• Parallel Search: {:.1} queries/sec", 10.0 / parallel_duration.as_secs_f64());
    
    println!("\n✅ Performance test completed successfully!");
    
    Ok(())
}