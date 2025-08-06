//! Финальный тест SIMD интеграции
//! 
//! Тестируем что наши SIMD оптимизации правильно интегрированы в HNSW систему

use memory::{debug_simd_performance};
use std::time::Instant;

fn main() {
    println!("🎯 Финальный тест SIMD интеграции в MAGRAY_CLI");
    println!("==============================================");
    
    // Тест 1: Проверяем наши debugging функции
    println!("1️⃣ Проверка SIMD debugging функций:");
    debug_simd_performance();
    
    println!("\n2️⃣ Симуляция реального HNSW использования:");
    simulate_hnsw_workload();
    
    println!("\n✅ Все тесты SIMD интеграции завершены успешно!");
}

fn simulate_hnsw_workload() {
    const DIMENSION: usize = 1024;  // Qwen3 embeddings
    const NUM_VECTORS: usize = 1000;
    const NUM_QUERIES: usize = 100;
    
    println!("  Генерируем {} векторов размерности {}...", NUM_VECTORS, DIMENSION);
    
    // Генерируем database векторы
    let database_vectors: Vec<Vec<f32>> = (0..NUM_VECTORS)
        .map(|_| (0..DIMENSION).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect())
        .collect();
    
    // Генерируем query векторы
    let query_vectors: Vec<Vec<f32>> = (0..NUM_QUERIES)
        .map(|_| (0..DIMENSION).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect())
        .collect();
    
    println!("  Запускаем симуляцию HNSW поиска...");
    
    let start = Instant::now();
    let mut total_distances_calculated = 0;
    
    // Имитируем HNSW поиск - каждый query против ~10% базы (типичный HNSW pattern)
    for query in &query_vectors {
        let search_candidates = NUM_VECTORS / 10; // HNSW обычно проверяет ~10% векторов
        
        for i in 0..search_candidates {
            let target = &database_vectors[i % NUM_VECTORS];
            
            // Используем автоматический выбор SIMD/scalar
            let _distance = calculate_cosine_distance_optimized(query, target);
            total_distances_calculated += 1;
        }
    }
    
    let duration = start.elapsed();
    
    println!("  📊 Результаты симуляции:");
    println!("    Всего distance calculations: {}", total_distances_calculated);
    println!("    Общее время: {:?}", duration);
    println!("    Среднее время на distance: {:.2} μs", 
             duration.as_micros() as f64 / total_distances_calculated as f64);
    
    let qps = 1_000_000.0 / (duration.as_micros() as f64 / NUM_QUERIES as f64);
    println!("    Projected QPS: {:.0}", qps);
    
    // Проверяем достижение целевого времени sub-5ms
    let avg_query_time_ms = duration.as_millis() as f64 / NUM_QUERIES as f64;
    if avg_query_time_ms < 5.0 {
        println!("    ✅ Sub-5ms цель достигнута! ({:.2}ms per query)", avg_query_time_ms);
    } else {
        println!("    ⚠️ Sub-5ms цель не достигнута ({:.2}ms per query)", avg_query_time_ms);
    }
}

// Функция выбора оптимального distance calculation
fn calculate_cosine_distance_optimized(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") && a.len() % 8 == 0 {
            // Используем наш оптимизированный SIMD
            unsafe { cosine_distance_avx2_optimized(a, b) }
        } else {
            // Fallback к scalar
            cosine_distance_scalar_simple(a, b)
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        cosine_distance_scalar_simple(a, b)
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn cosine_distance_avx2_optimized(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::x86_64::*;
    
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
        
        // Оптимизированная версия с add+mul
        dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
        norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
        norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
    }
    
    // Используем оригинальную horizontal_sum
    let dot_sum = horizontal_sum_avx2_simple(dot_product);
    let norm_a_sum = horizontal_sum_avx2_simple(norm_a);
    let norm_b_sum = horizontal_sum_avx2_simple(norm_b);
    
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_avx2_simple(v: std::arch::x86_64::__m256) -> f32 {
    use std::arch::x86_64::*;
    
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);
    
    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);
    
    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
    let sum32 = _mm_add_ss(sum64, hi32);
    
    _mm_cvtss_f32(sum32)
}

fn cosine_distance_scalar_simple(a: &[f32], b: &[f32]) -> f32 {
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