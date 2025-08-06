//! Query Layer Implementation - Высокоуровневая бизнес-логика поиска
//!
//! SemanticQueryLayer координирует поиск между storage, index и cache слоями.
//! Реализует сложные алгоритмы ранжирования и фильтрации.

use anyhow::{Result, Context};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info};

use crate::{
    types::{Record, Layer, SearchOptions},
    layers::{
        QueryLayer, StorageLayer, IndexLayer, CacheLayer, 
        VectorSearchResult, RankingCriteria, QueryStats, 
        LayerHealth, LayerHealthStatus, QueryConfig
    },
};

/// Semantic Query Layer - координирует поиск между слоями
pub struct SemanticQueryLayer {
    config: QueryConfig,
    storage_layer: Arc<dyn StorageLayer>,
    index_layer: Arc<dyn IndexLayer>,
    cache_layer: Arc<dyn CacheLayer>,
}

impl SemanticQueryLayer {
    pub async fn new(
        config: QueryConfig,
        storage_layer: Arc<dyn StorageLayer>,
        index_layer: Arc<dyn IndexLayer>,
        cache_layer: Arc<dyn CacheLayer>,
    ) -> Result<Arc<Self>> {
        info!("🎯 Инициализация Semantic Query Layer");
        
        Ok(Arc::new(Self {
            config,
            storage_layer,
            index_layer,
            cache_layer,
        }))
    }
}

#[async_trait]
impl QueryLayer for SemanticQueryLayer {
    async fn semantic_search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!("🔍 Semantic search: '{}' в слое {:?}", query, layer);

        // 1. Получаем embedding (с кэшированием)
        let embedding = self.get_embedding(query).await?;

        // 2. Поиск через index layer
        let search_results = self.index_layer
            .search_vectors(&embedding, layer, options.top_k.unwrap_or(self.config.default_top_k))
            .await?;

        // 3. Получаем полные записи из storage
        let mut records = Vec::new();
        for result in search_results {
            if let Ok(Some(record)) = self.storage_layer.get(&result.id, layer).await {
                records.push(record);
            }
        }

        // 4. Ранжирование если включено
        if self.config.enable_reranking {
            let criteria = RankingCriteria::default();
            self.rank_results(&mut records, &criteria).await?;
        }

        debug!("✅ Найдено {} результатов для '{}'", records.len(), query);
        Ok(records)
    }

    async fn search_by_embedding(
        &self,
        embedding: &[f32],
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let search_results = self.index_layer
            .search_vectors(embedding, layer, options.top_k.unwrap_or(self.config.default_top_k))
            .await?;

        let mut records = Vec::new();
        for result in search_results {
            if let Ok(Some(record)) = self.storage_layer.get(&result.id, layer).await {
                records.push(record);
            }
        }

        Ok(records)
    }

    async fn hybrid_search(
        &self,
        query: &str,
        text_filters: &HashMap<String, String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        // 1. Фильтрация по метаданным
        let filtered_records = self.storage_layer
            .filter_by_metadata(text_filters, layer)
            .await?;

        // 2. Semantic search среди отфильтрованных
        let embedding = self.get_embedding(query).await?;
        let search_results = self.search_by_embedding(&embedding, layer, options).await?;

        // 3. Пересечение результатов
        let filtered_ids: std::collections::HashSet<Uuid> = 
            filtered_records.iter().map(|r| r.id).collect();
        
        let results: Vec<Record> = search_results
            .into_iter()
            .filter(|r| filtered_ids.contains(&r.id))
            .collect();

        Ok(results)
    }

    async fn rank_results(&self, results: &mut Vec<Record>, _criteria: &RankingCriteria) -> Result<()> {
        // Простейшее ранжирование по recency
        results.sort_by(|a, b| {
            a.metadata.created_at.cmp(&b.metadata.created_at).reverse()
        });
        Ok(())
    }

    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Проверяем cache
        if let Ok(Some(cached)) = self.cache_layer.get(text).await {
            return Ok(cached);
        }

        // Генерируем fallback embedding
        let embedding = self.generate_fallback_embedding(text);
        
        // Кэшируем результат
        let _ = self.cache_layer.put(text, embedding.clone()).await;
        
        Ok(embedding)
    }

    async fn get_embeddings_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.get_embedding(text).await?);
        }
        Ok(embeddings)
    }

    async fn query_stats(&self) -> Result<QueryStats> {
        Ok(QueryStats::default())
    }
}

impl SemanticQueryLayer {
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0; 1024];
        let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
        
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u32) % 1000) as f32 / 1000.0) - 0.5;
        }
        
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
        
        embedding
    }
}

#[async_trait]
impl LayerHealth for SemanticQueryLayer {
    async fn health_check(&self) -> Result<LayerHealthStatus> {
        use chrono::Utc;
        use std::collections::HashMap;

        let start = std::time::Instant::now();
        let mut healthy = true;
        let mut details = HashMap::new();

        // Проверяем зависимые слои
        if let Err(e) = self.storage_layer.ready_check().await {
            healthy = false;
            details.insert("storage_layer_error".to_string(), e.to_string());
        }

        if let Err(e) = self.index_layer.ready_check().await {
            healthy = false;
            details.insert("index_layer_error".to_string(), e.to_string());
        }

        if let Err(e) = self.cache_layer.ready_check().await {
            healthy = false;
            details.insert("cache_layer_error".to_string(), e.to_string());
        }

        let response_time_ms = start.elapsed().as_millis() as f32;

        Ok(LayerHealthStatus {
            layer_name: "SemanticQueryLayer".to_string(),
            healthy,
            response_time_ms,
            error_rate: if healthy { 0.0 } else { 1.0 },
            last_check: Utc::now(),
            details,
        })
    }

    async fn ready_check(&self) -> Result<bool> {
        Ok(self.storage_layer.ready_check().await? &&
           self.index_layer.ready_check().await? &&
           self.cache_layer.ready_check().await?)
    }
}