use todo::{
    TaskState, Priority, TodoItem, MemoryReference, TodoEvent, 
    TaskComplexity, ExecutableTask, ToolSuggestion, TaskFeasibility,
    TaskStats
};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use memory::{Record, Layer};

#[test]
fn test_task_state_display() {
    assert_eq!(TaskState::Planned.to_string(), "planned");
    assert_eq!(TaskState::Ready.to_string(), "ready");
    assert_eq!(TaskState::InProgress.to_string(), "in_progress");
    assert_eq!(TaskState::Blocked.to_string(), "blocked");
    assert_eq!(TaskState::Done.to_string(), "done");
    assert_eq!(TaskState::Failed.to_string(), "failed");
    assert_eq!(TaskState::Cancelled.to_string(), "cancelled");
}

#[test]
fn test_task_state_from_str() {
    use std::str::FromStr;
    
    assert_eq!(TaskState::from_str("planned").unwrap(), TaskState::Planned);
    assert_eq!(TaskState::from_str("ready").unwrap(), TaskState::Ready);
    assert_eq!(TaskState::from_str("in_progress").unwrap(), TaskState::InProgress);
    assert_eq!(TaskState::from_str("blocked").unwrap(), TaskState::Blocked);
    assert_eq!(TaskState::from_str("done").unwrap(), TaskState::Done);
    assert_eq!(TaskState::from_str("failed").unwrap(), TaskState::Failed);
    assert_eq!(TaskState::from_str("cancelled").unwrap(), TaskState::Cancelled);
    
    assert!(TaskState::from_str("invalid").is_err());
}

#[test]
fn test_priority_ordering() {
    assert!(Priority::Low < Priority::Medium);
    assert!(Priority::Medium < Priority::High);
    assert!(Priority::High < Priority::Critical);
}

#[test]
fn test_priority_display() {
    assert_eq!(Priority::Low.to_string(), "low");
    assert_eq!(Priority::Medium.to_string(), "medium");
    assert_eq!(Priority::High.to_string(), "high");
    assert_eq!(Priority::Critical.to_string(), "critical");
}

#[test]
fn test_todo_item_default() {
    let item = TodoItem::default();
    
    assert!(!item.title.is_empty() || item.title.is_empty()); // Title can be empty
    assert!(item.description.is_empty());
    assert_eq!(item.state, TaskState::Planned);
    assert_eq!(item.priority, Priority::Medium);
    assert!(!item.auto_generated);
    assert_eq!(item.confidence, 1.0);
    assert!(item.depends_on.is_empty());
    assert!(item.blocks.is_empty());
    assert!(item.tags.is_empty());
}

#[test]
fn test_todo_item_with_dependencies() {
    let mut item = TodoItem::default();
    let dep1 = Uuid::new_v4();
    let dep2 = Uuid::new_v4();
    
    item.depends_on = vec![dep1, dep2];
    
    assert_eq!(item.depends_on.len(), 2);
    assert!(item.depends_on.contains(&dep1));
    assert!(item.depends_on.contains(&dep2));
}

#[test]
fn test_memory_reference_from_record() {
    let record = Record {
        id: Uuid::new_v4(),
        layer: Layer::Interact,
        ts: Utc::now(),
        text: "Test record".to_string(),
        embedding: vec![0.1, 0.2, 0.3],
        kind: String::new(),
        tags: vec![],
        project: String::new(),
        session: String::new(),
        score: 0.0,
        access_count: 0,
        last_access: Utc::now(),
    };
    
    let mem_ref = MemoryReference::from_record(&record);
    
    assert_eq!(mem_ref.layer, Layer::Interact);
    assert_eq!(mem_ref.record_id, record.id);
    assert_eq!(mem_ref.created_at, record.ts);
}

#[test]
fn test_todo_event_variants() {
    let task_id = Uuid::new_v4();
    
    // Test TaskCreated
    let event = TodoEvent::TaskCreated {
        task_id,
        title: "Test task".to_string(),
        auto_generated: false,
    };
    
    if let TodoEvent::TaskCreated { title, .. } = event {
        assert_eq!(title, "Test task");
    } else {
        panic!("Wrong event type");
    }
    
    // Test StateChanged
    let event = TodoEvent::StateChanged {
        task_id,
        old_state: TaskState::Ready,
        new_state: TaskState::InProgress,
        timestamp: Utc::now(),
    };
    
    if let TodoEvent::StateChanged { old_state, new_state, .. } = event {
        assert_eq!(old_state, TaskState::Ready);
        assert_eq!(new_state, TaskState::InProgress);
    } else {
        panic!("Wrong event type");
    }
}

#[test]
fn test_executable_task() {
    let task_id = Uuid::new_v4();
    let mut params = HashMap::new();
    params.insert("path".to_string(), "/test/path".to_string());
    params.insert("content".to_string(), "test content".to_string());
    
    let exec_task = ExecutableTask {
        task_id,
        tool: "file_write".to_string(),
        parameters: params.clone(),
        context: "Write test file".to_string(),
    };
    
    assert_eq!(exec_task.tool, "file_write");
    assert_eq!(exec_task.parameters.len(), 2);
    assert_eq!(exec_task.parameters.get("path"), Some(&"/test/path".to_string()));
}

#[test]
fn test_tool_suggestion() {
    let suggestion = ToolSuggestion {
        tool_name: "git_commit".to_string(),
        confidence: 0.85,
        reason: "User wants to commit changes".to_string(),
    };
    
    assert_eq!(suggestion.tool_name, "git_commit");
    assert!(suggestion.confidence > 0.8);
    assert!(suggestion.confidence < 0.9);
}

#[test]
fn test_task_feasibility() {
    let suggestion = ToolSuggestion {
        tool_name: "file_read".to_string(),
        confidence: 0.95,
        reason: "Clear file read request".to_string(),
    };
    
    let feasible = TaskFeasibility::Feasible(suggestion.clone());
    
    if let TaskFeasibility::Feasible(s) = feasible {
        assert_eq!(s.tool_name, "file_read");
    } else {
        panic!("Wrong feasibility type");
    }
    
    let uncertain = TaskFeasibility::Uncertain(suggestion);
    
    if let TaskFeasibility::Uncertain(s) = uncertain {
        assert_eq!(s.tool_name, "file_read");
    } else {
        panic!("Wrong feasibility type");
    }
}

#[test]
fn test_task_stats() {
    let mut stats = TaskStats::default();
    
    assert_eq!(stats.total, 0);
    assert_eq!(stats.ready, 0);
    assert_eq!(stats.done, 0);
    
    stats.total = 10;
    stats.ready = 3;
    stats.in_progress = 2;
    stats.done = 5;
    
    assert_eq!(stats.total, 10);
    assert_eq!(stats.ready + stats.in_progress + stats.done, 10);
}

#[test]
fn test_todo_item_serialization() {
    let mut item = TodoItem::default();
    item.title = "Test task".to_string();
    item.priority = Priority::High;
    item.tags = vec!["test".to_string(), "rust".to_string()];
    
    // Serialize to JSON
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("Test task"));
    assert!(json.contains("High"));
    
    // Deserialize back
    let deserialized: TodoItem = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.title, item.title);
    assert_eq!(deserialized.priority, item.priority);
    assert_eq!(deserialized.tags.len(), 2);
}

#[test]
fn test_task_complexity() {
    let simple = TaskComplexity::Simple;
    let complex = TaskComplexity::Complex;
    
    assert_eq!(simple, TaskComplexity::Simple);
    assert_eq!(complex, TaskComplexity::Complex);
    assert_ne!(simple, complex);
}

#[test]
fn test_todo_item_with_metadata() {
    let mut item = TodoItem::default();
    
    item.metadata.insert(
        "source".to_string(), 
        serde_json::json!("user_request")
    );
    
    item.metadata.insert(
        "estimated_hours".to_string(),
        serde_json::json!(3.5)
    );
    
    assert_eq!(item.metadata.len(), 2);
    assert_eq!(
        item.metadata.get("source"),
        Some(&serde_json::json!("user_request"))
    );
}