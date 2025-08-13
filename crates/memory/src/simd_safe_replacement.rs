//! БЕЗОПАСНАЯ ЗАМЕНА для simd_ultra_optimized.rs
//!
//! КРИТИЧЕСКОЕ ИСПРАВЛЕНИЕ: Замена небезопасных SIMD операций на безопасные эквиваленты
//! Удалены все unsafe блоки с потенциальными segfault рисками
//! Сохранена функциональность с акцентом на безопасность

use std::time::Instant;

/// БЕЗОПАСНАЯ замена для AlignedVector без unsafe операций
#[derive(Debug, Clone)]
pub struct SafeAlignedVector {
    data: Vec<f32>,
}

impl SafeAlignedVector {
    /// Создание выравненного вектора с безопасной проверкой длины
    pub fn new(mut data: Vec<f32>) -> Self {
        // Безопасное выравнивание до кратности 8 без unsafe операций
        while data.len() % 8 != 0 {
            data.push(0.0);
        }

        Self { data }
    }

    /// Получить данные как безопасный slice
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// БЕЗОПАСНАЯ проверка выравнивания без unsafe операций
    pub fn is_properly_sized(&self) -> bool {
        !self.data.is_empty() && self.data.len() % 8 == 0
    }

    /// Безопасная проверка для AVX2 совместимости
    pub fn is_avx2_compatible(&self) -> bool {
        self.data.len() >= 8 && self.data.len() % 8 == 0
    }
}

/// ИСПРАВЛЕНО: Безопасное вычисление cosine distance без небезопасных SIMD операций
/// Использует Rust стандартные операции, которые компилятор может автовекторизовать
pub fn cosine_distance_safe(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 1.0; // Максимальное расстояние для несовместимых векторов
    }

    if a.is_empty() {
        return 0.0;
    }

    // Безопасные векторизованные операции через iterator
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

    // Безопасная проверка на деление на ноль
    let norm_product = norm_a * norm_b;
    if norm_product < f32::EPSILON {
        return 0.0;
    }

    let similarity = dot_product / norm_product.sqrt();

    // Безопасное clamp для numerical stability
    1.0 - similarity.clamp(-1.0, 1.0)
}

/// ИСПРАВЛЕНО: Безопасная batch обработка cosine distance
pub fn batch_cosine_distance_safe(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    queries
        .iter()
        .map(|query| cosine_distance_safe(query, target))
        .collect()
}

/// БЕЗОПАСНАЯ замена для автовыбора алгоритма расчета distance
pub fn cosine_distance_auto_safe(a: &[f32], b: &[f32]) -> f32 {
    // Простой безопасный выбор на основе размера данных
    if a.len() >= 1024 {
        // Для больших векторов используем chunked обработку для лучшей cache locality
        cosine_distance_chunked_safe(a, b)
    } else {
        // Для малых векторов используем простой алгоритм
        cosine_distance_safe(a, b)
    }
}

/// БЕЗОПАСНАЯ chunked обработка для больших векторов
pub fn cosine_distance_chunked_safe(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return if a.is_empty() { 0.0 } else { 1.0 };
    }

    const CHUNK_SIZE: usize = 256; // Оптимальный размер для cache locality

    let mut total_dot = 0.0f32;
    let mut total_norm_a = 0.0f32;
    let mut total_norm_b = 0.0f32;

    // Безопасная chunked обработка
    for (chunk_a, chunk_b) in a.chunks(CHUNK_SIZE).zip(b.chunks(CHUNK_SIZE)) {
        let (dot, norm_a, norm_b) = chunk_a.iter().zip(chunk_b.iter()).fold(
            (0.0f32, 0.0f32, 0.0f32),
            |(dot, norm_a, norm_b), (&a_val, &b_val)| {
                (
                    dot + a_val * b_val,
                    norm_a + a_val * a_val,
                    norm_b + b_val * b_val,
                )
            },
        );

        total_dot += dot;
        total_norm_a += norm_a;
        total_norm_b += norm_b;
    }

    let norm_product = total_norm_a * total_norm_b;
    if norm_product < f32::EPSILON {
        return 0.0;
    }

    let similarity = total_dot / norm_product.sqrt();
    1.0 - similarity.clamp(-1.0, 1.0)
}

/// БЕЗОПАСНАЯ batch обработка (последовательная реализация для избежания зависимости от rayon)
pub fn parallel_batch_cosine_distance_safe(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
    // Используем последовательную обработку для избежания зависимости от rayon
    queries
        .iter()
        .map(|query| cosine_distance_auto_safe(query, target))
        .collect()
}

/// БЕЗОПАСНАЯ проверка SIMD совместимости системы (без небезопасных операций)
pub fn check_simd_compatibility() -> SIMDCompatibility {
    SIMDCompatibility {
        has_sse2: true, // Доступно на всех x86_64
        has_avx2: is_x86_feature_detected!("avx2"),
        has_avx512: is_x86_feature_detected!("avx512f"),
        recommended_chunk_size: if is_x86_feature_detected!("avx2") {
            8
        } else {
            4
        },
    }
}

#[derive(Debug, Clone)]
pub struct SIMDCompatibility {
    pub has_sse2: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub recommended_chunk_size: usize,
}

/// Benchmark функция для тестирования производительности безопасных алгоритмов
pub fn benchmark_safe_algorithms(vector_size: usize, num_iterations: usize) -> BenchmarkResult {
    let a: Vec<f32> = (0..vector_size)
        .map(|i| (i as f32) / vector_size as f32)
        .collect();
    let b: Vec<f32> = (0..vector_size)
        .map(|i| ((i + 1) as f32) / vector_size as f32)
        .collect();

    let start = Instant::now();

    for _ in 0..num_iterations {
        let _result = cosine_distance_safe(&a, &b);
        std::hint::black_box(_result); // Prevent optimization
    }

    let duration = start.elapsed();
    let avg_duration = duration / num_iterations as u32;

    BenchmarkResult {
        vector_size,
        iterations: num_iterations,
        total_duration: duration,
        avg_duration,
        operations_per_second: 1_000_000_000.0 / avg_duration.as_nanos() as f64,
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub vector_size: usize,
    pub iterations: usize,
    pub total_duration: std::time::Duration,
    pub avg_duration: std::time::Duration,
    pub operations_per_second: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_cosine_distance_basic() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let distance = cosine_distance_safe(&a, &b);
        assert!((distance - 1.0).abs() < 1e-6); // Ортогональные векторы
    }

    #[test]
    fn test_safe_cosine_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let distance = cosine_distance_safe(&a, &b);
        assert!(distance.abs() < 1e-6); // Идентичные векторы
    }

    #[test]
    fn test_safe_aligned_vector() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5 элементов
        let aligned = SafeAlignedVector::new(data);
        assert_eq!(aligned.as_slice().len(), 8); // Выравнен до 8
        assert!(aligned.is_properly_sized());
    }

    #[test]
    fn test_batch_processing() {
        let queries = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];
        let target = vec![1.0, 0.0];

        let results = batch_cosine_distance_safe(&queries, &target);
        assert_eq!(results.len(), 3);
        assert!(results[0].abs() < 1e-6); // Первый идентичен target
    }

    #[test]
    fn test_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let distance = cosine_distance_safe(&a, &b);
        assert_eq!(distance, 0.0);
    }

    #[test]
    fn test_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let distance = cosine_distance_safe(&a, &b);
        assert_eq!(distance, 1.0); // Максимальное расстояние
    }
}
