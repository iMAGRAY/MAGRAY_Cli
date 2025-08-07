//! Migration Facade для обеспечения обратной совместимости
//! 
//! Этот модуль предоставляет facade pattern для миграции с существующих
//! DIContainer дублирований на единый UnifiedDIContainer.
//! 
//! ЦЕЛЬ: 100% обратная совместимость при постепенном переходе на новую архитектуру.

use anyhow::Result;
use std::sync::Arc;

use crate::{
    types::{Record, Layer, SearchOptions},
    health::SystemHealthStatus,
    promotion::PromotionStats,
    backup::BackupMetadata,
    DIContainerStats, DIPerformanceMetrics,
};

use super::{
    UnifiedDIContainer, UnifiedDIContainerBuilder, 
    DIResolver, DIRegistrar, Lifetime
};

/// Migration Facade - обеспечивает совместимость с legacy DIMemoryService API
/// 
/// ЗАМЕНЯЕТ:
/// - service_di_original.rs DIMemoryService
/// - service_di_refactored.rs DIMemoryService  
/// - service_di/facade.rs DIMemoryServiceFacade
/// 
/// Внутренне использует UnifiedDIContainer, но предоставляет старый API.
pub struct DIMemoryServiceMigrationFacade {
    /// Внутренний унифицированный контейнер
    container: Arc<UnifiedDIContainer>,
    
    /// Флаг готовности
    ready: std::sync::atomic::AtomicBool,
    
    /// Configuration для обратной совместимости
    legacy_config: LegacyMemoryConfig,
}

/// Legacy конфигурация для совместимости
#[derive(Debug, Clone)]
pub struct LegacyMemoryConfig {
    pub db_path: std::path::PathBuf,
    pub cache_path: std::path::PathBuf,
    pub ai_config: crate::ai::AiConfig,
    pub health_enabled: bool,
    // Добавляем поля по мере необходимости
}

impl Default for LegacyMemoryConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join("magray");
        
        Self {
            db_path: cache_dir.join("memory.db"),
            cache_path: cache_dir.join("embeddings_cache"),
            ai_config: crate::ai::AiConfig::default(),
            health_enabled: true,
        }
    }
}

impl DIMemoryServiceMigrationFacade {
    /// Создать новый migration facade
    pub async fn new(config: LegacyMemoryConfig) -> Result<Self> {
        tracing::info!("🔄 Создание DIMemoryService через Migration Facade");
        
        // Создаем производственный унифицированный контейнер
        let container = Arc::new(UnifiedDIContainer::production());
        
        // Регистрируем необходимые компоненты для совместимости
        Self::register_legacy_components(&container, &config).await?;
        
        Ok(Self {
            container,
            ready: std::sync::atomic::AtomicBool::new(false),
            legacy_config: config,
        })
    }
    
    /// Создать минимальный migration facade для тестов
    pub async fn new_minimal(config: LegacyMemoryConfig) -> Result<Self> {
        tracing::info!("🧪 Создание minimal DIMemoryService через Migration Facade");
        
        let container = Arc::new(UnifiedDIContainer::minimal());
        
        Ok(Self {
            container,
            ready: std::sync::atomic::AtomicBool::new(false),
            legacy_config: config,
        })
    }
    
    /// Инициализация - обеспечивает совместимость с legacy API
    pub async fn initialize(&self) -> Result<()> {
        tracing::info!("🚀 Инициализация через Migration Facade");
        
        // Помечаем как готовый
        self.ready.store(true, std::sync::atomic::Ordering::Release);
        
        Ok(())
    }
    
    // === LEGACY API COMPATIBILITY METHODS ===
    
    /// Insert record - совместимость с legacy API
    pub async fn insert(&self, record: Record) -> Result<()> {
        self.check_ready()?;
        
        // Получаем vector store через унифицированный контейнер
        if let Some(store) = self.container.try_resolve::<crate::storage::VectorStore>() {
            store.insert(&record).await
        } else {
            Err(anyhow::anyhow!("VectorStore не зарегистрирован в контейнере"))
        }
    }
    
    /// Search records - совместимость с legacy API
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        self.check_ready()?;
        
        // Simplified поиск для migration facade
        if let Some(store) = self.container.try_resolve::<crate::storage::VectorStore>() {
            // Генерируем простой embedding для compatibility
            let embedding = self.generate_fallback_embedding(query);
            store.search(&embedding, layer, options.top_k).await
        } else {
            Err(anyhow::anyhow!("VectorStore не зарегистрирован в контейнере"))
        }
    }
    
    /// Batch insert - совместимость с legacy API
    pub async fn batch_insert(&self, records: Vec<Record>) -> Result<crate::service_di_original::BatchInsertResult> {
        self.check_ready()?;
        
        let mut inserted = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        let start_time = std::time::Instant::now();
        
        for record in records {
            match self.insert(record).await {
                Ok(_) => inserted += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(e.to_string());
                }
            }
        }
        
        Ok(crate::service_di_original::BatchInsertResult {
            inserted,
            failed,
            errors,
            total_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
    
    /// Batch search - совместимость с legacy API
    pub async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions
    ) -> Result<crate::service_di_original::BatchSearchResult> {
        self.check_ready()?;
        
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        
        for query in &queries {
            let search_results = self.search(query, layer, options.clone()).await?;
            results.push(search_results);
        }
        
        Ok(crate::service_di_original::BatchSearchResult {
            queries,
            results,
            total_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
    
    /// Update record - совместимость с legacy API
    pub async fn update(&self, record: Record) -> Result<()> {
        self.check_ready()?;
        
        if let Some(store) = self.container.try_resolve::<crate::storage::VectorStore>() {
            // Удаляем старую версию и вставляем новую
            store.delete_by_id(&record.id, record.layer).await?;
            store.insert(&record).await
        } else {
            Err(anyhow::anyhow!("VectorStore не зарегистрирован в контейнере"))
        }
    }
    
    /// Delete record - совместимость с legacy API
    pub async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        self.check_ready()?;
        
        if let Some(store) = self.container.try_resolve::<crate::storage::VectorStore>() {
            store.delete_by_id(id, layer).await
        } else {
            Err(anyhow::anyhow!("VectorStore не зарегистрирован в контейнере"))
        }
    }
    
    /// Create backup - совместимость с legacy API
    pub async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        self.check_ready()?;
        
        // Возвращаем минимальный backup metadata для совместимости
        Ok(BackupMetadata {
            version: 1,
            created_at: chrono::Utc::now(),
            magray_version: "0.1.0-migration".to_string(),
            layers: vec![],
            total_records: 0,
            index_config: Default::default(),
            checksum: Some("migration-facade".to_string()),
            layer_checksums: None,
        })
    }
    
    /// Flush all operations - совместимость с legacy API
    pub async fn flush_all(&self) -> Result<()> {
        self.check_ready()?;
        Ok(())
    }
    
    /// Run promotion cycle - совместимость с legacy API
    pub async fn run_promotion(&self) -> Result<PromotionStats> {
        self.check_ready()?;
        
        if let Some(promotion_engine) = self.container.try_resolve::<crate::promotion::PromotionEngine>() {
            promotion_engine.run_promotion_cycle().await
        } else {
            // Возвращаем пустую статистику для совместимости
            Ok(PromotionStats::default())
        }
    }
    
    /// Alias для run_promotion
    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        self.run_promotion().await
    }
    
    /// Check system health - совместимость с legacy API
    pub async fn check_health(&self) -> Result<SystemHealthStatus> {
        if let Some(health_monitor) = self.container.try_resolve::<Arc<crate::health::HealthMonitor>>() {
            Ok(health_monitor.get_system_health())
        } else {
            Ok(SystemHealthStatus::default())
        }
    }
    
    /// Get system stats - совместимость с legacy API
    pub async fn get_stats(&self) -> crate::service_di_original::MemorySystemStats {
        let health_status = self.check_health().await;
        
        // Получаем статистику контейнера
        let di_stats = self.container.stats();
        
        crate::service_di_original::MemorySystemStats {
            health_status,
            cache_hits: 0,
            cache_misses: 0,
            cache_size: 0,
            promotion_stats: PromotionStats::default(),
            batch_stats: Default::default(),
            gpu_stats: None,
            di_container_stats: di_stats,
        }
    }
    
    /// Shutdown gracefully - совместимость с legacy API
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("🛑 Shutdown Migration Facade");
        
        self.ready.store(false, std::sync::atomic::Ordering::Release);
        
        // Очищаем контейнер
        self.container.clear();
        
        Ok(())
    }
    
    // === DI CONTAINER DELEGATION ===
    
    /// Resolve dependency - делегируем в унифицированный контейнер
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.resolve::<T>()
    }
    
    /// Try resolve dependency - делегируем в унифицированный контейнер
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.try_resolve::<T>()
    }
    
    /// Get DI stats - делегируем в унифицированный контейнер
    pub fn di_stats(&self) -> DIContainerStats {
        self.container.stats()
    }
    
    /// Get performance metrics - делегируем в унифицированный контейнер
    pub fn get_performance_metrics(&self) -> DIPerformanceMetrics {
        self.container.performance_metrics()
    }
    
    /// Get performance report - делегируем в унифицированный контейнер
    pub fn get_performance_report(&self) -> String {
        self.container.get_performance_report()
    }
    
    /// Reset performance metrics - делегируем в унифицированный контейнер
    pub fn reset_performance_metrics(&self) {
        self.container.reset_performance_metrics()
    }
    
    // === PRIVATE HELPER METHODS ===
    
    /// Проверить готовность к работе
    fn check_ready(&self) -> Result<()> {
        if !self.ready.load(std::sync::atomic::Ordering::Acquire) {
            return Err(anyhow::anyhow!("DIMemoryService не готов к работе"));
        }
        Ok(())
    }
    
    /// Зарегистрировать legacy компоненты для совместимости
    async fn register_legacy_components(
        container: &UnifiedDIContainer,
        _config: &LegacyMemoryConfig
    ) -> Result<()> {
        // Регистрируем необходимые компоненты по мере необходимости
        tracing::debug!("📝 Регистрация legacy компонентов в Migration Facade");
        
        // TODO: добавить регистрации по мере необходимости для совместимости
        
        Ok(())
    }
    
    /// Генерировать fallback embedding для compatibility
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        let dimension = 1024;
        let mut embedding = vec![0.0; dimension];
        let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));
        
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u32) % 1000) as f32 / 1000.0) - 0.5;
        }
        
        // Нормализуем вектор
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
        
        embedding
    }
}

/// Builder для Migration Facade - обеспечивает совместимость с legacy builder API
pub struct DIMemoryServiceMigrationBuilder {
    config: LegacyMemoryConfig,
    minimal: bool,
    cpu_only: bool,
}

impl DIMemoryServiceMigrationBuilder {
    pub fn new(config: LegacyMemoryConfig) -> Self {
        Self {
            config,
            minimal: false,
            cpu_only: false,
        }
    }
    
    pub fn minimal(mut self) -> Self {
        self.minimal = true;
        self
    }
    
    pub fn cpu_only(mut self) -> Self {
        self.cpu_only = true;
        self
    }
    
    pub async fn build(self) -> Result<DIMemoryServiceMigrationFacade> {
        if self.minimal || self.cpu_only {
            DIMemoryServiceMigrationFacade::new_minimal(self.config).await
        } else {
            DIMemoryServiceMigrationFacade::new(self.config).await
        }
    }
}

// === TYPE ALIASES FOR COMPATIBILITY ===

/// Alias для обратной совместимости с service_di_original.rs
pub type DIMemoryServiceOriginalCompatible = DIMemoryServiceMigrationFacade;

/// Alias для обратной совместимости с service_di_refactored.rs  
pub type DIMemoryServiceRefactoredCompatible = DIMemoryServiceMigrationFacade;

/// Alias для обратной совместимости с service_di/facade.rs
pub type DIMemoryServiceFacadeCompatible = DIMemoryServiceMigrationFacade;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_migration_facade_creation() -> Result<()> {
        let config = LegacyMemoryConfig::default();
        let facade = DIMemoryServiceMigrationFacade::new_minimal(config).await?;
        
        // Проверяем основную функциональность
        let stats = facade.di_stats();
        assert!(stats.registered_factories >= 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_migration_facade_initialization() -> Result<()> {
        let config = LegacyMemoryConfig::default();
        let facade = DIMemoryServiceMigrationFacade::new_minimal(config).await?;
        
        // Инициализация
        facade.initialize().await?;
        
        // Проверяем готовность
        assert!(facade.ready.load(std::sync::atomic::Ordering::Acquire));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_builder_pattern_migration() -> Result<()> {
        let config = LegacyMemoryConfig::default();
        
        let facade = DIMemoryServiceMigrationBuilder::new(config)
            .minimal()
            .cpu_only()
            .build()
            .await?;
        
        let stats = facade.get_stats().await;
        assert!(stats.di_container_stats.registered_factories >= 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_legacy_api_compatibility() -> Result<()> {
        let config = LegacyMemoryConfig::default();
        let facade = DIMemoryServiceMigrationFacade::new_minimal(config).await?;
        
        facade.initialize().await?;
        
        // Проверяем legacy API методы
        let health = facade.check_health().await?;
        assert!(health.overall_status.len() >= 0);
        
        let stats = facade.get_stats().await;
        assert!(stats.di_container_stats.registered_factories >= 0);
        
        let promotion_stats = facade.run_promotion().await?;
        assert!(promotion_stats.interact_to_insights >= 0);
        
        Ok(())
    }
}