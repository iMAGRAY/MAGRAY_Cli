use anyhow::Result;
use memory::{MemoryService, Record, Layer, default_config};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== CLEAN VECTOR INDEX TEST ===\n");
    
    // Setup logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Use a temporary directory for clean test
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("test_lancedb");
    let cache_path = temp_dir.path().join("test_cache");
    
    println!("Using temporary paths:");
    println!("  DB: {:?}", db_path);
    println!("  Cache: {:?}\n", cache_path);
    
    // Create config
    let mut config = default_config().unwrap();
    config.db_path = db_path;
    config.cache_path = cache_path;
    config.ai_config.models_dir = PathBuf::from("crates/memory/models");
    config.ai_config.embedding.model_name = "bge-m3".to_string();
    config.ai_config.embedding.max_length = 512;
    config.ai_config.embedding.batch_size = 8;
    config.ai_config.embedding.use_gpu = false;
    config.ai_config.reranking.model_name = "mxbai".to_string();
    config.ai_config.reranking.max_length = 512;
    config.ai_config.reranking.batch_size = 8;
    config.ai_config.reranking.use_gpu = false;
    
    println!("1. Creating MemoryService with clean database...");
    let memory_service = MemoryService::new(config).await?;
    
    // Insert a few test documents
    let test_docs = vec![
        ("Machine learning is revolutionizing AI", vec!["ai", "ml"]),
        ("Deep learning uses neural networks", vec!["ai", "deep-learning"]),
        ("Natural language processing is amazing", vec!["ai", "nlp"]),
    ];
    
    println!("\n2. Inserting {} documents...", test_docs.len());
    
    let mut inserted_ids = Vec::new();
    
    for (_i, (text, tags)) in test_docs.iter().enumerate() {
        let id = uuid::Uuid::new_v4();
        let record = Record {
            id,
            text: text.to_string(),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            project: "clean_test".to_string(),
            session: "test".to_string(),
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            score: 0.0,
            access_count: 0,
        };
        
        memory_service.insert(record).await?;
        inserted_ids.push(id);
        println!("   ✅ Inserted: '{}' with ID: {}", text, id);
    }
    
    // Search
    println!("\n3. Searching for 'neural networks'...");
    
    let results = memory_service
        .search("neural networks")
        .with_layer(Layer::Interact)
        .top_k(3)
        .execute()
        .await?;
    
    println!("\n📊 RESULTS: Found {} documents", results.len());
    
    if results.is_empty() {
        println!("❌ NO RESULTS - Vector index issue!");
    } else {
        println!("✅ Vector index V2 is working!");
        for (i, result) in results.iter().enumerate() {
            println!("\n   {}. Score: {:.4}", i + 1, result.score);
            println!("      ID: {}", result.id);
            println!("      Text: '{}'", result.text);
            println!("      Tags: {:?}", result.tags);
        }
    }
    
    // Verify the returned IDs match what we inserted
    println!("\n4. Verifying IDs...");
    let result_ids: Vec<_> = results.iter().map(|r| r.id).collect();
    let matching_ids = result_ids.iter().filter(|id| inserted_ids.contains(id)).count();
    println!("   Matching IDs: {}/{}", matching_ids, results.len());
    
    Ok(())
}