//! Ultra SIMD Performance Benchmark - Sub-1ms optimization validation
//!
//! Тестирует экстремальные SIMD оптимизации для достижения microsecond-level performance

use memory::{
    simd_ultra_optimized::{
        cosine_distance_ultra_optimized, AlignedVector, batch_cosine_distance_ultra,
        test_ultra_optimized_performance, benchmark_horizontal_sum_variants
    },
    simd_optimized::cosine_distance_auto
};
use std::time::Instant;
use rand::{thread_rng, Rng};

const VECTOR_DIM: usize = 1024;
const BENCHMARK_ITERATIONS: usize = 100000;
const BATCH_SIZE: usize = 100;

fn generate_aligned_vector(dim: usize) -> AlignedVector {
    let mut rng = thread_rng();
    let data: Vec<f32> = (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect();
    AlignedVector::new(data)
}

fn generate_regular_vector(dim: usize) -> Vec<f32> {
    let mut rng = thread_rng();
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn main() -> anyhow::Result<()> {
    println!("🚀 Ultra SIMD Performance Benchmark");
    println!("====================================");
    
    // Run comprehensive ultra-optimized tests first
    println!("📊 Running comprehensive ultra-optimized tests...\n");
    test_ultra_optimized_performance()?;
    
    println!("\n{}", "=".repeat(50));
    println!("📊 COMPARATIVE PERFORMANCE ANALYSIS");
    println!("{}", "=".repeat(50));
    
    // Generate test vectors
    println!("🔧 Generating test vectors...");
    let regular_a = generate_regular_vector(VECTOR_DIM);
    let regular_b = generate_regular_vector(VECTOR_DIM);
    let aligned_a = generate_aligned_vector(VECTOR_DIM);
    let aligned_b = generate_aligned_vector(VECTOR_DIM);
    
    // Single vector comparison
    println!("\n⚡ Single Vector Distance Comparison:");
    
    // Standard SIMD (from simd_optimized)
    let start = Instant::now();
    for _ in 0..BENCHMARK_ITERATIONS {
        let _distance = cosine_distance_auto(&regular_a, &regular_b);
    }
    let standard_simd_time = start.elapsed().as_nanos() as f64 / BENCHMARK_ITERATIONS as f64;
    
    // Ultra-optimized SIMD
    #[cfg(target_arch = "x86_64")]
    let ultra_simd_time = if std::arch::is_x86_feature_detected!("avx2") {
        let start = Instant::now();
        for _ in 0..BENCHMARK_ITERATIONS {
            unsafe {
                let _distance = cosine_distance_ultra_optimized(
                    aligned_a.as_aligned_slice(),
                    aligned_b.as_aligned_slice()
                );
            }
        }
        start.elapsed().as_nanos() as f64 / BENCHMARK_ITERATIONS as f64
    } else {
        standard_simd_time
    };
    
    #[cfg(not(target_arch = "x86_64"))]
    let ultra_simd_time = standard_simd_time;
    
    println!("  Standard SIMD:      {:.2}ns per operation", standard_simd_time);
    println!("  Ultra SIMD:         {:.2}ns per operation", ultra_simd_time);
    
    if ultra_simd_time < standard_simd_time {
        let speedup = standard_simd_time / ultra_simd_time;
        println!("  🚀 Ultra Speedup:   {:.1}x faster", speedup);
        
        // Calculate operations per second
        let ops_per_second = 1_000_000_000.0 / ultra_simd_time;
        println!("  🔥 Throughput:      {:.0} ops/sec", ops_per_second);
        
        if speedup >= 2.0 {
            println!("  Status:             🏆 EXCELLENT ULTRA-OPTIMIZATION");
        } else if speedup >= 1.5 {
            println!("  Status:             ✅ GOOD ULTRA-OPTIMIZATION");  
        } else {
            println!("  Status:             ⚠️ MODEST IMPROVEMENT");
        }
    } else {
        println!("  Status:             ❌ NO IMPROVEMENT");
    }
    
    // Batch processing comparison
    println!("\n🚀 Batch Processing Comparison:");
    
    // Generate batch test data
    let batch_regular: Vec<Vec<f32>> = (0..BATCH_SIZE)
        .map(|_| generate_regular_vector(VECTOR_DIM))
        .collect();
    
    let batch_aligned: Vec<AlignedVector> = (0..BATCH_SIZE)
        .map(|_| generate_aligned_vector(VECTOR_DIM))
        .collect();
    
    let target_regular = generate_regular_vector(VECTOR_DIM);
    let target_aligned = generate_aligned_vector(VECTOR_DIM);
    
    let batch_iterations = BENCHMARK_ITERATIONS / 100; // Fewer iterations for batch
    
    // Standard batch processing
    let start = Instant::now();
    for _ in 0..batch_iterations {
        let _results: Vec<f32> = batch_regular.iter()
            .map(|query| cosine_distance_auto(query, &target_regular))
            .collect();
    }
    let standard_batch_time = start.elapsed().as_secs_f64() * 1000.0 / batch_iterations as f64;
    
    // Ultra-optimized batch processing
    let start = Instant::now();
    for _ in 0..batch_iterations {
        let _results = batch_cosine_distance_ultra(&batch_aligned, &target_aligned);
    }
    let ultra_batch_time = start.elapsed().as_secs_f64() * 1000.0 / batch_iterations as f64;
    
    println!("  Standard Batch:     {:.3}ms per batch ({} vectors)", standard_batch_time, BATCH_SIZE);
    println!("  Ultra Batch:        {:.3}ms per batch ({} vectors)", ultra_batch_time, BATCH_SIZE);
    
    if ultra_batch_time < standard_batch_time {
        let batch_speedup = standard_batch_time / ultra_batch_time;
        println!("  🚀 Batch Speedup:   {:.1}x faster", batch_speedup);
        
        // Calculate vectors per second
        let vectors_per_second = BATCH_SIZE as f64 / (ultra_batch_time / 1000.0);
        println!("  🔥 Batch Throughput: {:.0} vectors/sec", vectors_per_second);
        
        if batch_speedup >= 2.0 {
            println!("  Status:             🏆 EXCELLENT BATCH OPTIMIZATION");
        } else if batch_speedup >= 1.5 {
            println!("  Status:             ✅ GOOD BATCH OPTIMIZATION");
        } else {
            println!("  Status:             ⚠️ MODEST BATCH IMPROVEMENT");
        }
    } else {
        println!("  Status:             ❌ NO BATCH IMPROVEMENT");
    }
    
    // Memory alignment analysis
    println!("\n📊 Memory Alignment Analysis:");
    println!("  Aligned Vector A:   {} bytes alignment", 
             aligned_a.as_aligned_slice().as_ptr() as usize % 64);
    println!("  Aligned Vector B:   {} bytes alignment", 
             aligned_b.as_aligned_slice().as_ptr() as usize % 64);
    println!("  AVX2 Aligned:       {}", aligned_a.is_avx2_aligned() && aligned_b.is_avx2_aligned());
    
    // Horizontal sum benchmark
    println!("\n⚙️ Horizontal Sum Optimization:");
    let (shuffle_ns, hadd_ns, branchless_ns) = benchmark_horizontal_sum_variants(BENCHMARK_ITERATIONS, VECTOR_DIM);
    
    if shuffle_ns > 0.0 {
        let best = [shuffle_ns, hadd_ns, branchless_ns]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        
        println!("  Shuffle Method:     {:.2}ns", shuffle_ns);
        println!("  Hadd Method:        {:.2}ns", hadd_ns);
        println!("  Branchless Method:  {:.2}ns", branchless_ns);
        
        if hadd_ns <= best {
            println!("  🏆 Best Method:     Hadd ({:.1}x speedup)", shuffle_ns / hadd_ns);
        } else if branchless_ns <= best {
            println!("  🏆 Best Method:     Branchless ({:.1}x speedup)", shuffle_ns / branchless_ns);
        } else {
            println!("  🏆 Best Method:     Traditional Shuffle");
        }
    }
    
    // Performance targets assessment
    println!("\n🎯 PERFORMANCE TARGETS ASSESSMENT:");
    println!("{}", "=".repeat(50));
    
    let single_latency_us = ultra_simd_time / 1000.0;
    let target_sub_1ms = single_latency_us < 1000.0; // < 1ms = 1000μs
    
    println!("  Single Operation:   {:.2}μs (target: <1000μs)", single_latency_us);
    if target_sub_1ms {
        println!("  Single Target:      ✅ SUB-1MS ACHIEVED");
    } else {
        println!("  Single Target:      ❌ EXCEEDS 1ms by {:.2}μs", single_latency_us - 1000.0);
    }
    
    let batch_latency_us = ultra_batch_time * 1000.0;
    let batch_per_vector_us = batch_latency_us / BATCH_SIZE as f64;
    let target_batch_efficient = batch_per_vector_us < 100.0; // <100μs per vector in batch
    
    println!("  Batch Per Vector:   {:.2}μs (target: <100μs)", batch_per_vector_us);
    if target_batch_efficient {
        println!("  Batch Target:       ✅ EFFICIENT BATCHING ACHIEVED");
    } else {
        println!("  Batch Target:       ❌ INEFFICIENT BATCHING");
    }
    
    // Overall assessment
    println!("\n📈 OVERALL ULTRA-OPTIMIZATION ASSESSMENT:");
    let targets_achieved = target_sub_1ms as i32 + target_batch_efficient as i32;
    
    match targets_achieved {
        2 => println!("  Status:             🏆 ALL ULTRA-TARGETS ACHIEVED - MICROSECOND-LEVEL PERFORMANCE"),
        1 => println!("  Status:             ✅ PARTIAL ULTRA-OPTIMIZATION SUCCESS"),
        _ => println!("  Status:             ⚠️ ULTRA-OPTIMIZATION NEEDS MORE WORK"),
    }
    
    if ultra_simd_time < standard_simd_time {
        println!("  Recommendation:     🚀 Deploy ultra-optimized version in production");
    } else {
        println!("  Recommendation:     🔧 Continue optimization or use standard SIMD");
    }
    
    println!("\n🎯 Ultra SIMD Benchmark Complete!");
    
    Ok(())
}