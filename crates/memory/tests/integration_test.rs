use anyhow::Result;
use memory::{Layer, MemoryConfig, MemoryService, Record};
use tempfile::TempDir;
use uuid::Uuid;

// No need for mock embedding function - AI service handles it now

#[tokio::test]
async fn test_memory_service_basic_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("lancedb"),
        cache_path: temp_dir.path().join("cache"),
        ..Default::default()
    };

    let service = MemoryService::new(config).await?;

    // Test inserting a record
    let record = Record {
        id: Uuid::new_v4(),
        text: "This is a test memory".to_string(),
        embedding: vec![],
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["test".to_string(), "memory".to_string()],
        project: "test-project".to_string(),
        session: "test-session".to_string(),
        ts: chrono::Utc::now(),
        score: 0.9,
        access_count: 0,
        last_access: chrono::Utc::now(),
    };

    service.insert(record.clone()).await?;

    // Test searching
    let results = service
        .search("test memory")
        .with_layer(Layer::Interact)
        .top_k(5)
        .execute()
        .await?;

    assert!(!results.is_empty());
    assert_eq!(results[0].text, "This is a test memory");

    Ok(())
}

#[tokio::test]
async fn test_memory_layers() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("lancedb"),
        cache_path: temp_dir.path().join("cache"),
        ..Default::default()
    };

    let service = MemoryService::new(config).await?;

    // Insert records in different layers
    let layers = [Layer::Interact, Layer::Insights, Layer::Assets];
    
    for (i, layer) in layers.iter().enumerate() {
        let record = Record {
            text: format!("Record in layer {:?}", layer),
            layer: *layer,
            score: 0.5 + (i as f32 * 0.1),
            ..Default::default()
        };
        service.insert(record).await?;
    }

    // Search across all layers
    let results = service
        .search("Record in layer")
        .with_layers(&[Layer::Interact, Layer::Insights, Layer::Assets])
        .top_k(10)
        .execute()
        .await?;

    assert_eq!(results.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_embedding_cache() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("lancedb"),
        cache_path: temp_dir.path().join("cache"),
        ..Default::default()
    };

    let service = MemoryService::new(config).await?;

    // Insert same text twice
    let text = "Cached text content";
    let record1 = Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        ..Default::default()
    };
    let record2 = Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        ..Default::default()
    };

    service.insert(record1).await?;
    service.insert(record2).await?;

    // Check cache stats
    let (hits, misses, _) = service.cache_stats();
    assert!(hits > 0); // Second insert should hit cache
    assert!(service.cache_hit_rate() > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("lancedb"),
        cache_path: temp_dir.path().join("cache"),
        ..Default::default()
    };

    let service = MemoryService::new(config).await?;

    // Create batch of records
    let records: Vec<Record> = (0..10)
        .map(|i| Record {
            text: format!("Batch record {}", i),
            score: i as f32 / 10.0,
            ..Default::default()
        })
        .collect();

    service.insert_batch(records).await?;

    // Search for batch records
    let results = service
        .search("Batch record")
        .top_k(20)
        .execute()
        .await?;

    assert_eq!(results.len(), 10);

    Ok(())
}

#[tokio::test]
async fn test_search_filters() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join("lancedb"),
        cache_path: temp_dir.path().join("cache"),
        ..Default::default()
    };

    let service = MemoryService::new(config).await?;

    // Insert records with different tags and projects
    let records = vec![
        Record {
            text: "Important decision".to_string(),
            tags: vec!["decision".to_string(), "important".to_string()],
            project: "project-a".to_string(),
            score: 0.9,
            ..Default::default()
        },
        Record {
            text: "Regular note".to_string(),
            tags: vec!["note".to_string()],
            project: "project-b".to_string(),
            score: 0.5,
            ..Default::default()
        },
        Record {
            text: "Another decision".to_string(),
            tags: vec!["decision".to_string()],
            project: "project-a".to_string(),
            score: 0.7,
            ..Default::default()
        },
    ];

    service.insert_batch(records).await?;

    // Test tag filtering
    let results = service
        .search("decision")
        .with_tags(vec!["decision".to_string()])
        .execute()
        .await?;

    assert_eq!(results.len(), 2);

    // Test project filtering
    let results = service
        .search("note")
        .in_project("project-a".to_string())
        .execute()
        .await?;

    // Project filtering would filter out the "Regular note" from project-b
    assert!(results.is_empty() || results.iter().all(|r| r.project == "project-a"));

    // Test score threshold
    let results = service
        .search("decision")
        .min_score(0.8)
        .execute()
        .await?;

    assert!(results.iter().all(|r| r.score >= 0.8));

    Ok(())
}