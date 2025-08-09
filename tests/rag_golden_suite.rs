#![cfg(feature = "extended-tests")]

use memory::api::{UnifiedMemoryAPI, MemoryContext, SearchOptions};
use memory::di::UnifiedContainer;
use memory::types::Layer;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

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
        ("tantivy bm25 keyword search complements ANN", Layer::Insights),
        ("qwen3 reranker 0.6B improves ranking quality", Layer::Insights),
        ("sqlite backups and restore procedures", Layer::Assets),
        ("hnsw parameters efConstruction and M affect recall", Layer::Insights),
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
        (
            "bm25 keyword engine",
            HashSet::from(["tantivy", "bm25"]),
        ),
        (
            "backup and restore sqlite",
            HashSet::from(["sqlite", "backup", "restore"]),
        ),
        (
            "qwen3 reranker quality",
            HashSet::from(["qwen3", "reranker", "0.6b"]),
        ),
    ];

    // baseline metrics
    let mut ap_sum = 0.0;
    let mut prec_sum = 0.0;
    let mut rec_sum = 0.0;
    let mut ndcg_sum = 0.0;
    let mut count = 0.0;

    // latency tracking
    let mut per_query_latencies_ms: Vec<u128> = Vec::new();

    for (q, labels) in queries {
        let t0 = Instant::now();
        let results = api
            .recall(
                q,
                SearchOptions::default()
                    .in_layers(vec![Layer::Insights])
                    .limit(5),
            )
            .await
            .expect("search ok");
        let elapsed_ms = t0.elapsed().as_millis();
        per_query_latencies_ms.push(elapsed_ms);

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

    // Latency SLOs (mock path should be fast)
    let p95_ms = {
        let mut v = per_query_latencies_ms.clone();
        v.sort();
        let idx = ((v.len() as f64) * 0.95).ceil() as usize - 1;
        v[idx.min(v.len() - 1)]
    };
    let avg_latency_ms: f64 = if per_query_latencies_ms.is_empty() {
        0.0
    } else {
        per_query_latencies_ms.iter().copied().map(|x| x as f64).sum::<f64>() / per_query_latencies_ms.len() as f64
    };

    // Golden expectations tuned for mock embeddings (slightly stricter)
    assert!(prec_avg >= 0.28, "avg precision too low: {}", prec_avg);
    assert!(rec_avg >= 0.48, "avg recall too low: {}", rec_avg);
    assert!(ndcg_avg >= 0.48, "avg ndcg too low: {}", ndcg_avg);
    assert!(ap_avg >= 0.28, "avg AP too low: {}", ap_avg);

    // Latency budgets
    assert!(p95_ms < 1800, "p95 latency too high: {}ms", p95_ms);
    assert!(avg_latency_ms < 1000.0, "avg latency too high: {:.1}ms", avg_latency_ms);

    // Print and persist report locally
    println!(
        "RAG_METRICS_SUMMARY: precision_avg={:.3} recall_avg={:.3} ndcg_avg={:.3} ap_avg={:.3} p95_ms={} avg_ms={:.1}",
        prec_avg, rec_avg, ndcg_avg, ap_avg, p95_ms, avg_latency_ms
    );
    let _ = std::fs::create_dir_all("reports");
    let json = format!(
        "{{\n  \"precision_avg\": {:.6},\n  \"recall_avg\": {:.6},\n  \"ndcg_avg\": {:.6},\n  \"ap_avg\": {:.6},\n  \"p95_ms\": {},\n  \"avg_ms\": {:.6}\n}}",
        prec_avg, rec_avg, ndcg_avg, ap_avg, p95_ms, avg_latency_ms
    );
    let _ = std::fs::write("reports/rag_metrics_summary.json", json);

    // === RERANK VARIANT ===
    std::env::set_var("MAGRAY_DISABLE_RERANK", "0");
    std::env::set_var("MAGRAY_FORCE_NO_ORT", "1");
    let t_start = Instant::now();
    let mut ap2 = 0.0f64; let mut prec2 = 0.0f64; let mut rec2 = 0.0f64; let mut ndcg2 = 0.0f64; let mut cnt2 = 0.0f64;
    let mut lat_rerank: Vec<u128> = Vec::new();
    for (q, labels) in vec![
        ("qwen3 reranker quality", HashSet::from(["qwen3","reranker"])) ,
        ("async runtime in rust", HashSet::from(["tokio","async"])) ,
    ] {
        let t0 = Instant::now();
        // emulate rerank by using same API (our in-memory engine already applies rerank if enabled)
        let results = api
            .recall(q, SearchOptions::default().in_layers(vec![Layer::Insights]).limit(5))
            .await
            .expect("search ok");
        lat_rerank.push(t0.elapsed().as_millis());

        let predicted: Vec<u8> = results.iter().map(|r| {
            let lower = r.text.to_lowercase(); let mut s=0u8; for lab in &labels { if lower.contains(&lab.to_lowercase()) { s=s.saturating_add(1);} } s
        }).collect();

        let mut ideal: Vec<u8> = docs.iter().filter(|(_,l)| *l==Layer::Insights).map(|(t,_)| {
            let lower=t.to_lowercase(); let mut s=0u8; for lab in &labels { if lower.contains(&lab.to_lowercase()) { s=s.saturating_add(1);} } s
        }).collect();
        ideal.sort_by(|a,b| b.cmp(a));
        let k = results.len().min(5);
        let hits = predicted.iter().filter(|&&r| r>0).count() as f64;
        let p = if k>0 { hits/k as f64 } else { 0.0 };
        let rd = ideal.iter().filter(|&&r| r>0).count() as f64;
        let r = if rd>0.0 { hits/rd } else { 0.0 };
        let nd = if !ideal.is_empty() && !predicted.is_empty() { ndcg_at_k(&ideal, &predicted, k) } else { 0.0 };
        let apx = average_precision(&predicted);
        ap2+=apx; prec2+=p; rec2+=r; ndcg2+=nd; cnt2+=1.0;
    }
    let ap2_avg = ap2/cnt2; let prec2_avg = prec2/cnt2; let rec2_avg = rec2/cnt2; let ndcg2_avg = ndcg2/cnt2;
    let p95_r = { let mut v=lat_rerank.clone(); v.sort(); let idx=((v.len() as f64)*0.95).ceil() as usize - 1; v[idx.min(v.len()-1)] };
    let avg_r = if lat_rerank.is_empty() { 0.0 } else { lat_rerank.iter().copied().map(|x| x as f64).sum::<f64>() / lat_rerank.len() as f64 };
    // Expect rerank to not degrade precision/ndcg on focused queries
    assert!(prec2_avg >= 0.30, "rerank precision too low: {}", prec2_avg);
    assert!(ndcg2_avg >= 0.50, "rerank ndcg too low: {}", ndcg2_avg);
    let json2 = format!(
        "{{\n  \"precision_avg\": {:.6},\n  \"recall_avg\": {:.6},\n  \"ndcg_avg\": {:.6},\n  \"ap_avg\": {:.6},\n  \"p95_ms\": {},\n  \"avg_ms\": {:.6}\n}}",
        prec2_avg, rec2_avg, ndcg2_avg, ap2_avg, p95_r, avg_r
    );
    let _ = std::fs::write("reports/rag_metrics_rerank.json", json2);
}