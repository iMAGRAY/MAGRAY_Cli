use anyhow::Result;
use memory::onnx_models::Qwen3EmbeddingModel;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing simple ONNX model loading...");
    
    // Инициализируем логирование
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    let model_path = PathBuf::from("../../models/Qwen3-Embedding-0.6B-ONNX");
    
    println!("\nChecking model files:");
    
    // Проверяем наличие файлов
    let onnx_file = model_path.join("model_fp16.onnx");
    let onnx_data_file = model_path.join("model_fp16.onnx_data");
    
    if onnx_file.exists() {
        let metadata = std::fs::metadata(&onnx_file)?;
        println!("✓ model_fp16.onnx: {} MB", metadata.len() / (1024 * 1024));
    } else {
        println!("✗ model_fp16.onnx not found");
    }
    
    if onnx_data_file.exists() {
        let metadata = std::fs::metadata(&onnx_data_file)?;
        println!("✓ model_fp16.onnx_data: {} MB", metadata.len() / (1024 * 1024));
    } else {
        println!("✗ model_fp16.onnx_data not found");
    }
    
    println!("\nLoading model...");
    
    match Qwen3EmbeddingModel::new(model_path).await {
        Ok(model) => {
            println!("✓ Model loaded successfully!");
            println!("  Embedding dimension: {}", model.embedding_dim());
            
            // Тестируем простой текст
            let test_text = "Hello, world!";
            println!("\nTesting embedding for: '{}'", test_text);
            
            match model.embed(&[test_text.to_string()]).await {
                Ok(embeddings) => {
                    println!("✓ Embedding generated successfully!");
                    println!("  Dimensions: {}", embeddings[0].len());
                    println!("  First 5 values: {:?}", &embeddings[0][..5.min(embeddings[0].len())]);
                    
                    // Проверяем нормализацию (L2 норма должна быть ~1.0)
                    let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
                    println!("  L2 norm: {:.4}", norm);
                }
                Err(e) => {
                    println!("✗ Failed to generate embedding: {}", e);
                    println!("Error chain:");
                    let mut source = e.source();
                    while let Some(err) = source {
                        println!("  Caused by: {}", err);
                        source = err.source();
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load model: {}", e);
            println!("Error chain:");
            let mut source = e.source();
            while let Some(err) = source {
                println!("  Caused by: {}", err);
                source = err.source();
            }
        }
    }
    
    Ok(())
}