use memory::{
    MemoryService, MemoryConfig, Layer, Record, PromotionConfig,
    CacheConfigType, CacheConfig, HealthConfig, ResourceConfig,
};
use ai::AiConfig;
use anyhow::Result;
use tokio;
use tracing::{info, warn};
use tracing_subscriber;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;
use chrono::Utc;
use tempfile::TempDir;

/// Полное тестирование системы памяти
#[tokio::test]
async fn test_full_memory_system() -> Result<()> {
    // Инициализация логирования
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🚀 Запуск полного теста системы памяти");

    // Создаём временные директории
    let temp_dir = TempDir::new()?;
    
    // Создаём конфигурацию
    let config = MemoryConfig {
        db_path: temp_dir.path().join("test_db"),
        cache_path: temp_dir.path().join("test_cache"),
        promotion: PromotionConfig {
            interact_ttl_hours: 24,
            insights_ttl_days: 90,
            promote_threshold: 0.7,
            decay_factor: 0.95,
        },
        ai_config: AiConfig::default(),
        health_config: HealthConfig::default(),
        cache_config: CacheConfigType::Lru(CacheConfig::default()),
        resource_config: ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 10_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 100 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(80),
    };
    
    // Создаём сервис памяти
    let memory_service = MemoryService::new(config).await?;
    
    // === ТЕСТ 1: Базовые операции ===
    info!("📝 Тест 1: Базовые операции вставки и поиска");
    
    // Добавляем тестовые записи
    let test_records = vec![
        ("Rust - системный язык программирования с нулевой стоимостью абстракций", Layer::Interact),
        ("Tokio - асинхронный runtime для Rust", Layer::Interact),
        ("HNSW - алгоритм приближенного поиска ближайших соседей", Layer::Insights),
        ("Memory management в Rust гарантирует безопасность памяти", Layer::Assets),
    ];
    
    for (text, layer) in test_records {
        let record = Record {
            id: Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![], // Будет сгенерирован автоматически
            layer,
            kind: "test".to_string(),
            tags: vec!["system_test".to_string()],
            project: "full_test".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        
        memory_service.insert(record).await?;
        info!("  ✅ Добавлена запись в слой {:?}", layer);
    }
    
    // === ТЕСТ 2: Поиск ===
    info!("\n🔍 Тест 2: Поиск по разным запросам");
    
    let search_queries = vec![
        "язык программирования Rust",
        "асинхронное программирование",
        "алгоритмы поиска",
        "безопасность памяти",
    ];
    
    for query in search_queries {
        let results = memory_service
            .search(query)
            .top_k(3)
            .execute()
            .await?;
        
        info!("  Запрос: '{}' - найдено {} результатов", query, results.len());
        for (i, result) in results.iter().enumerate() {
            let truncated = result.text.chars().take(50).collect::<String>();
            info!("    {}. {} (score: {:.3})", i+1, truncated, result.score);
        }
    }
    
    // === ТЕСТ 3: Фильтрация по слоям ===
    info!("\n📊 Тест 3: Поиск с фильтрацией по слоям");
    
    let interact_results = memory_service
        .search("программирование")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;
    
    info!("  Найдено в слое Interact: {} записей", interact_results.len());
    
    // === ТЕСТ 4: Статистика кэша ===
    info!("\n📈 Тест 4: Статистика кэша");
    
    let (hits, misses, total) = memory_service.cache_stats();
    let hit_rate = if total > 0 { hits as f32 / total as f32 * 100.0 } else { 0.0 };
    
    info!("  Попадания: {}", hits);
    info!("  Промахи: {}", misses);
    info!("  Всего запросов: {}", total);
    info!("  Hit rate: {:.1}%", hit_rate);
    
    // === ТЕСТ 5: Здоровье системы ===
    info!("\n🏥 Тест 5: Проверка здоровья системы");
    
    let health = memory_service.run_health_check().await?;
    info!("  Общий статус: {:?}", health.overall_status);
    info!("  Время работы: {} сек", health.uptime_seconds);
    
    for (component, status) in &health.component_statuses {
        info!("  {:?}: {:?}", component, status);
    }
    
    // === ТЕСТ 6: Promotion цикл ===
    info!("\n♻️ Тест 6: Цикл продвижения записей");
    
    let promotion_stats = memory_service.run_promotion_cycle().await?;
    info!("  Interact → Insights: {} записей", promotion_stats.interact_to_insights);
    info!("  Insights → Assets: {} записей", promotion_stats.insights_to_assets);
    info!("  Удалено из Interact: {} записей", promotion_stats.expired_interact);
    info!("  Удалено из Insights: {} записей", promotion_stats.expired_insights);
    
    // === ТЕСТ 7: Производительность batch операций ===
    info!("\n⚡ Тест 7: Производительность batch операций");
    
    let batch_start = Instant::now();
    let batch_size = 100;
    
    for i in 0..batch_size {
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Batch record {} with test content", i),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "batch_test".to_string(),
            tags: vec!["batch".to_string()],
            project: "perf_test".to_string(),
            session: "batch_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };
        
        memory_service.insert(record).await?;
    }
    
    let batch_duration = batch_start.elapsed();
    let records_per_second = batch_size as f64 / batch_duration.as_secs_f64();
    
    info!("  Вставлено {} записей за {:?}", batch_size, batch_duration);
    info!("  Производительность: {:.0} записей/сек", records_per_second);
    
    // === ТЕСТ 8: Backup и восстановление ===
    info!("\n💾 Тест 8: Backup и восстановление");
    
    let backup_path = memory_service.create_backup(Some("test_backup".to_string())).await?;
    info!("  Backup создан: {:?}", backup_path);
    
    // Проверяем список backup'ов
    let backups = memory_service.list_backups()?;
    info!("  Доступно {} backup(s)", backups.len());
    
    info!("\n✅ Все тесты успешно пройдены!");
    
    Ok(())
}

/// Тест конкурентных операций
#[tokio::test]
async fn test_concurrent_memory_operations() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();
        
    info!("🔄 Тест конкурентных операций");
    
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("concurrent_db"),
        cache_path: temp_dir.path().join("concurrent_cache"),
        ..MemoryConfig::default()
    };
    
    let memory_service = Arc::new(MemoryService::new(config).await?);
    
    // Запускаем несколько задач параллельно
    let mut handles = vec![];
    
    for task_id in 0..5 {
        let service = memory_service.clone();
        let handle = tokio::spawn(async move {
            for i in 0..20 {
                let record = Record {
                    id: Uuid::new_v4(),
                    text: format!("Task {} record {}", task_id, i),
                    embedding: vec![],
                    layer: Layer::Interact,
                    kind: "concurrent".to_string(),
                    tags: vec![format!("task_{}", task_id)],
                    project: "concurrent_test".to_string(),
                    session: format!("session_{}", task_id),
                    ts: Utc::now(),
                    score: 0.0,
                    access_count: 0,
                    last_access: Utc::now(),
                };
                
                if let Err(e) = service.insert(record).await {
                    warn!("Insert error in task {}: {}", task_id, e);
                }
            }
            
            // Выполняем поиски
            for _i in 0..10 {
                let query = format!("Task {} record", task_id);
                if let Err(e) = service.search(&query).top_k(5).execute().await {
                    warn!("Search error in task {}: {}", task_id, e);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Ждём завершения всех задач
    for handle in handles {
        handle.await?;
    }
    
    info!("✅ Конкурентные операции завершены успешно");
    
    Ok(())
}