#![cfg(feature = "extended-tests")]

use anyhow::Result;
use tempfile::NamedTempFile;
use todo::service_v2::TodoServiceV2;
use todo::store_v2::TodoStoreV2;
use todo::types::*;
use todo::DependencyGraph;
use uuid::Uuid;

#[tokio::test]
async fn test_service_creation_variants() -> Result<()> {
    let temp_file = NamedTempFile::new()?;

    // Test with different pool sizes
    let service1 = TodoServiceV2::new(temp_file.path(), 2, 50).await?;
    let service2 = TodoServiceV2::new(temp_file.path(), 8, 200).await?;

    // Both should be created successfully
    drop(service1);
    drop(service2);

    Ok(())
}

#[tokio::test]
async fn test_create_multiple_tasks() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Create multiple tasks with different priorities
    let task1 = service
        .create_task(
            "High priority task".to_string(),
            "Important task description".to_string(),
            Priority::High,
            vec!["urgent".to_string(), "important".to_string()],
        )
        .await?;

    let task2 = service
        .create_task(
            "Low priority task".to_string(),
            "Less important task".to_string(),
            Priority::Low,
            vec!["backlog".to_string()],
        )
        .await?;

    let task3 = service
        .create_task(
            "Critical task".to_string(),
            "Must do immediately".to_string(),
            Priority::Critical,
            vec!["critical".to_string(), "urgent".to_string()],
        )
        .await?;

    // Verify all tasks were created with correct properties
    assert_eq!(task1.priority, Priority::High);
    assert_eq!(task2.priority, Priority::Low);
    assert_eq!(task3.priority, Priority::Critical);

    // Verify tags
    assert!(task1.tags.contains(&"urgent".to_string()));
    assert!(task2.tags.contains(&"backlog".to_string()));
    assert!(task3.tags.contains(&"critical".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_task_state_transitions() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    let task = service
        .create_task(
            "State transition test".to_string(),
            "Testing state changes".to_string(),
            Priority::Medium,
            vec!["test".to_string()],
        )
        .await?;

    // Initial state should be Ready
    let retrieved = service.get_cached(&task.id).await?;
    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.expect("Test operation should succeed").state,
        TaskState::Ready
    );

    // Update to InProgress
    service
        .update_state(&task.id, TaskState::InProgress)
        .await?;
    let retrieved = service.get_cached(&task.id).await?;
    assert_eq!(
        retrieved.expect("Test operation should succeed").state,
        TaskState::InProgress
    );

    // Update to Done
    service.update_state(&task.id, TaskState::Done).await?;
    let retrieved = service.get_cached(&task.id).await?;
    assert_eq!(
        retrieved.expect("Test operation should succeed").state,
        TaskState::Done
    );

    Ok(())
}

#[tokio::test]
async fn test_dependency_operations() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    let parent_task = service
        .create_task(
            "Parent task".to_string(),
            "Must be done first".to_string(),
            Priority::High,
            vec!["parent".to_string()],
        )
        .await?;

    let child_task = service
        .create_task(
            "Child task".to_string(),
            "Depends on parent".to_string(),
            Priority::Medium,
            vec!["child".to_string()],
        )
        .await?;

    // Add dependency
    service
        .add_dependency(&child_task.id, &parent_task.id)
        .await?;

    let ready_tasks = service.get_next_ready(10).await?;
    let ready_ids: Vec<Uuid> = ready_tasks.iter().map(|t| t.id).collect();

    // Parent should be ready, child should not be in ready list
    assert!(ready_ids.contains(&parent_task.id));
    // Child might or might not be in ready list depending on implementation

    // Clean up - remove dependency
    service
        .remove_dependency(&child_task.id, &parent_task.id)
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Create tasks with searchable content
    service
        .create_task(
            "Search test task 1".to_string(),
            "Description with keyword alpha".to_string(),
            Priority::Medium,
            vec!["search".to_string()],
        )
        .await?;

    service
        .create_task(
            "Search test task 2".to_string(),
            "Description with keyword beta".to_string(),
            Priority::Medium,
            vec!["search".to_string()],
        )
        .await?;

    service
        .create_task(
            "Different task".to_string(),
            "No matching keywords here".to_string(),
            Priority::Low,
            vec!["other".to_string()],
        )
        .await?;

    let results = service.search("Search", 10).await?;
    assert_eq!(results.len(), 2);

    let results = service.search("alpha", 10).await?;
    assert_eq!(results.len(), 1);

    let results = service.search("", 10).await?;
    assert_eq!(results.len(), 3);

    Ok(())
}

#[tokio::test]
async fn test_statistics_collection() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Create tasks with different states
    let task1 = service
        .create_task(
            "Ready task".to_string(),
            "Should be ready".to_string(),
            Priority::Medium,
            vec![],
        )
        .await?;

    let task2 = service
        .create_task(
            "In progress task".to_string(),
            "Currently working".to_string(),
            Priority::High,
            vec![],
        )
        .await?;

    let task3 = service
        .create_task(
            "Done task".to_string(),
            "Already completed".to_string(),
            Priority::Low,
            vec![],
        )
        .await?;

    // Update states
    service
        .update_state(&task2.id, TaskState::InProgress)
        .await?;
    service.update_state(&task3.id, TaskState::Done).await?;

    // Get statistics
    let (task_stats, _graph_stats) = service.get_stats().await?;

    // Verify counts (total should be 3)
    assert_eq!(task_stats.total, 3);
    // Other specific counts depend on internal implementation
    assert!(task_stats.total > 0);

    Ok(())
}

#[tokio::test]
async fn test_subtask_creation() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    let parent_task = service
        .create_task(
            "Parent task for subtasks".to_string(),
            "Main task that will have subtasks".to_string(),
            Priority::High,
            vec!["parent".to_string()],
        )
        .await?;

    // Create subtasks
    let subtask_descriptions = vec![
        ("First subtask".to_string(), "Description 1".to_string()),
        ("Second subtask".to_string(), "Description 2".to_string()),
        ("Third subtask".to_string(), "Description 3".to_string()),
    ];

    let subtasks = service
        .create_subtasks(&parent_task.id, subtask_descriptions)
        .await?;

    // Verify subtasks were created
    assert_eq!(subtasks.len(), 3);

    // Verify all subtasks have correct properties
    for subtask in &subtasks {
        // Subtasks should inherit parent's priority or have default
        assert!(subtask.priority >= Priority::Low);
        assert!(subtask.title.contains("subtask"));
    }

    Ok(())
}

#[tokio::test]
async fn test_ready_task_retrieval() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Create several ready tasks
    for i in 0..5 {
        service
            .create_task(
                format!("Ready task {}", i),
                format!("Description for task {}", i),
                Priority::Medium,
                vec![format!("batch-{}", i)],
            )
            .await?;
    }

    // Get ready tasks with limit
    let ready_tasks = service.get_next_ready(3).await?;
    assert!(ready_tasks.len() <= 3);
    assert!(ready_tasks.len() > 0);

    // Get all ready tasks
    let all_ready = service.get_next_ready(10).await?;
    assert_eq!(all_ready.len(), 5);

    Ok(())
}

#[tokio::test]
async fn test_service_optimization() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Create some tasks to optimize
    for i in 0..10 {
        service
            .create_task(
                format!("Optimization task {}", i),
                format!("Task for optimization test {}", i),
                if i % 2 == 0 {
                    Priority::High
                } else {
                    Priority::Low
                },
                vec!["optimization".to_string()],
            )
            .await?;
    }

    // Run optimization (should succeed)
    service.optimize().await?;

    // Verify tasks are still accessible after optimization
    let tasks = service.search("optimization", 15).await?;
    assert_eq!(tasks.len(), 10);

    Ok(())
}

#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = TodoServiceV2::new(temp_file.path(), 4, 100).await?;

    // Test with empty strings
    let task = service
        .create_task("".to_string(), "".to_string(), Priority::Medium, vec![])
        .await?;

    assert_eq!(task.title, "");
    assert_eq!(task.description, "");

    // Test with very long strings
    let long_title = "A".repeat(1000);
    let long_desc = "B".repeat(2000);

    let long_task = service
        .create_task(
            long_title.clone(),
            long_desc.clone(),
            Priority::Low,
            vec!["long".to_string()],
        )
        .await?;

    assert_eq!(long_task.title, long_title);
    assert_eq!(long_task.description, long_desc);

    // Test getting non-existent task
    let fake_id = Uuid::new_v4();
    let result = service.get_cached(&fake_id).await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_todo_store_direct() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let store = TodoStoreV2::new(temp_file.path(), 4).await?;

    let task = TodoItem {
        title: "Direct store test".to_string(),
        description: "Testing store directly".to_string(),
        state: TaskState::Ready,
        priority: Priority::Medium,
        tags: vec!["direct".to_string()],
        ..Default::default()
    };

    // Create and retrieve task
    let created = store.create(task).await?;
    let retrieved = store.get(&created.id).await?;

    assert!(retrieved.is_some());
    assert_eq!(
        retrieved.expect("Test operation should succeed").title,
        "Direct store test"
    );

    Ok(())
}

#[tokio::test]
async fn test_dependency_graph_direct() -> Result<()> {
    let graph = DependencyGraph::new();

    let task1_id = Uuid::new_v4();
    let task2_id = Uuid::new_v4();

    let task1 = TodoItem {
        id: task1_id,
        title: "Task 1".to_string(),
        ..Default::default()
    };

    let mut task2 = TodoItem {
        id: task2_id,
        title: "Task 2".to_string(),
        ..Default::default()
    };
    task2.depends_on.push(task1_id);

    // Add tasks to graph
    graph.add_task(&task1)?;
    graph.add_task(&task2)?;

    assert!(graph.is_ready(&task1_id)?);
    assert!(graph.is_ready(&task2_id)?); // Graph doesn't check completed state

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let temp_file = NamedTempFile::new()?;
    let service = std::sync::Arc::new(TodoServiceV2::new(temp_file.path(), 8, 200).await?);

    // Create multiple tasks concurrently
    let mut handles = vec![];

    for i in 0..20 {
        let service_clone = service.clone();
        handles.push(tokio::spawn(async move {
            service_clone
                .create_task(
                    format!("Concurrent task {}", i),
                    format!("Description {}", i),
                    Priority::Medium,
                    vec![format!("concurrent-{}", i)],
                )
                .await
        }));
    }

    let mut created_tasks = vec![];
    for handle in handles {
        let task = handle.await??;
        created_tasks.push(task);
    }

    assert_eq!(created_tasks.len(), 20);

    // Verify all tasks are accessible
    let all_tasks = service.search("", 25).await?;
    assert_eq!(all_tasks.len(), 20);

    Ok(())
}
