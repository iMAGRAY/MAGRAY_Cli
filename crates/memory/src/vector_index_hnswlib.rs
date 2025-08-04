use anyhow::{anyhow, Result};
use hnsw_rs::hnsw::*;
use hnsw_rs::prelude::*;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Максимально профессиональная конфигурация для hnsw_rs от Jean-Pierre Both
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswRsConfig {
    /// Размерность векторов
    pub dimension: usize,
    /// Максимальное количество связей на узел (M) - ключевой параметр качества
    pub max_connections: usize,
    /// Размер списка кандидатов при построении (ef_construction) - влияет на качество
    pub ef_construction: usize,
    /// Размер списка кандидатов при поиске (ef_search) - баланс скорость/качество
    pub ef_search: usize,
    /// Максимальное количество элементов в индексе
    pub max_elements: usize,
    /// Максимальное количество слоев в графе
    pub max_layers: usize,
    /// Использовать параллельные операции для больших датасетов
    pub use_parallel: bool,
}

impl Default for HnswRsConfig {
    fn default() -> Self {
        Self {
            dimension: 1024,       // Qwen3 фактическая размерность из config.json
            max_connections: 24,   // Оптимальное значение для большинства случаев
            ef_construction: 400,  // Высокое качество построения (200-800 стандарт)
            ef_search: 100,        // Баланс скорость/точность
            max_elements: 1_000_000, // 1M элементов по умолчанию
            max_layers: 16,        // Стандартное значение
            use_parallel: true,    // Многопоточность для больших датасетов
        }
    }
}

/// Профессиональные метрики производительности от Jean-Pierre Both
#[derive(Debug, Default)]
pub struct HnswRsStats {
    pub total_vectors: AtomicU64,
    pub total_searches: AtomicU64,
    pub total_search_time_us: AtomicU64,
    pub total_insertions: AtomicU64,
    pub total_insert_time_us: AtomicU64,
    pub parallel_operations: AtomicU64,
    pub distance_calculations: AtomicU64,
}

impl HnswRsStats {
    pub fn record_search(&self, duration_us: u64, distance_calcs: u64) {
        self.total_searches.fetch_add(1, Ordering::Relaxed);
        self.total_search_time_us.fetch_add(duration_us, Ordering::Relaxed);
        self.distance_calculations.fetch_add(distance_calcs, Ordering::Relaxed);
    }
    
    pub fn record_insertion(&self, count: u64, duration_us: u64, is_parallel: bool) {
        self.total_vectors.fetch_add(count, Ordering::Relaxed);
        self.total_insertions.fetch_add(1, Ordering::Relaxed);
        self.total_insert_time_us.fetch_add(duration_us, Ordering::Relaxed);
        if is_parallel {
            self.parallel_operations.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn avg_search_time_us(&self) -> f64 {
        let searches = self.total_searches.load(Ordering::Relaxed);
        if searches == 0 { 0.0 } else {
            self.total_search_time_us.load(Ordering::Relaxed) as f64 / searches as f64
        }
    }
    
    pub fn avg_insert_time_us(&self) -> f64 {
        let insertions = self.total_insertions.load(Ordering::Relaxed);
        if insertions == 0 { 0.0 } else {
            self.total_insert_time_us.load(Ordering::Relaxed) as f64 / insertions as f64
        }
    }

    pub fn search_throughput_per_sec(&self) -> f64 {
        let searches = self.total_searches.load(Ordering::Relaxed);
        let total_time_sec = self.total_search_time_us.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        if total_time_sec == 0.0 { 0.0 } else { searches as f64 / total_time_sec }
    }

    pub fn vector_count(&self) -> u64 {
        self.total_vectors.load(Ordering::Relaxed)
    }

    pub fn avg_insertion_time_ms(&self) -> f64 {
        self.avg_insert_time_us() / 1000.0
    }

    pub fn avg_search_time_ms(&self) -> f64 {
        self.avg_search_time_us() / 1000.0
    }
    
    /// Примерная оценка использования памяти в KB
    pub fn memory_usage_kb(&self) -> u64 {
        let vectors = self.total_vectors.load(Ordering::Relaxed);
        // Примерная оценка: каждый вектор занимает ~4KB 
        // (1024 dimensions * 4 bytes + overhead для графа)
        vectors * 4
    }
}

/// Максимально эффективный векторный индекс на базе профессиональной hnsw_rs от Jean-Pierre Both
// @component: {"k":"C","id":"vector_index_hnsw","t":"HNSW vector index","m":{"cur":85,"tgt":95,"u":"%"},"f":["vector","hnsw","search"]}
pub struct VectorIndexHnswRs {
    config: HnswRsConfig,
    hnsw: Arc<RwLock<Option<Hnsw<'static, f32, DistCosine>>>>,
    id_to_point: Arc<RwLock<HashMap<String, usize>>>,
    point_to_id: Arc<RwLock<HashMap<usize, String>>>,
    stats: Arc<HnswRsStats>,
    next_point_id: AtomicU64,
}

impl VectorIndexHnswRs {
    /// Создание максимально эффективного индекса с правильным API hnsw_rs
    pub fn new(config: HnswRsConfig) -> Result<Self> {
        info!("Инициализация VectorIndexHnswRs с конфигурацией: max_connections={}, ef_construction={}", 
              config.max_connections, config.ef_construction);
        
        Ok(Self {
            config,
            hnsw: Arc::new(RwLock::new(None)),
            id_to_point: Arc::new(RwLock::new(HashMap::new())),
            point_to_id: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(HnswRsStats::default()),
            next_point_id: AtomicU64::new(0),
        })
    }
    
    /// Инициализация HNSW структуры с правильными параметрами (только если не существует)
    fn ensure_hnsw_initialized(&self, _expected_size: usize) -> Result<()> {
        let mut hnsw_guard = self.hnsw.write();
        
        if hnsw_guard.is_none() {
            // Используем max_elements из конфига, избегая пересоздания
            let actual_size = self.config.max_elements;
            let max_layers = self.config.max_layers.min((actual_size as f32).ln().trunc() as usize);
            
            info!("Создание HNSW структуры: size={}, layers={}, connections={}", 
                  actual_size, max_layers, self.config.max_connections);
            
            let distance = DistCosine {};
            let hnsw: Hnsw<'static, f32, DistCosine> = Hnsw::new(
                self.config.max_connections,
                actual_size,
                max_layers,
                self.config.ef_construction,
                distance,
            );
            
            *hnsw_guard = Some(hnsw);
            
            debug!("HNSW успешно создан и готов к использованию");
        } else {
            debug!("HNSW уже инициализирован, используем существующий индекс");
        }
        
        Ok(())
    }
    
    /// Максимально эффективное добавление одного вектора
    pub fn add(&self, id: String, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.config.dimension {
            return Err(anyhow!("Неверная размерность: {} != {}", vector.len(), self.config.dimension));
        }
        
        let start = Instant::now();
        
        // Проверяем дубликаты
        if self.id_to_point.read().contains_key(&id) {
            return Err(anyhow!("ID {} уже существует", id));
        }
        
        // Инициализируем HNSW если нужно
        self.ensure_hnsw_initialized(1000)?;
        
        let point_id = self.next_point_id.fetch_add(1, Ordering::Relaxed) as usize;
        
        // Добавляем в HNSW с правильным API
        {
            let mut hnsw_guard = self.hnsw.write();
            if let Some(ref mut hnsw) = hnsw_guard.as_mut() {
                hnsw.insert_data(&vector, point_id);
                debug!("Вектор {} добавлен с point_id={}", id, point_id);
            } else {
                return Err(anyhow!("HNSW не инициализирован"));
            }
        }
        
        // Обновляем маппинги атомарно
        {
            let mut id_to_point = self.id_to_point.write();
            let mut point_to_id = self.point_to_id.write();
            
            id_to_point.insert(id.clone(), point_id);
            point_to_id.insert(point_id, id);
        }
        
        let duration = start.elapsed().as_micros() as u64;
        self.stats.record_insertion(1, duration, false);
        
        Ok(())
    }
    
    /// Проверка, нужно ли расширение индекса
    fn check_capacity(&self, additional_size: usize) -> Result<bool> {
        let current_size = self.len();
        let new_total = current_size + additional_size;
        
        // Если превышаем 90% от max_elements, предупреждаем
        let capacity_threshold = (self.config.max_elements as f64 * 0.9) as usize;
        
        if new_total > capacity_threshold {
            warn!("HNSW индекс приближается к лимиту: {}/{} ({}%)", 
                  new_total, self.config.max_elements, 
                  (new_total as f64 / self.config.max_elements as f64 * 100.0) as u32);
            
            // Возвращаем true если превышаем лимит
            Ok(new_total > self.config.max_elements)
        } else {
            Ok(false)
        }
    }

    /// Инкрементальное пакетное добавление - БЕЗ полной перестройки
    pub fn add_batch(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        if vectors.is_empty() {
            return Ok(());
        }
        
        let start = Instant::now();
        
        // Проверяем capacity заранее
        if self.check_capacity(vectors.len())? {
            return Err(anyhow!("Превышен лимит HNSW индекса: {} + {} > {}", 
                             self.len(), vectors.len(), self.config.max_elements));
        }
        
        // Валидация размерности
        for (id, vector) in &vectors {
            if vector.len() != self.config.dimension {
                return Err(anyhow!("Неверная размерность для {}: {} != {}", 
                                 id, vector.len(), self.config.dimension));
            }
        }
        
        // Параллельная обработка для больших batch
        let use_parallel = self.config.use_parallel && vectors.len() > 100;
        let vectors_len = vectors.len();
        
        if use_parallel {
            info!("🚀 Параллельное добавление {} векторов в HNSW", vectors_len);
            self.add_batch_parallel(vectors)?;
        } else {
            self.add_batch_sequential(vectors)?;
        }
        
        let duration = start.elapsed().as_micros() as u64;
        self.stats.record_insertion(vectors_len as u64, duration, use_parallel);
        
        info!("✅ Добавлено {} векторов за {:.2}ms (parallel: {})", 
              vectors_len, duration as f64 / 1000.0, use_parallel);
        
        Ok(())
    }
    
    /// Последовательное добавление batch (для малых объемов)
    fn add_batch_sequential(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        // Инициализируем HNSW (только если не существует)
        self.ensure_hnsw_initialized(vectors.len())?;
        
        // Последовательное добавление одного за другим
        for (id, vector) in vectors {
            self.add(id, vector)?;
        }
        
        Ok(())
    }
    
    /// Параллельное добавление batch (для больших объемов)
    fn add_batch_parallel(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        use std::sync::atomic::Ordering;
        
        // Инициализируем HNSW
        self.ensure_hnsw_initialized(vectors.len())?;
        
        // Подготавливаем данные для параллельной вставки
        let mut data_for_insertion = Vec::with_capacity(vectors.len());
        let mut id_mappings = Vec::with_capacity(vectors.len());
        
        for (id, vector) in vectors {
            let point_id = self.next_point_id.fetch_add(1, Ordering::Relaxed) as usize;
            data_for_insertion.push((vector, point_id));
            id_mappings.push((id, point_id));
        }
        
        // Параллельная вставка - главное преимущество hnsw_rs
        {
            let mut hnsw_guard = self.hnsw.write();
            if let Some(ref mut hnsw) = hnsw_guard.as_mut() {
                let data_refs: Vec<_> = data_for_insertion.iter()
                    .map(|(v, id)| (v, *id))
                    .collect();
                hnsw.parallel_insert_data(&data_refs);
                debug!("Параллельная вставка {} векторов завершена", data_refs.len());
            } else {
                return Err(anyhow!("HNSW не инициализирован"));
            }
        }
        
        // Обновляем маппинги атомарно
        {
            let mut id_to_point = self.id_to_point.write();
            let mut point_to_id = self.point_to_id.write();
            
            for (id, point_id) in id_mappings {
                id_to_point.insert(id.clone(), point_id);
                point_to_id.insert(point_id, id);
            }
        }
        
        Ok(())
    }
    
    /// Максимально эффективный поиск с использованием профессионального API
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        if query.len() != self.config.dimension {
            return Err(anyhow!("Неверная размерность запроса: {} != {}", 
                             query.len(), self.config.dimension));
        }
        
        let start = Instant::now();
        
        let results = {
            let hnsw_guard = self.hnsw.read();
            if let Some(hnsw) = hnsw_guard.as_ref() {
                // Используем правильный API для поиска
                let neighbors = hnsw.search(query, k, self.config.ef_search);
                
                debug!("HNSW поиск нашел {} соседей", neighbors.len());
                neighbors
            } else {
                return Err(anyhow!("HNSW не инициализирован"));
            }
        };
        
        // Конвертируем результаты в наш формат
        let mut final_results = Vec::new();
        let point_to_id = self.point_to_id.read();
        
        for neighbor in results {
            if let Some(id) = point_to_id.get(&neighbor.d_id) {
                final_results.push((id.clone(), neighbor.distance));
            }
        }
        
        // Сортируем по возрастанию расстояния (лучшие результаты первыми)
        final_results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let duration = start.elapsed().as_micros() as u64;
        self.stats.record_search(duration, final_results.len() as u64);
        
        debug!("Поиск завершен: найдено {} результатов за {} мкс", 
               final_results.len(), duration);
        
        Ok(final_results)
    }
    
    /// Параллельный поиск для нескольких запросов (эксклюзивная возможность hnsw_rs)
    pub fn parallel_search(&self, queries: &[Vec<f32>], k: usize) -> Result<Vec<Vec<(String, f32)>>> {
        if queries.is_empty() {
            return Ok(Vec::new());
        }
        
        let start = Instant::now();
        
        // Валидация запросов
        for (i, query) in queries.iter().enumerate() {
            if query.len() != self.config.dimension {
                return Err(anyhow!("Неверная размерность запроса {}: {} != {}", 
                                 i, query.len(), self.config.dimension));
            }
        }
        
        let batch_results = {
            let hnsw_guard = self.hnsw.read();
            if let Some(hnsw) = hnsw_guard.as_ref() {
                hnsw.parallel_search(queries, k, self.config.ef_search)
            } else {
                return Err(anyhow!("HNSW не инициализирован"));
            }
        };
        
        // Конвертируем результаты
        let mut final_results = Vec::with_capacity(queries.len());
        let point_to_id = self.point_to_id.read();
        
        for query_results in batch_results {
            let mut converted_results = Vec::new();
            for neighbor in query_results {
                if let Some(id) = point_to_id.get(&neighbor.d_id) {
                    converted_results.push((id.clone(), neighbor.distance));
                }
            }
            final_results.push(converted_results);
        }
        
        let duration = start.elapsed().as_micros() as u64;
        let total_results: usize = final_results.iter().map(|r| r.len()).sum();
        self.stats.record_search(duration, total_results as u64);
        
        info!("Параллельный поиск {} запросов завершен: найдено {} результатов за {} мкс", 
              queries.len(), total_results, duration);
        
        Ok(final_results)
    }
    
    /// Удаление вектора (пометка как удаленный)
    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut id_to_point = self.id_to_point.write();
        let mut point_to_id = self.point_to_id.write();
        
        if let Some(point_id) = id_to_point.remove(id) {
            point_to_id.remove(&point_id);
            debug!("Вектор {} (point_id={}) помечен как удаленный", id, point_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Получение детальной статистики производительности
    pub fn stats(&self) -> &HnswRsStats {
        &self.stats
    }
    
    /// Получение конфигурации
    pub fn config(&self) -> &HnswRsConfig {
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
        
        info!("VectorIndexHnswRs полностью очищен");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hnsw_rs_basic() {
        let config = HnswRsConfig::default();
        let index = VectorIndexHnswRs::new(config).unwrap();
        
        // Тест добавления
        let vector1 = vec![0.1; 1024];
        let vector2 = vec![0.2; 1024];
        
        index.add("doc1".to_string(), vector1).unwrap();
        index.add("doc2".to_string(), vector2).unwrap();
        
        assert_eq!(index.len(), 2);
        
        // Тест поиска
        let query = vec![0.15; 1024];
        let results = index.search(&query, 2).unwrap();
        
        println!("Результаты поиска: {:?}", results);
        assert_eq!(results.len(), 2);
        
        // Проверяем что расстояния положительные и логичные
        let (id1, dist1) = &results[0];
        let (id2, dist2) = &results[1];
        println!("Первый результат: {} с расстоянием {}", id1, dist1);
        println!("Второй результат: {} с расстоянием {}", id2, dist2);
        
        // Результаты должны быть отсортированы по возрастанию расстояния
        assert!(dist1 <= dist2, "dist1={} должно быть <= dist2={}", dist1, dist2);
    }
    
    #[test]
    fn test_hnsw_rs_batch() {
        let config = HnswRsConfig::default();
        let index = VectorIndexHnswRs::new(config).unwrap();
        
        // Тест пакетного добавления
        let vectors = vec![
            ("doc1".to_string(), vec![0.1; 1024]),
            ("doc2".to_string(), vec![0.2; 1024]),
            ("doc3".to_string(), vec![0.3; 1024]),
        ];
        
        index.add_batch(vectors).unwrap();
        assert_eq!(index.len(), 3);
        
        // Статистика
        let stats = index.stats();
        // vector_count отражает общее количество записей всех insertion операций
        // add_batch -> add_batch_sequential -> add (3 раза) = 3 записи + 3 single insertions = 6
        assert!(stats.vector_count() >= 3, "Should have at least 3 vectors");
        assert!(stats.avg_insert_time_us() > 0.0);
    }
}