use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer};
use ai::{AiConfig, OptimizedEmbeddingService};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== TRACE SEARCH ISSUE ===\n");
    
    // Setup detailed logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    
    // First test OptimizedEmbeddingService directly
    println!("1. Testing direct embedding generation...");
    
    let embedding_config = ai::EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    let embedding_service = OptimizedEmbeddingService::new(embedding_config)?;
    
    let test_texts = vec![
        "This is a test document about machine learning",
        "This is a query about machine learning algorithms",
        "Completely different document about cooking recipes",
    ];
    
    println!("\nGenerating embeddings for {} texts...", test_texts.len());
    let mut embeddings = Vec::new();
    
    for text in &test_texts {
        let result = embedding_service.embed(text)?;
        println!("- Text: '{}' -> {} dims, {} tokens", 
                 if text.len() > 40 { &text[..40] } else { text },
                 result.embedding.len(),
                 result.token_count);
        embeddings.push(result.embedding);
    }
    
    // Calculate cosine similarities
    println!("\n2. Testing cosine similarity calculation...");
    
    for i in 0..embeddings.len() {
        for j in i+1..embeddings.len() {
            let similarity = cosine_similarity(&embeddings[i], &embeddings[j]);
            println!("   Similarity between text {} and {}: {:.4}", i, j, similarity);
            
            // Debug the vectors if similarity is 0
            if similarity.abs() < 0.0001 {
                println!("   ⚠️ ZERO SIMILARITY DETECTED!");
                println!("   First 5 values of embedding {}: {:?}", i, &embeddings[i][..5]);
                println!("   First 5 values of embedding {}: {:?}", j, &embeddings[j][..5]);
                
                // Check if vectors are identical
                let identical = embeddings[i].iter()
                    .zip(embeddings[j].iter())
                    .all(|(a, b)| (a - b).abs() < 0.0001);
                
                if identical {
                    println!("   ❌ EMBEDDINGS ARE IDENTICAL!");
                }
            }
        }
    }
    
    // Now test through MemoryService
    println!("\n3. Testing through MemoryService...");
    
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
    
    let memory_service = MemoryService::new(config).await?;
    
    // Insert test records
    println!("\n4. Inserting records...");
    
    for (i, text) in test_texts.iter().enumerate() {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![], // Let service compute it
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec![format!("test_{}", i)],
            project: "trace_test".to_string(),
            session: "trace_session".to_string(),
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            score: 0.0,
            access_count: 0,
        };
        
        memory_service.insert(record).await?;
        println!("   Inserted: '{}'", if text.len() > 40 { &text[..40] } else { text });
    }
    
    // Search
    println!("\n5. Searching...");
    
    let search_query = "machine learning algorithms";
    let results = memory_service
        .search(search_query)
        .with_layer(Layer::Interact)
        .top_k(10)
        .execute()
        .await?;
    
    println!("\n📊 SEARCH RESULTS for '{}': {} found", search_query, results.len());
    
    if results.is_empty() {
        println!("❌ NO RESULTS - Vector search completely broken");
        
        // Let's manually check what's in the pending list
        println!("\n6. Debugging vector index state...");
        
        // Unfortunately we can't access internals easily, but we can force a large insert
        // to trigger index rebuild
        println!("\n7. Forcing index rebuild with 100+ records...");
        
        for i in 0..100 {
            let record = Record {
                id: uuid::Uuid::new_v4(),
                text: format!("Force rebuild record number {}", i),
                embedding: vec![],
                layer: Layer::Interact,
                kind: "force".to_string(),
                tags: vec!["rebuild".to_string()],
                project: "trace_test".to_string(),
                session: "trace_session".to_string(),
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
                score: 0.0,
                access_count: 0,
            };
            memory_service.insert(record).await?;
        }
        
        println!("   Added 100 records to force rebuild");
        
        // Search again
        let results2 = memory_service
            .search(search_query)
            .with_layer(Layer::Interact)
            .top_k(10)
            .execute()
            .await?;
        
        println!("\n📊 SEARCH AFTER REBUILD: {} found", results2.len());
        
        if !results2.is_empty() {
            println!("✅ Search works after rebuild!");
            for (i, result) in results2.iter().take(5).enumerate() {
                println!("   {}. Score: {:.4} | Text: '{}'", 
                         i + 1, result.score,
                         if result.text.len() > 50 { &result.text[..50] } else { &result.text });
            }
        }
    } else {
        println!("✅ Search works!");
        for (i, result) in results.iter().enumerate() {
            println!("   {}. Score: {:.4} | Text: '{}'", 
                     i + 1, result.score,
                     if result.text.len() > 50 { &result.text[..50] } else { &result.text });
        }
    }
    
    Ok(())
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}