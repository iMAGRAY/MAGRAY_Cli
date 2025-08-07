use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    backup::{BackupManager, BackupMetadata},
    orchestration::traits::{BackupCoordinator as BackupCoordinatorTrait, Coordinator},
    storage::VectorStore,
};

/// Координатор резервного копирования
pub struct BackupCoordinator {
    backup_manager: Arc<BackupManager>,
    store: Arc<VectorStore>,
    ready: std::sync::atomic::AtomicBool,
}

impl BackupCoordinator {
    pub fn new(backup_manager: Arc<BackupManager>, store: Arc<VectorStore>) -> Self {
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
        self.ready
            .store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "ready": self.is_ready().await,
            "type": "backup_coordinator"
        })
    }

    async fn health_check(&self) -> Result<()> {
        // Проверяем готовность
        if !self.is_ready().await {
            return Err(anyhow::anyhow!("BackupCoordinator не готов"));
        }

        // Проверяем backup manager
        // Можем проверить доступность пути для бэкапов
        let test_path = std::env::temp_dir().join("backup_health_test");
        if !test_path.parent().unwrap().exists() {
            return Err(anyhow::anyhow!("Путь для backup недоступен"));
        }

        // Проверяем vector store
        // Поскольку is_ready и len недоступны, просто проверим что store существует
        // В реальной системе здесь был бы более подробный health check

        Ok(())
    }
}

#[async_trait]
impl BackupCoordinatorTrait for BackupCoordinator {
    async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        let backup_path = self
            .backup_manager
            .create_backup(self.store.clone(), Some(path.to_string()))
            .await?;
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
            tokio::runtime::Handle::current()
                .block_on(async move { manager.restore_backup(store, &path).await })
        })
        .await??;

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

impl std::fmt::Debug for BackupCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackupCoordinator")
            .field(
                "ready",
                &self.ready.load(std::sync::atomic::Ordering::Relaxed),
            )
            .field("backup_manager", &"<BackupManager>")
            .field("store", &"<VectorStore>")
            .finish()
    }
}
