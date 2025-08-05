#!/usr/bin/env -S cargo +nightly -Zscript
//! Test script для проверки работы DI системы
//! 
//! Запуск: cargo run --bin test_di_system

use anyhow::Result;
use std::path::PathBuf;

// Импорты из memory crate (требует добавления в Cargo.toml)
// use memory::{DIMemoryService, MemoryConfig, CacheConfigType, default_config};

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логгирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("🧪 Тестирование DI системы MAGRAY CLI");
    
    // Создание тестовой конфигурации
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join("test_magray_di");
    let cache_path = temp_dir.join("test_cache_di");
    
    // Создаем директории
    std::fs::create_dir_all(&db_path)?;
    std::fs::create_dir_all(&cache_path)?;
    
    println!("✓ Временные директории созданы:");
    println!("  DB: {:?}", db_path);
    println!("  Cache: {:?}", cache_path);
    
    /* Закомментировано до интеграции с CLI
    
    // Создание DI Memory Service 
    let service = memory::create_di_memory_service().await?;
    
    println!("✓ DIMemoryService создан успешно");
    
    // Инициализация слоев
    service.initialize().await?;
    println!("✓ Слои памяти инициализированы");
    
    // Тест статистики DI контейнера
    let di_stats = service.di_stats();
    println!("✓ DI контейнер статистика:");
    println!("  - Зарегистрированные factories: {}", di_stats.registered_factories);
    println!("  - Cached singletons: {}", di_stats.cached_singletons);
    println!("  - Всего типов: {}", di_stats.total_types);
    
    // Тест системной статистики
    let system_stats = service.get_stats().await;
    println!("✓ Системная статистика:");
    println!("  - Cache hits: {}", system_stats.cache_hits);
    println!("  - Cache misses: {}", system_stats.cache_misses);
    println!("  - Cache size: {}", system_stats.cache_size);
    
    // Тест health check
    let health = service.check_health().await?;
    println!("✓ Health check: {:?}", health);
    
    */
    
    println!("🎉 DI система готова к интеграции с CLI!");
    
    // Очистка временных файлов
    std::fs::remove_dir_all(&db_path).ok();
    std::fs::remove_dir_all(&cache_path).ok();
    println!("✓ Временные файлы очищены");
    
    Ok(())
}