use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Запрос на эмбеддинг текста
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub texts: Vec<String>,
    pub purpose: EmbedPurpose,
    pub model: Option<String>,
}

/// Назначение эмбеддинга (влияет на выбор модели/параметров)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbedPurpose {
    /// Для индексации в векторной БД
    Index,
    /// Для поискового запроса
    Query,
    /// Для описания инструментов
    ToolSpec,
    /// Для анализа кода
    Code,
}

/// Ответ с эмбеддингами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub vectors: Vec<Vec<f32>>,
    pub model: String,
    pub dimensions: usize,
    pub tokens_used: Option<u32>,
}

/// Запрос на reranking документов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<String>,
    pub top_k: usize,
    pub model: Option<String>,
}

/// Результат reranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankHit {
    pub index: usize,
    pub score: f32,
    pub document: String,
}

/// Ответ с reranked документами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResponse {
    pub hits: Vec<RerankHit>,
    pub model: String,
    pub query_time_ms: u64,
}

/// Событие в системе памяти для EventBus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEvent {
    /// Данные записаны в слой
    DataStored {
        layer: crate::MemLayer,
        key: String,
        size_bytes: usize,
    },
    /// Данные прочитаны из слоя
    DataAccessed {
        layer: crate::MemLayer,
        key: String,
        hit: bool,
    },
    /// Данные промоушены между слоями
    DataPromoted {
        from_layer: crate::MemLayer,
        to_layer: crate::MemLayer,
        key: String,
        reason: String,
    },
    /// Данные проиндексированы семантически
    DataIndexed {
        mem_ref: crate::MemRef,
        vector_dimensions: usize,
    },
    /// Выполнен семантический поиск
    SemanticSearch {
        query: String,
        results_count: usize,
        query_time_ms: u64,
    },
    /// Выполнена очистка слоя
    LayerCleaned {
        layer: crate::MemLayer,
        items_removed: u64,
        bytes_freed: u64,
    },
}

/// Контекст выполнения для передачи между компонентами
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub request_id: Uuid,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            request_id: Uuid::new_v4(),
            session_id: None,
            user_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Результат операции с памятью
#[derive(Debug, Clone)]
pub struct MemoryOperationResult {
    pub success: bool,
    pub mem_ref: Option<crate::MemRef>,
    pub bytes_processed: usize,
    pub operation_time_ms: u64,
    pub error: Option<String>,
}

/// Статистика использования памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub layers: HashMap<crate::MemLayer, crate::LayerStats>,
    pub total_items: u64,
    pub total_size_bytes: u64,
    pub cache_hit_rate: f64,
    pub avg_query_time_ms: f64,
    pub promotions_last_hour: u64,
}

/// Конфигурация для отдельного слоя памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    pub max_items: Option<u64>,
    pub max_size_bytes: Option<u64>,
    pub ttl_seconds: Option<u64>,
    pub cleanup_interval_seconds: u64,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            max_items: None,
            max_size_bytes: None,
            ttl_seconds: None,
            cleanup_interval_seconds: 3600, // 1 час
            compression_enabled: false,
            encryption_enabled: false,
        }
    }
}