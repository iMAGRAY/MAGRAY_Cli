#![cfg(feature = "extended-tests")]

use todo::{DependencyGraph, TaskState, TodoItem};
use uuid::Uuid;

fn create_test_task(title: &str) -> TodoItem {
    TodoItem {
        title: title.to_string(),
        ..TodoItem::default()
    }
}

#[test]
fn test_graph_creation() {
    let graph = DependencyGraph::new();
    // В V2 нет явного счетчика, проверим stats
    let stats = graph.stats();
    assert_eq!(stats.total_tasks, 0);
}

#[test]
fn test_add_task() {
    let graph = DependencyGraph::new();
    let task = create_test_task("Test task");
    graph
        .upsert_task(&task)
        .expect("Failed to upsert task in graph");
    let stats = graph.stats();
    assert_eq!(stats.total_tasks, 1);
}

#[test]
fn test_add_task_with_dependencies() {
    let graph = DependencyGraph::new();
    let task1 = create_test_task("Task 1");
    let mut task2 = create_test_task("Task 2");
    let id1 = task1.id;
    task2.depends_on.push(id1);

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");

    let stats = graph.stats();
    assert_eq!(stats.total_tasks, 2);
    assert_eq!(stats.total_dependencies, 1);
}

#[test]
fn test_circular_dependency_detection() {
    let graph = DependencyGraph::new();

    let task1 = create_test_task("Task 1");
    let mut task2 = create_test_task("Task 2");

    let id1 = task1.id;
    let id2 = task2.id;

    task2.depends_on.push(id1);

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");

    // Проверяем что обратная зависимость создаст путь 2->1 (цикл при 1<-2)
    assert!(graph
        .would_create_cycle(&id1, &id2)
        .expect("Graph test operation should succeed"));

    // Независимый id не должен создавать цикл
    let task3_id = Uuid::new_v4();
    assert!(!graph
        .would_create_cycle(&task3_id, &id1)
        .expect("Graph test operation should succeed"));
}

#[test]
fn test_update_dependencies() {
    let graph = DependencyGraph::new();

    let task1 = create_test_task("Task 1");
    let task2 = create_test_task("Task 2");
    let mut task3 = create_test_task("Task 3");

    let id1 = task1.id;
    let id2 = task2.id;

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task3)
        .expect("Graph test operation should succeed");

    // Обновляем зависимости task3 (в V2 upsert_task сам обновит рёбра)
    task3.depends_on = vec![id1, id2];
    graph
        .upsert_task(&task3)
        .expect("Graph test operation should succeed");

    let stats = graph.stats();
    assert_eq!(stats.total_tasks, 3);
    assert_eq!(stats.total_dependencies, 2);
}

#[test]
fn test_is_ready() {
    let graph = DependencyGraph::new();

    let task1 = create_test_task("Task 1");
    let task2 = create_test_task("Task 2");

    let id1 = task1.id;
    let id2 = task2.id;

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");

    // В V2 задача готова если все её зависимости выполнены (их нет)
    assert!(graph
        .is_ready(&id1)
        .expect("Graph test operation should succeed"));
    assert!(graph
        .is_ready(&id2)
        .expect("Graph test operation should succeed"));
}

#[test]
fn test_topological_sort() {
    let graph = DependencyGraph::new();

    let task1 = create_test_task("Task 1");
    let mut task2 = create_test_task("Task 2");
    let mut task3 = create_test_task("Task 3");

    let id1 = task1.id;
    let id2 = task2.id;
    let id3 = task3.id;

    // Создаем зависимости: 2 зависит от 1, 3 зависит от 2
    task2.depends_on.push(id1);
    task3.depends_on.push(id2);

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task3)
        .expect("Graph test operation should succeed");

    let sorted = graph
        .topological_sort()
        .expect("Graph test operation should succeed");

    assert_eq!(sorted.len(), 3);
    let index_of = |id: Uuid| {
        sorted
            .iter()
            .position(|&x| x == id)
            .expect("Graph test operation should succeed")
    };
    assert!(index_of(id1) < index_of(id2));
    assert!(index_of(id2) < index_of(id3));
}

#[test]
fn test_update_state() {
    let graph = DependencyGraph::new();

    let mut task = create_test_task("Test task");
    let task_id = task.id;

    graph
        .upsert_task(&task)
        .expect("Failed to upsert task in graph");

    // Обновляем состояние
    graph
        .update_state(&task_id, TaskState::InProgress)
        .expect("Graph test operation should succeed");

    // Готовность не должна быть true для InProgress
    assert!(!graph
        .is_ready(&task_id)
        .expect("Graph test operation should succeed"));
}

#[test]
fn test_would_create_cycle_complex() {
    let graph = DependencyGraph::new();

    let task1 = create_test_task("Task 1");
    let mut task2 = create_test_task("Task 2");
    let mut task3 = create_test_task("Task 3");

    let id1 = task1.id;
    let id2 = task2.id;
    let id3 = task3.id;

    // Цепочка: 1 -> 2 -> 3
    task2.depends_on.push(id1);
    task3.depends_on.push(id2);

    graph
        .upsert_task(&task1)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task2)
        .expect("Graph test operation should succeed");
    graph
        .upsert_task(&task3)
        .expect("Graph test operation should succeed");

    // Попытка создать цикл 1 <- 3 должна быть обнаружена
    assert!(graph
        .would_create_cycle(&id1, &id3)
        .expect("Graph test operation should succeed"));

    // Новая зависимость без цикла должна быть разрешена
    let task4_id = Uuid::new_v4();
    assert!(!graph
        .would_create_cycle(&task4_id, &id3)
        .expect("Graph test operation should succeed"));
}

#[test]
fn test_empty_graph_operations() {
    let graph = DependencyGraph::new();

    let task_id = Uuid::new_v4();

    // Операции с несуществующими задачами
    assert!(graph
        .is_ready(&task_id)
        .expect("Graph test operation should succeed"));
    assert!(!graph
        .would_create_cycle(&task_id, &Uuid::new_v4())
        .expect("Graph test operation should succeed"));

    let sorted = graph
        .topological_sort()
        .expect("Graph test operation should succeed");
    assert!(sorted.is_empty());
}
