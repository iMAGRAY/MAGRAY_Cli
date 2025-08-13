//! Storage Provider Port
//!
//! Абстракция для работы с системой хранения данных

use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use domain::entities::{memory_record::MemoryRecord, record_id::RecordId};
use std::collections::HashMap;

/// Trait для работы с системой хранения
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Сохранить запись
    async fn store(&self, record: &MemoryRecord) -> ApplicationResult<()>;

    /// Получить запись по ID
    async fn get(&self, id: &RecordId) -> ApplicationResult<Option<MemoryRecord>>;

    /// Удалить запись
    async fn delete(&self, id: &RecordId) -> ApplicationResult<()>;

    /// Получить все записи
    async fn get_all(&self) -> ApplicationResult<Vec<MemoryRecord>>;

    /// Получить записи по фильтру
    async fn get_by_filter(
        &self,
        filter: HashMap<String, String>,
    ) -> ApplicationResult<Vec<MemoryRecord>>;

    /// Проверить существование записи
    async fn exists(&self, id: &RecordId) -> ApplicationResult<bool>;

    /// Получить количество записей
    async fn count(&self) -> ApplicationResult<u64>;

    /// Очистить все данные
    async fn clear(&self) -> ApplicationResult<()>;

    /// Выполнить пакетную операцию сохранения
    async fn batch_store(&self, records: &[MemoryRecord]) -> ApplicationResult<()>;
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_records: u64,
    pub total_size_bytes: u64,
    pub cache_records: u64,
    pub index_records: u64,
    pub storage_records: u64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// Storage health status
#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_rate: f64,
    pub last_error: Option<String>,
    pub details: HashMap<String, String>,
}

/// Backup operation result
#[derive(Debug, Clone)]
pub struct BackupResult {
    pub backup_id: String,
    pub records_backed_up: u64,
    pub size_bytes: u64,
    pub duration_ms: u64,
}

/// Restore operation result
#[derive(Debug, Clone)]
pub struct RestoreResult {
    pub backup_id: String,
    pub records_restored: u64,
    pub duration_ms: u64,
    pub success: bool,
}
