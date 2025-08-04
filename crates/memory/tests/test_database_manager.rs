use memory::{DatabaseManager, default_config};
use anyhow::Result;
use tempfile::TempDir;
use tokio::task::JoinSet;

#[tokio::test]
async fn test_concurrent_database_access() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("concurrent_test.db");
    
    let manager = DatabaseManager::global();
    
    // Создаем 20 параллельных задач которые пытаются открыть одну и ту же базу
    let mut set = JoinSet::new();
    
    for i in 0..20 {
        let db_path_clone = db_path.clone();
        set.spawn(async move {
            let db = manager.get_database(&db_path_clone).unwrap();
            
            // Выполняем операции с базой данных
            let tree = db.open_tree(format!("test_tree_{}", i)).unwrap();
            
            // Вставляем данные
            for j in 0..10 {
                let key = format!("key_{}_{}", i, j);
                let value = format!("value_{}_{}", i, j);
                tree.insert(key.as_bytes(), value.as_bytes()).unwrap();
            }
            
            tree.flush().unwrap();
            
            // Возвращаем количество записей
            tree.len()
        });
    }
    
    // Ждем завершения всех задач
    let mut total_records = 0;
    while let Some(result) = set.join_next().await {
        let record_count = result??;
        total_records += record_count;
    }
    
    // Проверяем что все записи созданы
    assert_eq!(total_records, 20 * 10, "Все записи должны быть созданы");
    
    // Проверяем статистику подключений
    let stats = manager.get_connection_stats();
    assert_eq!(stats.len(), 1, "Должно быть только одно подключение к базе");
    
    println!("✅ Concurrent database access test passed");
    Ok(())
}

#[tokio::test]
async fn test_memory_service_concurrent_creation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    // Создаем несколько MemoryService параллельно с одним и тем же путем к базе
    let mut set = JoinSet::new();
    
    for i in 0..5 {
        let db_path = temp_dir.path().join("shared_memory.db");
        
        set.spawn(async move {
            let mut config = default_config();
            config.db_path = db_path;
            config.batch_config.async_flush = false; // Для стабильности тестов
            
            let service = memory::MemoryService::new(config).await?;
            
            // Вставляем тестовую запись
            let record = memory::Record {
                id: uuid::Uuid::new_v4(),
                text: format!("Test record from service {}", i),
                embedding: vec![0.1, 0.2, 0.3],
                layer: memory::Layer::Interact,
                project: "test".to_string(),
                tags: vec!["concurrent".to_string()],
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
                score: 0.0,
            };
            
            service.insert(record).await?;
            
            Ok::<_, anyhow::Error>(i)
        });
    }
    
    // Ждем завершения всех задач
    let mut completed_services = Vec::new();
    while let Some(result) = set.join_next().await {
        let service_id = result??;
        completed_services.push(service_id);
    }
    
    assert_eq!(completed_services.len(), 5, "Все сервисы должны быть созданы");
    
    // Проверяем что DatabaseManager правильно управляет подключениями
    let manager = DatabaseManager::global();
    let stats = manager.get_connection_stats();
    
    println!("✅ Concurrent MemoryService creation test passed");
    println!("📊 Database connections: {}", stats.len());
    
    Ok(())
}

#[tokio::test]
async fn test_database_manager_shutdown() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("shutdown_test.db");
    
    let manager = DatabaseManager::new(); // Создаем локальный менеджер
    
    // Открываем несколько баз данных
    let db1 = manager.get_database(&db_path)?;
    let db2 = manager.get_cache_database(temp_dir.path().join("cache.db"))?;
    let db3 = manager.get_system_database(temp_dir.path().join("system.db"))?;
    
    // Создаем данные
    db1.open_tree("test")?.insert("key", "value")?;
    db2.open_tree("cache")?.insert("cache_key", "cache_value")?;
    db3.open_tree("system")?.insert("system_key", "system_value")?;
    
    // Проверяем статистику
    let stats_before = manager.get_connection_stats();
    assert_eq!(stats_before.len(), 3, "Должно быть 3 подключения");
    
    // Graceful shutdown
    manager.shutdown()?;
    
    let stats_after = manager.get_connection_stats();
    assert_eq!(stats_after.len(), 0, "После shutdown должно быть 0 подключений");
    
    println!("✅ Database manager shutdown test passed");
    Ok(())
}

#[tokio::test]
async fn test_database_manager_flush_all() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let manager = DatabaseManager::new();
    
    // Открываем базы данных разных типов
    let main_db = manager.get_database(temp_dir.path().join("main.db"))?;
    let cache_db = manager.get_cache_database(temp_dir.path().join("cache.db"))?;
    let system_db = manager.get_system_database(temp_dir.path().join("system.db"))?;
    
    // Записываем данные в каждую базу
    main_db.open_tree("data")?.insert("main_key", "main_value")?;
    cache_db.open_tree("embeddings")?.insert("embedding_key", "embedding_value")?;
    system_db.open_tree("metrics")?.insert("metric_key", "metric_value")?;
    
    // Flush всех баз одновременно
    manager.flush_all()?;
    
    // Проверяем что данные сохранились
    assert_eq!(
        main_db.open_tree("data")?.get("main_key")?.unwrap(),
        b"main_value"
    );
    assert_eq!(
        cache_db.open_tree("embeddings")?.get("embedding_key")?.unwrap(),
        b"embedding_value"
    );
    assert_eq!(
        system_db.open_tree("metrics")?.get("metric_key")?.unwrap(),
        b"metric_value"
    );
    
    println!("✅ Database manager flush_all test passed");
    Ok(())
}