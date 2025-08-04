use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
<<<<<<< HEAD
    MemoryService, DIMemoryService, Layer, Record,
    types::SearchOptions as CoreSearchOptions,
    health::{HealthStatus, ComponentType, SystemHealthStatus},
    promotion::PromotionStats,
};

/// Trait для абстракции над различными реализациями memory service
pub trait MemoryServiceTrait: Send + Sync {
    /// Поиск записей (упрощенная версия без async проблем)
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>>;
    
    /// Запустить цикл продвижения памяти (упрощенная версия)
    fn run_promotion_sync(&self) -> Result<PromotionStats>;
    
    /// Получить статистику здоровья системы
    fn get_system_health(&self) -> SystemHealthStatus;
    
    /// Получить статистику кэша (hits, misses, total)
    fn cache_stats(&self) -> (u64, u64, u64);
    
    /// Добавить запись - простая версия
    fn remember_sync(&self, text: String, layer: Layer) -> Result<Uuid>;
}

/// Реализация trait для legacy MemoryService
impl MemoryServiceTrait for MemoryService {
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
        // Используем runtime blocking для sync версии
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            let builder = self.search(query)
                .with_layers(&[layer])
                .top_k(top_k);
            builder.execute().await
        })
    }
    
    fn run_promotion_sync(&self) -> Result<PromotionStats> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            self.run_promotion_cycle().await
        })
    }
    
    fn get_system_health(&self) -> SystemHealthStatus {
        self.get_system_health()
    }
    
    fn cache_stats(&self) -> (u64, u64, u64) {
        self.cache_stats()
    }
    
    fn remember_sync(&self, text: String, layer: Layer) -> Result<Uuid> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            let record = Record {
                id: Uuid::new_v4(),
                text,
                embedding: vec![],
                layer,
                kind: "note".to_string(),
                tags: vec![],
                project: "default".to_string(),
                session: Uuid::new_v4().to_string(),
                score: 0.5,
                access_count: 1,
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
            };
            
            self.insert(record.clone()).await?;
            Ok(record.id)
        })
    }
}

/// Реализация trait для DIMemoryService
impl MemoryServiceTrait for DIMemoryService {
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            let options = CoreSearchOptions {
                top_k,
                ..Default::default()
            };
            self.search(query, layer, options).await
        })
    }
    
    fn run_promotion_sync(&self) -> Result<PromotionStats> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            self.run_promotion().await
        })
    }
    
    fn get_system_health(&self) -> SystemHealthStatus {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.block_on(async { 
                    self.check_health().await.unwrap_or_else(|_| SystemHealthStatus::default())
                })
            }
            Err(_) => SystemHealthStatus::default()
        }
    }
    
    fn cache_stats(&self) -> (u64, u64, u64) {
        // Для DIMemoryService возвращаем базовую статистику
        // так как полная реализация требует async доступа к координаторам
        (0, 0, 0) // hits, misses, total
    }
    
    fn remember_sync(&self, text: String, layer: Layer) -> Result<Uuid> {
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|_| anyhow::anyhow!("No tokio runtime available"))?;
        
        rt.block_on(async {
            let record = Record {
                id: Uuid::new_v4(),
                text,
                embedding: vec![],
                layer,
                kind: "note".to_string(),
                tags: vec![],
                project: "default".to_string(),
                session: Uuid::new_v4().to_string(),
                score: 0.5,
                access_count: 1,
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
            };
            
            self.insert(record.clone()).await?;
            Ok(record.id)
        })
    }
}

/// Единый API интерфейс для MAGRAY CLI
/// Предоставляет упрощенный доступ ко всем функциям системы памяти
pub struct UnifiedMemoryAPI {
    service: Arc<dyn MemoryServiceTrait>,
}

impl UnifiedMemoryAPI {
    /// Создать новый API интерфейс с legacy service
=======
    MemoryService, Layer, Record,
    health::{HealthStatus, ComponentType},
};

/// Единый API интерфейс для MAGRAY CLI
/// Предоставляет упрощенный доступ ко всем функциям системы памяти
pub struct UnifiedMemoryAPI {
    service: Arc<MemoryService>,
}

impl UnifiedMemoryAPI {
    /// Создать новый API интерфейс
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    pub fn new(service: Arc<MemoryService>) -> Self {
        Self { service }
    }
    
<<<<<<< HEAD
    /// Создать новый API интерфейс с DI service
    pub fn new_di(service: Arc<DIMemoryService>) -> Self {
        Self { service }
    }
    
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    // ========== ОСНОВНЫЕ ОПЕРАЦИИ ==========
    
    /// Сохранить информацию в память
    pub async fn remember(&self, text: String, context: MemoryContext) -> Result<Uuid> {
<<<<<<< HEAD
        let layer = context.layer.unwrap_or(Layer::Interact);
        self.service.remember_sync(text, layer)
=======
        let record = Record {
            id: Uuid::new_v4(),
            text,
            embedding: vec![], // Будет заполнено автоматически
            layer: context.layer.unwrap_or(Layer::Interact),
            kind: context.kind,
            tags: context.tags,
            project: context.project.unwrap_or_else(|| "default".to_string()),
            session: context.session.unwrap_or_else(|| Uuid::new_v4().to_string()),
            score: 0.5, // Начальный score
            access_count: 1,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
        };
        
        self.service.insert(record.clone()).await?;
        Ok(record.id)
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    }
    
    /// Найти релевантную информацию
    pub async fn recall(&self, query: &str, options: SearchOptions) -> Result<Vec<MemoryResult>> {
<<<<<<< HEAD
        // Поиск по всем указанным слоям или всем слоям если не указано
        let layers_to_search = options.layers.unwrap_or_else(|| vec![Layer::Interact, Layer::Insights, Layer::Assets]);
        let limit = options.limit.unwrap_or(10);
        
        let mut all_results = Vec::new();
        
        for layer in layers_to_search {
            let layer_results = self.service.search_sync(query, layer, limit)?;
            all_results.extend(layer_results);
        }
        
        // Сортируем по релевантности и берем топ результатов
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(limit);
        
        Ok(all_results.into_iter()
=======
        let mut builder = self.service.search(query);
        
        if let Some(layers) = options.layers {
            builder = builder.with_layers(&layers);
        }
        
        if let Some(project) = options.project {
            builder = builder.with_project(&project);
        }
        
        if let Some(tags) = options.tags {
            builder = builder.with_tags(tags);
        }
        
        let results = builder
            .top_k(options.limit.unwrap_or(10))
            .execute()
            .await?;
        
        Ok(results.into_iter()
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            .map(|r| MemoryResult {
                id: r.id,
                text: r.text,
                layer: r.layer,
                kind: r.kind,
                tags: r.tags,
                project: r.project,
                relevance_score: r.score,
                created_at: r.ts,
                access_count: r.access_count,
            })
            .collect())
    }
    
    /// Получить конкретную запись по ID
    pub async fn get(&self, id: Uuid) -> Result<Option<MemoryResult>> {
<<<<<<< HEAD
        // Упрощенная реализация - поиск не поддерживается в sync trait
        let _ = id;
=======
        // Проверяем по всем слоям
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            if let Ok(Some(record)) = self.service.get_store().get_by_id(&id, layer).await {
                return Ok(Some(MemoryResult {
                    id: record.id,
                    text: record.text,
                    layer: record.layer,
                    kind: record.kind,
                    tags: record.tags,
                    project: record.project,
                    relevance_score: record.score,
                    created_at: record.ts,
                    access_count: record.access_count,
                }));
            }
        }
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        Ok(None)
    }
    
    /// Удалить запись
    pub async fn forget(&self, id: Uuid) -> Result<bool> {
<<<<<<< HEAD
        // Упрощенная реализация - удаление не поддерживается в sync trait
        let _ = id;
=======
        // Пытаемся удалить из всех слоев
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            if self.service.get_store().delete_by_id(&id, layer).await.is_ok() {
                return Ok(true);
            }
        }
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        Ok(false)
    }
    
    // ========== УПРАВЛЕНИЕ СИСТЕМОЙ ==========
    
    /// Запустить цикл продвижения памяти
    pub async fn optimize_memory(&self) -> Result<OptimizationResult> {
<<<<<<< HEAD
        let stats = self.service.run_promotion_sync()?;
=======
        let stats = self.service.run_promotion_cycle().await?;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        
        Ok(OptimizationResult {
            promoted_to_insights: stats.interact_to_insights,
            promoted_to_assets: stats.insights_to_assets,
            expired_interact: stats.expired_interact,
            expired_insights: stats.expired_insights,
            total_time_ms: stats.total_time_ms,
            index_update_time_ms: stats.index_update_time_ms,
            promotion_time_ms: stats.promotion_time_ms,
            cleanup_time_ms: stats.cleanup_time_ms,
        })
    }
    
    /// Получить состояние здоровья системы
    pub async fn health_check(&self) -> Result<SystemHealth> {
        let health = self.service.get_system_health();
        
        Ok(SystemHealth {
            status: match health.overall_status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded => "degraded",
                HealthStatus::Unhealthy => "unhealthy",
                HealthStatus::Down => "down",
            },
            uptime_seconds: health.uptime_seconds,
            component_count: health.component_statuses.len(),
            alert_count: health.active_alerts.len(),
            components: health.component_statuses.into_iter()
                .map(|(comp, status)| {
                    let comp_name = match comp {
                        ComponentType::VectorStore => "vector_store",
                        ComponentType::EmbeddingService => "embedding_service",
                        ComponentType::Cache => "cache",
                        ComponentType::PromotionEngine => "promotion_engine",
                        ComponentType::RerankingService => "reranking_service",
                        ComponentType::Database => "database",
                        ComponentType::Memory => "memory",
                    };
                    
                    let status_str = match status {
                        HealthStatus::Healthy => "healthy",
                        HealthStatus::Degraded => "degraded",
                        HealthStatus::Unhealthy => "unhealthy",
                        HealthStatus::Down => "down",
                    };
                    
                    (comp_name.to_string(), status_str.to_string())
                })
                .collect(),
        })
    }
    
    /// Выполнить полную проверку здоровья
    pub async fn full_health_check(&self) -> Result<DetailedHealth> {
<<<<<<< HEAD
        let result = self.service.get_system_health();
=======
        let result = self.service.run_health_check().await?;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        
        Ok(DetailedHealth {
            overall_status: match result.overall_status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded => "degraded",
                HealthStatus::Unhealthy => "unhealthy",
                HealthStatus::Down => "down",
            },
            uptime_seconds: result.uptime_seconds,
            alerts: result.active_alerts.into_iter()
                .map(|alert| HealthAlert {
                    severity: format!("{:?}", alert.severity),
                    component: format!("{:?}", alert.component),
                    title: alert.title,
                    message: alert.description,
                })
                .collect(),
            metrics: result.metrics_summary,
        })
    }
    
    // ========== СТАТИСТИКА ==========
    
    /// Получить общую статистику системы
    pub async fn get_stats(&self) -> Result<SystemStats> {
<<<<<<< HEAD
        let (cache_hits, _cache_misses, cache_total) = self.service.cache_stats();
        
        // Создаем базовую статистику
        let mut layer_counts = std::collections::HashMap::new();
        layer_counts.insert("interact".to_string(), 0);
        layer_counts.insert("insights".to_string(), 0);
        layer_counts.insert("assets".to_string(), 0);
        
        Ok(SystemStats {
            total_records: 0,
            layer_distribution: layer_counts,
            index_sizes: IndexSizes {
                time_indices: 0,
                score_indices: 0,
            },
            cache_stats: CacheStats {
                hit_rate: if cache_total > 0 { cache_hits as f32 / cache_total as f32 } else { 0.0 },
                size_bytes: 0,
                entries: cache_total as usize,
            },
            // Базовая статистика по слоям
            interact_count: 0,
            interact_size: 0,
            interact_avg_access: 0.0,
            insights_count: 0,
            insights_size: 0,
            insights_avg_access: 0.0,
            assets_count: 0,
            assets_size: 0,
            assets_avg_access: 0.0,
            // Статистика продвижения
=======
        let perf_stats = self.service.get_promotion_performance_stats().await?;
        let (cache_hits, _cache_misses, cache_total) = self.service.cache_stats();
        
        // Подсчитываем записи по слоям
        let mut layer_counts = std::collections::HashMap::new();
        layer_counts.insert("interact".to_string(), perf_stats.interact_time_index_size);
        layer_counts.insert("insights".to_string(), perf_stats.insights_time_index_size);
        layer_counts.insert("assets".to_string(), perf_stats.assets_time_index_size);
        
        // Получаем размеры в байтах (примерные)
        let avg_record_size = 1024; // Примерно 1KB на запись
        
        // Получаем реальную статистику доступа для каждого слоя
        let interact_avg_access = self.service.get_layer_average_access(Layer::Interact).await.unwrap_or(0.0);
        let insights_avg_access = self.service.get_layer_average_access(Layer::Insights).await.unwrap_or(0.0);
        let assets_avg_access = self.service.get_layer_average_access(Layer::Assets).await.unwrap_or(0.0);
        
        Ok(SystemStats {
            total_records: perf_stats.interact_time_index_size + 
                          perf_stats.insights_time_index_size + 
                          perf_stats.assets_time_index_size,
            layer_distribution: layer_counts,
            index_sizes: IndexSizes {
                time_indices: perf_stats.interact_time_index_size + 
                             perf_stats.insights_time_index_size + 
                             perf_stats.assets_time_index_size,
                score_indices: perf_stats.interact_score_index_size + 
                              perf_stats.insights_score_index_size + 
                              perf_stats.assets_score_index_size,
            },
            cache_stats: CacheStats {
                hit_rate: if cache_total > 0 { cache_hits as f32 / cache_total as f32 } else { 0.0 },
                size_bytes: self.service.get_cache_size().await.unwrap_or(0),
                entries: cache_total as usize,
            },
            // Статистика по слоям с реальными данными доступа
            interact_count: perf_stats.interact_time_index_size,
            interact_size: perf_stats.interact_time_index_size * avg_record_size,
            interact_avg_access,
            insights_count: perf_stats.insights_time_index_size,
            insights_size: perf_stats.insights_time_index_size * avg_record_size,
            insights_avg_access,
            assets_count: perf_stats.assets_time_index_size,
            assets_size: perf_stats.assets_time_index_size * avg_record_size,
            assets_avg_access,
            // Статистика продвижения (примерные значения)
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            interact_to_insights: 0,
            insights_to_assets: 0,
            expired_interact: 0,
            expired_insights: 0,
            total_time_ms: 0,
        })
    }
<<<<<<< HEAD
    
    /// Получить статистику кэша (hits, misses, total)
    pub fn cache_stats(&self) -> (u64, u64, u64) {
        self.service.cache_stats()
    }
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

// ========== ТИПЫ ДЛЯ API ==========

/// Контекст для сохранения в память
#[derive(Debug, Clone)]
pub struct MemoryContext {
    pub kind: String,
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub session: Option<String>,
    pub layer: Option<Layer>,
}

impl Default for MemoryContext {
    fn default() -> Self {
        Self {
            kind: "general".to_string(),
            tags: vec![],
            project: None,
            session: None,
            layer: None,
        }
    }
}

/// Опции поиска
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub layers: Option<Vec<Layer>>,
    pub project: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<usize>,
}

/// Результат поиска в памяти
#[derive(Debug, Clone)]
pub struct MemoryResult {
    pub id: Uuid,
    pub text: String,
    pub layer: Layer,
    pub kind: String,
    pub tags: Vec<String>,
    pub project: String,
    pub relevance_score: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u32,
}

/// Результат оптимизации памяти
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub promoted_to_insights: usize,
    pub promoted_to_assets: usize,
    pub expired_interact: usize,
    pub expired_insights: usize,
    pub total_time_ms: u64,
    pub index_update_time_ms: u64,
    pub promotion_time_ms: u64,
    pub cleanup_time_ms: u64,
}

/// Состояние здоровья системы
#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub status: &'static str,
    pub uptime_seconds: u64,
    pub component_count: usize,
    pub alert_count: usize,
    pub components: Vec<(String, String)>,
}

/// Детальная информация о здоровье
#[derive(Debug, Clone)]
pub struct DetailedHealth {
    pub overall_status: &'static str,
    pub uptime_seconds: u64,
    pub alerts: Vec<HealthAlert>,
    pub metrics: std::collections::HashMap<String, f64>,
}

/// Информация об alert
#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub severity: String,
    pub component: String,
    pub title: String,
    pub message: String,
}

/// Общая статистика системы
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_records: usize,
    pub layer_distribution: std::collections::HashMap<String, usize>,
    pub index_sizes: IndexSizes,
    pub cache_stats: CacheStats,
    // Статистика по слоям
    pub interact_count: usize,
    pub interact_size: usize,
    pub interact_avg_access: f32,
    pub insights_count: usize,
    pub insights_size: usize,
    pub insights_avg_access: f32,
    pub assets_count: usize,
    pub assets_size: usize,
    pub assets_avg_access: f32,
    // Статистика продвижения
    pub interact_to_insights: usize,
    pub insights_to_assets: usize,
    pub expired_interact: usize,
    pub expired_insights: usize,
    pub total_time_ms: u64,
}

/// Размеры индексов
#[derive(Debug, Clone)]
pub struct IndexSizes {
    pub time_indices: usize,
    pub score_indices: usize,
}

/// Статистика кэша
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hit_rate: f32,
    pub size_bytes: usize,
    pub entries: usize,
}

// ========== BUILDER PATTERN ДЛЯ УДОБСТВА ==========

impl MemoryContext {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            ..Default::default()
        }
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }
    
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }
    
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = Some(layer);
        self
    }
}

impl SearchOptions {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn in_layers(mut self, layers: Vec<Layer>) -> Self {
        self.layers = Some(layers);
        self
    }
    
    pub fn in_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }
    
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}