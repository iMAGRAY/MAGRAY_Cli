// Проверка использования GPU в ONNX Runtime
use ai::config::EmbeddingConfig;
use ai::embeddings_cpu::CpuEmbeddingService;
use ai::gpu_detector::GpuDetector;

fn main() -> anyhow::Result<()> {
    // Инициализация логирования с максимальным уровнем
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🔍 Проверка использования GPU в ONNX Runtime\n");

    // Проверяем доступность GPU
    let gpu_detector = GpuDetector::detect();
    gpu_detector.print_detailed_info();

    if !gpu_detector.available {
        println!("❌ GPU не обнаружен!");
        return Ok(());
    }

    // Создаём конфигурацию с GPU
    let gpu_config = EmbeddingConfig {
        model_name: "qwen3emb".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: true,
        gpu_config: Some(ai::GpuConfig::auto_optimized()),
        embedding_dim: Some(1024),
    };

    println!("\n📊 Создание embedding сервиса с GPU...");
    match CpuEmbeddingService::new(gpu_config) {
        Ok(service) => {
            println!("✅ Сервис создан успешно!");

            // Делаем тестовый embedding
            let test_text = "Тестовый текст для проверки GPU".to_string();
            println!("\n🧪 Выполняем тестовый embedding...");

            match service.embed(&test_text) {
                Ok(result) => {
                    println!("✅ Embedding выполнен!");
                    println!("   Размерность: {}", result.embedding.len());
                    println!("   Первые 5 значений: {:?}", &result.embedding[..5]);
                }
                Err(e) => {
                    println!("❌ Ошибка embedding: {}", e);
                }
            }

            // Проверяем использование GPU через nvidia-smi
            println!("\n📊 Проверка nvidia-smi после embedding:");
            std::process::Command::new("nvidia-smi")
                .args(&[
                    "--query-gpu=name,memory.used,utilization.gpu",
                    "--format=csv,noheader",
                ])
                .status()?;
        }
        Err(e) => {
            println!("❌ Не удалось создать сервис: {}", e);
            println!("\n🔍 Детали ошибки:");
            println!("{:#?}", e);
        }
    }

    Ok(())
}
