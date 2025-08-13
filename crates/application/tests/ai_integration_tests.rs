#![allow(unused_imports)]
#![allow(unused_attributes)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unexpected_cfgs)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::uninlined_format_args)]
#[cfg(test)]
mod ai_integration_tests {
    use anyhow::Result;
    use std::sync::Arc;

    #[cfg(feature = "test-utils")]
    use application::ports::cache_provider::MockCacheProvider;
    #[cfg(feature = "test-utils")]
    use application::ports::metrics_collector::MockMetricsCollector;
    use application::services::ai_application_service::AiApplicationService;
    use application::use_cases::ai_use_cases::{
        AiStatusRequest, AiUseCaseFactory, InferenceRequest, ListModelsRequest, LoadModelRequest,
    };

    // Mock service provider for testing
    struct MockAiServiceProvider;

    impl MockAiServiceProvider {
        fn new() -> Self {
            Self
        }
    }

    impl application::use_cases::ai_use_cases::AiServiceProvider for MockAiServiceProvider {
        #[cfg(feature = "embeddings")]
        fn get_embedding_service(&self) -> Option<Arc<dyn ai::EmbeddingServiceTrait>> {
            None
        }

        #[cfg(feature = "reranking")]
        fn get_reranking_service(&self) -> Option<Arc<dyn ai::RerankingServiceTrait>> {
            None
        }

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        fn get_model_registry(&self) -> Option<Arc<ai::ModelRegistry>> {
            None
        }
    }

    #[cfg(feature = "test-utils")]
    fn create_test_ai_service() -> AiApplicationService {
        let cache_provider = Arc::new(MockCacheProvider::new());
        let metrics_collector = Arc::new(MockMetricsCollector::new());

        AiApplicationService::new(cache_provider, metrics_collector)
    }

    #[cfg(not(feature = "test-utils"))]
    fn create_test_ai_service() -> AiApplicationService {
        // Fallback for when test-utils feature is not enabled
        unimplemented!("MockCacheProvider and MockMetricsCollector require test-utils feature")
    }

    #[tokio::test]
    async fn test_ai_service_creation() -> Result<()> {
        let ai_service = create_test_ai_service();

        // Verify feature availability
        assert_eq!(
            ai_service.is_ai_available(),
            cfg!(any(feature = "cpu", feature = "gpu"))
        );
        assert_eq!(ai_service.is_gpu_available(), cfg!(feature = "gpu"));
        assert_eq!(
            ai_service.is_embedding_available(),
            cfg!(feature = "embeddings")
        );
        assert_eq!(
            ai_service.is_reranking_available(),
            cfg!(feature = "reranking")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_health_info_minimal_build() -> Result<()> {
        let ai_service = create_test_ai_service();
        let health = ai_service.get_health_info().await?;

        // In minimal build, AI should not be available
        if !cfg!(any(feature = "cpu", feature = "gpu")) {
            assert!(!health.ai_available);
            assert!(!health.warnings.is_empty());
            assert!(health
                .warnings
                .iter()
                .any(|w| w.contains("AI functionality not available")));
        }

        Ok(())
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    #[tokio::test]
    async fn test_list_models_use_case() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = ListModelsRequest {
            model_type: None,
            include_loaded: true,
            include_available: true,
        };

        let response = ai_service.list_models(request).await?;

        // Should return empty list initially
        assert_eq!(response.models.len(), 0);
        assert_eq!(response.loaded_count, 0);

        Ok(())
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    #[tokio::test]
    async fn test_load_model_use_case() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = LoadModelRequest {
            model_name: "test_model".to_string(),
            force_reload: false,
            prefer_gpu: false,
            custom_path: None,
            device_id: None,
        };

        let response = ai_service.load_model(request).await?;

        // Mock implementation should return success
        assert_eq!(response.model_name, "test_model");
        assert!(!response.device.is_empty());
        assert!(response.memory_usage_mb > 0.0);

        Ok(())
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    #[tokio::test]
    async fn test_inference_use_case() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = InferenceRequest {
            model_name: "test_embedding_model".to_string(),
            input: "Hello, world!".to_string(),
            batch_size: 1,
            top_k: None,
            temperature: None,
            max_tokens: None,
        };

        let response = ai_service.run_inference(request).await?;

        // Mock implementation should return appropriate response
        assert_eq!(response.model_name, "test_embedding_model");
        assert_eq!(response.result_type, "embedding");
        assert!(response.embedding.is_some());
        assert!(response.processing_time_ms >= 0.0);

        Ok(())
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    #[tokio::test]
    async fn test_ai_status_use_case() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = AiStatusRequest {
            include_models: true,
            include_system_info: true,
            include_performance_stats: true,
        };

        let response = ai_service.get_status(request).await?;

        // Should return system status
        assert!(response.system_healthy);
        assert_eq!(response.loaded_models_count, 0);
        assert!(response.gpu_available.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_feature_flag_compatibility() -> Result<()> {
        let ai_service = create_test_ai_service();

        // Test that feature flags work correctly
        #[cfg(not(any(feature = "cpu", feature = "gpu")))]
        {
            assert!(!ai_service.is_ai_available());

            let request = ListModelsRequest {
                model_type: None,
                include_loaded: true,
                include_available: true,
            };

            // Should fail in minimal build
            assert!(ai_service.list_models(request).await.is_err());
        }

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            assert!(ai_service.is_ai_available());

            let request = ListModelsRequest {
                model_type: None,
                include_loaded: true,
                include_available: true,
            };

            // Should succeed with AI features enabled
            assert!(ai_service.list_models(request).await.is_ok());
        }

        Ok(())
    }

    #[cfg(feature = "test-utils")]
    #[tokio::test]
    async fn test_use_case_factory() -> Result<()> {
        let cache_provider = Arc::new(MockCacheProvider::new());
        let metrics_collector = Arc::new(MockMetricsCollector::new());
        let service_provider = Arc::new(MockAiServiceProvider::new());

        let factory = AiUseCaseFactory::new(cache_provider, metrics_collector, service_provider);

        // Test factory creates use cases without panicking
        let _list_use_case = factory.create_list_models_use_case();
        let _load_use_case = factory.create_load_model_use_case();
        let _inference_use_case = factory.create_inference_use_case();
        let _status_use_case = factory.create_ai_status_use_case();

        Ok(())
    }

    #[cfg(feature = "embeddings")]
    #[tokio::test]
    async fn test_embedding_inference() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = InferenceRequest {
            model_name: "bge-m3".to_string(), // Embedding model name
            input: "This is a test document for embedding".to_string(),
            batch_size: 1,
            top_k: None,
            temperature: None,
            max_tokens: None,
        };

        let response = ai_service.run_inference(request).await?;

        assert_eq!(response.result_type, "embedding");
        assert!(response.embedding.is_some());

        if let Some(embedding) = response.embedding {
            assert!(!embedding.is_empty());
            // Typical embedding dimensions
            assert!(embedding.len() >= 256);
        }

        Ok(())
    }

    #[cfg(feature = "reranking")]
    #[tokio::test]
    async fn test_reranking_inference() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = InferenceRequest {
            model_name: "qwen3_reranker".to_string(), // Reranking model name
            input: "What is the capital of France?".to_string(),
            batch_size: 1,
            top_k: Some(5),
            temperature: None,
            max_tokens: None,
        };

        let response = ai_service.run_inference(request).await?;

        assert_eq!(response.result_type, "reranking");
        assert!(response.scores.is_some());

        if let Some(scores) = response.scores {
            assert!(!scores.is_empty());
            // Check scores are in valid range
            for score in scores {
                assert!(score >= 0.0 && score <= 1.0);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<()> {
        let ai_service = create_test_ai_service();

        // Test with invalid model name
        let request = InferenceRequest {
            model_name: "nonexistent_model".to_string(),
            input: "test input".to_string(),
            batch_size: 1,
            top_k: None,
            temperature: None,
            max_tokens: None,
        };

        // Should handle gracefully in mock implementation
        let response = ai_service.run_inference(request).await;

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            // Mock implementation should succeed
            assert!(response.is_ok());
        }

        #[cfg(not(any(feature = "cpu", feature = "gpu")))]
        {
            // Should fail in minimal build
            assert!(response.is_err());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_integration_placeholder() -> Result<()> {
        // Placeholder test for AI-Memory integration
        // This will be expanded when Memory system integration is implemented

        let ai_service = create_test_ai_service();
        let health = ai_service.get_health_info().await?;

        // For now, just verify the service is working
        assert!(health.memory_usage_mb >= 0.0);

        println!("🔄 Memory integration test placeholder - will be expanded");

        Ok(())
    }

    #[cfg(feature = "gpu")]
    #[tokio::test]
    async fn test_gpu_acceleration() -> Result<()> {
        let ai_service = create_test_ai_service();

        let request = LoadModelRequest {
            model_name: "gpu_test_model".to_string(),
            force_reload: false,
            prefer_gpu: true, // Request GPU
            custom_path: None,
            device_id: None,
        };

        let response = ai_service.load_model(request).await?;

        // Mock implementation should handle GPU preference
        assert_eq!(response.model_name, "gpu_test_model");
        // In real implementation, would check if device is GPU

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_requests() -> Result<()> {
        let ai_service = Arc::new(create_test_ai_service());

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            let mut handles = Vec::new();

            // Spawn multiple concurrent requests
            for i in 0..5 {
                let service = ai_service.clone();
                let handle = tokio::spawn(async move {
                    let request = InferenceRequest {
                        model_name: format!("test_model_{}", i),
                        input: format!("test input {}", i),
                        batch_size: 1,
                        top_k: None,
                        temperature: None,
                        max_tokens: None,
                    };

                    service.run_inference(request).await
                });
                handles.push(handle);
            }

            // Wait for all requests to complete
            for handle in handles {
                let result = handle.await??;
                assert!(result.processing_time_ms >= 0.0);
            }
        }

        Ok(())
    }
}

// Integration tests that require actual AI crate functionality
#[cfg(all(test, any(feature = "cpu", feature = "gpu")))]
mod ai_real_integration_tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_model_registry_integration() -> anyhow::Result<()> {
        // Test with actual ModelRegistry from ai crate
        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            use ai::ModelRegistry;

            let registry =
                std::sync::Arc::new(ModelRegistry::new(std::path::PathBuf::from("./models")));

            // Test basic registry functionality
            let available_models = registry.get_available_models(None);
            // let loaded_models = registry.get_loaded_models(); // Method not available

            // Initially should have no loaded models
            // assert_eq!(loaded_models.len(), 0);

            println!("Available models: {}", available_models.len());
            // println!("Loaded models: {}", loaded_models.len());
        }

        Ok(())
    }

    #[cfg(feature = "onnx")]
    #[tokio::test]
    async fn test_onnx_runtime_availability() -> Result<()> {
        use ai::{ort_available, should_disable_ort};

        let ort_enabled = ort_available() && !should_disable_ort();
        println!("ONNX Runtime available: {}", ort_enabled);

        // Test should pass regardless of ONNX availability
        Ok(())
    }

    #[tokio::test]
    async fn test_warmup_models() -> anyhow::Result<()> {
        #[cfg(feature = "onnx")]
        {
            use ai::warmup_models;

            // Test model warmup (should handle missing models gracefully)
            let result = warmup_models(&["test_model"]);

            // Should not fail even if models don't exist
            assert!(result.is_ok());
        }

        Ok(())
    }
}
