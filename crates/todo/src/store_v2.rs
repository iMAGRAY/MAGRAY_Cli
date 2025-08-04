use crate::types::*;
use memory::Layer;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Row, OptionalExtension};
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, instrument};
use uuid::Uuid;

type DbPool = Pool<SqliteConnectionManager>;

/// Оптимизированное хранилище задач с батчевыми операциями
pub struct TodoStoreV2 {
    pool: Arc<DbPool>,
}

impl TodoStoreV2 {
    /// Создать новое хранилище с пулом соединений
    pub async fn new<P: AsRef<Path>>(path: P, pool_size: u32) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path.as_ref());
        let pool = Pool::builder()
            .max_size(pool_size)
            .build(manager)
            .context("Failed to create connection pool")?;
        
        // Инициализируем схему
        {
            let conn = pool.get()?;
            Self::init_schema(&conn)?;
        }
        
        Ok(Self {
            pool: Arc::new(pool),
        })
    }
    
    /// Инициализация схемы с оптимизированными индексами
    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            -- Основная таблица задач
            CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                state TEXT NOT NULL,
                priority INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                due_date TEXT,
                parent_id TEXT,
                auto_generated BOOLEAN NOT NULL DEFAULT 0,
                confidence REAL NOT NULL DEFAULT 1.0,
                reasoning TEXT,
                tool_hint TEXT,
                tool_params TEXT,
                metadata TEXT
            );
            
            -- Таблица зависимостей
            CREATE TABLE IF NOT EXISTS todo_dependencies (
                task_id TEXT NOT NULL,
                depends_on TEXT NOT NULL,
                PRIMARY KEY (task_id, depends_on),
                FOREIGN KEY (task_id) REFERENCES todos(id) ON DELETE CASCADE,
                FOREIGN KEY (depends_on) REFERENCES todos(id) ON DELETE CASCADE
            );
            
            -- Таблица тегов
            CREATE TABLE IF NOT EXISTS todo_tags (
                task_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (task_id, tag),
                FOREIGN KEY (task_id) REFERENCES todos(id) ON DELETE CASCADE
            );
            
            -- Таблица контекстных ссылок
            CREATE TABLE IF NOT EXISTS todo_context_refs (
                task_id TEXT NOT NULL,
                mem_layer TEXT NOT NULL,
                mem_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (task_id, mem_key),
                FOREIGN KEY (task_id) REFERENCES todos(id) ON DELETE CASCADE
            );
            
            -- Оптимизированные индексы
            CREATE INDEX IF NOT EXISTS idx_todos_state_priority ON todos(state, priority DESC, created_at ASC);
            CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id) WHERE parent_id IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_todos_updated ON todos(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_todos_due_date ON todos(due_date) WHERE due_date IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_deps_depends_on ON todo_dependencies(depends_on);
            CREATE INDEX IF NOT EXISTS idx_tags_tag ON todo_tags(tag);
            
            -- Материализованное представление для быстрого поиска готовых задач
            CREATE VIEW IF NOT EXISTS ready_tasks AS
            SELECT t.* 
            FROM todos t
            WHERE t.state = 'ready'
            AND NOT EXISTS (
                SELECT 1 FROM todo_dependencies d
                JOIN todos dep ON d.depends_on = dep.id
                WHERE d.task_id = t.id
                AND dep.state != 'done'
            );
            
            -- Триггер для автоматического обновления updated_at
            CREATE TRIGGER IF NOT EXISTS update_todo_timestamp 
            AFTER UPDATE ON todos
            BEGIN
                UPDATE todos SET updated_at = datetime('now') WHERE id = NEW.id;
            END;
            
            -- Включаем оптимизации SQLite
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = 10000;
            PRAGMA temp_store = MEMORY;
            "#
        )?;
        
        Ok(())
    }
    
    /// Создать задачу с батчевой вставкой связанных данных
    #[instrument(skip(self, task))]
    pub async fn create(&self, mut task: TodoItem) -> Result<TodoItem> {
        task.id = Uuid::new_v4();
        task.created_at = Utc::now();
        task.updated_at = Utc::now();
        
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        // Вставляем основную задачу
        tx.execute(
            "INSERT INTO todos (
                id, title, description, state, priority,
                created_at, updated_at, started_at, completed_at, due_date,
                parent_id, auto_generated, confidence, reasoning, 
                tool_hint, tool_params, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                task.id.to_string(),
                task.title,
                task.description,
                task.state.to_string(),
                task.priority as i32,
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.started_at.map(|d| d.to_rfc3339()),
                task.completed_at.map(|d| d.to_rfc3339()),
                task.due_date.map(|d| d.to_rfc3339()),
                task.parent_id.map(|id| id.to_string()),
                task.auto_generated,
                task.confidence,
                task.reasoning,
                task.tool_hint,
                task.tool_params.as_ref().map(|p| serde_json::to_string(p).unwrap()),
                serde_json::to_string(&task.metadata)?,
            ],
        )?;
        
        // Батчевая вставка зависимостей
        if !task.depends_on.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO todo_dependencies (task_id, depends_on) VALUES (?1, ?2)"
            )?;
            
            for dep in &task.depends_on {
                stmt.execute(params![task.id.to_string(), dep.to_string()])?;
            }
        }
        
        // Батчевая вставка тегов
        if !task.tags.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO todo_tags (task_id, tag) VALUES (?1, ?2)"
            )?;
            
            for tag in &task.tags {
                stmt.execute(params![task.id.to_string(), tag])?;
            }
        }
        
        // Батчевая вставка контекстных ссылок
        if !task.context_refs.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO todo_context_refs (task_id, mem_layer, mem_key, created_at) VALUES (?1, ?2, ?3, ?4)"
            )?;
            
            for mem_ref in &task.context_refs {
                stmt.execute(params![
                    task.id.to_string(),
                    format!("{:?}", mem_ref.layer),
                    mem_ref.record_id.to_string(),
                    mem_ref.created_at.to_rfc3339()
                ])?;
            }
        }
        
        tx.commit()?;
        
        debug!("Created task {} with {} dependencies", task.id, task.depends_on.len());
        Ok(task)
    }
    
    /// Получить задачу со всеми связанными данными одним запросом
    pub async fn get(&self, id: &Uuid) -> Result<Option<TodoItem>> {
        let conn = self.pool.get()?;
        
        // Используем CTE для эффективной загрузки всех данных
        let query = r#"
            WITH task_data AS (
                SELECT * FROM todos WHERE id = ?1
            )
            SELECT 
                t.*,
                (
                    SELECT json_group_array(depends_on) 
                    FROM todo_dependencies 
                    WHERE task_id = t.id
                ) as dependencies,
                (
                    SELECT json_group_array(tag) 
                    FROM todo_tags 
                    WHERE task_id = t.id
                ) as tags,
                (
                    SELECT json_group_array(
                        json_object('layer', mem_layer, 'key', mem_key, 'created_at', created_at)
                    ) 
                    FROM todo_context_refs 
                    WHERE task_id = t.id
                ) as context_refs
            FROM task_data t
        "#;
        
        let result = conn.query_row(query, params![id.to_string()], |row| {
            Self::parse_todo_row(row)
        }).optional()?;
        
        Ok(result)
    }
    
    /// Батчевая загрузка задач
    pub async fn get_batch(&self, ids: &[Uuid]) -> Result<Vec<TodoItem>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let conn = self.pool.get()?;
        
        // Формируем список ID для SQL
        let id_list = ids.iter()
<<<<<<< HEAD
            .map(|id| format!("'{id}'"))
=======
            .map(|id| format!("'{}'", id))
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            .collect::<Vec<_>>()
            .join(",");
        
        let query = format!(
            r#"
            SELECT 
                t.*,
                (
                    SELECT json_group_array(depends_on) 
                    FROM todo_dependencies 
                    WHERE task_id = t.id
                ) as dependencies,
                (
                    SELECT json_group_array(tag) 
                    FROM todo_tags 
                    WHERE task_id = t.id
                ) as tags,
                (
                    SELECT json_group_array(
                        json_object('layer', mem_layer, 'key', mem_key, 'created_at', created_at)
                    ) 
                    FROM todo_context_refs 
                    WHERE task_id = t.id
                ) as context_refs
            FROM todos t
<<<<<<< HEAD
            WHERE t.id IN ({id_list})
            "#
        );
        
        let mut stmt = conn.prepare(&query)?;
        let tasks = stmt.query_map(params![], Self::parse_todo_row)?
=======
            WHERE t.id IN ({})
            "#,
            id_list
        );
        
        let mut stmt = conn.prepare(&query)?;
        let tasks = stmt.query_map(params![], |row| Self::parse_todo_row(row))?
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(tasks)
    }
    
    /// Найти готовые к выполнению задачи с учетом зависимостей
    pub async fn find_ready_tasks(&self, limit: usize) -> Result<Vec<TodoItem>> {
        let conn = self.pool.get()?;
        
        let query = r#"
            SELECT 
                t.*,
                (
                    SELECT json_group_array(depends_on) 
                    FROM todo_dependencies 
                    WHERE task_id = t.id
                ) as dependencies,
                (
                    SELECT json_group_array(tag) 
                    FROM todo_tags 
                    WHERE task_id = t.id
                ) as tags,
                (
                    SELECT json_group_array(
                        json_object('layer', mem_layer, 'key', mem_key, 'created_at', created_at)
                    ) 
                    FROM todo_context_refs 
                    WHERE task_id = t.id
                ) as context_refs
            FROM todos t
            WHERE t.state = 'ready'
            AND NOT EXISTS (
                SELECT 1 FROM todo_dependencies d
                JOIN todos dep ON d.depends_on = dep.id
                WHERE d.task_id = t.id
                AND dep.state != 'done'
            )
            ORDER BY t.priority DESC, t.created_at ASC
            LIMIT ?1
        "#;
        
        let mut stmt = conn.prepare(query)?;
<<<<<<< HEAD
        let tasks = stmt.query_map(params![limit as i64], Self::parse_todo_row)?
=======
        let tasks = stmt.query_map(params![limit as i64], |row| Self::parse_todo_row(row))?
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(tasks)
    }
    
    /// Обновить состояние с каскадным обновлением зависимых задач
    pub async fn update_state_cascade(&self, id: &Uuid, new_state: TaskState) -> Result<Vec<Uuid>> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        
        let now = Utc::now();
        let mut affected_ids = vec![*id];
        
        // Обновляем основную задачу
        match new_state {
            TaskState::InProgress => {
                tx.execute(
                    "UPDATE todos SET state = ?1, started_at = ?2 WHERE id = ?3",
                    params![new_state.to_string(), now.to_rfc3339(), id.to_string()],
                )?;
            }
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                tx.execute(
                    "UPDATE todos SET state = ?1, completed_at = ?2 WHERE id = ?3",
                    params![new_state.to_string(), now.to_rfc3339(), id.to_string()],
                )?;
            }
            _ => {
                tx.execute(
                    "UPDATE todos SET state = ?1 WHERE id = ?2",
                    params![new_state.to_string(), id.to_string()],
                )?;
            }
        }
        
        // Если задача выполнена, обновляем зависимые
        if new_state == TaskState::Done {
            let query = r#"
                UPDATE todos 
                SET state = 'ready'
                WHERE id IN (
                    SELECT DISTINCT t.id 
                    FROM todos t
                    JOIN todo_dependencies d ON t.id = d.task_id
                    WHERE d.depends_on = ?1
                    AND t.state = 'blocked'
                    AND NOT EXISTS (
                        SELECT 1 FROM todo_dependencies d2
                        JOIN todos dep ON d2.depends_on = dep.id
                        WHERE d2.task_id = t.id
                        AND dep.id != ?1
                        AND dep.state != 'done'
                    )
                )
                RETURNING id
            "#;
            
            let mut stmt = tx.prepare(query)?;
            let updated: Vec<String> = stmt.query_map(params![id.to_string()], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            
            for id_str in updated {
                if let Ok(uuid) = Uuid::parse_str(&id_str) {
                    affected_ids.push(uuid);
                }
            }
        }
        
        tx.commit()?;
        Ok(affected_ids)
    }
    
    /// Поиск задач с полнотекстовым поиском
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<TodoItem>> {
        let conn = self.pool.get()?;
        
        // Используем LIKE для простого поиска (можно заменить на FTS5)
<<<<<<< HEAD
        let search_pattern = format!("%{query}%");
=======
        let search_pattern = format!("%{}%", query);
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        
        let sql = r#"
            SELECT 
                t.*,
                (
                    SELECT json_group_array(depends_on) 
                    FROM todo_dependencies 
                    WHERE task_id = t.id
                ) as dependencies,
                (
                    SELECT json_group_array(tag) 
                    FROM todo_tags 
                    WHERE task_id = t.id
                ) as tags,
                (
                    SELECT json_group_array(
                        json_object('layer', mem_layer, 'key', mem_key, 'created_at', created_at)
                    ) 
                    FROM todo_context_refs 
                    WHERE task_id = t.id
                ) as context_refs
            FROM todos t
            WHERE t.title LIKE ?1 OR t.description LIKE ?1
            OR EXISTS (SELECT 1 FROM todo_tags WHERE task_id = t.id AND tag LIKE ?1)
            ORDER BY 
                CASE 
                    WHEN t.title LIKE ?1 THEN 1
                    WHEN t.description LIKE ?1 THEN 2
                    ELSE 3
                END,
                t.updated_at DESC
            LIMIT ?2
        "#;
        
        let mut stmt = conn.prepare(sql)?;
        let tasks = stmt.query_map(params![search_pattern, limit as i64], |row| {
            Self::parse_todo_row(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(tasks)
    }
    
    /// Получить статистику по задачам
    pub async fn get_stats(&self) -> Result<TaskStats> {
        let conn = self.pool.get()?;
        
        let query = r#"
            SELECT 
                state,
                COUNT(*) as count
            FROM todos
            WHERE state NOT IN ('cancelled')
            GROUP BY state
        "#;
        
        let mut stmt = conn.prepare(query)?;
        let counts: HashMap<String, usize> = stmt.query_map(params![], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
        })?
        .collect::<Result<HashMap<_, _>, _>>()?;
        
        Ok(TaskStats {
            total: counts.values().sum(),
            planned: counts.get("planned").copied().unwrap_or(0),
            ready: counts.get("ready").copied().unwrap_or(0),
            in_progress: counts.get("in_progress").copied().unwrap_or(0),
            blocked: counts.get("blocked").copied().unwrap_or(0),
            done: counts.get("done").copied().unwrap_or(0),
            failed: counts.get("failed").copied().unwrap_or(0),
            cancelled: counts.get("cancelled").copied().unwrap_or(0),
        })
    }
    
    /// Парсинг строки результата в TodoItem
    fn parse_todo_row(row: &Row) -> rusqlite::Result<TodoItem> {
        let id = Uuid::parse_str(&row.get::<_, String>(0)?).unwrap();
        
        // Парсим JSON массивы
        let deps_json: Option<String> = row.get("dependencies")?;
        let dependencies = if let Some(json) = deps_json {
            serde_json::from_str::<Vec<String>>(&json)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|s| Uuid::parse_str(&s).ok())
                .collect()
        } else {
            Vec::new()
        };
        
        let tags_json: Option<String> = row.get("tags")?;
        let tags = if let Some(json) = tags_json {
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        let context_refs_json: Option<String> = row.get("context_refs")?;
        let context_refs = if let Some(json) = context_refs_json {
            serde_json::from_str::<Vec<serde_json::Value>>(&json)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| {
                    let layer = v["layer"].as_str()?;
                    let key = v["key"].as_str()?;
                    let created_at = v["created_at"].as_str()?;
                    
                    Some(MemoryReference {
                        layer: match layer {
                            "Interact" => Layer::Interact,
                            "Insights" => Layer::Insights, 
                            "Assets" => Layer::Assets,
                            // Legacy compatibility
                            "Ephemeral" => Layer::Interact,
                            "Short" => Layer::Interact,
                            "Medium" => Layer::Insights,
                            "Long" => Layer::Insights,
                            "Semantic" => Layer::Assets,
                            _ => return None,
                        },
                        record_id: uuid::Uuid::parse_str(key).ok()?,
                        created_at: DateTime::parse_from_rfc3339(created_at).ok()?.with_timezone(&Utc),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        
        let tool_params_json: Option<String> = row.get(15)?;
        let tool_params = tool_params_json
            .and_then(|json| serde_json::from_str(&json).ok());
        
        let metadata_json: String = row.get(16)?;
        let metadata = serde_json::from_str(&metadata_json).unwrap_or_default();
        
        Ok(TodoItem {
            id,
            title: row.get(1)?,
            description: row.get(2)?,
            state: row.get::<_, String>(3)?.parse().unwrap(),
            priority: match row.get::<_, i32>(4)? {
                1 => Priority::Low,
                2 => Priority::Medium,
                3 => Priority::High,
                4 => Priority::Critical,
                _ => Priority::Medium,
            },
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z").unwrap())
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                .unwrap_or_else(|_| DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z").unwrap())
                .with_timezone(&Utc),
            started_at: row.get::<_, Option<String>>(7)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            completed_at: row.get::<_, Option<String>>(8)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            due_date: row.get::<_, Option<String>>(9)?.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            parent_id: row.get::<_, Option<String>>(10)?.and_then(|s| Uuid::parse_str(&s).ok()),
            auto_generated: row.get(11)?,
            confidence: row.get(12)?,
            reasoning: row.get(13)?,
            tool_hint: row.get(14)?,
            tool_params,
            metadata,
            depends_on: dependencies,
            blocks: Vec::new(), // Вычисляется отдельно при необходимости
            context_refs,
            artifacts: Vec::new(), // Загружается отдельно при необходимости
            tags,
        })
    }
    
    /// Добавить зависимость между задачами
    pub async fn add_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            "INSERT OR IGNORE INTO todo_dependencies (task_id, depends_on) VALUES (?1, ?2)",
            params![task_id.to_string(), depends_on.to_string()],
        )?;
        
        debug!("Добавлена зависимость: {} -> {}", task_id, depends_on);
        Ok(())
    }
    
    /// Удалить зависимость между задачами
    pub async fn remove_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            "DELETE FROM todo_dependencies WHERE task_id = ?1 AND depends_on = ?2",
            params![task_id.to_string(), depends_on.to_string()],
        )?;
        
        debug!("Удалена зависимость: {} -> {}", task_id, depends_on);
        Ok(())
    }
}