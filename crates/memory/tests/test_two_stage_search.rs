use anyhow::Result;
use memory::{Layer, MemoryConfig, MemoryService, Record};
use tempfile::TempDir;
use uuid::Uuid;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_test_config() -> Result<(TempDir, MemoryConfig)> {
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = TempDir::new()?;
    let config = MemoryConfig {
        db_path: temp_dir.path().join(format!("lancedb_{}", test_id)),
        cache_path: temp_dir.path().join(format!("cache_{}", test_id)),
        ..Default::default()
    };
    Ok((temp_dir, config))
}

#[tokio::test]
async fn test_two_stage_search_with_reranking() -> Result<()> {
    let (_temp_dir, config) = create_test_config()?;

    let service = MemoryService::new(config).await?;

    // Insert test documents with varying relevance to query
    let records = vec![
        Record {
            text: "Authentication system using OAuth tokens for secure login".to_string(),
            layer: Layer::Interact,
            kind: "solution".to_string(),
            tags: vec!["auth".to_string(), "oauth".to_string()],
            project: "auth-system".to_string(),
            score: 0.8,
            ..Default::default()
        },
        Record {
            text: "Database connection pool configuration for better performance".to_string(),
            layer: Layer::Insights,
            kind: "config".to_string(),
            tags: vec!["database".to_string(), "performance".to_string()],
            project: "backend".to_string(),
            score: 0.6,
            ..Default::default()
        },
        Record {
            text: "OAuth implementation guide with token refresh mechanism".to_string(),
            layer: Layer::Assets,
            kind: "documentation".to_string(),
            tags: vec!["oauth".to_string(), "guide".to_string()],
            project: "auth-system".to_string(),
            score: 0.9,
            ..Default::default()
        },
        Record {
            text: "User authentication flow using JWT tokens".to_string(),
            layer: Layer::Interact,
            kind: "implementation".to_string(),
            tags: vec!["auth".to_string(), "jwt".to_string()],
            project: "auth-system".to_string(),
            score: 0.7,
            ..Default::default()
        },
        Record {
            text: "Frontend login form validation and error handling".to_string(),
            layer: Layer::Insights,
            kind: "ui".to_string(),
            tags: vec!["frontend".to_string(), "validation".to_string()],
            project: "frontend".to_string(),
            score: 0.5,
            ..Default::default()
        },
    ];

    service.insert_batch(records).await?;

    // Test search with reranking
    let query = "OAuth authentication tokens";
    let results = service
        .search(query)
        .with_layers(&[Layer::Interact, Layer::Insights, Layer::Assets])
        .top_k(3)
        .execute()
        .await?;

    println!("=== Two-Stage Search Results ===");
    println!("Query: '{}'", query);
    println!("Results ({} found):", results.len());
    
    for (i, record) in results.iter().enumerate() {
        println!("  {}. [{}] Score: {:.3}", i + 1, record.layer.as_str(), record.score);
        println!("     Text: {}", &record.text[..60.min(record.text.len())]);
        println!("     Tags: {:?}", record.tags);
        println!();
    }

    // Verify we got results
    assert!(!results.is_empty(), "Search should return results");
    assert!(results.len() <= 3, "Should respect top_k limit");

    // Test that scores are in descending order (reranking should sort them)
    for i in 1..results.len() {
        assert!(
            results[i-1].score >= results[i].score,
            "Results should be sorted by score in descending order"
        );
    }

    // Verify that OAuth-related documents should score higher
    let oauth_results: Vec<_> = results.iter()
        .filter(|r| r.text.to_lowercase().contains("oauth"))
        .collect();
    
    if !oauth_results.is_empty() {
        println!("OAuth-related results found: {}", oauth_results.len());
        // OAuth documents should generally score high for this query
        assert!(oauth_results[0].score > 0.3, "OAuth documents should have decent scores");
    }

    Ok(())
}

#[tokio::test]
async fn test_embedding_consistency() -> Result<()> {
    let (_temp_dir, config) = create_test_config()?;

    let service = MemoryService::new(config).await?;

    // Insert the same text twice to test embedding consistency
    let text = "Consistent embedding test document";
    let record1 = Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        layer: Layer::Interact,
        ..Default::default()
    };
    let record2 = Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        layer: Layer::Insights,
        ..Default::default()
    };

    service.insert(record1).await?;
    service.insert(record2).await?;

    // Search for the text - both records should be found with identical scores
    let results = service
        .search(text)
        .with_layers(&[Layer::Interact, Layer::Insights])
        .top_k(10)
        .execute()
        .await?;

    assert_eq!(results.len(), 2, "Should find both identical records");
    
    // Mock embeddings are deterministic, so identical text should have very similar scores
    let score_diff = (results[0].score - results[1].score).abs();
    assert!(score_diff < 0.1, "Identical texts should have very similar scores, diff: {}", score_diff);

    println!("Embedding consistency test passed:");
    println!("  Record 1 score: {:.4}", results[0].score);
    println!("  Record 2 score: {:.4}", results[1].score);
    println!("  Score difference: {:.4}", score_diff);

    Ok(())
}

#[tokio::test]
async fn test_reranking_vs_embedding_scores() -> Result<()> {
    let (_temp_dir, config) = create_test_config()?;

    let service = MemoryService::new(config).await?;

    // Insert documents with different embedding vs reranking relevance
    let records = vec![
        Record {
            text: "authentication oauth token secure login system".to_string(), // High word overlap
            layer: Layer::Interact,
            score: 0.9,
            ..Default::default()
        },
        Record {
            text: "User access control with secure authentication mechanisms".to_string(), // Medium overlap
            layer: Layer::Interact,
            score: 0.7,
            ..Default::default()
        },
        Record {
            text: "Database performance optimization techniques".to_string(), // Low overlap
            layer: Layer::Interact,
            score: 1.0, // High initial score but low relevance
            ..Default::default()
        },
    ];

    service.insert_batch(records).await?;

    let query = "oauth authentication token";
    let results = service
        .search(query)
        .with_layer(Layer::Interact)
        .top_k(3)
        .execute()
        .await?;

    println!("=== Reranking vs Embedding Score Test ===");
    println!("Query: '{}'", query);
    
    for (i, record) in results.iter().enumerate() {
        println!("  {}. Score: {:.3} - {}", 
                 i + 1, 
                 record.score, 
                 &record.text[..50.min(record.text.len())]);
    }

    // The document with high word overlap should rank higher after reranking,
    // even if its initial embedding score was lower
    assert!(!results.is_empty(), "Should have results");
    
    // Find the high-overlap document
    let high_overlap_pos = results.iter().position(|r| 
        r.text.contains("oauth") && r.text.contains("authentication") && r.text.contains("token")
    );
    
    if let Some(pos) = high_overlap_pos {
        // High overlap document should be in top 2 positions after reranking
        assert!(pos <= 1, "High overlap document should rank highly after reranking");
        println!("✓ High overlap document ranked at position {}", pos + 1);
    }

    Ok(())
}