use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{Layer, Record};

/// Решение о promotion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionDecision {
    pub record_id: Uuid,
    pub record: Record,
    pub current_layer: Layer,
    pub target_layer: Layer,
    pub confidence: f32,
    pub features: PromotionFeatures,
    pub decision_timestamp: DateTime<Utc>,
    pub algorithm_used: String,
}

/// Feature vector для ML модели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionFeatures {
    /// Temporal features
    pub age_hours: f32,
    pub access_recency: f32,
    pub temporal_pattern_score: f32,
    
    /// Usage features  
    pub access_count: f32,
    pub access_frequency: f32,
    pub session_importance: f32,
    
    /// Semantic features
    pub semantic_importance: f32,
    pub keyword_density: f32,
    pub topic_relevance: f32,
    
    /// Context features
    pub layer_affinity: f32,
    pub co_occurrence_score: f32,
    pub user_preference_score: f32,
}

/// ML promotion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLPromotionConfig {
    /// Минимальный access count для рассмотрения promotion
    pub min_access_threshold: u32,
    /// Вес для temporal features (0.0-1.0)
    pub temporal_weight: f32,
    /// Вес для semantic features (0.0-1.0)
    pub semantic_weight: f32,
    /// Вес для usage features (0.0-1.0)
    pub usage_weight: f32,
    /// Порог для promotion (0.0-1.0)
    pub promotion_threshold: f32,
    /// Размер batch для ML inference
    pub ml_batch_size: usize,
    /// Интервал обучения модели (в часах)
    pub training_interval_hours: u64,
    /// Использовать GPU для ML операций
    pub use_gpu_for_ml: bool,
    /// Конфигурация алгоритма
    pub algorithm_name: String,
}

impl Default for MLPromotionConfig {
    fn default() -> Self {
        Self {
            min_access_threshold: 3,
            temporal_weight: 0.3,
            semantic_weight: 0.4,
            usage_weight: 0.3,
            promotion_threshold: 0.7,
            ml_batch_size: 32,
            training_interval_hours: 24,
            use_gpu_for_ml: true,
            algorithm_name: "hybrid".to_string(),
        }
    }
}

impl MLPromotionConfig {
    pub fn production() -> Self {
        Self {
            min_access_threshold: 5,
            temporal_weight: 0.25,
            semantic_weight: 0.45,
            usage_weight: 0.3,
            promotion_threshold: 0.8,
            ml_batch_size: 64,
            training_interval_hours: 12,
            use_gpu_for_ml: true,
            algorithm_name: "hybrid".to_string(),
        }
    }

    pub fn minimal() -> Self {
        Self {
            min_access_threshold: 1,
            temporal_weight: 0.4,
            semantic_weight: 0.3,
            usage_weight: 0.3,
            promotion_threshold: 0.6,
            ml_batch_size: 16,
            training_interval_hours: 48,
            use_gpu_for_ml: false,
            algorithm_name: "frequency".to_string(),
        }
    }
}

/// Результаты ML-based promotion
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MLPromotionStats {
    pub total_analyzed: usize,
    pub promoted_interact_to_insights: usize,
    pub promoted_insights_to_assets: usize,
    pub ml_inference_time_ms: u64,
    pub feature_extraction_time_ms: u64,
    pub model_accuracy: f32,
    pub avg_confidence_score: f32,
    pub cache_hit_rate: f32,
    pub gpu_utilization: f32,
    // Дополнительные поля для совместимости
    pub analyzed_records: usize,
    pub promoted_records: usize,
    pub processing_time_ms: f64,
    pub algorithm_used: String,
}

/// Паттерн доступа к записи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub total_accesses: u64,
    pub recent_accesses: u64,
    pub access_velocity: f32,
    pub last_access: DateTime<Utc>,
    pub peak_access_time: Option<DateTime<Utc>>,
}

/// Семантический контекст записи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticContext {
    pub topic_scores: Vec<(String, f32)>,
    pub keyword_weights: Vec<(String, f32)>,
    pub similarity_scores: Vec<(Uuid, f32)>,
    pub content_categories: Vec<String>,
}

/// Результаты промоции для внутреннего использования
#[derive(Debug, Default)]
pub struct PromotionResults {
    pub analyzed_count: usize,
    pub promoted_count: usize,
    pub decisions: Vec<PromotionDecision>,
    pub avg_confidence: f32,
    pub processing_time_ms: u64,
}

/// Конверсия MLPromotionStats в стандартный PromotionStats для совместимости
impl From<MLPromotionStats> for crate::promotion::PromotionStats {
    fn from(ml_stats: MLPromotionStats) -> Self {
        Self {
            interact_to_insights: ml_stats.promoted_interact_to_insights,
            insights_to_assets: ml_stats.promoted_insights_to_assets,
            expired_interact: 0, // ML система не отслеживает expiration
            expired_insights: 0, // ML система не отслеживает expiration
            total_time_ms: ml_stats.ml_inference_time_ms + ml_stats.feature_extraction_time_ms,
            index_update_time_ms: 0, // Приблизительная оценка
            promotion_time_ms: ml_stats.ml_inference_time_ms,
            cleanup_time_ms: 0, // ML система не требует cleanup
        }
    }
}