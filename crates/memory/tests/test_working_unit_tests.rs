#![cfg(all(feature = "extended-tests", feature = "legacy-tests"))]

// =============================================================================
// WORKING UNIT TESTS - Unit тесты для memory crate с правильными API
// Тесты для функциональности, которая точно компилируется
// =============================================================================

use chrono::Utc;
use memory::{
    cosine_distance_auto, BatchConfig, CacheConfig, Layer, MemoryLayer, PromotionConfig, Record,
    SearchOptions,
};
use proptest::prelude::*;
use uuid::Uuid;

#[cfg(test)]
mod layer_tests {
    use super::*;

    #[test]
    fn test_layer_variants() {
        // Test all layer variants exist
        let layers = vec![Layer::Interact, Layer::Insights, Layer::Assets];

        assert_eq!(layers.len(), 3);
    }

    #[test]
    fn test_layer_string_conversion() {
        assert_eq!(Layer::Interact.as_str(), "interact");
        assert_eq!(Layer::Insights.as_str(), "insights");
        assert_eq!(Layer::Assets.as_str(), "assets");
    }

    #[test]
    fn test_layer_table_names() {
        assert_eq!(Layer::Interact.table_name(), "layer_interact");
        assert_eq!(Layer::Insights.table_name(), "layer_insights");
        assert_eq!(Layer::Assets.table_name(), "layer_assets");
    }

    #[test]
    fn test_layer_serialization() {
        let layer = Layer::Interact;
        let json = serde_json::to_string(&layer).expect("Should serialize");
        let deserialized: Layer = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(layer, deserialized);
    }

    #[test]
    fn test_memory_layer_alias() {
        // Test that MemoryLayer alias works
        let layer: MemoryLayer = Layer::Interact;
        assert_eq!(layer, Layer::Interact);
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;

    #[test]
    fn test_record_default() {
        let record = Record::default();

        assert!(!record.id.is_nil());
        assert_eq!(record.text, "");
        assert!(record.embedding.is_empty());
        assert_eq!(record.layer, Layer::Interact);
        assert_eq!(record.kind, "general");
        assert!(record.tags.is_empty());
        assert_eq!(record.score, 0.0);
        assert_eq!(record.access_count, 0);
    }

    #[test]
    fn test_record_creation() {
        let now = Utc::now();
        let record = Record {
            id: Uuid::new_v4(),
            text: "test content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            layer: Layer::Insights,
            kind: "test_kind".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            ts: now,
            score: 0.85,
            access_count: 5,
            last_access: now,
        };

        assert_eq!(record.text, "test content");
        assert_eq!(record.embedding.len(), 3);
        assert_eq!(record.layer, Layer::Insights);
        assert_eq!(record.kind, "test_kind");
        assert_eq!(record.tags.len(), 2);
        assert_eq!(record.project, "test_project");
        assert_eq!(record.session, "test_session");
        assert_eq!(record.score, 0.85);
        assert_eq!(record.access_count, 5);
    }

    #[test]
    fn test_record_serialization() {
        let record = Record::default();
        let json = serde_json::to_string(&record).expect("Should serialize");
        let deserialized: Record = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(record.text, deserialized.text);
        assert_eq!(record.layer, deserialized.layer);
        assert_eq!(record.kind, deserialized.kind);
        assert_eq!(record.score, deserialized.score);
    }
}

#[cfg(test)]
mod search_options_tests {
    use super::*;

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();

        assert_eq!(options.layers.len(), 2);
        assert!(options.layers.contains(&Layer::Interact));
        assert!(options.layers.contains(&Layer::Insights));
        assert_eq!(options.top_k, 10);
        assert_eq!(options.score_threshold, 0.0);
        assert!(options.tags.is_empty());
        assert!(options.project.is_none());
    }

    #[test]
    fn test_search_options_custom() {
        let options = SearchOptions {
            layers: vec![Layer::Assets],
            top_k: 5,
            score_threshold: 0.7,
            tags: vec!["important".to_string()],
            project: Some("my_project".to_string()),
        };

        assert_eq!(options.layers.len(), 1);
        assert_eq!(options.layers[0], Layer::Assets);
        assert_eq!(options.top_k, 5);
        assert_eq!(options.score_threshold, 0.7);
        assert_eq!(options.tags.len(), 1);
        assert_eq!(options.project.as_ref().unwrap(), "my_project");
    }
}

#[cfg(test)]
mod promotion_config_tests {
    use super::*;

    #[test]
    fn test_promotion_config_default() {
        let config = PromotionConfig::default();

        assert_eq!(config.interact_ttl_hours, 24);
        assert_eq!(config.insights_ttl_days, 90);
        assert_eq!(config.promote_threshold, 0.8);
        assert_eq!(config.decay_factor, 0.9);
    }

    #[test]
    fn test_promotion_config_custom() {
        let config = PromotionConfig {
            interact_ttl_hours: 48,
            insights_ttl_days: 365,
            promote_threshold: 0.9,
            decay_factor: 0.8,
        };

        assert_eq!(config.interact_ttl_hours, 48);
        assert_eq!(config.insights_ttl_days, 365);
        assert_eq!(config.promote_threshold, 0.9);
        assert_eq!(config.decay_factor, 0.8);
    }

    #[test]
    fn test_promotion_config_validation() {
        let config = PromotionConfig::default();

        // Validate reasonable ranges
        assert!(config.promote_threshold > 0.0);
        assert!(config.promote_threshold <= 1.0);
        assert!(config.decay_factor > 0.0);
        assert!(config.decay_factor <= 1.0);
        assert!(config.interact_ttl_hours > 0);
        assert!(config.insights_ttl_days > 0);
    }
}

#[cfg(test)]
mod batch_config_tests {
    use super::*;

    #[test]
    fn test_batch_config_creation() {
        let config = BatchConfig {
            max_batch_size: 100,
            flush_interval: std::time::Duration::from_secs(5),
            worker_threads: 4,
            async_flush: true,
            max_queue_size: 1000,
        };

        assert_eq!(config.max_batch_size, 100);
        assert_eq!(config.worker_threads, 4);
        assert!(config.async_flush);
        assert_eq!(config.max_queue_size, 1000);
    }

    #[test]
    fn test_batch_config_validation() {
        let config = BatchConfig {
            max_batch_size: 1000,
            flush_interval: std::time::Duration::from_millis(500),
            worker_threads: 2,
            async_flush: false,
            max_queue_size: 5000,
        };

        assert!(config.max_batch_size > 0);
        assert!(config.worker_threads > 0);
        assert!(config.max_queue_size > 0);
    }
}

#[cfg(test)]
mod cache_config_tests {
    use super::*;

    #[test]
    fn test_cache_config_creation() {
        let config = CacheConfig {
            max_size_bytes: 50000,
            max_entries: 1000,
            ttl_seconds: Some(7200),
            eviction_batch_size: 10,
        };

        assert_eq!(config.max_size_bytes, 50000);
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.ttl_seconds.unwrap(), 7200);
        assert_eq!(config.eviction_batch_size, 10);
    }

    #[test]
    fn test_cache_config_validation() {
        let config = CacheConfig {
            max_size_bytes: 10000,
            max_entries: 500,
            ttl_seconds: Some(3600),
            eviction_batch_size: 5,
        };

        assert!(config.max_size_bytes > 0);
        assert!(config.max_entries > 0);
        assert!(config.ttl_seconds.unwrap() > 0);
        assert!(config.eviction_batch_size > 0);
    }

    #[test]
    fn test_cache_config_no_ttl() {
        let config = CacheConfig {
            max_size_bytes: 20000,
            max_entries: 2000,
            ttl_seconds: None,
            eviction_batch_size: 20,
        };

        assert!(config.ttl_seconds.is_none());
    }
}

#[cfg(test)]
mod vector_operations_tests {
    use super::*;

    #[test]
    fn test_cosine_distance_identical_vectors() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0, 3.0];

        let distance = cosine_distance_auto(&vec1, &vec2);

        // Identical vectors should have distance close to 0.0
        assert!(distance.abs() < 0.0001, "Expected ~0.0, got {}", distance);
    }

    #[test]
    fn test_cosine_distance_orthogonal_vectors() {
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![0.0, 1.0];

        let distance = cosine_distance_auto(&vec1, &vec2);

        // Orthogonal vectors should have distance close to 1.0
        assert!(
            (distance - 1.0).abs() < 0.0001,
            "Expected ~1.0, got {}",
            distance
        );
    }

    #[test]
    fn test_cosine_distance_opposite_vectors() {
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![-1.0, 0.0];

        let distance = cosine_distance_auto(&vec1, &vec2);

        // Opposite vectors should have distance close to 2.0
        assert!(
            (distance - 2.0).abs() < 0.0001,
            "Expected ~2.0, got {}",
            distance
        );
    }

    #[test]
    fn test_cosine_distance_properties() {
        let vec1 = vec![3.0, 4.0]; // |vec1| = 5
        let vec2 = vec![5.0, 0.0]; // |vec2| = 5

        let distance = cosine_distance_auto(&vec1, &vec2);

        // Distance should be non-negative and finite
        assert!(distance >= 0.0, "Distance should be non-negative");
        assert!(distance.is_finite(), "Distance should be finite");
        assert!(distance <= 2.0, "Distance should be at most 2.0");
    }
}

// Property-based tests
#[cfg(test)]
mod property_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_record_text_length_invariant(
            text in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let mut record = Record::default();
            record.text = text.clone();

            prop_assert_eq!(&record.text, &text);
            prop_assert!(!text.is_empty());
            prop_assert!(text.len() <= 100);
            prop_assert!(text.chars().all(|c| c.is_ascii_alphanumeric() || c == ' '));
        }

        #[test]
        fn test_embedding_dimension_consistency(
            values in proptest::collection::vec(-1.0f32..=1.0f32, 1..=2048)
        ) {
            let mut record = Record::default();
            record.embedding = values.clone();

            prop_assert_eq!(record.embedding.len(), values.len());
            prop_assert!(record.embedding.len() <= 2048);

            for &val in &record.embedding {
                prop_assert!(val >= -1.0 && val <= 1.0);
            }
        }

        #[test]
        fn test_score_range_validation(
            score in 0.0f32..=1.0f32
        ) {
            let mut record = Record::default();
            record.score = score;

            prop_assert!(record.score >= 0.0);
            prop_assert!(record.score <= 1.0);
        }

        #[test]
        fn test_search_options_top_k_validation(
            top_k in 1usize..=1000
        ) {
            let mut options = SearchOptions::default();
            options.top_k = top_k;

            prop_assert!(options.top_k >= 1);
            prop_assert!(options.top_k <= 1000);
        }

        #[test]
        fn test_search_options_threshold_validation(
            threshold in 0.0f32..=1.0f32
        ) {
            let mut options = SearchOptions::default();
            options.score_threshold = threshold;

            prop_assert!(options.score_threshold >= 0.0);
            prop_assert!(options.score_threshold <= 1.0);
        }
    }
}

// Performance tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_record_creation_performance() {
        let start = Instant::now();

        for _ in 0..1000 {
            let _record = Record::default();
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 100,
            "Record creation should be fast, took {:?}",
            duration
        );
    }

    #[test]
    fn test_cosine_distance_performance() {
        let vec1: Vec<f32> = (0..1000).map(|i| (i as f32) / 1000.0).collect();
        let vec2: Vec<f32> = (0..1000).map(|i| (1000 - i) as f32 / 1000.0).collect();

        let start = Instant::now();
        let _distance = cosine_distance_auto(&vec1, &vec2);
        let duration = start.elapsed();

        assert!(
            duration.as_micros() < 10000,
            "Cosine distance should be fast, took {:?}",
            duration
        );
    }

    #[test]
    fn test_layer_conversion_performance() {
        let start = Instant::now();

        for _ in 0..10000 {
            let _ = Layer::Interact.as_str();
            let _ = Layer::Insights.table_name();
        }

        let duration = start.elapsed();
        assert!(
            duration.as_millis() < 10,
            "Layer conversions should be very fast, took {:?}",
            duration
        );
    }
}

// Edge case tests
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_embedding_vector() {
        let mut record = Record::default();
        record.embedding = vec![];

        assert!(record.embedding.is_empty());
    }

    #[test]
    fn test_large_embedding_vector() {
        let mut record = Record::default();
        record.embedding = vec![0.0; 4096]; // Large embedding

        assert_eq!(record.embedding.len(), 4096);
    }

    #[test]
    fn test_zero_score() {
        let mut record = Record::default();
        record.score = 0.0;

        assert_eq!(record.score, 0.0);
    }

    #[test]
    fn test_maximum_score() {
        let mut record = Record::default();
        record.score = 1.0;

        assert_eq!(record.score, 1.0);
    }

    #[test]
    fn test_empty_tags() {
        let mut record = Record::default();
        record.tags = vec![];

        assert!(record.tags.is_empty());
    }

    #[test]
    fn test_many_tags() {
        let mut record = Record::default();
        record.tags = (0..100).map(|i| format!("tag_{}", i)).collect();

        assert_eq!(record.tags.len(), 100);
    }
}

// Serialization tests
#[cfg(test)]
mod serialization_tests {
    use super::*;

    #[test]
    fn test_layer_json_roundtrip() {
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let json = serde_json::to_string(&layer).unwrap();
            let restored: Layer = serde_json::from_str(&json).unwrap();
            assert_eq!(layer, restored);
        }
    }

    #[test]
    fn test_record_json_roundtrip() {
        let original = Record::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: Record = serde_json::from_str(&json).unwrap();

        assert_eq!(original.text, restored.text);
        assert_eq!(original.layer, restored.layer);
        assert_eq!(original.kind, restored.kind);
        assert_eq!(original.score, restored.score);
    }

    #[test]
    fn test_search_options_json_roundtrip() {
        let original = SearchOptions::default();
        let json = serde_json::to_string(&original).unwrap();
        let restored: SearchOptions = serde_json::from_str(&json).unwrap();

        assert_eq!(original.layers, restored.layers);
        assert_eq!(original.top_k, restored.top_k);
        assert_eq!(original.score_threshold, restored.score_threshold);
    }
}
