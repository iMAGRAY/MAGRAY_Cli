use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record},
};

// @component: {"k":"C","id":"promotion_engine","t":"Memory layer promotion","m":{"cur":75,"tgt":90,"u":"%"},"d":["vector_store"],"f":["memory","lifecycle"]}
// @real_implementation: ✅
// @bottleneck: Full table scan for candidates
// @upgrade_effort: 1 day for optimization
pub struct PromotionEngine {
    store: Arc<VectorStore>,
    config: PromotionConfig,
}

impl PromotionEngine {
    pub fn new(store: Arc<VectorStore>, config: PromotionConfig) -> Self {
        Self { store, config }
    }

    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        let mut stats = PromotionStats::default();

        // Process each layer
        stats.interact_to_insights = self.promote_interact_to_insights().await?;
        stats.insights_to_assets = self.promote_insights_to_assets().await?;
        stats.expired_interact = self.expire_interact_records().await?;
        stats.expired_insights = self.expire_insights_records().await?;

        info!("Promotion cycle completed: {:?}", stats);
        Ok(stats)
    }

    async fn promote_interact_to_insights(&self) -> Result<usize> {
        let now = Utc::now();
        let threshold_time = now - Duration::hours(self.config.interact_ttl_hours as i64);
        
        // Find candidates: high-score records older than threshold
        let candidates = self.find_promotion_candidates(
            Layer::Interact,
            threshold_time,
            self.config.promote_threshold,
        ).await?;

        let count = candidates.len();
        if count > 0 {
            info!("Promoting {} records from Interact to Insights", count);
            
            // Update layer and insert into Insights
            let promoted: Vec<_> = candidates.into_iter()
                .map(|mut r| {
                    r.layer = Layer::Insights;
                    r.score *= self.config.decay_factor; // Apply decay
                    r
                })
                .collect();

            self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
            
            // Remove from Interact layer
            for record in &promoted {
                self.delete_record(Layer::Interact, &record.id).await?;
            }
        }

        Ok(count)
    }

    async fn promote_insights_to_assets(&self) -> Result<usize> {
        let now = Utc::now();
        let threshold_time = now - Duration::days(self.config.insights_ttl_days as i64);
        
        // Find high-value insights that are old enough
        let candidates = self.find_promotion_candidates(
            Layer::Insights,
            threshold_time,
            self.config.promote_threshold * 1.2, // Higher threshold for assets
        ).await?;

        let count = candidates.len();
        if count > 0 {
            info!("Promoting {} records from Insights to Assets", count);
            
            // Assets are permanent, so we just copy
            let promoted: Vec<_> = candidates.into_iter()
                .map(|mut r| {
                    r.layer = Layer::Assets;
                    r
                })
                .collect();

            self.store.insert_batch(&promoted.iter().collect::<Vec<_>>()).await?;
            
            // Remove from Insights
            for record in &promoted {
                self.delete_record(Layer::Insights, &record.id).await?;
            }
        }

        Ok(count)
    }

    async fn expire_interact_records(&self) -> Result<usize> {
        let expiry_time = Utc::now() - Duration::hours(self.config.interact_ttl_hours as i64 * 2);
        self.store.delete_expired(Layer::Interact, expiry_time).await
    }

    async fn expire_insights_records(&self) -> Result<usize> {
        let expiry_time = Utc::now() - Duration::days(self.config.insights_ttl_days as i64);
        self.store.delete_expired(Layer::Insights, expiry_time).await
    }

    async fn find_promotion_candidates(
        &self,
        layer: Layer,
        before: DateTime<Utc>,
        min_score: f32,
    ) -> Result<Vec<Record>> {
        debug!(
            "Finding promotion candidates in {:?} before {} with min score {}",
            layer, before, min_score
        );
        
        // Use the new get_promotion_candidates method
        let min_access_count = match layer {
            Layer::Interact => 2,  // At least 2 accesses
            Layer::Insights => 5,  // Higher threshold for assets
            Layer::Assets => 0,    // Assets don't get promoted
        };
        
        self.store.get_promotion_candidates(
            layer,
            before,
            min_score,
            min_access_count,
        ).await
    }

    async fn delete_record(&self, layer: Layer, id: &uuid::Uuid) -> Result<()> {
        debug!("Deleting record {} from layer {:?}", id, layer);
        self.store.delete_by_id(id, layer).await?;
        Ok(())
    }

    pub fn calculate_promotion_score(record: &Record) -> f32 {
        let age_hours = (Utc::now() - record.ts).num_hours() as f32;
        let recency_factor = 1.0 / (1.0 + age_hours / 24.0);
        let access_factor = (record.access_count as f32).ln_1p() / 10.0;
        
        record.score * 0.6 + recency_factor * 0.2 + access_factor * 0.2
    }
}

#[derive(Debug, Default)]
pub struct PromotionStats {
    pub interact_to_insights: usize,
    pub insights_to_assets: usize,
    pub expired_interact: usize,
    pub expired_insights: usize,
}