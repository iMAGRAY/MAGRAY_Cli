use crate::{LayerStats, MemMeta, MemoryStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// M1: ShortTerm - SQLite KV хранилище для недавних фактов/ответов
/// 
/// Особенности:
/// - Персистентное хранилище в SQLite
/// - Key-Value структура для быстрого доступа
/// - Автоматическая очистка по TTL и количеству обращений
/// - Индексация в M4 для семантического поиска
#[derive(Debug)]
pub struct ShortTermStore {
    conn: Arc<Mutex<Connection>>,
}

impl ShortTermStore {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context("Failed to open short-term SQLite database")?;
        
        // Создаём таблицу если не существует
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS kv_short (
                key TEXT PRIMARY KEY,
                value BLOB NOT NULL,
                meta TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                accessed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                access_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
            [],
        ).context("Failed to create kv_short table")?;
        
        // Создаём индексы для оптимизации
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kv_short_created_at ON kv_short(created_at)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kv_short_accessed_at ON kv_short(accessed_at)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_kv_short_access_count ON kv_short(access_count)",
            [],
        )?;
        
        tracing::info!("Initialized short-term memory store");
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    
    /// Очистка устаревших записей по различным критериям
    pub async fn cleanup(&self, max_age_seconds: u64, min_access_count: u64) -> Result<u64> {
        let conn = self.conn.lock().await;
        
        let cutoff_time = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        let cutoff_str = cutoff_time.to_rfc3339();
        
        let removed = conn.execute(
            r#"
            DELETE FROM kv_short 
            WHERE created_at < ?1 AND access_count < ?2
            "#,
            params![cutoff_str, min_access_count],
        ).context("Failed to cleanup short-term store")?;
        
        tracing::debug!("Cleaned up {} items from short-term store", removed);
        Ok(removed as u64)
    }
    
    /// Получить записи для промоушена в следующий слой
    pub async fn get_promotion_candidates(&self, min_access_count: u64, max_age_seconds: u64) -> Result<Vec<(String, Vec<u8>, MemMeta)>> {
        let conn = self.conn.lock().await;
        
        let cutoff_time = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        let cutoff_str = cutoff_time.to_rfc3339();
        
        let mut stmt = conn.prepare(
            r#"
            SELECT key, value, meta 
            FROM kv_short 
            WHERE access_count >= ?1 AND created_at <= ?2
            ORDER BY access_count DESC, created_at ASC
            "#
        ).context("Failed to prepare promotion query")?;
        
        let rows = stmt.query_map(params![min_access_count, cutoff_str], |row| {
            let key: String = row.get(0)?;
            let value: Vec<u8> = row.get(1)?;
            let meta_json: String = row.get(2)?;
            
            let meta: MemMeta = serde_json::from_str(&meta_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(0, format!("JSON parse error: {}", e).into(), rusqlite::types::Type::Text))?;
            
            Ok((key, value, meta))
        })?;
        
        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row?);
        }
        
        tracing::debug!("Found {} promotion candidates in short-term store", candidates.len());
        Ok(candidates)
    }
    
    /// Получить статистику по частоте доступа
    pub async fn get_access_stats(&self) -> Result<Vec<(String, u64, DateTime<Utc>)>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            "SELECT key, access_count, accessed_at FROM kv_short ORDER BY access_count DESC LIMIT 100"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let access_count: u64 = row.get::<_, i64>(1)? as u64;
            let accessed_at_str: String = row.get(2)?;
            
            let accessed_at = DateTime::parse_from_rfc3339(&accessed_at_str)
                .map_err(|e| rusqlite::Error::InvalidColumnType(2, format!("DateTime parse error: {}", e).into(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            
            Ok((key, access_count, accessed_at))
        })?;
        
        let mut stats = Vec::new();
        for row in rows {
            stats.push(row?);
        }
        
        Ok(stats)
    }
}

#[async_trait]
impl MemoryStore for ShortTermStore {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        let conn = self.conn.lock().await;
        let meta_json = serde_json::to_string(meta)
            .context("Failed to serialize metadata")?;
        
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO kv_short (key, value, meta, created_at, accessed_at, access_count)
            VALUES (?1, ?2, ?3, ?4, ?5, 0)
            "#,
            params![key, data, meta_json, now, now],
        ).context("Failed to insert into short-term store")?;
        
        tracing::trace!("Stored {} bytes in short-term layer with key: {}", data.len(), key);
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        let conn = self.conn.lock().await;
        
        // Сначала получаем данные и текущий access_count
        let result = conn.query_row(
            "SELECT value, meta, access_count FROM kv_short WHERE key = ?1",
            params![key],
            |row| {
                let value: Vec<u8> = row.get(0)?;
                let meta_json: String = row.get(1)?;
                let current_access_count: i64 = row.get(2)?;
                Ok((value, meta_json, current_access_count))
            }
        ).optional().context("Failed to query short-term store")?;
        
        if let Some((value, meta_json, current_access_count)) = result {
            // Обновляем статистику доступа
            let now = Utc::now().to_rfc3339();
            let new_access_count = current_access_count + 1;
            conn.execute(
                r#"
                UPDATE kv_short 
                SET access_count = ?1, accessed_at = ?2 
                WHERE key = ?3
                "#,
                params![new_access_count, now, key],
            ).context("Failed to update access stats")?;
            
            let mut meta: MemMeta = serde_json::from_str(&meta_json)
                .context("Failed to deserialize metadata")?;
            
            meta.access_count = new_access_count as u64;
            meta.last_accessed = Utc::now();
            
            tracing::trace!("Retrieved {} bytes from short-term layer: {}", value.len(), key);
            Ok(Some((value, meta)))
        } else {
            tracing::trace!("Key not found in short-term layer: {}", key);
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        
        let removed = conn.execute(
            "DELETE FROM kv_short WHERE key = ?1",
            params![key],
        ).context("Failed to delete from short-term store")?;
        
        let deleted = removed > 0;
        if deleted {
            tracing::trace!("Deleted key from short-term layer: {}", key);
        }
        
        Ok(deleted)
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        
        let exists = conn.query_row(
            "SELECT 1 FROM kv_short WHERE key = ?1",
            params![key],
            |_| Ok(())
        ).optional()?.is_some();
        
        Ok(exists)
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare("SELECT key FROM kv_short ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;
        
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        
        Ok(keys)
    }
    
    async fn stats(&self) -> Result<LayerStats> {
        let conn = self.conn.lock().await;
        
        let (total_items, total_size_bytes) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(value)), 0) FROM kv_short",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                let size: i64 = row.get(1)?;
                Ok((count as u64, size as u64))
            }
        ).context("Failed to get basic stats")?;
        
        let oldest_item = conn.query_row(
            "SELECT MIN(created_at) FROM kv_short",
            [],
            |row| {
                let created_at_str: Option<String> = row.get(0)?;
                Ok(created_at_str)
            }
        ).optional()?.flatten().and_then(|s| {
            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
        });
        
        let newest_item = conn.query_row(
            "SELECT MAX(created_at) FROM kv_short",
            [],
            |row| {
                let created_at_str: Option<String> = row.get(0)?;
                Ok(created_at_str)
            }
        ).optional()?.flatten().and_then(|s| {
            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
        });
        
        let avg_access_count = if total_items > 0 {
            conn.query_row(
                "SELECT AVG(CAST(access_count AS REAL)) FROM kv_short",
                [],
                |row| Ok(row.get::<_, f64>(0)?)
            ).unwrap_or(0.0)
        } else {
            0.0
        };
        
        Ok(LayerStats {
            total_items,
            total_size_bytes,
            oldest_item,
            newest_item,
            avg_access_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_short_term_basic_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = ShortTermStore::new(temp_file.path()).await.unwrap();
        
        let key = "test_key";
        let data = b"test data";
        let meta = MemMeta::default();
        
        // Test put
        store.put(key, data, &meta).await.unwrap();
        
        // Test exists
        assert!(store.exists(key).await.unwrap());
        
        // Test get
        let result = store.get(key).await.unwrap();
        assert!(result.is_some());
        let (retrieved_data, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_data, data);
        assert_eq!(retrieved_meta.access_count, 1);
        
        // Test second get (should increment access count)
        let result = store.get(key).await.unwrap();
        assert!(result.is_some());
        let (_, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_meta.access_count, 2);
        
        // Test delete
        assert!(store.delete(key).await.unwrap());
        assert!(!store.exists(key).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_short_term_promotion_candidates() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = ShortTermStore::new(temp_file.path()).await.unwrap();
        
        // Добавляем записи с разным количеством обращений
        for i in 0..5 {
            let key = format!("key_{}", i);
            let data = format!("data_{}", i);
            let meta = MemMeta::default();
            
            store.put(&key, data.as_bytes(), &meta).await.unwrap();
            
            // Делаем разное количество обращений
            for _ in 0..i {
                store.get(&key).await.unwrap();
            }
        }
        
        // Ждём достаточно времени чтобы записи стали достаточно старыми для промоушена
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Получаем кандидатов для промоушена (минимум 2 обращения, старше 1 секунды)
        let candidates = store.get_promotion_candidates(2, 1).await.unwrap();
        
        // Должны быть key_2, key_3, key_4 (у них >= 2 обращений)
        assert_eq!(candidates.len(), 3);
        
        // Проверяем что они отсортированы по количеству обращений
        assert!(candidates[0].0.contains("key_4")); // Максимальное количество обращений
    }
}