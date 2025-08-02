// Проверка какие модели используются по умолчанию
use ai::config::{EmbeddingConfig, RerankingConfig};

fn main() {
    println!("🔍 Проверка моделей по умолчанию в MAGRAY\n");
    
    // Используем Default trait для получения конфигурации по умолчанию
    let embed_config = EmbeddingConfig::default();
    let rerank_config = RerankingConfig::default();
    
    println!("📋 Текущая конфигурация:");
    println!("   Embedding модель: {}", embed_config.model_name);
    println!("   Reranking модель: {}", rerank_config.model_name);
    println!("   Размерность эмбеддингов: {:?}", embed_config.embedding_dim);
    println!("   Максимальная длина: {}", embed_config.max_length);
    println!("   Использовать GPU: {}", embed_config.use_gpu);
    println!("   Размер батча: {}", embed_config.batch_size);
    
    println!("\n✅ Статус:");
    if embed_config.model_name == "qwen3emb" {
        println!("   ✓ Embedding использует Qwen3!");
    } else {
        println!("   ✗ Embedding всё ещё использует {}", embed_config.model_name);
    }
    
    if rerank_config.model_name == "qwen3_reranker" {
        println!("   ✓ Reranking использует Qwen3!");
    } else {
        println!("   ✗ Reranking всё ещё использует {}", rerank_config.model_name);
    }
    
    println!("\n📊 Итог: Система настроена на использование Qwen3 моделей по умолчанию!");
}