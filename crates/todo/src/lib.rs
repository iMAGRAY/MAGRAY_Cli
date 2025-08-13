#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use std::path::Path;

pub mod graph;
pub mod service_v2;
pub mod store;
pub mod store_v2;
pub mod types;

// Экспортируем v2 как основную версию
pub use service_v2::{TodoEventStream, TodoServiceV2 as TodoService};
pub use types::*;

// Экспортируем для обратной совместимости
pub use graph::DependencyGraphV2 as DependencyGraph;
pub use store::TodoStore;

/// Создать оптимизированный TodoService
///
/// # Параметры
/// - `db_path` - путь к файлу базы данных SQLite
/// - `pool_size` - размер пула соединений (рекомендуется 4-8)
/// - `cache_size` - размер LRU кэша для задач (рекомендуется 100-1000)
///
/// # Пример
/// ```no_run
/// use todo::create_service;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let service = create_service("tasks.db", 4, 100).await?;
///     
///     // Создаем задачу
///     let task = service.create_task(
///         "Implement feature X".to_string(),
///         "Description of the feature".to_string(),
///         todo::Priority::High,
///         vec!["feature".to_string()],
///     ).await?;
///     
///     // Получаем следующие готовые задачи
///     let ready_tasks = service.get_next_ready(5).await?;
///     
///     Ok(())
/// }
/// ```
pub async fn create_service<P: AsRef<Path>>(
    db_path: P,
    pool_size: u32,
    cache_size: usize,
) -> Result<TodoService> {
    TodoService::new(db_path, pool_size, cache_size).await
}

/// Создать TodoService с настройками по умолчанию
pub async fn create_default_service<P: AsRef<Path>>(db_path: P) -> Result<TodoService> {
    create_service(db_path, 4, 100).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_service_creation() {
        let temp_dir = TempDir::new().expect("Operation failed - converted from unwrap()");
        let db_path = temp_dir.path().join("test.db");

        let service = create_default_service(&db_path)
            .await
            .expect("Async operation should succeed");

        // Создаем задачу
        let task = service
            .create_task(
                "Test task".to_string(),
                "Test description".to_string(),
                Priority::Medium,
                vec!["test".to_string()],
            )
            .await
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(task.title, "Test task");
        assert_eq!(task.state, TaskState::Ready);
    }

    #[tokio::test]
    async fn test_event_system() {
        let temp_dir = TempDir::new().expect("Operation failed - converted from unwrap()");
        let db_path = temp_dir.path().join("test.db");

        let service = create_service(&db_path, 2, 50)
            .await
            .expect("Async operation should succeed");
        let events = service.subscribe();

        // Создаем задачу
        let task = service
            .create_task(
                "Event test".to_string(),
                "Testing events".to_string(),
                Priority::High,
                vec![],
            )
            .await
            .expect("Operation failed - converted from unwrap()");

        // Должны получить событие о создании
        if let Some(event) = events.next().await {
            match event {
                TodoEvent::TaskCreated { task_id, .. } => {
                    assert_eq!(task_id, task.id);
                }
                _ => panic!("Unexpected event type"),
            }
        } else {
            panic!("No event received");
        }
    }
}
