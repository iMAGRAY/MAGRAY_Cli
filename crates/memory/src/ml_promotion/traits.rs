use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{MLPromotionStats, PromotionDecision, PromotionFeatures};
use crate::types::{Layer, Record};

/// Trait для ML алгоритмов promotion
#[async_trait]
pub trait PromotionAlgorithm: Send + Sync {
    /// Предсказывает promotion score для записи
    fn predict_score(&self, features: &PromotionFeatures) -> f32;

    /// Обучает модель на исторических данных
    async fn train(&mut self, training_data: &[TrainingExample]) -> Result<f32>;

    /// Получает точность модели
    fn get_accuracy(&self) -> f32;

    /// Сохраняет лучшие веса модели
    fn save_best_weights(&mut self);

    /// Восстанавливает лучшие веса
    fn restore_best_weights(&mut self);
}

/// Trait для сбора метрик и аналитики
#[async_trait]
pub trait PromotionMetrics: Send + Sync {
    /// Записывает метрики inference
    fn record_inference(&mut self, inference_time_ms: u64, accuracy: f32);

    /// Записывает метрики feature extraction
    fn record_feature_extraction(&mut self, extraction_time_ms: u64);

    /// Обновляет cache статистику
    fn update_cache_stats(&mut self, hit_rate: f32);

    /// Обновляет GPU utilization
    fn update_gpu_stats(&mut self, utilization: f32);

    /// Получает агрегированную статистику
    fn get_stats(&self) -> MLPromotionStats;

    /// Сбрасывает накопленные метрики
    fn reset_metrics(&mut self);
}

/// Trait для business rules и promotion стратегий
#[async_trait]
pub trait PromotionRulesEngine: Send + Sync {
    /// Проверяет может ли запись быть promoted
    async fn can_promote(&self, record: &Record) -> bool;

    /// Определяет целевой layer для promotion
    fn determine_target_layer(&self, record: &Record, confidence: f32) -> Layer;

    /// Применяет business rules для фильтрации кандидатов
    async fn filter_candidates(&self, candidates: Vec<Record>) -> Result<Vec<Record>>;

    /// Валидирует promotion decision
    fn validate_promotion(&self, decision: &PromotionDecision) -> bool;
}

/// Trait для обработки данных в ML pipeline
#[async_trait]
pub trait DataProcessor: Send + Sync {
    /// Извлекает features из записи
    async fn extract_features(&self, record: &Record) -> Result<PromotionFeatures>;

    /// Подготавливает training data
    async fn prepare_training_data(&self) -> Result<Vec<TrainingExample>>;

    /// Нормализует features для ML модели
    fn normalize_features(&self, features: &mut PromotionFeatures);

    /// Обновляет usage tracking
    async fn update_usage_tracking(&self, record_id: &Uuid) -> Result<()>;
}

/// Trait для usage tracking
pub trait UsageTracker: Send + Sync {
    /// Записывает доступ к записи
    fn record_access(&mut self, record_id: &Uuid);

    /// Получает temporal pattern score
    fn get_temporal_pattern_score(&self, record_id: &Uuid) -> f32;

    /// Вычисляет access frequency
    fn calculate_access_frequency(&self, record: &Record) -> f32;

    /// Получает access recency score
    fn calculate_access_recency(&self, record: &Record) -> f32;
}

/// Trait для семантического анализа
#[async_trait]
pub trait SemanticAnalyzer: Send + Sync {
    /// Анализирует важность текста
    async fn analyze_importance(&self, text: &str) -> Result<f32>;

    /// Вычисляет keyword density
    fn calculate_keyword_density(&self, text: &str) -> f32;

    /// Получает topic relevance
    async fn get_topic_relevance(&self, text: &str) -> Result<f32>;

    /// Обновляет keyword weights
    fn update_keyword_weights(&mut self, keywords: Vec<(String, f32)>);
}

/// Пример для обучения ML модели
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub features: PromotionFeatures,
    pub label: f32,
}

/// Результаты ML inference
#[derive(Debug, Clone)]
pub struct InferenceResult {
    pub score: f32,
    pub inference_time_ms: u64,
    pub features_used: usize,
}

/// Конфигурация для различных алгоритмов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    pub learning_rate: f32,
    pub epochs: usize,
    pub batch_size: usize,
    pub l2_regularization: f32,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            epochs: 100,
            batch_size: 32,
            l2_regularization: 0.001,
        }
    }
}
