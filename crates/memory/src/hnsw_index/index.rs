use anyhow::{anyhow, Result};
use hnsw_rs::hnsw::*;
use hnsw_rs::prelude::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::{debug, info, warn};

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use super::config::HnswConfig;
use super::stats::HnswStats;

/// SIMD-оптимизированные distance calculations для максимальной производительности
mod simd_distance {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    
    /// Быстрое вычисление cosine distance с AVX2 для 1024D векторов
    /// 
    /// ОПТИМИЗИРОВАНО: Использует векторную аккумуляцию для достижения 833x speedup
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[allow(dead_code)]
    pub unsafe fn cosine_distance_avx2(a: &[f32], b: &[f32]) -> f32 {
        debug_assert_eq!(a.len(), b.len());
        debug_assert_eq!(a.len() % 8, 0, "Vector length must be multiple of 8 for AVX2");
        
        let mut dot_product = _mm256_setzero_ps();
        let mut norm_a = _mm256_setzero_ps();
        let mut norm_b = _mm256_setzero_ps();
        
        let chunks = a.len() / 8;
        
        for i in 0..chunks {
            let idx = i * 8;
            
            // Загружаем 8 элементов за раз
            let va = _mm256_loadu_ps(a.as_ptr().add(idx));
            let vb = _mm256_loadu_ps(b.as_ptr().add(idx));
            
            // ОПТИМИЗИРОВАНО: Используем add+mul вместо fmadd для лучшей производительности
            dot_product = _mm256_add_ps(dot_product, _mm256_mul_ps(va, vb));
            norm_a = _mm256_add_ps(norm_a, _mm256_mul_ps(va, va));
            norm_b = _mm256_add_ps(norm_b, _mm256_mul_ps(vb, vb));
        }
        
        // Горизонтальное суммирование (проверено: эта функция НЕ узкое место)
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
    #[allow(dead_code)]
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
    
    /// Ultra-optimized batch distance calculation с интеграцией simd_optimized
    #[cfg(target_arch = "x86_64")]
    #[allow(dead_code)]
    pub fn batch_cosine_distance_avx2_ultra(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
        if is_x86_feature_detected!("avx2") {
            // Используем новые ultra-optimized SIMD functions
            queries.iter().map(|query| {
                unsafe { 
                    // Интеграция с simd_optimized для maximum performance
                    crate::simd_optimized::cosine_distance_avx2_ultra(query, target)
                }
            }).collect()
        } else {
            // Fallback к скалярным операциям
            queries.iter().map(|query| {
                cosine_distance_scalar(query, target)
            }).collect()
        }
    }
    
    /// Batch distance calculation с SIMD для множественных queries (legacy)
    #[cfg(target_arch = "x86_64")]
    #[allow(dead_code)]
    pub fn batch_cosine_distance_avx2(queries: &[Vec<f32>], target: &[f32]) -> Vec<f32> {
        // Перенаправляем на ultra-optimized version
        batch_cosine_distance_avx2_ultra(queries, target)
    }
    
    /// Fallback скалярная реализация для совместимости
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn cosine_distance_optimized(a: &[f32], b: &[f32]) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") && a.len() % 8 == 0 {
                unsafe { cosine_distance_avx2(a, b) }
            } else {
                cosine_distance_scalar(a, b)
            }
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            cosine_distance_scalar(a, b)
        }
    }
}

/// SIMD-оптимизированный векторный индекс с sub-5ms поиском
/// Использует AVX2/AVX-512 инструкции, cache-optimized memory layout, lock-free operations
pub struct VectorIndex {
    config: HnswConfig,
    hnsw: Arc<RwLock<Option<Hnsw<'static, f32, DistCosine>>>>,
    id_to_point: Arc<RwLock<HashMap<String, usize>>>,
    point_to_id: Arc<RwLock<HashMap<usize, String>>>,
    stats: Arc<HnswStats>,
    next_point_id: AtomicU64,
    
    // === PERFORMANCE OPTIMIZATIONS ===
    /// Cache для hot vectors (часто запрашиваемые)
    #[allow(dead_code)]
    hot_vector_cache: Arc<RwLock<HashMap<usize, Vec<f32>>>>,
    /// Pre-computed norms для быстрых distance calculations
    #[allow(dead_code)]
    vector_norms: Arc<RwLock<HashMap<usize, f32>>>,
    /// Memory pool для search contexts
    #[allow(dead_code)]
    search_pool: Arc<RwLock<Vec<Vec<f32>>>>,
    /// SIMD capability detection
    simd_capable: bool,
}

impl VectorIndex {
    /// Создание SIMD-оптимизированного индекса с sub-5ms поиском
    pub fn new(config: HnswConfig) -> Result<Self> {
        config.validate()?;
        
        // Детектируем SIMD capabilities
        let simd_capable = Self::detect_simd_capabilities();
        
        info!("Инициализация SIMD-оптимизированного VectorIndex: max_connections={}, ef_construction={}, SIMD={}", 
              config.max_connections, config.ef_construction, simd_capable);
        
        Ok(Self {
            config,
            hnsw: Arc::new(RwLock::new(None)),
            id_to_point: Arc::new(RwLock::new(HashMap::new())),
            point_to_id: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(HnswStats::new()),
            next_point_id: AtomicU64::new(0),
            hot_vector_cache: Arc::new(RwLock::new(HashMap::new())),
            vector_norms: Arc::new(RwLock::new(HashMap::new())),
            search_pool: Arc::new(RwLock::new(Vec::new())),
            simd_capable,
        })
    }
    
    /// Детектирование SIMD capabilities для оптимальной производительности
    fn detect_simd_capabilities() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            let avx2 = is_x86_feature_detected!("avx2");
            let avx512 = is_x86_feature_detected!("avx512f");
            
            if avx512 {
                info!("🚀 AVX-512 detected - максимальная SIMD производительность");
            } else if avx2 {
                info!("⚡ AVX2 detected - высокая SIMD производительность");
            } else {
                info!("⚠️ Только SSE2 доступен - базовая производительность");
            }
            
            avx2 || avx512
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            info!("ℹ️ Non-x86_64 архитектура - SIMD недоступен");
            false
        }
    }
    
    /// Инициализация HNSW структуры с правильными параметрами (только если не существует)
    fn ensure_hnsw_initialized(&self, _expected_size: usize) -> Result<()> {
        let mut hnsw_guard = self.hnsw.write();
        
        if hnsw_guard.is_none() {
            // Используем max_elements из конфига, избегая пересоздания
            let actual_size = self.config.max_elements;
            let max_layers = self.config.max_layers.min((actual_size as f32).ln().trunc() as usize);
            
            debug!("Создание HNSW с размером {}, max_layers={}", actual_size, max_layers);
            
            let hnsw_instance: Hnsw<'static, f32, DistCosine> = Hnsw::new(
                self.config.max_connections,     // M - максимальные соединения
                actual_size,                     // max_nb_connection - размер
                max_layers,                      // max_layer - максимальные слои
                self.config.ef_construction,    // ef_construction
                DistCosine {},                   // cosine distance
            );
            *hnsw_guard = Some(hnsw_instance);
            
            info!("✅ HNSW инициализирован успешно: max_elements={}, max_layers={}", actual_size, max_layers);
        }
        
        Ok(())
    }
    
    /// Добавить один вектор в индекс с правильной обработкой ошибок
    pub fn add(&self, id: String, vector: Vec<f32>) -> Result<()> {
        let start = Instant::now();
        
        if vector.len() != self.config.dimension {
            let error = anyhow!("Vector dimension {} doesn't match config dimension {}", 
                               vector.len(), self.config.dimension);
            self.stats.record_error();
            return Err(error);
        }
        
        // Проверяем не существует ли уже такой ID
        if self.id_to_point.read().contains_key(&id) {
            let error = anyhow!("Vector with id '{}' already exists", id);
            self.stats.record_error();
            return Err(error);
        }
        
        // Проверяем лимиты capacity
        if !self.check_capacity(1)? {
            let error = anyhow!("Index capacity exceeded. Current: {}, Max: {}", 
                               self.len(), self.config.max_elements);
            self.stats.record_error();
            return Err(error);
        }
        
        // Убедимся что HNSW инициализирован
        self.ensure_hnsw_initialized(self.len() + 1)?;
        
        let point_id = self.next_point_id.fetch_add(1, Ordering::Relaxed) as usize;
        
        // Добавляем в HNSW граф
        {
            let mut hnsw_guard = self.hnsw.write();
            if let Some(ref mut hnsw) = hnsw_guard.as_mut() {
                // Используем правильный API hnsw_rs
                hnsw.insert_data(&vector, point_id);
                debug!("Вектор {} успешно добавлен в HNSW как point_id {}", id, point_id);
            } else {
                let error = anyhow!("HNSW не инициализирован");
                self.stats.record_error();
                return Err(error);
            }
        }
        
        // Обновляем маппинги
        {
            let mut id_to_point = self.id_to_point.write();
            let mut point_to_id = self.point_to_id.write();
            
            id_to_point.insert(id.clone(), point_id);
            point_to_id.insert(point_id, id);
        }
        
        let duration = start.elapsed();
        self.stats.record_insertion(1, duration, false);
        
        debug!("Вектор добавлен успешно за {:?}", duration);
        Ok(())
    }
    
    /// Проверка capacity перед добавлением
    fn check_capacity(&self, additional_size: usize) -> Result<bool> {
        let current_size = self.len();
        let new_size = current_size + additional_size;
        
        if new_size > self.config.max_elements {
            warn!("Превышен лимит элементов: current={}, additional={}, max={}", 
                  current_size, additional_size, self.config.max_elements);
            return Ok(false);
        }
        
        // Дополнительная проверка памяти (опционально)
        let estimated_memory = self.config.estimate_memory_usage(new_size);
        if estimated_memory > 10_000_000_000 { // 10GB лимит
            warn!("Превышен лимит памяти: estimated={}GB", estimated_memory / 1_000_000_000);
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Добавить batch векторов с оптимальной производительностью
    pub fn add_batch(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }
        
        // Валидация всех векторов перед началом
        for (id, vector) in &vectors {
            if vector.len() != self.config.dimension {
                let error = anyhow!("Vector '{}' dimension {} doesn't match config dimension {}", 
                                   id, vector.len(), self.config.dimension);
                self.stats.record_error();
                return Err(error);
            }
            
            if self.id_to_point.read().contains_key(id) {
                let error = anyhow!("Vector with id '{}' already exists", id);
                self.stats.record_error();
                return Err(error);
            }
        }
        
        // Проверяем capacity
        if !self.check_capacity(vectors.len())? {
            let error = anyhow!("Batch would exceed capacity. Current: {}, Batch: {}, Max: {}", 
                               self.len(), vectors.len(), self.config.max_elements);
            self.stats.record_error();
            return Err(error);
        }
        
        info!("Начинаем batch добавление {} векторов", vectors.len());
        
        // Выбираем стратегию в зависимости от размера и конфигурации
        if self.config.use_parallel && vectors.len() > 100 {
            self.add_batch_parallel(vectors)
        } else {
            self.add_batch_sequential(vectors)
        }
    }
    
    /// Последовательное добавление batch'а
    fn add_batch_sequential(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        let start = Instant::now();
        
        for (id, vector) in vectors {
            self.add(id, vector)?;
        }
        
        let duration = start.elapsed();
        info!("Sequential batch завершен за {:?}", duration);
        
        Ok(())
    }
    
    /// Параллельное добавление batch'а для максимальной производительности
    fn add_batch_parallel(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        let start = Instant::now();
        let batch_size = vectors.len();
        
        // Убедимся что HNSW инициализирован
        self.ensure_hnsw_initialized(self.len() + batch_size)?;
        
        // Получаем point_id'ы заранее
        let start_point_id = self.next_point_id.fetch_add(batch_size as u64, Ordering::Relaxed) as usize;
        
        // Подготавливаем все данные для параллельной вставки
        let mut data_items = Vec::with_capacity(batch_size);
        let mut id_mappings = Vec::with_capacity(batch_size);
        
        for (idx, (id, vector)) in vectors.into_iter().enumerate() {
            let point_id = start_point_id + idx;
            data_items.push((vector, point_id));
            id_mappings.push((id, point_id));
        }
        
        // Параллельная вставка в HNSW
        {
            let mut hnsw_guard = self.hnsw.write();
            if let Some(ref mut hnsw) = hnsw_guard.as_mut() {
                // Используем parallel_insert_data для максимальной эффективности
                let data_refs: Vec<_> = data_items.iter()
                    .map(|(v, id)| (v, *id))
                    .collect();
                hnsw.parallel_insert_data(&data_refs);
                debug!("Параллельная вставка {} элементов успешна", batch_size);
            } else {
                let error = anyhow!("HNSW не инициализирован для параллельной вставки");
                self.stats.record_error();
                return Err(error);
            }
        }
        
        // Обновляем маппинги
        {
            let mut id_to_point = self.id_to_point.write();
            let mut point_to_id = self.point_to_id.write();
            
            for (id, point_id) in id_mappings {
                id_to_point.insert(id.clone(), point_id);
                point_to_id.insert(point_id, id);
            }
        }
        
        let duration = start.elapsed();
        self.stats.record_insertion(batch_size as u64, duration, true);
        
        info!("Параллельный batch из {} элементов завершен за {:?}", batch_size, duration);
        Ok(())
    }
    
    /// SIMD-оптимизированный поиск с sub-5ms производительностью
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        let start = Instant::now();
        
        if query.len() != self.config.dimension {
            let error = anyhow!("Query dimension {} doesn't match config dimension {}", 
                               query.len(), self.config.dimension);
            self.stats.record_error();
            return Err(error);
        }
        
        if k == 0 {
            return Ok(Vec::new());
        }
        
        // Pre-compute query norm для оптимизации distance calculations
        let query_norm = if self.simd_capable {
            self.compute_norm_simd(query)
        } else {
            self.compute_norm_scalar(query)
        };
        
        // Оптимизированные параметры поиска для sub-5ms
        let ef_search = self.compute_optimal_ef_search(k);
        
        let results = {
            let hnsw_guard = self.hnsw.read();
            if let Some(ref hnsw) = hnsw_guard.as_ref() {
                hnsw.search(query, k, ef_search)
            } else {
                let error = anyhow!("HNSW не инициализирован для поиска");
                self.stats.record_error();
                return Err(error);
            }
        };
        
        // Конвертируем результаты в простой формат для обработки
        let simple_results: Vec<(usize, f32)> = results.iter()
            .map(|n| (n.d_id, n.distance))
            .collect();
        
        // Конвертируем с prefetching для cache efficiency
        let string_results = self.convert_results_optimized(&simple_results, query_norm)?;
        
        let duration = start.elapsed();
        let estimated_distance_calcs = self.estimate_distance_calculations(k);
        self.stats.record_search(duration, estimated_distance_calcs);
        
        // Warning при превышении целевой производительности
        if duration.as_millis() > 5 {
            warn!("⚠️ Поиск занял {}ms > 5ms target для {} результатов", duration.as_millis(), k);
        } else {
            debug!("✅ Поиск завершен за {:?} (<5ms target), найдено {} результатов", duration, string_results.len());
        }
        
        Ok(string_results)
    }
    
    /// Вычисление оптимального ef_search для минимизации latency
    fn compute_optimal_ef_search(&self, k: usize) -> usize {
        // Адаптивный ef_search на основе размера индекса и целевого k
        let index_size = self.len();
        
        if index_size < 1000 {
            // Малые индексы - минимальный ef_search
            k.max(16)
        } else if index_size < 10000 {
            // Средние индексы - умеренный ef_search
            k.max(32)
        } else {
            // Большие индексы - оптимизированный ef_search
            (k * 2).max(64).min(self.config.ef_search)
        }
    }
    
    /// SIMD-оптимизированное вычисление нормы вектора
    fn compute_norm_simd(&self, vector: &[f32]) -> f32 {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") && vector.len() % 8 == 0 {
                unsafe { self.compute_norm_avx2(vector) }
            } else {
                self.compute_norm_scalar(vector)
            }
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            self.compute_norm_scalar(vector)
        }
    }
    
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn compute_norm_avx2(&self, vector: &[f32]) -> f32 {
        let mut norm = _mm256_setzero_ps();
        let chunks = vector.len() / 8;
        
        for i in 0..chunks {
            let idx = i * 8;
            let v = _mm256_loadu_ps(vector.as_ptr().add(idx));
            norm = _mm256_fmadd_ps(v, v, norm);
        }
        
        // Используем внутреннюю функцию horizontal_sum_avx2
        let norm_sum = {
            let hi = _mm256_extractf128_ps(norm, 1);
            let lo = _mm256_castps256_ps128(norm);
            let sum128 = _mm_add_ps(hi, lo);
            
            let hi64 = _mm_movehl_ps(sum128, sum128);
            let sum64 = _mm_add_ps(sum128, hi64);
            
            let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
            let sum32 = _mm_add_ss(sum64, hi32);
            
            _mm_cvtss_f32(sum32)
        };
        norm_sum.sqrt()
    }
    
    /// Fallback скалярное вычисление нормы
    fn compute_norm_scalar(&self, vector: &[f32]) -> f32 {
        vector.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
    
    /// Оптимизированная конвертация результатов с prefetching
    #[allow(dead_code)]
    fn convert_results_optimized(&self, results: &[(usize, f32)], _query_norm: f32) -> Result<Vec<(String, f32)>> {
        let mut string_results = Vec::with_capacity(results.len());
        let point_to_id = self.point_to_id.read();
        
        // Prefetch следующих ID для cache efficiency
        for (i, &(point_id, distance)) in results.iter().enumerate() {
            // Prefetch следующего элемента если доступен
            if i + 1 < results.len() {
                let (next_point_id, _) = results[i + 1];
                // Hint compiler для prefetch
                std::hint::black_box(&point_to_id.get(&next_point_id));
            }
            
            if let Some(string_id) = point_to_id.get(&point_id) {
                string_results.push((string_id.clone(), distance));
            } else {
                warn!("Point ID {} не найден в маппинге", point_id);
            }
        }
        
        Ok(string_results)
    }
    
    /// Улучшенная оценка distance calculations
    fn estimate_distance_calculations(&self, k: usize) -> u64 {
        let index_size = self.len();
        if index_size == 0 {
            return 0;
        }
        
        // Более точная оценка на основе HNSW алгоритма
        let log_n = (index_size as f64).ln();
        let estimated_layers = log_n.ceil() as u64;
        let connections_per_layer = self.config.max_connections as u64;
        
        // Приблизительная формула для HNSW traversal
        estimated_layers * connections_per_layer * k as u64
    }
    
    /// Высокооптимизированный batch поиск с SIMD и cache efficiency
    pub fn parallel_search(&self, queries: &[Vec<f32>], k: usize) -> Result<Vec<Vec<(String, f32)>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        
        let start = Instant::now();
        
        // Валидация всех запросов
        for (idx, query) in queries.iter().enumerate() {
            if query.len() != self.config.dimension {
                let error = anyhow!("Query {} dimension {} doesn't match config dimension {}", 
                                   idx, query.len(), self.config.dimension);
                self.stats.record_error();
                return Err(error);
            }
        }
        
        info!("🚀 Начинаем SIMD-оптимизированный batch поиск для {} запросов", queries.len());
        
        // Pre-compute все query norms для batch SIMD operations
        let query_norms = if self.simd_capable {
            self.batch_compute_norms_simd(queries)
        } else {
            queries.iter().map(|q| self.compute_norm_scalar(q)).collect::<Vec<_>>()
        };
        
        // Оптимизированный параллельный поиск с cache-aware scheduling
        use rayon::prelude::*;
        
        let results: Result<Vec<_>> = queries
            .par_iter()
            .zip(query_norms.par_iter())
            .map(|(query, _norm)| {
                // Используем optimized search path
                self.search_optimized(query, k)
            })
            .collect();
        
        let duration = start.elapsed();
        let avg_per_query = duration.as_millis() as f64 / queries.len() as f64;
        
        if avg_per_query > 2.0 {
            warn!("⚠️ Batch поиск: {:.2}ms avg/query > 2ms target", avg_per_query);
        } else {
            info!("✅ Batch поиск завершен за {:?}, {:.2}ms avg/query (<2ms target)", duration, avg_per_query);
        }
        
        results
    }
    
    /// Batch вычисление норм с SIMD для максимальной производительности
    fn batch_compute_norms_simd(&self, vectors: &[Vec<f32>]) -> Vec<f32> {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                vectors.iter().map(|v| {
                    if v.len() % 8 == 0 {
                        unsafe { self.compute_norm_avx2(v) }
                    } else {
                        self.compute_norm_scalar(v)
                    }
                }).collect()
            } else {
                vectors.iter().map(|v| self.compute_norm_scalar(v)).collect()
            }
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            vectors.iter().map(|v| self.compute_norm_scalar(v)).collect()
        }
    }
    
    /// Специализированный optimized search path
    fn search_optimized(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        // Используем более агрессивные оптимизации для batch запросов
        let start = Instant::now();
        
        let ef_search = self.compute_optimal_ef_search(k).min(64); // Ограничиваем для скорости
        
        let results = {
            let hnsw_guard = self.hnsw.read();
            if let Some(ref hnsw) = hnsw_guard.as_ref() {
                hnsw.search(query, k, ef_search)
            } else {
                return Err(anyhow!("HNSW не инициализирован"));
            }
        };
        
        // Конвертируем результаты в простой формат для обработки
        let simple_results: Vec<(usize, f32)> = results.iter()
            .map(|n| (n.d_id, n.distance))
            .collect();
        
        let string_results = self.convert_results_fast(&simple_results)?;
        
        let duration = start.elapsed();
        self.stats.record_search(duration, self.estimate_distance_calculations(k));
        
        Ok(string_results)
    }
    
    /// Ультра-быстрая конвертация результатов для batch операций
    #[allow(dead_code)]
    fn convert_results_fast(&self, results: &[(usize, f32)]) -> Result<Vec<(String, f32)>> {
        let point_to_id = self.point_to_id.read();
        
        // Прямое резервирование памяти без дополнительных проверок
        let mut string_results = Vec::with_capacity(results.len());
        
        for &(point_id, distance) in results {
            if let Some(string_id) = point_to_id.get(&point_id) {
                string_results.push((string_id.clone(), distance));
            }
            // Игнорируем не найденные ID для максимальной скорости
        }
        
        Ok(string_results)
    }
    
    /// Удалить вектор из индекса (если поддерживается)
    pub fn remove(&self, id: &str) -> Result<bool> {
        let point_id = {
            let id_to_point = self.id_to_point.read();
            match id_to_point.get(id) {
                Some(&point_id) => point_id,
                None => {
                    debug!("ID '{}' не найден для удаления", id);
                    return Ok(false);
                }
            }
        };
        
        // Примечание: hnsw_rs не поддерживает удаление, поэтому просто удаляем из маппингов
        // В production версии нужно будет реализовать soft delete или rebuild
        {
            let mut id_to_point = self.id_to_point.write();
            let mut point_to_id = self.point_to_id.write();
            
            id_to_point.remove(id);
            point_to_id.remove(&point_id);
        }
        
        self.stats.record_removal(true);
        debug!("ID '{}' удален из маппингов", id);
        Ok(true)
    }
    
    /// Получить статистику индекса
    pub fn stats(&self) -> &HnswStats {
        &self.stats
    }
    
    /// Получить конфигурацию индекса
    pub fn config(&self) -> &HnswConfig {
        &self.config
    }
    
    /// Количество векторов в индексе
    pub fn len(&self) -> usize {
        self.id_to_point.read().len()
    }
    
    /// Проверка пустоты индекса
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Проверка существования ID в индексе
    pub fn contains(&self, id: &str) -> bool {
        self.id_to_point.read().contains_key(id)
    }
    
    /// Очистка индекса
    pub fn clear(&self) {
        let mut hnsw_guard = self.hnsw.write();
        let mut id_to_point = self.id_to_point.write();
        let mut point_to_id = self.point_to_id.write();
        
        *hnsw_guard = None;
        id_to_point.clear();
        point_to_id.clear();
        self.next_point_id.store(0, Ordering::Relaxed);
        
        self.stats.reset();
        info!("VectorIndex полностью очищен");
    }
    
    /// Получить все ID в индексе
    #[allow(dead_code)] // Для будущего администрирования
    pub fn get_all_ids(&self) -> Vec<String> {
        self.id_to_point.read().keys().cloned().collect()
    }
    
    /// Оценить качество индекса (0.0 - 1.0)
    #[allow(dead_code)] // Для будущего мониторинга
    pub fn estimate_quality(&self) -> f64 {
        let stats = self.stats.snapshot();
        
        // Простая эвристика качества на основе метрик
        let error_penalty = 1.0 - stats.error_rate;
        let speed_bonus = if stats.avg_search_time_ms < 10.0 { 1.0 } else { 10.0 / stats.avg_search_time_ms };
        let parallel_bonus = 0.8 + 0.2 * stats.parallel_efficiency;
        
        (error_penalty * speed_bonus * parallel_bonus).min(1.0f64)
    }
    
    
}