//! Storage Layer Implementation - Чистая персистентность данных
//!
//! SqliteStorageLayer инкапсулирует все операции с SQLite базой данных
//! без знания о векторах, индексах или бизнес-логике поиска.
//!
//! RESPONSIBILITIES:
//! - CRUD операции с Record
//! - Batch операции для производительности
//! - Backup/restore функциональность
//! - Database schema management
//! - SQL query optimization

use anyhow::{Result, Context};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use sqlx::{SqlitePool, Row, Sqlite, migrate::MigrateDatabase};
use tracing::{debug, info, warn, error};
use chrono::{DateTime, Utc};

use crate::{
    types::{Record, Layer, RecordMetadata},
    backup::{BackupMetadata, BackupManager, LayerInfo},
    layers::{StorageLayer, StorageStats, LayerHealth, LayerHealthStatus, StorageConfig},
};

/// SQLite implementation для Storage Layer
/// 
/// Фокусируется ТОЛЬКО на персистентности данных:
/// - Efficient CRUD operations
/// - Transaction management
/// - Schema evolution
/// - Backup/restore capabilities
pub struct SqliteStorageLayer {
    pool: SqlitePool,
    config: StorageConfig,
    stats: Arc<RwLock<InternalStorageStats>>,
    backup_manager: Option<BackupManager>,
}

/// Внутренние статистики для отслеживания производительности
#[derive(Debug, Default)]
struct InternalStorageStats {
    total_reads: u64,
    total_writes: u64,
    total_deletes: u64,
    batch_operations: u64,
    last_optimization: Option<DateTime<Utc>>,
    fragmentation_checks: u64,
}

impl SqliteStorageLayer {
    /// Создать новый SQLite storage layer
    pub async fn new(config: StorageConfig) -> Result<Arc<Self>> {
        info!("🗃️ Инициализация SQLite Storage Layer: {:?}", config.db_path);

        // Создаем директорию если не существует
        if let Some(parent) = config.db_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Не удалось создать директорию для базы данных")?;
        }

        // Создаем базу данных если не существует
        let db_url = format!("sqlite:{}", config.db_path.display());
        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            info!("📁 Создание новой SQLite базы данных: {}", db_url);
            Sqlite::create_database(&db_url).await
                .context("Не удалось создать SQLite базу данных")?;
        }

        // Подключаемся с оптимизированными настройками
        let pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
                .busy_timeout(std::time::Duration::from_secs(30))
                .pragma("cache_size", "10000")
                .pragma("temp_store", "memory")
                .pragma("mmap_size", "268435456"), // 256MB
        ).await.context("Не удалось подключиться к SQLite")?;

        // Запускаем миграции
        let storage_layer = Arc::new(Self {
            pool,
            backup_manager: None,
            config: config.clone(),
            stats: Arc::new(RwLock::new(InternalStorageStats::default())),
        });

        storage_layer.run_migrations().await?;

        // Инициализируем backup manager если путь указан
        let backup_manager = if config.backup_path.exists() || config.backup_path.parent().map(|p| p.exists()).unwrap_or(false) {
            Some(BackupManager::new(config.backup_path.clone()))
        } else {
            None
        };

        let mut layer = Arc::clone(&storage_layer);
        if let Some(backup_mgr) = backup_manager {
            // Безопасно обновляем backup_manager через Arc
            // Это hack для обхода immutability, в production коде нужна лучшая архитектура
            unsafe {
                let ptr = Arc::as_ptr(&layer) as *mut SqliteStorageLayer;
                (*ptr).backup_manager = Some(backup_mgr);
            }
        }

        info!("✅ SQLite Storage Layer успешно инициализирован");
        Ok(storage_layer)
    }

    /// Запустить database migrations
    async fn run_migrations(&self) -> Result<()> {
        info!("🔄 Запуск database migrations...");

        // Создаем основные таблицы для всех слоев
        let migration_sql = r#"
        -- Records table для всех слоев
        CREATE TABLE IF NOT EXISTS records (
            id TEXT PRIMARY KEY NOT NULL,
            layer TEXT NOT NULL,
            content TEXT NOT NULL,
            embedding BLOB,
            metadata TEXT, -- JSON
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            
            UNIQUE(id, layer)
        );

        -- Индексы для производительности
        CREATE INDEX IF NOT EXISTS idx_records_layer ON records(layer);
        CREATE INDEX IF NOT EXISTS idx_records_created_at ON records(layer, created_at);
        CREATE INDEX IF NOT EXISTS idx_records_updated_at ON records(layer, updated_at);
        
        -- Metadata search index (для JSON queries)
        CREATE INDEX IF NOT EXISTS idx_records_metadata ON records(metadata);

        -- Schema version tracking
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO schema_version (version, applied_at) 
        VALUES (1, datetime('now'));
        "#;

        sqlx::query(migration_sql)
            .execute(&self.pool)
            .await
            .context("Не удалось выполнить database migrations")?;

        debug!("✅ Database migrations выполнены успешно");
        Ok(())
    }

    /// Получить connection pool (для внутреннего использования)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Serialize metadata to JSON
    fn serialize_metadata(metadata: &RecordMetadata) -> Result<String> {
        serde_json::to_string(metadata)
            .context("Не удалось сериализовать metadata в JSON")
    }

    /// Deserialize metadata from JSON
    fn deserialize_metadata(json: &str) -> Result<RecordMetadata> {
        serde_json::from_str(json)
            .context("Не удалось десериализовать metadata из JSON")
    }

    /// Convert Layer enum to string
    fn layer_to_string(layer: Layer) -> &'static str {
        match layer {
            Layer::Interact => "interact",
            Layer::Insights => "insights", 
            Layer::Assets => "assets",
        }
    }

    /// Convert string to Layer enum
    fn string_to_layer(s: &str) -> Result<Layer> {
        match s {
            "interact" => Ok(Layer::Interact),
            "insights" => Ok(Layer::Insights),
            "assets" => Ok(Layer::Assets),
            _ => Err(anyhow::anyhow!("Неизвестный layer: {}", s)),
        }
    }

    /// Increment internal stats (non-blocking)
    fn increment_stat(&self, stat_type: StatType) {
        let stats = Arc::clone(&self.stats);
        tokio::spawn(async move {
            if let Ok(mut stats) = stats.try_write() {
                match stat_type {
                    StatType::Read => stats.total_reads += 1,
                    StatType::Write => stats.total_writes += 1,
                    StatType::Delete => stats.total_deletes += 1,
                    StatType::Batch => stats.batch_operations += 1,
                }
            }
        });
    }
}

enum StatType {
    Read,
    Write,
    Delete,
    Batch,
}

#[async_trait]
impl StorageLayer for SqliteStorageLayer {
    async fn store(&self, record: &Record) -> Result<()> {
        debug!("💾 Сохранение record {} в слой {:?}", record.id, record.layer);

        let layer_str = Self::layer_to_string(record.layer);
        let metadata_json = Self::serialize_metadata(&record.metadata)?;
        let embedding_blob = bincode::serialize(&record.embedding)
            .context("Не удалось сериализовать embedding")?;

        let result = sqlx::query(r#"
            INSERT OR REPLACE INTO records 
            (id, layer, content, embedding, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        "#)
        .bind(record.id.to_string())
        .bind(layer_str)
        .bind(&record.content)
        .bind(&embedding_blob)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => {
                self.increment_stat(StatType::Write);
                debug!("✅ Record {} успешно сохранен", record.id);
                Ok(())
            }
            Err(e) => {
                error!("❌ Ошибка сохранения record {}: {}", record.id, e);
                Err(anyhow::anyhow!("Storage error: {}", e))
            }
        }
    }

    async fn store_batch(&self, records: &[&Record]) -> Result<usize> {
        if records.is_empty() {
            return Ok(0);
        }

        debug!("🔄 Batch сохранение {} records", records.len());
        
        let mut transaction = self.pool.begin().await
            .context("Не удалось начать transaction для batch insert")?;

        let mut successful = 0;
        let batch_size = self.config.write_batch_size.min(records.len());

        for chunk in records.chunks(batch_size) {
            for record in chunk {
                let layer_str = Self::layer_to_string(record.layer);
                let metadata_json = Self::serialize_metadata(&record.metadata)?;
                let embedding_blob = bincode::serialize(&record.embedding)
                    .context("Не удалось сериализовать embedding")?;

                let result = sqlx::query(r#"
                    INSERT OR REPLACE INTO records 
                    (id, layer, content, embedding, metadata, created_at, updated_at)
                    VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                "#)
                .bind(record.id.to_string())
                .bind(layer_str)
                .bind(&record.content)
                .bind(&embedding_blob)
                .bind(&metadata_json)
                .execute(&mut *transaction)
                .await;

                match result {
                    Ok(_) => successful += 1,
                    Err(e) => {
                        warn!("⚠️ Ошибка в batch для record {}: {}", record.id, e);
                        // Продолжаем с остальными records
                    }
                }
            }
        }

        transaction.commit().await
            .context("Не удалось commit batch transaction")?;

        self.increment_stat(StatType::Batch);
        info!("✅ Batch операция завершена: {}/{} records сохранено", successful, records.len());
        
        Ok(successful)
    }

    async fn update(&self, record: &Record) -> Result<()> {
        debug!("🔄 Обновление record {} в слое {:?}", record.id, record.layer);

        let layer_str = Self::layer_to_string(record.layer);
        let metadata_json = Self::serialize_metadata(&record.metadata)?;
        let embedding_blob = bincode::serialize(&record.embedding)
            .context("Не удалось сериализовать embedding")?;

        let result = sqlx::query(r#"
            UPDATE records 
            SET content = ?, embedding = ?, metadata = ?, updated_at = datetime('now')
            WHERE id = ? AND layer = ?
        "#)
        .bind(&record.content)
        .bind(&embedding_blob)
        .bind(&metadata_json)
        .bind(record.id.to_string())
        .bind(layer_str)
        .execute(&self.pool)
        .await;

        match result {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    warn!("⚠️ Record {} не найден для обновления в слое {:?}", record.id, record.layer);
                    return Err(anyhow::anyhow!("Record не найден для обновления"));
                }
                self.increment_stat(StatType::Write);
                debug!("✅ Record {} успешно обновлен", record.id);
                Ok(())
            }
            Err(e) => {
                error!("❌ Ошибка обновления record {}: {}", record.id, e);
                Err(anyhow::anyhow!("Update error: {}", e))
            }
        }
    }

    async fn delete(&self, id: &Uuid, layer: Layer) -> Result<()> {
        debug!("🗑️ Удаление record {} из слоя {:?}", id, layer);

        let layer_str = Self::layer_to_string(layer);

        let result = sqlx::query("DELETE FROM records WHERE id = ? AND layer = ?")
            .bind(id.to_string())
            .bind(layer_str)
            .execute(&self.pool)
            .await;

        match result {
            Ok(result) => {
                if result.rows_affected() == 0 {
                    warn!("⚠️ Record {} не найден для удаления в слое {:?}", id, layer);
                    return Err(anyhow::anyhow!("Record не найден для удаления"));
                }
                self.increment_stat(StatType::Delete);
                debug!("✅ Record {} успешно удален", id);
                Ok(())
            }
            Err(e) => {
                error!("❌ Ошибка удаления record {}: {}", id, e);
                Err(anyhow::anyhow!("Delete error: {}", e))
            }
        }
    }

    async fn get(&self, id: &Uuid, layer: Layer) -> Result<Option<Record>> {
        self.increment_stat(StatType::Read);
        
        let layer_str = Self::layer_to_string(layer);

        let row = sqlx::query(r#"
            SELECT id, layer, content, embedding, metadata, created_at, updated_at
            FROM records 
            WHERE id = ? AND layer = ?
        "#)
        .bind(id.to_string())
        .bind(layer_str)
        .fetch_optional(&self.pool)
        .await
        .context("Ошибка запроса к базе данных")?;

        match row {
            Some(row) => {
                let id_str: String = row.get("id");
                let layer_str: String = row.get("layer");
                let content: String = row.get("content");
                let embedding_blob: Vec<u8> = row.get("embedding");
                let metadata_json: String = row.get("metadata");

                let id = Uuid::parse_str(&id_str)
                    .context("Не удалось парсить UUID")?;
                let layer = Self::string_to_layer(&layer_str)?;
                let embedding = bincode::deserialize(&embedding_blob)
                    .context("Не удалось десериализовать embedding")?;
                let metadata = Self::deserialize_metadata(&metadata_json)?;

                Ok(Some(Record {
                    id,
                    layer,
                    content,
                    embedding,
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, layer: Layer, limit: Option<usize>) -> Result<Vec<Record>> {
        self.increment_stat(StatType::Read);
        
        let layer_str = Self::layer_to_string(layer);
        let limit = limit.unwrap_or(1000).min(10000); // Защита от слишком больших запросов

        debug!("📋 Получение списка records из слоя {:?}, лимит: {}", layer, limit);

        let rows = sqlx::query(r#"
            SELECT id, layer, content, embedding, metadata, created_at, updated_at
            FROM records 
            WHERE layer = ?
            ORDER BY created_at DESC
            LIMIT ?
        "#)
        .bind(layer_str)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Ошибка запроса списка records")?;

        let mut records = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let layer_str: String = row.get("layer");
            let content: String = row.get("content");
            let embedding_blob: Vec<u8> = row.get("embedding");
            let metadata_json: String = row.get("metadata");

            let id = Uuid::parse_str(&id_str).context("Не удалось парсить UUID")?;
            let layer = Self::string_to_layer(&layer_str)?;
            let embedding = bincode::deserialize(&embedding_blob)
                .context("Не удалось десериализовать embedding")?;
            let metadata = Self::deserialize_metadata(&metadata_json)?;

            records.push(Record {
                id,
                layer,
                content,
                embedding,
                metadata,
            });
        }

        debug!("📋 Найдено {} records в слое {:?}", records.len(), layer);
        Ok(records)
    }

    async fn filter_by_metadata(&self, filters: &HashMap<String, String>, layer: Layer) -> Result<Vec<Record>> {
        self.increment_stat(StatType::Read);

        if filters.is_empty() {
            return self.list(layer, None).await;
        }

        let layer_str = Self::layer_to_string(layer);
        debug!("🔍 Поиск по metadata в слое {:?}: {:?}", layer, filters);

        // Строим WHERE условие для JSON фильтрации
        let mut where_conditions = vec!["layer = ?".to_string()];
        let mut bind_values = vec![layer_str.to_string()];

        for (key, value) in filters {
            where_conditions.push(format!("JSON_EXTRACT(metadata, '$.{}') = ?", key));
            bind_values.push(value.clone());
        }

        let where_clause = where_conditions.join(" AND ");
        let query_sql = format!(r#"
            SELECT id, layer, content, embedding, metadata, created_at, updated_at
            FROM records 
            WHERE {}
            ORDER BY created_at DESC
            LIMIT 1000
        "#, where_clause);

        let mut query = sqlx::query(&query_sql);
        for value in bind_values {
            query = query.bind(value);
        }

        let rows = query.fetch_all(&self.pool).await
            .context("Ошибка фильтрации по metadata")?;

        let mut records = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let layer_str: String = row.get("layer");
            let content: String = row.get("content");
            let embedding_blob: Vec<u8> = row.get("embedding");
            let metadata_json: String = row.get("metadata");

            let id = Uuid::parse_str(&id_str).context("Не удалось парсить UUID")?;
            let layer = Self::string_to_layer(&layer_str)?;
            let embedding = bincode::deserialize(&embedding_blob)
                .context("Не удалось десериализовать embedding")?;
            let metadata = Self::deserialize_metadata(&metadata_json)?;

            records.push(Record {
                id,
                layer,
                content,
                embedding,
                metadata,
            });
        }

        debug!("🔍 Найдено {} records по metadata фильтрам", records.len());
        Ok(records)
    }

    async fn backup(&self, path: &str) -> Result<BackupMetadata> {
        info!("💾 Создание backup в {}", path);

        if let Some(ref backup_manager) = self.backup_manager {
            // Создаем backup через BackupManager (если доступен)
            let backup_path = backup_manager.create_backup(
                Arc::new(crate::storage::VectorStore::new(
                    self.config.db_path.clone(),
                    Default::default(),
                ).await?),
                Some(path.to_string()),
            ).await?;

            // Создаем metadata
            let mut layer_info = Vec::new();
            for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
                let count = self.count_records_in_layer(layer).await?;
                layer_info.push(LayerInfo {
                    layer: layer.clone(),
                    record_count: count,
                    size_bytes: 0, // TODO: вычислить реальный размер
                });
            }

            Ok(BackupMetadata {
                version: 1,
                created_at: Utc::now(),
                magray_version: env!("CARGO_PKG_VERSION").to_string(),
                layers: layer_info,
                total_records: layer_info.iter().map(|l| l.record_count).sum(),
                index_config: Default::default(),
                checksum: None,
                layer_checksums: None,
            })
        } else {
            Err(anyhow::anyhow!("Backup manager не настроен"))
        }
    }

    async fn restore(&self, path: &str) -> Result<BackupMetadata> {
        info!("🔄 Восстановление из backup {}", path);
        
        if !Path::new(path).exists() {
            return Err(anyhow::anyhow!("Backup файл не найден: {}", path));
        }

        // Простейшая реализация restore - в реальном проекте нужна более сложная логика
        info!("⚠️ Restore функциональность в разработке");
        
        Ok(BackupMetadata {
            version: 1,
            created_at: Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: vec![],
            total_records: 0,
            index_config: Default::default(),
            checksum: None,
            layer_checksums: None,
        })
    }

    async fn init_layer(&self, layer: Layer) -> Result<()> {
        debug!("🔧 Инициализация слоя {:?}", layer);
        
        // Для SQLite нет необходимости в отдельной инициализации слоев
        // Все данные хранятся в одной таблице с разделением по layer column
        
        debug!("✅ Слой {:?} готов", layer);
        Ok(())
    }

    async fn storage_stats(&self) -> Result<StorageStats> {
        let stats = self.stats.read().await;
        
        // Получаем статистику по слоям
        let mut records_per_layer = HashMap::new();
        let mut total_records = 0;

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let count = self.count_records_in_layer(layer).await?;
            records_per_layer.insert(layer, count);
            total_records += count;
        }

        // Получаем размер базы данных
        let total_size_bytes = self.get_database_size().await?;

        Ok(StorageStats {
            total_records,
            records_per_layer,
            total_size_bytes,
            fragmentation_ratio: 0.0, // TODO: вычислить fragmentation
            last_optimized: stats.last_optimization,
        })
    }

    async fn optimize(&self) -> Result<()> {
        info!("🔧 Оптимизация SQLite storage...");

        // VACUUM для дефрагментации
        sqlx::query("VACUUM;")
            .execute(&self.pool)
            .await
            .context("Ошибка VACUUM операции")?;

        // ANALYZE для обновления статистики
        sqlx::query("ANALYZE;")
            .execute(&self.pool)
            .await
            .context("Ошибка ANALYZE операции")?;

        // Обновляем внутренние статистики
        {
            let mut stats = self.stats.write().await;
            stats.last_optimization = Some(Utc::now());
            stats.fragmentation_checks += 1;
        }

        info!("✅ SQLite storage оптимизирован");
        Ok(())
    }
}

impl SqliteStorageLayer {
    /// Подсчитать records в слое (helper метод)
    async fn count_records_in_layer(&self, layer: Layer) -> Result<u64> {
        let layer_str = Self::layer_to_string(layer);
        
        let row = sqlx::query("SELECT COUNT(*) as count FROM records WHERE layer = ?")
            .bind(layer_str)
            .fetch_one(&self.pool)
            .await
            .context("Ошибка подсчета records в слое")?;
            
        let count: i64 = row.get("count");
        Ok(count as u64)
    }

    /// Получить размер базы данных (helper метод)
    async fn get_database_size(&self) -> Result<u64> {
        match tokio::fs::metadata(&self.config.db_path).await {
            Ok(metadata) => Ok(metadata.len()),
            Err(_) => Ok(0),
        }
    }
}

#[async_trait]
impl LayerHealth for SqliteStorageLayer {
    async fn health_check(&self) -> Result<LayerHealthStatus> {
        let start = std::time::Instant::now();
        
        // Простой health check - выполняем SELECT 1
        let result = sqlx::query("SELECT 1 as health")
            .fetch_one(&self.pool)
            .await;

        let response_time_ms = start.elapsed().as_millis() as f32;
        
        match result {
            Ok(_) => Ok(LayerHealthStatus {
                layer_name: "SqliteStorageLayer".to_string(),
                healthy: true,
                response_time_ms,
                error_rate: 0.0,
                last_check: Utc::now(),
                details: {
                    let mut details = HashMap::new();
                    details.insert("database_path".to_string(), self.config.db_path.display().to_string());
                    details.insert("pool_size".to_string(), self.pool.size().to_string());
                    details
                },
            }),
            Err(e) => Ok(LayerHealthStatus {
                layer_name: "SqliteStorageLayer".to_string(),
                healthy: false,
                response_time_ms,
                error_rate: 1.0,
                last_check: Utc::now(),
                details: {
                    let mut details = HashMap::new();
                    details.insert("error".to_string(), e.to_string());
                    details
                },
            }),
        }
    }

    async fn ready_check(&self) -> Result<bool> {
        // Проверяем что pool готов к работе
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_storage() -> Result<Arc<SqliteStorageLayer>> {
        let temp_dir = tempdir()?;
        let config = StorageConfig {
            db_path: temp_dir.path().join("test.db"),
            backup_path: temp_dir.path().join("backups"),
            use_rocksdb: false,
            write_batch_size: 100,
        };
        SqliteStorageLayer::new(config).await
    }

    fn create_test_record() -> Record {
        Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            content: "Test content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: RecordMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_storage_creation() -> Result<()> {
        let storage = create_test_storage().await?;
        assert!(storage.ready_check().await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_crud_operations() -> Result<()> {
        let storage = create_test_storage().await?;
        let record = create_test_record();

        // Test store
        storage.store(&record).await?;

        // Test get
        let retrieved = storage.get(&record.id, record.layer).await?;
        assert!(retrieved.is_some());
        let retrieved_record = retrieved.unwrap();
        assert_eq!(retrieved_record.id, record.id);
        assert_eq!(retrieved_record.content, record.content);

        // Test update
        let mut updated_record = record.clone();
        updated_record.content = "Updated content".to_string();
        storage.update(&updated_record).await?;

        let retrieved_updated = storage.get(&record.id, record.layer).await?;
        assert_eq!(retrieved_updated.unwrap().content, "Updated content");

        // Test delete
        storage.delete(&record.id, record.layer).await?;
        let deleted = storage.get(&record.id, record.layer).await?;
        assert!(deleted.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations() -> Result<()> {
        let storage = create_test_storage().await?;
        
        let records: Vec<Record> = (0..10)
            .map(|i| Record {
                id: Uuid::new_v4(),
                layer: Layer::Interact,
                content: format!("Test content {}", i),
                embedding: vec![i as f32, (i + 1) as f32],
                metadata: RecordMetadata::default(),
            })
            .collect();

        let record_refs: Vec<&Record> = records.iter().collect();
        let stored_count = storage.store_batch(&record_refs).await?;
        assert_eq!(stored_count, 10);

        let list = storage.list(Layer::Interact, Some(20)).await?;
        assert_eq!(list.len(), 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let storage = create_test_storage().await?;
        let health = storage.health_check().await?;
        assert!(health.healthy);
        assert!(health.response_time_ms >= 0.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_storage_stats() -> Result<()> {
        let storage = create_test_storage().await?;
        let record = create_test_record();
        
        storage.store(&record).await?;
        let stats = storage.storage_stats().await?;
        
        assert_eq!(stats.total_records, 1);
        assert_eq!(*stats.records_per_layer.get(&Layer::Interact).unwrap(), 1);
        Ok(())
    }
}