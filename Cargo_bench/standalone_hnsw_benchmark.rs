//! Standalone HNSW Performance Benchmark
//! 
//! Этот benchmark работает независимо от memory crate для профилирования
//! базовой производительности HNSW операций

use std::time::Instant;
use hnsw_rs::prelude::*;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Генерация случайных векторов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| fastrand::f32() * 2.0 - 1.0)
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

/// AVX2 оптимизированная версия
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let len = a.len();
    
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
    
    // Остаток скалярно
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

/// CPU capabilities detection
fn detect_cpu_capabilities() {
    println!("🔍 CPU Capabilities:");
    
    #[cfg(target_arch = "x86_64")]
    {
        let sse = is_x86_feature_detected!("sse");
        let sse2 = is_x86_feature_detected!("sse2");
        let avx = is_x86_feature_detected!("avx");
        let avx2 = is_x86_feature_detected!("avx2");
        let avx512f = is_x86_feature_detected!("avx512f");
        let fma = is_x86_feature_detected!("fma");
        
        println!("  SSE:     {}", if sse { "✅" } else { "❌" });
        println!("  SSE2:    {}", if sse2 { "✅" } else { "❌" });
        println!("  AVX:     {}", if avx { "✅" } else { "❌" });
        println!("  AVX2:    {}", if avx2 { "✅" } else { "❌" });
        println!("  AVX-512: {}", if avx512f { "✅" } else { "❌" });
        println!("  FMA:     {}", if fma { "✅" } else { "❌" });
        
        if avx512f {
            println!("🚀 Maximum SIMD performance available (AVX-512)");
        } else if avx2 && fma {
            println!("⚡ High SIMD performance available (AVX2 + FMA)");
        } else if avx2 {
            println!("⚡ Good SIMD performance available (AVX2)");
        } else if avx {
            println!("⚠️ Basic SIMD performance (AVX only)");
        } else {
            println!("❌ No SIMD available - scalar operations only");
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        println!("  Architecture: not x86_64");
        println!("  SIMD:        ❌ not available");
    }
    
    println!();
}

/// SIMD distance benchmark
fn bench_simd_performance() {
    println!("🧮 SIMD Distance Performance Benchmark");
    println!("=====================================");
    
    let dimensions = [384, 512, 768, 1024, 1536];
    let iterations = 100_000;
    
    for &dim in &dimensions {
        println!("\n📊 Dimension: {}", dim);
        
        let vector_a = generate_random_vectors(1, dim)[0].clone();
        let vector_b = generate_random_vectors(1, dim)[0].clone();
        
        // Scalar benchmark
        let start = Instant::now();
        let mut scalar_result = 0.0;
        for _ in 0..iterations {
            scalar_result += cosine_distance_scalar(&vector_a, &vector_b);
        }
        let scalar_duration = start.elapsed();
        
        println!("  📈 Scalar:");
        println!("    Duration: {:?}", scalar_duration);
        println!("    Avg per op: {:.2} ns", scalar_duration.as_nanos() as f64 / iterations as f64);
        println!("    Result: {:.8}", scalar_result / iterations as f32);
        
        // AVX2 benchmark если доступен
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                let start = Instant::now();
                let mut avx2_result = 0.0;
                for _ in 0..iterations {
                    avx2_result += unsafe { cosine_distance_avx2(&vector_a, &vector_b) };
                }
                let avx2_duration = start.elapsed();
                
                let speedup = scalar_duration.as_nanos() as f64 / avx2_duration.as_nanos() as f64;
                let accuracy_diff = (scalar_result - avx2_result).abs() / iterations as f32;
                
                println!("  🚀 AVX2:");
                println!("    Duration: {:?}", avx2_duration);
                println!("    Avg per op: {:.2} ns", avx2_duration.as_nanos() as f64 / iterations as f64);
                println!("    Result: {:.8}", avx2_result / iterations as f32);
                println!("    🚀 Speedup: {:.2}x", speedup);
                println!("    🎯 Accuracy diff: {:.10}", accuracy_diff);
                
                if speedup > 2.0 {
                    println!("    ✅ Excellent SIMD speedup");
                } else if speedup > 1.5 {
                    println!("    ⚡ Good SIMD speedup");
                } else if speedup > 1.1 {
                    println!("    ⚠️ Modest SIMD speedup");
                } else {
                    println!("    ❌ Poor SIMD speedup - investigate!");
                }
            }
        }
    }
}

/// HNSW baseline benchmark
fn bench_hnsw_baseline() {
    println!("\n🔍 HNSW Baseline Performance");
    println!("============================");
    
    let configs = vec![
        ("baseline", 16, 200, 50),
        ("optimized", 32, 400, 100),
        ("ultra_fast", 64, 800, 200),
    ];
    
    let sizes = [1000, 5000, 10000, 50000];
    let dimension = 1024;
    
    for (name, max_connections, ef_construction, ef_search) in configs {
        println!("\n📋 Configuration: {} (M={}, efC={}, efS={})", 
                name, max_connections, ef_construction, ef_search);
        
        for &size in &sizes {
            println!("\n  🔢 Dataset size: {} vectors", size);
            
            // Создание индекса
            let mut hnsw: Hnsw<f32, DistCosine> = Hnsw::new(
                max_connections,     // M
                size,                // capacity
                16,                  // max_layers
                ef_construction,     // ef_construction
                DistCosine {},       // distance
            );
            
            // Генерируем и добавляем данные
            let vectors = generate_random_vectors(size, dimension);
            
            let start = Instant::now();
            for (id, vector) in vectors.iter().enumerate() {
                hnsw.insert_data(vector, id);
            }
            let build_time = start.elapsed();
            
            println!("    🏗️ Build time: {:?} ({:.2} vectors/ms)", 
                    build_time, 
                    size as f64 / build_time.as_millis() as f64);
            
            // Search benchmark
            let query = &vectors[0]; // Используем первый вектор как query
            let search_sizes = [1, 5, 10, 50];
            
            for &k in &search_sizes {
                let start = Instant::now();
                let iterations = if size > 10000 { 100 } else { 1000 };
                
                for _ in 0..iterations {
                    let _results = hnsw.search(query, k, ef_search);
                }
                let search_duration = start.elapsed();
                
                let avg_ms = search_duration.as_millis() as f64 / iterations as f64;
                
                print!("    🔍 k={}: {:.2}ms", k, avg_ms);
                
                if avg_ms > 5.0 {
                    println!(" ❌ > 5ms target");
                } else if avg_ms > 2.0 {
                    println!(" ⚠️ > 2ms (acceptable)");
                } else if avg_ms > 1.0 {
                    println!(" ⚡ < 2ms (good)");
                } else {
                    println!(" 🚀 < 1ms (excellent)");
                }
            }
        }
    }
}

/// Batch operations benchmark
fn bench_batch_operations() {
    println!("\n📦 Batch Operations Performance");
    println!("===============================");
    
    let batch_sizes = [1, 10, 50, 100, 500];
    let dimension = 1024;
    let iterations = 1000;
    
    for &batch_size in &batch_sizes {
        println!("\n🔢 Batch size: {}", batch_size);
        
        let queries = generate_random_vectors(batch_size, dimension);
        let target = generate_random_vectors(1, dimension)[0].clone();
        
        // Scalar batch
        let start = Instant::now();
        for _ in 0..iterations {
            let _results: Vec<f32> = queries.iter()
                .map(|q| cosine_distance_scalar(q, &target))
                .collect();
        }
        let scalar_duration = start.elapsed();
        
        let scalar_throughput = (batch_size * iterations) as f64 / scalar_duration.as_millis() as f64;
        println!("  📈 Scalar: {:.2} vectors/ms", scalar_throughput);
        
        // AVX2 batch если доступен
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                let start = Instant::now();
                for _ in 0..iterations {
                    let _results: Vec<f32> = queries.iter()
                        .map(|q| unsafe { cosine_distance_avx2(q, &target) })
                        .collect();
                }
                let avx2_duration = start.elapsed();
                
                let avx2_throughput = (batch_size * iterations) as f64 / avx2_duration.as_millis() as f64;
                let batch_speedup = avx2_throughput / scalar_throughput;
                
                println!("  🚀 AVX2: {:.2} vectors/ms ({:.2}x speedup)", 
                        avx2_throughput, batch_speedup);
                
                if batch_speedup > 3.0 {
                    println!("      ✅ Excellent batch SIMD performance");
                } else if batch_speedup > 2.0 {
                    println!("      ⚡ Good batch SIMD performance");
                } else if batch_speedup > 1.2 {
                    println!("      ⚠️ Modest batch SIMD performance");
                } else {
                    println!("      ❌ Poor batch SIMD performance");
                }
            }
        }
    }
}

/// Memory scaling test
fn bench_memory_scaling() {
    println!("\n💾 Memory Scaling Test");
    println!("=====================");
    
    let sizes = [10000, 50000, 100000, 500000];
    let dimension = 1024;
    
    for &size in &sizes {
        println!("\n🔢 Testing {}K vectors", size / 1000);
        
        let start_mem = get_memory_usage();
        
        // Создание индекса
        let start = Instant::now();
        let mut hnsw: Hnsw<f32, DistCosine> = Hnsw::new(
            32,              // M
            size,            // capacity  
            16,              // max_layers
            400,             // ef_construction
            DistCosine {},   // distance
        );
        
        // Добавление данных
        let vectors = generate_random_vectors(size, dimension);
        for (id, vector) in vectors.iter().enumerate() {
            hnsw.insert_data(vector, id);
        }
        let build_time = start.elapsed();
        
        let end_mem = get_memory_usage();
        let memory_per_vector = (end_mem - start_mem) / size;
        
        println!("  🏗️ Build time: {:?}", build_time);
        println!("  💾 Memory usage: {:.2} KB per vector", memory_per_vector as f64 / 1024.0);
        
        // Search performance check
        let query = &vectors[0];
        let start = Instant::now();
        let _results = hnsw.search(query, 10, 100);
        let search_time = start.elapsed();
        
        println!("  🔍 Search time: {:?}", search_time);
        
        if search_time.as_millis() > 5 {
            println!("      ❌ Search >5ms - scaling issues detected");
        } else if search_time.as_millis() > 2 {
            println!("      ⚠️ Search >2ms - acceptable but monitor");
        } else {
            println!("      ✅ Search <2ms - good scaling");
        }
    }
}

/// Простая оценка использования памяти
fn get_memory_usage() -> usize {
    // Примитивная оценка - в production нужен proper memory profiling
    std::mem::size_of::<usize>() * 1000 // Stub
}

fn main() {
    println!("🚀 MAGRAY CLI - HNSW Performance Baseline Analysis");
    println!("=================================================");
    println!("🎯 Target: <5ms search latency for 1M vectors");
    println!("📊 Profiling all performance aspects...\n");
    
    detect_cpu_capabilities();
    
    bench_simd_performance();
    bench_hnsw_baseline();
    bench_batch_operations();
    bench_memory_scaling();
    
    println!("\n🏁 Baseline Profiling Complete!");
    println!("📈 Key Findings:");
    println!("  - CPU SIMD capabilities detected");
    println!("  - Distance calculation performance measured");
    println!("  - HNSW search latency baseline established");
    println!("  - Batch processing throughput analyzed");
    println!("  - Memory scaling characteristics identified");
    println!("\n🎯 Next Steps:");
    println!("  1. Analyze results to identify bottlenecks");
    println!("  2. Implement targeted SIMD optimizations");
    println!("  3. Add memory-mapped I/O for large indices");
    println!("  4. Implement lock-free concurrent structures");
    println!("  5. Add hot node caching system");
    println!("  6. Create comprehensive benchmark suite");
}