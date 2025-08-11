#![cfg(all(not(feature = "minimal"), feature = "persistence"))]
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memory::storage::VectorStore;
use memory::{Layer, Record};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

const DIMENSIONS: usize = 1024; // Qwen3 actual dimension
const BATCH_SIZES: &[usize] = &[1, 10, 50, 100, 500, 1000];
const SEARCH_LIMITS: &[usize] = &[1, 5, 10, 50, 100];

fn create_test_record(id: &str, dimension: usize) -> Record {
    let embedding: Vec<f32> = (0..dimension).map(|i| (i as f32) * 0.001).collect();
    Record {
        id: Uuid::new_v4(),
        text: format!("Test content for {}", id),
        embedding,
        layer: Layer::Interact,
        kind: "test".to_string(),
        tags: vec!["benchmark".to_string()],
        project: "benchmark".to_string(),
        session: "test_session".to_string(),
        ts: chrono::Utc::now(),
        access_count: 1,
        last_access: chrono::Utc::now(),
        score: 0.0,
    }
}

fn bench_vector_store_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Test vector store operations with different batch sizes
    let mut group = c.benchmark_group("vector_store_operations");

    for &batch_size in BATCH_SIZES {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("batch_insert", batch_size),
            &batch_size,
            |b, &size| {
                b.to_async(&rt).iter_batched(
                    || {
                        // Setup: create fresh store and records
                        let temp_dir = tempfile::tempdir().unwrap();
                        let records: Vec<_> = (0..size)
                            .map(|i| create_test_record(&format!("test_{}", i), DIMENSIONS))
                            .collect();
                        (temp_dir, records)
                    },
                    |(temp_dir, records)| async move {
                        let store = VectorStore::new(temp_dir.path()).await.unwrap();
                        let record_refs: Vec<_> = records.iter().collect();
                        store.insert_batch(&record_refs).await.unwrap();
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_hnsw_search_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let store = rt.block_on(async {
        let store = VectorStore::new(temp_dir.path()).await.unwrap();

        let records: Vec<_> = (0..10000)
            .map(|i| create_test_record(&format!("search_test_{}", i), DIMENSIONS))
            .collect();
        let record_refs: Vec<_> = records.iter().collect();
        store.insert_batch(&record_refs).await.unwrap();
        store
    });

    let query_embedding: Vec<f32> = (0..DIMENSIONS).map(|i| (i as f32) * 0.001).collect();

    let mut group = c.benchmark_group("hnsw_search");

    for &limit in SEARCH_LIMITS {
        group.throughput(Throughput::Elements(1)); // One search query

        group.bench_with_input(
            BenchmarkId::new("search", limit),
            &limit,
            |b, &search_limit| {
                b.to_async(&rt).iter(|| async {
                    store
                        .search(&query_embedding, Layer::Interact, search_limit)
                        .await
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("embedding_cache");

    for &batch_size in &[1, 10, 100, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("lru_cache_operations", batch_size),
            &batch_size,
            |b, &size| {
                b.to_async(&rt).iter_batched(
                    || {
                        // Setup: create fresh cache
                        let temp_dir = tempfile::tempdir().unwrap();
                        let config = CacheConfig::default();
                        let cache = EmbeddingCache::new(temp_dir.path(), config).unwrap();

                        let test_data: Vec<_> = (0..size)
                            .map(|i| {
                                let embedding: Vec<f32> =
                                    (0..DIMENSIONS).map(|j| j as f32 * 0.001).collect();
                                (
                                    format!("test_key_{}", i),
                                    embedding,
                                    "test_model".to_string(),
                                )
                            })
                            .collect();

                        (cache, test_data)
                    },
                    |(cache, test_data)| async move {
                        // Benchmark: insert and retrieve
                        for (key, embedding, model) in &test_data {
                            let _ = cache.insert(key, model, embedding.clone());
                        }

                        for (key, _, model) in &test_data {
                            let _ = cache.get(key, model);
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_promotion_engine(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("promotion_engine");

    group.bench_function("ml_promotion_cycle", |b| {
        b.to_async(&rt).iter_batched(
            || {
                // Setup: create store with test data
                let temp_dir = tempfile::tempdir().unwrap();
                rt.block_on(async {
                    let store = VectorStore::new(temp_dir.path()).await.unwrap();

                    let records: Vec<_> = (0..1000)
                        .map(|i| {
                            let mut record =
                                create_test_record(&format!("promote_test_{}", i), DIMENSIONS);
                            record.access_count = (i % 10) as u32 + 1; // Varying access counts
                            record
                        })
                        .collect();
                    let record_refs: Vec<_> = records.iter().collect();
                    store.insert_batch(&record_refs).await.unwrap();

                    let config = MLPromotionConfig::default();
                    rt.block_on(async {
                        MLPromotionEngine::new(std::sync::Arc::new(store), config)
                            .await
                            .unwrap()
                    })
                })
            },
            |mut engine| async move {
                // Benchmark: run promotion cycle
                engine.run_ml_promotion_cycle().await.unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_memory_service_integration(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_service_integration");

    group.bench_function("full_workflow", |b| {
        b.to_async(&rt).iter_batched(
            || {
                // Setup: create full memory service
                rt.block_on(async {
                    let config = default_config().unwrap();
                    let service = DIMemoryService::new(config).await.unwrap();
                    service.initialize().await.unwrap();

                    let test_records: Vec<_> = (0..100)
                        .map(|i| create_test_record(&format!("integration_test_{}", i), DIMENSIONS))
                        .collect();

                    (service, test_records)
                })
            },
            |(service, records)| async move {
                // Benchmark: full workflow
                for record in &records {
                    service.insert(record.clone()).await.unwrap();
                }

                let query = "test content";
                let search_options = SearchOptions {
                    layers: vec![Layer::Interact],
                    top_k: 10,
                    score_threshold: 0.5,
                    tags: vec![],
                    project: Some("benchmark".to_string()),
                };
                let search_results = service
                    .search(query, Layer::Interact, search_options)
                    .await
                    .unwrap();

                // Verify results
                assert!(!search_results.is_empty());
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    name = comprehensive_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(30))
        .sample_size(50)
        .warm_up_time(Duration::from_secs(5));
    targets =
        bench_vector_store_operations,
        bench_hnsw_search_performance,
        bench_cache_performance,
        bench_promotion_engine,
        bench_memory_service_integration
);

criterion_main!(comprehensive_benches);
