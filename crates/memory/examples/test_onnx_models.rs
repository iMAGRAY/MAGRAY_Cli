use anyhow::Result;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализируем логирование
    tracing_subscriber::fmt::init();

    println!("Testing ONNX models integration...\n");

    // Проверяем, какая реализация используется
    #[cfg(feature = "use_real_onnx")]
    {
        println!("Using REAL ONNX implementation");
        test_real_onnx().await?;
    }

    #[cfg(not(feature = "use_real_onnx"))]
    {
        println!("Using SIMPLIFIED (mock) implementation");
        test_simplified().await?;
    }

    Ok(())
}

#[cfg(feature = "use_real_onnx")]
async fn test_real_onnx() -> Result<()> {
    use memory::onnx_models::{Qwen3EmbeddingModel, Qwen3RerankerModel};
    use std::path::PathBuf;

    println!("Attempting to load real ONNX models...");

    // Пробуем разные пути к моделям
    let possible_paths = vec![
        PathBuf::from("models/Qwen3-Embedding-0.6B-ONNX"),
        PathBuf::from("../../models/Qwen3-Embedding-0.6B-ONNX"),
        PathBuf::from("C:/Users/1/Documents/GitHub/MAGRAY_Cli/models/Qwen3-Embedding-0.6B-ONNX"),
    ];

    let mut model_path = None;
    for path in &possible_paths {
        println!("Checking path: {}", path.display());
        if path.exists() {
            println!("  ✓ Path exists");
            
            // Проверяем наличие файлов модели
            let onnx_files = vec!["model.onnx", "model_fp16.onnx"];
            let mut found_model = false;
            for onnx_file in &onnx_files {
                if path.join(onnx_file).exists() {
                    println!("  ✓ Found {}", onnx_file);
                    found_model = true;
                }
            }
            
            if path.join("tokenizer.json").exists() {
                println!("  ✓ Found tokenizer.json");
            } else {
                println!("  ✗ tokenizer.json NOT found");
            }
            
            if path.join("config.json").exists() {
                println!("  ✓ Found config.json");
            } else {
                println!("  ✗ config.json NOT found");
            }
            
            if found_model {
                model_path = Some(path.clone());
                break;
            }
        } else {
            println!("  ✗ Path does not exist");
        }
    }

    if let Some(path) = model_path {
        println!("\nUsing model path: {}", path.display());
        
        // Пытаемся загрузить embedding модель
        println!("\nLoading embedding model...");
        match Qwen3EmbeddingModel::new(path.clone()).await {
            Ok(model) => {
                println!("✓ Embedding model loaded successfully!");
                println!("  Embedding dimension: {}", model.embedding_dim());
                
                // Тестируем эмбеддинги
                let texts = vec![
                    "Hello world".to_string(),
                    "This is a test".to_string(),
                ];
                
                println!("\nGenerating embeddings for {} texts...", texts.len());
                match model.embed(&texts).await {
                    Ok(embeddings) => {
                        println!("✓ Embeddings generated successfully!");
                        for (i, (text, emb)) in texts.iter().zip(embeddings.iter()).enumerate() {
                            println!("  Text {}: \"{}\" -> embedding with {} dimensions", 
                                i, text, emb.len());
                            // Показываем первые 5 значений
                            let preview: Vec<String> = emb.iter()
                                .take(5)
                                .map(|v| format!("{:.4}", v))
                                .collect();
                            println!("    First 5 values: [{}]", preview.join(", "));
                        }
                    }
                    Err(e) => {
                        println!("✗ Failed to generate embeddings: {}", e);
                        println!("  Error details: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to load embedding model: {}", e);
                println!("  Error details: {:?}", e);
            }
        }
        
        // Пытаемся загрузить reranker модель
        let reranker_paths = vec![
            PathBuf::from("models/Qwen3-Reranker-0.6B-ONNX"),
            PathBuf::from("../../models/Qwen3-Reranker-0.6B-ONNX"),
            PathBuf::from("C:/Users/1/Documents/GitHub/MAGRAY_Cli/models/Qwen3-Reranker-0.6B-ONNX"),
        ];
        
        let mut reranker_path = None;
        for path in &reranker_paths {
            if path.exists() && path.join("model.onnx").exists() {
                reranker_path = Some(path.clone());
                break;
            }
        }
        
        if let Some(path) = reranker_path {
            println!("\nLoading reranker model from: {}", path.display());
            match Qwen3RerankerModel::new(path).await {
                Ok(model) => {
                    println!("✓ Reranker model loaded successfully!");
                    
                    // Тестируем reranking
                    let query = "vector database";
                    let documents = vec![
                        "This is about vector database systems".to_string(),
                        "Random text about cats".to_string(),
                        "Database vectors and embeddings".to_string(),
                    ];
                    
                    println!("\nTesting reranking...");
                    println!("Query: \"{}\"", query);
                    match model.rerank(query, &documents, 3).await {
                        Ok(results) => {
                            println!("✓ Reranking completed successfully!");
                            for (idx, score) in results {
                                println!("  Doc {}: \"{}\" -> score: {:.4}", 
                                    idx, documents[idx], score);
                            }
                        }
                        Err(e) => {
                            println!("✗ Failed to rerank: {}", e);
                            println!("  Error details: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to load reranker model: {}", e);
                    println!("  Error details: {:?}", e);
                }
            }
        } else {
            println!("\n⚠ Reranker model not found");
        }
    } else {
        println!("\n✗ No valid model path found!");
        println!("Please ensure models are placed in one of the expected locations.");
    }

    Ok(())
}

#[cfg(not(feature = "use_real_onnx"))]
async fn test_simplified() -> Result<()> {
    use memory::onnx_models::{Qwen3EmbeddingModel, Qwen3RerankerModel};
    use std::path::PathBuf;

    println!("Testing simplified (mock) implementation...");

    let model_path = PathBuf::from("fake/path");
    
    // Тестируем embedding модель
    let embedding_model = Qwen3EmbeddingModel::new(model_path.clone()).await?;
    println!("✓ Mock embedding model created");
    println!("  Embedding dimension: {}", embedding_model.embedding_dim());
    
    let texts = vec![
        "Hello world".to_string(),
        "This is a test".to_string(),
    ];
    
    let embeddings = embedding_model.embed(&texts).await?;
    println!("✓ Mock embeddings generated");
    for (i, (text, emb)) in texts.iter().zip(embeddings.iter()).enumerate() {
        println!("  Text {}: \"{}\" -> {} dimensions", i, text, emb.len());
    }
    
    // Тестируем reranker модель
    let reranker_model = Qwen3RerankerModel::new(model_path).await?;
    println!("\n✓ Mock reranker model created");
    
    let query = "vector database";
    let documents = vec![
        "This is about vector database systems".to_string(),
        "Random text about cats".to_string(),
        "Database vectors and embeddings".to_string(),
    ];
    
    let results = reranker_model.rerank(query, &documents, 3).await?;
    println!("✓ Mock reranking completed");
    for (idx, score) in results {
        println!("  Doc {}: score {:.4}", idx, score);
    }

    Ok(())
}