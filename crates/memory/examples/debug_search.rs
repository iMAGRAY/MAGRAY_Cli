use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer};
use ai::AiConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== DEBUG SEARCH ISSUE ===\n");
    
    // Setup detailed logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    println!("🔍 Debug search with detailed logging");
    
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
    
    println!("\n1. Creating MemoryService...");
    let memory_service = MemoryService::new(config).await?;
    println!("✅ MemoryService created");
    
    println!("\n2. Inserting test record...");
    let test_id = uuid::Uuid::new_v4();
    let test_record = Record {
        id: test_id,
        text: "Test record for debugging search functionality".to_string(),
        embedding: vec![], // Empty - will be computed
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["debug".to_string()],
        project: "debug_test".to_string(),
        session: "debug_session".to_string(),
        ts: chrono::Utc::now(),
        last_access: chrono::Utc::now(),
        score: 0.0,
        access_count: 0,
    };
    
    println!("   Record ID: {}", test_id);
    memory_service.insert(test_record).await?;
    println!("✅ Record inserted");
    
    // Try immediate search
    println!("\n3. Immediate search for exact text...");
    let search_results = memory_service
        .search("Test record for debugging search functionality")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    println!("📊 SEARCH RESULTS:");
    println!("- Found: {} results", search_results.len());
    
    if search_results.is_empty() {
        println!("❌ No results found!");
        
        // Force index rebuild
        println!("\n4. Forcing index rebuild...");
        
        // Insert multiple records to trigger rebuild
        for i in 0..5 {
            let record = Record {
                id: uuid::Uuid::new_v4(),
                text: format!("Additional test record number {}", i),
                embedding: vec![],
                layer: Layer::Interact,
                kind: "test".to_string(),
                tags: vec!["batch".to_string()],
                project: "debug_test".to_string(),
                session: "debug_session".to_string(),
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
                score: 0.0,
                access_count: 0,
            };
            memory_service.insert(record).await?;
        }
        
        println!("✅ Added 5 more records");
        
        // Search again
        println!("\n5. Search after adding more records...");
        let search_results2 = memory_service
            .search("test record")
            .with_layer(Layer::Interact)
            .top_k(10)
            .execute()
            .await?;
        
        println!("📊 SECOND SEARCH RESULTS:");
        println!("- Found: {} results", search_results2.len());
        
        if !search_results2.is_empty() {
            println!("✅ SEARCH NOW WORKS!");
            for (i, result) in search_results2.iter().enumerate() {
                println!("   {}. Score: {:.4} | Text: '{}'", 
                         i + 1, result.score, 
                         if result.text.len() > 50 { 
                             format!("{}...", &result.text[..47])
                         } else { 
                             result.text.clone() 
                         });
            }
        } else {
            println!("❌ STILL NO RESULTS - deeper issue");
        }
    } else {
        println!("✅ SEARCH WORKS!");
        for (i, result) in search_results.iter().enumerate() {
            println!("   {}. Score: {:.4} | ID: {} | Text: '{}'", 
                     i + 1, result.score, result.id,
                     if result.text.len() > 50 { 
                         format!("{}...", &result.text[..47])
                     } else { 
                         result.text.clone() 
                     });
        }
    }
    
    Ok(())
}