use anyhow::Result;
use chrono::{Duration, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use memory::{
    Layer, Record, PromotionConfig, PromotionEngine, VectorStore,
};
use std::sync::Arc;

async fn create_test_store() -> Result<(Arc<VectorStore>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let store = Arc::new(VectorStore::new(&db_path).await?);
    
    // Initialize layers
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        store.init_layer(layer).await?;
    }
    
    Ok((store, temp_dir))
}

#[tokio::test]
async fn test_promotion_logic() -> Result<()> {
    let (store, _temp_dir) = create_test_store().await?;
    
    let config = PromotionConfig {
        interact_ttl_hours: 1,
        insights_ttl_days: 1,
        promote_threshold: 0.5,
        decay_factor: 0.9,
    };
    
    let promotion_engine = PromotionEngine::new(store.clone(), config);
    let now = Utc::now();
    
    // Create test records
    let records = vec![
        // Old, high-score, frequently accessed - should be promoted
        Record {
            id: Uuid::new_v4(),
            text: "Popular content".to_string(),
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec!["popular".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.1; 768],
            ts: now - Duration::hours(2),
            last_access: now - Duration::minutes(10),
            access_count: 10,
            score: 0.8,
        },
        // Old, low-score - should expire
        Record {
            id: Uuid::new_v4(),
            text: "Unpopular content".to_string(),
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec!["unpopular".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.2; 768],
            ts: now - Duration::hours(3),
            last_access: now - Duration::hours(3),
            access_count: 1,
            score: 0.3,
        },
        // New record - should stay
        Record {
            id: Uuid::new_v4(),
            text: "Fresh content".to_string(),
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec!["new".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.3; 768],
            ts: now - Duration::minutes(30),
            last_access: now,
            access_count: 5,
            score: 0.9,
        },
    ];
    
    // Insert records directly into store
    let record_refs: Vec<&Record> = records.iter().collect();
    store.insert_batch(&record_refs).await?;
    
    println!("✅ Inserted {} test records", records.len());
    
    // Run promotion cycle
    let stats = promotion_engine.run_promotion_cycle().await?;
    
    println!("\n📊 Promotion Stats:");
    println!("  Interact → Insights: {}", stats.interact_to_insights);
    println!("  Insights → Assets: {}", stats.insights_to_assets);
    println!("  Expired Interact: {}", stats.expired_interact);
    println!("  Expired Insights: {}", stats.expired_insights);
    
    // Verify promotion happened
    assert!(stats.interact_to_insights > 0, "Should promote some records");
    
    // Check specific record was promoted
    let promoted_id = &records[0].id;
    let in_insights = store.get_by_id(promoted_id, Layer::Insights).await?;
    assert!(in_insights.is_some(), "Popular record should be in Insights");
    
    let not_in_interact = store.get_by_id(promoted_id, Layer::Interact).await?;
    assert!(not_in_interact.is_none(), "Promoted record should not be in Interact");
    
    // Check new record stayed
    let new_id = &records[2].id;
    let still_interact = store.get_by_id(new_id, Layer::Interact).await?;
    assert!(still_interact.is_some(), "New record should remain in Interact");
    
    println!("\n✅ Promotion logic test passed!");
    
    Ok(())
}

#[tokio::test]
async fn test_promotion_scoring() -> Result<()> {
    let now = Utc::now();
    
    // Test record with high access and recent activity
    let active_record = Record {
        id: Uuid::new_v4(),
        text: "Active".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![],
        ts: now - Duration::hours(1),
        last_access: now - Duration::minutes(5),
        access_count: 20,
        score: 0.7,
    };
    
    // Test record with low access and old activity
    let inactive_record = Record {
        id: Uuid::new_v4(),
        text: "Inactive".to_string(),
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec![],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![],
        ts: now - Duration::days(7),
        last_access: now - Duration::days(7),
        access_count: 2,
        score: 0.7,
    };
    
    let active_score = PromotionEngine::calculate_promotion_score(&active_record);
    let inactive_score = PromotionEngine::calculate_promotion_score(&inactive_record);
    
    println!("Active record score: {:.3}", active_score);
    println!("Inactive record score: {:.3}", inactive_score);
    
    assert!(active_score > inactive_score, "Active record should have higher promotion score");
    
    println!("✅ Promotion scoring test passed!");
    
    Ok(())
}

#[tokio::test]
async fn test_layer_to_layer_promotion() -> Result<()> {
    let (store, _temp_dir) = create_test_store().await?;
    
    let config = PromotionConfig {
        interact_ttl_hours: 1,
        insights_ttl_days: 1,
        promote_threshold: 0.5,
        decay_factor: 0.8,
    };
    
    let promotion_engine = PromotionEngine::new(store.clone(), config);
    let now = Utc::now();
    
    // Insert an old Insights record that should go to Assets
    let insights_record = Record {
        id: Uuid::new_v4(),
        text: "Important insight".to_string(),
        layer: Layer::Insights,
        kind: "insight".to_string(),
        tags: vec!["important".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.5; 768],
        ts: now - Duration::days(2),
        last_access: now - Duration::hours(12),
        access_count: 50,
        score: 0.9,
    };
    
    store.insert(&insights_record).await?;
    
    // Run promotion
    let stats = promotion_engine.run_promotion_cycle().await?;
    
    println!("\n📊 Layer-to-layer promotion stats:");
    println!("  Insights → Assets: {}", stats.insights_to_assets);
    
    // Verify promotion to Assets
    let in_assets = store.get_by_id(&insights_record.id, Layer::Assets).await?;
    assert!(in_assets.is_some(), "High-value insight should be promoted to Assets");
    
    println!("✅ Layer-to-layer promotion test passed!");
    
    Ok(())
}