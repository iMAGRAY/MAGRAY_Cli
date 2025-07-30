use anyhow::Result;
use ai::{EmbeddingService, EmbeddingConfig, ModelLoader};

fn main() -> Result<()> {
    println!("=== FULL PIPELINE TEST: BGE-M3 + MXBai ===\n");
    
    // Setup logging
    tracing_subscriber::fmt::init();
    
    println!("🎯 Testing complete pipeline: embedding + reranking");
    
    // Create config for BGE-M3
    let config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    // Create model loader
    let model_loader = ModelLoader::new("crates/memory/models")?;
    
    // Create embedding service
    println!("\n1. Creating embedding service with BGE-M3 + MXBai...");
    let service = EmbeddingService::new(&model_loader, config)?;
    
    // Check capabilities
    println!("✅ Embedding service created");
    if service.is_using_real_model() {
        println!("✅ Using real BGE-M3 embeddings (1024 dims)");
    } else {
        println!("⚠️ Using fallback embeddings");
    }
    
    if service.has_reranker() {
        println!("✅ MXBai reranker is available");
    } else {
        println!("⚠️ Using mock reranker");
    }
    
    // Test embedding generation
    println!("\n2. Testing embedding generation...");
    let query = "machine learning algorithms for natural language processing";
    let documents = vec![
        "Deep learning models like transformers are revolutionizing NLP tasks".to_string(),
        "Computer vision techniques for image recognition and object detection".to_string(), 
        "Reinforcement learning agents in game playing and robotics applications".to_string(),
        "Natural language processing with BERT and GPT models for text understanding".to_string(),
        "Database management systems and SQL query optimization techniques".to_string(),
    ];
    
    println!("   Query: '{}'", query);
    println!("   Documents: {} items", documents.len());
    
    // Generate query embedding
    let query_embedding = service.embed(query)?;
    println!("   Query embedding: {} dims, norm: {:.6}", 
             query_embedding.embedding.len(), 
             query_embedding.embedding.iter().map(|x| x * x).sum::<f32>().sqrt());
    
    // Generate document embeddings
    let doc_embeddings = service.embed_batch(&documents)?;
    println!("   Document embeddings: {} generated", doc_embeddings.len());
    
    // Calculate cosine similarities for baseline
    println!("\n3. Testing semantic similarity (embeddings only)...");
    let mut similarities = Vec::new();
    for (i, doc_emb) in doc_embeddings.iter().enumerate() {
        let dot_product: f32 = query_embedding.embedding.iter()
            .zip(doc_emb.embedding.iter())
            .map(|(a, b)| a * b)
            .sum();
        similarities.push((i, dot_product));
        println!("   Doc {}: similarity = {:.4}", i + 1, dot_product);
    }
    
    // Sort by similarity
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("   Embedding ranking: {:?}", similarities.iter().map(|(i, _)| i + 1).collect::<Vec<_>>());
    
    // Test reranking
    println!("\n4. Testing MXBai reranking...");
    let rerank_results = service.rerank(query, &documents, Some(3))?;
    
    println!("   Top 3 reranked results:");
    for (rank, result) in rerank_results.iter().enumerate() {
        println!("   {}. [Doc {}] Score: {:.4}", 
                 rank + 1, result.index + 1, result.score);
        println!("      '{}'", result.document);
    }
    
    // Compare rankings
    let rerank_order: Vec<usize> = rerank_results.iter().map(|r| r.index + 1).collect();
    let embed_order: Vec<usize> = similarities.iter().take(3).map(|(i, _)| i + 1).collect();
    
    println!("\n5. Ranking comparison:");
    println!("   Embedding ranking (top 3): {:?}", embed_order);
    println!("   Reranking result (top 3):  {:?}", rerank_order);
    
    let ranking_diff = embed_order != rerank_order;
    if ranking_diff {
        println!("   ✅ Reranking changed the order (working as expected)");
    } else {
        println!("   ⚠️ Same ranking (may indicate limited reranking effect)");
    }
    
    println!("\n🏆 FULL PIPELINE TEST RESULTS:");
    println!("- BGE-M3 embeddings: {}", if service.is_using_real_model() { "✅ Real" } else { "⚠️ Mock" });
    println!("- MXBai reranking: {}", if service.has_reranker() { "✅ Real" } else { "⚠️ Mock" });
    println!("- Query embedding: ✅");
    println!("- Document embeddings: ✅");
    println!("- Similarity calculation: ✅");
    println!("- Reranking: ✅");
    println!("- Result comparison: ✅");
    
    if service.is_using_real_model() && service.has_reranker() {
        println!("\n🎊 FULL SUCCESS: Complete BGE-M3 + MXBai pipeline working!");
    } else if service.is_using_real_model() {
        println!("\n✅ PARTIAL SUCCESS: BGE-M3 working, MXBai using mock");
    } else {
        println!("\n⚠️ FALLBACK MODE: Using mock implementations");
    }
    
    Ok(())
}