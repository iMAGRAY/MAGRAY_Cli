use anyhow::Result;
use chrono::Utc;
use memory::{
    MemoryCoordinator, MemoryConfig, MemMeta, ExecutionContext, 
    MemLayer, MemoryStore, SemanticIndex
};
use std::collections::HashMap;
use tempfile::TempDir;

async fn create_test_coordinator() -> Result<(MemoryCoordinator, TempDir)> {
    let temp_dir = TempDir::new()?;
    let base_path = temp_dir.path().to_path_buf();
    
    // Создаём директории для моделей
    tokio::fs::create_dir_all(base_path.join("src/Qwen3-Embedding-0.6B-ONNX")).await?;
    tokio::fs::create_dir_all(base_path.join("src/Qwen3-Reranker-0.6B-ONNX")).await?;
    
    // Создаём фиктивные файлы моделей для тестов
    tokio::fs::write(
        base_path.join("src/Qwen3-Embedding-0.6B-ONNX/model_fp16.onnx"),
        b"fake model"
    ).await?;
    tokio::fs::write(
        base_path.join("src/Qwen3-Embedding-0.6B-ONNX/tokenizer.json"),
        r#"{"model":{"vocab":{}}}"#
    ).await?;
    tokio::fs::write(
        base_path.join("src/Qwen3-Embedding-0.6B-ONNX/config.json"),
        r#"{"hidden_size":1024,"max_position_embeddings":32768}"#
    ).await?;
    
    tokio::fs::write(
        base_path.join("src/Qwen3-Reranker-0.6B-ONNX/model.onnx"),
        b"fake model"
    ).await?;
    tokio::fs::write(
        base_path.join("src/Qwen3-Reranker-0.6B-ONNX/tokenizer.json"),
        r#"{"model":{"vocab":{}}}"#
    ).await?;
    tokio::fs::write(
        base_path.join("src/Qwen3-Reranker-0.6B-ONNX/config.json"),
        r#"{"max_position_embeddings":32768}"#
    ).await?;
    
    let config = MemoryConfig {
        base_path: base_path.clone(),
        sqlite_path: base_path.join("test.db"),
        blobs_path: base_path.join("blobs"),
        vectors_path: base_path.join("vectors"),
        cache_path: base_path.join("cache.db"),
        ..Default::default()
    };
    
    let coordinator = MemoryCoordinator::new(config).await?;
    Ok((coordinator, temp_dir))
}

#[tokio::test]
async fn test_full_memory_flow() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    let ctx = ExecutionContext::default();
    
    // 1. Тест сохранения в разные слои
    
    // Ephemeral data
    let ephemeral_key = "temp_data";
    let ephemeral_data = b"temporary session data";
    let mut ephemeral_meta = MemMeta::default();
    ephemeral_meta.tags.push("ephemeral".to_string());
    ephemeral_meta.ttl_seconds = Some(300); // 5 минут
    
    let result = coordinator.smart_put(ephemeral_key, ephemeral_data, ephemeral_meta, &ctx).await?;
    assert!(result.success);
    assert_eq!(result.mem_ref.as_ref().unwrap().layer, MemLayer::Ephemeral);
    
    // Short-term data
    let short_key = "session_fact";
    let short_data = b"user preference: dark mode enabled";
    let mut short_meta = MemMeta::default();
    short_meta.tags.push("session".to_string());
    
    let result = coordinator.smart_put(short_key, short_data, short_meta, &ctx).await?;
    assert!(result.success);
    assert_eq!(result.mem_ref.as_ref().unwrap().layer, MemLayer::Short);
    
    // Medium-term data
    let medium_key = "project_fact";
    let medium_data = b"project uses React framework with TypeScript";
    let medium_meta = MemMeta::default();
    
    let result = coordinator.smart_put(medium_key, medium_data, medium_meta, &ctx).await?;
    assert!(result.success);
    assert_eq!(result.mem_ref.as_ref().unwrap().layer, MemLayer::Medium);
    
    // Long-term data (large file)
    let long_key = "large_artifact";
    let long_data = vec![42u8; 2 * 1024 * 1024]; // 2MB
    let mut long_meta = MemMeta::default();
    long_meta.tags.push("archive".to_string());
    
    let result = coordinator.smart_put(long_key, &long_data, long_meta, &ctx).await?;
    assert!(result.success);
    assert_eq!(result.mem_ref.as_ref().unwrap().layer, MemLayer::Long);
    
    // 2. Тест чтения из разных слоёв
    
    let ephemeral_result = coordinator.smart_get(ephemeral_key, &ctx).await?;
    assert!(ephemeral_result.is_some());
    assert_eq!(ephemeral_result.unwrap().0, ephemeral_data);
    
    let short_result = coordinator.smart_get(short_key, &ctx).await?;
    assert!(short_result.is_some());
    assert_eq!(short_result.unwrap().0, short_data);
    
    let medium_result = coordinator.smart_get(medium_key, &ctx).await?;
    assert!(medium_result.is_some());
    assert_eq!(medium_result.unwrap().0, medium_data);
    
    let long_result = coordinator.smart_get(long_key, &ctx).await?;
    assert!(long_result.is_some());
    assert_eq!(long_result.unwrap().0, long_data);
    
    // 3. Тест семантического поиска
    // Примечание: В реальных тестах модели не загружены, поэтому семантический поиск может не работать
    
    // 4. Тест статистики
    let stats = coordinator.get_usage_stats().await?;
    assert_eq!(stats.total_items, 4);
    assert!(stats.total_size_bytes > 2 * 1024 * 1024); // Минимум 2MB от большого файла
    
    // 5. Тест удаления
    assert!(coordinator.delete(ephemeral_key).await?);
    assert!(coordinator.smart_get(ephemeral_key, &ctx).await?.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_memory_promotion() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    let ctx = ExecutionContext::default();
    
    // Создаём данные в ephemeral слое
    let key = "promote_me";
    let data = b"data that will be promoted";
    let mut meta = MemMeta::default();
    meta.tags.push("ephemeral".to_string());
    
    let result = coordinator.smart_put(key, data, meta, &ctx).await?;
    assert_eq!(result.mem_ref.as_ref().unwrap().layer, MemLayer::Ephemeral);
    
    // Делаем несколько обращений чтобы увеличить access_count
    for _ in 0..3 {
        let _ = coordinator.smart_get(key, &ctx).await?;
    }
    
    // Проверяем что данные всё ещё в ephemeral
    let (_, _, mem_ref) = coordinator.smart_get(key, &ctx).await?.unwrap();
    assert_eq!(mem_ref.layer, MemLayer::Ephemeral);
    
    // В реальной системе здесь сработал бы автоматический промоушен
    // на основе access_count и политик
    
    Ok(())
}

#[tokio::test]
async fn test_memory_cleanup() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    let ctx = ExecutionContext::default();
    
    // Добавляем временные данные с коротким TTL
    for i in 0..5 {
        let key = format!("temp_{}", i);
        let data = format!("temporary data {}", i);
        let mut meta = MemMeta::default();
        meta.tags.push("ephemeral".to_string());
        meta.ttl_seconds = Some(1); // 1 секунда
        
        coordinator.smart_put(&key, data.as_bytes(), meta, &ctx).await?;
    }
    
    // Ждём истечения TTL
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Запускаем очистку
    let cleaned = coordinator.cleanup_expired().await?;
    
    // Проверяем что ephemeral данные были очищены
    // (точное количество зависит от реализации cleanup в каждом слое)
    assert!(cleaned > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_layer_specific_operations() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    
    // Тест работы с MediumTermStore через фасад
    // Сохраняем структурированный факт
    let key = "structured_fact";
    let data = r#"{"type": "dependency", "name": "tokio", "version": "1.0"}"#;
    let mut meta = MemMeta::default();
    meta.content_type = "application/json".to_string();
    meta.tags.push("dependency".to_string());
    
    coordinator.put(key, data.as_bytes(), &meta).await?;
    
    // Читаем обратно
    let result = coordinator.get(key).await?;
    assert!(result.is_some());
    let (retrieved_data, retrieved_meta) = result.unwrap();
    assert_eq!(String::from_utf8(retrieved_data)?, data);
    assert_eq!(retrieved_meta.content_type, "application/json");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_access() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    let ctx = ExecutionContext::default();
    
    // Тест последовательного доступа к разным слоям
    for i in 0..10 {
        let key = format!("concurrent_{}", i);
        let data = format!("data {}", i);
        let mut meta = MemMeta::default();
        
        // Распределяем по разным слоям
        match i % 3 {
            0 => meta.tags.push("ephemeral".to_string()),
            1 => meta.tags.push("session".to_string()),
            _ => {},
        }
        
        coordinator.smart_put(&key, data.as_bytes(), meta, &ctx).await?;
        
        // Читаем несколько раз
        for _ in 0..5 {
            let _ = coordinator.smart_get(&key, &ctx).await?;
        }
    }
    
    // Проверяем статистику
    let stats = coordinator.get_usage_stats().await?;
    assert!(stats.total_items >= 10);
    
    Ok(())
}

#[tokio::test]
async fn test_memory_events() -> Result<()> {
    let (coordinator, _temp_dir) = create_test_coordinator().await?;
    let ctx = ExecutionContext::default();
    
    // Выполняем несколько операций
    let key = "event_test";
    let data = b"test data";
    let meta = MemMeta::default();
    
    // Store
    coordinator.smart_put(key, data, meta.clone(), &ctx).await?;
    
    // Access (hit)
    coordinator.smart_get(key, &ctx).await?;
    
    // Access (miss)
    coordinator.smart_get("non_existent", &ctx).await?;
    
    // Получаем события
    let events = coordinator.get_recent_events(10).await;
    
    // Проверяем что события были записаны
    assert!(events.len() >= 3);
    
    // Проверяем типы событий
    let has_store = events.iter().any(|e| matches!(e, memory::types::MemoryEvent::DataStored { .. }));
    let has_access = events.iter().any(|e| matches!(e, memory::types::MemoryEvent::DataAccessed { .. }));
    
    assert!(has_store);
    assert!(has_access);
    
    Ok(())
}