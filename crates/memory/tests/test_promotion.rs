use anyhow::Result;
use chrono::{Duration, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use memory::{
    MemoryService, MemoryConfig, Layer, Record, PromotionConfig,
};
use ai::AiConfig;

#[tokio::test]
async fn test_promotion_engine() -> Result<()> {
    // Create temporary directories
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let cache_path = temp_dir.path().join("test_cache");
    
    // Configure with short TTLs for testing
    let config = MemoryConfig {
        db_path: db_path.clone(),
        cache_path: cache_path.clone(),
        promotion: PromotionConfig {
            interact_ttl_hours: 1,      // 1 hour for testing
            insights_ttl_days: 1,       // 1 day for testing
            promote_threshold: 0.5,     // Lower threshold for testing
            decay_factor: 0.9,
        },
        ai_config: AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfigType::Lru(memory::CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
    };
    
    // Initialize memory service
    let memory_service = MemoryService::new(config).await?;
    
    // Insert test records with different ages and access patterns
    let now = Utc::now();
    
    // Old, frequently accessed record (should be promoted)
    let old_popular = Record {
        id: Uuid::new_v4(),
        text: "Popular old content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["popular".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.1; 768], // Mock embedding
        ts: now - Duration::hours(2), // 2 hours old
        last_access: now - Duration::minutes(10),
        access_count: 10,
        score: 0.8,
    };
    
    // Old, rarely accessed record (should expire)
    let old_unpopular = Record {
        id: Uuid::new_v4(),
        text: "Unpopular old content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["unpopular".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.2; 768],
        ts: now - Duration::hours(3), // 3 hours old
        last_access: now - Duration::hours(3),
        access_count: 1,
        score: 0.3,
    };
    
    // New record (should stay in Interact)
    let new_record = Record {
        id: Uuid::new_v4(),
        text: "Fresh content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["new".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.3; 768],
        ts: now - Duration::minutes(30), // 30 minutes old
        last_access: now,
        access_count: 5,
        score: 0.9,
    };
    
    // Insert all records
    memory_service.insert(old_popular.clone()).await?;
    memory_service.insert(old_unpopular.clone()).await?;
    memory_service.insert(new_record.clone()).await?;
    
    println!("✅ Inserted 3 test records");
    
    // Run promotion cycle
    let stats = memory_service.run_promotion_cycle().await?;
    
    println!("\n📊 Promotion Stats:");
    println!("  Interact → Insights: {}", stats.interact_to_insights);
    println!("  Insights → Assets: {}", stats.insights_to_assets);
    println!("  Expired Interact: {}", stats.expired_interact);
    println!("  Expired Insights: {}", stats.expired_insights);
    
    // Verify old popular record was promoted to Insights
    let promoted = memory_service.get_by_id(&old_popular.id, Layer::Insights).await?;
    assert!(promoted.is_some(), "Popular record should be promoted to Insights");
    
    // Verify it was removed from Interact
    let in_interact = memory_service.get_by_id(&old_popular.id, Layer::Interact).await?;
    assert!(in_interact.is_none(), "Promoted record should be removed from Interact");
    
    // Verify new record stays in Interact
    let still_new = memory_service.get_by_id(&new_record.id, Layer::Interact).await?;
    assert!(still_new.is_some(), "New record should remain in Interact");
    
    // Search in Insights layer
    let insights_results = memory_service.search("popular")
        .with_layer(Layer::Insights)
        .execute()
        .await?;
    
    assert_eq!(insights_results.len(), 1, "Should find promoted record in Insights");
    assert_eq!(insights_results[0].id, old_popular.id);
    
    println!("\n✅ All promotion tests passed!");
    
    Ok(())
}

#[tokio::test]
async fn test_layer_ttl_expiration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let cache_path = temp_dir.path().join("test_cache");
    
    let config = MemoryConfig {
        db_path,
        cache_path,
        promotion: PromotionConfig {
            interact_ttl_hours: 1,
            insights_ttl_days: 1,
            promote_threshold: 0.7,
            decay_factor: 0.9,
        },
        ai_config: AiConfig::default(),
        health_config: memory::HealthConfig::default(),
        cache_config: memory::CacheConfigType::Lru(memory::CacheConfig::default()),
        resource_config: memory::ResourceConfig::default(),
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
    };
    
    let memory_service = MemoryService::new(config).await?;
    let now = Utc::now();
    
    // Insert very old record that should be expired
    let ancient_record = Record {
        id: Uuid::new_v4(),
        text: "Ancient content".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["ancient".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.4; 768],
        ts: now - Duration::hours(10), // 10 hours old (way past TTL)
        last_access: now - Duration::hours(10),
        access_count: 0,
        score: 0.1,
    };
    
    memory_service.insert(ancient_record.clone()).await?;
    
    // Run promotion cycle
    let stats = memory_service.run_promotion_cycle().await?;
    
    // Should have expired the ancient record
    assert!(stats.expired_interact > 0, "Should expire old records");
    
    // Verify it's gone
    let gone = memory_service.get_by_id(&ancient_record.id, Layer::Interact).await?;
    assert!(gone.is_none(), "Ancient record should be expired");
    
    println!("✅ TTL expiration test passed!");
    
    Ok(())
}