use anyhow::Result;
use ai::{EmbeddingConfig, CpuEmbeddingService};

fn main() -> Result<()> {
    // Настройка логирования
    tracing_subscriber::fmt::init();
    
    println!("🔍 Тестирование инициализации Qwen3 модели...");
    
    // Проверяем что модель существует
    let model_path = std::path::Path::new("models/qwen3emb/model.onnx");
    let tokenizer_path = std::path::Path::new("models/qwen3emb/tokenizer.json");
    
    println!("📂 Проверка файлов:");
    println!("   Модель: {} (существует: {})", model_path.display(), model_path.exists());
    println!("   Токенизатор: {} (существует: {})", tokenizer_path.display(), tokenizer_path.exists());
    
    if !model_path.exists() {
        return Err(anyhow::anyhow!("Файл модели не найден: {}", model_path.display()));
    }
    
    if !tokenizer_path.exists() {
        return Err(anyhow::anyhow!("Файл токенизатора не найден: {}", tokenizer_path.display()));
    }
    
    // Настройка для тестирования с CPU только
    let config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: false, // Принудительно CPU для отладки
        gpu_config: None,
        embedding_dim: Some(1024),
    };
    
    println!("🚀 Создание CPU embedding сервиса...");
    
    match CpuEmbeddingService::new(config) {
        Ok(_service) => {
            println!("✅ Успешно создан CpuEmbeddingService для qwen3emb!");
        }
        Err(e) => {
            println!("❌ Ошибка создания сервиса: {:?}", e);
            println!("📋 Причина: {}", e);
            
            // Цепочка ошибок
            let mut current = e.source();
            let mut level = 1;
            while let Some(err) = current {
                println!("    {}: {}", level, err);
                current = err.source();
                level += 1;
            }
            
            return Err(e);
        }
    }
    
    println!("🎯 Тест завершён успешно!");
    Ok(())
}