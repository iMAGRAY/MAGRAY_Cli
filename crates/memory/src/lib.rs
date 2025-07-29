use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Removed unused import

pub mod layers;
pub mod coordinator;
pub mod semantic;
pub mod types;
pub mod onnx_models;
pub mod chunking;
pub mod ingestion;
pub mod vector_index;

pub use coordinator::MemoryCoordinator;
pub use semantic::{SemanticRouter, VectorizerService, RerankerService};
pub use types::*;
pub use chunking::{UniversalChunker, ChunkingStrategy, ContentChunk, ChunkType};
pub use ingestion::{IngestionPipeline, IngestionConfig, IngestionEvent};
pub use vector_index::VectorIndex;

/// Основные слои памяти согласно архитектуре
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemLayer {
    /// M0: Ephemeral - RAM, временные данные шага
    Ephemeral,
    /// M1: ShortTerm - SQLite KV, недавние факты/ответы  
    Short,
    /// M2: MediumTerm - SQLite tables, структурированные знания
    Medium,
    /// M3: LongTerm - blobs/files, большие артефакты
    Long,
    /// M4: Semantic - векторный индекс со ссылками на все слои
    Semantic,
}

/// Ссылка на данные в определённом слое памяти
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemRef {
    pub layer: MemLayer,
    pub key: String,
    pub created_at: DateTime<Utc>,
}

impl MemRef {
    pub fn new(layer: MemLayer, key: String) -> Self {
        Self {
            layer,
            key,
            created_at: Utc::now(),
        }
    }
}

/// Метаданные для записи в памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemMeta {
    pub content_type: String,
    pub size_bytes: usize,
    pub tags: Vec<String>,
    pub ttl_seconds: Option<u64>,
    pub access_count: u64,
    pub last_accessed: DateTime<Utc>,
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for MemMeta {
    fn default() -> Self {
        Self {
            content_type: "text/plain".to_string(),
            size_bytes: 0,
            tags: Vec::new(),
            ttl_seconds: None,
            access_count: 0,
            last_accessed: Utc::now(),
            extra: HashMap::new(),
        }
    }
}

/// Результат поиска в памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemSearchResult {
    pub mem_ref: MemRef,
    pub score: f32,
    pub snippet: Option<String>,
    pub meta: MemMeta,
}

/// Базовый трейт для хранилища данных в слое памяти
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Сохранить данные в слое
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()>;
    
    /// Получить данные из слоя
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>>;
    
    /// Удалить данные из слоя
    async fn delete(&self, key: &str) -> Result<bool>;
    
    /// Проверить существование ключа
    async fn exists(&self, key: &str) -> Result<bool>;
    
    /// Получить список всех ключей (для отладки/миграции)
    async fn list_keys(&self) -> Result<Vec<String>>;
    
    /// Получить статистику слоя
    async fn stats(&self) -> Result<LayerStats>;
}

/// Статистика слоя памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub total_items: u64,
    pub total_size_bytes: u64,
    pub oldest_item: Option<DateTime<Utc>>,
    pub newest_item: Option<DateTime<Utc>>,
    pub avg_access_count: f64,
}

/// Трейт для семантического поиска и индексации
#[async_trait]
pub trait SemanticIndex: Send + Sync {
    /// Добавить текст в семантический индекс
    async fn ingest(&self, text: &str, mem_ref: &MemRef, meta: &MemMeta) -> Result<()>;
    
    /// Семантический поиск по тексту
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemSearchResult>>;
    
    /// Удалить из индекса
    async fn remove(&self, mem_ref: &MemRef) -> Result<bool>;
    
    /// Переиндексировать (для обновлений)
    async fn reindex(&self) -> Result<()>;
    
    /// Очистить сиротские векторы
    async fn vacuum(&self) -> Result<u64>;
}

/// Политики промоушена данных между слоями
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionPolicy {
    /// TTL для автоматического промоушена (секунды)
    pub ttl_seconds: u64,
    /// Минимальное количество обращений для промоушена
    pub min_access_count: u64,
    /// Максимальный размер данных для промоушена (байты)
    pub max_size_bytes: usize,
    /// Теги, которые форсируют промоушен
    pub force_promotion_tags: Vec<String>,
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            ttl_seconds: 3600, // 1 час
            min_access_count: 2,
            max_size_bytes: 1024 * 1024, // 1MB
            force_promotion_tags: vec!["important".to_string(), "persistent".to_string()],
        }
    }
}

/// Конфигурация системы памяти
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub base_path: std::path::PathBuf,
    pub sqlite_path: std::path::PathBuf,
    pub blobs_path: std::path::PathBuf,
    pub vectors_path: std::path::PathBuf,
    pub cache_path: std::path::PathBuf,
    
    // Политики промоушена для каждого слоя
    pub ephemeral_to_short: PromotionPolicy,
    pub short_to_medium: PromotionPolicy,
    pub medium_to_long: PromotionPolicy,
    
    // Настройки семантического поиска
    pub semantic_top_k: usize,
    pub rerank_top_k: usize,
    pub embedding_model: String,
    pub rerank_model: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        let base_path = std::path::PathBuf::from("~/.ourcli/projects/default");
        Self {
            sqlite_path: base_path.join("sqlite.db"),
            blobs_path: base_path.join("blobs"),
            vectors_path: base_path.join("vectors"),
            cache_path: base_path.join("embed_cache.db"),
            base_path,
            
            ephemeral_to_short: PromotionPolicy {
                ttl_seconds: 300, // 5 минут
                min_access_count: 1,
                max_size_bytes: 10 * 1024, // 10KB
                force_promotion_tags: vec!["session".to_string()],
            },
            
            short_to_medium: PromotionPolicy {
                ttl_seconds: 3600, // 1 час
                min_access_count: 3,
                max_size_bytes: 100 * 1024, // 100KB
                force_promotion_tags: vec!["important".to_string()],
            },
            
            medium_to_long: PromotionPolicy {
                ttl_seconds: 24 * 3600, // 1 день
                min_access_count: 5,
                max_size_bytes: 1024 * 1024, // 1MB
                force_promotion_tags: vec!["archive".to_string(), "persistent".to_string()],
            },
            
            semantic_top_k: 128,
            rerank_top_k: 32,
            embedding_model: "qwen3".to_string(),
            rerank_model: "qwen3".to_string(),
        }
    }
}