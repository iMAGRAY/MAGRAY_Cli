use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    backup::{BackupManager, BackupMetadata},
    storage::VectorStore,
    orchestration::traits::{Coordinator, BackupCoordinator as BackupCoordinatorTrait},
};

/// Координатор резервного копирования
// @component: {"k":"C","id":"backup_coordinator","t":"Backup orchestration coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","backup","coordinator"]}
pub struct BackupCoordinator {
    backup_manager: Arc<BackupManager>,
    store: Arc<VectorStore>,
    ready: std::sync::atomic::AtomicBool,
}

impl BackupCoordinator {
    pub fn new(
        backup_manager: Arc<BackupManager>,
        store: Arc<VectorStore>,
    ) -> Self {
        Self {
            backup_manager,
            store,
            ready: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Coordinator for BackupCoordinator {
    async fn initialize(&self) -> Result<()> {
        info!("Инициализация BackupCoordinator");
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
            "type": "backup_coordinator"
        })
    }
}

#[async_trait]
impl BackupCoordinatorTrait for BackupCoordinator {
    async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        let backup_path = self.backup_manager.create_backup(self.store.clone(), Some(path.to_string())).await?;
        // Читаем метаданные из созданного backup
        self.backup_manager.read_backup_metadata(&backup_path)
    }
    
    async fn create_incremental_backup(&self, path: &str) -> Result<BackupMetadata> {
        // TODO: Реализовать инкрементальный backup
        self.create_backup(path).await
    }
    
    async fn restore_backup(&self, path: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let manager = Arc::clone(&self.backup_manager);
        let path = path.to_string();
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Handle::current().block_on(async move {
                manager.restore_backup(store, &path).await
            })
        }).await??;
        
        Ok(())
    }
    
    async fn list_backups(&self) -> Result<Vec<BackupMetadata>> {
        let backup_infos = self.backup_manager.list_backups()?;
        Ok(backup_infos.into_iter().map(|info| info.metadata).collect())
    }
    
    async fn verify_backup(&self, path: &str) -> Result<bool> {
        self.backup_manager.verify_backup(path).await
    }
}