use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    orchestration::{
        EmbeddingCoordinator,
        SearchCoordinator,
        HealthManager,
        PromotionCoordinator,
        ResourceController,
        BackupCoordinator,
        traits::{Coordinator, SearchCoordinator as SearchCoordinatorTrait, 
                PromotionCoordinator as PromotionCoordinatorTrait,
                HealthCoordinator, ResourceCoordinator, BackupCoordinator as BackupCoordinatorTrait},
    },
    types::{Layer, Record, SearchOptions},
    promotion::PromotionStats,
    health::SystemHealthStatus,
    backup::BackupMetadata,
};

/// Главный оркестратор memory системы
// @component: {"k":"C","id":"memory_orchestrator","t":"Main memory system orchestrator","m":{"cur":0,"tgt":95,"u":"%"},"f":["orchestration","coordinator","main"]}
pub struct MemoryOrchestrator {
    /// Координатор embeddings
    pub embedding: Arc<EmbeddingCoordinator>,
    /// Координатор поиска
    pub search: Arc<SearchCoordinator>,
    /// Менеджер здоровья
    pub health: Arc<HealthManager>,
    /// Координатор promotion
    pub promotion: Arc<PromotionCoordinator>,
    /// Контроллер ресурсов
    pub resources: Arc<ResourceController>,
    /// Координатор backup
    pub backup: Arc<BackupCoordinator>,
}

impl MemoryOrchestrator {
    /// Создать новый оркестратор из DI контейнера
    pub fn from_container(container: &crate::di_container::DIContainer) -> Result<Self> {
        debug!("Создание MemoryOrchestrator из DI контейнера");
        
        // Разрешаем координаторы из контейнера
        let embedding = container.resolve::<EmbeddingCoordinator>()?;
        let search = container.resolve::<SearchCoordinator>()?;
        let health = container.resolve::<HealthManager>()?;
        let promotion = container.resolve::<PromotionCoordinator>()?;
        let resources = container.resolve::<ResourceController>()?;
        let backup = container.resolve::<BackupCoordinator>()?;
        
        Ok(Self {
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
        })
    }
    
    /// Инициализировать все координаторы
    pub async fn initialize_all(&self) -> Result<()> {
        info!("Инициализация всех координаторов");
        
        // Инициализируем в правильном порядке
        self.resources.initialize().await?;
        self.health.initialize().await?;
        self.embedding.initialize().await?;
        self.search.initialize().await?;
        self.promotion.initialize().await?;
        self.backup.initialize().await?;
        
        info!("✅ Все координаторы инициализированы");
        Ok(())
    }
    
    /// Проверить готовность всех координаторов
    pub async fn all_ready(&self) -> bool {
        self.embedding.is_ready().await &&
        self.search.is_ready().await &&
        self.health.is_ready().await &&
        self.promotion.is_ready().await &&
        self.resources.is_ready().await &&
        self.backup.is_ready().await
    }
    
    /// Graceful shutdown всех координаторов
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("Остановка всех координаторов");
        
        // Останавливаем в обратном порядке
        if let Err(e) = self.backup.shutdown().await {
            warn!("Ошибка при остановке backup координатора: {}", e);
        }
        
        if let Err(e) = self.promotion.shutdown().await {
            warn!("Ошибка при остановке promotion координатора: {}", e);
        }
        
        if let Err(e) = self.search.shutdown().await {
            warn!("Ошибка при остановке search координатора: {}", e);
        }
        
        if let Err(e) = self.embedding.shutdown().await {
            warn!("Ошибка при остановке embedding координатора: {}", e);
        }
        
        if let Err(e) = self.health.shutdown().await {
            warn!("Ошибка при остановке health менеджера: {}", e);
        }
        
        if let Err(e) = self.resources.shutdown().await {
            warn!("Ошибка при остановке resource контроллера: {}", e);
        }
        
        info!("✅ Все координаторы остановлены");
        Ok(())
    }
    
    /// Получить метрики всех координаторов
    pub async fn all_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "orchestrator": {
                "ready": self.all_ready().await,
                "coordinators": {
                    "embedding": self.embedding.metrics().await,
                    "search": self.search.metrics().await,
                    "health": self.health.metrics().await,
                    "promotion": self.promotion.metrics().await,
                    "resources": self.resources.metrics().await,
                    "backup": self.backup.metrics().await,
                }
            }
        })
    }
    
    // === Удобные методы-обертки для общих операций ===
    
    /// Поиск через оркестратор
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        // Проверяем ресурсы
        if !self.resources.check_resources("search").await? {
            warn!("Недостаточно ресурсов для поиска");
            return Ok(vec![]);
        }
        
        // Выполняем поиск
        self.search.search(query, layer, options).await
    }
    
    /// Запустить promotion
    pub async fn run_promotion(&self) -> Result<PromotionStats> {
        // Проверяем ресурсы
        if !self.resources.check_resources("promotion").await? {
            warn!("Недостаточно ресурсов для promotion");
            return Ok(PromotionStats::default());
        }
        
        self.promotion.run_promotion().await
    }
    
    /// Проверить здоровье системы
    pub async fn check_health(&self) -> Result<SystemHealthStatus> {
        self.health.system_health().await
    }
    
    /// Создать backup
    pub async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        // Проверяем ресурсы
        if !self.resources.check_resources("backup").await? {
            return Err(anyhow::anyhow!("Недостаточно ресурсов для backup"));
        }
        
        self.backup.create_backup(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_orchestrator_lifecycle() -> Result<()> {
        // TODO: Добавить тесты после полной реализации
        Ok(())
    }
}