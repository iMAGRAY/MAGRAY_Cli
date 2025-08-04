use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer, default_config};
use ai::gpu_detector::GpuDetector;
use std::time::Instant;
use tracing::info;
use uuid::Uuid;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логирования
    tracing_subscriber::fmt::init();

    info!("🚀 Тестирование GPU ускорения для системы памяти MAGRAY CLI");
    
    // 1. Проверка доступности GPU
    test_gpu_detection()?;
    
    // 2. Тест интеграции GPU с системой памяти
    test_memory_gpu_integration().await?;
    
    // 3. Тест батчевой обработки
    test_batch_processing().await?;
    
    // 4. Тест производительности CPU vs GPU
    test_performance_comparison().await?;
    
    // 5. Тест векторного поиска с GPU эмбеддингами
    test_vector_search().await?;
    
    info!("✅ Все тесты завершены успешно!");
    
    Ok(())
}

/// Тест определения GPU
fn test_gpu_detection() -> Result<()> {
    info!("\n📍 Тест 1: Определение GPU");
    
    let detector = GpuDetector::detect();
    
    if detector.available {
        info!("✅ GPU обнаружен!");
        info!("  - Количество устройств: {}", detector.devices.len());
        info!("  - CUDA версия: {}", detector.cuda_version);
        info!("  - Драйвер: {}", detector.driver_version);
        
        for (idx, device) in detector.devices.iter().enumerate() {
            info!("\n  GPU #{}: {}", idx, device.name);
            info!("    - Память: {} MB (свободно: {} MB)", 
                device.total_memory_mb, device.free_memory_mb);
            if let Some(temp) = device.temperature_c {
                info!("    - Температура: {}°C", temp);
            }
            if let Some(util) = device.utilization_percent {
                info!("    - Загрузка: {}%", util);
            }
            info!("    - Compute capability: {}", device.compute_capability);
        }
        
        // Информация о GPU доступности
        info!("\n  ✅ GPU готов для использования в ONNX Runtime");
    } else {
        info!("❌ GPU не обнаружен, будет использоваться CPU");
    }
    
    Ok(())
}

/// Тест интеграции GPU с системой памяти
async fn test_memory_gpu_integration() -> Result<()> {
    info!("\n📍 Тест 2: Интеграция GPU с системой памяти");
    
    // Создаем конфигурацию с GPU
    let mut config = default_config().unwrap();
    config.ai_config.embedding.use_gpu = true;
    
    // Инициализируем сервис
    let service = MemoryService::new(config).await?;
    
    // Тестируем одиночную вставку
    let record = Record {
        id: Uuid::new_v4(),
        text: "Testing GPU-accelerated embeddings in memory system".to_string(),
        embedding: vec![],
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["gpu".to_string()],
        project: "gpu_test".to_string(),
        session: Uuid::new_v4().to_string(),
        score: 0.5,
        access_count: 1,
        ts: Utc::now(),
        last_access: Utc::now(),
    };
    
    let start = Instant::now();
    service.insert(record).await?;
    info!("  Одиночная вставка с GPU: {:?}", start.elapsed());
    
    // Проверяем статистику кэша
    let (hits, misses, size) = service.cache_stats();
    info!("  Cache stats - Hits: {}, Misses: {}, Size: {} bytes", hits, misses, size);
    
    Ok(())
}

/// Тест батчевой обработки
async fn test_batch_processing() -> Result<()> {
    info!("\n📍 Тест 3: Батчевая обработка эмбеддингов");
    
    let mut config = default_config().unwrap();
    config.ai_config.embedding.use_gpu = true;
    let service = MemoryService::new(config).await?;
    
    let batch_sizes = vec![10, 50, 100, 200];
    
    for size in batch_sizes {
        let records: Vec<Record> = (0..size)
            .map(|i| Record {
                id: Uuid::new_v4(),
                text: format!("Batch test record #{}: Testing GPU batch processing with meaningful text content for better embeddings", i),
                embedding: vec![],
                layer: Layer::Interact,
                kind: "batch_test".to_string(),
                tags: vec!["batch".to_string(), "gpu".to_string()],
                project: "gpu_test".to_string(),
                session: Uuid::new_v4().to_string(),
                score: 0.5,
                access_count: 1,
                ts: Utc::now(),
                last_access: Utc::now(),
            })
            .collect();
        
        let start = Instant::now();
        service.insert_batch(records).await?;
        let elapsed = start.elapsed();
        
        info!("  Batch size {}: {:.2}ms ({:.1} records/sec)", 
            size, 
            elapsed.as_millis(),
            size as f64 / elapsed.as_secs_f64()
        );
    }
    
    Ok(())
}

/// Тест сравнения производительности  
async fn test_performance_comparison() -> Result<()> {
    info!("\n📍 Тест 4: Сравнение производительности CPU vs GPU");
    
    let test_data: Vec<String> = (0..100)
        .map(|i| format!("Test text #{}: This is a meaningful sentence for testing embedding performance with both CPU and GPU implementations", i))
        .collect();
    
    // CPU конфигурация
    let mut cpu_config = MemoryConfig::default();
    cpu_config.ai_config.embedding.use_gpu = false;
    cpu_config.db_path = cpu_config.db_path.parent().unwrap().join("cpu_test_db");
    cpu_config.cache_path = cpu_config.cache_path.parent().unwrap().join("cpu_test_cache");
    
    // GPU конфигурация
    let mut gpu_config = MemoryConfig::default();
    gpu_config.ai_config.embedding.use_gpu = true;
    gpu_config.db_path = gpu_config.db_path.parent().unwrap().join("gpu_test_db");
    gpu_config.cache_path = gpu_config.cache_path.parent().unwrap().join("gpu_test_cache");
    
    // Тест CPU
    info!("\n💻 Тестирование CPU:");
    let cpu_service = MemoryService::new(cpu_config).await?;
    
    let cpu_records: Vec<Record> = test_data.iter()
        .map(|text| Record {
            id: Uuid::new_v4(),
            text: text.clone(),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "perf_test".to_string(),
            tags: vec!["cpu".to_string()],
            project: "perf_test".to_string(),
            session: Uuid::new_v4().to_string(),
            score: 0.5,
            access_count: 1,
            ts: Utc::now(),
            last_access: Utc::now(),
        })
        .collect();
    
    let start = Instant::now();
    cpu_service.insert_batch(cpu_records).await?;
    let cpu_time = start.elapsed();
    info!("  CPU время: {:?} ({:.1} texts/sec)", cpu_time, test_data.len() as f64 / cpu_time.as_secs_f64());
    
    // Тест GPU
    info!("\n🎮 Тестирование GPU:");
    let gpu_service = MemoryService::new(gpu_config).await?;
    
    let gpu_records: Vec<Record> = test_data.iter()
        .map(|text| Record {
            id: Uuid::new_v4(),
            text: text.clone(),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "perf_test".to_string(),
            tags: vec!["gpu".to_string()],
            project: "perf_test".to_string(),
            session: Uuid::new_v4().to_string(),
            score: 0.5,
            access_count: 1,
            ts: Utc::now(),
            last_access: Utc::now(),
        })
        .collect();
    
    let start = Instant::now();
    gpu_service.insert_batch(gpu_records).await?;
    let gpu_time = start.elapsed();
    info!("  GPU время: {:?} ({:.1} texts/sec)", gpu_time, test_data.len() as f64 / gpu_time.as_secs_f64());
    
    // Сравнение
    if gpu_time < cpu_time {
        let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
        info!("\n📊 GPU ускорение: {:.2}x быстрее", speedup);
    } else {
        info!("\n📊 CPU оказался быстрее (возможно, GPU не доступен)");
    }
    
    Ok(())
}

/// Тест векторного поиска с GPU эмбеддингами
async fn test_vector_search() -> Result<()> {
    info!("\n📍 Тест 5: Векторный поиск с GPU эмбеддингами");
    
    let mut config = default_config().unwrap();
    config.ai_config.embedding.use_gpu = true;
    let service = MemoryService::new(config).await?;
    
    // Добавляем тестовые документы
    let documents = vec![
        "GPU acceleration enables faster machine learning model training",
        "CUDA cores are specialized processors designed for parallel computing",
        "TensorRT optimizes neural network inference on NVIDIA GPUs",
        "Vector databases use embeddings for semantic search capabilities",
        "HNSW algorithm provides efficient approximate nearest neighbor search",
        "Memory caching reduces latency in embedding generation pipelines",
        "Rust provides memory safety without garbage collection overhead",
        "The quick brown fox jumps over the lazy dog",
    ];
    
    info!("  Добавление {} документов...", documents.len());
    let records: Vec<Record> = documents.iter()
        .map(|text| Record {
            id: Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![],
            layer: Layer::Insights,
            kind: "document".to_string(),
            tags: vec!["search_test".to_string()],
            project: "gpu_search".to_string(),
            session: Uuid::new_v4().to_string(),
            score: 0.5,
            access_count: 1,
            ts: Utc::now(),
            last_access: Utc::now(),
        })
        .collect();
    
    service.insert_batch(records).await?;
    
    // Тестируем поиск
    let queries = vec![
        "GPU parallel computing",
        "vector search algorithm",
        "memory safety Rust",
    ];
    
    for query in queries {
        info!("\n  Поиск: '{}'", query);
        let start = Instant::now();
        
        let results = service.search(query)
            .with_layer(Layer::Insights)
            .top_k(3)
            .execute()
            .await?;
        
        let search_time = start.elapsed();
        info!("    Время поиска: {:?}", search_time);
        
        for (i, result) in results.iter().enumerate() {
            info!("    {}. Score: {:.3} - {}", 
                i + 1, 
                result.score,
                &result.text[..result.text.len().min(60)]
            );
        }
    }
    
    // Статистика
    info!("\n📊 Статистика системы:");
    let (hits, misses, size) = service.cache_stats();
    info!("  Cache - Hits: {}, Misses: {}, Size: {} KB", hits, misses, size / 1024);
    info!("  Cache hit rate: {:.1}%", service.cache_hit_rate() * 100.0);
    
    let health = service.get_system_health();
    info!("  System health: {:?}", health);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_memory_gpu_basic() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = default_config().unwrap();
        config.db_path = temp_dir.path().join("test_db");
        config.cache_path = temp_dir.path().join("test_cache");
        
        // Должен создаться с CPU fallback если нет GPU
        let service = MemoryService::new(config).await.unwrap();
        
        // Базовый тест вставки
        let record = Record {
            id: Uuid::new_v4(),
            text: "Test".to_string(),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec![],
            project: "test".to_string(),
            session: Uuid::new_v4().to_string(),
            score: 0.5,
            access_count: 1,
            ts: Utc::now(),
            last_access: Utc::now(),
        };
        
        assert!(service.insert(record).await.is_ok());
    }
}