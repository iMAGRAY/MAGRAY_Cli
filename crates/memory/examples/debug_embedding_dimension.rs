use anyhow::Result;
use ai::{OptimizedEmbeddingService, EmbeddingConfig};

/// Отладочный тест для определения реальной размерности эмбеддингов BGE-M3
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 Проверка реальной размерности эмбеддингов BGE-M3...\n");

    // Test with BGE-M3 config
    let config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: false,
    };

    match OptimizedEmbeddingService::new(config) {
        Ok(service) => {
            println!("✅ OptimizedEmbeddingService инициализирован");
            
            // Test embedding
            let test_text = "Тестовый текст для определения размерности эмбеддинга";
            
            match service.embed(test_text) {
                Ok(result) => {
                    let actual_dim = result.embedding.len();
                    println!("📏 Реальная размерность эмбеддинга: {}", actual_dim);
                    println!("🎯 Ожидаемая размерность (config.json): 1024");
                    
                    if actual_dim == 1024 {
                        println!("✅ РАЗМЕРНОСТЬ ПРАВИЛЬНАЯ: 1024");
                    } else if actual_dim == 768 {
                        println!("⚠️  РАЗМЕРНОСТЬ НЕОЖИДАННАЯ: 768 (возможно, обрезка после pooling)");
                    } else {
                        println!("❌ НЕИЗВЕСТНАЯ РАЗМЕРНОСТЬ: {}", actual_dim);
                    }
                    
                    // Показать первые 10 значений
                    println!("\n📊 Первые 10 значений эмбеддинга:");
                    for (i, val) in result.embedding.iter().take(10).enumerate() {
                        println!("  [{}]: {:.6}", i, val);
                    }
                    
                    // Проверить нормализацию
                    let norm: f32 = result.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                    println!("\n📏 L2 норма: {:.6}", norm);
                    if (norm - 1.0).abs() < 0.01 {
                        println!("✅ Эмбеддинг нормализован");
                    } else {
                        println!("⚠️  Эмбеддинг не нормализован");
                    }
                    
                    println!("\n🔧 Отладочная информация:");
                    println!("  Токенов: {}", result.token_count);
                    println!("  Время обработки: {} мс", result.processing_time_ms);
                    
                } Err(e) => {
                    println!("❌ Ошибка генерации эмбеддинга: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Ошибка инициализации службы: {}", e);
        }
    }

    Ok(())
}