use anyhow::Result;
use magray_core::{DocStore, TodoItem, TaskState};
use rusqlite::{Connection, params, Row};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use tracing::{debug, info, error};
use chrono::{DateTime, Utc};

pub struct TodoService {
    conn: Arc<Mutex<Connection>>,
}

impl TodoService {
    pub fn new(docstore: &DocStore) -> Result<Self> {
        let conn = Connection::open(&docstore.tasks_path())?;
        
        // Initialize todos table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                desc TEXT NOT NULL DEFAULT '',
                state TEXT NOT NULL DEFAULT 'planned',
                priority INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                deps TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                due_at INTEGER,
                last_touch INTEGER NOT NULL,
                staleness REAL NOT NULL DEFAULT 0.0
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_todos_state ON todos(state)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_todos_priority ON todos(priority DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_todos_staleness ON todos(staleness DESC)",
            [],
        )?;

        debug!("TodoService initialized: {}", docstore.tasks_path().display());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn add(&self, mut todo: TodoItem) -> Result<TodoItem> {
        todo.touch(); // Update staleness
        
        let conn = self.conn.lock().unwrap();
        
        let tags_json = serde_json::to_string(&todo.tags)?;
        let deps_json = serde_json::to_string(&todo.deps)?;
        
        conn.execute(
            "INSERT INTO todos (id, title, desc, state, priority, tags, deps, created_at, due_at, last_touch, staleness)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                todo.id.to_string(),
                todo.title,
                todo.desc,
                todo.state.to_string(),
                todo.priority,
                tags_json,
                deps_json,
                todo.created_at.timestamp(),
                todo.due_at.map(|d| d.timestamp()),
                todo.last_touch.timestamp(),
                todo.staleness,
            ],
        )?;

        info!("Added todo: {} ({})", todo.title, todo.id);
        Ok(todo)
    }

    pub async fn get(&self, id: &Uuid) -> Result<Option<TodoItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, desc, state, priority, tags, deps, created_at, due_at, last_touch, staleness 
             FROM todos WHERE id = ?"
        )?;
        
        let mut rows = stmt.query_map([id.to_string()], |row| {
            self.row_to_todo(row)
        })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub async fn list(&self, state_filter: Option<TaskState>, sort_by: &str, limit: Option<usize>) -> Result<Vec<TodoItem>> {
        let mut query = "SELECT id, title, desc, state, priority, tags, deps, created_at, due_at, last_touch, staleness FROM todos".to_string();
        let mut params = Vec::new();

        if let Some(ref state) = state_filter {
            query.push_str(" WHERE state = ?");
            params.push(state.to_string());
        }

        // Add sorting
        match sort_by {
            "priority" => query.push_str(" ORDER BY priority DESC, created_at DESC"),
            "created" => query.push_str(" ORDER BY created_at DESC"),
            "due" => query.push_str(" ORDER BY due_at ASC, priority DESC"),
            "staleness" => query.push_str(" ORDER BY staleness DESC"),
            _ => query.push_str(" ORDER BY priority DESC, created_at DESC"),
        }

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query)?;
        
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let rows = stmt.query_map(&param_refs[..], |row| {
            self.row_to_todo(row)
        })?;

        let mut todos = Vec::new();
        for row in rows {
            todos.push(row?);
        }

        debug!("Listed {} todos with filter {:?}, sort {}", todos.len(), state_filter, sort_by);
        Ok(todos)
    }

    pub async fn update_state(&self, id: &Uuid, new_state: TaskState) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        
        let changes = conn.execute(
            "UPDATE todos SET state = ?, last_touch = ? WHERE id = ?",
            params![new_state.to_string(), now, id.to_string()],
        )?;

        if changes > 0 {
            info!("Updated todo {} state to {}", id, new_state);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn update_staleness(&self, id: &Uuid) -> Result<bool> {
        if let Some(mut todo) = self.get(id).await? {
            todo.update_staleness();
            
            let conn = self.conn.lock().unwrap();
            let changes = conn.execute(
                "UPDATE todos SET staleness = ?, last_touch = ? WHERE id = ?",
                params![todo.staleness, todo.last_touch.timestamp(), id.to_string()],
            )?;

            Ok(changes > 0)
        } else {
            Ok(false)
        }
    }

    pub async fn touch(&self, id: &Uuid) -> Result<bool> {
        let now = Utc::now().timestamp();
        
        let conn = self.conn.lock().unwrap();
        let changes = conn.execute(
            "UPDATE todos SET last_touch = ? WHERE id = ?",
            params![now, id.to_string()],
        )?;

        if changes > 0 {
            // Recalculate staleness
            self.update_staleness(id).await?;
            debug!("Touched todo: {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn delete(&self, id: &Uuid) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changes = conn.execute("DELETE FROM todos WHERE id = ?", [id.to_string()])?;
        
        if changes > 0 {
            info!("Deleted todo: {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn prune_stale(&self, staleness_threshold: f32) -> Result<usize> {
        // First update all staleness scores
        let todos = self.list(None, "staleness", None).await?;
        for todo in &todos {
            self.update_staleness(&todo.id).await?;
        }

        // Delete stale todos
        let conn = self.conn.lock().unwrap();
        let changes = conn.execute(
            "DELETE FROM todos WHERE staleness > ? AND state IN ('done', 'archived')",
            [staleness_threshold],
        )?;

        info!("Pruned {} stale todos (staleness > {})", changes, staleness_threshold);
        Ok(changes)
    }

    pub async fn find_by_title_prefix(&self, prefix: &str) -> Result<Vec<TodoItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, desc, state, priority, tags, deps, created_at, due_at, last_touch, staleness 
             FROM todos WHERE title LIKE ? ORDER BY priority DESC"
        )?;
        
        let rows = stmt.query_map([format!("{}%", prefix)], |row| {
            self.row_to_todo(row)
        })?;

        let mut todos = Vec::new();
        for row in rows {
            todos.push(row?);
        }

        Ok(todos)
    }

    pub async fn count_by_state(&self) -> Result<std::collections::HashMap<TaskState, usize>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT state, COUNT(*) FROM todos GROUP BY state")?;
        
        let rows = stmt.query_map([], |row| {
            let state_str: String = row.get(0)?;
            let count: usize = row.get(1)?;
            
            let state = match state_str.as_str() {
                "planned" => TaskState::Planned,
                "ready" => TaskState::Ready,
                "in-progress" => TaskState::InProgress,
                "blocked" => TaskState::Blocked,
                "done" => TaskState::Done,
                "archived" => TaskState::Archived,
                _ => TaskState::Planned,
            };
            
            Ok((state, count))
        })?;

        let mut counts = std::collections::HashMap::new();
        for row in rows {
            let (state, count) = row?;
            counts.insert(state, count);
        }

        Ok(counts)
    }

    fn row_to_todo(&self, row: &Row) -> rusqlite::Result<TodoItem> {
        let id_str: String = row.get(0)?;
        let title: String = row.get(1)?;
        let desc: String = row.get(2)?;
        let state_str: String = row.get(3)?;
        let priority: i32 = row.get(4)?;
        let tags_json: String = row.get(5)?;
        let deps_json: String = row.get(6)?;
        let created_at: i64 = row.get(7)?;
        let due_at: Option<i64> = row.get(8)?;
        let last_touch: i64 = row.get(9)?;
        let staleness: f32 = row.get(10)?;

        let id = Uuid::parse_str(&id_str).map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            0, rusqlite::types::Type::Text, Box::new(e)
        ))?;

        let state = match state_str.as_str() {
            "planned" => TaskState::Planned,
            "ready" => TaskState::Ready,
            "in-progress" => TaskState::InProgress,
            "blocked" => TaskState::Blocked,
            "done" => TaskState::Done,
            "archived" => TaskState::Archived,
            _ => TaskState::Planned,
        };

        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let deps: Vec<Uuid> = serde_json::from_str(&deps_json).unwrap_or_default();

        Ok(TodoItem {
            id,
            title,
            desc,
            state,
            priority,
            tags,
            deps,
            created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
            due_at: due_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            last_touch: DateTime::from_timestamp(last_touch, 0).unwrap_or_else(Utc::now),
            staleness,
        })
    }
}