use anyhow::Result;
use std::path::PathBuf;
use memory::onnx_models::{Qwen3EmbeddingModel, Qwen3RerankerModel};

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализируем логирование
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("\n🧪 Testing Real ONNX Models\n");

    // Пути к моделям
    let embedding_path = PathBuf::from("../../models/Qwen3-Embedding-0.6B-ONNX");
    let reranker_path = PathBuf::from("../../models/Qwen3-Reranker-0.6B-ONNX");

    // Тестируем Embedding модель
    println!("📝 Testing Qwen3 Embedding Model");
    println!("   Path: {}", embedding_path.display());
    
    match Qwen3EmbeddingModel::new(embedding_path).await {
        Ok(model) => {
            println!("   ✅ Model loaded successfully!");
            println!("   - Embedding dimension: {}", model.embedding_dim());
            
            // Тестируем генерацию эмбеддингов
            let texts = vec![
                "Rust is a systems programming language".to_string(),
                "Memory safety without garbage collection".to_string(),
                "Concurrent programming made easy".to_string(),
            ];
            
            println!("\n   🔢 Generating embeddings for {} texts...", texts.len());
            match model.embed(&texts).await {
                Ok(embeddings) => {
                    println!("   ✅ Generated {} embeddings", embeddings.len());
                    for (i, (text, emb)) in texts.iter().zip(&embeddings).enumerate() {
                        println!("      [{}] Text: {:?}", i+1, text);
                        println!("          Embedding dims: {}", emb.len());
                        println!("          First 5 values: [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}]",
                                emb[0], emb[1], emb[2], emb[3], emb[4]);
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to generate embeddings: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Failed to load model: {}", e);
            if let Some(source) = e.source() {
                println!("      Caused by: {}", source);
            }
        }
    }

    // Тестируем Reranker модель
    println!("\n📊 Testing Qwen3 Reranker Model");
    println!("   Path: {}", reranker_path.display());
    
    match Qwen3RerankerModel::new(reranker_path).await {
        Ok(model) => {
            println!("   ✅ Model loaded successfully!");
            
            // Тестируем реранкинг
            let query = "How to ensure memory safety in Rust?";
            let documents = vec![
                "Rust guarantees memory safety through ownership and borrowing".to_string(),
                "Python is a high-level programming language".to_string(),
                "The borrow checker prevents data races at compile time".to_string(),
                "JavaScript runs in the browser".to_string(),
                "Lifetimes ensure references are always valid".to_string(),
            ];
            
            println!("\n   🎯 Testing reranking");
            println!("      Query: {:?}", query);
            println!("      Documents: {} items", documents.len());
            
            match model.rerank(query, &documents, 3).await {
                Ok(results) => {
                    println!("   ✅ Reranking completed!");
                    println!("      Top {} results:", results.len());
                    for (idx, score) in &results {
                        println!("         [{}] Score: {:.4} - {:?}", 
                                idx + 1, score, &documents[*idx][..50.min(documents[*idx].len())]);
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to rerank: {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ❌ Failed to load model: {}", e);
            if let Some(source) = e.source() {
                println!("      Caused by: {}", source);
            }
        }
    }

    println!("\n✨ Test completed!");
    Ok(())
}