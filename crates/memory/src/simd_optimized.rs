//! Оптимизированные SIMD реализации для векторных операций
//! 
//! Этот модуль содержит высокопроизводительные SIMD реализации cosine distance
//! и других векторных операций, оптимизированные для достижения 2-4x speedup
//! 
//! @component: {"k":"C","id":"simd_optimized","t":"High-performance SIMD cosine distance","m":{"cur":95,"tgt":100,"u":"%"},"f":["simd","avx2","avx512","performance","vectorization","cache-optimized"]}

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::time::Instant;

/// Высокопроизводительная горизонтальная сумма AVX2
/// 
/// Использует эффективные hadd инструкции вместо медленного извлечения
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn horizontal_sum_avx2_optimized(v: __m256) -> f32 {
    // Используем hadd_ps для более эффективного горизонтального сложения
    let hadd1 = _mm256_hadd_ps(v, v);           // [a0+a1, a2+a3, a4+a5, a6+a7, a0+a1, a2+a3, a4+a5, a6+a7]
    let hadd2 = _mm256_hadd_ps(hadd1, hadd1);   // [a0+a1+a2+a3, a4+a5+a6+a7, *, *, a0+a1+a2+a3, a4+a5+a6+a7, *, *]
    
    // Извлекаем high и low 128-bit части
    let sum_low = _mm256_castps256_ps128(hadd2);
    let sum_high = _mm256_extractf128_ps(hadd2, 1);
    
    // Финальное сложение
    let final_sum = _mm_add_ss(sum_low, sum_high);
    _mm_cvtss_f32(final_sum)
}

/// Альтернативная оптимизированная горизонтальная сумма
/// 
/// Использует permute operations для лучшей pipeline efficiency
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn horizontal_sum_avx2_permute(v: __m256) -> f32 {
    // Fold 256-bit to 128-bit
    let sum128 = _mm_add_ps(_mm256_castps256_ps128(v), _mm256_extractf128_ps(v, 1));
    
    // Fold 128-bit to 64-bit 
    let sum64 = _mm_add_ps(sum128, _mm_movehl_ps(sum128, sum128));
    
    // Fold 64-bit to 32-bit - используем более эффективный shuffle
    let sum32 = _mm_add_ss(sum64, _mm_shuffle_ps(sum64, sum64, 0x01));
    
    _mm_cvtss_f32(sum32)
}

/// Ультра-оптимизированный cosine distance с AVX2
/// 
/// Основные оптимизации:
/// - Оптимизированная горизонтальная сумма
/// - Prefetching hints для cache performance
/// - Минимальное количество memory accesses
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn cosine_distance_avx2_ultra(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 8, 0, "Vector length must be multiple of 8 for AVX2");
    
    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();
    
    let len = a.len();
    let chunks = len / 8;
    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    
    // Prefetch первые cache lines
    _mm_prefetch(a_ptr as *const i8, _MM_HINT_T0);
    _mm_prefetch(b_ptr as *const i8, _MM_HINT_T0);
    
    for i in 0..chunks {
        let idx = i * 8;
        
        // Prefetch следующие cache lines заблаговременно
        if (idx + 64) < len {
            _mm_prefetch(a_ptr.add(idx + 64) as *const i8, _MM_HINT_T0);
            _mm_prefetch(b_ptr.add(idx + 64) as *const i8, _MM_HINT_T0);
        }
        
        // Загружаем 8 элементов за раз - используем aligned load если возможно
        let va = _mm256_loadu_ps(a_ptr.add(idx));
        let vb = _mm256_loadu_ps(b_ptr.add(idx));
        
        // Все FMA операции в одном блоке для лучшего пайплайнинга
        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }
    
    // Оптимизированное горизонтальное суммирование
    let dot_sum = horizontal_sum_avx2_optimized(dot_product);
    let norm_a_sum = horizontal_sum_avx2_optimized(norm_a);
    let norm_b_sum = horizontal_sum_avx2_optimized(norm_b);
    
    // Cosine similarity с fast inverse sqrt если нужно
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    
    // Cosine distance = 1 - similarity
    1.0 - similarity
}

/// Memory-aligned версия для оптимального cache usage
/// 
/// Требует 32-byte aligned data для максимальной производительности
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn cosine_distance_avx2_aligned(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 8, 0, "Vector length must be multiple of 8 for AVX2");
    
    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();
    
    let chunks = a.len() / 8;
    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    
    for i in 0..chunks {
        let idx = i * 8;
        
        // Проверяем alignment и используем aligned load если возможно
        let va = if (a_ptr.add(idx) as usize) % 32 == 0 {
            _mm256_load_ps(a_ptr.add(idx))
        } else {
            _mm256_loadu_ps(a_ptr.add(idx))
        };
        
        let vb = if (b_ptr.add(idx) as usize) % 32 == 0 {
            _mm256_load_ps(b_ptr.add(idx))
        } else {
            _mm256_loadu_ps(b_ptr.add(idx))
        };
        
        // FMA операции
        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }
    
    // Используем permute версию для разнообразия
    let dot_sum = horizontal_sum_avx2_permute(dot_product);
    let norm_a_sum = horizontal_sum_avx2_permute(norm_a);
    let norm_b_sum = horizontal_sum_avx2_permute(norm_b);
    
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// AVX-512 оптимизированная версия для современных процессоров
/// 
/// Обрабатывает 16 элементов за раз для максимальной производительности
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn cosine_distance_avx512(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 16, 0, "Vector length must be multiple of 16 for AVX512");
    
    let mut dot_product = _mm512_setzero_ps();
    let mut norm_a = _mm512_setzero_ps();
    let mut norm_b = _mm512_setzero_ps();
    
    let len = a.len();
    let chunks = len / 16;
    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    
    // Prefetch более агрессивно для AVX-512
    _mm_prefetch(a_ptr as *const i8, _MM_HINT_T0);
    _mm_prefetch(b_ptr as *const i8, _MM_HINT_T0);
    if len >= 128 {
        _mm_prefetch(a_ptr.add(128) as *const i8, _MM_HINT_T0);
        _mm_prefetch(b_ptr.add(128) as *const i8, _MM_HINT_T0);
    }
    
    for i in 0..chunks {
        let idx = i * 16;
        
        // Prefetch следующие данные
        if (idx + 128) < len {
            _mm_prefetch(a_ptr.add(idx + 128) as *const i8, _MM_HINT_T0);
            _mm_prefetch(b_ptr.add(idx + 128) as *const i8, _MM_HINT_T0);
        }
        
        // Загружаем 16 элементов за раз
        let va = _mm512_loadu_ps(a_ptr.add(idx));
        let vb = _mm512_loadu_ps(b_ptr.add(idx));
        
        // AVX-512 FMA операции
        dot_product = _mm512_fmadd_ps(va, vb, dot_product);
        norm_a = _mm512_fmadd_ps(va, va, norm_a);
        norm_b = _mm512_fmadd_ps(vb, vb, norm_b);
    }
    
    // AVX-512 horizontal сумма - намного эффективнее
    let dot_sum = horizontal_sum_avx512(dot_product);
    let norm_a_sum = horizontal_sum_avx512(norm_a);
    let norm_b_sum = horizontal_sum_avx512(norm_b);
    
    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// Высокопроизводительная горизонтальная сумма AVX-512
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
pub unsafe fn horizontal_sum_avx512(v: __m512) -> f32 {
    // AVX-512 имеет более эффективные reduce операции
    let sum256_low = _mm512_castps512_ps256(v);
    let sum256_high = _mm512_extractf32x8_ps(v, 1);
    let sum256 = _mm256_add_ps(sum256_low, sum256_high);
    
    // Используем нашу оптимизированную AVX2 функцию
    horizontal_sum_avx2_optimized(sum256)
}

/// Batch обработка с SIMD оптимизациями
/// 
/// Оптимизировано для обработки множественных векторов с кэш-дружественным доступом
#[cfg(target_arch = "x86_64")]
pub fn batch_cosine_distance_optimized(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    let mut results = Vec::with_capacity(queries.len());
    
    // Определяем лучшую стратегию SIMD
    let use_avx512 = is_x86_feature_detected!("avx512f") && target.len() % 16 == 0;
    let use_avx2 = is_x86_feature_detected!("avx2") && target.len() % 8 == 0;
    
    if use_avx512 {
        for query in queries.iter() {
            let distance = unsafe { cosine_distance_avx512(query, target) };
            results.push(distance);
        }
    } else if use_avx2 {
        for query in queries.iter() {
            let distance = unsafe { cosine_distance_avx2_ultra(query, target) };
            results.push(distance);
        }
    } else {
        // Fallback к скалярной версии
        for query in queries.iter() {
            let distance = cosine_distance_scalar(query, target);
            results.push(distance);
        }
    }
    
    results
}

/// Скалярная реализация для сравнения и fallback
pub fn cosine_distance_scalar(a: &[f32], b: &[f32]) -> f32 {
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

/// Автоматический выбор наилучшей реализации
/// 
/// Выбирает оптимальную SIMD реализацию основываясь на возможностях CPU
pub fn cosine_distance_auto(a: &[f32], b: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") && a.len() % 16 == 0 {
            unsafe { cosine_distance_avx512(a, b) }
        } else if is_x86_feature_detected!("avx2") && a.len() % 8 == 0 {
            unsafe { cosine_distance_avx2_ultra(a, b) }
        } else {
            cosine_distance_scalar(a, b)
        }
    }
    
    #[cfg(not(target_arch = "x86_64"))]
    {
        cosine_distance_scalar(a, b)
    }
}

/// Память-оптимизированная версия с предварительным выравниванием
/// 
/// Создает aligned copies данных для максимальной производительности
#[cfg(target_arch = "x86_64")]
pub fn cosine_distance_memory_optimized(a: &[f32], b: &[f32]) -> f32 {
    // Проверяем можем ли использовать данные как есть
    let a_aligned = (a.as_ptr() as usize) % 32 == 0;
    let b_aligned = (b.as_ptr() as usize) % 32 == 0;
    
    if a_aligned && b_aligned && a.len() % 8 == 0 && is_x86_feature_detected!("avx2") {
        unsafe { cosine_distance_avx2_aligned(a, b) }
    } else if is_x86_feature_detected!("avx2") && a.len() % 8 == 0 {
        unsafe { cosine_distance_avx2_ultra(a, b) }
    } else {
        cosine_distance_scalar(a, b)
    }
}

/// Comprehensive performance benchmarking
/// 
/// Тестирует все реализации и выводит детальную статистику производительности
pub fn run_comprehensive_benchmark() {
    println!("🚀 Comprehensive SIMD Optimization Benchmark");
    println!("============================================");
    
    // Тестовые данные
    const DIMENSION: usize = 1024;
    const ITERATIONS: usize = 10000;
    
    let vector_a: Vec<f32> = (0..DIMENSION).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect();
    let vector_b: Vec<f32> = (0..DIMENSION).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect();
    
    // Детектируем SIMD capabilities
    #[cfg(target_arch = "x86_64")]
    {
        let avx2 = is_x86_feature_detected!("avx2");
        let avx512 = is_x86_feature_detected!("avx512f");
        let fma = is_x86_feature_detected!("fma");
        
        println!("SIMD Support:");
        println!("  AVX2: {}", if avx2 { "✅" } else { "❌" });
        println!("  AVX-512: {}", if avx512 { "✅" } else { "❌" });
        println!("  FMA: {}", if fma { "✅" } else { "❌" });
    }
    println!();
    
    // Базовый scalar benchmark
    let start = Instant::now();
    let mut scalar_result = 0.0;
    for _ in 0..ITERATIONS {
        scalar_result += cosine_distance_scalar(&vector_a, &vector_b);
    }
    let scalar_duration = start.elapsed();
    
    println!("📊 Scalar Implementation:");
    println!("  Duration: {:?}", scalar_duration);
    println!("  Avg per operation: {:.2} μs", scalar_duration.as_micros() as f64 / ITERATIONS as f64);
    println!("  Result: {:.6}", scalar_result / ITERATIONS as f32);
    println!();
    
    // Тестируем оптимизированные SIMD версии
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            // AVX2 Ultra версия
            let start = Instant::now();
            let mut avx2_result = 0.0;
            for _ in 0..ITERATIONS {
                avx2_result += unsafe { cosine_distance_avx2_ultra(&vector_a, &vector_b) };
            }
            let avx2_duration = start.elapsed();
            
            println!("⚡ AVX2 Ultra-Optimized:");
            println!("  Duration: {:?}", avx2_duration);
            println!("  Avg per operation: {:.2} μs", avx2_duration.as_micros() as f64 / ITERATIONS as f64);
            println!("  Result: {:.6}", avx2_result / ITERATIONS as f32);
            
            let speedup = scalar_duration.as_nanos() as f64 / avx2_duration.as_nanos() as f64;
            println!("  🚀 Speedup: {:.2}x", speedup);
            
            let accuracy_diff = (scalar_result - avx2_result).abs() / ITERATIONS as f32;
            println!("  🎯 Accuracy diff: {:.8}", accuracy_diff);
            println!();
            
            // Memory-aligned версия
            let start = Instant::now();
            let mut _aligned_result = 0.0;
            for _ in 0..ITERATIONS {
                _aligned_result += cosine_distance_memory_optimized(&vector_a, &vector_b);
            }
            let aligned_duration = start.elapsed();
            
            println!("⚡ Memory-Aligned Version:");
            println!("  Duration: {:?}", aligned_duration);
            let aligned_speedup = scalar_duration.as_nanos() as f64 / aligned_duration.as_nanos() as f64;
            println!("  🚀 Speedup: {:.2}x", aligned_speedup);
            println!();
        }
        
        // AVX-512 если доступно
        if is_x86_feature_detected!("avx512f") {
            let start = Instant::now();
            let mut _avx512_result = 0.0;
            for _ in 0..ITERATIONS {
                _avx512_result += unsafe { cosine_distance_avx512(&vector_a, &vector_b) };
            }
            let avx512_duration = start.elapsed();
            
            println!("🚀 AVX-512 Implementation:");
            println!("  Duration: {:?}", avx512_duration);
            let avx512_speedup = scalar_duration.as_nanos() as f64 / avx512_duration.as_nanos() as f64;
            println!("  🚀 Speedup: {:.2}x", avx512_speedup);
            println!();
        }
    }
    
    // Batch тестирование
    println!("📦 Batch Performance:");
    let batch_queries: Vec<Vec<f32>> = (0..100)
        .map(|_| (0..DIMENSION).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect())
        .collect();
    
    let start = Instant::now();
    let _batch_results = batch_cosine_distance_optimized(&batch_queries, &vector_a);
    let batch_duration = start.elapsed();
    
    println!("  Optimized batch (100 vectors): {:?}", batch_duration);
    println!("  Per vector: {:.2} μs", batch_duration.as_micros() as f64 / 100.0);
    println!();
    
    println!("🏁 Optimization benchmark completed!");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_horizontal_sum_optimizations() {
        let test_values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let expected_sum: f32 = test_values.iter().sum();
        
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    let v = _mm256_loadu_ps(test_values.as_ptr());
                    
                    let sum1 = horizontal_sum_avx2_optimized(v);
                    let sum2 = horizontal_sum_avx2_permute(v);
                    
                    assert!((sum1 - expected_sum).abs() < 1e-6);
                    assert!((sum2 - expected_sum).abs() < 1e-6);
                }
            }
        }
    }
    
    #[test]
    fn test_cosine_distance_accuracy() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        
        let scalar_result = cosine_distance_scalar(&a, &b);
        let auto_result = cosine_distance_auto(&a, &b);
        
        assert!((scalar_result - auto_result).abs() < 1e-6);
    }
    
    #[test]
    fn test_batch_operations() {
        let queries = vec![
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
        ];
        let target = vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        
        let results = batch_cosine_distance_optimized(&queries, &target);
        assert_eq!(results.len(), 2);
        
        // Проверяем что результаты разумные (cosine distance между 0 и 2)
        for result in results {
            assert!(result >= 0.0 && result <= 2.0);
        }
    }
}