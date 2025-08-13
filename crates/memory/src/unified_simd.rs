//! Unified SIMD Operations - объединяет все SIMD реализации в единый модуль
//!
//! Консолидирует дублированный код из 4 SIMD файлов:
//! - simd_ultra_optimized.rs -> условно безопасные SIMD операции
//! - simd_safe_replacement.rs -> полностью безопасная замена
//! - simd_feature_detection.rs -> определение возможностей CPU
//! - Автоматический fallback между реализациями
//!
//! @component: {"k":"C","id":"unified_simd","t":"Unified SIMD operations with automatic fallback","m":{"cur":100,"tgt":100,"u":"%"},"f":["simd","avx2","safe","unified","performance"]}

use std::time::Instant;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Переименовываем SafeAlignedVector для единообразия API
pub use crate::simd_safe_replacement::SafeAlignedVector as AlignedVector;

/// Объединенная SIMD конфигурация
#[derive(Debug, Clone)]
pub struct SIMDConfig {
    /// Использовать unsafe SIMD когда возможно (по умолчанию false для безопасности)
    pub allow_unsafe_simd: bool,
    /// Минимальный размер вектора для использования SIMD (байт)
    pub min_simd_size: usize,
    /// Предпочтительный размер chunk для обработки
    pub preferred_chunk_size: usize,
    /// Включить extensive feature detection
    pub enable_feature_detection: bool,
}

impl Default for SIMDConfig {
    fn default() -> Self {
        Self {
            allow_unsafe_simd: false, // По умолчанию безопасный режим
            min_simd_size: 32,        // 8 float32 = 32 bytes минимум для AVX2
            preferred_chunk_size: 256,
            enable_feature_detection: true,
        }
    }
}

/// Unified SIMD capability detection
#[derive(Debug, Clone)]
pub struct UnifiedSIMDCapabilities {
    pub has_sse2: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_fma: bool,
    pub has_avx512f: bool,
    pub recommended_vector_width: usize,
    pub max_safe_batch_size: usize,
    pub preferred_algorithm: SIMDAlgorithm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SIMDAlgorithm {
    /// Скалярная обработка - самая безопасная
    Scalar,
    /// Safe auto-vectorized код - баланс безопасности и производительности
    SafeVectorized,
    /// Unsafe SIMD intrinsics - максимальная производительность
    UnsafeSIMD,
}

impl UnifiedSIMDCapabilities {
    /// Комплексное определение SIMD возможностей
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            let has_sse2 = is_x86_feature_detected!("sse2");
            let has_avx = is_x86_feature_detected!("avx");
            let has_avx2 = is_x86_feature_detected!("avx2");
            let has_fma = is_x86_feature_detected!("fma");
            let has_avx512f = is_x86_feature_detected!("avx512f");

            let recommended_vector_width = if has_avx512f {
                64 // 16 float32 values
            } else if has_avx2 {
                32 // 8 float32 values
            } else if has_sse2 {
                16 // 4 float32 values
            } else {
                4 // scalar
            };

            let max_safe_batch_size = if has_avx512f {
                2048
            } else if has_avx2 {
                1024
            } else {
                256
            };

            // Выбираем самый безопасный алгоритм по умолчанию
            let preferred_algorithm = if has_avx2 && has_fma {
                SIMDAlgorithm::SafeVectorized
            } else {
                SIMDAlgorithm::Scalar
            };

            Self {
                has_sse2,
                has_avx,
                has_avx2,
                has_fma,
                has_avx512f,
                recommended_vector_width,
                max_safe_batch_size,
                preferred_algorithm,
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            Self {
                has_sse2: false,
                has_avx: false,
                has_avx2: false,
                has_fma: false,
                has_avx512f: false,
                recommended_vector_width: 4,
                max_safe_batch_size: 256,
                preferred_algorithm: SIMDAlgorithm::Scalar,
            }
        }
    }

    pub fn print_capabilities(&self) {
        println!("🔍 SIMD Capabilities Detection:");
        println!("  SSE2:          {}", self.has_sse2);
        println!("  AVX:           {}", self.has_avx);
        println!("  AVX2:          {}", self.has_avx2);
        println!("  FMA:           {}", self.has_fma);
        println!("  AVX-512F:      {}", self.has_avx512f);
        println!("  Vector Width:  {} bytes", self.recommended_vector_width);
        println!("  Max Batch:     {}", self.max_safe_batch_size);
        println!("  Algorithm:     {:?}", self.preferred_algorithm);
    }
}

/// Unified SIMD processor с автоматическим выбором реализации
pub struct UnifiedSIMDProcessor {
    config: SIMDConfig,
    capabilities: UnifiedSIMDCapabilities,
    stats: SIMDStats,
}

#[derive(Debug, Default)]
pub struct SIMDStats {
    pub scalar_operations: std::sync::atomic::AtomicU64,
    pub safe_vectorized_operations: std::sync::atomic::AtomicU64,
    pub unsafe_simd_operations: std::sync::atomic::AtomicU64,
    pub total_elements_processed: std::sync::atomic::AtomicU64,
    pub total_processing_time_ns: std::sync::atomic::AtomicU64,
}

impl SIMDStats {
    pub fn record_operation(
        &self,
        algorithm: SIMDAlgorithm,
        elements: usize,
        duration: std::time::Duration,
    ) {
        use std::sync::atomic::Ordering;

        match algorithm {
            SIMDAlgorithm::Scalar => self.scalar_operations.fetch_add(1, Ordering::Relaxed),
            SIMDAlgorithm::SafeVectorized => self
                .safe_vectorized_operations
                .fetch_add(1, Ordering::Relaxed),
            SIMDAlgorithm::UnsafeSIMD => {
                self.unsafe_simd_operations.fetch_add(1, Ordering::Relaxed)
            }
        };

        self.total_elements_processed
            .fetch_add(elements as u64, Ordering::Relaxed);
        self.total_processing_time_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn throughput_elements_per_second(&self) -> f64 {
        use std::sync::atomic::Ordering;

        let total_elements = self.total_elements_processed.load(Ordering::Relaxed);
        let total_time_s =
            self.total_processing_time_ns.load(Ordering::Relaxed) as f64 / 1_000_000_000.0;

        if total_time_s > 0.0 {
            total_elements as f64 / total_time_s
        } else {
            0.0
        }
    }
}

impl UnifiedSIMDProcessor {
    pub fn new(config: SIMDConfig) -> Self {
        let capabilities = UnifiedSIMDCapabilities::detect();

        if config.enable_feature_detection {
            capabilities.print_capabilities();
        }

        Self {
            config,
            capabilities,
            stats: SIMDStats::default(),
        }
    }

    /// Автоматический выбор лучшего алгоритма для данных
    pub fn select_algorithm(&self, data_size: usize) -> SIMDAlgorithm {
        // Безопасный выбор алгоритма на основе конфигурации и возможностей
        if data_size < self.config.min_simd_size {
            return SIMDAlgorithm::Scalar;
        }

        if self.config.allow_unsafe_simd && self.capabilities.has_avx2 && self.capabilities.has_fma
        {
            // Разрешены unsafe операции и есть поддержка AVX2+FMA
            SIMDAlgorithm::UnsafeSIMD
        } else if self.capabilities.preferred_algorithm == SIMDAlgorithm::SafeVectorized {
            // Используем безопасную векторизацию
            SIMDAlgorithm::SafeVectorized
        } else {
            // Fallback к скалярной обработке
            SIMDAlgorithm::Scalar
        }
    }

    /// Unified cosine distance с автоматическим выбором алгоритма
    pub fn cosine_distance(&self, a: &[f32], b: &[f32]) -> f32 {
        let start_time = Instant::now();
        let algorithm = self.select_algorithm(a.len() * std::mem::size_of::<f32>());

        let result = match algorithm {
            SIMDAlgorithm::Scalar => cosine_distance_scalar_safe(a, b),
            SIMDAlgorithm::SafeVectorized => {
                crate::simd_safe_replacement::cosine_distance_auto_safe(a, b)
            }
            SIMDAlgorithm::UnsafeSIMD => {
                if self.is_suitable_for_unsafe_simd(a, b) {
                    self.cosine_distance_unsafe_simd(a, b)
                } else {
                    // Fallback к безопасной версии если данные не подходят
                    crate::simd_safe_replacement::cosine_distance_auto_safe(a, b)
                }
            }
        };

        let duration = start_time.elapsed();
        self.stats.record_operation(algorithm, a.len(), duration);
        result
    }

    /// Batch cosine distance с оптимальной обработкой
    pub fn batch_cosine_distance(&self, queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
        let start_time = Instant::now();

        // Определяем алгоритм на основе размера batch
        let total_elements = queries.iter().map(|q| q.len()).sum::<usize>();
        let algorithm = self.select_algorithm(total_elements * std::mem::size_of::<f32>());

        let results = match algorithm {
            SIMDAlgorithm::Scalar => queries
                .iter()
                .map(|query| cosine_distance_scalar_safe(query, target))
                .collect(),
            SIMDAlgorithm::SafeVectorized => {
                crate::simd_safe_replacement::batch_cosine_distance_safe(queries, target)
            }
            SIMDAlgorithm::UnsafeSIMD => {
                // Проверяем каждый query на совместимость
                let all_suitable = queries
                    .iter()
                    .all(|q| self.is_suitable_for_unsafe_simd(q, target));

                if all_suitable {
                    self.batch_cosine_distance_unsafe_simd(queries, target)
                } else {
                    // Fallback к безопасной версии
                    crate::simd_safe_replacement::batch_cosine_distance_safe(queries, target)
                }
            }
        };

        let duration = start_time.elapsed();
        self.stats
            .record_operation(algorithm, total_elements, duration);
        results
    }

    /// Проверка пригодности данных для unsafe SIMD
    fn is_suitable_for_unsafe_simd(&self, a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.len() >= 8 && a.len() % 8 == 0 && !a.is_empty() && !b.is_empty()
    }

    /// Unsafe SIMD реализация (только когда безопасно)
    #[cfg(target_arch = "x86_64")]
    fn cosine_distance_unsafe_simd(&self, a: &[f32], b: &[f32]) -> f32 {
        if !self.is_suitable_for_unsafe_simd(a, b) {
            return crate::simd_safe_replacement::cosine_distance_auto_safe(a, b);
        }

        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            unsafe { self.cosine_distance_avx2_fma(a, b) }
        } else {
            crate::simd_safe_replacement::cosine_distance_auto_safe(a, b)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn cosine_distance_unsafe_simd(&self, a: &[f32], b: &[f32]) -> f32 {
        crate::simd_safe_replacement::cosine_distance_auto_safe(a, b)
    }

    /// Unsafe batch SIMD реализация
    fn batch_cosine_distance_unsafe_simd(&self, queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
        queries
            .iter()
            .map(|query| self.cosine_distance_unsafe_simd(query, target))
            .collect()
    }

    /// AVX2 + FMA реализация (только x86_64)
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2,fma")]
    unsafe fn cosine_distance_avx2_fma(&self, a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len());
        debug_assert!(a.len() % 8 == 0);
        debug_assert!(!a.is_empty());

        let len = a.len();
        let chunks = len / 8;

        let mut dot_acc = _mm256_setzero_ps();
        let mut norm_a_acc = _mm256_setzero_ps();
        let mut norm_b_acc = _mm256_setzero_ps();

        for i in 0..chunks {
            let idx = i * 8;
            let va = _mm256_loadu_ps(a.as_ptr().add(idx));
            let vb = _mm256_loadu_ps(b.as_ptr().add(idx));

            dot_acc = _mm256_fmadd_ps(va, vb, dot_acc);
            norm_a_acc = _mm256_fmadd_ps(va, va, norm_a_acc);
            norm_b_acc = _mm256_fmadd_ps(vb, vb, norm_b_acc);
        }

        let dot_sum = horizontal_sum_avx2(dot_acc);
        let norm_a_sum = horizontal_sum_avx2(norm_a_acc);
        let norm_b_sum = horizontal_sum_avx2(norm_b_acc);

        let norm_product = norm_a_sum * norm_b_sum;
        if norm_product < f32::EPSILON {
            return 0.0;
        }

        let similarity = dot_sum / norm_product.sqrt();
        1.0 - similarity.clamp(-1.0, 1.0)
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> &SIMDStats {
        &self.stats
    }

    /// Get capabilities
    pub fn get_capabilities(&self) -> &UnifiedSIMDCapabilities {
        &self.capabilities
    }
}

/// AVX2 horizontal sum helper
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_avx2(v: __m256) -> f32 {
    let sum_dual = _mm256_hadd_ps(v, v);
    let sum_quad = _mm256_hadd_ps(sum_dual, sum_dual);
    let lo = _mm256_castps256_ps128(sum_quad);
    let hi = _mm256_extractf128_ps(sum_quad, 1);
    let final_sum = _mm_add_ps(lo, hi);
    _mm_cvtss_f32(final_sum)
}

/// Безопасная скалярная реализация для fallback
pub fn cosine_distance_scalar_safe(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 1.0; // Максимальное расстояние для несовместимых векторов
    }

    if a.is_empty() {
        return 0.0;
    }

    let (dot_product, norm_a, norm_b) = a.iter().zip(b.iter()).fold(
        (0.0f32, 0.0f32, 0.0f32),
        |(dot, norm_a, norm_b), (&a_val, &b_val)| {
            (
                dot + a_val * b_val,
                norm_a + a_val * a_val,
                norm_b + b_val * b_val,
            )
        },
    );

    let norm_product = norm_a * norm_b;
    if norm_product < f32::EPSILON {
        return 0.0;
    }

    let similarity = dot_product / norm_product.sqrt();
    1.0 - similarity.clamp(-1.0, 1.0)
}

/// Глобальная инстанция unified SIMD processor
pub static GLOBAL_SIMD_PROCESSOR: std::sync::LazyLock<UnifiedSIMDProcessor> =
    std::sync::LazyLock::new(|| UnifiedSIMDProcessor::new(SIMDConfig::default()));

/// Convenience функции для прямого использования

/// Unified cosine distance - автоматически выбирает лучшую реализацию
pub fn cosine_distance_unified(a: &[f32], b: &[f32]) -> f32 {
    GLOBAL_SIMD_PROCESSOR.cosine_distance(a, b)
}

/// Unified batch cosine distance
pub fn batch_cosine_distance_unified(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    GLOBAL_SIMD_PROCESSOR.batch_cosine_distance(queries, target)
}

/// Benchmark функция для сравнения всех реализаций
pub fn benchmark_unified_simd(vector_size: usize, num_iterations: usize) -> BenchmarkResults {
    let a: Vec<f32> = (0..vector_size).map(|i| (i as f32).sin()).collect();
    let b: Vec<f32> = (0..vector_size).map(|i| (i as f32).cos()).collect();

    let mut results = BenchmarkResults::default();
    results.vector_size = vector_size;
    results.iterations = num_iterations;

    // Benchmark scalar
    let start = Instant::now();
    for _ in 0..num_iterations {
        let _result = cosine_distance_scalar_safe(&a, &b);
        std::hint::black_box(_result);
    }
    results.scalar_time = start.elapsed();

    // Benchmark safe vectorized
    let start = Instant::now();
    for _ in 0..num_iterations {
        let _result = crate::simd_safe_replacement::cosine_distance_auto_safe(&a, &b);
        std::hint::black_box(_result);
    }
    results.safe_vectorized_time = start.elapsed();

    // Benchmark unified (automatic selection)
    let start = Instant::now();
    for _ in 0..num_iterations {
        let _result = cosine_distance_unified(&a, &b);
        std::hint::black_box(_result);
    }
    results.unified_time = start.elapsed();

    results
}

#[derive(Debug, Default)]
pub struct BenchmarkResults {
    pub vector_size: usize,
    pub iterations: usize,
    pub scalar_time: std::time::Duration,
    pub safe_vectorized_time: std::time::Duration,
    pub unified_time: std::time::Duration,
}

impl BenchmarkResults {
    pub fn print_results(&self) {
        println!("🚀 Unified SIMD Benchmark Results");
        println!("================================");
        println!("Vector size: {}", self.vector_size);
        println!("Iterations: {}", self.iterations);

        let scalar_ns = self.scalar_time.as_nanos() as f64 / self.iterations as f64;
        let safe_ns = self.safe_vectorized_time.as_nanos() as f64 / self.iterations as f64;
        let unified_ns = self.unified_time.as_nanos() as f64 / self.iterations as f64;

        println!("Scalar:           {:.2} ns/op", scalar_ns);
        println!("Safe Vectorized:  {:.2} ns/op", safe_ns);
        println!("Unified Auto:     {:.2} ns/op", unified_ns);

        if unified_ns < scalar_ns {
            let speedup = scalar_ns / unified_ns;
            println!("🚀 Unified Speedup: {:.1}x vs scalar", speedup);
        }

        if safe_ns < scalar_ns {
            let speedup = scalar_ns / safe_ns;
            println!("⚡ Safe Speedup: {:.1}x vs scalar", speedup);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_capabilities_detection() {
        let capabilities = UnifiedSIMDCapabilities::detect();

        // На большинстве современных x86_64 должен быть хотя бы SSE2
        #[cfg(target_arch = "x86_64")]
        assert!(capabilities.has_sse2);

        assert!(capabilities.recommended_vector_width >= 4);
        assert!(capabilities.max_safe_batch_size > 0);
    }

    #[test]
    fn test_unified_processor_creation() {
        let config = SIMDConfig {
            enable_feature_detection: false, // Отключаем для тестов
            ..Default::default()
        };

        let processor = UnifiedSIMDProcessor::new(config);
        assert!(processor.capabilities.recommended_vector_width > 0);
    }

    #[test]
    fn test_algorithm_selection() {
        let config = SIMDConfig::default();
        let processor = UnifiedSIMDProcessor::new(config);

        // Маленькие данные -> scalar
        let small_algorithm = processor.select_algorithm(16);
        assert_eq!(small_algorithm, SIMDAlgorithm::Scalar);

        // Большие данные -> vectorized (если allow_unsafe_simd = false)
        let large_algorithm = processor.select_algorithm(1024);
        // По умолчанию должен быть safe или scalar
        assert!(
            large_algorithm == SIMDAlgorithm::SafeVectorized
                || large_algorithm == SIMDAlgorithm::Scalar
        );
    }

    #[test]
    fn test_cosine_distance_consistency() {
        let config = SIMDConfig {
            enable_feature_detection: false,
            ..Default::default()
        };
        let processor = UnifiedSIMDProcessor::new(config);

        let a = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let unified_result = processor.cosine_distance(&a, &b);
        let scalar_result = cosine_distance_scalar_safe(&a, &b);

        // Результаты должны быть близки (ортогональные векторы -> расстояние ~1.0)
        assert!((unified_result - 1.0).abs() < 0.001);
        assert!((scalar_result - 1.0).abs() < 0.001);
        assert!((unified_result - scalar_result).abs() < 0.001);
    }

    #[test]
    fn test_batch_processing() {
        let config = SIMDConfig {
            enable_feature_detection: false,
            ..Default::default()
        };
        let processor = UnifiedSIMDProcessor::new(config);

        let queries = vec![
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        ];
        let target = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let results = processor.batch_cosine_distance(&queries, &target);
        assert_eq!(results.len(), 2);
        assert!(results[0].abs() < 0.001); // Первый идентичен target
        assert!((results[1] - 1.0).abs() < 0.001); // Второй ортогональный
    }

    #[test]
    fn test_global_processor() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let b = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

        let distance = cosine_distance_unified(&a, &b);
        assert!(distance.abs() < 0.001); // Идентичные векторы

        let stats = GLOBAL_SIMD_PROCESSOR.get_stats();
        assert!(
            stats
                .total_elements_processed
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
    }

    #[test]
    fn test_edge_cases() {
        let processor = UnifiedSIMDProcessor::new(SIMDConfig::default());

        // Пустые векторы
        let empty_a: Vec<f32> = vec![];
        let empty_b: Vec<f32> = vec![];
        assert_eq!(processor.cosine_distance(&empty_a, &empty_b), 0.0);

        // Разные размеры
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(processor.cosine_distance(&a, &b), 1.0);

        // Нулевые векторы
        let zero_a = vec![0.0; 8];
        let zero_b = vec![0.0; 8];
        assert_eq!(processor.cosine_distance(&zero_a, &zero_b), 0.0);
    }
}
