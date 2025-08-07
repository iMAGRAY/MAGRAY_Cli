use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    ml_promotion::{MLPromotionEngine, MLPromotionStats},
    orchestration::traits::{Coordinator, PromotionCoordinator as PromotionCoordinatorTrait},
    promotion::{PromotionEngine, PromotionStats},
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
        self.ready
            .store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "ready": self.is_ready().await,
            "ml_promotion_enabled": self.ml_promotion.is_some(),
            "type": "promotion_coordinator"
        })
    }

    async fn health_check(&self) -> Result<()> {
        // Проверяем готовность
        if !self.is_ready().await {
            return Err(anyhow::anyhow!("PromotionCoordinator не готов"));
        }

        // Проверяем promotion engine
        // Проверяем что можем получить статистику
        let stats = self.promotion_engine.stats();
        if stats.total_time_ms > 10000 {
            // Если promotion занимает слишком много времени, возможно проблема
            return Err(anyhow::anyhow!(
                "Promotion занимает слишком много времени: {}ms",
                stats.total_time_ms
            ));
        }

        // Если ML promotion включен, проверяем его
        if let Some(ml_engine) = &self.ml_promotion {
            let _engine = ml_engine.read();
            // ML engine доступен
        }

        Ok(())
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
                    let mut engine = engine_arc.write();
                    engine.promote().await
                })
            })
            .await??;
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

impl std::fmt::Debug for PromotionCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromotionCoordinator")
            .field(
                "ready",
                &self.ready.load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("promotion_engine", &"<PromotionEngine>")
            .field("ml_promotion_enabled", &self.ml_promotion.is_some())
            .finish()
    }
}
