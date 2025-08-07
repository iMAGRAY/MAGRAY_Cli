//! ML-based promotion система - полная декомпозиция God Object
//! 
//! Этот модуль заменяет монолитный ml_promotion.rs на модульную архитектуру
//! следующую всем принципам SOLID:
//! 
//! - **Single Responsibility**: каждый модуль имеет единственную ответственность
//! - **Open/Closed**: extensible через trait abstractions
//! - **Liskov Substitution**: взаимозаменяемые implementations
//! - **Interface Segregation**: minimal, focused interfaces
//! - **Dependency Inversion**: зависимости от abstractions, DI pattern
//! 
//! ## Архитектура модулей:
//! 
//! ### Core Traits (`traits.rs`)
//! - `PromotionAlgorithm` - ML алгоритмы promotion
//! - `PromotionMetrics` - сбор метрик и аналитика
//! - `PromotionRulesEngine` - business rules
//! - `DataProcessor` - ML data preparation pipeline
//! - `UsageTracker`, `SemanticAnalyzer` - специализированные traits
//! 
//! ### Types (`types.rs`)
//! - Shared types и data structures
//! - Configuration objects
//! - Conversion traits для backward compatibility
//! 
//! ### Algorithm Implementations (`algorithms.rs`)
//! - `FrequencyAlgorithm` - frequency-based promotion
//! - `SemanticAlgorithm` - semantic-based promotion  
//! - `HybridAlgorithm` - комбинированный подход
//! - `AlgorithmFactory` - factory pattern для создания алгоритмов
//! 
//! ### Metrics Collection (`metrics.rs`)
//! - `MLPromotionMetricsCollector` - comprehensive metrics
//! - Real-time performance tracking
//! - Historical data analysis
//! - Export capabilities
//! 
//! ### Business Rules Engine (`rules_engine.rs`)
//! - `ConfigurableRulesEngine` - flexible business rules
//! - Layer strategies
//! - Time-based rules
//! - Promotion history tracking
//! 
//! ### Data Processing Pipeline (`data_processor.rs`)
//! - `MLDataProcessor` - feature extraction & preparation
//! - Training data collection
//! - Feature engineering & normalization
//! - Caching optimizations
//! 
//! ### Main Coordinator (`coordinator.rs`)
//! - `PromotionCoordinator` - facade pattern с DI
//! - Orchestrates all components
//! - 100% backward compatibility
//! - Builder pattern for flexibility

pub mod traits;
pub mod types;
pub mod algorithms;
pub mod metrics;
pub mod rules_engine;
pub mod data_processor;
pub mod coordinator;
pub mod legacy_facade;

// Re-exports для backward compatibility с оригинальным API
pub use coordinator::{PromotionCoordinator, PromotionCoordinatorBuilder, CoordinatorInfo};
pub use types::{MLPromotionConfig, MLPromotionStats, PromotionDecision, PromotionFeatures};
pub use traits::{PromotionAlgorithm, PromotionMetrics, PromotionRulesEngine, DataProcessor};
pub use algorithms::{FrequencyAlgorithm, SemanticAlgorithm, HybridAlgorithm, AlgorithmFactory};
pub use metrics::{MLPromotionMetricsCollector, MetricsConfig, PerformanceBreakdown};
pub use rules_engine::{ConfigurableRulesEngine, RulesConfig, LayerStrategy, SpecialCondition, BusinessRule};
pub use data_processor::{MLDataProcessor, DataProcessorConfig, SimpleUsageTracker, SimpleSemanticAnalyzer};

// Legacy API compatibility - 100% drop-in replacement
pub use legacy_facade::{MLPromotionEngine, UsageTracker, SemanticAnalyzer, PerformanceOptimizer};

/// Factory function для быстрого создания production-ready coordinator
pub async fn create_production_coordinator(
    store: std::sync::Arc<crate::storage::VectorStore>
) -> anyhow::Result<PromotionCoordinator> {
    PromotionCoordinator::new_production(store).await
}

/// Factory function для development/testing coordinator
pub async fn create_development_coordinator(
    store: std::sync::Arc<crate::storage::VectorStore>
) -> anyhow::Result<PromotionCoordinator> {
    use traits::AlgorithmConfig;
    
    let algo_config = AlgorithmConfig {
        learning_rate: 0.1,  // Faster learning для development
        epochs: 50,
        batch_size: 16,
        l2_regularization: 0.0,
    };
    
    let algorithm = AlgorithmFactory::create("frequency", algo_config)?;
    let metrics = Box::new(MLPromotionMetricsCollector::new(MetricsConfig::debug()));
    let rules_engine = Box::new(ConfigurableRulesEngine::new(RulesConfig::development()));
    
    let usage_tracker = Box::new(SimpleUsageTracker::new());
    let semantic_analyzer = Box::new(SimpleSemanticAnalyzer::new());
    let data_processor = Box::new(MLDataProcessor::new(
        store.clone(),
        usage_tracker,
        semantic_analyzer,
        DataProcessorConfig::default(),
    ).await?);
    
    PromotionCoordinatorBuilder::development()
        .with_store(store)
        .with_algorithm(algorithm)
        .with_metrics(metrics)
        .with_rules_engine(rules_engine)
        .with_data_processor(data_processor)
        .build()
        .await
}

/// Утилита для миграции с оригинального ml_promotion.rs
pub struct MigrationHelper;

impl MigrationHelper {
    /// Создает coordinator совместимый с оригинальным MLPromotionEngine API
    pub async fn create_compatible_coordinator(
        store: std::sync::Arc<crate::storage::VectorStore>,
        config: MLPromotionConfig,
    ) -> anyhow::Result<PromotionCoordinator> {
        let algo_config = traits::AlgorithmConfig {
            learning_rate: 0.01,
            epochs: 100,
            batch_size: config.ml_batch_size,
            l2_regularization: 0.001,
        };
        
        PromotionCoordinatorBuilder::new()
            .with_store(store)
            .with_config(config)
            .with_algorithm(AlgorithmFactory::create("hybrid", algo_config)?)
            .build()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Проверяем что все основные типы доступны
        let _config = MLPromotionConfig::default();
        let _stats = MLPromotionStats::default();
        
        // Проверяем конфигурации
        let prod_config = MLPromotionConfig::production();
        let dev_config = MLPromotionConfig::minimal();
        
        assert!(prod_config.promotion_threshold > dev_config.promotion_threshold);
        assert!(prod_config.min_access_threshold > dev_config.min_access_threshold);
    }
    
    #[test]
    fn test_factory_configs() {
        let debug_metrics = MetricsConfig::debug();
        let prod_metrics = MetricsConfig::production();
        
        assert!(debug_metrics.detailed_logging);
        assert!(!prod_metrics.detailed_logging);
        
        let dev_rules = RulesConfig::development();
        let prod_rules = RulesConfig::production();
        
        assert!(dev_rules.min_repromotion_interval_hours < prod_rules.min_repromotion_interval_hours);
        assert!(!dev_rules.strict_validation);
        assert!(prod_rules.strict_validation);
    }
    
    #[tokio::test]
    async fn test_algorithm_factory() {
        use traits::AlgorithmConfig;
        
        let config = AlgorithmConfig::default();
        
        let freq_algo = AlgorithmFactory::create("frequency", config.clone());
        assert!(freq_algo.is_ok());
        
        let semantic_algo = AlgorithmFactory::create("semantic", config.clone());
        assert!(semantic_algo.is_ok());
        
        let hybrid_algo = AlgorithmFactory::create("hybrid", config.clone());
        assert!(hybrid_algo.is_ok());
        
        let unknown_algo = AlgorithmFactory::create("unknown", config);
        assert!(unknown_algo.is_err());
    }
    
    #[test]
    fn test_available_algorithms() {
        let available = AlgorithmFactory::available_algorithms();
        assert!(available.contains(&"frequency"));
        assert!(available.contains(&"semantic"));
        assert!(available.contains(&"hybrid"));
        assert_eq!(available.len(), 3);
    }
}