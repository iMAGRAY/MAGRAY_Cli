use chrono::{DateTime, Utc};
#[cfg(not(feature = "minimal"))]
use memory::{Layer, Record};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Современная замена для deprecated MemRef
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReference {
    pub layer: Layer,
    pub record_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl MemoryReference {
    /// Создает MemoryReference из Record
    pub fn from_record(record: &Record) -> Self {
        Self {
            layer: record.layer,
            record_id: record.id,
            created_at: record.ts,
        }
    }
}

/// Состояние задачи в жизненном цикле
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    /// Создана, но не готова к выполнению (есть блокирующие зависимости)
    Planned,
    /// Все зависимости выполнены, готова к выполнению
    Ready,
    /// Выполняется в данный момент
    InProgress,
    /// Заблокирована зависимостями или внешними факторами
    Blocked,
    /// Успешно выполнена
    Done,
    /// Провалена
    Failed,
    /// Отменена пользователем
    Cancelled,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskState::Planned => write!(f, "planned"),
            TaskState::Ready => write!(f, "ready"),
            TaskState::InProgress => write!(f, "in_progress"),
            TaskState::Blocked => write!(f, "blocked"),
            TaskState::Done => write!(f, "done"),
            TaskState::Failed => write!(f, "failed"),
            TaskState::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for TaskState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(TaskState::Planned),
            "ready" => Ok(TaskState::Ready),
            "in_progress" => Ok(TaskState::InProgress),
            "blocked" => Ok(TaskState::Blocked),
            "done" => Ok(TaskState::Done),
            "failed" => Ok(TaskState::Failed),
            "cancelled" => Ok(TaskState::Cancelled),
            _ => Err(anyhow::anyhow!("Unknown task state: {}", s)),
        }
    }
}

/// Приоритет задачи
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

/// Основная структура задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub state: TaskState,
    pub priority: Priority,

    // Метаданные
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,

    // Связи
    pub parent_id: Option<Uuid>,
    pub depends_on: Vec<Uuid>,
    pub blocks: Vec<Uuid>,

    // Контекст из Memory
    pub context_refs: Vec<MemoryReference>,

    // Результаты выполнения
    pub artifacts: Vec<MemoryReference>,

    // AI метаданные
    pub auto_generated: bool,
    pub confidence: f32,
    pub reasoning: Option<String>,

    // Подсказки для выполнения
    pub tool_hint: Option<String>,
    pub tool_params: Option<HashMap<String, String>>,

    // Дополнительные метаданные
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for TodoItem {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: String::new(),
            description: String::new(),
            state: TaskState::Planned,
            priority: Priority::Medium,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: None,
            completed_at: None,
            due_date: None,
            parent_id: None,
            depends_on: Vec::new(),
            blocks: Vec::new(),
            context_refs: Vec::new(),
            artifacts: Vec::new(),
            auto_generated: false,
            confidence: 1.0,
            reasoning: None,
            tool_hint: None,
            tool_params: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Сложность задачи
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskComplexity {
    Simple,
    Complex,
}

/// События в системе задач
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TodoEvent {
    TaskCreated {
        task_id: Uuid,
        title: String,
        auto_generated: bool,
    },
    StateChanged {
        task_id: Uuid,
        old_state: TaskState,
        new_state: TaskState,
        timestamp: DateTime<Utc>,
    },
    DependencyAdded {
        task_id: Uuid,
        depends_on: Uuid,
    },
    DependencyRemoved {
        task_id: Uuid,
        depends_on: Uuid,
    },
    TaskCompleted {
        task_id: Uuid,
        duration: chrono::Duration,
        artifacts: Vec<MemoryReference>,
    },
    TaskFailed {
        task_id: Uuid,
        reason: String,
    },
}

/// Подготовленная к выполнению задача
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableTask {
    pub task_id: Uuid,
    pub tool: String,
    pub parameters: HashMap<String, String>,
    pub context: String,
}

/// Предложение инструмента для задачи
#[derive(Debug, Clone)]
pub struct ToolSuggestion {
    pub tool_name: String,
    pub confidence: f32,
    pub reason: String,
}

/// Выполнимость задачи
#[derive(Debug, Clone)]
pub enum TaskFeasibility {
    Feasible(ToolSuggestion),
    Uncertain(ToolSuggestion),
    NeedsHumanInput,
}

/// Результат обработки пайплайна задач
#[derive(Debug)]
pub struct TodoPipelineResult {
    pub created_tasks: Vec<TodoItem>,
    pub executable_tasks: Vec<ExecutableTask>,
    pub needs_clarification: bool,
}

/// Действие с задачей
#[derive(Debug)]
pub enum TodoAction {
    Created(TodoItem),
    Updated(TodoItem),
    Deleted(Uuid),
    StatusReport(String),
}

/// Запись о выполненной задаче для обучения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompletionRecord {
    pub description: String,
    pub tool_used: Option<String>,
    pub parameters: Option<HashMap<String, String>>,
    pub duration: Option<chrono::Duration>,
    pub success: bool,
}

/// Статистика по задачам
#[derive(Debug, Default, Clone)]
pub struct TaskStats {
    pub total: usize,
    pub planned: usize,
    pub ready: usize,
    pub in_progress: usize,
    pub blocked: usize,
    pub done: usize,
    pub failed: usize,
    pub cancelled: usize,
}

#[cfg(feature = "minimal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    ShortTerm,
    LongTerm,
}

#[cfg(feature = "minimal")]
#[derive(Debug, Clone)]
pub struct Record {
    pub id: uuid::Uuid,
    pub text: String,
    pub layer: Layer,
}
