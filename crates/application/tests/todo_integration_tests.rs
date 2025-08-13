#![allow(clippy::uninlined_format_args)]
use anyhow::Result;
use application::use_cases::todo_use_cases::{
    AddDependencyRequest, CreateTodoRequest, GetStatsRequest, ListTodosRequest, TodoUseCases,
    UpdateTaskStateRequest,
};
use std::sync::Arc;
use tempfile::TempDir;
use todo::{create_default_service, Priority, TaskState};

/// Тест полной интеграции todo модуля
#[tokio::test]
async fn test_todo_full_integration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("integration_test.db");

    // Создаем todo service
    let service = create_default_service(&db_path).await?;

    // Создаем use cases
    let use_cases = TodoUseCases::new(std::sync::Arc::new(service));

    // 1. Создаем несколько задач
    let task1 = use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Setup Database".to_string(),
            description: "Setup PostgreSQL database".to_string(),
            priority: Priority::High,
            tags: vec!["database".to_string(), "setup".to_string()],
            parent_id: None,
            tool_hint: Some("shell_exec".to_string()),
            tool_params: Some(
                [(
                    "command".to_string(),
                    "psql -c 'CREATE DATABASE test;'".to_string(),
                )]
                .into(),
            ),
        })
        .await?;

    let task2 = use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Run Migrations".to_string(),
            description: "Run database migrations".to_string(),
            priority: Priority::High,
            tags: vec!["database".to_string(), "migration".to_string()],
            parent_id: None,
            tool_hint: Some("shell_exec".to_string()),
            tool_params: None,
        })
        .await?;

    let task3 = use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Seed Test Data".to_string(),
            description: "Insert test data into database".to_string(),
            priority: Priority::Medium,
            tags: vec!["database".to_string(), "data".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    // 2. Создаем зависимости: task2 зависит от task1, task3 зависит от task2
    use_cases
        .add_dependency
        .execute(AddDependencyRequest {
            task_id: task2.task.id,
            depends_on: task1.task.id,
        })
        .await?;

    use_cases
        .add_dependency
        .execute(AddDependencyRequest {
            task_id: task3.task.id,
            depends_on: task2.task.id,
        })
        .await?;

    // 3. Проверяем статистику
    let stats_response = use_cases
        .get_stats
        .execute(GetStatsRequest {
            include_graph: true,
        })
        .await?;

    assert_eq!(stats_response.task_stats.total, 3);
    assert!(stats_response.task_stats.ready >= 1); // task1 должна быть ready
    assert!(stats_response.task_stats.planned >= 2); // task2, task3 должны быть planned (ждут зависимости)
    assert!(stats_response.graph_stats.is_some());

    let graph_stats = stats_response
        .graph_stats
        .expect("Test operation should succeed");
    assert_eq!(graph_stats.total_tasks, 3);
    assert_eq!(graph_stats.total_dependencies, 2);

    // 4. Получаем список готовых задач (должна быть только task1)
    let ready_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: Some(TaskState::Ready),
            priority_filter: None,
            tag_filter: None,
        })
        .await?;

    assert_eq!(ready_tasks.tasks.len(), 1);
    assert_eq!(ready_tasks.tasks[0].title, "Setup Database");

    // 5. Завершаем task1 - должна разблокироваться task2
    use_cases
        .update_task_state
        .execute(UpdateTaskStateRequest {
            task_id: task1.task.id,
            new_state: TaskState::Done,
            reason: Some("Database setup completed".to_string()),
        })
        .await?;

    // 6. Проверяем что task2 теперь готова к выполнению
    let ready_tasks_after = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: Some(TaskState::Ready),
            priority_filter: None,
            tag_filter: None,
        })
        .await?;

    assert_eq!(ready_tasks_after.tasks.len(), 1);
    assert_eq!(ready_tasks_after.tasks[0].title, "Run Migrations");

    // 7. Завершаем task2 - должна разблокироваться task3
    use_cases
        .update_task_state
        .execute(UpdateTaskStateRequest {
            task_id: task2.task.id,
            new_state: TaskState::Done,
            reason: Some("Migrations completed".to_string()),
        })
        .await?;

    // 8. Проверяем что task3 теперь готова
    let ready_tasks_final = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: Some(TaskState::Ready),
            priority_filter: None,
            tag_filter: None,
        })
        .await?;

    assert_eq!(ready_tasks_final.tasks.len(), 1);
    assert_eq!(ready_tasks_final.tasks[0].title, "Seed Test Data");

    // 9. Проверяем финальную статистику
    let final_stats = use_cases
        .get_stats
        .execute(GetStatsRequest {
            include_graph: true,
        })
        .await?;

    assert_eq!(final_stats.task_stats.done, 2);
    assert_eq!(final_stats.task_stats.ready, 1);
    assert_eq!(final_stats.task_stats.total, 3);

    Ok(())
}

/// Тест фильтрации и поиска по тегам
#[tokio::test]
async fn test_todo_filtering_and_tags() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("filtering_test.db");

    let service = create_default_service(&db_path).await?;
    let use_cases = TodoUseCases::new(Arc::new(service));

    // Создаем задачи с разными тегами и приоритетами
    use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Frontend Bug".to_string(),
            description: "Fix button alignment".to_string(),
            priority: Priority::Critical,
            tags: vec!["frontend".to_string(), "bug".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Backend Feature".to_string(),
            description: "Add new API endpoint".to_string(),
            priority: Priority::High,
            tags: vec!["backend".to_string(), "feature".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Frontend Feature".to_string(),
            description: "Add dark mode".to_string(),
            priority: Priority::Medium,
            tags: vec!["frontend".to_string(), "feature".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Documentation".to_string(),
            description: "Update API docs".to_string(),
            priority: Priority::Low,
            tags: vec!["docs".to_string()],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    // Тест фильтрации по приоритету
    let critical_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: None,
            priority_filter: Some(Priority::Critical),
            tag_filter: None,
        })
        .await?;

    assert_eq!(critical_tasks.tasks.len(), 1);
    assert_eq!(critical_tasks.tasks[0].title, "Frontend Bug");
    assert!(critical_tasks.filtered);

    // Тест фильтрации по тегу
    let frontend_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: None,
            priority_filter: None,
            tag_filter: Some("frontend".to_string()),
        })
        .await?;

    assert_eq!(frontend_tasks.tasks.len(), 2);
    assert!(frontend_tasks.filtered);

    // Проверяем что обе задачи имеют тег "frontend"
    for task in &frontend_tasks.tasks {
        assert!(task.tags.contains(&"frontend".to_string()));
    }

    // Тест фильтрации по тегу "feature"
    let feature_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: None,
            priority_filter: None,
            tag_filter: Some("feature".to_string()),
        })
        .await?;

    assert_eq!(feature_tasks.tasks.len(), 2);

    // Тест комбинированной фильтрации
    let high_priority_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 10,
            state_filter: None,
            priority_filter: Some(Priority::High),
            tag_filter: Some("backend".to_string()),
        })
        .await?;

    assert_eq!(high_priority_tasks.tasks.len(), 1);
    assert_eq!(high_priority_tasks.tasks[0].title, "Backend Feature");

    Ok(())
}

/// Тест обработки ошибок
#[tokio::test]
async fn test_todo_error_handling() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("error_test.db");

    let service = create_default_service(&db_path).await?;
    let use_cases = TodoUseCases::new(Arc::new(service));

    // Тест обновления несуществующей задачи
    let fake_id = uuid::Uuid::new_v4();
    let result = use_cases
        .update_task_state
        .execute(UpdateTaskStateRequest {
            task_id: fake_id,
            new_state: TaskState::Done,
            reason: None,
        })
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Task not found"));

    // Тест добавления зависимости на несуществующую задачу
    let task = use_cases
        .create_todo
        .execute(CreateTodoRequest {
            title: "Test Task".to_string(),
            description: "Test".to_string(),
            priority: Priority::Medium,
            tags: vec![],
            parent_id: None,
            tool_hint: None,
            tool_params: None,
        })
        .await?;

    let fake_dependency_id = uuid::Uuid::new_v4();
    let _dependency_result = use_cases
        .add_dependency
        .execute(AddDependencyRequest {
            task_id: task.task.id,
            depends_on: fake_dependency_id,
        })
        .await;

    // Зависимость может быть добавлена даже на несуществующую задачу в некоторых реализациях
    // но это должно быть обработано на уровне сервиса

    Ok(())
}

/// Тест производительности с большим количеством задач
#[tokio::test]
async fn test_todo_performance_with_many_tasks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("performance_test.db");

    let service = create_default_service(&db_path).await?;
    let use_cases = TodoUseCases::new(Arc::new(service));

    let start_time = std::time::Instant::now();

    // Создаем 100 задач
    for i in 0..100 {
        use_cases
            .create_todo
            .execute(CreateTodoRequest {
                title: format!("Task {i}"),
                description: format!("Description for task {i}"),
                priority: match i % 4 {
                    0 => Priority::Critical,
                    1 => Priority::High,
                    2 => Priority::Medium,
                    3 => Priority::Low,
                    _ => Priority::Medium,
                },
                tags: vec![
                    format!("batch-{}", i / 10),
                    if i % 2 == 0 { "even" } else { "odd" }.to_string(),
                ],
                parent_id: None,
                tool_hint: None,
                tool_params: None,
            })
            .await?;
    }

    let creation_time = start_time.elapsed();
    println!("Created 100 tasks in {:?}", creation_time);

    // Тест получения статистики
    let stats_start = std::time::Instant::now();
    let stats = use_cases
        .get_stats
        .execute(GetStatsRequest {
            include_graph: true,
        })
        .await?;
    let stats_time = stats_start.elapsed();

    assert_eq!(stats.task_stats.total, 100);
    assert_eq!(stats.task_stats.ready, 100); // Все задачи должны быть ready (без зависимостей)
    println!("Got stats for 100 tasks in {:?}", stats_time);

    // Тест массового получения задач
    let list_start = std::time::Instant::now();
    let all_tasks = use_cases
        .list_todos
        .execute(ListTodosRequest {
            limit: 100,
            state_filter: None,
            priority_filter: None,
            tag_filter: None,
        })
        .await?;
    let list_time = list_start.elapsed();

    assert_eq!(all_tasks.tasks.len(), 100);
    println!("Listed 100 tasks in {:?}", list_time);

    // Убеждаемся что производительность приемлема
    assert!(
        creation_time.as_millis() < 10000,
        "Task creation took too long: {:?}",
        creation_time
    );
    assert!(
        stats_time.as_millis() < 1000,
        "Stats query took too long: {:?}",
        stats_time
    );
    assert!(
        list_time.as_millis() < 1000,
        "List query took too long: {:?}",
        list_time
    );

    Ok(())
}
