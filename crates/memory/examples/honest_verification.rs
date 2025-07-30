use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer};
use ai::AiConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ЧЕСТНАЯ ПРОВЕРКА РЕАЛЬНОСТИ СИСТЕМЫ ===\n");
    
    // Setup logging
    tracing_subscriber::fmt::init();
    
    println!("🔍 Проверяем что реально работает в системе");
    
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
    
    println!("\n1. Создаем MemoryService...");
    let memory_service = match MemoryService::new(config).await {
        Ok(service) => {
            println!("✅ MemoryService создан");
            service
        },
        Err(e) => {
            println!("❌ Не удалось создать MemoryService: {}", e);
            println!("   Это означает что OptimizedEmbeddingService не работает");
            return Ok(());
        }
    };
    
    println!("\n2. Вставляем тестовую запись...");
    let test_record = Record {
        id: uuid::Uuid::new_v4(),
        text: "Тестовая запись для проверки реальности эмбеддингов".to_string(),
        embedding: vec![], // Empty - will be computed
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["test".to_string()],
        project: "verification".to_string(),
        session: "honest_test".to_string(),
        ts: chrono::Utc::now(),
        last_access: chrono::Utc::now(),
        score: 0.0,
        access_count: 0,
    };
    
    memory_service.insert(test_record).await?;
    println!("✅ Запись вставлена");
    
    println!("\n3. Ищем похожую запись...");
    let search_results = memory_service
        .search("тестовая запись эмбеддинг")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    println!("📊 РЕЗУЛЬТАТЫ ПОИСКА:");
    println!("- Найдено результатов: {}", search_results.len());
    
    if search_results.is_empty() {
        println!("❌ ПОИСК НЕ РАБОТАЕТ - 0 результатов!");
        println!("   Возможные причины:");
        println!("   - Эмбеддинги генерируются но не индексируются");
        println!("   - Векторный поиск сломан");
        println!("   - Используются моки вместо реальных эмбеддингов");
    } else {
        for (i, result) in search_results.iter().enumerate() {
            println!("   {}. Score: {:.4} | Text: '{}'", 
                     i + 1, result.score, 
                     if result.text.len() > 50 { 
                         format!("{}...", &result.text[..47])
                     } else { 
                         result.text.clone() 
                     });
        }
        println!("✅ ПОИСК РАБОТАЕТ");
    }
    
    println!("\n4. Проверяем прямое использование OptimizedEmbeddingService...");
    
    // Попробуем напрямую
    match ai::OptimizedEmbeddingService::new(ai::EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    }) {
        Ok(service) => {
            println!("✅ OptimizedEmbeddingService создан напрямую");
            
            match service.embed("прямой тест эмбеддинга") {
                Ok(result) => {
                    println!("✅ Эмбеддинг сгенерирован напрямую:");
                    println!("   - Размерность: {}", result.embedding.len());
                    println!("   - Токенов: {}", result.token_count);
                    println!("   - Время: {}ms", result.processing_time_ms);
                    println!("   - Первые 5 значений: {:?}", &result.embedding[..5]);
                    
                    if result.embedding.len() == 1024 {
                        println!("✅ РАЗМЕРНОСТЬ ВЕРНА (BGE-M3: 1024)");
                    } else {
                        println!("❌ НЕВЕРНАЯ РАЗМЕРНОСТЬ (ожидалось 1024, получено {})", result.embedding.len());
                    }
                },
                Err(e) => {
                    println!("❌ Не удалось сгенерировать эмбеддинг: {}", e);
                }
            }
        },
        Err(e) => {
            println!("❌ Не удалось создать OptimizedEmbeddingService напрямую: {}", e);
            println!("   Проблема в отсутствии модели или токенизатора");
        }
    }
    
    println!("\n🏆 ЧЕСТНАЯ ОЦЕНКА СИСТЕМЫ:");
    
    // Проверяем файлы
    let model_exists = std::path::Path::new("crates/memory/models/bge-m3/model.onnx").exists();
    let tokenizer_exists = std::path::Path::new("crates/memory/models/bge-m3/tokenizer.json").exists();
    
    println!("📁 ФАЙЛЫ:");
    println!("- BGE-M3 model.onnx: {}", if model_exists { "✅ Есть" } else { "❌ Нет" });
    println!("- BGE-M3 tokenizer.json: {}", if tokenizer_exists { "✅ Есть" } else { "❌ Нет" });
    
    if model_exists && tokenizer_exists {
        println!("\n🎊 РЕАЛЬНЫЕ МОДЕЛИ ДОСТУПНЫ!");
        if search_results.is_empty() {
            println!("⚠️ НО ПОИСК НЕ РАБОТАЕТ - нужно разбираться с индексацией");
        } else {
            println!("🚀 ВСЕ РАБОТАЕТ РЕАЛЬНО!");
        }
    } else {
        println!("\n❌ МОДЕЛИ ОТСУТСТВУЮТ - система работает на моках");
    }
    
    Ok(())
}