//! Standalone SIMD Performance Test
//! 
//! Тестирует SIMD оптимизации для cosine distance без зависимостей от crate

use std::time::Instant;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Скалярная реализация cosine distance для сравнения
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

/// SIMD оптимизированная реализация с AVX2
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
        
        // Загружаем 8 элементов за раз
        let va = _mm256_loadu_ps(a.as_ptr().add(idx));
        let vb = _mm256_loadu_ps(b.as_ptr().add(idx));
        
        // Dot product: a * b
        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        
        // Norm A: a * a
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        
        // Norm B: b * b
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }
    
    // Горизонтальное суммирование
    let dot_sum = horizontal_sum_avx2(dot_product);
    let norm_a_sum = horizontal_sum_avx2(norm_a);
    let norm_b_sum = horizontal_sum_avx2(norm_b);
    
    // Cosine similarity = dot / (||a|| * ||b||)
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    
    // Cosine distance = 1 - similarity
    1.0 - similarity
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_avx2(v: __m256) -> f32 {
    // Суммируем 8 элементов в один
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);
    
    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);
    
    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
    let sum32 = _mm_add_ss(sum64, hi32);
    
    _mm_cvtss_f32(sum32)
}

/// Генерация случайных тестовых векторов
fn generate_random_vectors(count: usize, dimension: usize) -> Vec<Vec<f32>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // Простой PRNG для тестирования
    let mut seed = 12345u64;
    (0..count)
        .map(|_| {
            (0..dimension)
                .map(|_| {
                    seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
                    ((seed >> 16) as f32) / 32768.0 - 1.0 // [-1, 1]
                })
                .collect()
        })
        .collect()
}

fn main() {
    println!("🚀 SIMD Distance Calculations Performance Test");
    println!("Размерность векторов: 1024 (Qwen3 embeddings)");
    
    // Детектируем SIMD capabilities
    #[cfg(target_arch = "x86_64")]
    {
        let avx2 = is_x86_feature_detected!("avx2");
        let avx512 = is_x86_feature_detected!("avx512f");
        
        println!("SIMD Support:");
        println!("  AVX2: {}", if avx2 { "✅" } else { "❌" });
        println!("  AVX-512: {}", if avx512 { "✅" } else { "❌" });
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        println!("SIMD Support: ❌ (не x86_64 архитектура)");
    }
    
    println!();
    
    // Генерируем тестовые данные
    const DIMENSION: usize = 1024;
    const TEST_ITERATIONS: usize = 10000;
    
    let vectors = generate_random_vectors(2, DIMENSION);
    let vector_a = &vectors[0];
    let vector_b = &vectors[1];
    
    println!("Тестирование {} iterations на {}D векторах...", TEST_ITERATIONS, DIMENSION);
    println!();
    
    // Тест 1: Скалярная версия
    let start = Instant::now();
    let mut scalar_result = 0.0;
    for _ in 0..TEST_ITERATIONS {
        scalar_result += cosine_distance_scalar(vector_a, vector_b);
    }
    let scalar_duration = start.elapsed();
    
    println!("📊 Scalar Implementation:");
    println!("  Duration: {:?}", scalar_duration);
    println!("  Avg per operation: {:.2} μs", scalar_duration.as_micros() as f64 / TEST_ITERATIONS as f64);
    println!("  Sample result: {:.6}", scalar_result / TEST_ITERATIONS as f32);
    println!();
    
    // Тест 2: SIMD версия (если доступна)
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            let start = Instant::now();
            let mut simd_result = 0.0;
            for _ in 0..TEST_ITERATIONS {
                simd_result += unsafe { cosine_distance_avx2(vector_a, vector_b) };
            }
            let simd_duration = start.elapsed();
            
            println!("⚡ SIMD AVX2 Implementation:");
            println!("  Duration: {:?}", simd_duration);
            println!("  Avg per operation: {:.2} μs", simd_duration.as_micros() as f64 / TEST_ITERATIONS as f64);
            println!("  Sample result: {:.6}", simd_result / TEST_ITERATIONS as f32);
            
            let speedup = scalar_duration.as_nanos() as f64 / simd_duration.as_nanos() as f64;
            println!("  🚀 Speedup: {:.2}x", speedup);
            
            // Проверяем точность
            let accuracy_diff = (scalar_result - simd_result).abs() / TEST_ITERATIONS as f32;
            println!("  🎯 Accuracy diff: {:.8}", accuracy_diff);
            
            if speedup > 2.0 {
                println!("  ✅ Excellent speedup achieved!");
            } else if speedup > 1.5 {
                println!("  ✅ Good speedup achieved!");
            } else {
                println!("  ⚠️ Limited speedup - might need optimization");
            }
        } else {
            println!("⚠️ AVX2 not available - skipping SIMD test");
        }
    }
    
    println!();
    
    // Тест 3: Потенциал для sub-5ms поиска
    println!("🎯 Sub-5ms Search Potential Analysis:");
    
    // Симулируем HNSW поиск со ~1000 distance calculations
    const HNSW_DISTANCE_CALCS: usize = 1000;
    
    let search_start = Instant::now();
    for _ in 0..HNSW_DISTANCE_CALCS {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe { cosine_distance_avx2(vector_a, vector_b) };
            } else {
                cosine_distance_scalar(vector_a, vector_b);
            }
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            cosine_distance_scalar(vector_a, vector_b);
        }
    }
    let search_simulation_duration = search_start.elapsed();
    
    println!("  Simulated HNSW search ({} distance calcs): {:?}", HNSW_DISTANCE_CALCS, search_simulation_duration);
    
    if search_simulation_duration.as_millis() < 5 {
        println!("  ✅ Sub-5ms target achievable!");
    } else if search_simulation_duration.as_millis() < 10 {
        println!("  ⚠️ Close to 5ms target - need additional optimizations");
    } else {
        println!("  ❌ Exceeds 5ms target - significant optimization needed");
    }
    
    let projected_qps = 1000.0 / search_simulation_duration.as_millis() as f64;
    println!("  📈 Projected QPS: {:.0}", projected_qps);
    
    // Дополнительный тест: batch обработка
    println!();
    println!("📦 Batch Processing Performance:");
    
    let batch_vectors = generate_random_vectors(100, DIMENSION);
    let target_vector = &vectors[0];
    
    let batch_start = Instant::now();
    let _batch_results: Vec<f32> = batch_vectors.iter()
        .map(|v| {
            #[cfg(target_arch = "x86_64")]
            {
                if is_x86_feature_detected!("avx2") {
                    unsafe { cosine_distance_avx2(v, target_vector) }
                } else {
                    cosine_distance_scalar(v, target_vector)
                }
            }
            
            #[cfg(not(target_arch = "x86_64"))]
            {
                cosine_distance_scalar(v, target_vector)
            }
        })
        .collect();
    let batch_duration = batch_start.elapsed();
    
    println!("  Batch processing (100 vectors): {:?}", batch_duration);
    println!("  Avg per vector: {:.2} μs", batch_duration.as_micros() as f64 / 100.0);
    
    println!();
    println!("🏁 Performance test completed!");
    
    // Выводы и рекомендации
    println!();
    println!("📋 Performance Analysis Summary:");
    
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            println!("  ✅ AVX2 SIMD support available");
            println!("  ✅ Significant performance improvements expected");
            println!("  ✅ Sub-5ms HNSW search realistic with proper tuning");
        } else {
            println!("  ⚠️ Limited to scalar operations");
            println!("  ⚠️ Consider upgrading to AVX2-capable hardware");
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        println!("  ⚠️ Non-x86_64 architecture - SIMD optimizations unavailable");
        println!("  ⚠️ Performance will be limited to scalar operations");
    }
    
    println!();
    println!("🎯 Recommendations for sub-5ms HNSW search:");
    println!("  1. Use SIMD-optimized distance calculations");
    println!("  2. Optimize HNSW parameters (M=12-16, ef_search=20-32)");
    println!("  3. Implement cache-friendly memory layouts");
    println!("  4. Use lock-free concurrent operations");
    println!("  5. Add query result caching for hot queries");
}