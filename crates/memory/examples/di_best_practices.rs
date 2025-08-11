#![cfg(feature = "extended-tests")]

use anyhow::Result;
use chrono::Utc;
use memory::{default_config, DIMemoryService, Layer, MemoryServiceConfig, Record, SearchOptions};
use std::sync::Arc;
use uuid::Uuid;

/// Пример лучших практик использования DIMemoryService

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логирования с полной информацией
    tracing_subscriber::fmt()
        .with_env_filter("memory=debug,ai=info")
        .init();

    println!("=== DI Memory Service Best Practices Demo ===\n");

    // 1. Правильная конфигурация сервиса
    println!("1. Настройка конфигурации...");
    let config = create_optimized_config()?;

    // 2. Создание сервиса с обработкой ошибок
    println!("2. Создание DI Memory Service...");
    let service = match DIMemoryService::new(config).await {
        Ok(s) => {
            println!("   ✅ Сервис создан успешно");
            Arc::new(s)
        }
        Err(e) => {
            eprintln!("   ❌ Ошибка создания сервиса: {}", e);
            return Err(e);
        }
    };

    // 3. Инициализация слоев с проверкой
    println!("3. Инициализация слоев памяти...");
    match service.initialize().await {
        Ok(_) => println!("   ✅ Слои инициализированы"),
        Err(e) => {
            eprintln!("   ⚠️  Ошибка инициализации слоев: {}", e);
            println!("   ℹ️  Продолжаем работу - слои будут созданы при первом использовании");
        }
    }

    // 4. Демонстрация корректной вставки данных
    println!("\n4. Вставка данных с полной структурой...");
    demo_insert_records(&service).await?;

    // 5. Эффективный поиск с опциями
    println!("\n5. Эффективный поиск...");
    demo_efficient_search(&service).await?;

    // 6. Работа с метриками производительности
    println!("\n6. Анализ производительности...");
    demo_performance_analysis(&service).await?;

    // 7. Управление памятью и promotion
    println!("\n7. Управление жизненным циклом данных...");
    demo_memory_management(&service).await?;

    // 8. Проверка здоровья системы
    println!("\n8. Мониторинг здоровья...");
    demo_health_monitoring(&service).await?;

    // 9. Batch операции для производительности
    println!("\n9. Batch операции...");
    demo_batch_operations(&service).await?;

    // 10. Graceful shutdown
    println!("\n10. Завершение работы...");
    // DIMemoryService автоматически сохраняет состояние при drop
    drop(service);
    println!("    ✅ Сервис корректно завершен");

    println!("\n=== Demo completed successfully! ===");
    Ok(())
}

/// Создание оптимизированной конфигурации
fn create_optimized_config() -> Result<MemoryServiceConfig> {
    let mut config = default_config()?;

    // Оптимизация для production
    config.promotion.interact_ttl = 3600; // 1 час для взаимодействий
    config.promotion.insights_ttl = 86400 * 7; // 1 неделя для инсайтов
    config.promotion.promotion_threshold = 0.7; // Порог для продвижения

    // Настройка кэша
    use memory::CacheConfig;
    config.cache_config = CacheConfig::production();

    // Настройка здоровья системы
    config.health_config.check_interval_seconds = 60;
    config.health_config.enable_auto_recovery = true;

    // Batch конфигурация для производительности
    config.batch_config.max_batch_size = 100;
    config.batch_config.batch_timeout_ms = 50;

    Ok(config)
}

/// Демонстрация правильной вставки записей
async fn demo_insert_records(service: &Arc<DIMemoryService>) -> Result<()> {
    // Создаем полноценные записи со всеми полями
    let records = vec![
        create_record(
            "Rust async/await позволяет писать асинхронный код как синхронный",
            Layer::Interact,
            "learning",
            vec!["rust", "async", "programming"],
            "rust-learning",
        ),
        create_record(
            "Используйте Arc<Mutex<T>> для shared state в многопоточном коде",
            Layer::Insights,
            "best-practice",
            vec!["rust", "concurrency", "threading"],
            "rust-patterns",
        ),
        create_record(
            "SOLID принципы применимы и в Rust через traits и модули",
            Layer::Assets,
            "architecture",
            vec!["design", "solid", "rust"],
            "architecture",
        ),
    ];

    for (i, record) in records.into_iter().enumerate() {
        match service.insert(record).await {
            Ok(_) => println!("   ✅ Запись {} вставлена", i + 1),
            Err(e) => eprintln!("   ❌ Ошибка вставки записи {}: {}", i + 1, e),
        }
    }

    Ok(())
}

/// Создание записи с полной структурой
fn create_record(text: &str, layer: Layer, kind: &str, tags: Vec<&str>, project: &str) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        embedding: vec![], // Будет создан автоматически
        layer,
        kind: kind.to_string(),
        tags: tags.into_iter().map(String::from).collect(),
        project: project.to_string(),
        session: Uuid::new_v4().to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    }
}

/// Демонстрация эффективного поиска
async fn demo_efficient_search(service: &Arc<DIMemoryService>) -> Result<()> {
    // Поиск с полными опциями
    let options = SearchOptions {
        top_k: 3,
        score_threshold: 0.5,
        tags: vec!["rust".to_string()],
        project: Some("rust-learning".to_string()),
        layers: vec![Layer::Interact, Layer::Insights],
    };

    let results = service
        .search("async programming", Layer::Interact, options)
        .await?;

    println!("   Найдено {} результатов:", results.len());
    for (i, record) in results.iter().enumerate() {
        println!(
            "   {}. [{}] {} (score: {:.3})",
            i + 1,
            record.layer,
            &record.text[..50.min(record.text.len())],
            record.score
        );
    }

    Ok(())
}

/// Анализ производительности
async fn demo_performance_analysis(service: &Arc<DIMemoryService>) -> Result<()> {
    let metrics = service.get_performance_metrics();

    println!("   📊 Метрики производительности:");
    println!("      • Всего операций resolve: {}", metrics.total_resolves);
    println!("      • Cache hit rate: {:.1}%", metrics.cache_hit_rate());
    println!(
        "      • Средняя скорость resolve: {:.1}μs",
        metrics.avg_resolve_time_us()
    );

    // Показываем детальный отчет
    println!("\n   📈 Детальный отчет:");
    let report = service.get_performance_report();
    for line in report.lines() {
        println!("      {}", line);
    }

    Ok(())
}

/// Управление памятью
async fn demo_memory_management(service: &Arc<DIMemoryService>) -> Result<()> {
    // Запускаем promotion cycle
    let stats = service.run_promotion().await?;

    println!("   🔄 Результаты promotion:");
    println!(
        "      • Interact → Insights: {}",
        stats.interact_to_insights
    );
    println!("      • Insights → Assets: {}", stats.insights_to_assets);
    println!(
        "      • Удалено устаревших: {} + {}",
        stats.expired_interact, stats.expired_insights
    );
    println!("      • Время выполнения: {}ms", stats.total_time_ms);

    // Получаем общую статистику
    let system_stats = service.get_stats().await;
    println!("\n   📊 Статистика системы:");
    println!("      • Всего записей: {}", system_stats.total_records);
    println!("      • Распределение по слоям:");
    println!("        - Interact: {}", system_stats.interact_count);
    println!("        - Insights: {}", system_stats.insights_count);
    println!("        - Assets: {}", system_stats.assets_count);

    Ok(())
}

/// Мониторинг здоровья
async fn demo_health_monitoring(service: &Arc<DIMemoryService>) -> Result<()> {
    let health = service.check_health().await?;

    let status_icon = match health.overall_status {
        memory::health::HealthStatus::Healthy => "✅",
        memory::health::HealthStatus::Degraded => "⚠️",
        memory::health::HealthStatus::Unhealthy => "❌",
        memory::health::HealthStatus::Down => "💀",
    };

    println!(
        "   {} Общий статус: {:?}",
        status_icon, health.overall_status
    );
    println!("   ⏱️  Uptime: {} секунд", health.uptime_seconds);

    if !health.active_alerts.is_empty() {
        println!("   ⚠️  Активные алерты:");
        for alert in &health.active_alerts {
            println!(
                "      • [{:?}] {}: {}",
                alert.severity, alert.title, alert.description
            );
        }
    }

    Ok(())
}

/// Batch операции для производительности
async fn demo_batch_operations(service: &Arc<DIMemoryService>) -> Result<()> {
    // Создаем batch записей
    let batch_records: Vec<Record> = (0..5)
        .map(|i| {
            create_record(
                &format!("Batch record {}: test data for performance", i),
                Layer::Interact,
                "batch-test",
                vec!["batch", "test"],
                "batch-demo",
            )
        })
        .collect();

    // Вставляем по одной для демонстрации
    // В реальном коде используйте batch методы если они доступны
    let start = std::time::Instant::now();
    for record in batch_records {
        service.insert(record).await?;
    }
    let elapsed = start.elapsed();

    println!("   ✅ Вставлено 5 записей за {:?}", elapsed);
    println!("   ⚡ Среднее время на запись: {:?}", elapsed / 5);

    Ok(())
}
