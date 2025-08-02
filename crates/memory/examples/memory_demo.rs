use anyhow::Result;
use memory::{Layer, MemoryService, Record, default_config};
use std::path::PathBuf;

// No need for mock embedding function - AI service handles it now

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure memory service
    let mut config = default_config().unwrap();
    config.db_path = PathBuf::from("./demo_db");
    config.cache_path = PathBuf::from("./demo_cache");

    println!("Initializing memory service...");
    let service = MemoryService::new(config).await?;

    // Insert some sample records
    println!("\nInserting sample memories...");
    
    let records = vec![
        Record {
            text: "User solved authentication issue by resetting OAuth tokens".to_string(),
            layer: Layer::Interact,
            kind: "decision".to_string(),
            tags: vec!["auth".to_string(), "solution".to_string()],
            project: "auth-system".to_string(),
            session: "session-123".to_string(),
            score: 0.9,
            ..Default::default()
        },
        Record {
            text: "Database query optimization: use indexes on user_id and timestamp".to_string(),
            layer: Layer::Insights,
            kind: "optimization".to_string(),
            tags: vec!["database".to_string(), "performance".to_string()],
            project: "backend".to_string(),
            session: "session-456".to_string(),
            score: 0.8,
            ..Default::default()
        },
        Record {
            text: "Architecture decision: use event-driven microservices pattern".to_string(),
            layer: Layer::Assets,
            kind: "architecture".to_string(),
            tags: vec!["design".to_string(), "microservices".to_string()],
            project: "platform".to_string(),
            session: "session-789".to_string(),
            score: 0.95,
            ..Default::default()
        },
    ];

    service.insert_batch(records).await?;
    println!("Inserted {} records", 3);

    // Demonstrate different search patterns
    println!("\n=== Search Examples ===");

    // 1. Basic search
    println!("\n1. Basic search for 'authentication':");
    let results = service
        .search("authentication OAuth")
        .top_k(5)
        .execute()
        .await?;
    
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
    let results = service
        .search("optimization performance")
        .with_layer(Layer::Insights)
        .execute()
        .await?;
    
    for record in &results {
        println!("  - {}", &record.text[..60.min(record.text.len())]);
    }

    // 3. Tag-filtered search
    println!("\n3. Search with tag filter:");
    let results = service
        .search("system design")
        .with_tags(vec!["design".to_string()])
        .execute()
        .await?;
    
    for record in &results {
        println!("  - {} (tags: {:?})", 
            &record.text[..50.min(record.text.len())], 
            record.tags
        );
    }

    // 4. Project-scoped search
    println!("\n4. Search within specific project:");
    let results = service
        .search("backend optimization")
        .in_project("backend".to_string())
        .execute()
        .await?;
    
    for record in &results {
        println!("  - [{}] {}", record.project, &record.text[..50.min(record.text.len())]);
    }

    // 5. High-quality results only
    println!("\n5. Search with minimum score threshold:");
    let results = service
        .search("important decisions")
        .min_score(0.85)
        .execute()
        .await?;
    
    println!("  Found {} high-quality matches", results.len());

    // Show cache statistics
    println!("\n=== Cache Statistics ===");
    let (hits, misses, inserts) = service.cache_stats();
    println!("  Cache hits: {}", hits);
    println!("  Cache misses: {}", misses);
    println!("  Cache inserts: {}", inserts);
    println!("  Hit rate: {:.2}%", service.cache_hit_rate() * 100.0);

    // Demonstrate promotion cycle
    println!("\n=== Running Promotion Cycle ===");
    let stats = service.run_promotion_cycle().await?;
    println!("  Promoted from Interact to Insights: {}", stats.interact_to_insights);
    println!("  Promoted from Insights to Assets: {}", stats.insights_to_assets);
    println!("  Expired from Interact: {}", stats.expired_interact);
    println!("  Expired from Insights: {}", stats.expired_insights);

    println!("\nDemo completed!");
    Ok(())
}