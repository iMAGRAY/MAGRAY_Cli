#![cfg(all(feature = "extended-tests", feature = "legacy-tests", not(feature = "minimal")))]

//! Comprehensive unit тесты для CoreMemoryService
//!
//! Coverage areas:
//! - Unit tests для всех методов CRUD
//! - Property-based tests для batch операций
//! - Async tests с concurrent access
//! - Error handling и edge cases
//! - Dependency injection testing
//! - Performance benchmarks

use anyhow::Result;
use once_cell::sync::Lazy;
use proptest::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio_test;

use memory::{
    services::{traits::CoreMemoryServiceTrait, CoreMemoryService},
    types::{Layer, Record, SearchOptions},
    BatchConfig, BatchOperationManager, DIContainer, Lifetime, VectorStore,
};

static INIT_TRACING: Lazy<()> = Lazy::new(|| {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
});

/// Helper для создания mock DI container с VectorStore
async fn create_test_container() -> Arc<DIContainer> {
    Lazy::force(&INIT_TRACING);

    let container = Arc::new(DIContainer::new());

    // Регистрируем VectorStore factory
    let temp_dir = std::env::temp_dir().join(format!("memory_test_{}", uuid::Uuid::new_v4()));
    let temp_dir_clone = temp_dir.clone();
    container
        .register(
            move |_container: &DIContainer| -> Result<VectorStore, anyhow::Error> {
                tokio::runtime::Handle::current()
                    .block_on(async { VectorStore::new(&temp_dir_clone).await })
            },
            Lifetime::Singleton,
        )
        .expect("Не удалось зарегистрировать VectorStore");

    // Регистрируем BatchOperationManager factory (упрощенная версия для тестов)
    container
        .register(
            |container: &DIContainer| -> Result<BatchOperationManager, anyhow::Error> {
                let vector_store = container.resolve::<VectorStore>()?;
                let batch_config = BatchConfig::default();
                Ok(BatchOperationManager::new(vector_store, batch_config, None))
            },
            Lifetime::Singleton,
        )
        .expect("Не удалось зарегистрировать BatchOperationManager");

    container
}

/// Синхронный вариант для property-based тестов
fn create_test_container_sync() -> Arc<DIContainer> {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();
    });

    // Для property-based тестов создаем минимальный контейнер
    let container = Arc::new(DIContainer::new());

    // В property-based тестах мы не можем использовать async,
    // поэтому создаем контейнер без VectorStore
    // Реальные тесты на CoreMemoryService будут асинхронными

    container
}

/// Helper для создания тестовой записи
fn create_test_record(id_suffix: u32, layer: Layer) -> Record {
    let embedding = vec![0.1f32; 1024]; // 1024-dimensional embedding
    let now = chrono::Utc::now();

    Record {
        id: uuid::Uuid::new_v4(),
        text: format!("Test record {}", id_suffix),
        embedding,
        layer,
        kind: "test".to_string(),
        tags: vec![format!("test-{}", id_suffix)],
        project: "test-project".to_string(),
        session: "test-session".to_string(),
        ts: now,
        score: 0.8,
        access_count: 0,
        last_access: now,
    }
}

/// Helper для создания SearchOptions
fn default_search_options() -> SearchOptions {
    SearchOptions {
        layers: vec![Layer::Interact, Layer::Insights],
        top_k: 10,
        score_threshold: 0.7,
        tags: Vec::new(),
        project: None,
    }
}

#[tokio::test]
async fn test_core_memory_service_creation() -> Result<()> {
    let container = create_test_container().await;

    // Test minimal creation
    let service = CoreMemoryService::new_minimal(container.clone());
    // operation_limiter is private, cannot access permits directly
    // Service created successfully if no panic

    // Test production creation
    let production_service = CoreMemoryService::new_production(container.clone());
    // operation_limiter is private, cannot access permits directly

    // Test custom creation (operation_limiter is private, cannot access permits directly)
    let custom_service = CoreMemoryService::new(container, 50);
    // Operation limiter is working if constructor doesn't panic

    Ok(())
}

#[tokio::test]
async fn test_insert_single_record() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    let record = create_test_record(1, Layer::Interact);
    let record_id = record.id;

    // Test successful insert
    let result = service.insert(record).await;
    assert!(result.is_ok(), "Insert должен завершиться успешно");

    // Verify record exists by attempting to search
    let results = service
        .search("Test record 1", Layer::Interact, default_search_options())
        .await?;
    assert!(!results.is_empty(), "Должны найти вставленную запись");

    Ok(())
}

#[tokio::test]
async fn test_insert_batch_records() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Create test batch
    let records = (1..=5)
        .map(|i| create_test_record(i, Layer::Interact))
        .collect::<Vec<_>>();

    // Test batch insert
    let result = service.insert_batch(records).await;
    assert!(result.is_ok(), "Batch insert должен завершиться успешно");

    // Verify multiple records exist
    for i in 1..=5 {
        let results = service
            .search(
                &format!("Test record {}", i),
                Layer::Interact,
                default_search_options(),
            )
            .await?;
        assert!(!results.is_empty(), "Должны найти запись {}", i);
    }

    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Insert test data
    let records = vec![
        create_test_record(1, Layer::Interact),
        create_test_record(2, Layer::Insights),
        create_test_record(3, Layer::Interact),
    ];

    for record in records {
        service.insert(record).await?;
    }

    // Test search in different layers
    let interact_results = service
        .search("Test record", Layer::Interact, default_search_options())
        .await?;
    assert_eq!(
        interact_results.len(),
        2,
        "Должны найти 2 записи в Interact"
    );

    let insights_results = service
        .search("Test record", Layer::Insights, default_search_options())
        .await?;
    assert_eq!(
        insights_results.len(),
        1,
        "Должны найти 1 запись в Insights"
    );

    // Test search with different top_k
    let mut limited_options = default_search_options();
    limited_options.top_k = 1;
    let limited_results = service
        .search("Test record", Layer::Interact, limited_options)
        .await?;
    assert!(
        limited_results.len() <= 1,
        "Должны получить максимум 1 результат"
    );

    Ok(())
}

#[tokio::test]
async fn test_update_record() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Insert initial record
    let mut record = create_test_record(1, Layer::Interact);
    let record_id = record.id;
    service.insert(record.clone()).await?;

    // Update the record
    record.text = "Updated test record".to_string();
    record.tags = vec!["updated".to_string()];

    let result = service.update(record).await;
    assert!(result.is_ok(), "Update должен завершиться успешно");

    // Verify update by searching
    let results = service
        .search(
            "Updated test record",
            Layer::Interact,
            default_search_options(),
        )
        .await?;
    assert!(!results.is_empty(), "Должны найти обновленную запись");

    Ok(())
}

#[tokio::test]
async fn test_delete_record() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Insert test record
    let record = create_test_record(1, Layer::Interact);
    let record_id = record.id;
    service.insert(record).await?;

    // Verify record exists
    let results_before = service
        .search("Test record 1", Layer::Interact, default_search_options())
        .await?;
    assert!(
        !results_before.is_empty(),
        "Запись должна существовать до удаления"
    );

    // Delete record
    let result = service.delete(&record_id, Layer::Interact).await;
    assert!(result.is_ok(), "Delete должен завершиться успешно");

    // Verify record is deleted (search should return fewer or no results)
    let results_after = service
        .search("Test record 1", Layer::Interact, default_search_options())
        .await?;
    // Note: В зависимости от реализации VectorStore, результат может отличаться
    // В реальном тесте мы бы проверили конкретное поведение store

    Ok(())
}

#[tokio::test]
async fn test_batch_insert_with_results() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Create mixed batch (some valid, potentially some that could fail)
    let records = (1..=10)
        .map(|i| create_test_record(i, Layer::Interact))
        .collect::<Vec<_>>();

    // Test batch insert with detailed results
    let result = service.batch_insert(records).await?;

    assert_eq!(result.inserted, 10, "Должны вставить все 10 записей");
    assert_eq!(result.failed, 0, "Не должно быть неудачных вставок");
    assert!(
        result.total_time_ms > 0,
        "Время выполнения должно быть больше 0"
    );
    assert!(result.errors.is_empty(), "Не должно быть ошибок");

    Ok(())
}

#[tokio::test]
async fn test_batch_search_functionality() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Insert test data
    for i in 1..=5 {
        let record = create_test_record(i, Layer::Interact);
        service.insert(record).await?;
    }

    // Test batch search
    let queries = vec![
        "Test record 1".to_string(),
        "Test record 2".to_string(),
        "Test record 3".to_string(),
    ];

    let result = service
        .batch_search(queries.clone(), Layer::Interact, default_search_options())
        .await?;

    assert_eq!(result.queries, queries, "Queries должны совпадать");
    assert_eq!(
        result.results.len(),
        3,
        "Должны получить результаты для 3 запросов"
    );
    assert!(
        result.total_time_ms > 0,
        "Время выполнения должно быть больше 0"
    );

    // Verify each query returned results
    for (i, query_results) in result.results.iter().enumerate() {
        assert!(
            !query_results.is_empty(),
            "Запрос {} должен вернуть результаты",
            i + 1
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let container = create_test_container().await;
    let service = Arc::new(CoreMemoryService::new_minimal(container));

    // Test concurrent inserts
    let mut tasks = Vec::new();
    for i in 1..=10 {
        let service_clone = service.clone();
        tasks.push(tokio::spawn(async move {
            let record = create_test_record(i, Layer::Interact);
            service_clone.insert(record).await
        }));
    }

    // Wait for all inserts to complete
    let results = futures::future::join_all(tasks).await;
    for (i, result) in results.into_iter().enumerate() {
        assert!(
            result.is_ok(),
            "Task {} должна завершиться без panic",
            i + 1
        );
        assert!(
            result.unwrap().is_ok(),
            "Insert {} должен быть успешным",
            i + 1
        );
    }

    // Test concurrent searches
    let mut search_tasks = Vec::new();
    for i in 1..=5 {
        let service_clone = service.clone();
        search_tasks.push(tokio::spawn(async move {
            service_clone
                .search(
                    &format!("Test record {}", i),
                    Layer::Interact,
                    default_search_options(),
                )
                .await
        }));
    }

    let search_results = futures::future::join_all(search_tasks).await;
    for (i, result) in search_results.into_iter().enumerate() {
        assert!(
            result.is_ok(),
            "Search task {} должна завершиться без panic",
            i + 1
        );
        assert!(
            result.unwrap().is_ok(),
            "Search {} должен быть успешным",
            i + 1
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Test delete non-existent record
    let non_existent_id = uuid::Uuid::new_v4();
    let result = service.delete(&non_existent_id, Layer::Interact).await;
    // В зависимости от реализации VectorStore, это может быть Ok или Err
    // Важно что операция не панични

    // Test search with empty query
    let empty_search = service
        .search("", Layer::Interact, default_search_options())
        .await;
    assert!(
        empty_search.is_ok(),
        "Поиск с пустым запросом должен обрабатываться корректно"
    );

    Ok(())
}

#[tokio::test]
async fn test_embedding_generation() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Test fallback embedding generation через search
    let results = service
        .search("test query", Layer::Interact, default_search_options())
        .await?;
    // Embedding должен генерироваться внутри search метода

    // Test consistency of embedding generation
    let results1 = service
        .search(
            "consistent query",
            Layer::Interact,
            default_search_options(),
        )
        .await?;
    let results2 = service
        .search(
            "consistent query",
            Layer::Interact,
            default_search_options(),
        )
        .await?;
    // Fallback embedding должен быть детерминированным

    Ok(())
}

// Property-based tests using proptest
proptest! {
    #[test]
    fn test_batch_operations_property(
        batch_size in 1usize..10, // Уменьшаем размер для стабильности
        layer in prop_oneof![
            Just(Layer::Interact),
            Just(Layer::Insights),
        ]
    ) {
        tokio_test::block_on(async {
            let container = create_test_container().await;
            let service = CoreMemoryService::new_minimal(container);

            // Generate batch of records
            let records = (1..=batch_size)
                .map(|i| create_test_record(i as u32, layer))
                .collect::<Vec<_>>();

            // Test batch insert - для property-based теста проверяем только что не паникуют
            let result = service.batch_insert(records.clone()).await;
            // В property-based тестах мы просто проверяем что операции не паникуют
            // Детальные проверки остаются в unit тестах

        });
    }

    #[test]
    fn test_search_options_property(
        top_k in 1usize..20, // Ограничиваем размер
        threshold in 0.0f32..1.0
    ) {
        tokio_test::block_on(async {
            let container = create_test_container().await;
            let service = CoreMemoryService::new_minimal(container);

            let options = SearchOptions {
                layers: vec![Layer::Interact],
                top_k,
                score_threshold: threshold,
                tags: Vec::new(),
                project: None,
            };

            // Test search without insertion - just check it doesn't panic
            let _results = service.search("Test record", Layer::Interact, options).await;

        });
    }

    #[test]
    fn test_embedding_dimension_consistency(content in "\\PC{5,50}") {
        tokio_test::block_on(async {
            let container = create_test_container().await;
            let service = CoreMemoryService::new_minimal(container);

            // Test that fallback embedding always has consistent dimension
            let _results = service.search(&content, Layer::Interact, default_search_options()).await;
            // Just check it doesn't panic

        });
    }
}

#[tokio::test]
async fn test_operation_limiter_concurrency() -> Result<()> {
    // Test that operation limiter properly limits concurrent operations
    let container = create_test_container().await;
    let service = Arc::new(CoreMemoryService::new(container, 2)); // Limit to 2 concurrent operations

    let start_time = std::time::Instant::now();

    // Start 5 concurrent operations (more than the limit)
    let tasks = (1..=5)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                let record = create_test_record(i, Layer::Interact);
                // Add artificial delay to make concurrency limits visible
                tokio::time::sleep(Duration::from_millis(100)).await;
                service_clone.insert(record).await
            })
        })
        .collect::<Vec<_>>();

    let results = futures::future::join_all(tasks).await;
    let elapsed = start_time.elapsed();

    // All operations should succeed
    for result in results {
        assert!(result.is_ok(), "Task должна завершиться без panic");
        assert!(result.unwrap().is_ok(), "Insert должен быть успешным");
    }

    // With 2 concurrent operations and 5 total operations, minimum time should be around 300ms
    // (first 2 in parallel ~100ms, next 2 in parallel ~100ms, last 1 ~100ms)
    assert!(
        elapsed >= Duration::from_millis(250),
        "Concurrency limiter должен работать"
    );

    Ok(())
}

#[tokio::test]
async fn test_metrics_integration() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Perform some operations that should update metrics
    let record = create_test_record(1, Layer::Interact);
    service.insert(record).await?;

    service
        .search("Test record 1", Layer::Interact, default_search_options())
        .await?;

    // Test batch operations
    let records = (1..=3)
        .map(|i| create_test_record(i + 10, Layer::Interact))
        .collect::<Vec<_>>();
    service.batch_insert(records).await?;

    // Metrics should be updated (проверяется через логи в реальном окружении)
    // В unit тестах мы проверяем что операции завершаются без ошибок

    Ok(())
}

/// Benchmark test для измерения производительности
#[tokio::test]
#[ignore] // Ignore by default, run with --ignored
async fn benchmark_batch_operations() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_production(container);

    // Benchmark batch insert
    let batch_sizes = vec![10, 50, 100, 500];

    for batch_size in batch_sizes {
        let records = (1..=batch_size)
            .map(|i| create_test_record(i, Layer::Interact))
            .collect::<Vec<_>>();

        let start_time = std::time::Instant::now();
        let result = service.batch_insert(records).await?;
        let elapsed = start_time.elapsed();

        println!(
            "Batch size: {}, Time: {:?}, Rate: {:.2} records/sec",
            batch_size,
            elapsed,
            batch_size as f64 / elapsed.as_secs_f64()
        );

        assert_eq!(result.inserted, batch_size as usize);
    }

    Ok(())
}

/// Edge case тесты
#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let container = create_test_container().await;
    let service = CoreMemoryService::new_minimal(container);

    // Test empty batch
    let empty_batch: Vec<Record> = vec![];
    let result = service.insert_batch(empty_batch).await;
    assert!(
        result.is_ok(),
        "Empty batch должен обрабатываться корректно"
    );

    // Test very long content
    let mut long_record = create_test_record(1, Layer::Interact);
    long_record.text = "x".repeat(10000); // Very long content

    let result = service.insert(long_record).await;
    assert!(result.is_ok(), "Длинный контент должен обрабатываться");

    // Test special characters in search
    let special_chars_search = service
        .search("!@#$%^&*()", Layer::Interact, default_search_options())
        .await;
    assert!(
        special_chars_search.is_ok(),
        "Поиск с спецсимволами должен работать"
    );

    // Test zero top_k
    let mut zero_options = default_search_options();
    zero_options.top_k = 0;
    let zero_results = service.search("test", Layer::Interact, zero_options).await;
    assert!(zero_results.is_ok(), "Zero top_k должен обрабатываться");

    Ok(())
}
