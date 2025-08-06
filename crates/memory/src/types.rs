use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Layer {
    Interact,   // L1 - hot context (24h TTL)
    Insights,   // L2 - distilled knowledge (90d TTL)
    Assets,     // L3 - cold artifacts (permanent)
}

impl Layer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Layer::Interact => "interact",
            Layer::Insights => "insights",
            Layer::Assets => "assets",
        }
    }

    pub fn table_name(&self) -> &'static str {
        match self {
            Layer::Interact => "layer_interact",
            Layer::Insights => "layer_insights",
            Layer::Assets => "layer_assets",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub text: String,
    pub embedding: Vec<f32>,
    pub layer: Layer,
    pub kind: String,
    pub tags: Vec<String>,
    pub project: String,
    pub session: String,
    pub ts: DateTime<Utc>,
    pub score: f32,
    pub access_count: u32,
    pub last_access: DateTime<Utc>,
}

impl Default for Record {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            text: String::new(),
            embedding: Vec::new(),
            layer: Layer::Interact,
            kind: "general".to_string(),
            tags: Vec::new(),
            project: String::new(),
            session: String::new(),
            ts: now,
            score: 0.0,
            access_count: 0,
            last_access: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub layers: Vec<Layer>,
    pub top_k: usize,
    pub score_threshold: f32,
    pub tags: Vec<String>,
    pub project: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            layers: vec![Layer::Interact, Layer::Insights],
            top_k: 10,
            score_threshold: 0.0,
            tags: Vec::new(),
            project: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromotionConfig {
    pub interact_ttl_hours: u64,
    pub insights_ttl_days: u64,
    pub promote_threshold: f32,
    pub decay_factor: f32,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        Self {
            interact_ttl_hours: 24,
            insights_ttl_days: 90,
            promote_threshold: 0.8,
            decay_factor: 0.9,
        }
    }
}