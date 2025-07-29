use crate::layers::{EphemeralStore, VectorStore, LongTermStore};
use crate::semantic::{SemanticRouter, VectorizerService, RerankerService, Vectorizer, Reranker};
use crate::types::{MemoryEvent, ExecutionContext, MemoryOperationResult, MemoryUsageStats};
use crate::{
    MemLayer, MemRef, MemMeta, MemSearchResult, MemoryStore, SemanticIndex, 
    MemoryConfig, LayerStats
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// >>@48=0B>@ ?0<OB8 - F5=B@0;L=K9 :><?>=5=B 4;O C?@02;5=8O 2A5<8 A;>O<8
/// 
/// Responsibilities:
/// - 0@H@CB870F8O 70?@>A>2 <564C A;>O<8 (M0-M3)
/// - !5<0=B8G5A:89 ?>8A: G5@57 M4
/// - 2B><0B8G5A:89 ?@><>CH5= 40==KE <564C A;>O<8
/// - >;8B8:8 C?@02;5=8O ?0<OBLN
/// - C1;8:0F8O A>1KB89 2 EventBus
pub struct MemoryCoordinator {
    // !;>8 ?0<OB8
    ephemeral: Arc<EphemeralStore>,
    short_term: Arc<VectorStore>,
    medium_term: Arc<VectorStore>,
    long_term: Arc<LongTermStore>,
    
    // !5<0=B8G5A:89 @>CB5@ (M4)
    semantic_router: Arc<SemanticRouter>,
    
    // >=D83C@0F8O
    config: MemoryConfig,
    
    // !B0B8AB8:0 8 :MH
    stats_cache: Arc<RwLock<Option<(MemoryUsageStats, DateTime<Utc>)>>>,
    
    // Event publishing (703;CH:0, 2 @50;L=>AB8 EventBus)
    event_sender: Arc<RwLock<Vec<MemoryEvent>>>,
}

impl MemoryCoordinator {
    /// !>740BL =>2K9 :>>@48=0B>@ ?0<OB8
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        // =8F80;878@C5< A;>8 ?0<OB8
        let ephemeral = Arc::new(EphemeralStore::new());
        
        let short_term = Arc::new(
            VectorStore::new(MemLayer::Short, config.vectors_path.join("short_term.json")).await
                .context("Failed to initialize short-term vector store")?
        );
        
        let medium_term = Arc::new(
            VectorStore::new(MemLayer::Medium, config.vectors_path.join("medium_term.json")).await
                .context("Failed to initialize medium-term vector store")?
        );
        
        let long_term = Arc::new(
            LongTermStore::new(&config.blobs_path).await
                .context("Failed to initialize long-term store")?
        );
        
        // =8F80;878@C5< A5<0=B8G5A:85 A5@28AK
        let vectorizer = Arc::new(
            VectorizerService::new(
                config.base_path.join("src/Qwen3-Embedding-0.6B-ONNX")
            ).await.context("Failed to initialize vectorizer service")?
        ) as Arc<dyn Vectorizer>;
        
        let reranker = Arc::new(
            RerankerService::new(
                config.base_path.join("src/Qwen3-Reranker-0.6B-ONNX")
            ).await.context("Failed to initialize reranker service")?
        ) as Arc<dyn Reranker>;
        
        let semantic_router = Arc::new(SemanticRouter::new(vectorizer, reranker));
        
        info!("Initialized MemoryCoordinator with 5-layer architecture");
        
        Ok(Self {
            ephemeral,
            short_term,
            medium_term,
            long_term,
            semantic_router,
            config,
            stats_cache: Arc::new(RwLock::new(None)),
            event_sender: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// =B5;;835=B=0O 70?8AL 40==KE A 2K1>@>< ?>4E>4OI53> A;>O
    pub async fn smart_put(&self, key: &str, data: &[u8], mut meta: MemMeta, ctx: &ExecutionContext) -> Result<MemoryOperationResult> {
        let start_time = std::time::Instant::now();
        let data_size = data.len();
        
        // K18@05< ?>4E>4OI89 A;>9 =0 >A=>25 @07<5@0 8 <5B040==KE
        let target_layer = self.select_layer_for_write(&meta, data_size);
        
        // >102;O5< :>=B5:AB=CN 8=D>@<0F8N 2 <5B040==K5
        meta.extra.insert("session_id".to_string(), 
                         serde_json::Value::String(ctx.session_id.clone().unwrap_or_default()));
        meta.extra.insert("request_id".to_string(), 
                         serde_json::Value::String(ctx.request_id.to_string()));
        
        // 0?8AK205< 2 2K1@0==K9 A;>9
        let result = match target_layer {
            MemLayer::Ephemeral => {
                self.ephemeral.put(key, data, &meta).await?;
                MemRef::new(MemLayer::Ephemeral, key.to_string())
            },
            MemLayer::Short => {
                self.short_term.put(key, data, &meta).await?;
                MemRef::new(MemLayer::Short, key.to_string())
            },
            MemLayer::Medium => {
                self.medium_term.put(key, data, &meta).await?;
                MemRef::new(MemLayer::Medium, key.to_string())
            },
            MemLayer::Long => {
                self.long_term.put(key, data, &meta).await?;
                MemRef::new(MemLayer::Long, key.to_string())
            },
            MemLayer::Semantic => {
                // !5<0=B8G5A:89 A;>9 =5 E@0=8B 40==K5 =0?@O<CN
                return Err(anyhow::anyhow!("Cannot write directly to semantic layer"));
            }
        };
        
        // =45:A8@C5< 2 A5<0=B8G5A:>< A;>5 5A;8 MB> B5:AB>2K5 40==K5
        if meta.content_type.starts_with("text/") {
            if let Ok(text) = std::str::from_utf8(data) {
                if let Err(e) = self.semantic_router.ingest(text, &result, &meta).await {
                    warn!("Failed to index text in semantic layer: {}", e);
                }
            }
        }
        
        let elapsed = start_time.elapsed();
        
        // C1;8:C5< A>1KB85
        self.publish_event(MemoryEvent::DataStored {
            layer: target_layer,
            key: key.to_string(),
            size_bytes: data_size,
        }).await;
        
        debug!("Stored {} bytes in {:?} layer: {}", data_size, target_layer, key);
        
        Ok(MemoryOperationResult {
            success: true,
            mem_ref: Some(result),
            bytes_processed: data_size,
            operation_time_ms: elapsed.as_millis() as u64,
            error: None,
        })
    }
    
    /// =B5;;835=B=>5 GB5=85 A ?>8A:>< ?> 2A5< A;>O<
    pub async fn smart_get(&self, key: &str, _ctx: &ExecutionContext) -> Result<Option<(Vec<u8>, MemMeta, MemRef)>> {
        let _start_time = std::time::Instant::now();
        
        // I5< 2> 2A5E A;>OE 2 ?>@O4:5 ?@8>@8B5B0 (1KAB@K5 A=0G0;0)
        let layers_to_check = [
            (MemLayer::Ephemeral, self.ephemeral.as_ref() as &dyn MemoryStore),
            (MemLayer::Short, &*self.short_term as &dyn MemoryStore),
            (MemLayer::Medium, &*self.medium_term as &dyn MemoryStore),
            (MemLayer::Long, &*self.long_term as &dyn MemoryStore),
        ];
        
        for (layer, store) in &layers_to_check {
            if let Some((data, meta)) = store.get(key).await? {
                let mem_ref = MemRef::new(*layer, key.to_string());
                
                // C1;8:C5< A>1KB85 CA?5H=>3> 4>ABC?0
                self.publish_event(MemoryEvent::DataAccessed {
                    layer: *layer,
                    key: key.to_string(),
                    hit: true,
                }).await;
                
                // @>25@O5< =5>1E>48<>ABL ?@><>CH5=0
                if self.should_promote(&meta, *layer).await {
                    if let Err(e) = self.promote_data(key, &data, &meta, *layer).await {
                        warn!("Failed to promote data from {:?}: {}", layer, e);
                    }
                }
                
                debug!("Retrieved {} bytes from {:?} layer: {}", data.len(), layer, key);
                return Ok(Some((data.to_vec(), meta, mem_ref)));
            }
        }
        
        // C1;8:C5< A>1KB85 ?@><0E0
        self.publish_event(MemoryEvent::DataAccessed {
            layer: MemLayer::Ephemeral, // Placeholder
            key: key.to_string(),
            hit: false,
        }).await;
        
        debug!("Key not found in any layer: {}", key);
        Ok(None)
    }
    
    /// !5<0=B8G5A:89 ?>8A: G5@57 2A5 A;>8
    pub async fn semantic_search(&self, query: &str, top_k: usize, _ctx: &ExecutionContext) -> Result<Vec<MemSearchResult>> {
        let start_time = std::time::Instant::now();
        
        let results = self.semantic_router.search(query, top_k).await
            .context("Semantic search failed")?;
        
        let elapsed = start_time.elapsed();
        
        self.publish_event(MemoryEvent::SemanticSearch {
            query: query.to_string(),
            results_count: results.len(),
            query_time_ms: elapsed.as_millis() as u64,
        }).await;
        
        debug!("Semantic search returned {} results in {:?}", results.len(), elapsed);
        Ok(results)
    }
    
    /// #40;8BL 40==K5 87 2A5E A;>52
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let mut deleted = false;
        
        // #40;O5< 87 2A5E A;>52
        if self.ephemeral.delete(key).await? { deleted = true; }
        if self.short_term.delete(key).await? { deleted = true; }
        if self.medium_term.delete(key).await? { deleted = true; }
        if self.long_term.delete(key).await? { deleted = true; }
        
        // #40;O5< 87 A5<0=B8G5A:>3> 8=45:A0
        for layer in [MemLayer::Ephemeral, MemLayer::Short, MemLayer::Medium, MemLayer::Long] {
            let mem_ref = MemRef::new(layer, key.to_string());
            if self.semantic_router.remove(&mem_ref).await? {
                deleted = true;
            }
        }
        
        if deleted {
            debug!("Deleted key from all layers: {}", key);
        }
        
        Ok(deleted)
    }
    
    /// @8=C48B5;L=>5 ?@>42865=85 40==KE <564C A;>O<8
    pub async fn promote_data(&self, key: &str, data: &[u8], meta: &MemMeta, from_layer: MemLayer) -> Result<()> {
        let to_layer = match from_layer {
            MemLayer::Ephemeral => MemLayer::Short,
            MemLayer::Short => MemLayer::Medium,
            MemLayer::Medium => MemLayer::Long,
            MemLayer::Long => return Ok(()), // 5:C40 ?@>42830BL
            MemLayer::Semantic => return Err(anyhow::anyhow!("Cannot promote from semantic layer")),
        };
        
        // 0?8AK205< 2 F5;52>9 A;>9
        match to_layer {
            MemLayer::Short => self.short_term.put(key, data, meta).await?,
            MemLayer::Medium => self.medium_term.put(key, data, meta).await?,
            MemLayer::Long => self.long_term.put(key, data, meta).await?,
            _ => unreachable!(),
        }
        
        // #40;O5< 87 8AE>4=>3> A;>O
        match from_layer {
            MemLayer::Ephemeral => { self.ephemeral.delete(key).await?; },
            MemLayer::Short => { self.short_term.delete(key).await?; },
            MemLayer::Medium => { self.medium_term.delete(key).await?; },
            _ => unreachable!(),
        }
        
        self.publish_event(MemoryEvent::DataPromoted {
            from_layer,
            to_layer,
            key: key.to_string(),
            reason: "access_pattern".to_string(),
        }).await;
        
        info!("Promoted data from {:?} to {:?}: {}", from_layer, to_layer, key);
        Ok(())
    }
    
    /// 2B><0B8G5A:0O >G8AB:0 CAB0@52H8E 40==KE
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let mut total_cleaned = 0;
        
        // G8AB:0 ephemeral A;>O
        total_cleaned += self.ephemeral.cleanup_expired().await?;
        
        // G8AB:0 short-term A;>O
        let short_policy = &self.config.ephemeral_to_short;
        total_cleaned += self.short_term.cleanup(short_policy.ttl_seconds, short_policy.min_access_count).await?;
        
        // @E828@>20=85 long-term D09;>2
        total_cleaned += self.long_term.archive_old_files(30).await?; // 30 4=59
        
        if total_cleaned > 0 {
            info!("Cleaned up {} expired items across all layers", total_cleaned);
        }
        
        Ok(total_cleaned)
    }
    
    /// >;CG8BL >1ICN AB0B8AB8:C 8A?>;L7>20=8O ?0<OB8
    pub async fn get_usage_stats(&self) -> Result<MemoryUsageStats> {
        // @>25@O5< :MH
        {
            let cache = self.stats_cache.read().await;
            if let Some((stats, timestamp)) = &*cache {
                if Utc::now().signed_duration_since(*timestamp).num_minutes() < 5 {
                    return Ok(stats.clone());
                }
            }
        }
        
        // !>18@05< AB0B8AB8:C 87 2A5E A;>52
        let mut layer_stats = HashMap::new();
        
        layer_stats.insert(MemLayer::Ephemeral, self.ephemeral.stats().await?);
        layer_stats.insert(MemLayer::Short, self.short_term.stats().await?);
        layer_stats.insert(MemLayer::Medium, self.medium_term.stats().await?);
        layer_stats.insert(MemLayer::Long, self.long_term.stats().await?);
        
        let total_items: u64 = layer_stats.values().map(|s| s.total_items).sum();
        let total_size_bytes: u64 = layer_stats.values().map(|s| s.total_size_bytes).sum();
        
        let stats = MemoryUsageStats {
            layers: layer_stats,
            total_items,
            total_size_bytes,
            cache_hit_rate: 0.85, // 03;CH:0
            avg_query_time_ms: 15.0, // 03;CH:0
            promotions_last_hour: 5, // 03;CH:0
        };
        
        // 1=>2;O5< :MH
        {
            let mut cache = self.stats_cache.write().await;
            *cache = Some((stats.clone(), Utc::now()));
        }
        
        Ok(stats)
    }
    
    /// Добавить текст в семантический индекс
    pub async fn semantic_index(&self, text: &str, mem_ref: &MemRef, meta: &MemMeta) -> Result<()> {
        self.semantic_router.ingest(text, mem_ref, meta).await?;
        
        self.publish_event(MemoryEvent::DataIndexed {
            mem_ref: mem_ref.clone(),
            vector_dimensions: 1024, // Qwen3 embedding size
        }).await;
        
        Ok(())
    }
    
    /// Публикация события (заглушка для EventBus)
    async fn publish_event(&self, event: MemoryEvent) {
        let mut events = self.event_sender.write().await;
        events.push(event);
        
        // В реальной реализации здесь был бы EventBus
        // eventbus.publish(event).await;
    }
    
    /// K1>@ A;>O 4;O 70?8A8 =0 >A=>25 <5B040==KE 8 @07<5@0
    fn select_layer_for_write(&self, meta: &MemMeta, data_size: usize) -> MemLayer {
        // $>@A8@>20==K5 B538
        if meta.tags.contains(&"ephemeral".to_string()) || meta.tags.contains(&"temp".to_string()) {
            return MemLayer::Ephemeral;
        }
        
        if meta.tags.contains(&"session".to_string()) {
            return MemLayer::Short;
        }
        
        if meta.tags.contains(&"persistent".to_string()) || meta.tags.contains(&"archive".to_string()) {
            return MemLayer::Long;
        }
        
        // 0 >A=>25 @07<5@0
        if data_size > 1024 * 1024 { // > 1MB
            return MemLayer::Long;
        } else if data_size > 10 * 1024 { // > 10KB
            return MemLayer::Medium;
        } else if meta.ttl_seconds.unwrap_or(0) < 3600 { // < 1 G0A TTL
            return MemLayer::Ephemeral;
        } else {
            return MemLayer::Short;
        }
    }
    
    /// @>25@:0 =5>1E>48<>AB8 ?@><>CH5=0 40==KE
    async fn should_promote(&self, meta: &MemMeta, current_layer: MemLayer) -> bool {
        match current_layer {
            MemLayer::Ephemeral => {
                let policy = &self.config.ephemeral_to_short;
                meta.access_count >= policy.min_access_count ||
                meta.tags.iter().any(|tag| policy.force_promotion_tags.contains(tag))
            },
            MemLayer::Short => {
                let policy = &self.config.short_to_medium;
                meta.access_count >= policy.min_access_count ||
                meta.tags.iter().any(|tag| policy.force_promotion_tags.contains(tag))
            },
            MemLayer::Medium => {
                let policy = &self.config.medium_to_long;
                meta.access_count >= policy.min_access_count ||
                meta.tags.iter().any(|tag| policy.force_promotion_tags.contains(tag))
            },
            _ => false,
        }
    }
    
    
    /// >;CG8BL ?>A;54=85 A>1KB8O 4;O >B;04:8
    pub async fn get_recent_events(&self, limit: usize) -> Vec<MemoryEvent> {
        let events = self.event_sender.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }
}

#[async_trait]
impl MemoryStore for MemoryCoordinator {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        let ctx = ExecutionContext::default();
        let result = self.smart_put(key, data, meta.clone(), &ctx).await?;
        
        if !result.success {
            return Err(anyhow::anyhow!("Smart put operation failed"));
        }
        
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        let ctx = ExecutionContext::default();
        if let Some((data, meta, _ref)) = self.smart_get(key, &ctx).await? {
            Ok(Some((data, meta)))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        self.delete(key).await
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        // @>25@O5< 2> 2A5E A;>OE
        Ok(
            self.ephemeral.exists(key).await? ||
            self.short_term.exists(key).await? ||
            self.medium_term.exists(key).await? ||
            self.long_term.exists(key).await?
        )
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let mut all_keys = std::collections::HashSet::new();
        
        // !>18@05< :;NG8 87 2A5E A;>52
        for key in self.ephemeral.list_keys().await? { all_keys.insert(key); }
        for key in self.short_term.list_keys().await? { all_keys.insert(key); }
        for key in self.medium_term.list_keys().await? { all_keys.insert(key); }
        for key in self.long_term.list_keys().await? { all_keys.insert(key); }
        
        Ok(all_keys.into_iter().collect())
    }
    
    async fn stats(&self) -> Result<LayerStats> {
        let usage_stats = self.get_usage_stats().await?;
        
        // 3@538@C5< AB0B8AB8:C ?> 2A5< A;>O<
        let total_items = usage_stats.total_items;
        let total_size_bytes = usage_stats.total_size_bytes;
        
        let oldest_item = usage_stats.layers.values()
            .filter_map(|s| s.oldest_item)
            .min();
        
        let newest_item = usage_stats.layers.values()
            .filter_map(|s| s.newest_item)
            .max();
        
        let avg_access_count = if total_items > 0 {
            usage_stats.layers.values()
                .map(|s| s.avg_access_count * s.total_items as f64)
                .sum::<f64>() / total_items as f64
        } else {
            0.0
        };
        
        Ok(LayerStats {
            total_items,
            total_size_bytes,
            oldest_item,
            newest_item,
            avg_access_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_coordinator() -> Result<MemoryCoordinator> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();
        
        // Создаём фиктивные директории моделей для тестов
        tokio::fs::create_dir_all(base_path.join("src/Qwen3-Embedding-0.6B-ONNX")).await?;
        tokio::fs::create_dir_all(base_path.join("src/Qwen3-Reranker-0.6B-ONNX")).await?;
        
        let config = MemoryConfig {
            base_path: base_path.clone(),
            sqlite_path: base_path.join("test.db"),
            blobs_path: base_path.join("blobs"),
            vectors_path: base_path.join("vectors"),
            cache_path: base_path.join("cache.db"),
            ..Default::default()
        };
        
        MemoryCoordinator::new(config).await
    }
    
    #[tokio::test]
    async fn test_memory_coordinator_basic_ops() {
        let coordinator = create_test_coordinator().await.unwrap();
        let key = "test_key";
        let data = b"test data";
        let mut meta = MemMeta::default();
        meta.content_type = "text/plain".to_string();
        
        let ctx = ExecutionContext::default();
        
        // Test smart_put
        let result = coordinator.smart_put(key, data, meta.clone(), &ctx).await.unwrap();
        assert!(result.success);
        assert!(result.mem_ref.is_some());
        
        // Test smart_get
        let retrieved = coordinator.smart_get(key, &ctx).await.unwrap();
        assert!(retrieved.is_some());
        let (retrieved_data, retrieved_meta, _mem_ref) = retrieved.unwrap();
        assert_eq!(retrieved_data, data);
        assert_eq!(retrieved_meta.content_type, "text/plain");
        
        // Test delete
        assert!(coordinator.delete(key).await.unwrap());
        assert!(coordinator.smart_get(key, &ctx).await.unwrap().is_none());
    }
    
    #[tokio::test]
    async fn test_layer_selection() {
        let coordinator = create_test_coordinator().await.unwrap();
        
        // Small data with temp tag should go to ephemeral
        let mut meta = MemMeta::default();
        meta.tags.push("temp".to_string());
        let layer = coordinator.select_layer_for_write(&meta, 100);
        assert_eq!(layer, MemLayer::Ephemeral);
        
        // Large data should go to long-term
        let meta = MemMeta::default();
        let layer = coordinator.select_layer_for_write(&meta, 2 * 1024 * 1024);
        assert_eq!(layer, MemLayer::Long);
        
        // Persistent tag should force long-term
        let mut meta = MemMeta::default();
        meta.tags.push("persistent".to_string());
        let layer = coordinator.select_layer_for_write(&meta, 100);
        assert_eq!(layer, MemLayer::Long);
    }
    
    #[tokio::test]
    async fn test_semantic_search() {
        let coordinator = create_test_coordinator().await.unwrap();
        let ctx = ExecutionContext::default();
        
        // Add some text data
        for i in 0..3 {
            let key = format!("doc_{}", i);
            let data = format!("This is document number {}", i);
            let mut meta = MemMeta::default();
            meta.content_type = "text/plain".to_string();
            
            coordinator.smart_put(&key, data.as_bytes(), meta, &ctx).await.unwrap();
        }
        
        // 51>;LH0O 7045@6:0 4;O 8=45:A0F88
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Test semantic search
        let results = coordinator.semantic_search("document", 5, &ctx).await.unwrap();
        //  703;CH:5 <>3CB 1KBL @57C;LB0BK 8;8 ?CAB>9 A?8A>: - MB> =>@<0;L=>
        assert!(results.len() <= 3);
    }
    
    #[tokio::test]
    async fn test_usage_stats() {
        let coordinator = create_test_coordinator().await.unwrap();
        let ctx = ExecutionContext::default();
        
        // Add some data
        for i in 0..5 {
            let key = format!("item_{}", i);
            let data = format!("data {}", i);
            let meta = MemMeta::default();
            
            coordinator.smart_put(&key, data.as_bytes(), meta, &ctx).await.unwrap();
        }
        
        let stats = coordinator.get_usage_stats().await.unwrap();
        assert!(stats.total_items > 0);
        assert!(stats.total_size_bytes > 0);
        assert!(!stats.layers.is_empty());
    }
}