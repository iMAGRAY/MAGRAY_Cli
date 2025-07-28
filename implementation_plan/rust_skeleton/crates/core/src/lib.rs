use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

// === Project & Request ===

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl ProjectId {
    pub fn from_path(path: &std::path::Path) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Self(hash[..16].to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Uuid,
    pub goal: String,
    pub params: serde_json::Value,
    pub project_id: ProjectId,
    pub created_at: DateTime<Utc>,
}

impl Request {
    pub fn new(goal: String, project_id: ProjectId) -> Self {
        Self {
            id: Uuid::new_v4(),
            goal,
            params: serde_json::Value::Null,
            project_id,
            created_at: Utc::now(),
        }
    }
}

// === Todo / TaskBoard ===

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    Planned,
    Ready,
    InProgress,
    Blocked,
    Done,
    Archived,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Planned => write!(f, "planned"),
            TaskState::Ready => write!(f, "ready"),
            TaskState::InProgress => write!(f, "in-progress"),
            TaskState::Blocked => write!(f, "blocked"),
            TaskState::Done => write!(f, "done"),
            TaskState::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: Uuid,
    pub title: String,
    pub desc: String,
    pub state: TaskState,
    pub priority: i32,
    pub deps: Vec<Uuid>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub last_touch: DateTime<Utc>,
    pub staleness: f32,
}

impl TodoItem {
    pub fn new(title: String, desc: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            desc,
            state: TaskState::Planned,
            priority: 0,
            deps: Vec::new(),
            tags: Vec::new(),
            created_at: now,
            due_at: None,
            last_touch: now,
            staleness: 0.0,
        }
    }

    pub fn update_staleness(&mut self) {
        let now = Utc::now();
        let days_since_touch = (now - self.last_touch).num_days() as f32;
        
        let due_factor = if let Some(due) = self.due_at {
            if due < now {
                2.0 // Overdue
            } else {
                1.0 / ((due - now).num_days() as f32 + 1.0)
            }
        } else {
            1.0
        };

        self.staleness = days_since_touch * due_factor;
    }

    pub fn touch(&mut self) {
        self.last_touch = Utc::now();
        self.update_staleness();
    }
}

// === Memory Layers ===

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemLayer {
    Ephemeral,  // M0 - RAM
    Short,      // M1 - SQLite KV
    Medium,     // M2 - SQLite tables
    Long,       // M3 - Blobs/Archives
    Semantic,   // M4 - Vector index
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemRef {
    pub layer: MemLayer,
    pub key: String,
    pub metadata: HashMap<String, String>,
}

impl MemRef {
    pub fn new(layer: MemLayer, key: String) -> Self {
        Self {
            layer,
            key,
            metadata: HashMap::new(),
        }
    }

    pub fn with_meta(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

// === Events ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    TaskStarted { node_id: Uuid, task_name: String },
    TaskFinished { node_id: Uuid, success: bool, duration_ms: u64 },
    TodoUpdated { todo_id: Uuid, old_state: TaskState, new_state: TaskState },
    MemoryIngested { ref_: MemRef, content_size: usize },
    SearchPerformed { query: String, results_count: usize, latency_ms: u64 },
    PolicyViolation { rule: String, details: String },
    SchedulerJobStarted { job_name: String },
    SchedulerJobFinished { job_name: String, success: bool },
}

impl Event {
    pub fn timestamp(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

// === Config ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub paths: PathsConfig,
    pub nlu: NluConfig,
    pub policy: PolicyConfig,
    pub scheduler: SchedulerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub sqlite: String,
    pub tasks: String,
    pub vectors: String,
    pub blobs: String,
    pub embed_cache: String,
    pub logs_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NluConfig {
    pub embed_model: String,
    pub rerank_model: String,
    pub top_k_semantic: usize,
    pub top_k_rerank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub max_tokens: usize,
    pub max_bg3_docs: usize,
    pub allow_net: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub reindex_interval: String,
    pub stale_review_interval: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            paths: PathsConfig {
                sqlite: "sqlite.db".to_string(),
                tasks: "tasks.db".to_string(),
                vectors: "vectors/".to_string(),
                blobs: "blobs/".to_string(),
                embed_cache: "embed_cache.db".to_string(),
                logs_dir: "logs/".to_string(),
            },
            nlu: NluConfig {
                embed_model: "bge3".to_string(),
                rerank_model: "bg3".to_string(),
                top_k_semantic: 128,
                top_k_rerank: 32,
            },
            policy: PolicyConfig {
                max_tokens: 8192,
                max_bg3_docs: 64,
                allow_net: false,
            },
            scheduler: SchedulerConfig {
                reindex_interval: "1h".to_string(),
                stale_review_interval: "24h".to_string(),
            },
        }
    }
}

// === DocStore paths ===

#[derive(Debug, Clone)]
pub struct DocStore {
    pub root: PathBuf,
    pub config: Config,
}

impl DocStore {
    pub fn new(project_id: &ProjectId) -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        let root = home.join(".ourcli").join("projects").join(project_id.as_str());
        
        let config = if root.join("config.toml").exists() {
            let content = std::fs::read_to_string(root.join("config.toml"))?;
            toml::from_str(&content)?
        } else {
            Config::default()
        };

        Ok(Self { root, config })
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::create_dir_all(self.root.join(&self.config.paths.vectors))?;
        std::fs::create_dir_all(self.root.join(&self.config.paths.blobs))?;
        std::fs::create_dir_all(self.root.join(&self.config.paths.logs_dir))?;

        // Create config if not exists
        let config_path = self.root.join("config.toml");
        if !config_path.exists() {
            let config_str = toml::to_string_pretty(&self.config)?;
            std::fs::write(config_path, config_str)?;
        }

        Ok(())
    }

    pub fn sqlite_path(&self) -> PathBuf {
        self.root.join(&self.config.paths.sqlite)
    }

    pub fn tasks_path(&self) -> PathBuf {
        self.root.join(&self.config.paths.tasks)
    }

    pub fn vectors_dir(&self) -> PathBuf {
        self.root.join(&self.config.paths.vectors)
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.root.join(&self.config.paths.blobs)
    }
}
