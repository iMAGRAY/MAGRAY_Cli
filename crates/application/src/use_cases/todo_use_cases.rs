use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use todo::{graph::GraphStats, Priority, TaskState, TaskStats, TodoEvent, TodoItem, TodoService};
use uuid::Uuid;

/// Request для создания новой задачи
#[derive(Debug, Clone)]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub parent_id: Option<Uuid>,
    pub tool_hint: Option<String>,
    pub tool_params: Option<HashMap<String, String>>,
}

/// Response для создания задачи
#[derive(Debug, Clone)]
pub struct CreateTodoResponse {
    pub task: TodoItem,
    pub created: bool,
}

/// Request для получения списка задач
#[derive(Debug, Clone)]
pub struct ListTodosRequest {
    pub limit: usize,
    pub state_filter: Option<TaskState>,
    pub priority_filter: Option<Priority>,
    pub tag_filter: Option<String>,
}

/// Response для списка задач
#[derive(Debug, Clone)]
pub struct ListTodosResponse {
    pub tasks: Vec<TodoItem>,
    pub total_count: usize,
    pub filtered: bool,
}

/// Request для обновления состояния задачи
#[derive(Debug, Clone)]
pub struct UpdateTaskStateRequest {
    pub task_id: Uuid,
    pub new_state: TaskState,
    pub reason: Option<String>,
}

/// Response для обновления состояния
#[derive(Debug, Clone)]
pub struct UpdateTaskStateResponse {
    pub updated: bool,
    pub old_state: TaskState,
    pub new_state: TaskState,
    pub event: TodoEvent,
}

/// Request для добавления зависимостей
#[derive(Debug, Clone)]
pub struct AddDependencyRequest {
    pub task_id: Uuid,
    pub depends_on: Uuid,
}

/// Response для зависимостей
#[derive(Debug, Clone)]
pub struct AddDependencyResponse {
    pub added: bool,
    pub updated_tasks: Vec<Uuid>,
}

/// Request для получения статистики
#[derive(Debug, Clone)]
pub struct GetStatsRequest {
    pub include_graph: bool,
}

/// Response для статистики
#[derive(Debug, Clone)]
pub struct GetStatsResponse {
    pub task_stats: TaskStats,
    pub graph_stats: Option<GraphStats>,
}

/// Use Case: Создание новой задачи
pub struct CreateTodoUseCase {
    todo_service: Arc<TodoService>,
}

impl CreateTodoUseCase {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self { todo_service }
    }

    pub async fn execute(&self, request: CreateTodoRequest) -> Result<CreateTodoResponse> {
        // Создаем задачу через todo сервис
        let task = self
            .todo_service
            .create_task(
                request.title,
                request.description,
                request.priority,
                request.tags,
            )
            .await?;

        // Если есть дополнительные поля, обновляем их
        let mut updated_task = task;
        if let Some(parent_id) = request.parent_id {
            // Обновляем parent_id если нужно
            updated_task.parent_id = Some(parent_id);
        }
        if let Some(tool_hint) = request.tool_hint {
            updated_task.tool_hint = Some(tool_hint);
        }
        if let Some(tool_params) = request.tool_params {
            updated_task.tool_params = Some(tool_params);
        }

        Ok(CreateTodoResponse {
            task: updated_task,
            created: true,
        })
    }
}

/// Use Case: Получение списка задач
pub struct ListTodosUseCase {
    todo_service: Arc<TodoService>,
}

impl ListTodosUseCase {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self { todo_service }
    }

    pub async fn execute(&self, request: ListTodosRequest) -> Result<ListTodosResponse> {
        let tasks = if let Some(state) = request.state_filter {
            self.todo_service.get_by_state(state, request.limit).await?
        } else {
            self.todo_service.get_next_ready(request.limit).await?
        };

        // Применяем дополнительную фильтрацию
        let mut filtered_tasks = tasks;
        let mut was_filtered = false;

        if let Some(priority) = request.priority_filter {
            filtered_tasks.retain(|task| task.priority == priority);
            was_filtered = true;
        }

        if let Some(tag_filter) = &request.tag_filter {
            filtered_tasks.retain(|task| {
                task.tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&tag_filter.to_lowercase()))
            });
            was_filtered = true;
        }

        let total_count = filtered_tasks.len();

        Ok(ListTodosResponse {
            tasks: filtered_tasks,
            total_count,
            filtered: was_filtered,
        })
    }
}

/// Use Case: Обновление состояния задачи
pub struct UpdateTaskStateUseCase {
    todo_service: Arc<TodoService>,
}

impl UpdateTaskStateUseCase {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self { todo_service }
    }

    pub async fn execute(
        &self,
        request: UpdateTaskStateRequest,
    ) -> Result<UpdateTaskStateResponse> {
        // Получаем текущее состояние задачи
        let current_task = self
            .todo_service
            .get_cached(&request.task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", request.task_id))?;

        let old_state = current_task.state;

        // Обновляем состояние
        self.todo_service
            .update_state(&request.task_id, request.new_state)
            .await?;

        // Создаем событие для системы
        let event = TodoEvent::StateChanged {
            task_id: request.task_id,
            old_state,
            new_state: request.new_state,
            timestamp: chrono::Utc::now(),
        };

        Ok(UpdateTaskStateResponse {
            updated: true,
            old_state,
            new_state: request.new_state,
            event,
        })
    }
}

/// Use Case: Добавление зависимостей
pub struct AddDependencyUseCase {
    todo_service: Arc<TodoService>,
}

impl AddDependencyUseCase {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self { todo_service }
    }

    pub async fn execute(&self, request: AddDependencyRequest) -> Result<AddDependencyResponse> {
        // Добавляем зависимость
        self.todo_service
            .add_dependency(&request.task_id, &request.depends_on)
            .await?;

        // Возвращаем результат
        Ok(AddDependencyResponse {
            added: true,
            updated_tasks: vec![request.task_id, request.depends_on],
        })
    }
}

/// Use Case: Получение статистики
pub struct GetStatsUseCase {
    todo_service: Arc<TodoService>,
}

impl GetStatsUseCase {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self { todo_service }
    }

    pub async fn execute(&self, request: GetStatsRequest) -> Result<GetStatsResponse> {
        let (task_stats, graph_stats) = self.todo_service.get_stats().await?;

        Ok(GetStatsResponse {
            task_stats,
            graph_stats: if request.include_graph {
                Some(graph_stats)
            } else {
                None
            },
        })
    }
}

/// Единый фасад для всех Todo Use Cases
pub struct TodoUseCases {
    pub create_todo: CreateTodoUseCase,
    pub list_todos: ListTodosUseCase,
    pub update_task_state: UpdateTaskStateUseCase,
    pub add_dependency: AddDependencyUseCase,
    pub get_stats: GetStatsUseCase,
}

impl TodoUseCases {
    pub fn new(todo_service: Arc<TodoService>) -> Self {
        Self {
            create_todo: CreateTodoUseCase::new(todo_service.clone()),
            list_todos: ListTodosUseCase::new(todo_service.clone()),
            update_task_state: UpdateTaskStateUseCase::new(todo_service.clone()),
            add_dependency: AddDependencyUseCase::new(todo_service.clone()),
            get_stats: GetStatsUseCase::new(todo_service),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use todo::create_default_service;

    #[tokio::test]
    async fn test_create_todo_use_case() {
        let temp_dir = TempDir::new().expect("Todo use case operation should succeed");
        let db_path = temp_dir.path().join("test.db");
        let service = create_default_service(&db_path)
            .await
            .expect("Todo use case operation should succeed");
        let use_case = CreateTodoUseCase::new(Arc::new(service));

        let request = CreateTodoRequest {
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            priority: Priority::High,
            tags: vec!["test".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        };

        let response = use_case
            .execute(request)
            .await
            .expect("Todo use case operation should succeed");

        assert!(response.created);
        assert_eq!(response.task.title, "Test Task");
        assert_eq!(response.task.priority, Priority::High);
        assert_eq!(response.task.state, TaskState::Ready);
    }

    #[tokio::test]
    async fn test_list_todos_use_case() {
        let temp_dir = TempDir::new().expect("Todo use case operation should succeed");
        let db_path = temp_dir.path().join("test.db");
        let service = create_default_service(&db_path)
            .await
            .expect("Todo use case operation should succeed");

        // Создаем несколько задач
        let _ = service
            .create_task(
                "Task 1".to_string(),
                "Description 1".to_string(),
                Priority::High,
                vec!["urgent".to_string()],
            )
            .await
            .expect("Todo use case operation should succeed");

        let _ = service
            .create_task(
                "Task 2".to_string(),
                "Description 2".to_string(),
                Priority::Low,
                vec!["normal".to_string()],
            )
            .await
            .expect("Todo use case operation should succeed");

        let use_case = ListTodosUseCase::new(Arc::new(service));

        let request = ListTodosRequest {
            limit: 10,
            state_filter: None,
            priority_filter: Some(Priority::High),
            tag_filter: None,
        };

        let response = use_case
            .execute(request)
            .await
            .expect("Todo use case operation should succeed");

        assert_eq!(response.tasks.len(), 1);
        assert!(response.filtered);
        assert_eq!(response.tasks[0].title, "Task 1");
        assert_eq!(response.tasks[0].priority, Priority::High);
    }

    #[tokio::test]
    async fn test_update_task_state_use_case() {
        let temp_dir = TempDir::new().expect("Todo use case operation should succeed");
        let db_path = temp_dir.path().join("test.db");
        let service = create_default_service(&db_path)
            .await
            .expect("Todo use case operation should succeed");

        // Создаем задачу
        let task = service
            .create_task(
                "Test Task".to_string(),
                "Test Description".to_string(),
                Priority::Medium,
                vec![],
            )
            .await
            .expect("Todo use case operation should succeed");

        let use_case = UpdateTaskStateUseCase::new(Arc::new(service));

        let request = UpdateTaskStateRequest {
            task_id: task.id,
            new_state: TaskState::Done,
            reason: Some("Completed successfully".to_string()),
        };

        let response = use_case
            .execute(request)
            .await
            .expect("Todo use case operation should succeed");

        assert!(response.updated);
        assert_eq!(response.old_state, TaskState::Ready);
        assert_eq!(response.new_state, TaskState::Done);
    }

    #[tokio::test]
    async fn test_todo_use_cases_facade() {
        let temp_dir = TempDir::new().expect("Todo use case operation should succeed");
        let db_path = temp_dir.path().join("test.db");
        let service = create_default_service(&db_path)
            .await
            .expect("Todo use case operation should succeed");

        let use_cases = TodoUseCases::new(Arc::new(service));

        // Создаем задачу
        let create_request = CreateTodoRequest {
            title: "Integration Test Task".to_string(),
            description: "Testing facade".to_string(),
            priority: Priority::Critical,
            tags: vec!["integration".to_string(), "test".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        };

        let create_response = use_cases
            .create_todo
            .execute(create_request)
            .await
            .expect("Todo use case operation should succeed");
        let task_id = create_response.task.id;

        // Получаем статистику
        let stats_request = GetStatsRequest {
            include_graph: true,
        };
        let stats_response = use_cases
            .get_stats
            .execute(stats_request)
            .await
            .expect("Todo use case operation should succeed");

        assert!(stats_response.task_stats.total >= 1);
        assert!(stats_response.graph_stats.is_some());

        // Обновляем состояние
        let update_request = UpdateTaskStateRequest {
            task_id,
            new_state: TaskState::InProgress,
            reason: Some("Starting work".to_string()),
        };

        let update_response = use_cases
            .update_task_state
            .execute(update_request)
            .await
            .expect("Todo use case operation should succeed");
        assert!(update_response.updated);
        assert_eq!(update_response.new_state, TaskState::InProgress);
    }
}
