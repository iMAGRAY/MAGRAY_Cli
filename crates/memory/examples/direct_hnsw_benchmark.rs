//! Direct HNSW Benchmark - Тест производительности текущего HNSW кода

use hnsw_rs::hnsw::*;
use hnsw_rs::prelude::*;
use std::time::{Duration, Instant};
use rand::{thread_rng, Rng};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const VECTOR_DIM: usize = 1024;
const NUM_VECTORS: usize = 1000;
const NUM_QUERIES: usize = 50;

fn generate_vectors(count: usize, dim: usize) -> Vec<Vec<f32>> {
    let mut rng = thread_rng();
    (0..count)
        .map(|_| {
            (0..dim)
                .map(|_| rng.gen_range(-1.0..1.0))
                .collect()
        })
        .collect()
}

// SIMD cosine distance для сравнения
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 8, 0);
    
    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();
    
    let chunks = a.len() / 8;
    
    for i in 0..chunks {
        let idx = i * 8;
        
        let va = _mm256_loadu_ps(a.as_ptr().add(idx));
        let vb = _mm256_loadu_ps(b.as_ptr().add(idx));
        
        dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
        norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
        norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
    }
    
    // Horizontal sum
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

fn cosine_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    let similarity = dot / (norm_a.sqrt() * norm_b.sqrt());
    1.0 - similarity
}

fn main() -> anyhow::Result<()> {
    println!("🚀 Direct HNSW Performance Benchmark");
    println!("===================================");
    
    // Generate test data
    println!("📊 Generating test data...");
    let vectors = generate_vectors(NUM_VECTORS, VECTOR_DIM);
    let queries = generate_vectors(NUM_QUERIES, VECTOR_DIM);
    
    // Create HNSW index
    println!("🏗️ Building HNSW index...");
    let build_start = Instant::now();
    
    let mut hnsw: Hnsw<f32, DistCosine> = Hnsw::new(
        16,                  // M - connections per node
        NUM_VECTORS * 2,     // max_nb_connection
        16,                  // max layers
        200,                 // ef_construction
        DistCosine {},       // distance function
    );
    
    // Insert vectors
    for (i, vector) in vectors.iter().enumerate() {
        hnsw.insert_data(vector, i);
    }
    
    let build_time = build_start.elapsed();
    println!("✅ Index built in {:?}", build_time);
    
    // Benchmark single queries
    println!("⚡ Benchmarking single queries...");
    let mut query_times = Vec::new();
    
    for query in &queries {
        let start = Instant::now();
        let _results = hnsw.search(query, 10, 100); // k=10, ef_search=100
        let latency = start.elapsed();
        query_times.push(latency.as_nanos() as f64 / 1_000_000.0);
    }
    
    query_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_latency = query_times.iter().sum::<f64>() / query_times.len() as f64;
    let p95_latency = query_times[(query_times.len() as f64 * 0.95) as usize];
    let p99_latency = query_times[(query_times.len() as f64 * 0.99) as usize];
    
    // Benchmark SIMD vs Scalar distance calculations
    println!("⚙️ Benchmarking SIMD performance...");
    
    let test_vector_a = &queries[0];
    let test_vector_b = &vectors[0];
    
    // Scalar benchmark
    let scalar_iterations = 10000;
    let scalar_start = Instant::now();
    for _ in 0..scalar_iterations {
        let _distance = cosine_distance_scalar(test_vector_a, test_vector_b);
    }
    let scalar_time = scalar_start.elapsed().as_nanos() as f64;
    
    // SIMD benchmark
    let simd_start = Instant::now();
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            for _ in 0..scalar_iterations {
                unsafe {
                    let _distance = cosine_distance_avx2(test_vector_a, test_vector_b);
                }
            }
        } else {
            for _ in 0..scalar_iterations {
                let _distance = cosine_distance_scalar(test_vector_a, test_vector_b);
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        for _ in 0..scalar_iterations {
            let _distance = cosine_distance_scalar(test_vector_a, test_vector_b);
        }
    }
    let simd_time = simd_start.elapsed().as_nanos() as f64;
    
    let simd_speedup = if simd_time > 0.0 { scalar_time / simd_time } else { 1.0 };
    
    // Print results
    println!("\n🔥 BASELINE PERFORMANCE RESULTS 🔥");
    println!("=================================");
    
    println!("📊 Index Building:");
    println!("  Build Time:      {:?}", build_time);
    println!("  Vectors:         {}", NUM_VECTORS);
    println!("  Dimension:       {}", VECTOR_DIM);
    
    println!("⚡ Query Latency (Target: <5ms):");
    println!("  Average:         {:.3}ms", avg_latency);
    println!("  P95:             {:.3}ms", p95_latency);
    println!("  P99:             {:.3}ms", p99_latency);
    
    if avg_latency <= 5.0 {
        println!("  Status:          ✅ SUB-5MS TARGET ACHIEVED");
    } else {
        println!("  Status:          ❌ EXCEEDS 5ms by {:.1}ms", avg_latency - 5.0);
    }
    
    println!("⚙️ SIMD Distance Calculations:");
    println!("  Scalar Time:     {:.1}ns per operation", scalar_time / scalar_iterations as f64);
    println!("  SIMD Time:       {:.1}ns per operation", simd_time / scalar_iterations as f64);
    println!("  Speedup:         {:.2}x", simd_speedup);
    
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            println!("  AVX2 Status:     ✅ AVAILABLE");
        } else {
            println!("  AVX2 Status:     ❌ NOT AVAILABLE");
        }
        
        if std::arch::is_x86_feature_detected!("avx512f") {
            println!("  AVX-512 Status:  ✅ AVAILABLE");
        } else {
            println!("  AVX-512 Status:  ❌ NOT AVAILABLE");
        }
    }
    
    if simd_speedup >= 2.0 {
        println!("  Status:          ✅ EXCELLENT SIMD PERFORMANCE");
    } else if simd_speedup >= 1.5 {
        println!("  Status:          ⚠️ GOOD SIMD PERFORMANCE");
    } else {
        println!("  Status:          ❌ POOR SIMD - NEEDS OPTIMIZATION");
    }
    
    // Estimate QPS potential
    let estimated_qps = 1000.0 / avg_latency;
    println!("🚀 Throughput Estimates:");
    println!("  Single Thread:   {:.0} QPS", estimated_qps);
    println!("  Multi-Thread:    {:.0} QPS (8 threads)", estimated_qps * 8.0);
    
    if estimated_qps >= 200.0 {
        println!("  Status:          ✅ HIGH THROUGHPUT");
    } else {
        println!("  Status:          ⚠️ NEEDS BATCH OPTIMIZATION");
    }
    
    // Optimization recommendations
    println!("\n💡 OPTIMIZATION ROADMAP:");
    
    if avg_latency > 5.0 {
        println!("  🔧 Priority 1: Query Latency Optimization");
        println!("     - Reduce ef_search parameter for speed");
        println!("     - Implement prefetching in distance calculations");
        println!("     - Add hot vector caching");
    }
    
    if simd_speedup < 3.0 {
        println!("  🔧 Priority 2: SIMD Optimization");
        println!("     - Implement optimized horizontal sum");
        println!("     - Add memory alignment for vectors");
        println!("     - Consider AVX-512 implementation");
    }
    
    if estimated_qps < 1000.0 {
        println!("  🔧 Priority 3: Batch Processing");
        println!("     - Implement batch distance calculations");
        println!("     - Add parallel search capabilities");
        println!("     - Consider GPU acceleration");
    }
    
    println!("\n🎯 BASELINE ESTABLISHED - Ready for ultra-optimization!");
    
    Ok(())
}