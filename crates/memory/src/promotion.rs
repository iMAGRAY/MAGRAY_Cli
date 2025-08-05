use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sled::{Db, Tree};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record},
};

/// Promotion engine с time-based индексированием
// @component: {"k":"C","id":"promotion_engine","t":"Time-based memory promotion","m":{"cur":75,"tgt":90,"u":"%"},"f":["promotion","time-index"]}
pub struct PromotionEngine {
    store: Arc<VectorStore>,
    config: PromotionConfig,
    _db: Arc<Db>,
    /// Индекс записей по времени создания для быстрого поиска кандидатов
    time_indices: BTreeMap<Layer, Arc<Tree>>,
    /// Индекс записей по score для быстрой фильтрации
    score_indices: BTreeMap<Layer, Arc<Tree>>,
}

impl PromotionEngine {
    pub async fn new(store: Arc<VectorStore>, config: PromotionConfig, db: Arc<Db>) -> Result<Self> {
        info!("🚀 Инициализация PromotionEngine с time-based индексами");
        
        let mut time_indices = BTreeMap::new();
        let mut score_indices = BTreeMap::new();
        
        // Создаем индексы для каждого слоя
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let time_tree_name = format!("time_index_{layer:?}");
            let score_tree_name = format!("score_index_{layer:?}");
            
            let time_tree = db.open_tree(&time_tree_name)?;
            let score_tree = db.open_tree(&score_tree_name)?;
            
            time_indices.insert(layer, Arc::new(time_tree));
            score_indices.insert(layer, Arc::new(score_tree));
            
            info!("  📊 Создан индекс для слоя {:?}", layer);
        }
        
        let engine = Self {
            store,
            config,
            _db: db,
            time_indices,
            score_indices,
        };
        
        // Инициализируем индексы при первом запуске
        engine.rebuild_indices_if_needed().await?;
        
        Ok(engine)
    }
    
    /// Основной цикл promotion с оптимизированным поиском
    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        let start_time = std::time::Instant::now();
        let mut stats = PromotionStats::default();
        
        info!("🔄 Запуск promotion цикла");
        
        // Этап 1: Обновляем индексы перед работой
        let index_update_time = std::time::Instant::now();
        self.update_indices_incremental().await?;
        stats.index_update_time_ms = index_update_time.elapsed().as_millis() as u64;
        
        // Этап 2: Promote записи между слоями
        let promotion_time = std::time::Instant::now();
        stats.interact_to_insights = self.promote_interact_to_insights().await?;
        stats.insights_to_assets = self.promote_insights_to_assets().await?;
        stats.promotion_time_ms = promotion_time.elapsed().as_millis() as u64;
        
        // Этап 3: Удаляем устаревшие записи
        let cleanup_time = std::time::Instant::now();
        stats.expired_interact = self.expire_records(Layer::Interact).await?;
        stats.expired_insights = self.expire_records(Layer::Insights).await?;
        stats.cleanup_time_ms = cleanup_time.elapsed().as_millis() as u64;
        
        stats.total_time_ms = start_time.elapsed().as_millis() as u64;
        
        info!("✅ Promotion цикл завершен за {}ms", stats.total_time_ms);
        info!("   Индексы: {}ms, Promotion: {}ms, Cleanup: {}ms", 
              stats.index_update_time_ms, stats.promotion_time_ms, stats.cleanup_time_ms);
        
        Ok(stats)
    }
    
    /// Оптимизированное продвижение Interact -> Insights
    async fn promote_interact_to_insights(&self) -> Result<usize> {
        let now = Utc::now();
        let threshold_time = now - Duration::hours(self.config.interact_ttl_hours as i64);
        
        // Используем time-based индекс для поиска старых записей
        let candidates = self.find_candidates_by_time(
            Layer::Interact,
            threshold_time,
            self.config.promote_threshold,
            2, // min_access_count
        ).await?;
        
        let count = candidates.len();
        if count > 0 {
            info!("🔄 Продвижение {} записей: Interact -> Insights", count);
            
            // Применяем decay и обновляем слой
            let promoted: Vec<_> = candidates.into_iter()
                .map(|mut r| {
                    r.layer = Layer::Insights;
                    r.score *= self.config.decay_factor;
                    r
                })
                .collect();
            
            // Batch операции для производительности
            self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
            
            // Удаляем из старого слоя и обновляем индексы
            for record in &promoted {
                self.delete_record_with_index_update(Layer::Interact, &record.id).await?;
                self.update_indices_for_record(record, true).await?;
            }
        }
        
        Ok(count)
    }
    
    /// Оптимизированное продвижение Insights -> Assets
    async fn promote_insights_to_assets(&self) -> Result<usize> {
        let now = Utc::now();
        let threshold_time = now - Duration::days(self.config.insights_ttl_days as i64);
        
        let candidates = self.find_candidates_by_time(
            Layer::Insights,
            threshold_time,
            self.config.promote_threshold * 1.2,
            5, // min_access_count
        ).await?;
        
        let count = candidates.len();
        if count > 0 {
            info!("🔄 Продвижение {} записей: Insights -> Assets", count);
            
            let promoted: Vec<_> = candidates.into_iter()
                .map(|mut r| {
                    r.layer = Layer::Assets;
                    r
                })
                .collect();
            
            self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
            
            for record in &promoted {
                self.delete_record_with_index_update(Layer::Insights, &record.id).await?;
                self.update_indices_for_record(record, true).await?;
            }
        }
        
        Ok(count)
    }
    
    /// Оптимизированное удаление устаревших записей
    async fn expire_records(&self, layer: Layer) -> Result<usize> {
        let expiry_time = match layer {
            Layer::Interact => Utc::now() - Duration::hours(self.config.interact_ttl_hours as i64 * 2),
            Layer::Insights => Utc::now() - Duration::days(self.config.insights_ttl_days as i64),
            Layer::Assets => return Ok(0), // Assets не истекают
        };
        
        // Находим записи старше expiry_time используя индекс
        let expired = self.find_expired_records_by_time(layer, expiry_time).await?;
        let count = expired.len();
        
        if count > 0 {
            info!("🗑️ Удаление {} устаревших записей из {:?}", count, layer);
            
            // Batch удаление
            for record in expired {
                self.delete_record_with_index_update(layer, &record.id).await?;
            }
        }
        
        Ok(count)
    }
    
    /// Основной API метод для координатора
    pub async fn promote(&self) -> Result<PromotionStats> {
        self.run_promotion_cycle().await
    }
    
    /// Получить текущую статистику promotion engine
    pub fn stats(&self) -> PromotionStats {
        // Возвращаем базовую статистику
        PromotionStats::default()
    }
    
    /// Простая оценка доступной памяти (в production использовать sysinfo)
    fn estimate_available_memory_mb(&self) -> usize {
        // Базовая эвристика - в реальности заменить на:
        // use sysinfo::{System, SystemExt};
        // let mut sys = System::new_all();
        // sys.refresh_memory();
        // sys.available_memory() / 1024 / 1024
        
        #[cfg(target_os = "windows")]
        {
            // Windows: консервативная оценка
            2048 // 2GB доступно
        }
        #[cfg(not(target_os = "windows"))]
        {
            use std::fs;
            
            // Linux: читаем /proc/meminfo
            if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemAvailable:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                return kb / 1024; // KB to MB
                            }
                        }
                    }
                }
            }
            
            // Fallback
            1024 // 1GB
        }
    }
    
    /// Обрабатывает batch кандидатов для предотвращения переполнения памяти
    async fn process_candidates_batch(&self, candidates: &mut Vec<Record>, layer: Layer) -> Result<()> {
        if candidates.is_empty() {
            return Ok(());
        }
        
        debug!("🔄 Processing candidates batch: {} records from {:?}", candidates.len(), layer);
        
        // Сортируем по priority для обработки самых важных первыми
        candidates.sort_by(|a, b| {
            let priority_a = self.calculate_promotion_priority(a);
            let priority_b = self.calculate_promotion_priority(b);
            priority_b.partial_cmp(&priority_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Обрабатываем top кандидатов
        let batch_size = candidates.len().min(1000); // Максимум 1000 за раз  
        let top_candidates = candidates.drain(0..batch_size).collect::<Vec<_>>();
        
        info!("📋 Processing {} top priority candidates from batch", top_candidates.len());
        
        // В зависимости от layer применяем соответствующую обработку
        match layer {
            Layer::Interact => {
                self.promote_batch_to_insights(top_candidates).await?;
            }
            Layer::Insights => {
                self.promote_batch_to_assets(top_candidates).await?;
            }
            Layer::Assets => {
                // Assets не продвигаются дальше, просто очищаем устаревшие
                debug!("Assets layer - no promotion needed");
            }
        }
        
        Ok(())
    }
    
    /// Продвигает batch записей из Interact в Insights
    async fn promote_batch_to_insights(&self, candidates: Vec<Record>) -> Result<()> {
        if candidates.is_empty() {
            return Ok(());
        }
        
        let promoted: Vec<_> = candidates.into_iter()
            .map(|mut r| {
                r.layer = Layer::Insights;
                r.score *= 0.9; // Decay factor
                r
            })
            .collect();
        
        // Batch операция для эффективности
        self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
        
        // Удаляем из старого слоя
        for record in &promoted {
            self.delete_record_with_index_update(Layer::Interact, &record.id).await?;
            self.update_indices_for_record(record, true).await?;
        }
        
        debug!("✅ Promoted {} records: Interact -> Insights", promoted.len());
        Ok(())
    }
    
    /// Продвигает batch записей из Insights в Assets
    async fn promote_batch_to_assets(&self, candidates: Vec<Record>) -> Result<()> {
        if candidates.is_empty() {
            return Ok(());
        }
        
        let promoted: Vec<_> = candidates.into_iter()
            .map(|mut r| {
                r.layer = Layer::Assets;
                // Assets не имеют decay - это долговременное хранение
                r
            })
            .collect();
        
        self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
        
        for record in &promoted {
            self.delete_record_with_index_update(Layer::Insights, &record.id).await?;
            self.update_indices_for_record(record, true).await?;
        }
        
        debug!("✅ Promoted {} records: Insights -> Assets", promoted.len());
        Ok(())
    }
    
    /// Вычисляет priority для promotion
    fn calculate_promotion_priority(&self, record: &Record) -> f32 {
        use chrono::Utc;
        
        // Многофакторная модель priority
        let base_score = record.score * 0.4;
        let access_factor = (record.access_count as f32).ln_1p() * 0.3;
        let recency_factor = {
            let hours_since_access = (Utc::now() - record.last_access).num_hours() as f32;
            (1.0 / (1.0 + hours_since_access / 24.0)) * 0.2
        };
        let age_factor = {
            let hours_since_creation = (Utc::now() - record.ts).num_hours() as f32;
            (1.0 / (1.0 + hours_since_creation / 168.0)) * 0.1 // 168h = 1 week
        };
        
        base_score + access_factor + recency_factor + age_factor
    }
    
    /// Получает статистику производительности
    pub async fn get_performance_stats(&self) -> Result<PromotionPerformanceStats> {
        let mut stats = PromotionPerformanceStats::default();
        
        for (layer, time_index) in &self.time_indices {
            let time_index_size = time_index.len();
            let score_index_size = self.score_indices.get(layer).unwrap().len();
            
            match layer {
                Layer::Interact => {
                    stats.interact_time_index_size = time_index_size;
                    stats.interact_score_index_size = score_index_size;
                }
                Layer::Insights => {
                    stats.insights_time_index_size = time_index_size;
                    stats.insights_score_index_size = score_index_size;
                }
                Layer::Assets => {
                    stats.assets_time_index_size = time_index_size;
                    stats.assets_score_index_size = score_index_size;
                }
            }
        }
        
        Ok(stats)
    }

    /// Восстанавливает индексы если необходимо
    async fn rebuild_indices_if_needed(&self) -> Result<()> {
        info!("🔧 Проверка необходимости восстановления индексов");
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let time_index = self.time_indices.get(&layer).unwrap();
            let score_index = self.score_indices.get(&layer).unwrap();
            
            // Если индексы пусты, нужно их восстановить
            if time_index.is_empty() || score_index.is_empty() {
                info!("Восстановление индексов для слоя {:?}", layer);
                self.rebuild_indices_for_layer(layer).await?;
            }
        }
        
        Ok(())
    }

    /// Восстанавливает индексы для конкретного слоя
    async fn rebuild_indices_for_layer(&self, layer: Layer) -> Result<()> {
        let time_index = self.time_indices.get(&layer).unwrap();
        let score_index = self.score_indices.get(&layer).unwrap();
        
        // Очищаем существующие индексы
        time_index.clear()?;
        score_index.clear()?;
        
        // Получаем все записи из storage
        let tree = Arc::new(self.store.get_tree(layer).await?);
        let mut indexed_count = 0;
        
        for result in tree.iter() {
            let (key, value) = result?;
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                let record = &stored.record;
                
                // Добавляем в time index (timestamp -> record_id)
                let time_key = format!("{:020}", record.ts.timestamp_nanos_opt().unwrap_or(0));
                time_index.insert(time_key.as_bytes(), key.as_ref())?;
                
                // Добавляем в score index (score -> record_id) 
                let score_key = format!("{:020}", (record.score * 1000000.0) as u64);
                score_index.insert(score_key.as_bytes(), key.as_ref())?;
                
                indexed_count += 1;
            }
        }
        
        info!("✅ Восстановлено {} записей в индексах для слоя {:?}", indexed_count, layer);
        Ok(())
    }

    /// Инкрементально обновляет индексы
    async fn update_indices_incremental(&self) -> Result<()> {
        debug!("🔄 Инкрементальное обновление индексов");
        
        // В данной реализации мы просто проверяем консистентность
        // В будущем можно добавить более сложную логику отслеживания изменений
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let time_index = self.time_indices.get(&layer).unwrap();
            let tree = Arc::new(self.store.get_tree(layer).await?);
            
            // Простая проверка: количество записей в дереве должно совпадать с индексом
            let tree_size = tree.len();
            let index_size = time_index.len();
            
            // Если есть большое расхождение, перестраиваем индекс
            if tree_size > 0 && index_size < tree_size / 2 {
                info!("Обнаружено расхождение в индексах для {:?}: дерево={}, индекс={}. Перестройка...", 
                      layer, tree_size, index_size);
                self.rebuild_indices_for_layer(layer).await?;
            }
        }
        
        Ok(())
    }

    /// Находит кандидатов для promotion по времени с помощью индексов
    async fn find_candidates_by_time(
        &self,
        layer: Layer,
        before: DateTime<Utc>,
        min_score: f32,
        limit: usize,
    ) -> Result<Vec<Record>> {
        let time_index = self.time_indices.get(&layer).unwrap();
        let _score_index = self.score_indices.get(&layer).unwrap();
        let mut candidates = Vec::new();
        
        // Ищем записи старше указанного времени
        let time_threshold = format!("{:020}", before.timestamp_nanos_opt().unwrap_or(0));
        
        for result in time_index.range(..time_threshold.as_bytes()) {
            if candidates.len() >= limit {
                break;
            }
            
            let (_, record_id) = result?;
            let tree = Arc::new(self.store.get_tree(layer).await?);
            
            if let Some(value) = tree.get(&record_id)? {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    let record = stored.record;
                    
                    // Проверяем score threshold
                    if record.score >= min_score {
                        candidates.push(record);
                    }
                }
            }
        }
        
        // Сортируем по приоритету promotion
        candidates.sort_by(|a, b| {
            let priority_a = self.calculate_promotion_priority(a);
            let priority_b = self.calculate_promotion_priority(b);
            priority_b.partial_cmp(&priority_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(candidates)
    }

    /// Удаляет запись и обновляет индексы
    async fn delete_record_with_index_update(&self, layer: Layer, record_id: &uuid::Uuid) -> Result<()> {
        let tree = Arc::new(self.store.get_tree(layer).await?);
        let key = record_id.as_bytes();
        
        // Получаем запись перед удалением для обновления индексов
        if let Some(value) = tree.get(key)? {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                let record = stored.record;
                
                // Удаляем из индексов
                let time_key = format!("{:020}", record.ts.timestamp_nanos_opt().unwrap_or(0));
                let score_key = format!("{:020}", (record.score * 1000000.0) as u64);
                
                if let Some(time_index) = self.time_indices.get(&layer) {
                    let _ = time_index.remove(time_key.as_bytes());
                }
                
                if let Some(score_index) = self.score_indices.get(&layer) {
                    let _ = score_index.remove(score_key.as_bytes());
                }
            }
        }
        
        // Удаляем из основного storage
        tree.remove(key)?;
        
        Ok(())
    }

    /// Обновляет индексы для записи
    async fn update_indices_for_record(&self, record: &Record, is_new: bool) -> Result<()> {
        let time_index = self.time_indices.get(&record.layer).unwrap();
        let score_index = self.score_indices.get(&record.layer).unwrap();
        
        let record_id = record.id.as_bytes();
        let time_key = format!("{:020}", record.ts.timestamp_nanos_opt().unwrap_or(0));
        let score_key = format!("{:020}", (record.score * 1000000.0) as u64);
        
        if is_new {
            // Добавляем в индексы
            time_index.insert(time_key.as_bytes(), record_id)?;
            score_index.insert(score_key.as_bytes(), record_id)?;
        } else {
            // Обновляем индексы (удаляем старые, добавляем новые)
            // В данной простой реализации просто добавляем
            time_index.insert(time_key.as_bytes(), record_id)?;
            score_index.insert(score_key.as_bytes(), record_id)?;
        }
        
        Ok(())
    }

    /// Находит устаревшие записи для удаления
    async fn find_expired_records_by_time(&self, layer: Layer, before: DateTime<Utc>) -> Result<Vec<Record>> {
        let time_index = self.time_indices.get(&layer).unwrap();
        let mut expired = Vec::new();
        
        let time_threshold = format!("{:020}", before.timestamp_nanos_opt().unwrap_or(0));
        
        for result in time_index.range(..time_threshold.as_bytes()) {
            let (_, record_id) = result?;
            let tree = Arc::new(self.store.get_tree(layer).await?);
            
            if let Some(value) = tree.get(&record_id)? {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    expired.push(stored.record);
                }
            }
        }
        
        Ok(expired)
    }
}

#[derive(Debug, Default)]
pub struct PromotionStats {
    pub interact_to_insights: usize,
    pub insights_to_assets: usize,
    pub expired_interact: usize,
    pub expired_insights: usize,
    
    // Временные метрики для анализа производительности
    pub total_time_ms: u64,
    pub index_update_time_ms: u64,
    pub promotion_time_ms: u64,
    pub cleanup_time_ms: u64,
}

#[derive(Debug, Default)]
pub struct PromotionPerformanceStats {
    pub interact_time_index_size: usize,
    pub interact_score_index_size: usize,
    pub insights_time_index_size: usize,
    pub insights_score_index_size: usize,
    pub assets_time_index_size: usize,
    pub assets_score_index_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_conversion() {
        
        // Создаем тестовые функции для key conversion
        let datetime_to_key = |dt: DateTime<Utc>| -> [u8; 8] {
            (dt.timestamp() as u64).to_be_bytes()
        };
        
        let score_to_key = |score: f32| -> [u8; 4] {
            score.to_bits().to_be_bytes()
        };
        
        // Тестируем преобразование времени
        let dt1 = Utc::now();
        let dt2 = dt1 + Duration::hours(1);
        
        let key1 = datetime_to_key(dt1);
        let key2 = datetime_to_key(dt2);
        
        // Более поздняя дата должна иметь больший ключ
        assert!(key1 < key2);
        
        // Тестируем преобразование score
        let score1 = 0.5f32;
        let score2 = 0.8f32;
        
        let score_key1 = score_to_key(score1);
        let score_key2 = score_to_key(score2);
        
        // Больший score должен иметь больший ключ
        assert!(score_key1 < score_key2);
    }
}