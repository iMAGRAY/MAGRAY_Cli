use anyhow::Result;
use memory::{
    MLPromotionEngine, MLPromotionConfig, PromotionFeatures,
    VectorStore, Layer, Record,
};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;
use chrono::{Utc, Duration};

#[tokio::test]
async fn test_ml_model_training() -> Result<()> {
    // Создаем временную директорию
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    
    // Создаем store
    let store = Arc::new(VectorStore::new(&db_path).await?);
    
    // Конфигурация с быстрым обучением для теста
    let config = MLPromotionConfig {
        min_access_threshold: 2,
        temporal_weight: 0.3,
        semantic_weight: 0.4,
        usage_weight: 0.3,
        promotion_threshold: 0.7,
        ml_batch_size: 16,
        training_interval_hours: 0, // Обучать сразу
        use_gpu_for_ml: false,
    };
    
    // Создаем ML promotion engine
    let mut engine = MLPromotionEngine::new(store.clone(), config).await?;
    
    // Создаем тестовые данные для обучения
    let now = Utc::now();
    
    // Добавляем "успешные" записи в Assets (важные)
    for i in 0..50 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Assets,
            text: format!("Critical error in production system {}", i),
            kind: "error".to_string(),
            tags: vec!["critical".to_string(), "production".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.1; 1024],
            ts: now - Duration::days(5 + i as i64),
            last_access: now - Duration::hours(2),
            access_count: 50 + i as u32,
            score: 0.9,
        };
        store.insert(&record).await?;
    }
    
    // Добавляем "средние" записи в Insights
    for i in 0..50 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Insights,
            text: format!("Important feature request {}", i),
            kind: "feature".to_string(),
            tags: vec!["feature".to_string(), "important".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.2; 1024],
            ts: now - Duration::days(3 + i as i64),
            last_access: now - Duration::hours(24),
            access_count: 10 + i as u32,
            score: 0.7,
        };
        store.insert(&record).await?;
    }
    
    // Добавляем "неуспешные" записи в Interact (не promoted)
    for i in 0..50 {
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("Debug log message {}", i),
            kind: "debug".to_string(),
            tags: vec!["debug".to_string()],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding: vec![0.3; 1024],
            ts: now - Duration::days(2),
            last_access: now - Duration::days(1),
            access_count: 1,
            score: 0.3,
        };
        store.insert(&record).await?;
    }
    
    println!("✅ Создано 150 тестовых записей для обучения");
    
    // Запускаем promotion цикл с обучением
    let stats = engine.run_ml_promotion_cycle().await?;
    
    println!("\n📊 Статистика ML promotion:");
    println!("  - Проанализировано: {}", stats.total_analyzed);
    println!("  - Promoted: {}", stats.promoted_interact_to_insights);
    println!("  - ML inference время: {} мс", stats.ml_inference_time_ms);
    println!("  - Точность модели: {:.1}%", stats.model_accuracy * 100.0);
    println!("  - Средняя уверенность: {:.2}", stats.avg_confidence_score);
    
    // Проверяем, что модель обучилась
    assert!(stats.model_accuracy > 0.7, "Модель должна достичь хорошей точности");
    
    // Тестируем предсказания на новых данных
    println!("\n🔬 Тестирование предсказаний модели...");
    
    // Тест 1: Критическая ошибка (должна быть promoted)
    let critical_features = PromotionFeatures {
        age_hours: 30.0,
        access_recency: 0.9,
        temporal_pattern_score: 0.8,
        access_count: 0.9,
        access_frequency: 0.8,
        session_importance: 0.9,
        semantic_importance: 0.95, // "critical" keyword
        keyword_density: 0.8,
        topic_relevance: 0.9,
        layer_affinity: 0.8,
        co_occurrence_score: 0.7,
        user_preference_score: 0.8,
    };
    
    let critical_score = engine.predict_promotion_score(&critical_features);
    println!("  - Критическая ошибка: score = {:.2} (порог = 0.7)", critical_score);
    assert!(critical_score > 0.7, "Критическая ошибка должна быть promoted");
    
    // Тест 2: Debug сообщение (не должно быть promoted)
    let debug_features = PromotionFeatures {
        age_hours: 48.0,
        access_recency: 0.1,
        temporal_pattern_score: 0.2,
        access_count: 0.1,
        access_frequency: 0.1,
        session_importance: 0.2,
        semantic_importance: 0.1, // низкая важность
        keyword_density: 0.1,
        topic_relevance: 0.2,
        layer_affinity: 0.1,
        co_occurrence_score: 0.1,
        user_preference_score: 0.1,
    };
    
    let debug_score = engine.predict_promotion_score(&debug_features);
    println!("  - Debug сообщение: score = {:.2} (порог = 0.7)", debug_score);
    assert!(debug_score < 0.5, "Debug сообщение не должно быть promoted");
    
    println!("\n✅ Все тесты ML модели пройдены!");
    
    Ok(())
}

#[tokio::test]
async fn test_ml_features_extraction() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    let store = Arc::new(VectorStore::new(&db_path).await?);
    
    let config = MLPromotionConfig::default();
    let engine = MLPromotionEngine::new(store.clone(), config).await?;
    
    // Создаем тестовую запись
    let record = Record {
        id: Uuid::new_v4(),
        layer: Layer::Interact,
        text: "Critical security vulnerability detected in authentication module".to_string(),
        kind: "security".to_string(),
        tags: vec!["critical".to_string(), "security".to_string()],
        project: "test".to_string(),
        session: "test".to_string(),
        embedding: vec![0.5; 1024],
        ts: Utc::now() - Duration::hours(24),
        last_access: Utc::now() - Duration::hours(1),
        access_count: 15,
        score: 0.85,
    };
    
    // Извлекаем features
    let features = engine.extract_features(&record).await?;
    
    println!("🔬 Извлеченные features:");
    println!("  - age_hours: {:.1}", features.age_hours);
    println!("  - access_recency: {:.2}", features.access_recency);
    println!("  - access_count: {:.2}", features.access_count);
    println!("  - semantic_importance: {:.2}", features.semantic_importance);
    println!("  - keyword_density: {:.2}", features.keyword_density);
    
    // Проверяем корректность извлечения
    assert!(features.age_hours > 23.0 && features.age_hours < 25.0);
    assert!(features.semantic_importance > 0.8, "Security + critical должны дать высокую важность");
    assert!(features.access_count > 0.0, "Access count должен быть нормализован");
    
    println!("✅ Feature extraction работает корректно!");
    
    Ok(())
}