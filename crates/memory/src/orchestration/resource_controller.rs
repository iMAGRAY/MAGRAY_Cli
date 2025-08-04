use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    resource_manager::{ResourceManager, ResourceUsage},
    orchestration::traits::{Coordinator, ResourceCoordinator},
};

/// Контроллер ресурсов системы
// @component: {"k":"C","id":"resource_controller","t":"Resource management coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","resources","coordinator"]}
pub struct ResourceController {
    resource_manager: Arc<parking_lot::RwLock<ResourceManager>>,
    ready: std::sync::atomic::AtomicBool,
}

impl ResourceController {
    pub fn new(resource_manager: Arc<parking_lot::RwLock<ResourceManager>>) -> Self {
        Self {
            resource_manager,
            ready: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Coordinator for ResourceController {
    async fn initialize(&self) -> Result<()> {
        info!("Инициализация ResourceController");
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
        let usage = self.resource_usage().await;
        serde_json::json!({
            "ready": self.is_ready().await,
            "vectors": {
                "current": usage.current_vectors,
                "max": usage.max_vectors,
                "usage_percent": usage.vector_usage_percent
            },
            "cache": {
                "current_mb": usage.current_cache_size / 1024 / 1024,
                "max_mb": usage.max_cache_size / 1024 / 1024,
                "usage_percent": usage.cache_usage_percent
            },
            "type": "resource_controller"
        })
    }
}

#[async_trait]
impl ResourceCoordinator for ResourceController {
    async fn resource_usage(&self) -> ResourceUsage {
        let manager = self.resource_manager.read();
        manager.current_usage()
    }
    
    async fn check_resources(&self, _operation: &str) -> Result<bool> {
        let mut manager = self.resource_manager.write();
        Ok(!manager.is_memory_pressure())
    }
    
    async fn adapt_limits(&self) -> Result<()> {
        let mut manager = self.resource_manager.write();
        manager.adapt_limits();
        Ok(())
    }
    
    async fn free_resources(&self) -> Result<()> {
        // TODO: Реализовать принудительное освобождение ресурсов
        Ok(())
    }
    
    async fn get_limits(&self) -> (usize, usize) {
        let manager = self.resource_manager.read();
        let limits = manager.get_current_limits();
        (limits.max_vectors, limits.cache_size_bytes / 1024 / 1024)
    }
}