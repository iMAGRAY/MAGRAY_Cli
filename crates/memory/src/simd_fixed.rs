//! ИСПРАВЛЕННАЯ SIMD реализация на основе анализа производительности
//!
//! КРИТИЧЕСКОЕ ОТКРЫТИЕ: hadd_ps операции медленные! Оригинальная реализация была правильная.
//! Проблема НЕ в horizontal_sum, а в других частях!
//!
//! @component: {"k":"C","id":"simd_fixed","t":"Fixed high-performance SIMD implementation","m":{"cur":95,"tgt":100,"u":"%"},"f":["simd","avx2","fixed","performance","debugging"]}

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::time::Instant;

/// ОРИГИНАЛЬНАЯ horizontal_sum (она правильная!)
///
/// После анализа - эта функция НЕ является узким местом
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
/// # Safety
/// Вызывающий должен гарантировать поддержку AVX2 CPU и корректное выравнивание типов SSE/AVX intrinsics.
pub unsafe fn horizontal_sum_avx2_correct(v: __m256) -> f32 {
    // Это ПРАВИЛЬНАЯ реализация из оригинала!
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);

    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);

    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
    let sum32 = _mm_add_ss(sum64, hi32);

    _mm_cvtss_f32(sum32)
}

/// МИНИМАЛИСТИЧНАЯ SIMD версия без лишних оптимизаций
///
/// Убираем все "умные" фичи и оставляем только основные инструкции
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
/// # Safety
/// Вызов допустим только на CPU с поддержкой AVX2; входные срезы должны иметь одинаковую длину и быть кратны 8.
pub unsafe fn cosine_distance_avx2_minimal(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 8, 0);

    let mut dot_sum = 0.0f32;
    let mut norm_a_sum = 0.0f32;
    let mut norm_b_sum = 0.0f32;

    let chunks = a.len() / 8;

    for i in 0..chunks {
        let idx = i * 8;

        // Простая загрузка без всяких alignment проверок
        let va = _mm256_loadu_ps(a.as_ptr().add(idx));
        let vb = _mm256_loadu_ps(b.as_ptr().add(idx));

        // Накапливаем в векторы
        let dot_vec = _mm256_mul_ps(va, vb);
        let norm_a_vec = _mm256_mul_ps(va, va);
        let norm_b_vec = _mm256_mul_ps(vb, vb);

        // Суммируем скалярно - возможно это проще
        dot_sum += horizontal_sum_avx2_correct(dot_vec);
        norm_a_sum += horizontal_sum_avx2_correct(norm_a_vec);
        norm_b_sum += horizontal_sum_avx2_correct(norm_b_vec);
    }

    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// ВЕКТОРНАЯ АККУМУЛЯЦИЯ версия (как в оригинале)
///
/// Именно тот подход который был в оригинальном коде
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
/// # Safety
/// Требуется AVX2; входные буферы одинаковой длины, кратной 8.
pub unsafe fn cosine_distance_avx2_vectorized(a: &[f32], b: &[f32]) -> f32 {
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

        // ОРИГИНАЛЬНЫЙ паттерн с add вместо fmadd
        dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
        norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
        norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
    }

    let dot_sum = horizontal_sum_avx2_correct(dot_product);
    let norm_a_sum = horizontal_sum_avx2_correct(norm_a);
    let norm_b_sum = horizontal_sum_avx2_correct(norm_b);

    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// ПОПЫТКА с FMA но правильным способом
///
/// Используем FMA только если точно знаем что делаем
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
/// # Safety
/// Требует AVX2 и FMA; входные буферы одинаковой длины, кратной 8.
pub unsafe fn cosine_distance_avx2_fma(a: &[f32], b: &[f32]) -> f32 {
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

        // FMA инструкции - но только если точно нужны
        dot_product = _mm256_fmadd_ps(va, vb, dot_product);
        norm_a = _mm256_fmadd_ps(va, va, norm_a);
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b);
    }

    let dot_sum = horizontal_sum_avx2_correct(dot_product);
    let norm_a_sum = horizontal_sum_avx2_correct(norm_a);
    let norm_b_sum = horizontal_sum_avx2_correct(norm_b);

    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// РУЧНОЙ LOOP UNROLLING - возможно компилятор плохо разворачивает цикл
///
/// Попробуем обработать 2 чанка за итерацию
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
/// # Safety
/// Требуется AVX2; входные буферы одинаковой длины, кратной 16.
pub unsafe fn cosine_distance_avx2_unrolled(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 16, 0); // Кратно 16 для двойных чанков

    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();

    let chunks = a.len() / 16; // 16 элементов за итерацию = 2 AVX2 регистра

    for i in 0..chunks {
        let idx1 = i * 16;
        let idx2 = idx1 + 8;

        // Первый чанк
        let va1 = _mm256_loadu_ps(a.as_ptr().add(idx1));
        let vb1 = _mm256_loadu_ps(b.as_ptr().add(idx1));

        // Второй чанк
        let va2 = _mm256_loadu_ps(a.as_ptr().add(idx2));
        let vb2 = _mm256_loadu_ps(b.as_ptr().add(idx2));

        // Операции для первого чанка
        dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va1, vb1));
        norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va1, va1));
        norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb1, vb1));

        // Операции для второго чанка
        dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va2, vb2));
        norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va2, va2));
        norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb2, vb2));
    }

    let dot_sum = horizontal_sum_avx2_correct(dot_product);
    let norm_a_sum = horizontal_sum_avx2_correct(norm_a);
    let norm_b_sum = horizontal_sum_avx2_correct(norm_b);

    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// ALIGNED LOAD версия - но с проверкой можем ли мы её использовать
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
/// # Safety
/// Требуется AVX2; входные буферы одинаковой длины, кратной 8.
pub unsafe fn cosine_distance_avx2_aligned_check(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    debug_assert_eq!(a.len() % 8, 0);

    let a_ptr = a.as_ptr();
    let b_ptr = b.as_ptr();
    let a_aligned = (a_ptr as usize).is_multiple_of(32);
    let b_aligned = (b_ptr as usize).is_multiple_of(32);

    let mut dot_product = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();

    let chunks = a.len() / 8;

    if a_aligned && b_aligned {
        // Используем aligned loads
        for i in 0..chunks {
            let idx = i * 8;
            let va = _mm256_load_ps(a_ptr.add(idx));
            let vb = _mm256_load_ps(b_ptr.add(idx));

            dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
            norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
            norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
        }
    } else {
        // Fallback к unaligned loads
        for i in 0..chunks {
            let idx = i * 8;
            let va = _mm256_loadu_ps(a_ptr.add(idx));
            let vb = _mm256_loadu_ps(b_ptr.add(idx));

            dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
            norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
            norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
        }
    }

    let dot_sum = horizontal_sum_avx2_correct(dot_product);
    let norm_a_sum = horizontal_sum_avx2_correct(norm_a);
    let norm_b_sum = horizontal_sum_avx2_correct(norm_b);

    let similarity = dot_sum / (norm_a_sum.sqrt() * norm_b_sum.sqrt());
    1.0 - similarity
}

/// Скалярная baseline реализация
pub fn cosine_distance_scalar_baseline(a: &[f32], b: &[f32]) -> f32 {
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

/// DEBUGGING бенчмарк для поиска реальной проблемы
pub fn debug_simd_performance() {
    println!("🔍 DEBUGGING SIMD Performance Issues");
    println!("====================================");

    const DIMENSION: usize = 1024;
    const ITERATIONS: usize = 10000;

    let vector_a: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    let vector_b: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();

    #[cfg(target_arch = "x86_64")]
    {
        println!("CPU Features:");
        println!("  AVX2: {}", is_x86_feature_detected!("avx2"));
        println!("  FMA: {}", is_x86_feature_detected!("fma"));
        println!();

        if !is_x86_feature_detected!("avx2") {
            println!("❌ AVX2 не поддерживается - используем только scalar");
            return;
        }

        // Baseline scalar
        let start = Instant::now();
        let mut scalar_result = 0.0;
        for _ in 0..ITERATIONS {
            scalar_result += cosine_distance_scalar_baseline(&vector_a, &vector_b);
        }
        let scalar_duration = start.elapsed();

        println!("📊 Scalar Baseline:");
        println!("  Duration: {:?}", scalar_duration);
        println!(
            "  Avg per operation: {:.2} μs",
            scalar_duration.as_micros() as f64 / ITERATIONS as f64
        );
        println!("  Result: {:.6}", scalar_result / ITERATIONS as f32);
        println!();

        // Test 1: Minimal SIMD
        let start = Instant::now();
        let mut result1 = 0.0;
        for _ in 0..ITERATIONS {
            result1 += unsafe { cosine_distance_avx2_minimal(&vector_a, &vector_b) };
        }
        let duration1 = start.elapsed();

        let speedup1 = scalar_duration.as_nanos() as f64 / duration1.as_nanos() as f64;
        println!("🔧 Minimal SIMD:");
        println!("  Duration: {:?}, Speedup: {:.2}x", duration1, speedup1);
        println!(
            "  Accuracy diff: {:.8}",
            (scalar_result - result1).abs() / ITERATIONS as f32
        );

        // Test 2: Vectorized (original approach)
        let start = Instant::now();
        let mut result2 = 0.0;
        for _ in 0..ITERATIONS {
            result2 += unsafe { cosine_distance_avx2_vectorized(&vector_a, &vector_b) };
        }
        let duration2 = start.elapsed();

        let speedup2 = scalar_duration.as_nanos() as f64 / duration2.as_nanos() as f64;
        println!("📊 Vectorized Accumulation:");
        println!("  Duration: {:?}, Speedup: {:.2}x", duration2, speedup2);
        println!(
            "  Accuracy diff: {:.8}",
            (scalar_result - result2).abs() / ITERATIONS as f32
        );

        // Test 3: FMA версия
        if is_x86_feature_detected!("fma") {
            let start = Instant::now();
            let mut result3 = 0.0;
            for _ in 0..ITERATIONS {
                result3 += unsafe { cosine_distance_avx2_fma(&vector_a, &vector_b) };
            }
            let duration3 = start.elapsed();

            let speedup3 = scalar_duration.as_nanos() as f64 / duration3.as_nanos() as f64;
            println!("⚡ FMA Version:");
            println!("  Duration: {:?}, Speedup: {:.2}x", duration3, speedup3);
            println!(
                "  Accuracy diff: {:.8}",
                (scalar_result - result3).abs() / ITERATIONS as f32
            );
        }

        // Test 4: Unrolled версия (только если размер кратен 16)
        if DIMENSION.is_multiple_of(16) {
            let start = Instant::now();
            let mut result4 = 0.0;
            for _ in 0..ITERATIONS {
                result4 += unsafe { cosine_distance_avx2_unrolled(&vector_a, &vector_b) };
            }
            let duration4 = start.elapsed();

            let speedup4 = scalar_duration.as_nanos() as f64 / duration4.as_nanos() as f64;
            println!("🔄 Loop Unrolled:");
            println!("  Duration: {:?}, Speedup: {:.2}x", duration4, speedup4);
            println!(
                "  Accuracy diff: {:.8}",
                (scalar_result - result4).abs() / ITERATIONS as f32
            );
        }

        // Test 5: Aligned check версия
        let start = Instant::now();
        let mut result5 = 0.0;
        for _ in 0..ITERATIONS {
            result5 += unsafe { cosine_distance_avx2_aligned_check(&vector_a, &vector_b) };
        }
        let duration5 = start.elapsed();

        let speedup5 = scalar_duration.as_nanos() as f64 / duration5.as_nanos() as f64;
        println!("🎯 Aligned Check:");
        println!("  Duration: {:?}, Speedup: {:.2}x", duration5, speedup5);
        println!(
            "  Accuracy diff: {:.8}",
            (scalar_result - result5).abs() / ITERATIONS as f32
        );
    }

    println!("\n🏁 Debugging completed!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_implementations_accuracy() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];

        let scalar_result = cosine_distance_scalar_baseline(&a, &b);

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    let minimal_result = cosine_distance_avx2_minimal(&a, &b);
                    let vectorized_result = cosine_distance_avx2_vectorized(&a, &b);
                    let aligned_result = cosine_distance_avx2_aligned_check(&a, &b);

                    assert!((scalar_result - minimal_result).abs() < 1e-6);
                    assert!((scalar_result - vectorized_result).abs() < 1e-6);
                    assert!((scalar_result - aligned_result).abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn test_horizontal_sum_correctness() {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    let test_data = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
                    let expected_sum: f32 = test_data.iter().sum();

                    let v = _mm256_loadu_ps(test_data.as_ptr());
                    let computed_sum = horizontal_sum_avx2_correct(v);

                    assert!((expected_sum - computed_sum).abs() < 1e-6);
                }
            }
        }
    }
}
