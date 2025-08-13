#![allow(clippy::new_without_default)]
#![allow(unused_imports)]
#![allow(clippy::uninlined_format_args)]
use tempfile::TempDir;
use todo::{Priority, TaskState, TodoEvent, TodoService};
use uuid::Uuid;

#[tokio::test]
async fn test_service_creation() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Service должен быть создан успешно
    let (stats, _graph_stats) = service.get_stats().await.expect("get_stats failed");
    assert_eq!(stats.total, 0);
}

#[tokio::test]
async fn test_create_task() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    let task = service
        .create_task(
            "Test task".to_string(),
            "Test description".to_string(),
            Priority::Medium,
            vec!["test".to_string(), "rust".to_string()],
        )
        .await
        .expect("Failed to create task");

    assert_eq!(task.title, "Test task");
    assert_eq!(task.description, "Test description");
    assert_eq!(task.priority, Priority::Medium);
    assert_eq!(task.state, TaskState::Ready);
    assert_eq!(task.tags.len(), 2);
}

#[tokio::test]
async fn test_create_subtasks() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем родительскую задачу
    let parent = service
        .create_task(
            "Parent task".to_string(),
            "Parent description".to_string(),
            Priority::High,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Создаем подзадачи
    let subtasks = vec![
        ("Subtask 1".to_string(), "Description 1".to_string()),
        ("Subtask 2".to_string(), "Description 2".to_string()),
    ];

    let created_subtasks = service
        .create_subtasks(&parent.id, subtasks)
        .await
        .expect("Failed to execute async operation");

    assert_eq!(created_subtasks.len(), 2);
    assert!(created_subtasks
        .iter()
        .all(|t| t.parent_id == Some(parent.id)));
}

#[tokio::test]
async fn test_get_cached() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    let task = service
        .create_task(
            "Cached task".to_string(),
            "Test caching".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Первый доступ - из БД
    let cached1 = service
        .get_cached(&task.id)
        .await
        .expect("get_cached failed");
    assert!(cached1.is_some());
    assert_eq!(
        cached1
            .expect("Expected cached task to exist in test")
            .title,
        "Cached task"
    );

    // Второй доступ - из кэша
    let cached2 = service
        .get_cached(&task.id)
        .await
        .expect("get_cached failed");
    assert!(cached2.is_some());
    assert_eq!(
        cached2
            .expect("Expected cached task to exist in test")
            .title,
        "Cached task"
    );
}

#[tokio::test]
async fn test_update_state() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    let task = service
        .create_task(
            "Test task".to_string(),
            "Test description".to_string(),
            Priority::High,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Обновляем состояние
    service
        .update_state(&task.id, TaskState::InProgress)
        .await
        .expect("Failed to create task");

    let updated = service
        .get_cached(&task.id)
        .await
        .expect("get_cached failed")
        .expect("Task should exist after operation");
    assert_eq!(updated.state, TaskState::InProgress);
}

#[tokio::test]
async fn test_get_next_ready() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем задачи с разными приоритетами
    let _critical = service
        .create_task(
            "Critical task".to_string(),
            "".to_string(),
            Priority::Critical,
            vec![],
        )
        .await
        .expect("Failed to create task");

    let _high = service
        .create_task(
            "High task".to_string(),
            "".to_string(),
            Priority::High,
            vec![],
        )
        .await
        .expect("Failed to create task");

    let _medium = service
        .create_task(
            "Medium task".to_string(),
            "".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Получаем готовые задачи
    let ready = service
        .get_next_ready(2)
        .await
        .expect("get_next_ready failed");

    assert_eq!(ready.len(), 2);
    // Critical должен быть первым
    assert_eq!(ready[0].title, "Critical task");
    assert_eq!(ready[1].title, "High task");
}

#[tokio::test]
async fn test_add_dependency() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем задачи
    let task1 = service
        .create_task(
            "Task 1".to_string(),
            "First task".to_string(),
            Priority::High,
            vec![],
        )
        .await
        .expect("Failed to create task");

    let task2 = service
        .create_task(
            "Task 2".to_string(),
            "Second task".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Добавляем зависимость: task2 зависит от task1
    service
        .add_dependency(&task2.id, &task1.id)
        .await
        .expect("add_dependency failed");

    let updated_task2 = service
        .get_cached(&task2.id)
        .await
        .expect("get_cached failed")
        .expect("Task should exist after operation");
    assert!(updated_task2.depends_on.contains(&task1.id));
}

#[tokio::test]
async fn test_remove_dependency() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем задачи
    let task1 = service
        .create_task(
            "Task 1".to_string(),
            "First task".to_string(),
            Priority::High,
            vec![],
        )
        .await
        .expect("Failed to create task");

    let task2 = service
        .create_task(
            "Task 2".to_string(),
            "Second task".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Добавляем и удаляем зависимость
    service
        .add_dependency(&task2.id, &task1.id)
        .await
        .expect("add_dependency failed");
    service
        .remove_dependency(&task2.id, &task1.id)
        .await
        .expect("Failed to create task");

    let updated_task2 = service
        .get_cached(&task2.id)
        .await
        .expect("get_cached failed")
        .expect("Task should exist after operation");
    assert!(!updated_task2.depends_on.contains(&task1.id));
}

#[tokio::test]
async fn test_search() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем задачи
    service
        .create_task(
            "Search test task".to_string(),
            "Searchable description".to_string(),
            Priority::Medium,
            vec!["search".to_string()],
        )
        .await
        .expect("Failed to create task");

    service
        .create_task(
            "Another task".to_string(),
            "Different content".to_string(),
            Priority::Low,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Поиск по всем задачам
    let all_results = service.search("", 10).await.expect("search failed");
    assert_eq!(all_results.len(), 2);

    // Поиск по конкретному термину
    let search_results = service.search("search", 10).await.expect("search failed");
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].title, "Search test task");
}

#[tokio::test]
async fn test_optimize() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем несколько задач
    for i in 1..=5 {
        service
            .create_task(
                format!("Task {}", i),
                format!("Description {}", i),
                Priority::Medium,
                vec![],
            )
            .await
            .expect("Failed to create task in loop");
    }

    // Оптимизируем
    let result = service.optimize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_event_stream() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");
    let events = service.subscribe();

    // Создаем задачу
    let task = service
        .create_task(
            "Event test".to_string(),
            "Test events".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Проверяем событие создания
    if let Some(event) = events.next().await {
        match event {
            TodoEvent::TaskCreated { task_id, title, .. } => {
                assert_eq!(task_id, task.id);
                assert_eq!(title, "Event test");
            }
            _ => panic!("Unexpected event type"),
        }
    } else {
        panic!("No event received");
    }
}

#[tokio::test]
async fn test_task_statistics() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    // Создаем задачи в разных состояниях
    let task1 = service
        .create_task("Task 1".to_string(), "".to_string(), Priority::High, vec![])
        .await
        .expect("Failed to create task");

    let task2 = service
        .create_task(
            "Task 2".to_string(),
            "".to_string(),
            Priority::Medium,
            vec![],
        )
        .await
        .expect("Failed to create task");

    // Одну в прогрессе
    service
        .update_state(&task1.id, TaskState::InProgress)
        .await
        .expect("Failed to create task");

    // Одну завершаем
    service
        .update_state(&task2.id, TaskState::Done)
        .await
        .expect("Failed to create task");

    let (stats, _graph_stats) = service.get_stats().await.expect("get_stats failed");

    assert_eq!(stats.total, 2);
    assert_eq!(stats.in_progress, 1);
    assert_eq!(stats.done, 1);
}

#[tokio::test]
async fn test_task_not_found() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    let service = TodoService::new(&db_path, 4, 100)
        .await
        .expect("failed to create TodoService");

    let non_existent_id = Uuid::new_v4();

    // Попытка получить несуществующую задачу
    let result = service
        .get_cached(&non_existent_id)
        .await
        .expect("get_cached failed");
    assert!(result.is_none());

    // Попытка обновить несуществующую задачу должна вернуть ошибку
    let update_result = service
        .update_state(&non_existent_id, TaskState::Done)
        .await;
    assert!(update_result.is_err());
}
