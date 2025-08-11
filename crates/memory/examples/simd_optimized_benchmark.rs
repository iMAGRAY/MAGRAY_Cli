#![cfg(feature = "extended-tests")]

//! Comprehensive SIMD Optimization Benchmark
//!
//! Сравнивает производительность различных SIMD реализаций cosine distance
//! для достижения целевого 2-4x speedup

use memory::simd_optimized::*;
use std::time::Instant;

fn main() {
    println!("🚀 Comprehensive SIMD Optimization Benchmark");
    println!("==============================================");

    // Проверим компиляцию и запустим comprehensive benchmark
    run_comprehensive_benchmark();

    println!("\n🔬 Detailed Performance Analysis");
    println!("================================");

    // Дополнительные специфичные тесты
    test_horizontal_sum_performance();
    test_memory_alignment_impact();
    test_prefetching_effectiveness();
    test_different_vector_sizes();

    println!("\n✅ All benchmarks completed successfully!");
}

fn test_horizontal_sum_performance() {
    println!("\n📊 Horizontal Sum Implementation Comparison:");

    const ITERATIONS: usize = 100_000;
    let test_data: Vec<f32> = (0..8).map(|i| i as f32 + 1.0).collect();

    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") {
            unsafe {
                use std::arch::x86_64::*;
                let v = _mm256_loadu_ps(test_data.as_ptr());

                // Оригинальная реализация (медленная)
                let start = Instant::now();
                for _ in 0..ITERATIONS {
                    let _sum = horizontal_sum_original(v);
                }
                let original_duration = start.elapsed();

                // Наша оптимизированная версия
                let start = Instant::now();
                for _ in 0..ITERATIONS {
                    let _sum = horizontal_sum_avx2_optimized(v);
                }
                let optimized_duration = start.elapsed();

                // Permute версия
                let start = Instant::now();
                for _ in 0..ITERATIONS {
                    let _sum = horizontal_sum_avx2_permute(v);
                }
                let permute_duration = start.elapsed();

                println!("  Original horizontal_sum: {:?}", original_duration);
                println!("  Optimized horizontal_sum: {:?}", optimized_duration);
                println!("  Permute horizontal_sum: {:?}", permute_duration);

                let speedup1 =
                    original_duration.as_nanos() as f64 / optimized_duration.as_nanos() as f64;
                let speedup2 =
                    original_duration.as_nanos() as f64 / permute_duration.as_nanos() as f64;

                println!("  Horizontal sum optimization: {:.2}x speedup", speedup1);
                println!("  Permute optimization: {:.2}x speedup", speedup2);
            }
        } else {
            println!("  ❌ AVX2 not supported - skipping horizontal sum test");
        }
    }
}

// Реализация оригинальной (медленной) horizontal_sum для сравнения
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_original(v: std::arch::x86_64::__m256) -> f32 {
    use std::arch::x86_64::*;
    // Это копия оригинальной медленной реализации из бенчмарка
    let hi = _mm256_extractf128_ps(v, 1);
    let lo = _mm256_castps256_ps128(v);
    let sum128 = _mm_add_ps(hi, lo);

    let hi64 = _mm_movehl_ps(sum128, sum128);
    let sum64 = _mm_add_ps(sum128, hi64);

    let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
    let sum32 = _mm_add_ss(sum64, hi32);

    _mm_cvtss_f32(sum32)
}

fn test_memory_alignment_impact() {
    println!("\n🧠 Memory Alignment Impact Analysis:");

    const DIMENSION: usize = 1024;
    const ITERATIONS: usize = 1000;

    // Создаем aligned и unaligned данные
    let mut aligned_data_a = vec![0.0f32; DIMENSION + 8]; // Дополнительное пространство для alignment
    let mut aligned_data_b = vec![0.0f32; DIMENSION + 8];

    // Заполняем случайными данными
    for i in 0..DIMENSION {
        aligned_data_a[i] = rand::random::<f32>() * 2.0 - 1.0;
        aligned_data_b[i] = rand::random::<f32>() * 2.0 - 1.0;
    }

    // Обычные векторы (potentially unaligned)
    let unaligned_a: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    let unaligned_b: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();

    // Тест с unaligned данными
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _result = cosine_distance_auto(&unaligned_a, &unaligned_b);
    }
    let unaligned_duration = start.elapsed();

    // Тест с potentially aligned данными
    let aligned_slice_a = &aligned_data_a[..DIMENSION];
    let aligned_slice_b = &aligned_data_b[..DIMENSION];

    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _result = cosine_distance_memory_optimized(aligned_slice_a, aligned_slice_b);
    }
    let aligned_duration = start.elapsed();

    println!("  Unaligned data: {:?}", unaligned_duration);
    println!("  Aligned data: {:?}", aligned_duration);

    if aligned_duration < unaligned_duration {
        let speedup = unaligned_duration.as_nanos() as f64 / aligned_duration.as_nanos() as f64;
        println!("  Memory alignment speedup: {:.2}x", speedup);
    } else {
        println!("  No significant alignment benefit detected");
    }
}

fn test_prefetching_effectiveness() {
    println!("\n🎯 Prefetching Effectiveness Test:");

    const DIMENSION: usize = 4096; // Больший размер для демонстрации prefetching
    const ITERATIONS: usize = 100;

    let large_vector_a: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();
    let large_vector_b: Vec<f32> = (0..DIMENSION)
        .map(|_| rand::random::<f32>() * 2.0 - 1.0)
        .collect();

    // Без prefetching (используем обычную функцию)
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        let _result = cosine_distance_scalar(&large_vector_a, &large_vector_b);
    }
    let no_prefetch_duration = start.elapsed();

    // С prefetching (наша оптимизированная версия)
    #[cfg(target_arch = "x86_64")]
    {
        if std::arch::is_x86_feature_detected!("avx2") && DIMENSION % 8 == 0 {
            let start = Instant::now();
            for _ in 0..ITERATIONS {
                let _result =
                    unsafe { cosine_distance_avx2_ultra(&large_vector_a, &large_vector_b) };
            }
            let prefetch_duration = start.elapsed();

            println!("  Without prefetching: {:?}", no_prefetch_duration);
            println!("  With prefetching: {:?}", prefetch_duration);

            if prefetch_duration < no_prefetch_duration {
                let speedup =
                    no_prefetch_duration.as_nanos() as f64 / prefetch_duration.as_nanos() as f64;
                println!("  Prefetching speedup: {:.2}x", speedup);
            } else {
                println!("  No significant prefetching benefit for this size");
            }
        } else {
            println!("  ❌ AVX2 not supported or invalid dimension - skipping prefetch test");
        }
    }
}

fn test_different_vector_sizes() {
    println!("\n📏 Vector Size Scalability Analysis:");

    let sizes = vec![128, 256, 512, 1024, 2048, 4096];

    for &size in &sizes {
        if size % 8 != 0 {
            continue;
        } // Skip sizes not divisible by 8 for AVX2

        let vector_a: Vec<f32> = (0..size)
            .map(|_| rand::random::<f32>() * 2.0 - 1.0)
            .collect();
        let vector_b: Vec<f32> = (0..size)
            .map(|_| rand::random::<f32>() * 2.0 - 1.0)
            .collect();

        const ITERATIONS: usize = 1000;

        // Scalar версия
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _result = cosine_distance_scalar(&vector_a, &vector_b);
        }
        let scalar_duration = start.elapsed();

        // Optimized SIMD версия
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            let _result = cosine_distance_auto(&vector_a, &vector_b);
        }
        let simd_duration = start.elapsed();

        let speedup = scalar_duration.as_nanos() as f64 / simd_duration.as_nanos() as f64;

        println!(
            "  Size {}: Scalar={:?}, SIMD={:?}, Speedup={:.2}x",
            size, scalar_duration, simd_duration, speedup
        );
    }
}
