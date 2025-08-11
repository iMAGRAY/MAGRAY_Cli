use ai::{RerankingConfig, RerankingService};

#[test]
fn test_reranker_initialization() {
    // Set ONNX Runtime path
    std::env::set_var(
        "ORT_DYLIB_PATH",
        "../../../scripts/onnxruntime/lib/onnxruntime.dll",
    );

    let config = RerankingConfig {
        model_name: "BGE-reranker-v2-m3".to_string(),
        batch_size: 4,
        max_length: 512,
        use_gpu: false,
        gpu_config: None,
    };

    match RerankingService::new(&config) {
        Ok(_) => {
            println!("✅ Reranker initialized successfully!");
        }
        Err(e) => {
            println!("❌ Failed to initialize reranker: {}", e);
        }
    }
}

#[test]
fn test_reranker_mock() {
    let config = RerankingConfig {
        model_name: "test-model".to_string(),
        batch_size: 4,
        max_length: 512,
        use_gpu: false,
        gpu_config: None,
    };

    let reranker = RerankingService::new(&config).unwrap();

    let query = "machine learning";
    let documents = vec!["AI is great".to_string(), "Pizza is tasty".to_string()];

    let results = reranker.rerank(query, &documents).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results[0].score >= 0.0 && results[0].score <= 1.0);
}
