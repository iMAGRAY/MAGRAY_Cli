use chrono::Utc;
use memory::{Layer, PromotionConfig, Record, SearchOptions};
use uuid::Uuid;

#[test]
fn test_layer_enum() {
    let layer = Layer::Interact;
    assert_eq!(layer.as_str(), "interact");

    let layer = Layer::Insights;
    assert_eq!(layer.as_str(), "insights");

    let layer = Layer::Assets;
    assert_eq!(layer.as_str(), "assets");
}

#[test]
fn test_layer_table_names() {
    assert_eq!(Layer::Interact.table_name(), "layer_interact");
    assert_eq!(Layer::Insights.table_name(), "layer_insights");
    assert_eq!(Layer::Assets.table_name(), "layer_assets");
}

#[test]
fn test_layer_ordering() {
    // Проверяем что Layer implement Ord
    let mut layers = [Layer::Assets, Layer::Interact, Layer::Insights];
    layers.sort();

    // После сортировки должны быть в порядке: Interact, Insights, Assets
    assert_eq!(layers[0], Layer::Interact);
    assert_eq!(layers[1], Layer::Insights);
    assert_eq!(layers[2], Layer::Assets);
}

#[test]
fn test_record_default() {
    let record = Record::default();

    assert_eq!(record.layer, Layer::Interact);
    assert_eq!(record.kind, "general");
    assert_eq!(record.access_count, 0);
    assert_eq!(record.score, 0.0);
    assert!(record.text.is_empty());
    assert!(record.embedding.is_empty());
    assert!(record.tags.is_empty());
    assert!(record.project.is_empty());
    assert!(record.session.is_empty());
}

#[test]
fn test_record_creation() {
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = Record {
        id,
        text: "Test content".to_string(),
        embedding: vec![1.0, 2.0, 3.0],
        layer: Layer::Insights,
        kind: "technical".to_string(),
        tags: vec!["rust".to_string(), "test".to_string()],
        project: "magray".to_string(),
        session: "session-123".to_string(),
        ts: now,
        score: 0.95,
        access_count: 5,
        last_access: now,
    };

    assert_eq!(record.id, id);
    assert_eq!(record.text, "Test content");
    assert_eq!(record.embedding, vec![1.0, 2.0, 3.0]);
    assert_eq!(record.layer, Layer::Insights);
    assert_eq!(record.kind, "technical");
    assert_eq!(record.tags.len(), 2);
    assert_eq!(record.project, "magray");
    assert_eq!(record.session, "session-123");
    assert_eq!(record.score, 0.95);
    assert_eq!(record.access_count, 5);
}

#[test]
fn test_record_serialization() {
    let record = Record {
        id: Uuid::new_v4(),
        text: "Serialization test".to_string(),
        embedding: vec![1.0, 2.0],
        layer: Layer::Assets,
        kind: "document".to_string(),
        tags: vec!["test".to_string()],
        project: "test_project".to_string(),
        session: "test_session".to_string(),
        ts: Utc::now(),
        score: 0.8,
        access_count: 10,
        last_access: Utc::now(),
    };

    // Сериализация в JSON
    let json = serde_json::to_string(&record).expect("Test operation should succeed");
    assert!(json.contains("Serialization test"));
    assert!(json.contains("document"));

    // Десериализация обратно
    let deserialized: Record = serde_json::from_str(&json).expect("Test operation should succeed");
    assert_eq!(deserialized.id, record.id);
    assert_eq!(deserialized.text, record.text);
    assert_eq!(deserialized.layer, record.layer);
    assert_eq!(deserialized.kind, record.kind);
}

#[test]
fn test_search_options_default() {
    let options = SearchOptions::default();

    assert_eq!(options.layers, vec![Layer::Interact, Layer::Insights]);
    assert_eq!(options.top_k, 10);
    assert_eq!(options.score_threshold, 0.0);
    assert!(options.tags.is_empty());
    assert_eq!(options.project, None);
}

#[test]
fn test_search_options_custom() {
    let options = SearchOptions {
        layers: vec![Layer::Assets],
        top_k: 20,
        score_threshold: 0.7,
        tags: vec!["rust".to_string(), "memory".to_string()],
        project: Some("magray".to_string()),
    };

    assert_eq!(options.layers, vec![Layer::Assets]);
    assert_eq!(options.top_k, 20);
    assert_eq!(options.score_threshold, 0.7);
    assert_eq!(options.tags.len(), 2);
    assert_eq!(options.project, Some("magray".to_string()));
}

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
        insights_ttl_days: 180,
        promote_threshold: 0.6,
        decay_factor: 0.95,
    };

    assert_eq!(config.interact_ttl_hours, 48);
    assert_eq!(config.insights_ttl_days, 180);
    assert_eq!(config.promote_threshold, 0.6);
    assert_eq!(config.decay_factor, 0.95);
}

#[test]
fn test_record_clone() {
    let original = Record {
        id: Uuid::new_v4(),
        text: "Clone test".to_string(),
        embedding: vec![1.0, 2.0, 3.0],
        layer: Layer::Assets,
        kind: "test".to_string(),
        tags: vec!["clone".to_string()],
        project: "test".to_string(),
        session: "session".to_string(),
        ts: Utc::now(),
        score: 0.9,
        access_count: 15,
        last_access: Utc::now(),
    };

    let cloned = original.clone();

    assert_eq!(cloned.id, original.id);
    assert_eq!(cloned.text, original.text);
    assert_eq!(cloned.embedding, original.embedding);
    assert_eq!(cloned.layer, original.layer);
    assert_eq!(cloned.kind, original.kind);
    assert_eq!(cloned.tags, original.tags);
    assert_eq!(cloned.project, original.project);
    assert_eq!(cloned.session, original.session);
    assert_eq!(cloned.score, original.score);
    assert_eq!(cloned.access_count, original.access_count);
}

#[test]
fn test_search_options_serialization() {
    let options = SearchOptions {
        layers: vec![Layer::Interact, Layer::Assets],
        top_k: 15,
        score_threshold: 0.5,
        tags: vec!["ai".to_string(), "llm".to_string()],
        project: Some("magray_cli".to_string()),
    };

    // Сериализация и десериализация
    let json = serde_json::to_string(&options).expect("Test operation should succeed");
    let deserialized: SearchOptions =
        serde_json::from_str(&json).expect("Test operation should succeed");

    assert_eq!(deserialized.layers, options.layers);
    assert_eq!(deserialized.top_k, options.top_k);
    assert_eq!(deserialized.score_threshold, options.score_threshold);
    assert_eq!(deserialized.tags, options.tags);
    assert_eq!(deserialized.project, options.project);
}

#[test]
fn test_layer_equality() {
    assert_eq!(Layer::Interact, Layer::Interact);
    assert_ne!(Layer::Interact, Layer::Insights);
    assert_ne!(Layer::Insights, Layer::Assets);

    // Test Copy trait
    let layer1 = Layer::Assets;
    let layer2 = layer1; // Copy
    assert_eq!(layer1, layer2);
}

#[test]
fn test_layer_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(Layer::Interact);
    set.insert(Layer::Insights);
    set.insert(Layer::Assets);

    assert_eq!(set.len(), 3);
    assert!(set.contains(&Layer::Interact));
    assert!(set.contains(&Layer::Insights));
    assert!(set.contains(&Layer::Assets));
}
