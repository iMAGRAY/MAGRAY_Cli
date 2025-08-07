use crate::types::*;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// SQLite хранилище для задач
pub struct TodoStore {
    conn: Arc<Mutex<Connection>>,
}

impl TodoStore {
    /// Создать новое хранилище
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref()).context("Failed to open SQLite database")?;

        // Создаем таблицы
        conn.execute_batch(
            r#"
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
                tool_hint TEXT
            );
            
            CREATE TABLE IF NOT EXISTS todo_dependencies (
                task_id TEXT NOT NULL,
                depends_on TEXT NOT NULL,
                PRIMARY KEY (task_id, depends_on),
                FOREIGN KEY (task_id) REFERENCES todos(id) ON DELETE CASCADE,
                FOREIGN KEY (depends_on) REFERENCES todos(id) ON DELETE CASCADE
            );
            
            CREATE TABLE IF NOT EXISTS todo_tags (
                task_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (task_id, tag),
                FOREIGN KEY (task_id) REFERENCES todos(id) ON DELETE CASCADE
            );
            
            CREATE INDEX IF NOT EXISTS idx_todos_state ON todos(state);
            CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);
            CREATE INDEX IF NOT EXISTS idx_todos_created ON todos(created_at);
            "#,
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Создать новую задачу
    pub async fn create(&self, mut task: TodoItem) -> Result<TodoItem> {
        task.id = Uuid::new_v4();
        task.created_at = Utc::now();
        task.updated_at = Utc::now();

        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO todos (
                id, title, description, state, priority,
                created_at, updated_at, started_at, completed_at, due_date,
                parent_id, auto_generated, confidence, reasoning, tool_hint
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
            ],
        )?;

        // Сохраняем зависимости
        for dep in &task.depends_on {
            conn.execute(
                "INSERT INTO todo_dependencies (task_id, depends_on) VALUES (?1, ?2)",
                params![task.id.to_string(), dep.to_string()],
            )?;
        }

        // Сохраняем теги
        for tag in &task.tags {
            conn.execute(
                "INSERT INTO todo_tags (task_id, tag) VALUES (?1, ?2)",
                params![task.id.to_string(), tag],
            )?;
        }

        Ok(task)
    }

    /// Получить задачу по ID
    pub async fn get(&self, id: &Uuid) -> Result<Option<TodoItem>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id, title, description, state, priority,
                    created_at, updated_at, started_at, completed_at, due_date,
                    parent_id, auto_generated, confidence, reasoning, tool_hint
             FROM todos WHERE id = ?1",
        )?;

        let task = stmt
            .query_row(params![id.to_string()], |row| {
                Ok(TodoItem {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
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
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    started_at: row.get::<_, Option<String>>(7)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    completed_at: row.get::<_, Option<String>>(8)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    due_date: row.get::<_, Option<String>>(9)?.map(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .unwrap()
                            .with_timezone(&Utc)
                    }),
                    parent_id: row
                        .get::<_, Option<String>>(10)?
                        .map(|s| Uuid::parse_str(&s).unwrap()),
                    auto_generated: row.get(11)?,
                    confidence: row.get(12)?,
                    reasoning: row.get(13)?,
                    tool_hint: row.get(14)?,
                    ..Default::default()
                })
            })
            .optional()?;

        if let Some(mut task) = task {
            // Загружаем зависимости
            let mut dep_stmt =
                conn.prepare("SELECT depends_on FROM todo_dependencies WHERE task_id = ?1")?;

            task.depends_on = dep_stmt
                .query_map(params![id.to_string()], |row| {
                    Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
                })?
                .collect::<Result<Vec<_>, _>>()?;

            // Загружаем теги
            let mut tag_stmt = conn.prepare("SELECT tag FROM todo_tags WHERE task_id = ?1")?;

            task.tags = tag_stmt
                .query_map(params![id.to_string()], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Найти задачи по состоянию
    pub async fn find_by_state(&self, state: TaskState) -> Result<Vec<TodoItem>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id FROM todos WHERE state = ?1 ORDER BY priority DESC, created_at ASC",
        )?;

        let ids: Vec<Uuid> = stmt
            .query_map(params![state.to_string()], |row| {
                Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut tasks = Vec::new();
        for id in ids {
            if let Some(task) = self.get(&id).await? {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// Обновить состояние задачи
    pub async fn update_state(&self, id: &Uuid, new_state: TaskState) -> Result<()> {
        let now = Utc::now();
        let conn = self.conn.lock().await;

        // Обновляем временные метки в зависимости от состояния
        match new_state {
            TaskState::InProgress => {
                conn.execute(
                    "UPDATE todos SET state = ?1, started_at = ?2, updated_at = ?3 WHERE id = ?4",
                    params![
                        new_state.to_string(),
                        now.to_rfc3339(),
                        now.to_rfc3339(),
                        id.to_string()
                    ],
                )?;
            }
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                conn.execute(
                    "UPDATE todos SET state = ?1, completed_at = ?2, updated_at = ?3 WHERE id = ?4",
                    params![
                        new_state.to_string(),
                        now.to_rfc3339(),
                        now.to_rfc3339(),
                        id.to_string()
                    ],
                )?;
            }
            _ => {
                conn.execute(
                    "UPDATE todos SET state = ?1, updated_at = ?2 WHERE id = ?3",
                    params![new_state.to_string(), now.to_rfc3339(), id.to_string()],
                )?;
            }
        }

        Ok(())
    }

    /// Найти все активные задачи
    pub async fn get_active(&self) -> Result<Vec<TodoItem>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT id FROM todos 
             WHERE state NOT IN ('done', 'failed', 'cancelled') 
             ORDER BY priority DESC, created_at ASC",
        )?;

        let ids: Vec<Uuid> = stmt
            .query_map(params![], |row| {
                Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut tasks = Vec::new();
        for id in ids {
            if let Some(task) = self.get(&id).await? {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// Получить подзадачи
    pub async fn get_subtasks(&self, parent_id: &Uuid) -> Result<Vec<TodoItem>> {
        let conn = self.conn.lock().await;

        let mut stmt =
            conn.prepare("SELECT id FROM todos WHERE parent_id = ?1 ORDER BY created_at ASC")?;

        let ids: Vec<Uuid> = stmt
            .query_map(params![parent_id.to_string()], |row| {
                Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);
        drop(conn);

        let mut tasks = Vec::new();
        for id in ids {
            if let Some(task) = self.get(&id).await? {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    /// Добавить зависимость
    pub async fn add_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT OR IGNORE INTO todo_dependencies (task_id, depends_on) VALUES (?1, ?2)",
            params![task_id.to_string(), depends_on.to_string()],
        )?;

        Ok(())
    }

    /// Удалить зависимость
    pub async fn remove_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "DELETE FROM todo_dependencies WHERE task_id = ?1 AND depends_on = ?2",
            params![task_id.to_string(), depends_on.to_string()],
        )?;

        Ok(())
    }

    /// Получить задачи, которые зависят от данной
    pub async fn get_dependent_tasks(&self, task_id: &Uuid) -> Result<Vec<Uuid>> {
        let conn = self.conn.lock().await;

        let mut stmt =
            conn.prepare("SELECT task_id FROM todo_dependencies WHERE depends_on = ?1")?;

        let ids: Vec<Uuid> = stmt
            .query_map(params![task_id.to_string()], |row| {
                Ok(Uuid::parse_str(&row.get::<_, String>(0)?).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }
}
