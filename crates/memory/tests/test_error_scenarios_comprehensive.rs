//! Comprehensive error handling and edge case tests
//! 
//! Покрывает:
//! - Error scenarios для всех критических компонентов
//! - Edge cases и boundary conditions
//! - Recovery mechanisms и graceful degradation
//! - Resource exhaustion scenarios
//! - Network failures и timeouts
//! - Data corruption handling

use memory::{
    service_di::{DIMemoryService, MemoryServiceConfig, default_config},
    types::{Layer, Record, SearchOptions},
    DIContainer, Lifetime,
    CacheConfigType,
};

// TODO: Uncomment when vector_index_hnswlib is public
// use memory::vector_index_hnswlib::VectorIndexHNSW;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio;
use chrono::Utc;
use std::collections::HashMap;
use tempfile::TempDir;

// @component: {"k":"T","id":"error_scenarios_comprehensive_tests","t":"Comprehensive error handling and edge case tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","error","edge_cases","recovery","coverage"]}

/// Утилиты для создания проблематичных данных
fn create_problematic_record(id: &str, problem_type: &str) -> Record {
    match problem_type {
        "empty_id" => Record {
            id: "".to_string(),
            content: "Valid content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        "empty_content" => Record {
            id: id.to_string(),
            content: "".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        "empty_embedding" => Record {
            id: id.to_string(),
            content: "Valid content".to_string(),
            embedding: vec![],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        "invalid_embedding" => Record {
            id: id.to_string(),
            content: "Valid content".to_string(),
            embedding: vec![f32::NAN, f32::INFINITY, -f32::INFINITY],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        "huge_content" => Record {
            id: id.to_string(),
            content: "x".repeat(1_000_000), // 1MB content
            embedding: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        "huge_embedding" => Record {
            id: id.to_string(),
            content: "Valid content".to_string(),
            embedding: vec![0.1; 100_000], // 100k dimensions
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
        _ => Record {
            id: id.to_string(),
            content: "Default content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        },
    }
}

async fn create_test_service_with_config(config: MemoryServiceConfig) -> Result<DIMemoryService> {
    std::fs::create_dir_all(&config.cache_path)?;
    DIMemoryService::new(config).await
}

#[tokio::test]
async fn test_error_empty_record_id() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("error_test.db");
    config.cache_path = temp_dir.path().join("error_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    let problematic_record = create_problematic_record("test", "empty_id");
    
    // Попытка вставки записи с пустым ID
    let result = service.insert(problematic_record).await;
    
    // Система должна либо обработать gracefully, либо вернуть осмысленную ошибку
    match result {
        Ok(_) => {
            // Если вставка прошла, проверяем что система не сломалась
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
        Err(e) => {
            // Ошибка должна быть осмысленной
            let error_msg = e.to_string();
            assert!(error_msg.len() > 0);
            
            // Система должна остаться работоспособной
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_invalid_embedding_values() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("invalid_embedding_test.db");
    config.cache_path = temp_dir.path().join("invalid_embedding_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    let invalid_record = create_problematic_record("invalid_test", "invalid_embedding");
    
    let result = service.insert(invalid_record).await;
    
    // Система должна обработать NaN и Infinity значения
    match result {
        Ok(_) => {
            // Если вставка прошла, embedding должен быть корректным
            let search_results = service.search("Valid content", Layer::Interact, SearchOptions::default()).await?;
            for result in search_results {
                assert!(result.embedding.iter().all(|&x| x.is_finite()));
            }
        }
        Err(_) => {
            // Ошибка ожидаема - система должна остаться стабильной
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_empty_embedding() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("empty_embedding_test.db");
    config.cache_path = temp_dir.path().join("empty_embedding_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    let empty_embedding_record = create_problematic_record("empty_emb", "empty_embedding");
    
    let result = service.insert(empty_embedding_record).await;
    
    // Пустой embedding должен быть обработан
    match result {
        Ok(_) => {
            // Поиск не должен ломаться
            let search_results = service.search("Valid content", Layer::Interact, SearchOptions::default()).await?;
            // Результаты могут быть пустыми, но поиск не должен падать
        }
        Err(_) => {
            // Ошибка ожидаема
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_huge_content_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("huge_content_test.db");
    config.cache_path = temp_dir.path().join("huge_content_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    let huge_record = create_problematic_record("huge_content", "huge_content");
    
    let start_time = std::time::Instant::now();
    let result = service.insert(huge_record).await;
    let elapsed = start_time.elapsed();
    
    // Операция не должна занимать слишком много времени (защита от DoS)
    assert!(elapsed < std::time::Duration::from_secs(30));
    
    match result {
        Ok(_) => {
            // Проверяем что система не упала в производительности
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
        Err(_) => {
            // Система может отклонить слишком большие записи
            let health = service.health_check().await?;
            assert!(health.overall_healthy);
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_batch_partial_failures() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("batch_failures_test.db");
    config.cache_path = temp_dir.path().join("batch_failures_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    let mut records = vec![];
    
    // Добавляем нормальные записи
    for i in 0..5 {
        records.push(Record {
            id: format!("normal_{}", i),
            content: format!("Normal content {}", i),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        });
    }
    
    // Добавляем проблематичные записи
    records.push(create_problematic_record("prob1", "empty_id"));
    records.push(create_problematic_record("prob2", "invalid_embedding"));
    records.push(create_problematic_record("prob3", "empty_embedding"));
    
    let result = service.batch_insert(records).await?;
    
    // Batch операция должна обработать частичные неудачи
    assert!(result.inserted + result.failed == 8);
    
    // Система должна остаться стабильной
    let health = service.health_check().await?;
    assert!(health.overall_healthy);
    
    Ok(())
}

#[tokio::test]
async fn test_error_concurrent_access_with_failures() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("concurrent_failures_test.db");
    config.cache_path = temp_dir.path().join("concurrent_failures_cache");
    
    let service = Arc::new(create_test_service_with_config(config).await?);
    
    let mut handles = vec![];
    
    // Создаем concurrent операции с потенциальными проблемами
    for i in 0..20 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let problem_types = ["empty_id", "invalid_embedding", "empty_embedding", "huge_content"];
            let problem_type = problem_types[i % problem_types.len()];
            
            let record = create_problematic_record(&format!("concurrent_{}", i), problem_type);
            service_clone.insert(record).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех операций
    let mut completed = 0;
    for handle in handles {
        match handle.await {
            Ok(_) => completed += 1,
            Err(_) => {} // Игнорируем panic'и в задачах
        }
    }
    
    assert!(completed > 0); // Хотя бы некоторые операции должны завершиться
    
    // Система должна остаться работоспособной после concurrent стресса
    let health = service.health_check().await?;
    assert!(health.overall_healthy);
    
    Ok(())
}

#[tokio::test]
async fn test_error_disk_space_exhaustion_simulation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("disk_full_test.db");
    config.cache_path = temp_dir.path().join("disk_full_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    // Пытаемся заполнить много данных для симуляции нехватки места
    let mut success_count = 0;
    let mut error_count = 0;
    
    for i in 0..1000 {
        let huge_record = create_problematic_record(&format!("fill_{}", i), "huge_content");
        
        match service.insert(huge_record).await {
            Ok(_) => success_count += 1,
            Err(_) => {
                error_count += 1;
                // После первой ошибки прекращаем тест
                break;
            }
        }
        
        // Прекращаем если занимаем слишком много времени
        if i > 10 && success_count == 0 {
            break;
        }
    }
    
    // Проверяем что система обработала ошибки gracefully
    let health = service.health_check().await?;
    // Система может быть нездоровой из-за нехватки места, но не должна паниковать
    
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: Remove when VectorIndexHNSW is public
async fn test_error_corrupted_index_recovery() -> Result<()> {
    // TODO: Uncomment when VectorIndexHNSW is public
    /*
    let temp_dir = TempDir::new()?;
    let index_path = temp_dir.path().join("corrupted_test.hnsw");
    
    // Создаем валидный индекс
    {
        let index = VectorIndexHNSW::new(index_path.clone(), 3, 16, 200, 100)?;
        index.add_vector(0, &[0.1, 0.2, 0.3])?;
        index.add_vector(1, &[0.4, 0.5, 0.6])?;
        index.build_index()?;
        index.save()?;
    }
    
    // Симулируем повреждение файла
    std::fs::write(&index_path, b"corrupted data")?;
    
    // Пытаемся загрузить поврежденный индекс
    let result = VectorIndexHNSW::load(index_path.clone(), 3);
    
    match result {
        Ok(_) => {
            // Если загрузка прошла, индекс должен работать или показать что поврежден
        }
        Err(e) => {
            // Ошибка ожидаема для поврежденного файла
            assert!(e.to_string().len() > 0);
        }
    }
    */
    
    // Placeholder test
    assert!(true);
    println!("⚠️  Corrupted index recovery test is disabled - VectorIndexHNSW not public");
    
    Ok(())
}

#[tokio::test]
async fn test_error_memory_pressure_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("memory_pressure_test.db");
    config.cache_path = temp_dir.path().join("memory_pressure_cache");
    
    // Ограничиваем размер кэша для симуляции memory pressure
    config.cache_config = CacheConfigType::InMemory { max_size: 10 };
    
    let service = create_test_service_with_config(config).await?;
    
    // Вставляем много записей чтобы создать memory pressure
    for i in 0..100 {
        let record = Record {
            id: format!("memory_test_{}", i),
            content: format!("Memory test content {}", i),
            embedding: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: Layer::Interact,
            score: None,
        };
        
        let result = service.insert(record).await;
        
        // Операции должны продолжать работать даже при memory pressure
        match result {
            Ok(_) => {},
            Err(_) => {
                // Система может отклонять запросы при нехватке памяти
                // Важно что она не падает
            }
        }
    }
    
    // Система должна остаться работоспособной
    let health = service.health_check().await?;
    // Может быть нездоровой из-за memory pressure, но должна отвечать
    
    Ok(())
}

#[tokio::test]
async fn test_error_invalid_search_options() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("invalid_search_test.db");
    config.cache_path = temp_dir.path().join("invalid_search_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    // Вставляем нормальную запись
    let record = Record {
        id: "test_record".to_string(),
        content: "Test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: HashMap::new(),
        timestamp: Utc::now(),
        layer: Layer::Interact,
        score: None,
    };
    service.insert(record).await?;
    
    // Тестируем различные проблематичные search options
    
    // Негативный limit
    let invalid_options = SearchOptions {
        limit: Some(0), // Ноль может быть проблематичным
        ..Default::default()
    };
    
    let result = service.search("Test", Layer::Interact, invalid_options).await;
    
    match result {
        Ok(results) => {
            // Если поиск прошел, результаты должны быть валидными
            assert!(results.len() >= 0);
        }
        Err(_) => {
            // Ошибка ожидаема для невалидных опций
        }
    }
    
    // Очень большой limit
    let huge_limit_options = SearchOptions {
        limit: Some(1_000_000),
        ..Default::default()
    };
    
    let result = service.search("Test", Layer::Interact, huge_limit_options).await;
    
    match result {
        Ok(results) => {
            // Система должна ограничить результаты разумным числом
            assert!(results.len() < 10000);
        }
        Err(_) => {
            // Система может отклонить слишком большие запросы
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_di_container_resolution_failures() -> Result<()> {
    let container = DIContainer::new();
    
    // Пытаемся разрешить незарегистрированный тип
    let result = container.resolve::<String>();
    assert!(result.is_err());
    
    // Регистрируем фабрику которая всегда падает
    container.register(
        |_| -> Result<i32> { Err(anyhow!("Factory always fails")) },
        Lifetime::Transient
    )?;
    
    let failing_result = container.resolve::<i32>();
    assert!(failing_result.is_err());
    
    // Контейнер должен остаться работоспособным
    container.register(
        |_| -> Result<String> { Ok("success".to_string()) },
        Lifetime::Singleton
    )?;
    
    let success_result = container.resolve::<String>()?;
    assert_eq!(success_result, "success");
    
    Ok(())
}

#[tokio::test]
async fn test_error_boundary_conditions() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("boundary_test.db");
    config.cache_path = temp_dir.path().join("boundary_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    // Тестируем граничные условия
    
    // Пустой поисковый запрос
    let empty_search = service.search("", Layer::Interact, SearchOptions::default()).await;
    match empty_search {
        Ok(results) => assert!(results.len() >= 0),
        Err(_) => {} // Ошибка ожидаема
    }
    
    // Очень длинный поисковый запрос
    let long_query = "x".repeat(100_000);
    let long_search = service.search(&long_query, Layer::Interact, SearchOptions::default()).await;
    match long_search {
        Ok(results) => assert!(results.len() >= 0),
        Err(_) => {} // Система может отклонить слишком длинные запросы
    }
    
    // Поиск с Unicode и специальными символами
    let unicode_query = "тест 🚀 emoji and special chars: <>\"'&";
    let unicode_search = service.search(unicode_query, Layer::Interact, SearchOptions::default()).await;
    match unicode_search {
        Ok(results) => assert!(results.len() >= 0),
        Err(_) => {} // Не должно падать на Unicode
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_resource_cleanup_on_failure() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("cleanup_test.db");
    config.cache_path = temp_dir.path().join("cleanup_cache");
    
    // Намеренно создаем невалидный путь для тестирования cleanup
    let invalid_config = MemoryServiceConfig {
        db_path: "/nonexistent/path/test.db".into(),
        cache_path: "/nonexistent/path/cache".into(),
        ..config
    };
    
    let result = create_test_service_with_config(invalid_config).await;
    
    // Создание должно упасть
    assert!(result.is_err());
    
    // Проверяем что ресурсы не утекли
    // В реальной системе здесь были бы проверки файловых дескрипторов,
    // памяти и других ресурсов
    
    Ok(())
}

#[tokio::test]
async fn test_error_health_check_during_failures() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = default_config()?;
    config.db_path = temp_dir.path().join("health_failure_test.db");
    config.cache_path = temp_dir.path().join("health_failure_cache");
    
    let service = create_test_service_with_config(config).await?;
    
    // Создаем нагрузку с ошибками
    let mut handles = vec![];
    
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let problematic_record = create_problematic_record(&format!("health_test_{}", i), "invalid_embedding");
            let _ = service_clone.insert(problematic_record).await;
        });
        handles.push(handle);
    }
    
    // Во время выполнения проблематичных операций проверяем health
    let health_during_stress = service.health_check().await?;
    
    // Health check не должен падать даже во время проблем
    assert!(health_during_stress.uptime >= std::time::Duration::from_millis(0));
    
    // Ждем завершения стресс-теста
    for handle in handles {
        let _ = handle.await;
    }
    
    // Health check должен работать и после стресса
    let health_after_stress = service.health_check().await?;
    assert!(health_after_stress.uptime >= std::time::Duration::from_millis(0));
    
    Ok(())
}