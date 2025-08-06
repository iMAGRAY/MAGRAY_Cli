use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    promotion::{PromotionEngine, PromotionStats},
    ml_promotion::{MLPromotionEngine, MLPromotionStats},
    orchestration::traits::{Coordinator, PromotionCoordinator as PromotionCoordinatorTrait},
};

/// Координатор продвижения записей между слоями
pub struct PromotionCoordinator {
    promotion_engine: Arc<PromotionEngine>,
    ml_promotion: Option<Arc<parking_lot::RwLock<MLPromotionEngine>>>,
    ready: std::sync::atomic::AtomicBool,
}

impl PromotionCoordinator {
    pub fn new(
        promotion_engine: Arc<PromotionEngine>,
        ml_promotion: Option<Arc<parking_lot::RwLock<MLPromotionEngine>>>,
    ) -> Self {
        Self {
            promotion_engine,
            ml_promotion,
            ready: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Coordinator for PromotionCoordinator {
    async fn initialize(&self) -> Result<()> {
        info!("Инициализация PromotionCoordinator");
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "ready": self.is_ready().await,
            "ml_promotion_enabled": self.ml_promotion.is_some(),
            "type": "promotion_coordinator"
        })
    }
}

#[async_trait]
impl PromotionCoordinatorTrait for PromotionCoordinator {
    async fn run_promotion(&self) -> Result<PromotionStats> {
        self.promotion_engine.promote().await
    }
    
    async fn run_ml_promotion(&self) -> Result<Option<MLPromotionStats>> {
        if let Some(ml_engine) = &self.ml_promotion {
            let engine_arc = Arc::clone(ml_engine);
            let stats = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    let engine = engine_arc.read();
                    engine.promote().await
                })
            }).await??;
            Ok(Some(stats))
        } else {
            Ok(None)
        }
    }
    
    async fn should_promote(&self) -> bool {
        // TODO: Реализовать логику определения необходимости promotion
        true
    }
    
    async fn promotion_stats(&self) -> PromotionStats {
        self.promotion_engine.stats()
    }
}