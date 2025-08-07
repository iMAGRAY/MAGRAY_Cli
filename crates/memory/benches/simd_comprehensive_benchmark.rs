//! Comprehensive SIMD Performance Benchmark
//!
//! Валидация microsecond-level performance оптимизаций:
//! - Ultra-optimized SIMD векторные операции
//! - HNSW index поиск производительность  
//! - AI embeddings SIMD acceleration
//! - Batch processing throughput
//! - Memory-mapped operations
//!
//! Цель: 10x+ performance improvement validation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use memory::{
    simd_ultra_optimized::{
        batch_cosine_distance_auto, batch_cosine_distance_ultra, benchmark_horizontal_sum_variants,
        cosine_distance_auto_ultra, cosine_distance_scalar, cosine_distance_scalar_optimized,
        cosine_distance_ultra_optimized, test_ultra_optimized_performance,
    },
    AlignedVector, HnswConfig, VectorIndex,
};
use rayon::prelude::*;
use std::time::{Duration, Instant};

/// Benchmark data generator для consistent testing
struct BenchmarkDataGenerator {
    vector_sizes: Vec<usize>,
    batch_sizes: Vec<usize>,
}

impl BenchmarkDataGenerator {
    fn new() -> Self {
        Self {
            vector_sizes: vec![256, 512, 768, 1024, 1536, 2048, 4096],
            batch_sizes: vec![1, 10, 50, 100, 500, 1000, 5000],
        }
    }

    /// Generate test vectors с различными размерностями
    fn generate_test_vectors(&self, dim: usize, count: usize) -> Vec<Vec<f32>> {
        (0..count)
            .map(|i| {
                (0..dim)
                    .map(|j| ((i * dim + j) as f32).sin() * 0.5 + 0.5)
                    .collect()
            })
            .collect()
    }

    /// Generate aligned vectors для SIMD optimization
    fn generate_aligned_vectors(&self, dim: usize, count: usize) -> Vec<AlignedVector> {
        self.generate_test_vectors(dim, count)
            .into_iter()
            .map(AlignedVector::new)
            .collect()
    }
}

/// Ultra-optimized cosine distance benchmarks
fn bench_cosine_distance_variants(c: &mut Criterion) {
    let gen = BenchmarkDataGenerator::new();
    let mut group = c.benchmark_group("cosine_distance_variants");

    for &dim in &gen.vector_sizes {
        let vectors = gen.generate_test_vectors(dim, 2);
        let a = &vectors[0];
        let b = &vectors[1];

        // Scalar baseline
        group.bench_with_input(
            BenchmarkId::new("scalar_optimized", dim),
            &dim,
            |bench, _| {
                bench.iter(|| {
                    black_box(cosine_distance_scalar_optimized(black_box(a), black_box(b)))
                })
            },
        );

        // Auto selection (best available)
        group.bench_with_input(BenchmarkId::new("auto_ultra", dim), &dim, |bench, _| {
            bench.iter(|| black_box(cosine_distance_auto_ultra(black_box(a), black_box(b))))
        });

        // Ultra-optimized AVX2 (if available)
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            group.bench_with_input(BenchmarkId::new("avx2_ultra", dim), &dim, |bench, _| {
                bench.iter(|| unsafe {
                    black_box(cosine_distance_ultra_optimized(black_box(a), black_box(b)))
                })
            });
        }

        // AVX-512 ultra (if available)
        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx512f") && dim >= 64 && dim % 16 == 0 {
            group.bench_with_input(BenchmarkId::new("avx512_ultra", dim), &dim, |bench, _| {
                bench.iter(|| {
                    black_box(unsafe {
                        cosine_distance_ultra_optimized(black_box(a), black_box(b))
                    })
                })
            });
        }
    }

    group.finish();
}

/// Batch cosine distance throughput benchmarks
fn bench_batch_cosine_distance(c: &mut Criterion) {
    let gen = BenchmarkDataGenerator::new();
    let mut group = c.benchmark_group("batch_cosine_distance");

    // Test различных размеров batch'ей
    for &batch_size in &gen.batch_sizes {
        for &dim in &[1024] {
            // Focus на 1024D для embedding use case
            let queries = gen.generate_test_vectors(dim, batch_size);
            let target = &gen.generate_test_vectors(dim, 1)[0];

            group.throughput(Throughput::Elements(batch_size as u64));

            // Regular batch processing
            group.bench_with_input(
                BenchmarkId::new(format!("regular_batch_{}", batch_size), dim),
                &batch_size,
                |bench, _| {
                    bench.iter(|| {
                        black_box(batch_cosine_distance_auto(
                            black_box(&queries),
                            black_box(target),
                        ))
                    })
                },
            );

            // Ultra-optimized batch с aligned vectors
            let aligned_queries = gen.generate_aligned_vectors(dim, batch_size);
            let aligned_target = AlignedVector::new(target.clone());

            group.bench_with_input(
                BenchmarkId::new(format!("ultra_aligned_batch_{}", batch_size), dim),
                &batch_size,
                |bench, _| {
                    bench.iter(|| {
                        black_box(batch_cosine_distance_ultra(
                            black_box(&aligned_queries),
                            black_box(&aligned_target),
                        ))
                    })
                },
            );
        }
    }

    group.finish();
}

/// HNSW index search performance benchmarks
fn bench_hnsw_search_performance(c: &mut Criterion) {
    let gen = BenchmarkDataGenerator::new();
    let mut group = c.benchmark_group("hnsw_search_performance");

    // Create HNSW index с оптимизированной конфигурацией
    let config = HnswConfig {
        max_connections: 16,
        ef_construction: 200,
        ef_search: 64,
        max_elements: 10000,
        dimension: 1024,
        use_parallel: true,
        max_layers: 16,
    };

    let index = VectorIndex::new(config).expect("Failed to create HNSW index");

    // Add test vectors to index
    let num_vectors = 5000;
    let test_vectors = gen.generate_test_vectors(1024, num_vectors);

    println!("🏗️ Building HNSW index with {} vectors...", num_vectors);
    let build_start = Instant::now();

    for (i, vector) in test_vectors.iter().enumerate() {
        index
            .add(format!("vec_{}", i), vector.clone())
            .expect("Failed to add vector to index");
    }

    println!("✅ HNSW index built in {:?}", build_start.elapsed());

    // Generate query vectors
    let query_vectors = gen.generate_test_vectors(1024, 100);

    // Benchmark different k values
    let k_values = vec![1, 5, 10, 50, 100];

    for &k in &k_values {
        group.bench_with_input(BenchmarkId::new("single_search", k), &k, |bench, &k| {
            bench.iter(|| {
                let query = &query_vectors[0];
                black_box(index.search(black_box(query), k))
            })
        });

        // Batch search performance
        let batch_queries = &query_vectors[..10.min(query_vectors.len())];
        group.bench_with_input(BenchmarkId::new("batch_search", k), &k, |bench, &k| {
            bench.iter(|| black_box(index.parallel_search(black_box(batch_queries), k)))
        });
    }

    group.finish();
}

/// Memory alignment и cache performance benchmarks
fn bench_memory_alignment_performance(c: &mut Criterion) {
    let gen = BenchmarkDataGenerator::new();
    let mut group = c.benchmark_group("memory_alignment");

    for &dim in &[1024, 2048] {
        let regular_vectors = gen.generate_test_vectors(dim, 1000);
        let aligned_vectors = gen.generate_aligned_vectors(dim, 1000);

        // Benchmark memory access patterns
        group.bench_with_input(
            BenchmarkId::new("regular_memory_access", dim),
            &dim,
            |bench, _| {
                bench.iter(|| {
                    let mut sum = 0.0f32;
                    for vector in &regular_vectors {
                        for &val in vector {
                            sum += val;
                        }
                    }
                    black_box(sum)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("aligned_memory_access", dim),
            &dim,
            |bench, _| {
                bench.iter(|| {
                    let mut sum = 0.0f32;
                    for aligned_vec in &aligned_vectors {
                        for &val in aligned_vec.as_aligned_slice() {
                            sum += val;
                        }
                    }
                    black_box(sum)
                })
            },
        );

        // SIMD vs scalar performance comparison
        if dim % 8 == 0 {
            let a = &regular_vectors[0];
            let b = &regular_vectors[1];
            let aligned_a = &aligned_vectors[0];
            let aligned_b = &aligned_vectors[1];

            group.bench_with_input(
                BenchmarkId::new("scalar_distance", dim),
                &dim,
                |bench, _| {
                    bench.iter(|| {
                        black_box(cosine_distance_scalar_optimized(black_box(a), black_box(b)))
                    })
                },
            );

            group.bench_with_input(BenchmarkId::new("simd_distance", dim), &dim, |bench, _| {
                bench.iter(|| {
                    black_box(cosine_distance_auto_ultra(
                        black_box(aligned_a.as_aligned_slice()),
                        black_box(aligned_b.as_aligned_slice()),
                    ))
                })
            });
        }
    }

    group.finish();
}

/// Horizontal sum optimization benchmarks
fn bench_horizontal_sum_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("horizontal_sum_variants");

    let test_iterations = 10000;
    let vector_size = 1024;

    // Generate test data
    let data: Vec<f32> = (0..vector_size)
        .map(|i| i as f32 / vector_size as f32)
        .collect();
    let aligned_data = AlignedVector::new(data);

    group.bench_function("horizontal_sum_benchmark", |bench| {
        bench.iter(|| {
            black_box(benchmark_horizontal_sum_variants(
                black_box(test_iterations),
                black_box(vector_size),
            ))
        })
    });

    group.finish();
}

/// Comprehensive performance validation
fn bench_comprehensive_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("comprehensive_validation");

    group.bench_function("ultra_optimized_performance_test", |bench| {
        bench.iter(|| black_box(test_ultra_optimized_performance().unwrap()))
    });

    group.finish();
}

/// Parallel processing scalability benchmarks
fn bench_parallel_scalability(c: &mut Criterion) {
    let gen = BenchmarkDataGenerator::new();
    let mut group = c.benchmark_group("parallel_scalability");

    // Test scaling с различным количеством threads
    let thread_counts = vec![1, 2, 4, 8, 16];
    let vector_count = 10000;
    let dim = 1024;

    let vectors = gen.generate_test_vectors(dim, vector_count);
    let target = &vectors[0];

    for &threads in &thread_counts {
        group.bench_with_input(
            BenchmarkId::new("parallel_distance", threads),
            &threads,
            |bench, &threads| {
                // Set thread pool size
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap();

                bench.iter(|| {
                    pool.install(|| {
                        let results: Vec<f32> = vectors
                            .par_iter()
                            .map(|query| cosine_distance_auto_ultra(query, target))
                            .collect();
                        black_box(results)
                    })
                })
            },
        );
    }

    group.finish();
}

// Configure criterion
criterion_group! {
    name = simd_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(30))
        .warm_up_time(Duration::from_secs(5));
    targets =
        bench_cosine_distance_variants,
        bench_batch_cosine_distance,
        bench_hnsw_search_performance,
        bench_memory_alignment_performance,
        bench_horizontal_sum_variants,
        bench_comprehensive_performance_validation,
        bench_parallel_scalability
}

criterion_main!(simd_benchmarks);
