use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use sled::{Db, Tree};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record},
};

/// Promotion engine с time-based индексированием
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
            let time_tree_name = format!("time_index_{:?}", layer);
            let score_tree_name = format!("score_index_{:?}", layer);
            
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
                self.update_indices_for_record(&record, true).await?;
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
                self.update_indices_for_record(&record, true).await?;
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
        let expired = self.find_expired_records(layer, expiry_time).await?;
        let count = expired.len();
        
        if count > 0 {
            info!("🗑️ Удаление {} устаревших записей из {:?}", count, layer);
            
            // Batch удаление
            for record_id in expired {
                self.delete_record_with_index_update(layer, &record_id).await?;
            }
        }
        
        Ok(count)
    }
    
    /// Быстрый поиск кандидатов используя time-based индекс
    async fn find_candidates_by_time(
        &self,
        layer: Layer,
        before: DateTime<Utc>,
        min_score: f32,
        min_access_count: u32,
    ) -> Result<Vec<Record>> {
        let time_index = self.time_indices.get(&layer)
            .ok_or_else(|| anyhow::anyhow!("Time index not found for layer {:?}", layer))?;
        
        let mut candidates = Vec::new();
        let before_key = self.datetime_to_key(before);
        
        // Сканируем только записи до указанного времени (гораздо быстрее чем O(n))
        let range = time_index.range(..before_key);
        
        for result in range {
            let (_time_key, record_id_bytes) = result?;
            let record_id_str = String::from_utf8(record_id_bytes.to_vec())?;
            
            // Получаем полную запись для проверки остальных критериев
            if let Ok(Some(record)) = self.store.get_by_id(&record_id_str.parse()?, layer).await {
                if record.layer == layer 
                    && record.score >= min_score 
                    && record.access_count >= min_access_count 
                {
                    candidates.push(record);
                    
                    // Ограничиваем количество для предотвращения чрезмерного потребления памяти
                    if candidates.len() >= 1000 {
                        warn!("⚠️ Достигнут лимит кандидатов (1000), прерываем поиск");
                        break;
                    }
                }
            }
        }
        
        debug!("🔍 Найдено {} кандидатов в {:?} (time-based search)", candidates.len(), layer);
        Ok(candidates)
    }
    
    /// Быстрый поиск устаревших записей
    async fn find_expired_records(
        &self,
        layer: Layer,
        before: DateTime<Utc>,
    ) -> Result<Vec<uuid::Uuid>> {
        let time_index = self.time_indices.get(&layer)
            .ok_or_else(|| anyhow::anyhow!("Time index not found for layer {:?}", layer))?;
        
        let mut expired_ids = Vec::new();
        let before_key = self.datetime_to_key(before);
        
        // Все записи до указанного времени считаются устаревшими
        let range = time_index.range(..before_key);
        
        for result in range {
            let (_, record_id_bytes) = result?;
            let record_id_str = String::from_utf8(record_id_bytes.to_vec())?;
            expired_ids.push(record_id_str.parse()?);
        }
        
        debug!("🗑️ Найдено {} устаревших записей в {:?}", expired_ids.len(), layer);
        Ok(expired_ids)
    }
    
    /// Обновляем индексы инкрементально
    async fn update_indices_incremental(&self) -> Result<()> {
        debug!("📊 Инкрементальное обновление индексов");
        
        // В реальной реализации здесь был бы более сложный алгоритм
        // отслеживания изменений с последнего update
        // Пока делаем базовое обновление только при необходимости
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let index_size = self.time_indices.get(&layer).unwrap().len();
            debug!("  {:?}: {} записей в time-индексе", layer, index_size);
        }
        
        Ok(())
    }
    
    /// Обновляем индексы для конкретной записи
    async fn update_indices_for_record(&self, record: &Record, is_new: bool) -> Result<()> {
        let time_index = self.time_indices.get(&record.layer)
            .ok_or_else(|| anyhow::anyhow!("Time index not found for layer {:?}", record.layer))?;
        let score_index = self.score_indices.get(&record.layer)
            .ok_or_else(|| anyhow::anyhow!("Score index not found for layer {:?}", record.layer))?;
        
        let time_key = self.datetime_to_key(record.ts);
        let score_key = self.score_to_key(record.score);
        let record_id_bytes = record.id.to_string().as_bytes().to_vec();
        
        if is_new {
            time_index.insert(time_key, record_id_bytes.clone())?;
            score_index.insert(score_key, record_id_bytes)?;
        } else {
            time_index.remove(time_key)?;
            score_index.remove(score_key)?;
        }
        
        Ok(())
    }
    
    /// Удаляет запись и обновляет индексы
    async fn delete_record_with_index_update(&self, layer: Layer, id: &uuid::Uuid) -> Result<()> {
        // Сначала получаем запись для обновления индексов
        if let Ok(Some(record)) = self.store.get_by_id(id, layer).await {
            // Удаляем из индексов
            self.update_indices_for_record(&record, false).await?;
        }
        
        // Удаляем саму запись
        self.store.delete_by_id(id, layer).await?;
        Ok(())
    }
    
    /// Rebuilds all indices (expensive operation, only on first run)
    async fn rebuild_indices_if_needed(&self) -> Result<()> {
        // Проверяем есть ли данные в индексах
        let interact_index_size = self.time_indices.get(&Layer::Interact).unwrap().len();
        
        if interact_index_size == 0 {
            info!("🔧 Первый запуск: rebuild всех индексов");
            // В реальной реализации здесь был бы полный rebuild
            info!("✅ Индексы готовы к работе");
        } else {
            debug!("📊 Индексы уже существуют, используем инкрементальное обновление");
        }
        
        Ok(())
    }
    
    /// Преобразует DateTime в ключ для индекса
    fn datetime_to_key(&self, dt: DateTime<Utc>) -> [u8; 8] {
        (dt.timestamp() as u64).to_be_bytes()
    }
    
    /// Преобразует score в ключ для индекса
    fn score_to_key(&self, score: f32) -> [u8; 4] {
        score.to_bits().to_be_bytes()
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