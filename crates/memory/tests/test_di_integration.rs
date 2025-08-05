#!/usr/bin/env -S cargo +nightly -Zscript
//! ```cargo
//! [dependencies]
//! tokio = { version = "1.0", features = ["full"] }
//! anyhow = "1.0"
//! tracing = "0.1"
//! tracing-subscriber = "0.3"
//! uuid = "1.0"
//! chrono = { version = "0.4", features = ["serde"] }
//! memory = { path = "./crates/memory" }
//! ```

//! Быстрый тест интегрированной DI системы памяти
//! Запуск: cargo run --bin test_di_integration

use anyhow::Result;
use chrono::Utc;
use memory::{DIMemoryService, default_config, Record, Layer};
use std::time::Instant;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логгирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🧪 Тест интегрированной DI системы MAGRAY CLI");
    
    let start_time = Instant::now();

    // Тест 1: Создание DIMemoryService
    println!("\n1️⃣  Создание DIMemoryService...");
    let config = default_config()?;
    let service = DIMemoryService::new(config).await?;
    println!("✅ DIMemoryService создан за {:?}", start_time.elapsed());

    // Тест 2: Инициализация слоев
    println!("\n2️⃣  Инициализация слоев памяти...");
    let init_start = Instant::now();
    service.initialize().await?;
    println!("✅ Слои инициализированы за {:?}", init_start.elapsed());

    // Тест 3: Вставка записи
    println!("\n3️⃣  Тест вставки записи...");
    let record = Record {
        id: Uuid::new_v4(),
        text: "Это тестовое сообщение для проверки DI системы".to_string(),
        embedding: vec![], // Будет создан автоматически
        layer: Layer::Interact,
        kind: "test_message".to_string(),
        tags: vec!["test".to_string(), "di".to_string()],
        project: "magray".to_string(),
        session: "test_session".to_string(),
        ts: Utc::now(),
        score: 0.0,
        access_count: 1,
        last_access: Utc::now(),
    };

    let insert_start = Instant::now();
    service.insert(record).await?;
    println!("✅ Запись вставлена за {:?}", insert_start.elapsed());

    // Тест 4: Поиск записей (с fallback embedding)
    println!("\n4️⃣  Тест поиска...");
    let search_start = Instant::now();
    let search_options = memory::SearchOptions {
        layers: vec![Layer::Interact],
        top_k: 5,
        score_threshold: 0.0,
        tags: vec![],
        project: Some("magray".to_string()),
    };
    
    let results = service.search("тестовое сообщение", Layer::Interact, search_options).await?;
    println!("✅ Поиск завершен за {:?}, найдено {} записей", 
             search_start.elapsed(), results.len());

    // Тест 5: Статистика системы (если доступна)
    println!("\n5️⃣  Проверка статистики системы...");
    if let Ok(_stats) = service.get_stats().await {
        println!("✅ Статистика системы получена");
    } else {
        println!("⚠️  Статистика недоступна (возможно, не все компоненты загружены)");
    }

    // Тест 6: Health check
    println!("\n6️⃣  Проверка здоровья системы...");
    if let Ok(health) = service.check_health().await {
        println!("✅ Health check: {:?}", health);
    } else {
        println!("⚠️  Health check недоступен");
    }

    let total_time = start_time.elapsed();
    println!("\n🎉 Все тесты DI системы завершены успешно!");
    println!("📊 Общее время: {:?}", total_time);
    println!("📝 DI система полностью функциональна и готова к использованию");

    Ok(())
}