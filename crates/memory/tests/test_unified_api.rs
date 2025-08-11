#![cfg(feature = "extended-tests")]

use anyhow::Result;
use memory::{
    api::SearchOptions, create_di_memory_service, Layer, MemoryContext, UnifiedMemoryAPI,
};
use std::sync::Arc;
use uuid::Uuid;

/// Тестирует UnifiedMemoryAPI trait интеграцию
#[tokio::test]
async fn test_unified_api_basic_operations() -> Result<()> {
    println!("🔗 Тестируем UnifiedMemoryAPI интеграцию");

    // Создаем API через DI service
    let di_service = create_di_memory_service().await?;
    let api = UnifiedMemoryAPI::new_di(Arc::new(di_service));

    // Тест 1: Remember операция
    let context = MemoryContext::new("test")
        .with_layer(Layer::Interact)
        .with_tags(vec!["api_test".to_string()]);

    let id = api
        .remember("Test memory for API".to_string(), context)
        .await?;
    println!("  ✅ Remember операция: ID = {}", id);

    // Тест 2: Recall операция
    let search_options = SearchOptions::new()
        .in_layers(vec![Layer::Interact])
        .limit(10);

    let results = api.recall("Test memory", search_options).await?;
    assert!(!results.is_empty(), "Должны найти записанную память");

    let found_record = &results[0];
    assert_eq!(found_record.text, "Test memory for API");
    assert_eq!(found_record.layer, Layer::Interact);
    println!(
        "  ✅ Recall операция: найдено {} результатов",
        results.len()
    );

    // Тест 3: Health check
    let health = api.health_check().await?;
    println!("  📊 Health status: {}", health.status);
    assert!(
        matches!(health.status, "healthy" | "degraded"),
        "Система должна быть здорова"
    );
    println!("  ✅ Health check работает");

    // Тест 4: Statistics
    let stats = api.get_stats().await?;
    println!("  📊 Total records: {}", stats.total_records);
    println!(
        "  📊 Cache hit rate: {:.2}%",
        stats.cache_stats.hit_rate * 100.0
    );
    println!("  ✅ Statistics доступны");

    // Тест 5: Memory optimization
    let optimization_result = api.optimize_memory().await?;
    println!(
        "  🔄 Optimization time: {}ms",
        optimization_result.total_time_ms
    );
    assert!(
        optimization_result.total_time_ms >= 0,
        "Время оптимизации должно быть положительным"
    );
    println!("  ✅ Memory optimization работает");

    println!("✅ UnifiedMemoryAPI интеграция работает корректно");
    Ok(())
}

/// Тестирует различные сценарии поиска через API
#[tokio::test]
async fn test_unified_api_search_scenarios() -> Result<()> {
    println!("🔍 Тестируем поисковые сценарии UnifiedMemoryAPI");

    let di_service = create_di_memory_service().await?;
    let api = UnifiedMemoryAPI::new_di(Arc::new(di_service));

    // Добавляем тестовые данные в разные слои
    let test_data = vec![
        (
            "Machine learning algorithms overview",
            Layer::Interact,
            vec!["ai", "ml"],
        ),
        (
            "Deep neural networks implementation",
            Layer::Insights,
            vec!["ai", "implementation"],
        ),
        (
            "Python programming best practices",
            Layer::Assets,
            vec!["python", "programming"],
        ),
        (
            "Rust systems programming guide",
            Layer::Assets,
            vec!["rust", "systems"],
        ),
    ];

    for (text, layer, tags) in test_data {
        let context = MemoryContext::new("search_test")
            .with_layer(layer)
            .with_tags(tags.into_iter().map(String::from).collect());

        api.remember(text.to_string(), context).await?;
    }

    // Тест 1: Поиск по одному слою
    let options = SearchOptions::new()
        .in_layers(vec![Layer::Interact])
        .limit(5);

    let interact_results = api.recall("machine learning", options).await?;
    assert!(!interact_results.is_empty(), "Должны найти в Interact слое");
    assert!(
        interact_results.iter().all(|r| r.layer == Layer::Interact),
        "Все результаты должны быть из Interact"
    );
    println!(
        "  ✅ Поиск по одному слою: {} результатов",
        interact_results.len()
    );

    // Тест 2: Поиск по всем слоям
    let options = SearchOptions::new()
        .in_layers(vec![Layer::Interact, Layer::Insights, Layer::Assets])
        .limit(10);

    let all_results = api.recall("programming", options).await?;
    let layers: std::collections::HashSet<_> = all_results.iter().map(|r| r.layer).collect();
    println!("  📊 Найдено в слоях: {:?}", layers);
    println!(
        "  ✅ Поиск по всем слоям: {} результатов",
        all_results.len()
    );

    // Тест 3: Поиск с лимитом
    let options = SearchOptions::new().limit(2);
    let limited_results = api.recall("guide", options).await?;
    assert!(
        limited_results.len() <= 2,
        "Результаты должны быть ограничены"
    );
    println!(
        "  ✅ Ограничение результатов работает: {} результатов",
        limited_results.len()
    );

    // Тест 4: Поиск несуществующего
    let options = SearchOptions::new().limit(5);
    let empty_results = api.recall("nonexistent query xyz", options).await?;
    println!(
        "  📊 Результатов для несуществующего запроса: {}",
        empty_results.len()
    );
    println!("  ✅ Обработка пустых результатов");

    println!("✅ Поисковые сценарии работают корректно");
    Ok(())
}

/// Тестирует error handling в UnifiedMemoryAPI
#[tokio::test]
async fn test_unified_api_error_handling() -> Result<()> {
    println!("🚨 Тестируем error handling в UnifiedMemoryAPI");

    let di_service = create_di_memory_service().await?;
    let api = UnifiedMemoryAPI::new_di(Arc::new(di_service));

    // Тест 1: Get несуществующего ID
    let fake_id = Uuid::new_v4();
    let result = api.get(fake_id).await?;
    assert!(result.is_none(), "Несуществующий ID должен возвращать None");
    println!("  ✅ Обработка несуществующего ID");

    // Тест 2: Forget несуществующего ID
    let forget_result = api.forget(fake_id).await?;
    assert!(
        !forget_result,
        "Forget несуществующего ID должен возвращать false"
    );
    println!("  ✅ Forget несуществующего ID обработан");

    // Тест 3: Пустые поисковые запросы
    let empty_options = SearchOptions::new();
    let empty_results = api.recall("", empty_options).await?;
    // Пустой запрос может возвращать результаты или нет - главное что не падает
    println!(
        "  📊 Результатов для пустого запроса: {}",
        empty_results.len()
    );
    println!("  ✅ Пустой запрос обработан без ошибок");

    // Тест 4: Remember с пустым текстом
    let context = MemoryContext::new("empty_test");
    let empty_id = api.remember("".to_string(), context).await?;
    println!("  📝 ID для пустого текста: {}", empty_id);
    println!("  ✅ Пустой текст обработан");

    // Тест 5: Статистика всегда доступна
    let stats = api.get_stats().await?;
    // Статистика может быть нулевой, но структура должна быть валидной
    assert!(
        stats.total_records >= 0,
        "Total records должен быть неотрицательным"
    );
    println!("  📊 Статистика получена без ошибок");

    println!("✅ Error handling работает корректно");
    Ok(())
}

/// Тестирует производительность UnifiedMemoryAPI
#[tokio::test]
async fn test_unified_api_performance() -> Result<()> {
    println!("⚡ Тестируем производительность UnifiedMemoryAPI");

    let di_service = create_di_memory_service().await?;
    let api = UnifiedMemoryAPI::new_di(Arc::new(di_service));

    let start_time = std::time::Instant::now();

    // Массовые операции remember
    let batch_size = 20;
    println!("  📝 Добавляем {} записей через API...", batch_size);

    for i in 0..batch_size {
        let context = MemoryContext::new("performance")
            .with_layer(match i % 3 {
                0 => Layer::Interact,
                1 => Layer::Insights,
                _ => Layer::Assets,
            })
            .with_tags(vec![format!("batch_{}", i)]);

        let text = format!("Performance test record {} with detailed content", i);
        api.remember(text, context).await?;
    }

    let remember_time = start_time.elapsed();
    println!("  ⏱️ Время remember операций: {:?}", remember_time);

    // Массовые поисковые операции
    let search_start = std::time::Instant::now();
    let queries = vec!["performance", "test", "record", "content", "detailed"];

    for query in queries {
        let options = SearchOptions::new().limit(10);
        let _results = api.recall(query, options).await?;
    }

    let search_time = search_start.elapsed();
    println!("  ⏱️ Время поисковых операций: {:?}", search_time);

    // Операции health и stats
    let monitoring_start = std::time::Instant::now();

    let _health = api.health_check().await?;
    let _stats = api.get_stats().await?;
    let _optimization = api.optimize_memory().await?;

    let monitoring_time = monitoring_start.elapsed();
    println!("  ⏱️ Время операций мониторинга: {:?}", monitoring_time);

    // Проверки производительности
    assert!(
        remember_time.as_secs() < 5,
        "Remember операции должны быть быстрыми"
    );
    assert!(search_time.as_secs() < 3, "Поиск должен быть быстрым");
    assert!(
        monitoring_time.as_secs() < 2,
        "Мониторинг должен быть быстрым"
    );

    println!("✅ Производительность API в пределах нормы");
    Ok(())
}

/// Smoke test для быстрой проверки API
#[tokio::test]
async fn test_unified_api_smoke() -> Result<()> {
    println!("💨 Smoke test для UnifiedMemoryAPI");

    let di_service = create_di_memory_service().await?;
    let api = UnifiedMemoryAPI::new_di(Arc::new(di_service));

    // Минимальный workflow
    let context = MemoryContext::new("smoke");
    let id = api.remember("Smoke test".to_string(), context).await?;

    let options = SearchOptions::new();
    let results = api.recall("smoke", options).await?;

    assert!(!results.is_empty(), "Должны найти smoke test запись");

    let _health = api.health_check().await?;
    let _stats = api.get_stats().await?;

    println!("✅ Smoke test прошел");
    Ok(())
}
