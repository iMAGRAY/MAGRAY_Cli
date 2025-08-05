use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use memory::{
    VectorStore,
    VectorIndexHnswRs, HnswRsConfig,
    Layer, Record,
};
use uuid::Uuid;
use tokio::runtime::Runtime;
use std::time::Instant;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

// @component: {"k":"T","id":"perf_benchmarks","t":"Performance benchmarks для memory system","m":{"cur":0,"tgt":100,"u":"%"},"f":["benchmarks","performance"]}

/// Генерация случайных векторов для тестов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| fastrand::f32() * 2.0 - 1.0) // [-1, 1]
                .collect()
        })
        .collect()
}

/// Создание тестовых записей
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

/// Benchmark: HNSW индекс - добавление векторов
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
                || {
                    let index = VectorIndexHnswRs::new(config.clone()).unwrap();
                    let vectors = generate_random_vectors(size, 768);
                    (index, vectors)
                },
                |(index, vectors)| {
                    for (i, vector) in vectors.into_iter().enumerate() {
                        black_box(index.add(format!("doc_{}", i), vector).unwrap());
                    }
                }
            );
        });
        
        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, &size| {
            b.iter_with_setup(
                || {
                    let index = VectorIndexHnswRs::new(config.clone()).unwrap();
                    let vectors = generate_random_vectors(size, 768);
                    let batch: Vec<_> = vectors.into_iter()
                        .enumerate()
                        .map(|(i, v)| (format!("doc_{}", i), v))
                        .collect();
                    (index, batch)
                },
                |(index, batch)| {
                    black_box(index.add_batch(batch).unwrap());
                }
            );
        });
    }
    
    group.finish();
}

/// Benchmark: HNSW индекс - поиск векторов
fn bench_hnsw_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };
    
    // Подготавливаем индекс с данными разных размеров
    let sizes = [1000, 10000, 50000];
    let mut prepared_indices = Vec::new();
    
    for &size in &sizes {
        let index = VectorIndexHnswRs::new(config.clone()).unwrap();
        let vectors = generate_random_vectors(size, 768);
        let batch: Vec<_> = vectors.into_iter()
            .enumerate()
            .map(|(i, v)| (format!("doc_{}", i), v))
            .collect();
        index.add_batch(batch).unwrap();
        prepared_indices.push((size, index));
    }
    
    let mut group = c.benchmark_group("hnsw_search");
    
    for (size, index) in prepared_indices {
        for k in [1, 10, 50, 100].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("{}k_vectors", size/1000), k), 
                k, 
                |b, &k| {
                    let query = generate_random_vectors(1, 768)[0].clone();
                    b.iter(|| {
                        black_box(index.search(&query, k).unwrap());
                    });
                }
            );
        }
    }
    
    group.finish();
}

/// Benchmark: VectorStore - комплексные операции
fn bench_vector_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("vector_store");
    
    for size in [100, 1000, 5000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        
        // Benchmark insert
        group.bench_with_input(BenchmarkId::new("insert", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                
                for record in records {
                    black_box(store.insert(&record).await.unwrap());
                }
            });
        });
        
        // Benchmark batch insert
        group.bench_with_input(BenchmarkId::new("batch_insert", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let temp_dir = tempfile::TempDir::new().unwrap();
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                
                let refs: Vec<&Record> = records.iter().collect();
                black_box(store.insert_batch(&refs).await.unwrap());
            });
        });
        
        // Benchmark search (with pre-populated data)
        group.bench_with_input(BenchmarkId::new("search", size), size, |b, &size| {
            // Pre-populate store once per benchmark group
            let temp_dir = tempfile::TempDir::new().unwrap();
            let store = rt.block_on(async {
                let store = VectorStore::new(temp_dir.path()).await.unwrap();
                let vectors = generate_random_vectors(size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                let refs: Vec<&Record> = records.iter().collect();
                store.insert_batch(&refs).await.unwrap();
                store
            });
            
            b.to_async(&rt).iter(|| async {
                let query = generate_random_vectors(1, 1024)[0].clone();
                black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
            });
        });
    }
    
    group.finish();
}

/// Benchmark: SIMD Distance Calculations Performance
fn bench_simd_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_distance");
    
    // Test vectors with Qwen3 dimension (1024)
    let vector_a = generate_random_vectors(1, 1024)[0].clone();
    let vector_b = generate_random_vectors(1, 1024)[0].clone();
    
    // Scalar implementation benchmark
    group.bench_function("cosine_distance_scalar", |b| {
        b.iter(|| {
            black_box(cosine_distance_scalar(&vector_a, &vector_b));
        });
    });
    
    // SIMD implementation benchmark (if available)
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            group.bench_function("cosine_distance_avx2", |b| {
                b.iter(|| {
                    unsafe {
                        black_box(cosine_distance_avx2(&vector_a, &vector_b));
                    }
                });
            });
        }
    }
    
    // Batch SIMD benchmark
    let queries = generate_random_vectors(100, 1024);
    group.throughput(Throughput::Elements(100));
    
    group.bench_function("batch_cosine_distance_simd", |b| {
        b.iter(|| {
            black_box(batch_cosine_distance_avx2(&queries, &vector_b));
        });
    });
    
    group.finish();
}

/// Benchmark: Sub-5ms Search Performance
fn bench_sub_5ms_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("sub_5ms_search");
    group.measurement_time(std::time::Duration::from_secs(60)); // Longer measurement for precision
    
    let configs = vec![
        ("ultra_fast_config", HnswRsConfig::ultra_fast()),
        ("cli_optimized_config", HnswRsConfig::cli_optimized()),
        ("high_speed_config", HnswRsConfig::high_speed()),
    ];
    
    for (name, config) in configs {
        // Prepare index with different sizes
        for &dataset_size in &[1000, 5000, 10000, 50000] {
            let index = VectorIndexHnswRs::new(config.clone()).unwrap();
            let vectors = generate_random_vectors(dataset_size, 1024);
            let batch: Vec<_> = vectors.into_iter()
                .enumerate()
                .map(|(i, v)| (format!("doc_{}", i), v))
                .collect();
            index.add_batch(batch).unwrap();
            
            group.bench_with_input(
                BenchmarkId::new(format!("{}/{}", name, dataset_size), dataset_size),
                &dataset_size,
                |b, _| {
                    let query = generate_random_vectors(1, 1024)[0].clone();
                    b.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            black_box(index.search(&query, 10).unwrap());
                        }
                        let duration = start.elapsed();
                        
                        // Check if under 5ms per search
                        let avg_ms = duration.as_millis() as f64 / iters as f64;
                        if avg_ms > 5.0 {
                            eprintln!("WARNING: {} search took {:.2}ms > 5ms target", name, avg_ms);
                        }
                        
                        duration
                    });
                }
            );
        }
    }
    
    group.finish();
}

/// Benchmark: Concurrent Search Operations
fn bench_concurrent_search(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_search");
    
    // Setup index
    let config = HnswRsConfig::cli_optimized();
    let index = std::sync::Arc::new(VectorIndexHnswRs::new(config).unwrap());
    let vectors = generate_random_vectors(10000, 1024);
    let batch: Vec<_> = vectors.into_iter()
        .enumerate()
        .map(|(i, v)| (format!("doc_{}", i), v))
        .collect();
    index.add_batch(batch).unwrap();
    
    for &thread_count in &[1, 2, 4, 8, 16, 32] {
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            &thread_count,
            |b, &threads| {
                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..threads).map(|_| {
                        let index_clone = index.clone();
                        tokio::spawn(async move {
                            let query = generate_random_vectors(1, 1024)[0].clone();
                            black_box(index_clone.search(&query, 10).unwrap());
                        })
                    }).collect();
                    
                    // Wait for all concurrent searches to complete
                    for task in tasks {
                        task.await.unwrap();
                    }
                });
            }
        );
    }
    
    group.finish();
}

/// Benchmark: Memory scaling под нагрузкой
fn bench_memory_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("memory_scaling_stress", |b| {
        b.to_async(&rt).iter(|| async {
            let temp_dir = tempfile::TempDir::new().unwrap();
            let store = VectorStore::new(temp_dir.path()).await.unwrap();
            
            // Симулируем быстрый рост данных
            for batch_size in [100, 500, 1000, 2000].iter() {
                let vectors = generate_random_vectors(*batch_size, 1024);
                let records = create_test_records(vectors, Layer::Interact);
                let refs: Vec<&Record> = records.iter().collect();
                
                black_box(store.insert_batch(&refs).await.unwrap());
                
                // Проверяем время search после каждого batch
                let query = generate_random_vectors(1, 1024)[0].clone();
                black_box(store.search(&query, Layer::Interact, 10).await.unwrap());
            }
        });
    });
}

// Helper functions for SIMD benchmarking
fn cosine_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    let similarity = dot_product / (norm_a.sqrt() * norm_b.sqrt());
    1.0 - similarity
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len() % 8, 0, "Vector length must be multiple of 8 for AVX2");
    
    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();
    
    let chunks = a.len() / 8;
    
    for i in 0..chunks {
        let idx = i * 8;
        
        let va = _mm256_loadu_ps(a.as_ptr().add(idx));
        let vb = _mm256_loadu_ps(b.as_ptr().add(idx));
        
        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }
    
    let dot_sum = horizontal_sum_avx2(dot_product);
    let norm_a_sum = horizontal_sum_avx2(norm_a);
    let norm_b_sum = horizontal_sum_avx2(norm_b);
    
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_avx2(v: __m256) -> f32 {
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);
    
    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);
    
    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
    let sum32 = _mm_add_ss(sum64, hi32);
    
    _mm_cvtss_f32(sum32)
}

#[cfg(target_arch = "x86_64")]
fn batch_cosine_distance_avx2(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    if is_x86_feature_detected!("avx2") {
        queries.iter().map(|query| {
            unsafe { cosine_distance_avx2(query, target) }
        }).collect()
    } else {
        queries.iter().map(|query| {
            cosine_distance_scalar(query, target)
        }).collect()
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn batch_cosine_distance_avx2(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    queries.iter().map(|query| {
        cosine_distance_scalar(query, target)
    }).collect()
}

/// Benchmark: Параллельный поиск (эксклюзивная фича hnsw_rs)
fn bench_parallel_search(c: &mut Criterion) {
    let config = HnswRsConfig {
        dimension: 1024,
        max_connections: 24,
        ef_construction: 400,
        ef_search: 100,
        max_elements: 100_000,
        max_layers: 16,
        use_parallel: true,
    };
    
    // Подготавливаем большой индекс
    let index = VectorIndexHnswRs::new(config).unwrap();
    let vectors = generate_random_vectors(10000, 768);
    let batch: Vec<_> = vectors.into_iter()
        .enumerate()
        .map(|(i, v)| (format!("doc_{}", i), v))
        .collect();
    index.add_batch(batch).unwrap();
    
    let mut group = c.benchmark_group("parallel_search");
    
    for query_count in [1, 4, 8, 16, 32].iter() {
        group.throughput(Throughput::Elements(*query_count as u64));
        
        group.bench_with_input(
            BenchmarkId::new("queries", query_count), 
            query_count, 
            |b, &query_count| {
                let queries = generate_random_vectors(query_count, 768);
                b.iter(|| {
                    black_box(index.parallel_search(&queries, 10).unwrap());
                });
            }
        );
    }
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(std::time::Duration::from_secs(30))
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = 
        bench_simd_distance_calculations,
        bench_sub_5ms_search,
        bench_concurrent_search,
        bench_hnsw_insert,
        bench_hnsw_search, 
        bench_vector_store,
        bench_memory_scaling,
        bench_parallel_search
);

criterion_main!(benches);