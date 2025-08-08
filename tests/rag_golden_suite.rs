#![cfg(feature = "extended-tests")]

use memory::api::{UnifiedMemoryAPI, MemoryContext, SearchOptions};
use memory::di::UnifiedContainer;
use memory::types::Layer;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn dcg(relevances: &[u8]) -> f64 {
    relevances
        .iter()
        .enumerate()
        .map(|(i, &rel)| if i == 0 { rel as f64 } else { (rel as f64) / ((i as f64 + 1.0).log2()) })
        .sum()
}

fn ndcg_at_k(ideal: &[u8], predicted: &[u8], k: usize) -> f64 {
    let k = k.min(predicted.len()).min(ideal.len());
    let idcg = dcg(&ideal[..k]);
    if idcg == 0.0 { return 0.0; }
    let dcg_v = dcg(&predicted[..k]);
    dcg_v / idcg
}

fn average_precision(predicted: &[u8]) -> f64 {
    let mut hits = 0u32;
    let mut sum_prec = 0.0;
    for (i, &rel) in predicted.iter().enumerate() {
        if rel > 0 {
            hits += 1;
            let prec = hits as f64 / (i as f64 + 1.0);
            sum_prec += prec;
        }
    }
    if hits == 0 { 0.0 } else { sum_prec / hits as f64 }
}

#[tokio::test]
async fn rag_golden_suite_metrics() {
    let api = UnifiedMemoryAPI::new(Arc::new(UnifiedContainer::new()));

    // Corpus
    let docs = vec![
        ("rust ownership and borrowing rules", Layer::Insights),
        ("tokio async runtime for rust applications", Layer::Insights),
        ("onnx runtime loads qwen3 models for inference", Layer::Assets),
        ("hnsw index enables fast approximate nearest neighbor search", Layer::Insights),
        ("sqlite persistence ensures data durability and ACID", Layer::Assets),
        ("introducing async/await and futures in rust", Layer::Insights),
        ("reranking with qwen3 to improve relevance", Layer::Insights),
        ("vector embeddings and cosine similarity basics", Layer::Insights),
        ("setting up onnxruntime and environment variables", Layer::Assets),
        ("rayon and multicore parallelism in rust", Layer::Insights),
    ];

    for (text, layer) in &docs {
        let _ = api
            .remember((*text).to_string(), MemoryContext::new("note").with_layer(*layer))
            .await;
    }

    // Queries with expected relevant phrases (labels)
    let queries: Vec<(&str, HashSet<&str>)> = vec![
        (
            "async runtime in rust",
            HashSet::from(["tokio", "async/await", "futures"]),
        ),
        (
            "nearest neighbor index",
            HashSet::from(["hnsw"]),
        ),
        (
            "onnx runtime setup",
            HashSet::from(["onnxruntime", "onnx"]),
        ),
        (
            "vector embeddings similarity",
            HashSet::from(["embeddings", "cosine"]),
        ),
    ];

    let mut ap_sum = 0.0;
    let mut prec_sum = 0.0;
    let mut rec_sum = 0.0;
    let mut ndcg_sum = 0.0;
    let mut count = 0.0;

    for (q, labels) in queries {
        let results = api
            .recall(
                q,
                SearchOptions::default()
                    .in_layers(vec![Layer::Insights])
                    .limit(5),
            )
            .await
            .expect("search ok");

        // Build predicted relevance from labels occurrence in text
        let predicted_rel: Vec<u8> = results
            .iter()
            .map(|r| {
                let lower = r.text.to_lowercase();
                let mut score = 0u8;
                for lab in &labels {
                    if lower.contains(&lab.to_lowercase()) {
                        // multi-label credit
                        score = score.saturating_add(1);
                    }
                }
                score
            })
            .collect();

        // Ideal relevance vector: sorted label counts for the corpus' insight docs
        let mut ideal_rel: Vec<u8> = docs
            .iter()
            .filter(|(_, l)| *l == Layer::Insights)
            .map(|(t, _)| {
                let lower = t.to_lowercase();
                let mut score = 0u8;
                for lab in &labels {
                    if lower.contains(&lab.to_lowercase()) {
                        score = score.saturating_add(1);
                    }
                }
                score
            })
            .collect();
        ideal_rel.sort_by(|a, b| b.cmp(a));

        let k = results.len().min(5);
        let hits = predicted_rel.iter().filter(|&&r| r > 0).count() as f64;
        let precision = if k > 0 { hits / k as f64 } else { 0.0 };
        let recall_den = ideal_rel.iter().filter(|&&r| r > 0).count() as f64;
        let recall = if recall_den > 0.0 { hits / recall_den } else { 0.0 };
        let ndcg = if !ideal_rel.is_empty() && !predicted_rel.is_empty() {
            ndcg_at_k(&ideal_rel, &predicted_rel, k)
        } else {
            0.0
        };
        let ap = average_precision(&predicted_rel);

        ap_sum += ap;
        prec_sum += precision;
        rec_sum += recall;
        ndcg_sum += ndcg;
        count += 1.0;
    }

    let ap_avg = ap_sum / count;
    let prec_avg = prec_sum / count;
    let rec_avg = rec_sum / count;
    let ndcg_avg = ndcg_sum / count;

    // Golden expectations tuned for mock embeddings
    assert!(prec_avg >= 0.3, "avg precision too low: {}", prec_avg);
    assert!(rec_avg >= 0.5, "avg recall too low: {}", rec_avg);
    assert!(ndcg_avg >= 0.5, "avg ndcg too low: {}", ndcg_avg);
    assert!(ap_avg >= 0.3, "avg AP too low: {}", ap_avg);

    // Print and persist report locally
    println!(
        "RAG_METRICS_SUMMARY: precision_avg={:.3} recall_avg={:.3} ndcg_avg={:.3} ap_avg={:.3}",
        prec_avg, rec_avg, ndcg_avg, ap_avg
    );
    let _ = std::fs::create_dir_all("reports");
    let json = format!(
        "{{\n  \"precision_avg\": {:.6},\n  \"recall_avg\": {:.6},\n  \"ndcg_avg\": {:.6},\n  \"ap_avg\": {:.6}\n}}",
        prec_avg, rec_avg, ndcg_avg, ap_avg
    );
    let _ = std::fs::write("reports/rag_metrics_summary.json", json);
}