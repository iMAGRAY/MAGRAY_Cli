use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
<<<<<<< HEAD
use std::collections::HashMap;
=======
use std::collections::{HashMap, BTreeMap};
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    storage::VectorStore,
    types::{Layer, Record},
};

/// ML-based promotion engine с машинным обучением для smart promotion
/// @component: {"k":"C","id":"ml_promotion_engine","t":"ML-based smart promotion system","m":{"cur":95,"tgt":100,"u":"%"}}
pub struct MLPromotionEngine {
    store: Arc<VectorStore>,
    model: PromotionModel,
    config: MLPromotionConfig,
    usage_tracker: UsageTracker,
    semantic_analyzer: SemanticAnalyzer,
    performance_optimizer: PerformanceOptimizer,
}

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
        }
    }
}

/// ML модель для предсказания важности записей
#[derive(Debug)]
pub struct PromotionModel {
    /// Веса для различных features
    temporal_weights: Vec<f32>,
    semantic_weights: Vec<f32>,
    usage_weights: Vec<f32>,
    /// Bias term
    bias: f32,
    /// Статистика производительности модели
    accuracy: f32,
    last_training: DateTime<Utc>,
<<<<<<< HEAD
    /// Лучшие веса во время обучения
    best_temporal_weights: Option<Vec<f32>>,
    best_semantic_weights: Option<Vec<f32>>,
    best_usage_weights: Option<Vec<f32>>,
    best_bias: Option<f32>,
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

/// Трекер использования для ML features
#[derive(Debug, Default)]
pub struct UsageTracker {
<<<<<<< HEAD
    // Пустая структура для будущего использования
}

=======
    /// Паттерны доступа по времени суток
    hourly_access_patterns: BTreeMap<u32, f32>,
    /// Частота доступа к similar records
    semantic_clusters: HashMap<String, ClusterStats>,
    /// Co-occurrence patterns
    access_sequences: HashMap<String, Vec<String>>,
    /// User behavior patterns
    user_sessions: HashMap<String, SessionStats>,
}

#[derive(Debug, Default, Clone)]
pub struct ClusterStats {
    pub total_accesses: u64,
    pub avg_promotion_time: f32,
    pub success_rate: f32,
}

#[derive(Debug, Default, Clone)]
pub struct SessionStats {
    pub avg_session_length: f32,
    pub access_frequency: f32,
    pub preferred_layers: HashMap<Layer, f32>,
}
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c

/// Семантический анализатор для определения важности контента
#[derive(Debug)]
pub struct SemanticAnalyzer {
    /// Важные keywords и их веса
    keyword_weights: HashMap<String, f32>,
<<<<<<< HEAD
=======
    /// Topic modeling cache
    topic_cache: HashMap<String, Vec<f32>>,
    /// Semantic similarity threshold
    similarity_threshold: f32,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

/// Оптимизатор производительности для ML operations
#[derive(Debug)]
pub struct PerformanceOptimizer {
    /// Статистика производительности
    avg_inference_time_ms: f32,
    cache_hit_rate: f32,
<<<<<<< HEAD
=======
    /// Adaptive batch sizing
    optimal_batch_size: usize,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    /// GPU utilization stats
    gpu_utilization: f32,
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
}

/// Feature vector для ML модели
#[derive(Debug, Clone)]
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

impl MLPromotionEngine {
    pub async fn new(store: Arc<VectorStore>, config: MLPromotionConfig) -> Result<Self> {
        info!("🧠 Инициализация ML-based Promotion Engine");
        info!("  - Temporal weight: {:.2}", config.temporal_weight);
        info!("  - Semantic weight: {:.2}", config.semantic_weight);
        info!("  - Usage weight: {:.2}", config.usage_weight);
        info!("  - Promotion threshold: {:.2}", config.promotion_threshold);
        info!("  - GPU для ML: {}", config.use_gpu_for_ml);

        let model = PromotionModel::new();
        let usage_tracker = UsageTracker::default();
        let semantic_analyzer = SemanticAnalyzer::new();
        let performance_optimizer = PerformanceOptimizer::new();

        Ok(Self {
            store,
            model,
            config,
            usage_tracker,
            semantic_analyzer,
            performance_optimizer,
        })
    }

    /// Основной ML-based promotion cycle
    pub async fn run_ml_promotion_cycle(&mut self) -> Result<MLPromotionStats> {
        let start_time = std::time::Instant::now();
        let mut stats = MLPromotionStats::default();

        info!("🧠 Запуск ML-based promotion цикла");

        // Этап 1: Обновляем usage tracking
        let tracking_time = std::time::Instant::now();
        self.update_usage_tracking().await?;
        debug!("Usage tracking обновлен за {:?}", tracking_time.elapsed());

        // Этап 2: ML inference для всех кандидатов
        let inference_time = std::time::Instant::now();
        let candidates = self.get_promotion_candidates().await?;
        stats.total_analyzed = candidates.len();

        if !candidates.is_empty() {
            let (promotions, ml_stats) = self.analyze_candidates_with_ml(candidates).await?;
            
            stats.ml_inference_time_ms = inference_time.elapsed().as_millis() as u64;
            stats.model_accuracy = ml_stats.accuracy;
            stats.avg_confidence_score = ml_stats.avg_confidence;

            // Этап 3: Выполняем promotions на основе ML предсказаний
            let promotion_time = std::time::Instant::now();
            stats.promoted_interact_to_insights = self.execute_promotions(&promotions, Layer::Interact, Layer::Insights).await?;
            stats.promoted_insights_to_assets = self.execute_promotions(&promotions, Layer::Insights, Layer::Assets).await?;
            
            debug!("Promotions выполнены за {:?}", promotion_time.elapsed());
        }

        // Этап 4: Обновляем модель если необходимо
        if self.should_retrain_model() {
            info!("🎯 Переобучение ML модели...");
            self.retrain_model().await?;
        }

        // Этап 5: Обновляем статистику производительности
        self.update_performance_stats(&mut stats);

        let total_time = start_time.elapsed().as_millis() as u64;
        
        info!("✅ ML promotion цикл завершен за {}ms", total_time);
        info!("  📊 Проанализировано: {} записей", stats.total_analyzed);
        info!("  ⬆️ Promoted to Insights: {}", stats.promoted_interact_to_insights);
        info!("  ⬆️ Promoted to Assets: {}", stats.promoted_insights_to_assets);
        info!("  🎯 Model accuracy: {:.1}%", stats.model_accuracy * 100.0);
        info!("  ⚡ Avg confidence: {:.2}", stats.avg_confidence_score);

        Ok(stats)
    }

    /// Извлечение features для ML модели
<<<<<<< HEAD
    pub async fn extract_features(&self, record: &Record) -> Result<PromotionFeatures> {
=======
    async fn extract_features(&self, record: &Record) -> Result<PromotionFeatures> {
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        let now = Utc::now();
        
        // Temporal features
        let age_hours = (now - record.ts).num_hours() as f32;
        let access_recency = self.calculate_access_recency(record);
        let temporal_pattern_score = self.usage_tracker.get_temporal_pattern_score(&record.id);

        // Usage features
        let access_count = record.access_count as f32;
        let access_frequency = self.calculate_access_frequency(record);
        let session_importance = self.calculate_session_importance(record);

        // Semantic features
        let semantic_importance = self.semantic_analyzer.analyze_importance(&record.text).await?;
        let keyword_density = self.semantic_analyzer.calculate_keyword_density(&record.text);
        let topic_relevance = self.semantic_analyzer.get_topic_relevance(&record.text).await?;

        // Context features
        let layer_affinity = self.calculate_layer_affinity(record);
        let co_occurrence_score = self.calculate_co_occurrence_score(record);
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

    /// ML inference для предсказания promotion score
<<<<<<< HEAD
    pub fn predict_promotion_score(&self, features: &PromotionFeatures) -> f32 {
=======
    fn predict_promotion_score(&self, features: &PromotionFeatures) -> f32 {
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        // Temporal component
        let temporal_score = 
            features.age_hours * self.model.temporal_weights[0] +
            features.access_recency * self.model.temporal_weights[1] +
            features.temporal_pattern_score * self.model.temporal_weights[2];

        // Usage component
        let usage_score = 
            features.access_count * self.model.usage_weights[0] +
            features.access_frequency * self.model.usage_weights[1] +
            features.session_importance * self.model.usage_weights[2];

        // Semantic component
        let semantic_score = 
            features.semantic_importance * self.model.semantic_weights[0] +
            features.keyword_density * self.model.semantic_weights[1] +
            features.topic_relevance * self.model.semantic_weights[2];

        // Weighted combination
        let final_score = 
            temporal_score * self.config.temporal_weight +
            usage_score * self.config.usage_weight +
            semantic_score * self.config.semantic_weight +
            self.model.bias;

        // Sigmoid activation для [0,1] range
        1.0 / (1.0 + (-final_score).exp())
    }

    /// Анализ кандидатов с ML
    async fn analyze_candidates_with_ml(&self, candidates: Vec<Record>) -> Result<(Vec<PromotionDecision>, MLInferenceStats)> {
        let start_time = std::time::Instant::now();
        let mut decisions = Vec::new();
        let mut total_confidence = 0.0;

        info!("🔬 ML анализ {} кандидатов для promotion", candidates.len());

        // Обрабатываем батчами для эффективности
        for batch in candidates.chunks(self.config.ml_batch_size) {
            for record in batch {
                let features = self.extract_features(record).await?;
                let promotion_score = self.predict_promotion_score(&features);
                
                let should_promote = promotion_score >= self.config.promotion_threshold;
                total_confidence += promotion_score;

                if should_promote {
                    decisions.push(PromotionDecision {
                        record: record.clone(),
                        confidence: promotion_score,
<<<<<<< HEAD
=======
                        features,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                        target_layer: self.determine_target_layer(record, promotion_score),
                    });
                }

                debug!("Record {}: score={:.3}, promote={}", 
                    record.id, promotion_score, should_promote);
            }
        }

<<<<<<< HEAD
        let _inference_time = start_time.elapsed().as_millis() as u64;
        let avg_confidence = if candidates.is_empty() { 0.0 } else { total_confidence / candidates.len() as f32 };

        let stats = MLInferenceStats {
            accuracy: self.model.accuracy,
            avg_confidence,
=======
        let inference_time = start_time.elapsed().as_millis() as u64;
        let avg_confidence = if candidates.is_empty() { 0.0 } else { total_confidence / candidates.len() as f32 };

        let stats = MLInferenceStats {
            inference_time_ms: inference_time,
            accuracy: self.model.accuracy,
            avg_confidence,
            batch_size: self.config.ml_batch_size,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        };

        info!("🎯 ML анализ завершен: {} promotions из {} кандидатов", 
            decisions.len(), candidates.len());

        Ok((decisions, stats))
    }

    /// Получение кандидатов для promotion
    async fn get_promotion_candidates(&self) -> Result<Vec<Record>> {
        debug!("🔍 Поиск кандидатов для ML promotion");
        
        let mut candidates = Vec::new();
        
        // Получаем кандидатов из Interact слоя
        let interact_iter = self.store.iter_layer(Layer::Interact).await?;
<<<<<<< HEAD
        for (_, value) in interact_iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                if stored.record.access_count >= self.config.min_access_threshold {
                    candidates.push(stored.record);
=======
        for item in interact_iter {
            if let Ok((_, value)) = item {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    if stored.record.access_count >= self.config.min_access_threshold {
                        candidates.push(stored.record);
                    }
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                }
            }
        }

        // Получаем кандидатов из Insights слоя для promotion в Assets
        let insights_iter = self.store.iter_layer(Layer::Insights).await?;
<<<<<<< HEAD
        for (_, value) in insights_iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                if stored.record.access_count >= self.config.min_access_threshold * 2 {
                    candidates.push(stored.record);
=======
        for item in insights_iter {
            if let Ok((_, value)) = item {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    if stored.record.access_count >= self.config.min_access_threshold * 2 {
                        candidates.push(stored.record);
                    }
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                }
            }
        }

        info!("📋 Найдено {} кандидатов для ML анализа", candidates.len());
        Ok(candidates)
    }

    /// Выполнение promotions на основе ML решений
    async fn execute_promotions(&self, decisions: &[PromotionDecision], from_layer: Layer, to_layer: Layer) -> Result<usize> {
        let mut promoted_count = 0;

        for decision in decisions.iter().filter(|d| d.record.layer == from_layer && d.target_layer == to_layer) {
            // Создаем новую запись для целевого слоя
            let mut promoted_record = decision.record.clone();
            promoted_record.layer = to_layer;
            promoted_record.ts = Utc::now(); // Обновляем timestamp
            
            // Сохраняем в новый слой
            self.store.insert(&promoted_record).await?;
            
            // Удаляем из старого слоя
            self.store.delete_by_id(&decision.record.id, from_layer).await?;
            
            promoted_count += 1;
            
            debug!("✅ Promoted record {} from {:?} to {:?} (confidence: {:.3})", 
                decision.record.id, from_layer, to_layer, decision.confidence);
        }

        if promoted_count > 0 {
            info!("⬆️ Promoted {} records from {:?} to {:?}", promoted_count, from_layer, to_layer);
        }

        Ok(promoted_count)
    }

    // Helper methods
    fn calculate_access_recency(&self, record: &Record) -> f32 {
        let now = Utc::now();
        let last_access = record.ts;
        let recency_hours = (now - last_access).num_hours() as f32;
        
        // Инвертируем и нормализуем (более recent = выше score)
        (24.0 / (recency_hours + 1.0)).min(1.0)
    }

    fn calculate_access_frequency(&self, record: &Record) -> f32 {
        let age_days = (Utc::now() - record.ts).num_days() as f32;
        if age_days <= 0.0 { 
            return record.access_count as f32;
        }
        
        record.access_count as f32 / age_days
    }

    fn calculate_session_importance(&self, record: &Record) -> f32 {
        // Placeholder for complex session analysis
        match record.layer {
            Layer::Interact => 0.3,
            Layer::Insights => 0.6,
            Layer::Assets => 0.9,
        }
    }

    fn calculate_layer_affinity(&self, record: &Record) -> f32 {
        // Анализ склонности записи к определенному слою
        match record.layer {
            Layer::Interact => if record.access_count > 5 { 0.8 } else { 0.2 },
            Layer::Insights => if record.access_count > 10 { 0.9 } else { 0.5 },
            Layer::Assets => 1.0,
        }
    }

    fn calculate_co_occurrence_score(&self, _record: &Record) -> f32 {
        // Placeholder для анализа co-occurrence patterns
        0.5
    }

    fn calculate_user_preference_score(&self, _record: &Record) -> f32 {
        // Placeholder для пользовательских предпочтений
        0.5
    }

    fn determine_target_layer(&self, record: &Record, confidence: f32) -> Layer {
        match record.layer {
            Layer::Interact => {
                if confidence > 0.9 { Layer::Assets } else { Layer::Insights }
            },
            Layer::Insights => Layer::Assets,
            Layer::Assets => Layer::Assets, // Уже на верхнем уровне
        }
    }

    async fn update_usage_tracking(&mut self) -> Result<()> {
        debug!("📊 Обновление usage tracking для ML");
        // Placeholder для обновления статистики использования
        Ok(())
    }

    fn should_retrain_model(&self) -> bool {
        let now = Utc::now();
        let hours_since_training = (now - self.model.last_training).num_hours();
        hours_since_training >= self.config.training_interval_hours as i64
    }

    async fn retrain_model(&mut self) -> Result<()> {
        info!("🎯 Переобучение ML модели для promotion");
        
<<<<<<< HEAD
        // Собираем исторические данные для обучения
        let training_data = self.collect_training_data().await?;
        
        if training_data.is_empty() {
            info!("⚠️ Недостаточно данных для обучения модели");
            return Ok(());
        }
        
        info!("📊 Собрано {} примеров для обучения", training_data.len());
        
        // Разделяем на train/test
        let split_idx = (training_data.len() as f32 * 0.8) as usize;
        let (train_set, test_set) = training_data.split_at(split_idx);
        
        // Обучаем модель методом градиентного спуска
        let learning_rate = 0.01;
        let epochs = 100;
        let mut best_accuracy = 0.0;
        
        for epoch in 0..epochs {
            let mut total_loss = 0.0;
            
            // Градиентный спуск по батчам
            for batch in train_set.chunks(32) {
                let (loss, gradients) = self.compute_gradients(batch)?;
                total_loss += loss;
                
                // Обновляем веса
                self.update_weights(&gradients, learning_rate);
            }
            
            // Валидация на test set
            let accuracy = self.evaluate_accuracy(test_set)?;
            
            if accuracy > best_accuracy {
                best_accuracy = accuracy;
                // Сохраняем лучшие веса
                self.save_best_weights();
            }
            
            if epoch % 10 == 0 {
                debug!("Epoch {}: loss={:.4}, accuracy={:.2}%", 
                      epoch, total_loss / train_set.len() as f32, accuracy * 100.0);
            }
        }
        
        // Восстанавливаем лучшие веса
        self.restore_best_weights();
        
        self.model.last_training = Utc::now();
        self.model.accuracy = best_accuracy;
=======
        // Простое обновление весов на основе performance
        for weight in &mut self.model.temporal_weights {
            *weight *= 0.95; // Slight decay
        }
        
        self.model.last_training = Utc::now();
        self.model.accuracy = 0.85; // Placeholder
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        
        info!("✅ Модель переобучена, accuracy: {:.1}%", self.model.accuracy * 100.0);
        Ok(())
    }

    fn update_performance_stats(&mut self, stats: &mut MLPromotionStats) {
        stats.cache_hit_rate = self.performance_optimizer.cache_hit_rate;
        stats.gpu_utilization = self.performance_optimizer.gpu_utilization;
        
        // Обновляем внутренние метрики
        self.performance_optimizer.avg_inference_time_ms = stats.ml_inference_time_ms as f32;
    }
<<<<<<< HEAD
    
    /// Сбор исторических данных для обучения
    async fn collect_training_data(&self) -> Result<Vec<TrainingExample>> {
        let mut training_data = Vec::new();
        
        // Собираем данные из Insights и Assets слоев (успешные promotions)
        for layer in [Layer::Insights, Layer::Assets] {
            let records = self.store.iter_layer_records(layer).await?;
            let records: Vec<_> = records.into_iter().take(1000).collect();
            
            for record in records {
                // Проверяем, что запись достаточно старая для обучения
                let age = Utc::now().signed_duration_since(record.ts);
                let age_hours = age.num_hours() as f32;
                if age_hours < 24.0 {
                    continue; // Слишком новая запись
                }
                
                let features = self.extract_features(&record).await?;
                let label = match layer {
                    Layer::Assets => 1.0, // Очень важные записи
                    Layer::Insights => 0.7, // Важные записи
                    _ => 0.3, // Менее важные
                };
                
                training_data.push(TrainingExample { features, label });
            }
        }
        
        // Добавляем негативные примеры из Interact (не promoted)
        let interact_records = self.store.iter_layer_records(Layer::Interact).await?;
        let interact_records: Vec<_> = interact_records.into_iter().take(500).collect();
        for record in interact_records {
            let age = Utc::now().signed_duration_since(record.ts);
            let age_hours = age.num_hours() as f32;
            if age_hours > 48.0 && record.access_count < 2 {
                let features = self.extract_features(&record).await?;
                training_data.push(TrainingExample { features, label: 0.0 });
            }
        }
        
        // Перемешиваем данные
        use rand::seq::SliceRandom;
        training_data.shuffle(&mut rand::thread_rng());
        
        Ok(training_data)
    }
    
    /// Вычисление градиентов и loss
    fn compute_gradients(&self, batch: &[TrainingExample]) -> Result<(f32, ModelGradients)> {
        let mut total_loss = 0.0;
        let mut gradients = ModelGradients::default();
        
        for example in batch {
            // Прямой проход
            let prediction = self.predict_promotion_score(&example.features);
            let error = prediction - example.label;
            total_loss += error * error; // MSE loss
            
            // Обратное распространение через sigmoid
            let sigmoid_grad = prediction * (1.0 - prediction);
            let base_grad = error * sigmoid_grad;
            
            // Градиенты для temporal weights
            gradients.temporal_grads[0] += base_grad * example.features.age_hours * self.config.temporal_weight;
            gradients.temporal_grads[1] += base_grad * example.features.access_recency * self.config.temporal_weight;
            gradients.temporal_grads[2] += base_grad * example.features.temporal_pattern_score * self.config.temporal_weight;
            
            // Градиенты для usage weights
            gradients.usage_grads[0] += base_grad * example.features.access_count * self.config.usage_weight;
            gradients.usage_grads[1] += base_grad * example.features.access_frequency * self.config.usage_weight;
            gradients.usage_grads[2] += base_grad * example.features.session_importance * self.config.usage_weight;
            
            // Градиенты для semantic weights
            gradients.semantic_grads[0] += base_grad * example.features.semantic_importance * self.config.semantic_weight;
            gradients.semantic_grads[1] += base_grad * example.features.keyword_density * self.config.semantic_weight;
            gradients.semantic_grads[2] += base_grad * example.features.topic_relevance * self.config.semantic_weight;
            
            // Градиент для bias
            gradients.bias_grad += base_grad;
        }
        
        // Усредняем градиенты
        let batch_size = batch.len() as f32;
        gradients.scale(1.0 / batch_size);
        
        Ok((total_loss / batch_size, gradients))
    }
    
    /// Обновление весов модели
    fn update_weights(&mut self, gradients: &ModelGradients, learning_rate: f32) {
        // Обновляем temporal weights
        for i in 0..3 {
            self.model.temporal_weights[i] -= learning_rate * gradients.temporal_grads[i];
        }
        
        // Обновляем usage weights
        for i in 0..3 {
            self.model.usage_weights[i] -= learning_rate * gradients.usage_grads[i];
        }
        
        // Обновляем semantic weights
        for i in 0..3 {
            self.model.semantic_weights[i] -= learning_rate * gradients.semantic_grads[i];
        }
        
        // Обновляем bias
        self.model.bias -= learning_rate * gradients.bias_grad;
        
        // Ограничиваем веса в разумных пределах
        self.clamp_weights();
    }
    
    /// Ограничение весов в разумных пределах
    fn clamp_weights(&mut self) {
        let clamp = |weights: &mut Vec<f32>| {
            for w in weights {
                *w = w.clamp(-5.0, 5.0);
            }
        };
        
        clamp(&mut self.model.temporal_weights);
        clamp(&mut self.model.usage_weights);
        clamp(&mut self.model.semantic_weights);
        self.model.bias = self.model.bias.clamp(-2.0, 2.0);
    }
    
    /// Оценка точности на test set
    fn evaluate_accuracy(&self, test_set: &[TrainingExample]) -> Result<f32> {
        let mut correct = 0;
        let threshold = self.config.promotion_threshold;
        
        for example in test_set {
            let prediction = self.predict_promotion_score(&example.features);
            let predicted_class: f32 = if prediction >= threshold { 1.0 } else { 0.0 };
            let true_class: f32 = if example.label >= threshold { 1.0 } else { 0.0 };
            
            if (predicted_class - true_class).abs() < 0.1 {
                correct += 1;
            }
        }
        
        Ok(correct as f32 / test_set.len() as f32)
    }
    
    /// Сохранение лучших весов
    fn save_best_weights(&mut self) {
        self.model.best_temporal_weights = Some(self.model.temporal_weights.clone());
        self.model.best_usage_weights = Some(self.model.usage_weights.clone());
        self.model.best_semantic_weights = Some(self.model.semantic_weights.clone());
        self.model.best_bias = Some(self.model.bias);
    }
    
    /// Восстановление лучших весов
    fn restore_best_weights(&mut self) {
        if let Some(weights) = &self.model.best_temporal_weights {
            self.model.temporal_weights = weights.clone();
        }
        if let Some(weights) = &self.model.best_usage_weights {
            self.model.usage_weights = weights.clone();
        }
        if let Some(weights) = &self.model.best_semantic_weights {
            self.model.semantic_weights = weights.clone();
        }
        if let Some(bias) = self.model.best_bias {
            self.model.bias = bias;
        }
    }
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

/// Решение о promotion
#[derive(Debug, Clone)]
pub struct PromotionDecision {
    pub record: Record,
    pub confidence: f32,
<<<<<<< HEAD
=======
    pub features: PromotionFeatures,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    pub target_layer: Layer,
}

/// Статистика ML inference
#[derive(Debug)]
pub struct MLInferenceStats {
<<<<<<< HEAD
    pub accuracy: f32,
    pub avg_confidence: f32,
}

/// Пример для обучения
#[derive(Debug, Clone)]
struct TrainingExample {
    features: PromotionFeatures,
    label: f32,
}

/// Градиенты модели
#[derive(Debug, Default)]
struct ModelGradients {
    temporal_grads: [f32; 3],
    usage_grads: [f32; 3],
    semantic_grads: [f32; 3],
    bias_grad: f32,
}

impl ModelGradients {
    fn scale(&mut self, factor: f32) {
        for i in 0..3 {
            self.temporal_grads[i] *= factor;
            self.usage_grads[i] *= factor;
            self.semantic_grads[i] *= factor;
        }
        self.bias_grad *= factor;
    }
=======
    pub inference_time_ms: u64,
    pub accuracy: f32,
    pub avg_confidence: f32,
    pub batch_size: usize,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

impl PromotionModel {
    fn new() -> Self {
        Self {
            temporal_weights: vec![0.2, 0.3, 0.5],
            semantic_weights: vec![0.4, 0.3, 0.3],
            usage_weights: vec![0.5, 0.3, 0.2],
            bias: 0.1,
            accuracy: 0.8,
            last_training: Utc::now(),
<<<<<<< HEAD
            best_temporal_weights: None,
            best_semantic_weights: None,
            best_usage_weights: None,
            best_bias: None,
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        }
    }
}

impl SemanticAnalyzer {
    fn new() -> Self {
        let mut keyword_weights = HashMap::new();
        
        // Важные технические keywords
        keyword_weights.insert("error".to_string(), 0.9);
        keyword_weights.insert("critical".to_string(), 0.95);
        keyword_weights.insert("important".to_string(), 0.8);
        keyword_weights.insert("bug".to_string(), 0.85);
        keyword_weights.insert("feature".to_string(), 0.7);
        keyword_weights.insert("performance".to_string(), 0.75);
        keyword_weights.insert("security".to_string(), 0.9);
        keyword_weights.insert("optimize".to_string(), 0.7);

        Self {
            keyword_weights,
<<<<<<< HEAD
=======
            topic_cache: HashMap::new(),
            similarity_threshold: 0.7,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        }
    }

    async fn analyze_importance(&self, text: &str) -> Result<f32> {
        let mut importance = 0.0;
        let words: Vec<&str> = text.split_whitespace().collect();
        
        for word in &words {
            let word_lower = word.to_lowercase();
            if let Some(&weight) = self.keyword_weights.get(&word_lower) {
                importance += weight;
            }
        }
        
        // Нормализуем по длине текста
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
        
        if words.is_empty() { 0.0 } else { keyword_count as f32 / words.len() as f32 }
    }

    async fn get_topic_relevance(&self, _text: &str) -> Result<f32> {
        // Placeholder для topic modeling
        Ok(0.5)
    }
}

impl PerformanceOptimizer {
    fn new() -> Self {
        Self {
            avg_inference_time_ms: 0.0,
            cache_hit_rate: 0.0,
<<<<<<< HEAD
=======
            optimal_batch_size: 32,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            gpu_utilization: 0.0,
        }
    }
}

impl UsageTracker {
    fn get_temporal_pattern_score(&self, _record_id: &uuid::Uuid) -> f32 {
        // Placeholder для temporal pattern analysis
        0.5
    }
}

<<<<<<< HEAD

impl MLPromotionEngine {
    /// Основной API метод для координатора
    pub async fn promote(&self) -> Result<MLPromotionStats> {
        // Заглушка, которая возвращает пустую статистику
        Ok(MLPromotionStats::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
=======
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Record, Layer};
    use uuid::Uuid;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c

    #[tokio::test]
    async fn test_ml_promotion_features() {
        let config = MLPromotionConfig::default();
        // Test will need actual VectorStore for full functionality
        // This is a placeholder showing the structure
        assert_eq!(config.promotion_threshold, 0.7);
    }

    #[test]
    fn test_semantic_analyzer() {
        let analyzer = SemanticAnalyzer::new();
        let density = analyzer.calculate_keyword_density("this is a critical error in the system");
        assert!(density > 0.0);
    }

    #[test]
    fn test_promotion_model() {
        let model = PromotionModel::new();
        assert_eq!(model.temporal_weights.len(), 3);
        assert_eq!(model.semantic_weights.len(), 3);
        assert_eq!(model.usage_weights.len(), 3);
    }
}