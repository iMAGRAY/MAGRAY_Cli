use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    di::UnifiedContainer as DIMemoryService,
    health::{ComponentType, HealthStatus, SystemHealthStatus},
    // promotion::PromotionStats,
    // services::RefactoredDIMemoryService,
    Layer, Record,
};
use common::event_bus::{EventBus, Topic};
use once_cell::sync::Lazy;

#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
use crate::promotion::PromotionStats;
#[cfg(any(feature = "minimal", not(feature = "persistence")))]
#[derive(Default, Clone, Copy)]
pub struct PromotionStats {
    pub interact_to_insights: usize,
    pub insights_to_assets: usize,
    pub expired_interact: usize,
    pub expired_insights: usize,
    pub total_time_ms: u64,
    pub index_update_time_ms: u64,
    pub promotion_time_ms: u64,
    pub cleanup_time_ms: u64,
}

// ===== Simple in-memory engine for CPU profile =====
#[cfg(feature = "embeddings")]
mod simple_engine {
    use super::*;
    use ai::{CpuEmbeddingService, EmbeddingConfig};
    #[cfg(feature = "reranking")]
    use ai::{OptimizedQwen3RerankerService, RerankBatch, RerankingConfig};
    use parking_lot::RwLock;
    use std::sync::OnceLock;
    use std::io::Write;

    // EventBus for memory events (recorded globally)
    #[derive(Debug, Clone)]
    pub enum MemoryEventPayload {
        Remember { id: uuid::Uuid, layer: Layer },
        Search { query: String, layer: Layer, results: usize },
    }

    pub static MEMORY_EVENT_BUS: Lazy<EventBus<MemoryEventPayload>> = Lazy::new(|| EventBus::new(1024, std::time::Duration::from_millis(250)));

    #[derive(Clone)]
    struct StoredRecord {
        record: Record,
        embedding: Vec<f32>,
    }

    pub struct SimpleMemoryEngine {
        embedding_service: Option<CpuEmbeddingService>,
        #[cfg(feature = "reranking")]
        reranker: Option<OptimizedQwen3RerankerService>,
        records: RwLock<Vec<StoredRecord>>,
        embedding_dim: usize,
        store_path: Option<std::path::PathBuf>,
    }

    static ENGINE: OnceLock<SimpleMemoryEngine> = OnceLock::new();

    impl SimpleMemoryEngine {
        fn init() -> &'static SimpleMemoryEngine {
            ENGINE.get_or_init(|| {
                // Try to create real embedding service
                let model_name = std::env::var("MAGRAY_EMBED_MODEL").unwrap_or_else(|_| "qwen3emb".to_string());
                let cfg = EmbeddingConfig {
                    model_name: model_name.clone(),
                    max_length: 512,
                    batch_size: 16,
                    use_gpu: false,
                    gpu_config: None,
                    embedding_dim: Some(1024),
                };
                let embedding_service = if ai::should_disable_ort() { None } else { CpuEmbeddingService::new(cfg).ok() };
                let embedding_dim = 1024;

                #[cfg(feature = "reranking")]
                let reranker = {
                    let disable_rerank = std::env::var("MAGRAY_DISABLE_RERANK")
                        .map(|v| matches!(v.to_lowercase().as_str(), "1"|"true"|"yes"|"y"))
                        .unwrap_or(false);
                    if disable_rerank { None } else {
                    let rcfg = RerankingConfig {
                        model_name: "qwen3_reranker".to_string(),
                        batch_size: 32,
                        max_length: 512,
                        use_gpu: false,
                        gpu_config: None,
                    };
                    OptimizedQwen3RerankerService::new_with_config(rcfg).ok()
                    }
                };
                let store_path = Some(std::env::var("MAGRAY_MEMORY_FILE").unwrap_or_else(|_| "magray_memory.jsonl".to_string()))
                    .map(std::path::PathBuf::from);

                // Preload existing JSONL if present
                let mut initial_records: Vec<StoredRecord> = Vec::new();
                if let Some(path) = store_path.as_ref() {
                    if let Ok(text) = std::fs::read_to_string(path) {
                        for line in text.lines() {
                            if line.trim().is_empty() { continue; }
                            if let Ok(mut rec) = serde_json::from_str::<Record>(line) {
                                // Compute embedding
                                let emb = match &embedding_service {
                                    Some(svc) => svc.embed(&rec.text).map(|e| e.embedding).unwrap_or_else(|_| Self::mock_embed_static(&rec.text, embedding_dim)),
                                    None => Self::mock_embed_static(&rec.text, embedding_dim),
                                };
                                rec.score = 0.0;
                                initial_records.push(StoredRecord { record: rec, embedding: emb });
                            }
                        }
                    }
                }

                SimpleMemoryEngine {
                    embedding_service,
                    #[cfg(feature = "reranking")]
                    reranker,
                    records: RwLock::new(initial_records),
                    embedding_dim,
                    store_path,
                }
            })
        }

        pub fn health_status() -> SystemHealthStatus {
            let engine = Self::init();
            let mut status = SystemHealthStatus::default();
            status.overall_status = if engine.embedding_service.is_some() {
                HealthStatus::Healthy
            } else {
                HealthStatus::Degraded
            };
            status
        }

        pub fn insert(&self, mut record: Record) -> Result<Uuid> {
            let emb = match &self.embedding_service {
                Some(svc) => svc.embed(&record.text)?.embedding,
                None => self.mock_embed(&record.text),
            };
            // store score placeholder
            record.score = 0.0;
            self.records.write().push(StoredRecord { record: record.clone(), embedding: emb });
            // append to JSONL store for cross-process persistence
            if let Some(path) = self.store_path.as_ref() {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                    if let Ok(line) = serde_json::to_string(&record) { let _ = writeln!(f, "{}", line); }
                }
            }
            // fire event (non-blocking publish with timeout inside)
            let payload = MemoryEventPayload::Remember { id: record.id, layer: record.layer };
            tokio::spawn(MEMORY_EVENT_BUS.publish(common::topics::TOPIC_MEMORY_UPSERT, payload));
            // also forward to global JSON bus for cross-crate observability
            let json_evt = serde_json::json!({"id": record.id, "layer": format!("{:?}", record.layer)});
            tokio::spawn(common::events::publish(common::topics::TOPIC_MEMORY_UPSERT, json_evt));
            Ok(record.id)
        }

        pub fn search(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
            let query_emb = match &self.embedding_service {
                Some(svc) => svc.embed(query)?.embedding,
                None => self.mock_embed(query),
            };

            // First-stage ANN by cosine on embeddings
            let mut scored: Vec<(f32, Record)> = self
                .records
                .read()
                .iter()
                .filter(|sr| sr.record.layer == layer)
                .map(|sr| (self.cosine(&query_emb, &sr.embedding), sr.record.clone()))
                .collect();

            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            // Take a wider beam for reranking if available
            let beam = top_k.max(16).min(scored.len());
            scored.truncate(beam);

            // Optional second-stage reranking
            #[cfg(feature = "reranking")]
            if let Some(reranker) = &self.reranker {
                let documents: Vec<String> = scored.iter().map(|(_, r)| r.text.clone()).collect();
                if !documents.is_empty() {
                    let batch = RerankBatch { query: query.to_string(), documents, top_k: Some(top_k) };
                    if let Ok(reranked) = reranker.rerank_batch(&batch) {
                        // Map back according to returned order (top_k applied inside)
                        let mut new_order = Vec::with_capacity(reranked.results.len());
                        for item in reranked.results {
                            // item.index corresponds to position in 'documents'
                            if let Some((_s, r)) = scored.get(item.index) {
                                let mut rr = r.clone();
                                rr.score = item.score;
                                new_order.push((item.score, rr));
                            }
                        }
                        if !new_order.is_empty() {
                            scored = new_order;
                        }
                    }
                }
            }

            for (s, r) in scored.iter_mut() {
                r.score = *s;
            }
            // Return top_k finally
            let mut out: Vec<Record> = scored.into_iter().map(|(_, r)| r).collect();
            let returned = out.len().min(top_k);
            out.truncate(top_k);
            // fire event with summary
            let payload = MemoryEventPayload::Search { query: query.to_string(), layer, results: returned };
            tokio::spawn(MEMORY_EVENT_BUS.publish(common::topics::TOPIC_MEMORY_SEARCH, payload));
            // also forward to global JSON bus
            let json_evt = serde_json::json!({"query": query, "layer": format!("{:?}", layer), "results": returned});
            tokio::spawn(common::events::publish(common::topics::TOPIC_MEMORY_SEARCH, json_evt));
            Ok(out)
        }

        // Export/import helpers for backup/restore
        pub fn export_records(&self) -> Vec<Record> {
            self.records
                .read()
                .iter()
                .map(|sr| sr.record.clone())
                .collect()
        }

        pub fn import_records(&self, records: &[Record]) -> Result<usize> {
            let mut inserted = 0usize;
            for mut rec in records.iter().cloned() {
                // Recompute embedding to keep index consistent
                let emb = match &self.embedding_service {
                    Some(svc) => svc.embed(&rec.text)?.embedding,
                    None => self.mock_embed(&rec.text),
                };
                // Ensure score is sane
                rec.score = 0.0;
                self.records.write().push(StoredRecord { record: rec, embedding: emb });
                inserted += 1;
            }
            Ok(inserted)
        }

        #[cfg(test)]
        pub fn clear_all(&self) {
            self.records.write().clear();
        }

        fn cosine(&self, a: &[f32], b: &[f32]) -> f32 {
            let mut dot = 0.0f32;
            let mut na = 0.0f32;
            let mut nb = 0.0f32;
            let len = a.len().min(b.len());
            for i in 0..len {
                let av = a[i];
                let bv = b[i];
                dot += av * bv;
                na += av * av;
                nb += bv * bv;
            }
            if na <= 1e-8 || nb <= 1e-8 { return 0.0; }
            dot / (na.sqrt() * nb.sqrt())
        }

        fn mock_embed(&self, text: &str) -> Vec<f32> {
            // Simple consistent hash-based embedding
            let mut v = vec![0f32; self.embedding_dim];
            let mut idx = 0usize;
            for token in text.split_whitespace() {
                let mut h: u64 = 1469598103934665603;
                for b in token.as_bytes() { h = h ^ (*b as u64); h = h.wrapping_mul(1099511628211); }
                let pos = (h as usize) % self.embedding_dim;
                v[pos] += 1.0;
                idx += 1;
                if idx >= self.embedding_dim { break; }
            }
            // L2 normalize
            let mut norm = 0.0f32; for x in &v { norm += *x * *x; }
            if norm > 0.0 { let inv = 1.0 / norm.sqrt(); for x in &mut v { *x *= inv; } }
            v
        }

        fn mock_embed_static(text: &str, dim: usize) -> Vec<f32> {
            let mut v = vec![0f32; dim];
            let mut idx = 0usize;
            for token in text.split_whitespace() {
                let mut h: u64 = 1469598103934665603;
                for b in token.as_bytes() { h = h ^ (*b as u64); h = h.wrapping_mul(1099511628211); }
                let pos = (h as usize) % dim;
                v[pos] += 1.0;
                idx += 1;
                if idx >= dim { break; }
            }
            let mut norm = 0.0f32; for x in &v { norm += *x * *x; }
            if norm > 0.0 { let inv = 1.0 / norm.sqrt(); for x in &mut v { *x *= inv; } }
            v
        }
    }

    // Public facade used by trait impl
    pub(super) fn engine() -> &'static SimpleMemoryEngine { SimpleMemoryEngine::init() }
}

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

// Legacy MemoryService реализация удалена - используем только DIMemoryService

/// Реализация trait для DIMemoryService
impl MemoryServiceTrait for DIMemoryService {
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
        #[cfg(feature = "embeddings")]
        {
            let engine = simple_engine::engine();
            engine.search(query, layer, top_k)
        }
        #[cfg(not(feature = "embeddings"))]
        {
            Ok(vec![])
        }
    }

    fn run_promotion_sync(&self) -> Result<PromotionStats> {
        Ok(PromotionStats::default())
    }

    fn get_system_health(&self) -> SystemHealthStatus {
        #[cfg(feature = "embeddings")]
        { simple_engine::SimpleMemoryEngine::health_status() }
        #[cfg(not(feature = "embeddings"))]
        { SystemHealthStatus::default() }
    }

    fn cache_stats(&self) -> (u64, u64, u64) {
        (0, 0, 0)
    }

    fn remember_sync(&self, text: String, layer: Layer) -> Result<Uuid> {
        #[cfg(feature = "embeddings")]
        {
            use chrono::Utc;
            let id = Uuid::new_v4();
            let record = Record {
                id,
                text,
                embedding: vec![],
                layer,
                kind: "note".to_string(),
                tags: vec![],
                project: "magray".to_string(),
                session: "default".to_string(),
                ts: Utc::now(),
                score: 0.0,
                access_count: 0,
                last_access: Utc::now(),
            };
            let engine = simple_engine::engine();
            engine.insert(record)
        }
        #[cfg(not(feature = "embeddings"))]
        {
            Ok(Uuid::new_v4())
        }
    }
}

/// Единый API интерфейс для MAGRAY CLI
/// Предоставляет упрощенный доступ ко всем функциям системы памяти
pub struct UnifiedMemoryAPI {
    service: Arc<dyn MemoryServiceTrait>,
}

impl UnifiedMemoryAPI {
    /// Создать новый API интерфейс с refactored DI service
    pub fn new(service: Arc<dyn MemoryServiceTrait>) -> Self {
        Self { service }
    }

    /// Найти релевантную информацию с timeout защитой
    pub async fn recall(&self, query: &str, options: SearchOptions) -> Result<Vec<MemoryResult>> {
        use tokio::time::{timeout, Duration};

        let search_future = async {
            // Поиск по всем указанным слоям или всем слоям если не указано
            let layers_to_search = options
                .layers
                .unwrap_or_else(|| vec![Layer::Interact, Layer::Insights, Layer::Assets]);
            let limit = options.limit.unwrap_or(10);

            let mut all_results = Vec::new();

            for layer in layers_to_search {
                let layer_results = self.service.search_sync(query, layer, limit)?;
                all_results.extend(layer_results);
            }

            // Сортируем по релевантности и берем топ результатов
            all_results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            all_results.truncate(limit);

            Ok::<Vec<MemoryResult>, anyhow::Error>(
                all_results
                    .into_iter()
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
                    .collect(),
            )
        };

        // Защита от зависания с таймаутом 30 секунд
        timeout(Duration::from_secs(30), search_future)
            .await
            .map_err(|_| anyhow::anyhow!("Search timeout after 30 seconds"))?
    }

    /// Экспорт всех записей в JSON и запись в файл
    pub async fn backup_to_path<P: AsRef<std::path::Path>>(&self, path: P) -> Result<usize> {
        use std::fs;
        use std::path::Path;
        let engine = simple_engine::engine();
        let records = engine.export_records();
        let json = serde_json::to_string_pretty(&records)?;
        let p = path.as_ref();
        if let Some(parent) = p.parent() { fs::create_dir_all(parent)?; }
        fs::write(p, json)?;
        Ok(records.len())
    }

    /// Импорт записей из JSON файла в память
    pub async fn restore_from_path<P: AsRef<std::path::Path>>(&self, path: P) -> Result<usize> {
        use std::fs;
        let data = fs::read_to_string(path)?;
        let records: Vec<Record> = serde_json::from_str(&data)?;
        let engine = simple_engine::engine();
        let n = engine.import_records(&records)?;
        Ok(n)
    }

    /// Получить конкретную запись по ID
    pub async fn get(&self, id: Uuid) -> Result<Option<MemoryResult>> {
        // Упрощенная реализация - поиск не поддерживается в sync trait
        let _ = id;
        Ok(None)
    }

    /// Удалить запись
    pub async fn forget(&self, id: Uuid) -> Result<bool> {
        // Упрощенная реализация - удаление не поддерживается в sync trait
        let _ = id;
        Ok(false)
    }

    /// Сохранить информацию в память с timeout защитой
    pub async fn remember(&self, text: String, context: MemoryContext) -> Result<Uuid> {
        use tokio::time::{timeout, Duration};

        let layer = context.layer.unwrap_or(Layer::Interact);
        let remember_future = async { self.service.remember_sync(text, layer) };

        // Защита от зависания с таймаутом 15 секунд
        timeout(Duration::from_secs(15), remember_future)
            .await
            .map_err(|_| anyhow::anyhow!("Remember timeout after 15 seconds"))?
    }

    // ========== УПРАВЛЕНИЕ СИСТЕМОЙ ==========

    /// Запустить цикл продвижения памяти с timeout защитой
    pub async fn optimize_memory(&self) -> Result<OptimizationResult> {
        use tokio::time::{timeout, Duration};

        let promotion_future = async { self.service.run_promotion_sync() };

        // Защита от зависания с таймаутом 60 секунд для promotion
        let stats = timeout(Duration::from_secs(60), promotion_future)
            .await
            .map_err(|_| anyhow::anyhow!("Memory optimization timeout after 60 seconds"))??;

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
            components: health
                .component_statuses
                .into_iter()
                .map(|(comp, status)| {
                    let comp_name = match comp {
                        ComponentType::VectorStore => "vector_store",
                        ComponentType::EmbeddingService => "embedding_service",
                        ComponentType::Cache => "cache",
                        ComponentType::PromotionEngine => "promotion_engine",
                        ComponentType::RerankingService => "reranking_service",
                        ComponentType::Database => "database",
                        ComponentType::Memory => "memory",
                        ComponentType::Disk => "disk",
                        ComponentType::Network => "network",
                        ComponentType::Api => "api",
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
        let result = self.service.get_system_health();

        Ok(DetailedHealth {
            overall_status: match result.overall_status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded => "degraded",
                HealthStatus::Unhealthy => "unhealthy",
                HealthStatus::Down => "down",
            },
            uptime_seconds: result.uptime_seconds,
            alerts: result
                .active_alerts
                .into_iter()
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
                hit_rate: if cache_total > 0 {
                    cache_hits as f32 / cache_total as f32
                } else {
                    0.0
                },
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
            interact_to_insights: 0,
            insights_to_assets: 0,
            expired_interact: 0,
            expired_insights: 0,
            total_time_ms: 0,
        })
    }

    /// Получить статистику кэша (hits, misses, total)
    pub fn cache_stats(&self) -> (u64, u64, u64) {
        self.service.cache_stats()
    }
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

impl MemoryContext {
    /// Установить тип записи
    pub fn with_kind(mut self, kind: String) -> Self {
        self.kind = kind;
        self
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

    /// Добавить теги
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Установить проект
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Установить сессию
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }

    /// Установить слой
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

#[cfg(all(test, feature = "embeddings"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn emits_events_on_remember_and_search() {
        if std::env::var("ORT_DYLIB_PATH").is_err() {
            eprintln!("Skipping memory event test: ORT_DYLIB_PATH not set");
            return;
        }
        use crate::types::Layer;
        // Subscribe before actions
        let mut rx_upsert = simple_engine::MEMORY_EVENT_BUS.subscribe(common::topics::TOPIC_MEMORY_UPSERT).await;
        let mut rx_search = simple_engine::MEMORY_EVENT_BUS.subscribe(common::topics::TOPIC_MEMORY_SEARCH).await;

        // Use public API to trigger events
        let api = UnifiedMemoryAPI::new(Arc::new(DIMemoryService::new()));
        let _id = api
            .remember("event bus note".to_string(), MemoryContext::new("note").with_layer(Layer::Insights))
            .await
            .expect("remember ok");

        // await upsert event (tolerate lag)
        let upsert = tokio::time::timeout(std::time::Duration::from_secs(2), rx_upsert.recv())
            .await
            .expect("upsert timeout")
            .expect("upsert recv");
        assert_eq!(upsert.topic.0, common::topics::TOPIC_MEMORY_UPSERT.0);

        // Trigger search
        let _ = api
            .recall("note" , SearchOptions::default().in_layers(vec![Layer::Insights]).limit(1))
            .await
            .expect("recall ok");

        let search = tokio::time::timeout(std::time::Duration::from_secs(2), rx_search.recv())
            .await
            .expect("search timeout")
            .expect("search recv");
        assert_eq!(search.topic.0, common::topics::TOPIC_MEMORY_SEARCH.0);
    }
}
