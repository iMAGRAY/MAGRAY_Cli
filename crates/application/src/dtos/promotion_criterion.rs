//! Promotion criteria DTOs

use domain::LayerType;
use serde::{Deserialize, Serialize};

/// Promotion criterion for memory records
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionCriterion {
    pub min_access_frequency: Option<u64>,
    pub min_similarity_score: Option<f32>,
    pub min_score_threshold: Option<f32>,
    pub time_window_hours: Option<u64>,
    pub max_hours_since_access: Option<u64>,
    pub from_layer: Option<LayerType>,
    pub to_layer: LayerType,
    pub target_layers: Vec<LayerType>,
    pub project_filter: Option<String>,
    pub boost_recent_activity: bool,
}

impl Default for PromotionCriterion {
    fn default() -> Self {
        Self {
            min_access_frequency: Some(5),
            min_similarity_score: Some(0.7),
            min_score_threshold: Some(0.7),
            time_window_hours: Some(24),
            max_hours_since_access: Some(168),
            from_layer: None,
            to_layer: LayerType::Insights,
            target_layers: vec![LayerType::Insights],
            project_filter: None,
            boost_recent_activity: true,
        }
    }
}
