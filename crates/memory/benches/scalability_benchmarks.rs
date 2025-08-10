#![cfg(all(not(feature = "minimal"), feature = "hnsw-index", feature = "persistence"))]
use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, PlotConfiguration,
};
use memory::{HnswRsConfig, Layer, Record, VectorIndexHnswRs, VectorStore};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Generate synthetic embeddings for testing
fn generate_embedding(dim: usize, seed: f32) -> Vec<f32> {
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha8Rng;

    let mut rng = ChaCha8Rng::seed_from_u64((seed * 1000.0) as u64);
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

/// Test how search performance scales with dataset size
fn bench_search_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_scalability");
    group
        .plot_config(PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic));

    // Test different dataset sizes (logarithmic scale)
    let sizes = vec![100, 500, 1000, 5000, 10000, 25000, 50000];

    for &size in &sizes {
        let config = HnswRsConfig {
            dimension: 1024,
            max_connections: 24,
            ef_construction: 400,
            ef_search: 100,
            max_elements: size * 2,
            max_layers: 16,
            use_parallel: true,
        };

        let index = VectorIndexHnswRs::new(config).unwrap();

        // Build index
        let vectors: Vec<(String, Vec<f32>)> = (0..size)
            .map(|i| (format!("doc_{}", i), generate_embedding(1024, i as f32)))
            .collect();

        index.add_batch(vectors).unwrap();

        // Benchmark search
        let query = generate_embedding(1024, size as f32 / 2.0);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let results = index.search(&query, 10).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

/// Test how insert performance scales
fn bench_insert_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_scalability");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    let batch_sizes = vec![1, 10, 50, 100, 500, 1000];

    for &batch_size in &batch_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &batch_size| {
                b.iter_with_setup(
                    || {
                        let config = HnswRsConfig {
                            dimension: 1024,
                            max_connections: 24,
                            ef_construction: 400,
                            ef_search: 100,
                            max_elements: 100000,
                            max_layers: 16,
                            use_parallel: batch_size > 50,
                        };

                        let index = VectorIndexHnswRs::new(config).unwrap();

                        // Pre-populate with some data
                        let initial: Vec<(String, Vec<f32>)> = (0..10000)
                            .map(|i| (format!("init_{}", i), generate_embedding(1024, i as f32)))
                            .collect();
                        index.add_batch(initial).unwrap();

                        // Prepare batch to insert
                        let batch: Vec<(String, Vec<f32>)> = (0..batch_size)
                            .map(|i| {
                                (
                                    format!("new_{}", i),
                                    generate_embedding(1024, (10000 + i) as f32),
                                )
                            })
                            .collect();

                        (index, batch)
                    },
                    |(index, batch)| {
                        index.add_batch(batch).unwrap();
                    },
                );
            },
        );
    }

    group.finish();
}

/// Test memory usage scaling
fn bench_memory_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scalability");
    group.sample_size(10);

    let rt = Runtime::new().unwrap();
    let sizes = vec![1000, 5000, 10000, 25000];

    for &size in &sizes {
        let temp_dir = TempDir::new().unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter_with_setup(
                || {
                    let rt_inner = Runtime::new().unwrap();
                    let temp_path = temp_dir.path().join(format!("test_{}", size));

                    let store = rt_inner.block_on(async {
                        let store = VectorStore::new(&temp_path).await.unwrap();
                        store.init_layer(Layer::Interact).await.unwrap();
                        store
                    });

                    let records: Vec<Record> = (0..size)
                        .map(|i| Record {
                            id: uuid::Uuid::new_v4(),
                            text: format!("Document {}", i),
                            embedding: generate_embedding(1024, i as f32),
                            layer: Layer::Interact,
                            kind: "test".to_string(),
                            tags: vec![],
                            project: "test".to_string(),
                            session: "test".to_string(),
                            score: 0.5,
                            ts: chrono::Utc::now(),
                            last_access: chrono::Utc::now(),
                            access_count: 0,
                        })
                        .collect();

                    (store, records)
                },
                |(store, records)| async move {
                    let refs: Vec<&Record> = records.iter().collect();
                    store.insert_batch(&refs).await.unwrap();

                    // Perform some searches to test complete workflow
                    for i in 0..10 {
                        let query = generate_embedding(1024, i as f32);
                        let results = store.search(&query, Layer::Interact, 10).await.unwrap();
                        black_box(results);
                    }
                },
            );
        });
    }

    group.finish();
}

/// Test concurrent operations scalability
fn bench_concurrent_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_scalability");
    group.sample_size(10);

    let rt = Runtime::new().unwrap();
    let concurrency_levels = vec![1, 2, 4, 8, 16];

    for &concurrency in &concurrency_levels {
        let config = HnswRsConfig {
            dimension: 1024,
            max_connections: 24,
            ef_construction: 400,
            ef_search: 100,
            max_elements: 100000,
            max_layers: 16,
            use_parallel: true,
        };

        let index = VectorIndexHnswRs::new(config).unwrap();

        // Pre-populate
        let initial: Vec<(String, Vec<f32>)> = (0..10000)
            .map(|i| (format!("doc_{}", i), generate_embedding(1024, i as f32)))
            .collect();
        index.add_batch(initial).unwrap();

        group.bench_with_input(
            BenchmarkId::new("concurrent_search", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let futures: Vec<_> = (0..concurrency)
                        .map(|i| {
                            let index_ref = &index;
                            let query = generate_embedding(1024, i as f32 * 100.0);
                            async move { index_ref.search(&query, 10).unwrap() }
                        })
                        .collect();

                    let results = futures::future::join_all(futures).await;
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Test performance with different embedding dimensions
fn bench_dimension_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("dimension_scalability");

    let dimensions = vec![128, 256, 512, 768, 1024, 1536];

    for &dim in &dimensions {
        let config = HnswRsConfig {
            dimension: dim,
            max_connections: 24,
            ef_construction: 400,
            ef_search: 100,
            max_elements: 10000,
            max_layers: 16,
            use_parallel: true,
        };

        let index = VectorIndexHnswRs::new(config).unwrap();

        // Build index
        let vectors: Vec<(String, Vec<f32>)> = (0..5000)
            .map(|i| (format!("doc_{}", i), generate_embedding(dim, i as f32)))
            .collect();

        index.add_batch(vectors).unwrap();

        // Benchmark search
        let query = generate_embedding(dim, 2500.0);

        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, _| {
            b.iter(|| {
                let results = index.search(&query, 10).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_scalability,
    bench_insert_scalability,
    bench_memory_scalability,
    bench_concurrent_scalability,
    bench_dimension_scalability
);

criterion_main!(benches);
