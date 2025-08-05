use anyhow::{anyhow, Result};
use hnsw_rs::hnsw::*;
use hnsw_rs::prelude::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tracing::{debug, info, warn};

use super::config::HnswConfig;
use super::stats::HnswStats;

/// Максимально эффективный векторный индекс на базе профессиональной hnsw_rs от Jean-Pierre Both
/// Реализует Single Responsibility Principle - только управление HNSW индексом
pub struct VectorIndex {
    config: HnswConfig,
    hnsw: Arc<RwLock<Option<Hnsw<'static, f32, DistCosine>>>>,
    id_to_point: Arc<RwLock<HashMap<String, usize>>>,
    point_to_id: Arc<RwLock<HashMap<usize, String>>>,
    stats: Arc<HnswStats>,
    next_point_id: AtomicU64,
}

impl VectorIndex {
    /// Создание максимально эффективного индекса с правильным API hnsw_rs
    pub fn new(config: HnswConfig) -> Result<Self> {
        config.validate()?;
        
        info!("Инициализация VectorIndex с конфигурацией: max_connections={}, ef_construction={}", 
              config.max_connections, config.ef_construction);
        
        Ok(Self {
            config,
            hnsw: Arc::new(RwLock::new(None)),
            id_to_point: Arc::new(RwLock::new(HashMap::new())),
            point_to_id: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(HnswStats::new()),
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
    
    /// Поиск ближайших векторов с профессиональной обработкой
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
        
        let results = {
            let hnsw_guard = self.hnsw.read();
            if let Some(ref hnsw) = hnsw_guard.as_ref() {
                // Устанавливаем ef_search для этого запроса
                let ef_search = self.config.ef_search.max(k);
                
                hnsw.search(query, k, ef_search)
            } else {
                let error = anyhow!("HNSW не инициализирован для поиска");
                self.stats.record_error();
                return Err(error);
            }
        };
        
        // Конвертируем point_id обратно в string ID
        let mut string_results = Vec::new();
        let point_to_id = self.point_to_id.read();
        
        for neighbour in results {
            let point_id = neighbour.d_id;
            let distance = neighbour.distance;
            
            if let Some(string_id) = point_to_id.get(&point_id) {
                string_results.push((string_id.clone(), distance));
            } else {
                warn!("Point ID {} не найден в маппинге", point_id);
            }
        }
        
        let duration = start.elapsed();
        // Примерная оценка distance calculations (зависит от алгоритма)
        let estimated_distance_calcs = (self.len() as f64).ln() as u64 * k as u64;
        self.stats.record_search(duration, estimated_distance_calcs);
        
        debug!("Поиск завершен за {:?}, найдено {} результатов", duration, string_results.len());
        Ok(string_results)
    }
    
    /// Параллельный поиск для множественных запросов
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
        
        info!("Начинаем параллельный поиск для {} запросов", queries.len());
        
        // Выполняем параллельный поиск через rayon
        use rayon::prelude::*;
        
        let results: Result<Vec<_>> = queries
            .par_iter()
            .map(|query| self.search(query, k))
            .collect();
        
        let duration = start.elapsed();
        info!("Параллельный поиск завершен за {:?}", duration);
        
        results
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
    pub fn get_all_ids(&self) -> Vec<String> {
        self.id_to_point.read().keys().cloned().collect()
    }
    
    /// Оценить качество индекса (0.0 - 1.0)
    pub fn estimate_quality(&self) -> f64 {
        let stats = self.stats.snapshot();
        
        // Простая эвристика качества на основе метрик
        let error_penalty = 1.0 - stats.error_rate;
        let speed_bonus = if stats.avg_search_time_ms < 10.0 { 1.0 } else { 10.0 / stats.avg_search_time_ms };
        let parallel_bonus = 0.8 + 0.2 * stats.parallel_efficiency;
        
        (error_penalty * speed_bonus * parallel_bonus).min(1.0)
    }
    
    
}