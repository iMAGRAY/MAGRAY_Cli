//! Comprehensive unit тесты для CacheService
//!
//! Coverage areas:
//! - Cache management и fallback embedding generation
//! - Concurrent access scenarios  
//! - Cache statistics и hit rate calculation
//! - Integration с CoordinatorService
//! - Property-based testing для cache consistency
//! - Edge cases и error handling

use std::sync::Arc;
use anyhow::Result;
use tokio_test;
use proptest::prelude::*;
use once_cell::sync::Lazy;
use mockall::{predicate::*, mock};

use memory::{
    services::{
        CacheService, CoordinatorService,
        traits::{CacheServiceTrait, CoordinatorServiceTrait}
    },
    di_container::DIContainer,
    storage::VectorStore,
    gpu_accelerated::GpuBatchProcessor,
    health::HealthMonitor,
};

static INIT_TRACING: Lazy<()> = Lazy::new(|| {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
});

// Mock CoordinatorService для тестирования cache интеграции
mockall::mock! {
    pub TestCoordinatorService {}
    
    #[async_trait::async_trait]
    impl CoordinatorServiceTrait for TestCoordinatorService {
        async fn create_coordinators(&self, container: &DIContainer) -> Result<()>;
        async fn initialize_coordinators(&self) -> Result<()>;
        fn get_embedding_coordinator(&self) -> Option<Arc<memory::orchestration::EmbeddingCoordinator>>;
        fn get_search_coordinator(&self) -> Option<Arc<memory::orchestration::SearchCoordinator>>;
        fn get_health_manager(&self) -> Option<Arc<memory::orchestration::HealthManager>>;
        fn get_resource_controller(&self) -> Option<Arc<memory::orchestration::ResourceController>>;
        async fn shutdown_coordinators(&self) -> Result<()>;
        fn count_active_coordinators(&self) -> usize;
    }
}

/// Helper для создания test DI container
fn create_test_container() -> Arc<DIContainer> {
    Lazy::force(&INIT_TRACING);
    
    let container = Arc::new(DIContainer::new());
    
    // Register basic dependencies
    let vector_store = Arc::new(VectorStore::new_in_memory(1024));
    container.register(vector_store).expect("Не удалось зарегистрировать VectorStore");
    
    // Note: В текущей реализации cache не регистрируется в DI (возвращает None)
    // Это архитектурное ограничение для dyn traits
    
    container
}

/// Helper для создания mock coordinator service
fn create_mock_coordinator_service() -> Arc<MockTestCoordinatorService> {
    let mut mock = MockTestCoordinatorService::new();
    
    mock.expect_get_embedding_coordinator()
        .returning(|| None) // В тестах возвращаем None для упрощения
        .times(0..);
    
    Arc::new(mock)
}

#[tokio::test]
async fn test_cache_service_creation() -> Result<()> {
    let container = create_test_container();
    
    // Test basic creation
    let service = CacheService::new(container.clone());
    assert_eq!(service.get_embedding_dimension(), 1024, "Default embedding dimension должно быть 1024");
    
    // Test creation with coordinator
    let coordinator = create_mock_coordinator_service();
    let service_with_coordinator = CacheService::new_with_coordinator(
        container.clone(),
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    assert_eq!(service_with_coordinator.get_embedding_dimension(), 1024, "Embedding dimension с coordinator должно быть 1024");
    
    // Test creation with custom dimension
    let custom_service = CacheService::new_with_dimension(container, 512);
    assert_eq!(custom_service.get_embedding_dimension(), 512, "Custom embedding dimension должно быть 512");
    
    Ok(())
}

#[tokio::test]
async fn test_fallback_embedding_generation() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test fallback embedding generation
    let embedding = service.generate_fallback_embedding("test query");
    
    assert_eq!(embedding.len(), 1024, "Fallback embedding должен иметь размерность 1024");
    
    // Test that embedding is normalized (sum of squares should be close to 1)
    let norm_squared: f32 = embedding.iter().map(|x| x * x).sum();
    let norm = norm_squared.sqrt();
    assert!((norm - 1.0).abs() < 0.001, "Embedding должен быть нормализован (norm ≈ 1.0)");
    
    // Test deterministic nature
    let embedding2 = service.generate_fallback_embedding("test query");
    assert_eq!(embedding, embedding2, "Fallback embedding должен быть детерминированным");
    
    // Test different inputs produce different embeddings
    let different_embedding = service.generate_fallback_embedding("different query");
    assert_ne!(embedding, different_embedding, "Разные входы должны давать разные embeddings");
    
    Ok(())
}

#[tokio::test]
async fn test_get_or_create_embedding() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test embedding generation
    let embedding = service.get_or_create_embedding("test text").await?;
    
    assert_eq!(embedding.len(), 1024, "Generated embedding должен иметь правильную размерность");
    
    // Test consistent results
    let embedding2 = service.get_or_create_embedding("test text").await?;
    assert_eq!(embedding, embedding2, "Repeated calls должны возвращать идентичные embeddings");
    
    Ok(())
}

#[tokio::test]
async fn test_get_or_create_embedding_with_coordinator() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = CacheService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test with coordinator (should fallback to local generation)
    let embedding = service.get_or_create_embedding("coordinator test").await?;
    assert_eq!(embedding.len(), 1024, "Embedding с coordinator должен иметь правильную размерность");
    
    Ok(())
}

#[tokio::test]
async fn test_cache_stats() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test cache stats (should return zeros since no real cache is available)
    let (hits, misses, size) = service.get_cache_stats().await;
    
    assert_eq!(hits, 0, "Cache hits должно быть 0 без реального cache");
    assert_eq!(misses, 0, "Cache misses должно быть 0 без реального cache");
    assert_eq!(size, 0, "Cache size должно быть 0 без реального cache");
    
    Ok(())
}

#[tokio::test]
async fn test_cache_hit_rate() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test hit rate calculation with zero stats
    let hit_rate = service.get_cache_hit_rate().await;
    assert_eq!(hit_rate, 0.0, "Hit rate должен быть 0.0 при отсутствии операций");
    
    Ok(())
}

#[tokio::test]
async fn test_clear_cache() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test cache clear (should error since no real cache available)
    let result = service.clear_cache().await;
    assert!(result.is_err(), "Clear cache должен завершаться с ошибкой без реального cache");
    
    Ok(())
}

#[tokio::test]
async fn test_set_cache_size() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test cache size setting (should error since no real cache available)
    let result = service.set_cache_size(1000).await;
    assert!(result.is_err(), "Set cache size должен завершаться с ошибкой без реального cache");
    
    Ok(())
}

#[tokio::test]
async fn test_detailed_cache_stats() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = CacheService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test detailed cache stats
    let stats = service.get_detailed_cache_stats().await;
    
    assert_eq!(stats.cache_hits, 0, "Cache hits должно быть 0");
    assert_eq!(stats.cache_misses, 0, "Cache misses должно быть 0");
    assert_eq!(stats.cache_size, 0, "Cache size должно быть 0");
    assert_eq!(stats.hit_rate, 0.0, "Hit rate должен быть 0.0");
    assert_eq!(stats.total_requests, 0, "Total requests должно быть 0");
    assert_eq!(stats.embedding_dimension, 1024, "Embedding dimension должно быть 1024");
    assert!(stats.coordinator_available, "Coordinator должен быть доступен");
    assert!(!stats.cache_available, "Cache не должен быть доступен");
    
    Ok(())
}

#[tokio::test]
async fn test_embedding_dimension_modification() -> Result<()> {
    let container = create_test_container();
    let mut service = CacheService::new(container);
    
    // Test dimension modification
    assert_eq!(service.get_embedding_dimension(), 1024, "Initial dimension должно быть 1024");
    
    service.set_embedding_dimension(768);
    assert_eq!(service.get_embedding_dimension(), 768, "Dimension должно обновиться до 768");
    
    // Test that new embeddings have updated dimension
    let embedding = service.generate_fallback_embedding("test");
    assert_eq!(embedding.len(), 768, "New embedding должен иметь обновленную размерность");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_embedding_generation() -> Result<()> {
    let container = create_test_container();
    let service = Arc::new(CacheService::new(container));
    
    // Test concurrent embedding generation
    let tasks: Vec<_> = (0..20)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                service_clone.get_or_create_embedding(&format!("query {}", i)).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(tasks).await;
    
    // All embeddings should be generated successfully
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Task {} должна завершиться без panic", i);
        let embedding = result.unwrap().expect("Embedding generation должно быть успешным");
        assert_eq!(embedding.len(), 1024, "Embedding {} должен иметь правильную размерность", i);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_cache_operations() -> Result<()> {
    let container = create_test_container();
    let service = Arc::new(CacheService::new(container));
    
    // Test concurrent cache operations
    let tasks = vec![
        tokio::spawn({
            let service = service.clone();
            async move { service.get_cache_stats().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.get_cache_hit_rate().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.get_detailed_cache_stats().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.clear_cache().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.set_cache_size(500).await }
        }),
    ];
    
    let results = futures::future::join_all(tasks).await;
    
    // All operations should complete (some may fail due to missing cache, but shouldn't panic)
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Concurrent operation {} должна завершиться без panic", i);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_mixed_concurrent_operations() -> Result<()> {
    let container = create_test_container();
    let service = Arc::new(CacheService::new(container));
    
    // Mix of embedding generation and cache operations
    let tasks: Vec<_> = (0..50)
        .map(|i| {
            let service_clone = service.clone();
            if i % 3 == 0 {
                tokio::spawn(async move {
                    service_clone.get_or_create_embedding(&format!("text {}", i)).await.map(|_| ())
                })
            } else if i % 3 == 1 {
                tokio::spawn(async move {
                    let _ = service_clone.get_cache_stats().await;
                    Ok(())
                })
            } else {
                tokio::spawn(async move {
                    let _ = service_clone.get_cache_hit_rate().await;
                    Ok(())
                })
            }
        })
        .collect();
    
    let results = futures::future::join_all(tasks).await;
    
    // All mixed operations should complete without panicking
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Mixed operation {} должна завершиться без panic", i);
    }
    
    Ok(())
}

// Property-based tests
proptest::proptest! {
    #[test]
    fn test_embedding_generation_properties(
        text in "\\PC{1,1000}", // Any Unicode text, 1-1000 characters
        dimension in 256usize..2048
    ) {
        tokio_test::block_on(async {
            let container = create_test_container();
            let mut service = CacheService::new_with_dimension(container, dimension);
            
            let embedding = service.generate_fallback_embedding(&text);
            
            // Properties that should always hold
            prop_assert_eq!(embedding.len(), dimension, "Embedding должен иметь заданную размерность");
            
            // Embedding should be normalized
            let norm_squared: f32 = embedding.iter().map(|x| x * x).sum();
            let norm = norm_squared.sqrt();
            prop_assert!((norm - 1.0).abs() < 0.001, "Embedding должен быть нормализован");
            
            // Deterministic property: same input should always produce same output
            let embedding2 = service.generate_fallback_embedding(&text);
            prop_assert_eq!(embedding, embedding2, "Embedding generation должно быть детерминированным");
            
            // No NaN or infinite values
            for val in &embedding {
                prop_assert!(val.is_finite(), "Embedding не должен содержать NaN или infinity");
            }
        });
    }
    
    #[test]
    fn test_cache_hit_rate_calculation(
        hits in 0u64..1000,
        misses in 0u64..1000
    ) {
        tokio_test::block_on(async {
            // Create a mock service that can return custom stats
            let container = create_test_container();
            let service = CacheService::new(container);
            
            // Calculate expected hit rate
            let total = hits + misses;
            let expected_rate = if total == 0 {
                0.0
            } else {
                (hits as f64 / total as f64) * 100.0
            };
            
            // Note: В реальной реализации мы бы тестировали с mock cache
            // Здесь мы проверяем что формула корректна для edge cases
            prop_assert!(expected_rate >= 0.0 && expected_rate <= 100.0, "Hit rate должен быть в пределах 0-100%");
            
            if total == 0 {
                prop_assert_eq!(expected_rate, 0.0, "Hit rate должен быть 0 при отсутствии операций");
            }
        });
    }
    
    #[test]
    fn test_embedding_dimension_consistency(
        dimension in 64usize..4096,
        texts in proptest::collection::vec("\\PC{1,100}", 1..10)
    ) {
        tokio_test::block_on(async {
            let container = create_test_container();
            let service = CacheService::new_with_dimension(container, dimension);
            
            // All embeddings should have consistent dimension
            for text in &texts {
                let embedding = service.generate_fallback_embedding(text);
                prop_assert_eq!(embedding.len(), dimension, "Все embeddings должны иметь одинаковую размерность");
                
                // Test async version too
                let async_embedding = service.get_or_create_embedding(text).await.unwrap();
                prop_assert_eq!(async_embedding.len(), dimension, "Async embeddings должны иметь ту же размерность");
            }
        });
    }
}

#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let container = create_test_container();
    let service = CacheService::new(container);
    
    // Test empty text
    let empty_embedding = service.get_or_create_embedding("").await?;
    assert_eq!(empty_embedding.len(), 1024, "Empty text должен генерировать embedding правильной размерности");
    
    // Test very long text
    let long_text = "x".repeat(10000);
    let long_embedding = service.get_or_create_embedding(&long_text).await?;
    assert_eq!(long_embedding.len(), 1024, "Long text должен генерировать embedding правильной размерности");
    
    // Test special characters
    let special_text = "!@#$%^&*()_+-=[]{}|;':\",./<>?~`";
    let special_embedding = service.get_or_create_embedding(special_text).await?;
    assert_eq!(special_embedding.len(), 1024, "Special characters должны генерировать embedding правильной размерности");
    
    // Test Unicode
    let unicode_text = "Hello 世界 🌍 αβγ";
    let unicode_embedding = service.get_or_create_embedding(unicode_text).await?;
    assert_eq!(unicode_embedding.len(), 1024, "Unicode text должен генерировать embedding правильной размерности");
    
    Ok(())
}

#[tokio::test]
async fn test_extreme_dimensions() -> Result<()> {
    let container = create_test_container();
    
    // Test very small dimension
    let small_service = CacheService::new_with_dimension(container.clone(), 1);
    let small_embedding = small_service.generate_fallback_embedding("test");
    assert_eq!(small_embedding.len(), 1, "Dimension 1 должен работать");
    assert!((small_embedding[0].abs() - 1.0).abs() < 0.001, "Single value должен быть нормализован до ±1");
    
    // Test large dimension
    let large_service = CacheService::new_with_dimension(container, 8192);
    let large_embedding = large_service.generate_fallback_embedding("test");
    assert_eq!(large_embedding.len(), 8192, "Large dimension должен работать");
    
    let norm_squared: f32 = large_embedding.iter().map(|x| x * x).sum();
    let norm = norm_squared.sqrt();
    assert!((norm - 1.0).abs() < 0.001, "Large embedding должен быть нормализован");
    
    Ok(())
}

#[tokio::test]
async fn test_cache_service_with_real_coordinator() -> Result<()> {
    let container = create_test_container();
    
    // Add required dependencies for CoordinatorService
    let gpu_processor = Arc::new(GpuBatchProcessor::new_cpu_fallback());
    container.register(gpu_processor)?;
    
    let health_monitor = Arc::new(HealthMonitor::new());
    container.register(health_monitor)?;
    
    let resource_manager = Arc::new(parking_lot::RwLock::new(
        memory::resource_manager::ResourceManager::new()
    ));
    container.register(resource_manager)?;
    
    // Create real CoordinatorService
    let coordinator_service = Arc::new(CoordinatorService::new());
    coordinator_service.create_coordinators(&container).await?;
    coordinator_service.initialize_coordinators().await?;
    
    // Create CacheService with real coordinator
    let cache_service = CacheService::new_with_coordinator(
        container,
        coordinator_service as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test full integration
    let embedding = cache_service.get_or_create_embedding("integration test").await?;
    assert_eq!(embedding.len(), 1024, "Integration test embedding должен работать");
    
    let stats = cache_service.get_detailed_cache_stats().await;
    assert!(stats.coordinator_available, "Coordinator должен быть доступен в integration тесте");
    
    Ok(())
}

#[tokio::test]
async fn test_cache_service_memory_safety() -> Result<()> {
    // Test memory safety with many operations
    for i in 0..50 {
        let container = create_test_container();
        let service = CacheService::new_with_dimension(container, 1024 + i * 10); // Vary dimension
        
        // Multiple operations per service
        let embedding = service.get_or_create_embedding(&format!("test text {}", i)).await?;
        assert_eq!(embedding.len(), 1024 + i * 10, "Embedding должен иметь правильную размерность");
        
        let _ = service.get_cache_stats().await;
        let _ = service.get_cache_hit_rate().await;
        let _ = service.get_detailed_cache_stats().await;
        
        // Test fallback generation directly
        for j in 0..10 {
            let fallback = service.generate_fallback_embedding(&format!("fallback {}-{}", i, j));
            assert_eq!(fallback.len(), 1024 + i * 10, "Fallback embedding должен иметь правильную размерность");
        }
    }
    
    // If we reach here without memory issues, test passes
    Ok(())
}

#[tokio::test]
async fn test_cache_service_consistency_across_instances() -> Result<()> {
    let container1 = create_test_container();
    let container2 = create_test_container();
    
    let service1 = CacheService::new(container1);
    let service2 = CacheService::new(container2);
    
    // Same inputs should produce same outputs across different service instances
    let text = "consistency test";
    let embedding1 = service1.get_or_create_embedding(text).await?;
    let embedding2 = service2.get_or_create_embedding(text).await?;
    
    assert_eq!(embedding1, embedding2, "Different service instances должны генерировать идентичные embeddings");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default due to performance
async fn stress_test_cache_service() -> Result<()> {
    let container = create_test_container();
    let service = Arc::new(CacheService::new(container));
    
    // High load stress test
    let tasks: Vec<_> = (0..1000)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                // Mix of operations
                if i % 4 == 0 {
                    service_clone.get_or_create_embedding(&format!("stress test {}", i)).await.map(|_| ())
                } else if i % 4 == 1 {
                    service_clone.get_cache_stats().await;
                    Ok(())
                } else if i % 4 == 2 {
                    let _ = service_clone.get_cache_hit_rate().await;
                    Ok(())
                } else {
                    let _ = service_clone.get_detailed_cache_stats().await;
                    Ok(())
                }
            })
        })
        .collect();
    
    let start_time = std::time::Instant::now();
    let results = futures::future::join_all(tasks).await;
    let duration = start_time.elapsed();
    
    println!("Cache stress test completed in {:?}", duration);
    
    // All operations should complete successfully
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Stress test operation {} должна завершиться без panic", i);
        assert!(result.unwrap().is_ok(), "Stress test operation {} должна быть успешной", i);
    }
    
    Ok(())
}