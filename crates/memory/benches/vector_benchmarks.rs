#![cfg(all(not(feature = "minimal"), feature = "hnsw-index", feature = "persistence"))]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memory::{HnswRsConfig, Layer, Record, VectorIndexHnswRs, VectorStore};
use std::time::Instant;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Генерация случайных векторов для тестов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| {
                    #[cfg(feature = "rand")] {
                        return rand::random::<f32>() * 2.0 - 1.0;
                    }
                    #[allow(unreachable_code)]
                    {
                        use std::cell::RefCell;
                        thread_local! { static SEED: RefCell<u64> = RefCell::new(0x9E3779B97F4A7C15); }
                        SEED.with(|s| {
                            let mut x = *s.borrow();
                            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                            *s.borrow_mut() = x;
                            let f = ((x >> 40) as u32) as f32 / (u32::MAX as f32);
                            f * 2.0 - 1.0
                        })
                    }
                })
                .collect()
        })
        .collect()
}

fn create_test_records(vectors: Vec<Vec<f32>>, layer: Layer) -> Vec<Record> {
    vectors
        .into_iter()
        .map(|embedding| Record {
            id: Uuid::new_v4(),
            text: "test document".to_string(),
            embedding,
            layer,
            kind: "benchmark".to_string(),
            tags: vec!["test".to_string(), "benchmark".to_string()],
            project: "benchmark".to_string(),
            session: "benchmark_session".to_string(),
            score: 0.0,
            ts: chrono::Utc::now(),
            access_count: 0,
            last_access: chrono::Utc::now(),
        })
        .collect()
}

fn bench_hnsw_insert(c: &mut Criterion) {
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };

    let mut group = c.benchmark_group("hnsw_insert");

    for size in [100, 1000, 5000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter_with_setup(
                || generate_random_vectors(size, config.dimension),
                |vectors| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let store = VectorStore::with_config("/tmp", config.clone()).await.unwrap();
                        let recs = create_test_records(vectors, Layer::Interact);
                        let refs: Vec<&Record> = recs.iter().collect();
                        store.insert_batch(&refs).await.unwrap();
                    });
                },
            );
        });
    }

    group.finish();
}

fn bench_hnsw_search(c: &mut Criterion) {
    let config = HnswRsConfig::default();
    let mut group = c.benchmark_group("hnsw_search");

    for &size in &[1000, 5000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("search", size), &size, |b, &size| {
            b.iter_custom(|iters| {
                let start = Instant::now();
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let store = VectorStore::with_config("/tmp", config.clone()).await.unwrap();
                    let vectors = generate_random_vectors(size, config.dimension);
                    let records = create_test_records(vectors, Layer::Interact);
                    let refs: Vec<&Record> = records.iter().collect();
                    store.insert_batch(&refs).await.unwrap();
                    let query = vec![0.0f32; config.dimension];
                    black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
                });
                start.elapsed()
            });
        });
    }

    group.finish();
}

fn bench_hybrid_end_to_end(c: &mut Criterion) {
    let config = HnswRsConfig::default();
    let rt = Runtime::new().unwrap();

    c.bench_function("hybrid_end_to_end", |b| {
        b.to_async(&rt).iter(|| async {
            let store = VectorStore::with_config("/tmp", config.clone()).await.unwrap();
            let vectors = generate_random_vectors(1000, config.dimension);
            let records = create_test_records(vectors, Layer::Interact);
            let refs: Vec<&Record> = records.iter().collect();
            store.insert_batch(&refs).await.unwrap();
            let query = vec![0.0f32; config.dimension];
            black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
        })
    });
}

criterion_group!(benches, bench_hnsw_insert, bench_hnsw_search, bench_hybrid_end_to_end);
criterion_main!(benches);
