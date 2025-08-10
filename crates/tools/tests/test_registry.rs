use anyhow::Result;
use std::collections::HashMap;
use tools::{Tool, ToolInput, ToolOutput, ToolRegistry, ToolSpec};

// Mock tool для тестирования
struct MockTool {
    name: String,
    calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl MockTool {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            calls: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    #[allow(dead_code)]
    fn call_count(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl Tool for MockTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: format!("Mock tool {}", self.name),
            usage: format!("mock_{} <args>", self.name),
            examples: vec![format!("mock_{} test", self.name)],
            input_schema: "{}".to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: false,
        }
    }

    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(ToolOutput {
            success: true,
            result: format!("Mock {} executed", self.name),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: self.name.clone(),
            args: HashMap::from([("query".to_string(), query.to_string())]),
            context: None,
            dry_run: false,
            timeout_ms: None,
        })
    }
}

#[test]
fn test_tool_registry_creation() {
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();

    // Проверяем что базовые инструменты зарегистрированы
    assert!(tools.len() >= 7, "Should have at least 7 tools registered");

    // Проверяем наличие основных инструментов
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"file_read".to_string()));
    assert!(tool_names.contains(&"file_write".to_string()));
    assert!(tool_names.contains(&"dir_list".to_string()));
    assert!(tool_names.contains(&"git_status".to_string()));
    assert!(tool_names.contains(&"git_commit".to_string()));
    assert!(tool_names.contains(&"web_search".to_string()));
    assert!(tool_names.contains(&"shell_exec".to_string()));
}

#[test]
fn test_tool_registration() {
    let mut registry = ToolRegistry::new();
    let mock_tool = MockTool::new("test_tool");

    registry.register("test_tool", Box::new(mock_tool));

    // Проверяем что инструмент зарегистрирован
    assert!(registry.get("test_tool").is_some());

    // Проверяем spec
    let tool = registry.get("test_tool").unwrap();
    let spec = tool.spec();
    assert_eq!(spec.name, "test_tool");
    assert_eq!(spec.description, "Mock tool test_tool");
}

#[test]
fn test_get_nonexistent_tool() {
    let registry = ToolRegistry::new();
    assert!(registry.get("nonexistent").is_none());
}

#[tokio::test]
async fn test_tool_execution() {
    let mut registry = ToolRegistry::new();
    let mock_tool = MockTool::new("executor");
    let tool_ref = Box::new(mock_tool);

    registry.register("executor", tool_ref);

    let tool = registry.get("executor").unwrap();
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = tool.execute(input).await.unwrap();
    assert!(output.success);
    assert_eq!(output.result, "Mock executor executed");
}

#[test]
fn test_list_tools() {
    let mut registry = ToolRegistry::new();

    // Добавляем несколько mock инструментов
    registry.register("tool1", Box::new(MockTool::new("tool1")));
    registry.register("tool2", Box::new(MockTool::new("tool2")));

    let tools = registry.list_tools();

    // Должно быть минимум 9 инструментов (7 базовых + 2 mock)
    assert!(tools.len() >= 9);

    // Проверяем что наши mock инструменты в списке
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    assert!(tool_names.contains(&"tool1".to_string()));
    assert!(tool_names.contains(&"tool2".to_string()));
}

#[tokio::test]
async fn test_natural_language_support() {
    let mock_tool = MockTool::new("nl_tool");

    // По умолчанию поддерживает natural language
    assert!(mock_tool.supports_natural_language());

    // Проверяем парсинг
    let input = mock_tool
        .parse_natural_language("test query")
        .await
        .unwrap();
    assert_eq!(input.command, "nl_tool");
    assert_eq!(input.args.get("query").unwrap(), "test query");
}

#[test]
fn test_tool_input_serialization() {
    let input = ToolInput {
        command: "test_cmd".to_string(),
        args: HashMap::from([
            ("arg1".to_string(), "value1".to_string()),
            ("arg2".to_string(), "value2".to_string()),
        ]),
        context: Some("test context".to_string()),
        dry_run: false,
        timeout_ms: None,
    };


    // Сериализация
    let json = serde_json::to_string(&input).unwrap();
    assert!(json.contains("test_cmd"));
    assert!(json.contains("arg1"));
    assert!(json.contains("value1"));

    // Десериализация
    let deserialized: ToolInput = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.command, "test_cmd");
    assert_eq!(deserialized.args.len(), 2);
    assert_eq!(deserialized.context, Some("test context".to_string()));
}

#[test]
fn test_tool_output_serialization() {
    let mut metadata = HashMap::new();
    metadata.insert("key1".to_string(), "value1".to_string());

    let output = ToolOutput {
        success: true,
        result: "test result".to_string(),
        formatted_output: Some("formatted result".to_string()),
        metadata,
    };

    // Сериализация
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("true"));
    assert!(json.contains("test result"));
    assert!(json.contains("formatted result"));

    // Десериализация
    let deserialized: ToolOutput = serde_json::from_str(&json).unwrap();
    assert!(deserialized.success);
    assert_eq!(deserialized.result, "test result");
    assert_eq!(
        deserialized.formatted_output,
        Some("formatted result".to_string())
    );
    assert_eq!(deserialized.metadata.get("key1").unwrap(), "value1");
}

#[test]
fn test_default_trait() {
    let registry1 = ToolRegistry::new();
    let registry2 = ToolRegistry::default();

    // Оба должны иметь одинаковое количество инструментов
    assert_eq!(registry1.list_tools().len(), registry2.list_tools().len());
}
