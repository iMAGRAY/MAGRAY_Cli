use memory::promotion::*;
use memory::types::*;
use std::collections::HashMap;
use chrono::{Utc, Duration as ChronoDuration};

#[test]
fn test_promotion_engine_creation() {
    let config = PromotionConfig::default();
    let engine = PromotionEngine::new(config);
    
    assert!(engine.is_ok());
    
    let engine = engine.unwrap();
    assert_eq!(engine.pending_promotions(), 0);
    assert!(engine.last_promotion_time().is_none());
}

#[test]
fn test_promotion_config_validation() {
    let mut config = PromotionConfig::default();
    
    // Valid config
    assert!(config.validate().is_ok());
    
    // Invalid min_age
    config.min_age_hours = 0;
    assert!(config.validate().is_err());
    
    // Reset and test invalid access threshold
    config = PromotionConfig::default();
    config.min_access_count = 0;
    assert!(config.validate().is_err());
    
    // Reset and test invalid importance threshold
    config = PromotionConfig::default();
    config.min_importance = -0.1;
    assert!(config.validate().is_err());
    
    config.min_importance = 1.1;
    assert!(config.validate().is_err());
}

#[test]
fn test_promotion_candidate_evaluation() {
    let config = PromotionConfig {
        min_age_hours: 1,
        min_access_count: 2,
        min_importance: 0.5,
        interaction_to_insights_ratio: 0.3,
        insights_to_assets_ratio: 0.1,
        max_promotions_per_cycle: 10,
    };
    
    let engine = PromotionEngine::new(config).unwrap();
    
    // Create test records
    let old_timestamp = Utc::now() - ChronoDuration::hours(2);
    
    let candidate_record = MemoryRecord {
        id: "candidate".to_string(),
        content: "Important content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: old_timestamp,
            tags: vec!["important".to_string()],
            source: Some("test".to_string()),
            importance: 0.8,
            access_count: 5,
            last_accessed: Utc::now() - ChronoDuration::minutes(30),
        },
    };
    
    let not_candidate_record = MemoryRecord {
        id: "not_candidate".to_string(),
        content: "Less important".to_string(),
        embedding: vec![0.1, 0.2],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now(), // Too new
            tags: vec![],
            source: None,
            importance: 0.3, // Too low
            access_count: 1, // Too low
            last_accessed: Utc::now(),
        },
    };
    
    let records = vec![candidate_record.clone(), not_candidate_record];
    let candidates = engine.evaluate_promotion_candidates(&records);
    
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].record.id, "candidate");
    assert!(candidates[0].score > 0.0);
}

#[test]
fn test_promotion_scoring() {
    let config = PromotionConfig::default();
    let engine = PromotionEngine::new(config).unwrap();
    
    let high_score_record = MemoryRecord {
        id: "high_score".to_string(),
        content: "Very important content".to_string(),
        embedding: vec![0.1, 0.2, 0.3, 0.4],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now() - ChronoDuration::days(1),
            tags: vec!["critical".to_string(), "important".to_string()],
            source: Some("system".to_string()),
            importance: 0.95,
            access_count: 20,
            last_accessed: Utc::now() - ChronoDuration::hours(1),
        },
    };
    
    let low_score_record = MemoryRecord {
        id: "low_score".to_string(),
        content: "Less important".to_string(),
        embedding: vec![0.1],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now() - ChronoDuration::hours(2),
            tags: vec![],
            source: None,
            importance: 0.2,
            access_count: 1,
            last_accessed: Utc::now() - ChronoDuration::hours(2),
        },
    };
    
    let records = vec![high_score_record, low_score_record];
    let candidates = engine.evaluate_promotion_candidates(&records);
    
    if candidates.len() >= 2 {
        // Higher importance and access should result in higher score
        let high_candidate = candidates.iter().find(|c| c.record.id == "high_score").unwrap();
        let low_candidate = candidates.iter().find(|c| c.record.id == "low_score").unwrap();
        assert!(high_candidate.score > low_candidate.score);
    }
}

#[test]
fn test_promotion_layer_transitions() {
    let config = PromotionConfig::default();
    let engine = PromotionEngine::new(config).unwrap();
    
    // Test Interact -> Insights promotion
    let interact_record = MemoryRecord {
        id: "interact_record".to_string(),
        content: "Interact content".to_string(),
        embedding: vec![0.1, 0.2],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now() - ChronoDuration::days(1),
            tags: vec!["test".to_string()],
            source: None,
            importance: 0.8,
            access_count: 10,
            last_accessed: Utc::now(),
        },
    };
    
    let promoted = engine.promote_record(interact_record);
    assert!(promoted.is_ok());
    
    let promoted_record = promoted.unwrap();
    assert_eq!(promoted_record.metadata.layer, Layer::Insights);
    assert_eq!(promoted_record.id, "interact_record");
    assert_eq!(promoted_record.content, "Interact content");
    
    // Test Insights -> Assets promotion
    let insights_record = MemoryRecord {
        id: "insights_record".to_string(),
        content: "Insights content".to_string(),
        embedding: vec![0.3, 0.4, 0.5],
        metadata: MemoryMetadata {
            layer: Layer::Insights,
            timestamp: Utc::now() - ChronoDuration::days(7),
            tags: vec!["knowledge".to_string()],
            source: Some("analysis".to_string()),
            importance: 0.9,
            access_count: 25,
            last_accessed: Utc::now(),
        },
    };
    
    let promoted = engine.promote_record(insights_record);
    assert!(promoted.is_ok());
    
    let promoted_record = promoted.unwrap();
    assert_eq!(promoted_record.metadata.layer, Layer::Assets);
}

#[test]
fn test_promotion_rules_validation() {
    let rules = PromotionRules::default();
    
    let valid_record = MemoryRecord {
        id: "valid".to_string(),
        content: "Valid content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now() - ChronoDuration::days(1),
            tags: vec!["valid".to_string()],
            source: None,
            importance: 0.8,
            access_count: 10,
            last_accessed: Utc::now(),
        },
    };
    
    assert!(rules.should_promote(&valid_record));
    
    let invalid_record = MemoryRecord {
        id: "invalid".to_string(),
        content: "Invalid content".to_string(),
        embedding: vec![0.1],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now(), // Too recent
            tags: vec![],
            source: None,
            importance: 0.1, // Too low
            access_count: 0, // Too low
            last_accessed: Utc::now(),
        },
    };
    
    assert!(!rules.should_promote(&invalid_record));
}

#[test]
fn test_promotion_batch_processing() {
    let config = PromotionConfig {
        min_age_hours: 1,
        min_access_count: 1,
        min_importance: 0.3,
        interaction_to_insights_ratio: 0.5,
        insights_to_assets_ratio: 0.2,
        max_promotions_per_cycle: 3,
    };
    
    let mut engine = PromotionEngine::new(config).unwrap();
    
    // Create multiple promotion candidates
    let mut records = vec![];
    for i in 0..5 {
        let record = MemoryRecord {
            id: format!("batch_record_{}", i),
            content: format!("Batch content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: Utc::now() - ChronoDuration::hours(2),
                tags: vec![format!("batch_{}", i)],
                source: None,
                importance: 0.5 + (i as f32 * 0.1),
                access_count: i + 2,
                last_accessed: Utc::now(),
            },
        };
        records.push(record);
    }
    
    let result = engine.process_promotion_cycle(&records);
    assert!(result.is_ok());
    
    let promoted = result.unwrap();
    // Should respect max_promotions_per_cycle
    assert!(promoted.len() <= 3);
    
    // Should promote records with higher scores first
    if promoted.len() >= 2 {
        for i in 0..promoted.len()-1 {
            let current_importance = promoted[i].metadata.importance;
            let next_importance = promoted[i+1].metadata.importance;
            assert!(current_importance >= next_importance);
        }
    }
}

#[test]
fn test_promotion_statistics() {
    let config = PromotionConfig::default();
    let mut engine = PromotionEngine::new(config).unwrap();
    
    let initial_stats = engine.get_promotion_statistics();
    assert_eq!(initial_stats.total_promotions, 0);
    assert_eq!(initial_stats.interact_to_insights, 0);
    assert_eq!(initial_stats.insights_to_assets, 0);
    assert_eq!(initial_stats.failed_promotions, 0);
    
    // Create test records for promotion
    let interact_record = MemoryRecord {
        id: "stats_test".to_string(),
        content: "Stats test content".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        metadata: MemoryMetadata {
            layer: Layer::Interact,
            timestamp: Utc::now() - ChronoDuration::days(1),
            tags: vec!["stats".to_string()],
            source: None,
            importance: 0.8,
            access_count: 10,
            last_accessed: Utc::now(),
        },
    };
    
    let records = vec![interact_record];
    engine.process_promotion_cycle(&records).unwrap();
    
    let updated_stats = engine.get_promotion_statistics();
    assert!(updated_stats.total_promotions > 0);
    assert!(updated_stats.interact_to_insights > 0);
}

#[test]
fn test_time_based_promotion_index() {
    let mut time_index = TimeBasedPromotionIndex::new();
    
    assert!(time_index.is_empty());
    assert_eq!(time_index.len(), 0);
    
    let old_time = Utc::now() - ChronoDuration::days(1);
    let recent_time = Utc::now() - ChronoDuration::hours(1);
    
    time_index.add_record("old_record".to_string(), old_time);
    time_index.add_record("recent_record".to_string(), recent_time);
    
    assert!(!time_index.is_empty());
    assert_eq!(time_index.len(), 2);
    
    // Get records older than 12 hours
    let cutoff = Utc::now() - ChronoDuration::hours(12);
    let old_records = time_index.get_records_older_than(cutoff);
    
    assert_eq!(old_records.len(), 1);
    assert_eq!(old_records[0], "old_record");
}

#[test]
fn test_promotion_config_defaults() {
    let config = PromotionConfig::default();
    
    assert!(config.min_age_hours > 0);
    assert!(config.min_access_count > 0);
    assert!(config.min_importance >= 0.0 && config.min_importance <= 1.0);
    assert!(config.interaction_to_insights_ratio > 0.0);
    assert!(config.insights_to_assets_ratio > 0.0);
    assert!(config.max_promotions_per_cycle > 0);
}

#[test]
fn test_promotion_engine_error_handling() {
    let config = PromotionConfig::default();
    let engine = PromotionEngine::new(config).unwrap();
    
    // Test promoting Assets record (should fail as it's already top layer)
    let assets_record = MemoryRecord {
        id: "assets_record".to_string(),
        content: "Assets content".to_string(),
        embedding: vec![0.1, 0.2],
        metadata: MemoryMetadata {
            layer: Layer::Assets,
            timestamp: Utc::now() - ChronoDuration::days(1),
            tags: vec![],
            source: None,
            importance: 0.9,
            access_count: 20,
            last_accessed: Utc::now(),
        },
    };
    
    let result = engine.promote_record(assets_record);
    assert!(result.is_err());
}

#[test]
fn test_promotion_candidate_sorting() {
    let config = PromotionConfig::default();
    let engine = PromotionEngine::new(config).unwrap();
    
    let mut records = vec![];
    
    // Create records with different scores
    for i in 0..5 {
        let record = MemoryRecord {
            id: format!("sort_test_{}", i),
            content: format!("Content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: Utc::now() - ChronoDuration::hours(2),
                tags: vec![],
                source: None,
                importance: 0.5 + (i as f32 * 0.1), // Increasing importance
                access_count: i + 1,
                last_accessed: Utc::now(),
            },
        };
        records.push(record);
    }
    
    let candidates = engine.evaluate_promotion_candidates(&records);
    
    // Candidates should be sorted by score (descending)
    for i in 0..candidates.len()-1 {
        assert!(candidates[i].score >= candidates[i+1].score);
    }
}

#[test]
fn test_promotion_frequency_limiting() {
    let config = PromotionConfig {
        min_age_hours: 1,
        min_access_count: 1,
        min_importance: 0.1,
        interaction_to_insights_ratio: 1.0, // Allow all
        insights_to_assets_ratio: 1.0, // Allow all
        max_promotions_per_cycle: 2, // Limit to 2
    };
    
    let mut engine = PromotionEngine::new(config).unwrap();
    
    // Create many promotion candidates
    let mut records = vec![];
    for i in 0..10 {
        let record = MemoryRecord {
            id: format!("freq_test_{}", i),
            content: format!("Content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: Utc::now() - ChronoDuration::hours(2),
                tags: vec![],
                source: None,
                importance: 0.8,
                access_count: 10,
                last_accessed: Utc::now(),
            },
        };
        records.push(record);
    }
    
    let result = engine.process_promotion_cycle(&records);
    assert!(result.is_ok());
    
    let promoted = result.unwrap();
    assert_eq!(promoted.len(), 2); // Should respect the limit
}

#[test]
fn test_promotion_layer_ratios() {
    let config = PromotionConfig {
        min_age_hours: 1,
        min_access_count: 1,
        min_importance: 0.1,
        interaction_to_insights_ratio: 0.5, // 50% of Interact records
        insights_to_assets_ratio: 0.3, // 30% of Insights records
        max_promotions_per_cycle: 100,
    };
    
    let engine = PromotionEngine::new(config).unwrap();
    
    // Create records in different layers
    let mut interact_records = vec![];
    let mut insights_records = vec![];
    
    // 10 Interact records (should promote ~5)
    for i in 0..10 {
        let record = MemoryRecord {
            id: format!("interact_{}", i),
            content: format!("Interact content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Interact,
                timestamp: Utc::now() - ChronoDuration::hours(2),
                tags: vec![],
                source: None,
                importance: 0.8,
                access_count: 5,
                last_accessed: Utc::now(),
            },
        };
        interact_records.push(record);
    }
    
    // 10 Insights records (should promote ~3)
    for i in 0..10 {
        let record = MemoryRecord {
            id: format!("insights_{}", i),
            content: format!("Insights content {}", i),
            embedding: vec![i as f32 * 0.1],
            metadata: MemoryMetadata {
                layer: Layer::Insights,
                timestamp: Utc::now() - ChronoDuration::days(1),
                tags: vec![],
                source: None,
                importance: 0.8,
                access_count: 15,
                last_accessed: Utc::now(),
            },
        };
        insights_records.push(record);
    }
    
    let all_records = [interact_records, insights_records].concat();
    let candidates = engine.evaluate_promotion_candidates(&all_records);
    
    // Should respect the ratios approximately
    let interact_candidates: Vec<_> = candidates.iter()
        .filter(|c| c.record.metadata.layer == Layer::Interact)
        .collect();
    let insights_candidates: Vec<_> = candidates.iter()
        .filter(|c| c.record.metadata.layer == Layer::Insights)
        .collect();
    
    // Ratios should be approximately respected (allowing some variance)
    assert!(interact_candidates.len() <= 6); // ~50% of 10
    assert!(insights_candidates.len() <= 4); // ~30% of 10
}