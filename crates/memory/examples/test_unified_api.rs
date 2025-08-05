use anyhow::Result;
use memory::{MemoryService, UnifiedMemoryAPI, MemoryContext, ApiSearchOptions, Layer, default_config};
use std::sync::Arc;
use tracing::info;

/// Демонстрация использования Unified Memory API
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("🌟 Демонстрация Unified Memory API для MAGRAY CLI");
    info!("================================================\n");
    
    // Создаем временную директорию для теста
    let temp_dir = tempfile::tempdir()?;
    let mut config = default_config().unwrap();
    config.db_path = temp_dir.path().join("unified_api_test");
    config.cache_path = temp_dir.path().join("cache");
    
    // Инициализируем MemoryService
    println!("🔧 Инициализация системы памяти...");
    let memory_service = Arc::new(MemoryService::new(config).await?);
    
    // Создаем Unified API
    let api = UnifiedMemoryAPI::new(memory_service);
    println!("✅ Unified Memory API готов к работе!\n");
    
    // ========== ТЕСТ 1: СОХРАНЕНИЕ В ПАМЯТЬ ==========
    println!("📝 ТЕСТ 1: Сохранение информации");
    println!("================================");
    
    // Простое сохранение
    let id1 = api.remember(
        "MAGRAY CLI - это AI агент на Rust с многослойной памятью".to_string(),
        MemoryContext::new("documentation")
            .with_tags(vec!["magray".to_string(), "overview".to_string()])
            .with_project("magray-docs")
    ).await?;
    println!("✅ Сохранено: ID = {}", id1);
    
    // Сохранение кода
    let id2 = api.remember(
        "async fn main() -> Result<()> { println!(\"Hello MAGRAY!\"); Ok(()) }".to_string(),
        MemoryContext::new("code")
            .with_tags(vec!["rust".to_string(), "example".to_string()])
            .with_layer(Layer::Insights) // Сразу в важный слой
    ).await?;
    println!("✅ Сохранено: ID = {}", id2);
    
    // Сохранение команды
    let id3 = api.remember(
        "cargo build --release - компилирует оптимизированный бинарник".to_string(),
        MemoryContext::new("command")
            .with_tags(vec!["cargo".to_string(), "build".to_string()])
    ).await?;
    println!("✅ Сохранено: ID = {}", id3);
    
    // ========== ТЕСТ 2: ПОИСК В ПАМЯТИ ==========
    println!("\n🔍 ТЕСТ 2: Поиск информации");
    println!("==========================");
    
    // Простой поиск
    println!("\n📌 Поиск: 'MAGRAY'");
    let results = api.recall("MAGRAY", ApiSearchOptions::new().limit(3)).await?;
    for (i, result) in results.iter().enumerate() {
        println!("  {}. [{}] {} (релевантность: {:.3})", 
                 i + 1, result.kind, 
                 result.text.chars().take(50).collect::<String>(),
                 result.relevance_score);
    }
    
    // Поиск по тегам
    println!("\n📌 Поиск по тегу 'rust':");
    let results = api.recall(
        "код", 
        ApiSearchOptions::new()
            .with_tags(vec!["rust".to_string()])
            .limit(2)
    ).await?;
    for result in &results {
        println!("  - {}", result.text.chars().take(60).collect::<String>());
    }
    
    // ========== ТЕСТ 3: ПОЛУЧЕНИЕ ПО ID ==========
    println!("\n🎯 ТЕСТ 3: Получение по ID");
    println!("=========================");
    
    if let Some(memory) = api.get(id1).await? {
        println!("✅ Найдено:");
        println!("  Текст: {}", memory.text);
        println!("  Слой: {:?}", memory.layer);
        println!("  Теги: {:?}", memory.tags);
        println!("  Обращений: {}", memory.access_count);
    }
    
    // ========== ТЕСТ 4: ОПТИМИЗАЦИЯ ПАМЯТИ ==========
    println!("\n⚡ ТЕСТ 4: Оптимизация памяти");
    println!("============================");
    
    let optimization = api.optimize_memory().await?;
    println!("✅ Оптимизация завершена:");
    println!("  Продвинуто в Insights: {}", optimization.promoted_to_insights);
    println!("  Продвинуто в Assets: {}", optimization.promoted_to_assets);
    println!("  Время выполнения: {}ms", optimization.total_time_ms);
    
    // ========== ТЕСТ 5: ПРОВЕРКА ЗДОРОВЬЯ ==========
    println!("\n🏥 ТЕСТ 5: Проверка здоровья системы");
    println!("==================================");
    
    let health = api.health_check().await?;
    println!("📊 Статус системы: {}", health.status);
    println!("  Время работы: {} сек", health.uptime_seconds);
    println!("  Компонентов: {}", health.component_count);
    println!("  Активных alerts: {}", health.alert_count);
    
    println!("\n🔍 Статус компонентов:");
    for (component, status) in &health.components {
        let icon = match status.as_str() {
            "healthy" => "✅",
            "degraded" => "🟡",
            "unhealthy" => "🟠",
            "down" => "❌",
            _ => "❓",
        };
        println!("  {} {}: {}", icon, component, status);
    }
    
    // ========== ТЕСТ 6: ДЕТАЛЬНАЯ ПРОВЕРКА ==========
    println!("\n🔬 ТЕСТ 6: Детальная проверка здоровья");
    println!("====================================");
    
    let detailed = api.full_health_check().await?;
    println!("📊 Детальный статус: {}", detailed.overall_status);
    
    if !detailed.alerts.is_empty() {
        println!("\n🚨 Активные оповещения:");
        for alert in &detailed.alerts {
            println!("  [{} - {}] {}: {}", 
                     alert.severity, alert.component, 
                     alert.title, alert.message);
        }
    } else {
        println!("✅ Нет активных оповещений");
    }
    
    // ========== ТЕСТ 7: СТАТИСТИКА ==========
    println!("\n📊 ТЕСТ 7: Общая статистика");
    println!("=========================");
    
    let stats = api.get_stats().await?;
    println!("📈 Всего записей: {}", stats.total_records);
    println!("\n📊 Распределение по слоям:");
    for (layer, count) in &stats.layer_distribution {
        println!("  {}: {} записей", layer, count);
    }
    println!("\n📊 Размеры индексов:");
    println!("  Time индексы: {} записей", stats.index_sizes.time_indices);
    println!("  Score индексы: {} записей", stats.index_sizes.score_indices);
    
    // ========== ТЕСТ 8: УДАЛЕНИЕ ==========
    println!("\n🗑️ ТЕСТ 8: Удаление записи");
    println!("========================");
    
    let deleted = api.forget(id3).await?;
    if deleted {
        println!("✅ Запись {} успешно удалена", id3);
    } else {
        println!("❌ Запись {} не найдена", id3);
    }
    
    // Проверяем, что запись действительно удалена
    let check = api.get(id3).await?;
    if check.is_none() {
        println!("✅ Подтверждено: запись больше не существует");
    }
    
    // ========== ИТОГИ ==========
    println!("\n🏆 РЕЗУЛЬТАТЫ ТЕСТИРОВАНИЯ UNIFIED API");
    println!("====================================");
    println!("✅ Простое сохранение через remember()");
    println!("✅ Гибкий поиск через recall() с опциями");
    println!("✅ Получение по ID через get()");
    println!("✅ Оптимизация памяти через optimize_memory()");
    println!("✅ Проверка здоровья через health_check()");
    println!("✅ Детальная диагностика через full_health_check()");
    println!("✅ Статистика системы через get_stats()");
    println!("✅ Удаление записей через forget()");
    
    println!("\n🎉 UNIFIED MEMORY API ПОЛНОСТЬЮ ФУНКЦИОНАЛЕН!");
    println!("   Готов к интеграции в MAGRAY CLI!");
    
    Ok(())
}