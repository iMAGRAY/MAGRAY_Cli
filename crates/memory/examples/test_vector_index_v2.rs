use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer};
use ai::{AiConfig, OptimizedEmbeddingService};
use std::path::PathBuf;
use std::time::Instant;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== TEST NEW VECTOR INDEX V2 ===\n");
    
    // Setup detailed logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    // Create config with BGE-M3 model
    let mut config = MemoryConfig::default();
    config.ai_config = AiConfig {
        models_dir: PathBuf::from("crates/memory/models"),
        embedding: ai::EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            batch_size: 8,
            use_gpu: false,
        },
        reranking: ai::RerankingConfig {
            model_name: "mxbai".to_string(),
            max_length: 512,
            batch_size: 8,
            use_gpu: false,
        },
    };
    
    println!("1. Creating MemoryService with new vector index...");
    let memory_service = MemoryService::new(config).await?;
    
    // Test documents
    let test_documents = vec![
        ("Machine learning is a subset of artificial intelligence", vec!["ai", "ml"]),
        ("Deep learning uses neural networks with multiple layers", vec!["ai", "deep-learning"]),
        ("Natural language processing helps computers understand human language", vec!["ai", "nlp"]),
        ("Computer vision enables machines to interpret visual information", vec!["ai", "cv"]),
        ("Reinforcement learning trains agents through rewards and penalties", vec!["ai", "rl"]),
        ("Rust is a systems programming language focused on safety", vec!["programming", "rust"]),
        ("Python is widely used for data science and machine learning", vec!["programming", "python"]),
        ("JavaScript powers web applications and Node.js backends", vec!["programming", "javascript"]),
        ("Cooking pasta requires boiling water and proper timing", vec!["cooking", "pasta"]),
        ("Baking bread involves yeast fermentation and precise temperatures", vec!["cooking", "baking"]),
    ];
    
    println!("\n2. Inserting {} test documents...", test_documents.len());
    let start = Instant::now();
    
    for (i, (text, tags)) in test_documents.iter().enumerate() {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![], // Will be computed
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            project: "vector_test".to_string(),
            session: "test_session".to_string(),
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            score: 0.0,
            access_count: 0,
        };
        
        memory_service.insert(record).await?;
        
        if (i + 1) % 5 == 0 {
            println!("   Inserted {} documents...", i + 1);
        }
    }
    
    let insert_time = start.elapsed();
    println!("✅ All documents inserted in {:.2}s", insert_time.as_secs_f64());
    
    // Test searches
    let test_queries = vec![
        "What is machine learning?",
        "How does deep learning work?",
        "Programming languages for web development",
        "Best practices for cooking Italian food",
        "Neural networks and artificial intelligence",
    ];
    
    println!("\n3. Testing searches...\n");
    
    for query in test_queries {
        println!("🔍 Query: \"{}\"", query);
        let start = Instant::now();
        
        let results = memory_service
            .search(query)
            .with_layer(Layer::Interact)
            .top_k(3)
            .execute()
            .await?;
        
        let search_time = start.elapsed();
        
        println!("   Found {} results in {:.3}s:", results.len(), search_time.as_secs_f64());
        
        for (i, result) in results.iter().enumerate() {
            println!("   {}. Score: {:.4} | Tags: {:?} | Text: '{}'", 
                     i + 1, 
                     result.score,
                     result.tags,
                     if result.text.len() > 60 { 
                         format!("{}...", &result.text[..57])
                     } else { 
                         result.text.clone() 
                     });
        }
        println!();
    }
    
    // Test batch insertion to trigger index rebuild
    println!("4. Testing batch insertion (should trigger HNSW rebuild)...");
    
    let mut batch_records = Vec::new();
    for i in 0..150 {
        batch_records.push(Record {
            id: uuid::Uuid::new_v4(),
            text: format!("Batch document number {} about various topics", i),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "batch".to_string(),
            tags: vec![format!("batch-{}", i / 10)],
            project: "vector_test".to_string(),
            session: "test_session".to_string(),
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            score: 0.0,
            access_count: 0,
        });
    }
    
    let start = Instant::now();
    for record in batch_records {
        memory_service.insert(record).await?;
    }
    let batch_time = start.elapsed();
    
    println!("✅ Batch insertion completed in {:.2}s", batch_time.as_secs_f64());
    
    // Final search to verify everything still works
    println!("\n5. Final search after batch insertion...");
    
    let start = Instant::now();
    let final_results = memory_service
        .search("machine learning and artificial intelligence")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    let final_time = start.elapsed();
    
    println!("📊 Final search completed in {:.3}s, found {} results", 
             final_time.as_secs_f64(), 
             final_results.len());
    
    for (i, result) in final_results.iter().enumerate() {
        println!("   {}. Score: {:.4} | Kind: {} | Text: '{}'", 
                 i + 1, 
                 result.score,
                 result.kind,
                 if result.text.len() > 60 { 
                     format!("{}...", &result.text[..57])
                 } else { 
                     result.text.clone() 
                 });
    }
    
    println!("\n✅ All tests completed successfully!");
    
    Ok(())
}