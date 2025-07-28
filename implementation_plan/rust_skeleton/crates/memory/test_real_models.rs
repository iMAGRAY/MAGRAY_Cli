// Простой тест для проверки загрузки реальных ONNX моделей
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализируем tracing для отладки
    tracing_subscriber::fmt::init();
    
    let base_path = PathBuf::from("crates/memory/src");
    
    println!("Попытка загрузки Qwen3 Embedding модели...");
    let embedding_path = base_path.join("Qwen3-Embedding-0.6B-ONNX");
    
    match memory::onnx_models::Qwen3EmbeddingModel::new(embedding_path).await {
        Ok(model) => {
            println!("✅ Embedding модель успешно загружена!");
            println!("   Размерность: {}", model.embedding_dim());
            
            // Попробуем простой эмбеддинг
            println!("Генерация тестового эмбеддинга...");
            let test_texts = vec!["Hello world".to_string()];
            match model.embed(&test_texts).await {
                Ok(embeddings) => {
                    println!("✅ Эмбеддинг сгенерирован! Размер: {}x{}", 
                           embeddings.len(), embeddings[0].len());
                    
                    // Проверим что вектор нормализован
                    let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
                    println!("   Норма вектора: {:.4}", norm);
                    
                    if (norm - 1.0).abs() < 0.1 {
                        println!("✅ Вектор корректно нормализован");
                    } else {
                        println!("⚠️  Вектор может быть не нормализован");
                    }
                },
                Err(e) => {
                    println!("❌ Ошибка генерации эмбеддинга: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ Не удалось загрузить embedding модель: {}", e);
        }
    }
    
    println!("\nПопытка загрузки Qwen3 Reranker модели...");
    let reranker_path = base_path.join("Qwen3-Reranker-0.6B-ONNX");
    
    match memory::onnx_models::Qwen3RerankerModel::new(reranker_path).await {
        Ok(model) => {
            println!("✅ Reranker модель успешно загружена!");
            
            // Попробуем простой reranking
            println!("Тестирование reranking...");
            let query = "machine learning";
            let docs = vec![
                "Machine learning is a subset of AI".to_string(),
                "The weather is nice today".to_string(),
                "Deep learning uses neural networks".to_string(),
            ];
            
            match model.rerank(query, &docs, 2).await {
                Ok(results) => {
                    println!("✅ Reranking выполнен! Результатов: {}", results.len());
                    for (i, (idx, score)) in results.iter().enumerate() {
                        println!("   {}. Документ {}: score={:.4}", i+1, idx, score);
                    }
                },
                Err(e) => {
                    println!("❌ Ошибка reranking: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ Не удалось загрузить reranker модель: {}", e);
        }
    }
    
    println!("\n🎯 Тест завершён");
    Ok(())
}