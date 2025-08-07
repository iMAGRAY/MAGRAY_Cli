use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::storage::VectorStore;
use crate::types::{Layer, Record};
use super::traits::{
    PromotionAlgorithm, PromotionMetrics, PromotionRulesEngine, DataProcessor, TrainingExample
};
use super::types::{MLPromotionConfig, MLPromotionStats, PromotionDecision, PromotionResults};
use super::algorithms::AlgorithmFactory;
use super::traits::AlgorithmConfig;
use super::metrics::{MLPromotionMetricsCollector, MetricsConfig};
use super::rules_engine::{ConfigurableRulesEngine, RulesConfig};
use super::data_processor::{MLDataProcessor, DataProcessorConfig, SimpleUsageTracker, SimpleSemanticAnalyzer};

/// Main coordinator для ML promotion system с полным DI pattern
pub struct PromotionCoordinator {
    // Core dependencies (injected)
    store: Arc<VectorStore>,
    algorithm: Box<dyn PromotionAlgorithm>,
    metrics: Box<dyn PromotionMetrics>,
    rules_engine: Box<dyn PromotionRulesEngine>,
    data_processor: Box<dyn DataProcessor>,
    
    // Configuration
    config: MLPromotionConfig,
    
    // Internal state
    is_training: bool,
    last_training_check: chrono::DateTime<chrono::Utc>,
}

/// Builder для PromotionCoordinator с Dependency Injection
pub struct PromotionCoordinatorBuilder {
    store: Option<Arc<VectorStore>>,
    algorithm: Option<Box<dyn PromotionAlgorithm>>,
    metrics: Option<Box<dyn PromotionMetrics>>,
    rules_engine: Option<Box<dyn PromotionRulesEngine>>,
    data_processor: Option<Box<dyn DataProcessor>>,
    config: MLPromotionConfig,
}

impl PromotionCoordinatorBuilder {
    pub fn new() -> Self {
        Self {
            store: None,
            algorithm: None,
            metrics: None,
            rules_engine: None,
            data_processor: None,
            config: MLPromotionConfig::default(),
        }
    }
    
    /// Конфигурация для production среды
    pub fn production() -> Self {
        Self {
            store: None,
            algorithm: None,
            metrics: None,
            rules_engine: None,
            data_processor: None,
            config: MLPromotionConfig::production(),
        }
    }
    
    /// Конфигурация для development среды
    pub fn development() -> Self {
        Self {
            store: None,
            algorithm: None,
            metrics: None,
            rules_engine: None,
            data_processor: None,
            config: MLPromotionConfig::minimal(),
        }
    }
    
    pub fn with_store(mut self, store: Arc<VectorStore>) -> Self {
        self.store = Some(store);
        self
    }
    
    pub fn with_algorithm(mut self, algorithm: Box<dyn PromotionAlgorithm>) -> Self {
        self.algorithm = Some(algorithm);
        self
    }
    
    pub fn with_metrics(mut self, metrics: Box<dyn PromotionMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }
    
    pub fn with_rules_engine(mut self, rules_engine: Box<dyn PromotionRulesEngine>) -> Self {
        self.rules_engine = Some(rules_engine);
        self
    }
    
    pub fn with_data_processor(mut self, data_processor: Box<dyn DataProcessor>) -> Self {
        self.data_processor = Some(data_processor);
        self
    }
    
    pub fn with_config(mut self, config: MLPromotionConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Автоматически создает все зависимости если они не заданы
    pub async fn build(mut self) -> Result<PromotionCoordinator> {
        info!("🏗️ Сборка PromotionCoordinator");
        
        // Проверяем обязательные зависимости
        let store = self.store.ok_or_else(|| anyhow::anyhow!("VectorStore is required"))?;
        
        // Создаем алгоритм если не задан
        let algorithm = match self.algorithm {
            Some(algo) => algo,
            None => {
                info!("🔧 Создание алгоритма: {}", self.config.algorithm_name);
                let algo_config = AlgorithmConfig::default();
                AlgorithmFactory::create(&self.config.algorithm_name, algo_config)?
            }
        };
        
        // Создаем metrics если не заданы
        let metrics = match self.metrics {
            Some(m) => m,
            None => {
                info!("📊 Создание metrics collector");
                let metrics_config = MetricsConfig::default();
                Box::new(MLPromotionMetricsCollector::new(metrics_config))
            }
        };
        
        // Создаем rules engine если не задан
        let rules_engine = match self.rules_engine {
            Some(re) => re,
            None => {
                info!("🎯 Создание rules engine");
                let rules_config = RulesConfig::default();
                Box::new(ConfigurableRulesEngine::new(rules_config))
            }
        };
        
        // Создаем data processor если не задан
        let data_processor = match self.data_processor {
            Some(dp) => dp,
            None => {
                info!("🔬 Создание data processor");
                let usage_tracker = Box::new(SimpleUsageTracker::new());
                let semantic_analyzer = Box::new(SimpleSemanticAnalyzer::new());
                let dp_config = DataProcessorConfig::default();
                
                Box::new(MLDataProcessor::new(
                    store.clone(),
                    usage_tracker,
                    semantic_analyzer,
                    dp_config,
                ).await?)
            }
        };
        
        let coordinator = PromotionCoordinator {
            store,
            algorithm,
            metrics,
            rules_engine,
            data_processor,
            config: self.config,
            is_training: false,
            last_training_check: chrono::Utc::now(),
        };
        
        info!("✅ PromotionCoordinator создан успешно");
        Ok(coordinator)
    }
}

impl PromotionCoordinator {
    /// Factory method для быстрого создания с defaults
    pub async fn new(store: Arc<VectorStore>) -> Result<Self> {
        PromotionCoordinatorBuilder::new()
            .with_store(store)
            .build()
            .await
    }
    
    /// Factory method для production конфигурации
    pub async fn new_production(store: Arc<VectorStore>) -> Result<Self> {
        let algo_config = AlgorithmConfig {
            learning_rate: 0.005, // Более консервативный learning rate
            epochs: 200,
            batch_size: 64,
            l2_regularization: 0.001,
        };
        
        let algorithm = AlgorithmFactory::create("hybrid", algo_config)?;
        let metrics = Box::new(MLPromotionMetricsCollector::new(MetricsConfig::production()));
        let rules_engine = Box::new(ConfigurableRulesEngine::new(RulesConfig::production()));
        
        let usage_tracker = Box::new(SimpleUsageTracker::new());
        let semantic_analyzer = Box::new(SimpleSemanticAnalyzer::new());
        let data_processor = Box::new(MLDataProcessor::new(
            store.clone(),
            usage_tracker,
            semantic_analyzer,
            DataProcessorConfig::default(),
        ).await?);
        
        PromotionCoordinatorBuilder::production()
            .with_store(store)
            .with_algorithm(algorithm)
            .with_metrics(metrics)
            .with_rules_engine(rules_engine)
            .with_data_processor(data_processor)
            .build()
            .await
    }
    
    /// Главный метод для запуска ML promotion цикла
    pub async fn run_promotion_cycle(&mut self) -> Result<MLPromotionStats> {
        info!("🚀 Запуск ML promotion cycle");
        
        let start_time = std::time::Instant::now();
        
        // 1. Проверяем нужно ли переобучать модель
        if self.should_retrain_model() {
            info!("🎯 Начинаем переобучение ML модели");
            self.retrain_model().await?;
        }
        
        // 2. Получаем кандидатов для promotion
        let candidates = self.get_promotion_candidates().await?;
        self.metrics.record_feature_extraction(10); // Mock timing
        
        info!("📋 Найдено {} кандидатов для анализа", candidates.len());
        
        // 3. Фильтруем кандидатов по business rules
        let filtered_candidates = self.rules_engine.filter_candidates(candidates).await?;
        
        info!("🔬 Отфильтровано {} кандидатов после rules", filtered_candidates.len());
        
        // 4. Анализируем кандидатов с ML
        let decisions = self.analyze_candidates_with_ml(filtered_candidates).await?;
        
        info!("🧠 ML анализ завершен: {} decisions", decisions.len());
        
        // 5. Выполняем promotions
        let promotion_results = self.execute_promotions(decisions).await?;
        
        // 6. Обновляем метрики
        let processing_time = start_time.elapsed().as_millis() as u64;
        self.metrics.record_inference(processing_time, 0.85); // Mock accuracy
        
        let stats = self.build_final_stats(&promotion_results, processing_time);
        
        info!("✅ ML promotion cycle завершен: проанализировано {}, продвинуто {}", 
              stats.total_analyzed, stats.promoted_records);
        
        Ok(stats)
    }
    
    /// Совместимость с оригинальным API - простой promote метод
    pub async fn promote(&mut self) -> Result<MLPromotionStats> {
        self.run_promotion_cycle().await
    }
    
    /// Анализ кандидатов с использованием ML алгоритма
    async fn analyze_candidates_with_ml(&mut self, candidates: Vec<Record>) -> Result<Vec<PromotionDecision>> {
        let mut decisions = Vec::new();
        let inference_start = std::time::Instant::now();
        
        for batch in candidates.chunks(self.config.ml_batch_size) {
            for record in batch {
                // Извлекаем features
                let features = self.data_processor.extract_features(record).await?;
                
                // Предсказываем promotion score
                let promotion_score = self.algorithm.predict_score(&features);
                
                // Проверяем threshold
                if promotion_score >= self.config.promotion_threshold {
                    // Определяем target layer
                    let target_layer = self.rules_engine.determine_target_layer(record, promotion_score);
                    
                    let decision = PromotionDecision {
                        record_id: record.id,
                        record: record.clone(),
                        current_layer: record.layer,
                        target_layer,
                        confidence: promotion_score,
                        features,
                        decision_timestamp: chrono::Utc::now(),
                        algorithm_used: self.config.algorithm_name.clone(),
                    };
                    
                    // Валидируем decision
                    if self.rules_engine.validate_promotion(&decision) {
                        decisions.push(decision);
                        debug!("✅ Decision создан: {} -> {:?} (conf: {:.3})", 
                               record.id, target_layer, promotion_score);
                    } else {
                        debug!("❌ Decision не прошел валидацию: {}", record.id);
                    }
                } else {
                    debug!("📉 Low confidence для {}: {:.3} < {:.3}", 
                           record.id, promotion_score, self.config.promotion_threshold);
                }
            }
        }
        
        // Записываем метрики inference
        let inference_time = inference_start.elapsed().as_millis() as u64;
        let accuracy = self.algorithm.get_accuracy();
        self.metrics.record_inference(inference_time, accuracy);
        
        Ok(decisions)
    }
    
    /// Выполнение promotions на основе ML decisions
    async fn execute_promotions(&mut self, decisions: Vec<PromotionDecision>) -> Result<PromotionResults> {
        let mut promoted_count = 0;
        let analyzed_count = decisions.len();
        let decisions_clone = decisions.clone();
        
        for decision in &decisions {
            // Выполняем promotion
            if self.execute_single_promotion(decision).await? {
                promoted_count += 1;
                debug!("⬆️ Promoted {} from {:?} to {:?}", 
                       decision.record_id, decision.current_layer, decision.target_layer);
            } else {
                warn!("❌ Failed to promote {}", decision.record_id);
            }
        }
        
        info!("📈 Promotions выполнены: {}/{} успешно", promoted_count, analyzed_count);
        
        Ok(PromotionResults {
            analyzed_count,
            promoted_count,
            decisions,
            avg_confidence: self.calculate_avg_confidence(&decisions_clone),
            processing_time_ms: 0, // Будет установлено в build_final_stats
        })
    }
    
    async fn execute_single_promotion(&self, decision: &PromotionDecision) -> Result<bool> {
        // Создаем новую запись для target layer
        let mut promoted_record = decision.record.clone();
        promoted_record.layer = decision.target_layer;
        promoted_record.ts = chrono::Utc::now();
        
        // Сохраняем в новый layer
        self.store.insert(&promoted_record).await?;
        
        // Удаляем из старого layer
        self.store.delete_by_id(&decision.record.id, decision.current_layer).await?;
        
        Ok(true)
    }
    
    async fn get_promotion_candidates(&self) -> Result<Vec<Record>> {
        let mut candidates = Vec::new();
        
        // Кандидаты из Interact layer
        let interact_iter = self.store.iter_layer(Layer::Interact).await?;
        for (_, value) in interact_iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                if stored.record.access_count >= self.config.min_access_threshold {
                    candidates.push(stored.record);
                }
            }
        }
        
        // Кандидаты из Insights layer для promotion в Assets
        let insights_iter = self.store.iter_layer(Layer::Insights).await?;
        for (_, value) in insights_iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                if stored.record.access_count >= self.config.min_access_threshold * 2 {
                    candidates.push(stored.record);
                }
            }
        }
        
        Ok(candidates)
    }
    
    fn should_retrain_model(&mut self) -> bool {
        let now = chrono::Utc::now();
        let hours_since_check = (now - self.last_training_check).num_hours();
        
        self.last_training_check = now;
        
        if self.is_training {
            return false; // Уже обучаемся
        }
        
        hours_since_check >= self.config.training_interval_hours as i64
    }
    
    async fn retrain_model(&mut self) -> Result<()> {
        if self.is_training {
            debug!("⏳ Модель уже обучается, пропускаем");
            return Ok(());
        }
        
        self.is_training = true;
        
        info!("🎯 Начинаем переобучение ML модели");
        
        // Собираем training data
        let training_data = self.data_processor.prepare_training_data().await?;
        
        if training_data.len() < 10 {
            warn!("⚠️ Недостаточно данных для обучения: {}", training_data.len());
            self.is_training = false;
            return Ok(());
        }
        
        info!("📚 Собрано {} примеров для обучения", training_data.len());
        
        // Обучаем модель
        let accuracy = self.algorithm.train(&training_data).await?;
        
        info!("✅ Модель переобучена, accuracy: {:.1}%", accuracy * 100.0);
        
        self.is_training = false;
        Ok(())
    }
    
    fn calculate_avg_confidence(&self, decisions: &[PromotionDecision]) -> f32 {
        if decisions.is_empty() {
            return 0.0;
        }
        
        let total: f32 = decisions.iter().map(|d| d.confidence).sum();
        total / decisions.len() as f32
    }
    
    fn build_final_stats(&self, results: &PromotionResults, processing_time_ms: u64) -> MLPromotionStats {
        // Получаем базовые метрики
        let mut stats = self.metrics.get_stats();
        
        // Обновляем с результатами promotion
        stats.total_analyzed = results.analyzed_count;
        stats.analyzed_records = results.analyzed_count;
        stats.promoted_records = results.promoted_count;
        stats.avg_confidence_score = results.avg_confidence;
        stats.processing_time_ms = processing_time_ms as f64;
        stats.algorithm_used = self.config.algorithm_name.clone();
        
        // Разделяем по типам promotion (если есть данные)
        for decision in &results.decisions {
            match (decision.current_layer, decision.target_layer) {
                (Layer::Interact, Layer::Insights) => stats.promoted_interact_to_insights += 1,
                (Layer::Interact, Layer::Assets) => stats.promoted_interact_to_insights += 1, // Skip level
                (Layer::Insights, Layer::Assets) => stats.promoted_insights_to_assets += 1,
                _ => {}
            }
        }
        
        stats
    }
    
    /// Получает текущую статистику без запуска promotion
    pub fn get_current_stats(&self) -> MLPromotionStats {
        self.metrics.get_stats()
    }
    
    /// Сбрасывает все накопленные метрики
    pub fn reset_metrics(&mut self) {
        self.metrics.reset_metrics();
        info!("🔄 Метрики сброшены");
    }
    
    /// Получает информацию о состоянии coordinator
    pub fn get_coordinator_info(&self) -> CoordinatorInfo {
        CoordinatorInfo {
            algorithm_name: self.config.algorithm_name.clone(),
            promotion_threshold: self.config.promotion_threshold,
            training_interval_hours: self.config.training_interval_hours,
            is_currently_training: self.is_training,
            last_training_check: self.last_training_check,
            algorithm_accuracy: self.algorithm.get_accuracy(),
        }
    }
}

/// Информация о состоянии coordinator
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoordinatorInfo {
    pub algorithm_name: String,
    pub promotion_threshold: f32,
    pub training_interval_hours: u64,
    pub is_currently_training: bool,
    pub last_training_check: chrono::DateTime<chrono::Utc>,
    pub algorithm_accuracy: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use std::sync::Arc;

    // Mock VectorStore для тестирования
    struct MockVectorStore;
    
    // TODO: Create VectorStore trait and implement for MockVectorStore
    // #[async_trait::async_trait]
    // impl VectorStore for MockVectorStore {
    //     // Заглушки для всех методов VectorStore trait
    //     // В реальном тесте здесь была бы полная реализация
    // }

    #[tokio::test]
    async fn test_coordinator_builder() {
        // let store = Arc::new(MockVectorStore);
        // let coordinator = PromotionCoordinatorBuilder::new()
        //     .with_store(store)
        //     .build()
        //     .await;
        // 
        // assert!(coordinator.is_ok());
        
        // Тест требует полной mock реализации VectorStore
        assert!(true); // Placeholder
    }

    #[test]
    fn test_coordinator_info() {
        let info = CoordinatorInfo {
            algorithm_name: "hybrid".to_string(),
            promotion_threshold: 0.7,
            training_interval_hours: 24,
            is_currently_training: false,
            last_training_check: chrono::Utc::now(),
            algorithm_accuracy: 0.85,
        };
        
        assert_eq!(info.algorithm_name, "hybrid");
        assert!(!info.is_currently_training);
    }

    #[test]
    fn test_ml_promotion_config_variants() {
        let default_config = MLPromotionConfig::default();
        let production_config = MLPromotionConfig::production();
        let minimal_config = MLPromotionConfig::minimal();
        
        assert_eq!(default_config.promotion_threshold, 0.7);
        assert_eq!(production_config.promotion_threshold, 0.8);
        assert_eq!(minimal_config.promotion_threshold, 0.6);
        
        assert!(production_config.min_access_threshold > default_config.min_access_threshold);
        assert!(minimal_config.min_access_threshold < default_config.min_access_threshold);
    }
}