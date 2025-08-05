use anyhow::Result;
use memory::{DIMemoryService, Layer, Record, default_config};
use uuid::Uuid;
use chrono::Utc;

/// Простой пример использования DIMemoryService

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== Simple DI Memory Service Demo ===\n");

    // Configure memory service
    let mut config = default_config()?;
    config.db_path = std::path::PathBuf::from("./demo_db");
    config.cache_path = std::path::PathBuf::from("./demo_cache");

    println!("1. Initializing DI Memory Service...");
    let service = DIMemoryService::new(config).await?;
    println!("   ✅ Service created");
    
    // Инициализация слоев памяти
    println!("\n2. Initializing memory layers...");
    service.initialize().await?;
    println!("   ✅ Layers initialized");

    // Insert a sample record
    println!("\n3. Inserting sample record...");
    let record = Record {
        id: Uuid::new_v4(),
        text: "Test memory record for DIMemoryService demo".to_string(),
        embedding: vec![], // Будет создан автоматически
        layer: Layer::Interact,
        kind: "demo".to_string(),
        tags: vec!["test".to_string()],
        project: "demo".to_string(),
        session: "demo-session".to_string(),
        ts: Utc::now(),
        score: 0.5,
        access_count: 1,
        last_access: Utc::now(),
    };

    service.insert(record).await?;
    println!("   ✅ Record inserted");

    // Get statistics
    println!("\n4. Getting system statistics...");
    let stats = service.get_stats().await;
    
    println!("   📊 DI Container stats:");
    println!("      • Registered types: {}", stats.di_container_stats.total_types);
    println!("      • Cached singletons: {}", stats.di_container_stats.cached_singletons);
    println!("      • Registered factories: {}", stats.di_container_stats.registered_factories);
    
    println!("\n   💾 Cache stats:");
    println!("      • Cache hits: {}", stats.cache_hits);
    println!("      • Cache misses: {}", stats.cache_misses);
    
    // Get performance metrics
    println!("\n5. Getting performance metrics...");
    let perf_report = service.get_performance_report();
    println!("{}", perf_report);

    // Run promotion cycle
    println!("\n6. Running promotion cycle...");
    let promotion_stats = service.run_promotion().await?;
    println!("   ✅ Promotion completed in {}ms", promotion_stats.total_time_ms);

    println!("\n=== Demo completed successfully! ===");
    Ok(())
}