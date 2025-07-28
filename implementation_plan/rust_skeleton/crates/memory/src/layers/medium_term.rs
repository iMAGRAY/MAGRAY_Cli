use crate::{LayerStats, MemMeta, MemoryStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// M2: MediumTerm - SQLite таблицы для структурированных знаний проекта
/// 
/// Особенности:
/// - Структурированное хранилище с типизированными таблицами
/// - Поддержка различных типов данных (facts, results, metadata)
/// - Связи между записями через foreign keys
/// - Оптимизированные запросы для аналитики
#[derive(Debug)]
pub struct MediumTermStore {
    conn: Arc<Mutex<Connection>>,
}

impl MediumTermStore {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context("Failed to open medium-term SQLite database")?;
        
        // Создаём основную таблицу для структурированных данных
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS facts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                title TEXT,
                body TEXT,
                meta TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                accessed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                access_count INTEGER NOT NULL DEFAULT 0,
                parent_id INTEGER,
                FOREIGN KEY (parent_id) REFERENCES facts(id)
            )
            "#,
            [],
        ).context("Failed to create facts table")?;
        
        // Таблица для результатов выполнения задач
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS execution_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL UNIQUE,
                task_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                result_data TEXT NOT NULL,
                metadata TEXT,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                execution_time_ms INTEGER,
                tokens_used INTEGER
            )
            "#,
            [],
        ).context("Failed to create execution_results table")?;
        
        // Таблица для связей между записями
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS fact_relations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_key TEXT NOT NULL,
                to_key TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                strength REAL DEFAULT 1.0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(from_key, to_key, relation_type)
            )
            "#,
            [],
        ).context("Failed to create fact_relations table")?;
        
        // Создаём индексы для оптимизации
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_facts_key ON facts(key)",
            "CREATE INDEX IF NOT EXISTS idx_facts_kind ON facts(kind)",
            "CREATE INDEX IF NOT EXISTS idx_facts_created_at ON facts(created_at)",
            "CREATE INDEX IF NOT EXISTS idx_facts_access_count ON facts(access_count)",
            "CREATE INDEX IF NOT EXISTS idx_facts_parent_id ON facts(parent_id)",
            "CREATE INDEX IF NOT EXISTS idx_execution_results_task_id ON execution_results(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_execution_results_step_id ON execution_results(step_id)",
            "CREATE INDEX IF NOT EXISTS idx_fact_relations_from_key ON fact_relations(from_key)",
            "CREATE INDEX IF NOT EXISTS idx_fact_relations_to_key ON fact_relations(to_key)",
        ];
        
        for index_sql in &indexes {
            conn.execute(index_sql, [])?;
        }
        
        tracing::info!("Initialized medium-term memory store");
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    
    /// Сохранить структурированный факт
    pub async fn put_fact(&self, key: &str, kind: &str, title: Option<&str>, body: &str, meta: &MemMeta, parent_id: Option<i64>) -> Result<i64> {
        let conn = self.conn.lock().await;
        let meta_json = serde_json::to_string(meta)?;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO facts (key, kind, title, body, meta, created_at, updated_at, accessed_at, parent_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![key, kind, title, body, meta_json, now, now, now, parent_id],
        )?;
        
        let fact_id = conn.last_insert_rowid();
        tracing::trace!("Stored fact in medium-term layer: {} (id: {})", key, fact_id);
        Ok(fact_id)
    }
    
    /// Получить факт по ключу
    pub async fn get_fact(&self, key: &str) -> Result<Option<(String, String, Option<String>, String, MemMeta)>> {
        let conn = self.conn.lock().await;
        
        let result = conn.query_row(
            r#"
            SELECT kind, title, body, meta FROM facts WHERE key = ?1
            "#,
            params![key],
            |row| {
                let kind: String = row.get(0)?;
                let title: Option<String> = row.get(1)?;
                let body: String = row.get(2)?;
                let meta_json: String = row.get(3)?;
                Ok((kind, title, body, meta_json))
            }
        ).optional()?;
        
        if let Some((kind, title, body, meta_json)) = result {
            // Обновляем статистику доступа
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE facts SET access_count = access_count + 1, accessed_at = ?1 WHERE key = ?2",
                params![now, key],
            )?;
            
            let mut meta: MemMeta = serde_json::from_str(&meta_json)?;
            meta.access_count += 1;
            meta.last_accessed = Utc::now();
            
            Ok(Some((kind, title.unwrap_or_default(), Some(body.clone()), body, meta)))
        } else {
            Ok(None)
        }
    }
    
    /// Сохранить результат выполнения задачи
    pub async fn put_execution_result(&self, key: &str, task_id: &str, step_id: &str, result_data: &str, metadata: Option<&str>, execution_time_ms: Option<u64>, tokens_used: Option<u32>) -> Result<i64> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO execution_results 
            (key, task_id, step_id, result_data, metadata, created_at, execution_time_ms, tokens_used)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![key, task_id, step_id, result_data, metadata, now, execution_time_ms.map(|t| t as i64), tokens_used.map(|t| t as i64)],
        )?;
        
        let result_id = conn.last_insert_rowid();
        tracing::trace!("Stored execution result: {} (task: {}, step: {})", key, task_id, step_id);
        Ok(result_id)
    }
    
    /// Получить результаты выполнения по task_id
    pub async fn get_execution_results(&self, task_id: &str) -> Result<Vec<(String, String, String, Option<String>)>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT key, step_id, result_data, metadata 
            FROM execution_results 
            WHERE task_id = ?1 
            ORDER BY created_at ASC
            "#
        )?;
        
        let rows = stmt.query_map(params![task_id], |row| {
            let key: String = row.get(0)?;
            let step_id: String = row.get(1)?;
            let result_data: String = row.get(2)?;
            let metadata: Option<String> = row.get(3)?;
            Ok((key, step_id, result_data, metadata))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
    
    /// Создать связь между фактами
    pub async fn add_relation(&self, from_key: &str, to_key: &str, relation_type: &str, strength: f32) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO fact_relations (from_key, to_key, relation_type, strength, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![from_key, to_key, relation_type, strength, now],
        )?;
        
        tracing::trace!("Added relation: {} -> {} ({})", from_key, to_key, relation_type);
        Ok(())
    }
    
    /// Получить связанные факты
    pub async fn get_related_facts(&self, key: &str, relation_type: Option<&str>) -> Result<Vec<(String, String, f32)>> {
        let conn = self.conn.lock().await;
        
        let (sql, params): (String, Vec<rusqlite::types::Value>) = if let Some(rel_type) = relation_type {
            (
                r#"
                SELECT to_key, relation_type, strength 
                FROM fact_relations 
                WHERE from_key = ?1 AND relation_type = ?2
                ORDER BY strength DESC
                "#.to_string(),
                vec![key.to_string().into(), rel_type.to_string().into()]
            )
        } else {
            (
                r#"
                SELECT to_key, relation_type, strength 
                FROM fact_relations 
                WHERE from_key = ?1
                ORDER BY strength DESC
                "#.to_string(),
                vec![key.to_string().into()]
            )
        };
        
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let to_key: String = row.get(0)?;
            let relation_type: String = row.get(1)?;
            let strength: f32 = row.get(2)?;
            Ok((to_key, relation_type, strength))
        })?;
        
        let mut relations = Vec::new();
        for row in rows {
            relations.push(row?);
        }
        
        Ok(relations)
    }
    
    /// Поиск фактов по типу и содержимому
    pub async fn search_facts(&self, kind: Option<&str>, query: Option<&str>, limit: usize) -> Result<Vec<(String, String, Option<String>, String)>> {
        let conn = self.conn.lock().await;
        
        let (sql, params): (String, Vec<rusqlite::types::Value>) = match (kind, query) {
            (Some(k), Some(q)) => {
                let query_pattern = format!("%{}%", q);
                (
                    r#"
                    SELECT key, kind, title, body 
                    FROM facts 
                    WHERE kind = ?1 AND (title LIKE ?2 OR body LIKE ?2)
                    ORDER BY access_count DESC, created_at DESC
                    LIMIT ?3
                    "#.to_string(),
                    vec![k.to_string().into(), query_pattern.into(), (limit as i64).into()]
                )
            },
            (Some(k), None) => (
                r#"
                SELECT key, kind, title, body 
                FROM facts 
                WHERE kind = ?1
                ORDER BY access_count DESC, created_at DESC
                LIMIT ?2
                "#.to_string(),
                vec![k.to_string().into(), (limit as i64).into()]
            ),
            (None, Some(q)) => {
                let query_pattern = format!("%{}%", q);
                (
                    r#"
                    SELECT key, kind, title, body 
                    FROM facts 
                    WHERE title LIKE ?1 OR body LIKE ?1
                    ORDER BY access_count DESC, created_at DESC
                    LIMIT ?2
                    "#.to_string(),
                    vec![query_pattern.into(), (limit as i64).into()]
                )
            },
            (None, None) => (
                r#"
                SELECT key, kind, title, body 
                FROM facts 
                ORDER BY access_count DESC, created_at DESC
                LIMIT ?1
                "#.to_string(),
                vec![(limit as i64).into()]
            ),
        };
        
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let key: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let title: Option<String> = row.get(2)?;
            let body: String = row.get(3)?;
            Ok((key, kind, title, body))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
}

#[async_trait]
impl MemoryStore for MediumTermStore {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        // Для базового интерфейса сохраняем как факт типа "raw"
        let body = String::from_utf8_lossy(data);
        self.put_fact(key, "raw", None, &body, meta, None).await?;
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        if let Some((_, _, _, body, meta)) = self.get_fact(key).await? {
            Ok(Some((body.into_bytes(), meta)))
        } else {
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        
        let removed = conn.execute(
            "DELETE FROM facts WHERE key = ?1",
            params![key],
        )?;
        
        Ok(removed > 0)
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock().await;
        
        let exists = conn.query_row(
            "SELECT 1 FROM facts WHERE key = ?1",
            params![key],
            |_| Ok(())
        ).optional()?.is_some();
        
        Ok(exists)
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare("SELECT key FROM facts ORDER BY created_at DESC")?;
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
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(body)), 0) FROM facts",
            [],
            |row| {
                let count: i64 = row.get(0)?;
                let size: i64 = row.get(1)?;
                Ok((count as u64, size as u64))
            }
        )?;
        
        let oldest_item = conn.query_row(
            "SELECT MIN(created_at) FROM facts",
            [],
            |row| {
                let created_at_str: Option<String> = row.get(0)?;
                Ok(created_at_str)
            }
        ).optional()?.flatten().and_then(|s| {
            DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
        });
        
        let newest_item = conn.query_row(
            "SELECT MAX(created_at) FROM facts",
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
                "SELECT AVG(CAST(access_count AS REAL)) FROM facts",
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
    async fn test_medium_term_facts() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = MediumTermStore::new(temp_file.path()).await.unwrap();
        
        let key = "test_fact";
        let kind = "knowledge";
        let title = Some("Test Knowledge");
        let body = "This is a test fact";
        let meta = MemMeta::default();
        
        // Test put_fact
        let fact_id = store.put_fact(key, kind, title, body, &meta, None).await.unwrap();
        assert!(fact_id > 0);
        
        // Test get_fact
        let result = store.get_fact(key).await.unwrap();
        assert!(result.is_some());
        let (retrieved_kind, retrieved_title, retrieved_body, _retrieved_text, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_kind, kind);
        assert_eq!(retrieved_title, title.unwrap_or_default());
        assert_eq!(retrieved_body, Some(body.to_string()));
        assert_eq!(retrieved_meta.access_count, 1);
    }
    
    #[tokio::test]
    async fn test_medium_term_execution_results() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = MediumTermStore::new(temp_file.path()).await.unwrap();
        
        let task_id = "task_123";
        let step_id = "step_1";
        let key = "result_key";
        let result_data = "execution completed successfully";
        
        // Test put_execution_result
        let result_id = store.put_execution_result(key, task_id, step_id, result_data, None, Some(1500), Some(100)).await.unwrap();
        assert!(result_id > 0);
        
        // Test get_execution_results
        let results = store.get_execution_results(task_id).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, key);
        assert_eq!(results[0].1, step_id);
        assert_eq!(results[0].2, result_data);
    }
    
    #[tokio::test]
    async fn test_medium_term_relations() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = MediumTermStore::new(temp_file.path()).await.unwrap();
        
        let from_key = "fact_a";
        let to_key = "fact_b";
        let relation_type = "depends_on";
        let strength = 0.8;
        
        // Test add_relation
        store.add_relation(from_key, to_key, relation_type, strength).await.unwrap();
        
        // Test get_related_facts
        let relations = store.get_related_facts(from_key, Some(relation_type)).await.unwrap();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].0, to_key);
        assert_eq!(relations[0].1, relation_type);
        assert_eq!(relations[0].2, strength);
    }
}