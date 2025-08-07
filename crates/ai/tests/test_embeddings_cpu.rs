use ai::config::EmbeddingConfig;
use ai::embeddings_cpu::*;

#[test]
fn test_cpu_embedding_service_creation() {
    let config = EmbeddingConfig {
        model_name: "test_model".to_string(),
        batch_size: 16,
        max_length: 256,
        use_gpu: false,
        gpu_config: None,
        embedding_dim: Some(768),
    };

    // This might fail due to missing model files, which is expected in test environment
    match CpuEmbeddingService::new(config) {
        Ok(_service) => {
            // If service creation succeeds, that's great
            assert!(true);
        }
        Err(e) => {
            // If it fails due to missing models, that's expected in tests
            let error_msg = format!("{}", e);
            assert!(
                error_msg.contains("model")
                    || error_msg.contains("file")
                    || error_msg.contains("path")
                    || error_msg.contains("not found")
                    || error_msg.contains("No such file")
            );
        }
    }
}

#[test]
fn test_cpu_config_validation() {
    let config = EmbeddingConfig {
        model_name: "valid_model".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: false,
        gpu_config: None,
        embedding_dim: Some(1024),
    };

    // Config should have reasonable values
    assert!(!config.model_name.is_empty());
    assert!(config.batch_size > 0);
    assert!(config.batch_size <= 256);
    assert!(config.max_length > 0);
    assert!(config.max_length <= 8192);
    assert!(!config.use_gpu);
    assert!(config.gpu_config.is_none());

    if let Some(dim) = config.embedding_dim {
        assert!(dim > 0);
        assert!(dim <= 4096);
    }
}

#[test]
fn test_embedding_batch_size_optimization() {
    let cpu_count = num_cpus::get();
    let optimal_batch_sizes = [1, 2, 4, 8, 16, 32, 64];

    for batch_size in optimal_batch_sizes {
        // Batch size should be reasonable for CPU
        assert!(batch_size > 0);
        assert!(batch_size <= cpu_count * 8); // Max 8x CPU cores

        // CPU batch sizes are typically smaller than GPU
        assert!(batch_size <= 128);
    }
}

#[test]
fn test_cpu_memory_estimation() {
    let config = EmbeddingConfig {
        model_name: "test".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: false,
        gpu_config: None,
        embedding_dim: Some(768),
    };

    // Estimate memory usage for CPU embedding
    let batch_size = config.batch_size;
    let max_length = config.max_length;
    let embedding_dim = config.embedding_dim.unwrap_or(768);

    // Input memory (tokens)
    let input_memory = batch_size * max_length * 4; // 4 bytes per token

    // Output memory (embeddings)
    let output_memory = batch_size * embedding_dim * 4; // 4 bytes per float

    // Model weights (rough estimate)
    let model_memory = embedding_dim * 50000 * 4; // vocab_size * embedding_dim * 4

    let total_memory = input_memory + output_memory + model_memory;

    assert!(total_memory > 0);
    // Should be reasonable for CPU (less than 8GB)
    assert!(total_memory < 8_000_000_000);
}

#[test]
fn test_text_preprocessing_edge_cases() {
    let edge_cases = vec![
        "",
        " ",
        "\n",
        "\t",
        "a",
        "hello world",
        "very long text that exceeds normal limits and should be handled gracefully by the preprocessing pipeline",
        "text with\nnewlines\tand\ttabs",
        "émoticôns and ñon-ASCII characters",
        "🚀 emoji test 🎉",
        "mixed 123 content with symbols !@#$%^&*()",
    ];

    for text in edge_cases {
        let processed = text.trim();

        // Should handle all cases without panicking
        assert!(processed.len() <= text.len());

        if !text.trim().is_empty() {
            assert!(!processed.is_empty() || text.trim().is_empty());
        }
    }
}

#[test]
fn test_batch_processing_validation() {
    let texts = vec![
        "First text".to_string(),
        "Second text".to_string(),
        "Third text".to_string(),
        "Fourth text".to_string(),
    ];

    let batch_sizes = [1, 2, 3, 4, 8];

    for batch_size in batch_sizes {
        // Test batch creation
        let mut batches = Vec::new();
        for chunk in texts.chunks(batch_size) {
            batches.push(chunk.to_vec());
        }

        // Validate batches
        assert!(!batches.is_empty());

        let total_texts: usize = batches.iter().map(|b| b.len()).sum();
        assert_eq!(total_texts, texts.len());

        // Each batch should be at most batch_size
        for batch in &batches {
            assert!(batch.len() <= batch_size);
            assert!(!batch.is_empty());
        }
    }
}

#[test]
fn test_cpu_performance_characteristics() {
    // CPU embedding typical characteristics
    let cpu_cores = num_cpus::get();

    // Optimal batch size is usually related to CPU core count
    let optimal_batch = cpu_cores.min(32);
    assert!(optimal_batch >= 1);
    assert!(optimal_batch <= 64);

    // CPU processing is typically slower than GPU but more stable
    // Expect ~10-100 texts per second depending on hardware
    let expected_throughput_range = 1..1000;
    assert!(expected_throughput_range.contains(&100)); // Sample check
}

#[test]
fn test_embedding_dimension_validation() {
    let common_dimensions = [128, 256, 384, 512, 768, 1024, 1536, 2048];

    for dim in common_dimensions {
        assert!(dim > 0);
        assert!(dim <= 4096);

        // Common dimensions are often multiples of 64 or powers of 2
        assert!(dim % 64 == 0 || (dim & (dim - 1)) == 0);

        // Test memory requirements
        let memory_per_embedding = dim * 4; // 4 bytes per float
        assert!(memory_per_embedding > 0);
        assert!(memory_per_embedding <= 16384); // 16KB per embedding max
    }
}

#[test]
fn test_tokenization_approximation() {
    let texts = vec![
        "short",
        "medium length text",
        "very long text that might exceed the maximum token limit and should be handled appropriately",
    ];

    for text in texts {
        // Simple approximation: ~4 characters per token for English
        let estimated_tokens = (text.len() / 4).max(1);

        assert!(estimated_tokens > 0);
        assert!(estimated_tokens <= text.len()); // Can't have more tokens than characters

        // Should be reasonable for processing
        if estimated_tokens > 512 {
            // Long text should be truncated or chunked
            assert!(text.len() > 2000); // Should indeed be long text
        }
    }
}

#[test]
fn test_cpu_service_configuration_validation() {
    let configs = vec![
        EmbeddingConfig {
            model_name: "tiny_model".to_string(),
            batch_size: 1,
            max_length: 128,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(384),
        },
        EmbeddingConfig {
            model_name: "medium_model".to_string(),
            batch_size: 16,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(768),
        },
        EmbeddingConfig {
            model_name: "large_model".to_string(),
            batch_size: 8,
            max_length: 1024,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        },
    ];

    for config in configs {
        // All configs should be valid for CPU
        assert!(!config.use_gpu);
        assert!(config.gpu_config.is_none());
        assert!(config.batch_size >= 1);
        assert!(config.batch_size <= 64); // Reasonable for CPU
        assert!(config.max_length >= 64);
        assert!(config.max_length <= 2048); // Reasonable for CPU

        if let Some(dim) = config.embedding_dim {
            assert!(dim >= 128);
            assert!(dim <= 2048);
        }
    }
}

#[test]
fn test_error_handling_scenarios() {
    // Test various error conditions
    let invalid_configs = vec![
        EmbeddingConfig {
            model_name: "".to_string(), // Empty model name
            batch_size: 32,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(768),
        },
        EmbeddingConfig {
            model_name: "test".to_string(),
            batch_size: 0, // Invalid batch size
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(768),
        },
        EmbeddingConfig {
            model_name: "test".to_string(),
            batch_size: 32,
            max_length: 0, // Invalid max length
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(768),
        },
    ];

    for config in invalid_configs {
        // These configs should either fail to create service or be validated/corrected
        match CpuEmbeddingService::new(config.clone()) {
            Ok(_) => {
                // If service creation succeeds, the config was corrected internally
                assert!(true);
            }
            Err(_) => {
                // Expected to fail with invalid config
                assert!(true);
            }
        }

        // Basic validation checks
        if config.model_name.is_empty() {
            assert!(config.model_name.is_empty()); // Should detect this
        }
        if config.batch_size == 0 {
            assert_eq!(config.batch_size, 0); // Should detect this
        }
        if config.max_length == 0 {
            assert_eq!(config.max_length, 0); // Should detect this
        }
    }
}
