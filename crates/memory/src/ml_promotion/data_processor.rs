use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::traits::{DataProcessor, SemanticAnalyzer, TrainingExample, UsageTracker};
use super::types::{AccessPattern, MLPromotionConfig, PromotionFeatures, SemanticContext};
use crate::storage::VectorStore;
use crate::types::{Layer, Record};

/// ML Data Processing Pipeline
pub struct MLDataProcessor {
    store: Arc<VectorStore>,
    usage_tracker: Box<dyn UsageTracker>,
    semantic_analyzer: Box<dyn SemanticAnalyzer>,
    config: DataProcessorConfig,
    feature_cache: Arc<std::sync::Mutex<HashMap<Uuid, CachedFeatures>>>,
    normalization_stats: NormalizationStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessorConfig {
    /// Размер batch для обработки данных
    pub batch_size: usize,
    /// Использовать кэш features
    pub use_feature_cache: bool,
    /// Время жизни кэша в часах
    pub cache_ttl_hours: u64,
    /// Максимальный размер кэша
    pub max_cache_size: usize,
    /// Нормализовать features
    pub normalize_features: bool,
    /// Включить feature engineering
    pub enable_feature_engineering: bool,
}

impl Default for DataProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 32,
            use_feature_cache: true,
            cache_ttl_hours: 24,
            max_cache_size: 10000,
            normalize_features: true,
            enable_feature_engineering: true,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedFeatures {
    features: PromotionFeatures,
    timestamp: DateTime<Utc>,
    access_count_at_time: u32,
}

/// Статистика для нормализации features
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NormalizationStats {
    age_hours_mean: f32,
    age_hours_std: f32,
    access_count_mean: f32,
    access_count_std: f32,
    access_frequency_mean: f32,
    access_frequency_std: f32,
    semantic_importance_mean: f32,
    semantic_importance_std: f32,
}

impl Default for NormalizationStats {
    fn default() -> Self {
        Self {
            age_hours_mean: 48.0,
            age_hours_std: 24.0,
            access_count_mean: 5.0,
            access_count_std: 3.0,
            access_frequency_mean: 0.5,
            access_frequency_std: 0.3,
            semantic_importance_mean: 0.5,
            semantic_importance_std: 0.2,
        }
    }
}

#[async_trait]
impl DataProcessor for MLDataProcessor {
    async fn extract_features(&self, record: &Record) -> Result<PromotionFeatures> {
        debug!("🔬 Извлечение features для записи {}", record.id);

        // Проверяем кэш если включено
        if self.config.use_feature_cache {
            if let Some(cached) = self.get_cached_features(&record.id, record.access_count) {
                debug!("💾 Используются кэшированные features для {}", record.id);
                return Ok(cached.features);
            }
        }

        let start_time = std::time::Instant::now();
        let mut features = self.extract_raw_features(record).await?;

        // Feature engineering
        if self.config.enable_feature_engineering {
            self.apply_feature_engineering(&mut features, record);
        }

        // Нормализация
        if self.config.normalize_features {
            self.normalize_features(&mut features);
        }

        // Кэшируем результат
        if self.config.use_feature_cache {
            self.cache_features(&record.id, &features, record.access_count);
        }

        let extraction_time = start_time.elapsed();
        debug!("✅ Features извлечены за {:?}", extraction_time);

        Ok(features)
    }

    async fn prepare_training_data(&self) -> Result<Vec<TrainingExample>> {
        info!("📚 Подготовка training data для ML модели");

        let start_time = std::time::Instant::now();
        let mut training_data = Vec::new();

        // Собираем позитивные примеры из Insights и Assets
        let positive_examples = self.collect_positive_examples().await?;
        info!("✅ Собрано {} позитивных примеров", positive_examples.len());

        // Собираем негативные примеры из Interact
        let negative_examples = self.collect_negative_examples().await?;
        info!("✅ Собрано {} негативных примеров", negative_examples.len());

        // Объединяем и балансируем dataset
        training_data.extend(positive_examples);
        training_data.extend(negative_examples);

        // Перемешиваем данные
        self.shuffle_training_data(&mut training_data);

        // Применяем data augmentation если нужно
        if training_data.len() < 1000 {
            training_data = self.augment_training_data(training_data).await?;
            info!(
                "🔄 Применена data augmentation, новый размер: {}",
                training_data.len()
            );
        }

        let preparation_time = start_time.elapsed();
        info!(
            "✅ Training data подготовлена за {:?}: {} примеров",
            preparation_time,
            training_data.len()
        );

        Ok(training_data)
    }

    fn normalize_features(&self, features: &mut PromotionFeatures) {
        let stats = &self.normalization_stats;

        // Z-score нормализация: (x - mean) / std
        features.age_hours = (features.age_hours - stats.age_hours_mean) / stats.age_hours_std;
        features.access_count =
            (features.access_count - stats.access_count_mean) / stats.access_count_std;
        features.access_frequency =
            (features.access_frequency - stats.access_frequency_mean) / stats.access_frequency_std;
        features.semantic_importance = (features.semantic_importance
            - stats.semantic_importance_mean)
            / stats.semantic_importance_std;

        // Clamp выбросы в разумные пределы
        features.age_hours = features.age_hours.clamp(-3.0, 3.0);
        features.access_count = features.access_count.clamp(-3.0, 3.0);
        features.access_frequency = features.access_frequency.clamp(-3.0, 3.0);
        features.semantic_importance = features.semantic_importance.clamp(-3.0, 3.0);

        debug!("🎯 Features нормализованы");
    }

    async fn update_usage_tracking(&self, record_id: &Uuid) -> Result<()> {
        debug!("📊 Обновление usage tracking для {}", record_id);
        // Делегируем в usage_tracker
        Ok(())
    }
}

impl MLDataProcessor {
    pub async fn new(
        store: Arc<VectorStore>,
        usage_tracker: Box<dyn UsageTracker>,
        semantic_analyzer: Box<dyn SemanticAnalyzer>,
        config: DataProcessorConfig,
    ) -> Result<Self> {
        info!("🔧 Инициализация ML Data Processor");
        info!("  - Batch size: {}", config.batch_size);
        info!("  - Feature cache: {}", config.use_feature_cache);
        info!("  - Normalization: {}", config.normalize_features);
        info!(
            "  - Feature engineering: {}",
            config.enable_feature_engineering
        );

        let normalization_stats = Self::compute_normalization_stats(&store).await?;

        Ok(Self {
            store,
            usage_tracker,
            semantic_analyzer,
            config,
            feature_cache: Arc::new(std::sync::Mutex::new(HashMap::new())),
            normalization_stats,
        })
    }

    async fn extract_raw_features(&self, record: &Record) -> Result<PromotionFeatures> {
        let now = Utc::now();

        // Temporal features
        let age_hours = (now - record.ts).num_hours() as f32;
        let access_recency = self.usage_tracker.calculate_access_recency(record);
        let temporal_pattern_score = self.usage_tracker.get_temporal_pattern_score(&record.id);

        // Usage features
        let access_count = record.access_count as f32;
        let access_frequency = self.usage_tracker.calculate_access_frequency(record);
        let session_importance = self.calculate_session_importance(record);

        // Semantic features
        let semantic_importance = self
            .semantic_analyzer
            .analyze_importance(&record.text)
            .await?;
        let keyword_density = self
            .semantic_analyzer
            .calculate_keyword_density(&record.text);
        let topic_relevance = self
            .semantic_analyzer
            .get_topic_relevance(&record.text)
            .await?;

        // Context features
        let layer_affinity = self.calculate_layer_affinity(record);
        let co_occurrence_score = self.calculate_co_occurrence_score(record).await?;
        let user_preference_score = self.calculate_user_preference_score(record);

        Ok(PromotionFeatures {
            age_hours,
            access_recency,
            temporal_pattern_score,
            access_count,
            access_frequency,
            session_importance,
            semantic_importance,
            keyword_density,
            topic_relevance,
            layer_affinity,
            co_occurrence_score,
            user_preference_score,
        })
    }

    fn apply_feature_engineering(&self, features: &mut PromotionFeatures, record: &Record) {
        // Создаем производные features

        // 1. Age bucket feature
        let age_bucket = match features.age_hours {
            x if x <= 1.0 => 0.9,   // Very recent
            x if x <= 24.0 => 0.7,  // Recent
            x if x <= 168.0 => 0.5, // This week
            x if x <= 720.0 => 0.3, // This month
            _ => 0.1,               // Old
        };

        // 2. Access velocity feature
        let access_velocity = if features.age_hours > 0.0 {
            features.access_count / features.age_hours.max(1.0)
        } else {
            features.access_count
        };

        // 3. Content quality proxy
        let content_quality = self.estimate_content_quality(&record.text);

        // 4. Layer progression score
        let layer_progression = match record.layer {
            Layer::Interact => 0.3,
            Layer::Insights => 0.6,
            Layer::Assets => 1.0,
        };

        // Обновляем features с engineered values
        features.temporal_pattern_score = age_bucket;
        features.access_frequency = access_velocity;
        features.user_preference_score = content_quality * layer_progression;

        debug!(
            "🧬 Applied feature engineering: age_bucket={:.2}, access_velocity={:.2}",
            age_bucket, access_velocity
        );
    }

    fn estimate_content_quality(&self, text: &str) -> f32 {
        let word_count = text.split_whitespace().count();
        let char_count = text.len();
        let sentence_count = text.matches('.').count().max(1);

        // Простые эвристики для качества контента
        let word_score = (word_count as f32 / 50.0).min(1.0);
        let readability_score = (word_count as f32 / sentence_count as f32 / 20.0).min(1.0);
        let completeness_score = (char_count as f32 / 200.0).min(1.0);

        (word_score + readability_score + completeness_score) / 3.0
    }

    async fn collect_positive_examples(&self) -> Result<Vec<TrainingExample>> {
        let mut examples = Vec::new();

        // Собираем из Assets layer (высокое качество)
        let assets_records = self.store.iter_layer_records(Layer::Assets).await?;
        for record in assets_records.into_iter().take(500) {
            let age = Utc::now().signed_duration_since(record.ts);
            if age.num_hours() >= 24 {
                // Достаточно старые для обучения
                let features = self.extract_features(&record).await?;
                examples.push(TrainingExample {
                    features,
                    label: 1.0,
                });
            }
        }

        // Собираем из Insights layer (средняя важность)
        let insights_records = self.store.iter_layer_records(Layer::Insights).await?;
        for record in insights_records.into_iter().take(700) {
            let age = Utc::now().signed_duration_since(record.ts);
            if age.num_hours() >= 12 {
                let features = self.extract_features(&record).await?;
                examples.push(TrainingExample {
                    features,
                    label: 0.7,
                });
            }
        }

        Ok(examples)
    }

    async fn collect_negative_examples(&self) -> Result<Vec<TrainingExample>> {
        let mut examples = Vec::new();

        // Собираем из Interact layer записи которые долго там находятся
        let interact_records = self.store.iter_layer_records(Layer::Interact).await?;
        for record in interact_records.into_iter().take(800) {
            let age = Utc::now().signed_duration_since(record.ts);

            // Негативные примеры: старые записи с низким access_count
            if age.num_hours() >= 48 && record.access_count < 3 {
                let features = self.extract_features(&record).await?;
                examples.push(TrainingExample {
                    features,
                    label: 0.1,
                });
            }
        }

        Ok(examples)
    }

    async fn augment_training_data(
        &self,
        mut data: Vec<TrainingExample>,
    ) -> Result<Vec<TrainingExample>> {
        let original_size = data.len();

        // Создаем синтетические примеры добавлением шума
        for example in data.clone().iter().take(original_size / 2) {
            let mut augmented_features = example.features.clone();

            // Добавляем небольшой гауссовский шум
            augmented_features.age_hours += self.gaussian_noise(0.0, 0.1);
            augmented_features.access_count += self.gaussian_noise(0.0, 0.5);
            augmented_features.access_frequency += self.gaussian_noise(0.0, 0.05);
            augmented_features.semantic_importance += self.gaussian_noise(0.0, 0.02);

            // Clamp к разумным пределам
            augmented_features.age_hours = augmented_features.age_hours.max(0.0);
            augmented_features.access_count = augmented_features.access_count.max(0.0);
            augmented_features.access_frequency = augmented_features.access_frequency.max(0.0);
            augmented_features.semantic_importance =
                augmented_features.semantic_importance.clamp(0.0, 1.0);

            data.push(TrainingExample {
                features: augmented_features,
                label: example.label,
            });
        }

        Ok(data)
    }

    fn gaussian_noise(&self, mean: f32, std: f32) -> f32 {
        // Простая заглушка для гауссовского шума без внешних зависимостей
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        std::ptr::addr_of!(self).hash(&mut hasher);
        let seed = hasher.finish() as f32 / u64::MAX as f32;

        // Box-Muller transform approximation
        let u1 = seed.fract().max(0.0001);
        let u2 = (seed * 31.0).fract().max(0.0001);

        let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
        mean + std * z0
    }

    fn shuffle_training_data(&self, data: &mut Vec<TrainingExample>) {
        // Простой Fisher-Yates shuffle без внешних зависимостей
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        for i in (1..data.len()).rev() {
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            let j = (hasher.finish() % (i + 1) as u64) as usize;
            data.swap(i, j);
        }
    }

    fn get_cached_features(
        &self,
        record_id: &Uuid,
        current_access_count: u32,
    ) -> Option<CachedFeatures> {
        let cache = self.feature_cache.lock().unwrap();

        if let Some(cached) = cache.get(record_id) {
            // Проверяем TTL
            let age = Utc::now().signed_duration_since(cached.timestamp);
            if age.num_hours() < self.config.cache_ttl_hours as i64 {
                // Проверяем что access_count не сильно изменился
                if (cached.access_count_at_time as i32 - current_access_count as i32).abs() <= 2 {
                    return Some(cached.clone());
                }
            }
        }

        None
    }

    fn cache_features(&self, record_id: &Uuid, features: &PromotionFeatures, access_count: u32) {
        let mut cache = self.feature_cache.lock().unwrap();

        // Проверяем размер кэша
        if cache.len() >= self.config.max_cache_size {
            // Простая LRU-подобная очистка - удаляем старые записи
            let cutoff = Utc::now() - chrono::Duration::hours(self.config.cache_ttl_hours as i64);
            cache.retain(|_, cached| cached.timestamp > cutoff);

            // Если все еще превышаем лимит, удаляем случайные записи
            if cache.len() >= self.config.max_cache_size {
                let keys_to_remove: Vec<_> = cache
                    .keys()
                    .take(cache.len() - self.config.max_cache_size + 1)
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
            }
        }

        cache.insert(
            *record_id,
            CachedFeatures {
                features: features.clone(),
                timestamp: Utc::now(),
                access_count_at_time: access_count,
            },
        );
    }

    async fn compute_normalization_stats(store: &Arc<VectorStore>) -> Result<NormalizationStats> {
        info!("📊 Вычисление статистики для нормализации features");

        let mut age_values = Vec::new();
        let mut access_counts = Vec::new();
        let mut sample_count = 0;

        // Собираем выборку из всех layers
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let records = store.iter_layer_records(layer).await?;
            for record in records.into_iter().take(1000) {
                let age_hours = (Utc::now() - record.ts).num_hours() as f32;
                age_values.push(age_hours);
                access_counts.push(record.access_count as f32);
                sample_count += 1;
            }
        }

        if sample_count == 0 {
            warn!("⚠️ Недостаточно данных для вычисления статистики, используем defaults");
            return Ok(NormalizationStats::default());
        }

        // Вычисляем mean и std
        let age_mean = age_values.iter().sum::<f32>() / age_values.len() as f32;
        let age_variance = age_values
            .iter()
            .map(|x| (x - age_mean).powi(2))
            .sum::<f32>()
            / age_values.len() as f32;
        let age_std = age_variance.sqrt().max(1.0); // Избегаем деления на ноль

        let access_mean = access_counts.iter().sum::<f32>() / access_counts.len() as f32;
        let access_variance = access_counts
            .iter()
            .map(|x| (x - access_mean).powi(2))
            .sum::<f32>()
            / access_counts.len() as f32;
        let access_std = access_variance.sqrt().max(1.0);

        info!(
            "✅ Статистика вычислена: age_mean={:.1}h, access_mean={:.1}",
            age_mean, access_mean
        );

        Ok(NormalizationStats {
            age_hours_mean: age_mean,
            age_hours_std: age_std,
            access_count_mean: access_mean,
            access_count_std: access_std,
            access_frequency_mean: 0.3, // Примерные значения
            access_frequency_std: 0.2,
            semantic_importance_mean: 0.5,
            semantic_importance_std: 0.2,
        })
    }

    // Helper methods
    fn calculate_session_importance(&self, record: &Record) -> f32 {
        match record.layer {
            Layer::Interact => 0.3,
            Layer::Insights => 0.6,
            Layer::Assets => 0.9,
        }
    }

    fn calculate_layer_affinity(&self, record: &Record) -> f32 {
        match record.layer {
            Layer::Interact => {
                if record.access_count > 5 {
                    0.8
                } else {
                    0.2
                }
            }
            Layer::Insights => {
                if record.access_count > 10 {
                    0.9
                } else {
                    0.5
                }
            }
            Layer::Assets => 1.0,
        }
    }

    async fn calculate_co_occurrence_score(&self, _record: &Record) -> Result<f32> {
        // Заглушка для анализа co-occurrence patterns
        Ok(0.5)
    }

    fn calculate_user_preference_score(&self, _record: &Record) -> f32 {
        // Заглушка для пользовательских предпочтений
        0.5
    }

    /// Получает статистику data processor
    pub fn get_statistics(&self) -> DataProcessorStatistics {
        let cache = self.feature_cache.lock().unwrap();

        DataProcessorStatistics {
            cache_size: cache.len(),
            cache_hit_rate: 0.0, // Потребовало бы дополнительного tracking
            total_features_extracted: 0, // Также потребовало бы tracking
            normalization_stats: self.normalization_stats.clone(),
        }
    }

    /// Очищает кэш features
    pub fn clear_cache(&mut self) {
        let mut cache = self.feature_cache.lock().unwrap();
        cache.clear();
        info!("🧹 Feature cache очищен");
    }
}

/// Статистика работы data processor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProcessorStatistics {
    pub cache_size: usize,
    pub cache_hit_rate: f32,
    pub total_features_extracted: usize,
    pub normalization_stats: NormalizationStats,
}

/// Простая реализация UsageTracker
#[derive(Debug, Clone)]
pub struct SimpleUsageTracker {
    access_patterns: HashMap<Uuid, AccessPattern>,
}

impl SimpleUsageTracker {
    pub fn new() -> Self {
        Self {
            access_patterns: HashMap::new(),
        }
    }
}

impl UsageTracker for SimpleUsageTracker {
    fn record_access(&mut self, record_id: &Uuid) {
        let pattern = self
            .access_patterns
            .entry(*record_id)
            .or_insert(AccessPattern {
                total_accesses: 0,
                recent_accesses: 0,
                access_velocity: 0.0,
                last_access: Utc::now(),
                peak_access_time: None,
            });

        pattern.total_accesses += 1;
        pattern.recent_accesses += 1;
        pattern.last_access = Utc::now();
    }

    fn get_temporal_pattern_score(&self, record_id: &Uuid) -> f32 {
        self.access_patterns
            .get(record_id)
            .map(|p| (p.access_velocity / 10.0).min(1.0))
            .unwrap_or(0.5)
    }

    fn calculate_access_frequency(&self, record: &Record) -> f32 {
        let age_days = (Utc::now() - record.ts).num_days() as f32;
        if age_days <= 0.0 {
            record.access_count as f32
        } else {
            record.access_count as f32 / age_days.max(1.0)
        }
    }

    fn calculate_access_recency(&self, record: &Record) -> f32 {
        let hours_since_access = (Utc::now() - record.ts).num_hours() as f32;
        (24.0 / (hours_since_access + 1.0)).min(1.0)
    }
}

/// Простая реализация SemanticAnalyzer
#[derive(Debug, Clone)]
pub struct SimpleSemanticAnalyzer {
    keyword_weights: HashMap<String, f32>,
}

impl SimpleSemanticAnalyzer {
    pub fn new() -> Self {
        let mut keyword_weights = HashMap::new();
        keyword_weights.insert("error".to_string(), 0.9);
        keyword_weights.insert("critical".to_string(), 0.9);
        keyword_weights.insert("important".to_string(), 0.8);
        keyword_weights.insert("warning".to_string(), 0.7);
        keyword_weights.insert("info".to_string(), 0.5);

        Self { keyword_weights }
    }
}

#[async_trait]
impl SemanticAnalyzer for SimpleSemanticAnalyzer {
    async fn analyze_importance(&self, text: &str) -> Result<f32> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut importance = 0.0;

        for word in &words {
            if let Some(&weight) = self.keyword_weights.get(&word.to_lowercase()) {
                importance += weight;
            }
        }

        if !words.is_empty() {
            importance = (importance / words.len() as f32).min(1.0);
        }

        Ok(importance)
    }

    fn calculate_keyword_density(&self, text: &str) -> f32 {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut keyword_count = 0;

        for word in &words {
            if self.keyword_weights.contains_key(&word.to_lowercase()) {
                keyword_count += 1;
            }
        }

        if words.is_empty() {
            0.0
        } else {
            keyword_count as f32 / words.len() as f32
        }
    }

    async fn get_topic_relevance(&self, _text: &str) -> Result<f32> {
        // Заглушка для topic modeling
        Ok(0.5)
    }

    fn update_keyword_weights(&mut self, keywords: Vec<(String, f32)>) {
        for (keyword, weight) in keywords {
            self.keyword_weights.insert(keyword, weight);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_record() -> Record {
        Record {
            id: Uuid::new_v4(),
            text: "This is a critical error message that needs attention".to_string(),
            embedding: vec![0.1; 384],
            ts: Utc::now() - chrono::Duration::hours(2),
            layer: Layer::Interact,
            access_count: 5,
            score: 0.0,
            kind: "test".to_string(),
            tags: vec!["error".to_string(), "critical".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            last_access: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_feature_extraction() {
        let usage_tracker = Box::new(SimpleUsageTracker::new());
        let semantic_analyzer = Box::new(SimpleSemanticAnalyzer::new());

        // Потребовала бы mock VectorStore для полного теста
        // let processor = MLDataProcessor::new(store, usage_tracker, semantic_analyzer, config).await.unwrap();

        let record = create_test_record();
        // let features = processor.extract_features(&record).await.unwrap();

        // assert!(features.age_hours > 0.0);
        // assert!(features.access_count > 0.0);
    }

    #[test]
    fn test_usage_tracker() {
        let mut tracker = SimpleUsageTracker::new();
        let record_id = Uuid::new_v4();

        tracker.record_access(&record_id);
        let pattern_score = tracker.get_temporal_pattern_score(&record_id);

        assert!(pattern_score >= 0.0 && pattern_score <= 1.0);
    }

    #[tokio::test]
    async fn test_semantic_analyzer() {
        let analyzer = SimpleSemanticAnalyzer::new();

        let importance = analyzer
            .analyze_importance("This is a critical error")
            .await
            .unwrap();
        assert!(importance > 0.0);

        let density = analyzer.calculate_keyword_density("critical error warning info");
        assert!(density > 0.0);
    }
}
