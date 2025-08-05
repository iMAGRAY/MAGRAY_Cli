use anyhow::Result;
use memory::{Layer, DIMemoryService, Record, default_config, SearchOptions};
use std::path::PathBuf;
use uuid::Uuid;
use chrono::Utc;

// Демонстрация использования DIMemoryService

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure memory service
    let mut config = default_config()?;
    config.db_path = PathBuf::from("./demo_db");
    config.cache_path = PathBuf::from("./demo_cache");

    println!("Initializing DI Memory Service...");
    let service = DIMemoryService::new(config).await?;
    
    // Инициализация слоев памяти
    service.initialize().await?;

    // Insert some sample records
    println!("\nInserting sample memories...");
    
    let records = vec![
        Record {
            id: Uuid::new_v4(),
            text: "User solved authentication issue by resetting OAuth tokens".to_string(),
            embedding: vec![], // Будет создан автоматически
            layer: Layer::Interact,
            kind: "decision".to_string(),
            tags: vec!["auth".to_string(), "solution".to_string()],
            project: "auth-system".to_string(),
            session: "session-123".to_string(),
            ts: Utc::now(),
            score: 0.9,
            access_count: 1,
            last_access: Utc::now(),
        },
        Record {
            id: Uuid::new_v4(),
            text: "Database query optimization: use indexes on user_id and timestamp".to_string(),
            embedding: vec![],
            layer: Layer::Insights,
            kind: "optimization".to_string(),
            tags: vec!["database".to_string(), "performance".to_string()],
            project: "backend".to_string(),
            session: "session-456".to_string(),
            ts: Utc::now(),
            score: 0.8,
            access_count: 1,
            last_access: Utc::now(),
        },
        Record {
            id: Uuid::new_v4(),
            text: "Architecture decision: use event-driven microservices pattern".to_string(),
            embedding: vec![],
            layer: Layer::Assets,
            kind: "architecture".to_string(),
            tags: vec!["design".to_string(), "microservices".to_string()],
            project: "platform".to_string(),
            session: "session-789".to_string(),
            ts: Utc::now(),
            score: 0.95,
            access_count: 1,
            last_access: Utc::now(),
        },
    ];

    // Вставляем записи по одной (DIMemoryService не имеет batch метода)
    for record in records {
        service.insert(record).await?;
    }
    println!("Inserted 3 records");

    // Demonstrate different search patterns
    println!("\n=== Search Examples ===");

    // 1. Basic search
    println!("\n1. Basic search for 'authentication':");
    let search_options = SearchOptions {
        top_k: 5,
        ..Default::default()
    };
    let results = service.search(
        "authentication OAuth",
        Layer::Interact,
        search_options
    ).await?;
    
    for (i, record) in results.iter().enumerate() {
        println!("  {}. [{}] {} (score: {:.2})", 
            i + 1, 
            record.layer.as_str(), 
            &record.text[..50.min(record.text.len())],
            record.score
        );
    }

    // 2. Layer-specific search
    println!("\n2. Search only in Insights layer:");
    let results = service.search(
        "optimization performance",
        Layer::Insights,
        SearchOptions::default()
    ).await?;
    
    for record in &results {
        println!("  - {}", &record.text[..60.min(record.text.len())]);
    }

    // 3. Tag-filtered search
    println!("\n3. Search with tag filter:");
    let search_options = SearchOptions {
        tags: vec!["design".to_string()],
        ..Default::default()
    };
    let results = service.search(
        "system design",
        Layer::Assets,
        search_options
    ).await?;
    
    for record in &results {
        println!("  - {} (tags: {:?})", 
            &record.text[..50.min(record.text.len())], 
            record.tags
        );
    }

    // 4. Project-scoped search
    println!("\n4. Search within specific project:");
    let search_options = SearchOptions {
        project: Some("backend".to_string()),
        ..Default::default()
    };
    let results = service.search(
        "backend optimization",
        Layer::Insights,
        search_options
    ).await?;
    
    for record in &results {
        println!("  - [{}] {}", record.project, &record.text[..50.min(record.text.len())]);
    }

    // 5. High-quality results only
    println!("\n5. Search with minimum score threshold:");
    let search_options = SearchOptions {
        score_threshold: 0.85,
        ..Default::default()
    };
    let results = service.search(
        "important decisions",
        Layer::Assets,
        search_options
    ).await?;
    
    println!("  Found {} high-quality matches", results.len());

    // Show statistics
    println!("\n=== DI System Statistics ===");
    let stats = service.get_stats().await;
    println!("  Total records: {}", stats.batch_stats.total_records);
    println!("  Cache hits: {}", stats.cache_hits);
    println!("  Cache misses: {}", stats.cache_misses);
    let hit_rate = if stats.cache_hits + stats.cache_misses > 0 {
        stats.cache_hits as f64 / (stats.cache_hits + stats.cache_misses) as f64
    } else {
        0.0
    };
    println!("  Cache hit rate: {:.2}%", hit_rate * 100.0);
    println!("  DI Container types: {}", stats.di_container_stats.total_types);
    println!("  Cached singletons: {}", stats.di_container_stats.cached_singletons);

    // Demonstrate promotion cycle
    println!("\n=== Running Promotion Cycle ===");
    let promotion_stats = service.run_promotion().await?;
    println!("  Promoted from Interact to Insights: {}", promotion_stats.interact_to_insights);
    println!("  Promoted from Insights to Assets: {}", promotion_stats.insights_to_assets);
    println!("  Expired from Interact: {}", promotion_stats.expired_interact);
    println!("  Expired from Insights: {}", promotion_stats.expired_insights);
    println!("  Total time: {}ms", promotion_stats.total_time_ms);
    
    // Show performance metrics
    println!("\n=== DI Performance Metrics ===");
    let perf_report = service.get_performance_report();
    println!("{}", perf_report);
    
    // Показываем статистику по слоям
    if let Ok(health) = &stats.health_status {
        println!("\n  Health status: {:?}", health.overall_status);
        println!("  Uptime: {} seconds", health.uptime_seconds);
    }

    println!("\nDemo completed!");
    Ok(())
}