//! Baseline профилирование HNSW производительности
//!
//! Этот benchmark специально создан для профилирования performance HNSW индекса
//! без зависимостей от других систем проекта

use criterion::{
    black_box, criterion_group, criterion_main, AxisScale, BenchmarkId, Criterion,
    PlotConfiguration, Throughput,
};
use std::sync::Arc;
use std::time::Instant;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Простая конфигурация HNSW для тестирования
#[derive(Clone, Debug)]
pub struct HnswConfig {
    pub dimension: usize,
    pub max_connections: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    pub max_elements: usize,
    pub max_layers: usize,
    pub use_parallel: bool,
}

impl HnswConfig {
    pub fn baseline() -> Self {
        Self {
            dimension: 1024,
            max_connections: 16,
            ef_construction: 200,
            ef_search: 50,
            max_elements: 100_000,
            max_layers: 16,
            use_parallel: false,
        }
    }

    pub fn optimized() -> Self {
        Self {
            dimension: 1024,
            max_connections: 32,
            ef_construction: 400,
            ef_search: 100,
            max_elements: 1_000_000,
            max_layers: 16,
            use_parallel: true,
        }
    }

    pub fn ultra_fast() -> Self {
        Self {
            dimension: 1024,
            max_connections: 64,
            ef_construction: 800,
            ef_search: 200,
            max_elements: 1_000_000,
            max_layers: 20,
            use_parallel: true,
        }
    }
}

/// Минимальная HNSW реализация для baseline тестирования
pub struct SimpleHnsw {
    config: HnswConfig,
    vectors: Vec<Vec<f32>>,
    ids: Vec<String>,
}

impl SimpleHnsw {
    pub fn new(config: HnswConfig) -> Self {
        Self {
            config,
            vectors: Vec::new(),
            ids: Vec::new(),
        }
    }

    pub fn add(&mut self, id: String, vector: Vec<f32>) {
        assert_eq!(vector.len(), self.config.dimension);
        self.vectors.push(vector);
        self.ids.push(id);
    }

    pub fn add_batch(&mut self, batch: Vec<(String, Vec<f32>)>) {
        for (id, vector) in batch {
            self.add(id, vector);
        }
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if self.vectors.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<_> = self
            .vectors
            .iter()
            .zip(self.ids.iter())
            .map(|(vec, id)| {
                let distance = cosine_distance_scalar(query, vec);
                (id.clone(), distance)
            })
            .collect();

        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        results.truncate(k);
        results
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }
}

/// Генерация случайных векторов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| fastrand::f32() * 2.0 - 1.0) // [-1, 1]
                .collect()
        })
        .collect()
}

/// Baseline scalar cosine distance
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

/// AVX2 оптимизированная версия cosine distance
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let len = a.len();

    // Обрабатываем по 8 элементов за раз
    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();

    let chunks = len / 8;
    for i in 0..chunks {
        let idx = i * 8;

        let va = _mm256_loadu_ps(a.as_ptr().add(idx));
        let vb = _mm256_loadu_ps(b.as_ptr().add(idx));

        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }

    // Horizontal sum
    let dot_sum = horizontal_sum_avx2(dot_product);
    let norm_a_sum = horizontal_sum_avx2(norm_a);
    let norm_b_sum = horizontal_sum_avx2(norm_b);

    // Обрабатываем остаток скалярно
    let remainder_start = chunks * 8;
    let mut remainder_dot = 0.0;
    let mut remainder_norm_a = 0.0;
    let mut remainder_norm_b = 0.0;

    for i in remainder_start..len {
        remainder_dot += a[i] * b[i];
        remainder_norm_a += a[i] * a[i];
        remainder_norm_b += b[i] * b[i];
    }

    let total_dot = dot_sum + remainder_dot;
    let total_norm_a = norm_a_sum + remainder_norm_a;
    let total_norm_b = norm_b_sum + remainder_norm_b;

    let similarity = total_dot / (total_norm_a.sqrt() * total_norm_b.sqrt());
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

/// Benchmark baseline производительности HNSW
fn bench_hnsw_baseline_performance(c: &mut Criterion) {
    // Создаем группы для разных конфигураций
    let mut group = c.benchmark_group("hnsw_baseline_performance");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));

    // Измеряем производительность для разных размеров данных
    let sizes = [1000, 5000, 10000, 50000, 100000];
    let dimensions = [384, 512, 768, 1024];

    for &size in &sizes {
        for &dim in &dimensions {
            if size > 50000 && dim > 768 {
                continue; // Пропускаем большие комбинации для экономии времени
            }

            let config = HnswConfig {
                dimension: dim,
                max_connections: 16,
                ef_construction: 200,
                ef_search: 50,
                max_elements: size,
                max_layers: 16,
                use_parallel: false,
            };

            // Подготавливаем данные
            let vectors = generate_random_vectors(size, dim);
            let batch: Vec<_> = vectors
                .into_iter()
                .enumerate()
                .map(|(i, v)| (format!("doc_{}", i), v))
                .collect();

            let mut index = SimpleHnsw::new(config);
            index.add_batch(batch);

            // Benchmark поиска
            group.throughput(Throughput::Elements(size as u64));
            group.bench_with_input(
                BenchmarkId::new(format("search_{}d", dim), size),
                &size,
                |b, _| {
                    let query = generate_random_vectors(1, dim)[0].clone();
                    b.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            let results = index.search(&query, 10);
                            black_box(results);
                        }
                        let duration = start.elapsed();

                        // Выводим предупреждение если поиск > 5ms
                        let avg_ms = duration.as_millis() as f64 / iters as f64;
                        if avg_ms > 5.0 {
                            eprintln!(
                                "⚠️  BASELINE: {}K vectors {}D search: {:.2}ms > 5ms target",
                                size / 1000,
                                dim,
                                avg_ms
                            );
                        } else {
                            eprintln!(
                                "✅ BASELINE: {}K vectors {}D search: {:.2}ms",
                                size / 1000,
                                dim,
                                avg_ms
                            );
                        }

                        duration
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark SIMD оптимизаций distance calculations
fn bench_simd_distance_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_distance_comparison");

    // Тестируем на разных размерах векторов
    let dimensions = [384, 512, 768, 1024, 1536, 2048];

    for &dim in &dimensions {
        let vector_a = generate_random_vectors(1, dim)[0].clone();
        let vector_b = generate_random_vectors(1, dim)[0].clone();

        // Scalar baseline
        group.bench_with_input(BenchmarkId::new("scalar", dim), &dim, |b, _| {
            b.iter(|| {
                black_box(cosine_distance_scalar(&vector_a, &vector_b));
            });
        });

        // AVX2 если доступен
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                group.bench_with_input(BenchmarkId::new("avx2", dim), &dim, |b, _| {
                    b.iter_custom(|iters| {
                        let start = Instant::now();
                        for _ in 0..iters {
                            let result = unsafe { cosine_distance_avx2(&vector_a, &vector_b) };
                            black_box(result);
                        }
                        let duration = start.elapsed();

                        // Измеряем speedup
                        let scalar_duration = {
                            let start = Instant::now();
                            for _ in 0..iters {
                                let result = cosine_distance_scalar(&vector_a, &vector_b);
                                black_box(result);
                            }
                            start.elapsed()
                        };

                        let speedup =
                            scalar_duration.as_nanos() as f64 / duration.as_nanos() as f64;
                        eprintln!("🚀 AVX2 speedup {}D: {:.2}x", dim, speedup);

                        duration
                    });
                });
            }
        }
    }

    group.finish();
}

/// Benchmark batch операций
fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    let batch_sizes = [1, 10, 50, 100, 500];
    let dimension = 1024;

    for &batch_size in &batch_sizes {
        let queries = generate_random_vectors(batch_size, dimension);
        let target = generate_random_vectors(1, dimension)[0].clone();

        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch_scalar", batch_size),
            &batch_size,
            |b, _| {
                b.iter(|| {
                    let results: Vec<f32> = queries
                        .iter()
                        .map(|q| cosine_distance_scalar(q, &target))
                        .collect();
                    black_box(results);
                });
            },
        );

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                group.bench_with_input(
                    BenchmarkId::new("batch_avx2", batch_size),
                    &batch_size,
                    |b, _| {
                        b.iter(|| {
                            let results: Vec<f32> = queries
                                .iter()
                                .map(|q| unsafe { cosine_distance_avx2(q, &target) })
                                .collect();
                            black_box(results);
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// Stress test для выявления scalability проблем
fn bench_scalability_stress(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability_stress");
    group.sample_size(10); // Меньше samples для stress tests
    group.measurement_time(std::time::Duration::from_secs(60));

    // Очень большие размеры данных
    let large_sizes = [100_000, 500_000, 1_000_000];

    for &size in &large_sizes {
        // Создаем большой индекс
        let config = HnswConfig::optimized();
        let mut index = SimpleHnsw::new(config);

        eprintln!("🔧 Подготовка {} векторов для stress test...", size);
        let vectors = generate_random_vectors(size, 1024);
        let batch: Vec<_> = vectors
            .into_iter()
            .enumerate()
            .map(|(i, v)| (format!("doc_{}", i), v))
            .collect();

        let start = Instant::now();
        index.add_batch(batch);
        let build_time = start.elapsed();
        eprintln!("✅ Построение индекса {}: {:?}", size, build_time);

        group.bench_with_input(BenchmarkId::new("large_search", size), &size, |b, _| {
            let query = generate_random_vectors(1, 1024)[0].clone();
            b.iter_custom(|iters| {
                let start = Instant::now();
                for _ in 0..iters {
                    let results = index.search(&query, 50);
                    black_box(results);
                }
                let duration = start.elapsed();

                let avg_ms = duration.as_millis() as f64 / iters as f64;
                if avg_ms > 5.0 {
                    eprintln!(
                        "⚠️  STRESS: {}M vectors search: {:.2}ms > 5ms target",
                        size / 1_000_000,
                        avg_ms
                    );
                }

                duration
            });
        });
    }

    group.finish();
}

/// CPU capability detection
fn detect_cpu_capabilities() {
    eprintln!("🔍 Детектирование CPU capabilities:");

    #[cfg(target_arch = "x86_64")]
    {
        let sse = is_x86_feature_detected!("sse");
        let sse2 = is_x86_feature_detected!("sse2");
        let avx = is_x86_feature_detected!("avx");
        let avx2 = is_x86_feature_detected!("avx2");
        let avx512f = is_x86_feature_detected!("avx512f");
        let fma = is_x86_feature_detected!("fma");

        eprintln!("  SSE:     {}", if sse { "✅" } else { "❌" });
        eprintln!("  SSE2:    {}", if sse2 { "✅" } else { "❌" });
        eprintln!("  AVX:     {}", if avx { "✅" } else { "❌" });
        eprintln!("  AVX2:    {}", if avx2 { "✅" } else { "❌" });
        eprintln!("  AVX-512: {}", if avx512f { "✅" } else { "❌" });
        eprintln!("  FMA:     {}", if fma { "✅" } else { "❌" });

        if avx512f {
            eprintln!("🚀 Максимальная SIMD производительность доступна (AVX-512)");
        } else if avx2 {
            eprintln!("⚡ Высокая SIMD производительность доступна (AVX2)");
        } else if avx {
            eprintln!("⚠️  Базовая SIMD производительность (AVX только)");
        } else {
            eprintln!("❌ SIMD недоступен - только scalar операции");
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        eprintln!("  Архитектура: не x86_64");
        eprintln!("  SIMD:        ❌ недоступен");
    }

    eprintln!();
}

/// Функция для инициализации перед benchmarks
fn setup_benchmarks() {
    detect_cpu_capabilities();
    eprintln!("🎯 ЦЕЛЬ: Поиск <5ms для всех размеров индекса");
    eprintln!("📊 Запуск baseline профилирования HNSW...\n");
}

criterion_group!(
    name = benches;
    config = {
        setup_benchmarks();
        Criterion::default()
            .measurement_time(std::time::Duration::from_secs(10))
            .sample_size(50)
            .warm_up_time(std::time::Duration::from_secs(3))
    };
    targets =
        bench_hnsw_baseline_performance,
        bench_simd_distance_comparison,
        bench_batch_operations,
        bench_scalability_stress
);

criterion_main!(benches);
