#![cfg(feature = "extended-tests")]

use memory::api::{UnifiedMemoryAPI, MemoryContext, SearchOptions};
use memory::di::UnifiedContainer;
use memory::types::Layer;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn rag_recall_smoke_golden() {
    // Arrange: build API with mock/real embeddings (falls back to mock if ORT not available)
    let api = UnifiedMemoryAPI::new(Arc::new(UnifiedContainer::new()));

    // Seed small corpus
    let docs = vec![
        ("rust ownership and borrowing rules", Layer::Insights),
        ("tokio provides async runtime for rust", Layer::Insights),
        ("onnx runtime loads qwen3 models", Layer::Assets),
        ("hnsw index enables fast nearest neighbor search", Layer::Insights),
        ("sqlite persistence ensures data durability", Layer::Assets),
    ];
    for (text, layer) in &docs {
        let _ = api
            .remember((*text).to_string(), MemoryContext::new("note").with_layer(*layer))
            .await;
    }

    // Act: query and measure latency
    let start = Instant::now();
    let results = api
        .recall(
            "async runtime in rust",
            SearchOptions::default().in_layers(vec![Layer::Insights]).limit(3),
        )
        .await
        .expect("search ok");
    let elapsed_ms = start.elapsed().as_millis();

    // Assert: recall@k should contain at least one semantically close doc
    let joined = results.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" | ");
    assert!(
        joined.contains("tokio") || joined.contains("async runtime"),
        "expected async/tokio related doc in top-k, got: {}",
        joined
    );

    // Assert: latency budget (mock path should be fast)
    assert!(elapsed_ms < 2000, "latency too high: {}ms", elapsed_ms);
}