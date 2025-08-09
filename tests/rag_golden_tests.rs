#![cfg(feature = "extended-tests")]

use memory::api::{UnifiedMemoryAPI, MemoryContext, SearchOptions};
use memory::di::UnifiedContainer;
use memory::types::Layer;
use std::sync::Arc;
use std::time::Instant;

fn dcg(relevances: &[u8]) -> f64 {
    relevances
        .iter()
        .enumerate()
        .map(|(i, &rel)| {
            if i == 0 { rel as f64 } else { (rel as f64) / ((i as f64 + 1.0).log2()) }
        })
        .sum()
}

fn ndcg_at_k(ideal: &[u8], predicted: &[u8], k: usize) -> f64 {
    let k = k.min(predicted.len()).min(ideal.len());
    let idcg = dcg(&ideal[..k]);
    if idcg == 0.0 { return 0.0; }
    let dcg_v = dcg(&predicted[..k]);
    dcg_v / idcg
}

#[tokio::test]
async fn rag_recall_smoke_golden() {
    let api = UnifiedMemoryAPI::new(Arc::new(UnifiedContainer::new()));

    // Seed small corpus with labels for relevance to query "async runtime in rust"
    let docs = vec![
        ("rust ownership and borrowing rules", Layer::Insights, 0u8),
        ("tokio provides async runtime for rust", Layer::Insights, 3u8),
        ("onnx runtime loads qwen3 models", Layer::Assets, 0u8),
        ("hnsw index enables fast nearest neighbor search", Layer::Insights, 0u8),
        ("sqlite persistence ensures data durability", Layer::Assets, 0u8),
        ("asynchronous tasks with tokio and futures", Layer::Insights, 2u8),
    ];
    for (text, layer, _rel) in &docs {
        let _ = api
            .remember((*text).to_string(), MemoryContext::new("note").with_layer(*layer))
            .await;
    }

    let start = Instant::now();
    let results = api
        .recall(
            "async runtime in rust",
            SearchOptions::default().in_layers(vec![Layer::Insights]).limit(5),
        )
        .await
        .expect("search ok");
    let elapsed_ms = start.elapsed().as_millis();

    // Build relevance vectors
    let gold_map: std::collections::HashMap<&str, u8> = docs
        .iter()
        .map(|(t, _, rel)| (*t, *rel))
        .collect();

    let predicted_rel: Vec<u8> = results
        .iter()
        .map(|r| *gold_map.get(r.text.as_str()).unwrap_or(&0))
        .collect();

    let ideal_rel: Vec<u8> = {
        let mut v: Vec<u8> = docs
            .iter()
            .filter(|(_, l, _)| *l == Layer::Insights)
            .map(|(_, _, rel)| *rel)
            .collect();
        v.sort_by(|a, b| b.cmp(a));
        v
    };

    // Compute metrics
    let k = results.len().min(5);
    let hits = predicted_rel.iter().filter(|&&r| r > 0).count() as f64;
    let precision = if k > 0 { hits / k as f64 } else { 0.0 };
    let recall_den = docs.iter().filter(|(t, l, r)| *l == Layer::Insights && *r > 0 && gold_map.contains_key(*t)).count() as f64;
    let recall = if recall_den > 0.0 { hits / recall_den } else { 0.0 };
    let ndcg = if !ideal_rel.is_empty() && !predicted_rel.is_empty() { ndcg_at_k(&ideal_rel, &predicted_rel, k) } else { 0.0 };

    // Golden expectations for mock embeddings path: should retrieve at least one relevant doc and decent ordering
    assert!(precision >= 0.2, "precision too low: {}", precision);
    assert!(recall >= 0.5, "recall too low: {}", recall);
    assert!(ndcg >= 0.4, "ndcg too low: {}", ndcg);

    // Latency budget (mock path should be fast)
    assert!(elapsed_ms < 2000, "latency too high: {}ms", elapsed_ms);
}