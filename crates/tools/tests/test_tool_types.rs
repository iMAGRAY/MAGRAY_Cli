use std::collections::HashMap;
use tools::{ToolInput, ToolOutput, ToolRegistry, ToolSpec};

#[test]
fn test_tool_input_creation() {
    let mut args = HashMap::new();
    args.insert("param1".to_string(), "value1".to_string());
    args.insert("param2".to_string(), "value2".to_string());

    let input = ToolInput {
        command: "test_command".to_string(),
        args: args.clone(),
        context: Some("test context".to_string()),
        dry_run: false,
        timeout_ms: None,
    };

    assert_eq!(input.command, "test_command");
    assert_eq!(input.args, args);
    assert_eq!(input.context, Some("test context".to_string()));
}

#[test]
fn test_tool_input_clone() {
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let cloned = input.clone();
    assert_eq!(input.command, cloned.command);
}

#[test]
fn test_tool_output_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("meta1".to_string(), "metavalue1".to_string());

    let output = ToolOutput {
        success: true,
        result: "test result".to_string(),
        formatted_output: Some("formatted test result".to_string()),
        metadata: metadata.clone(),
    };

    assert!(output.success);
    assert_eq!(output.result, "test result");
    assert_eq!(
        output.formatted_output,
        Some("formatted test result".to_string())
    );
    assert_eq!(output.metadata, metadata);
}

#[test]
fn test_tool_output_failure() {
    let output = ToolOutput {
        success: false,
        result: "error message".to_string(),
        formatted_output: None,
        metadata: HashMap::new(),
    };

    assert!(!output.success);
    assert_eq!(output.result, "error message");
    assert!(output.formatted_output.is_none());
}

#[test]
fn test_tool_spec_creation() {
    let spec = ToolSpec {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        usage: "test_tool <arg>".to_string(),
        examples: vec!["test_tool hello".to_string(), "test_tool world".to_string()],
        input_schema: r#"{"arg": "string"}"#.to_string(),
    };

    assert_eq!(spec.name, "test_tool");
    assert_eq!(spec.description, "A test tool");
    assert_eq!(spec.usage, "test_tool <arg>");
    assert_eq!(spec.examples.len(), 2);
    assert!(spec.input_schema.contains("arg"));
}

#[test]
fn test_tool_registry_creation() {
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();

    // Should have registered default tools
    assert!(!tools.is_empty());

    // Check that basic tools are registered
    let tool_names: Vec<String> = tools.iter().map(|spec| spec.name.clone()).collect();
    assert!(tool_names.contains(&"file_read".to_string()));
    assert!(tool_names.contains(&"file_write".to_string()));
    assert!(tool_names.contains(&"git_status".to_string()));
    assert!(tool_names.contains(&"web_search".to_string()));
    assert!(tool_names.contains(&"shell_exec".to_string()));
}

#[test]
fn test_tool_registry_default() {
    let registry = ToolRegistry::default();
    let tools = registry.list_tools();

    // Default should be same as new()
    assert!(!tools.is_empty());
}

#[test]
fn test_tool_registry_get_existing_tool() {
    let registry = ToolRegistry::new();

    let file_read = registry.get("file_read");
    assert!(file_read.is_some());

    let spec = file_read.unwrap().spec();
    assert_eq!(spec.name, "file_read");
}

#[test]
fn test_tool_registry_get_nonexistent_tool() {
    let registry = ToolRegistry::new();

    let result = registry.get("nonexistent_tool");
    assert!(result.is_none());
}

#[test]
fn test_tool_registry_list_tools_contains_specs() {
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();

    // Each tool should have a valid spec
    for spec in tools {
        assert!(!spec.name.is_empty());
        assert!(!spec.description.is_empty());
        assert!(!spec.usage.is_empty());
    }
}

#[test]
fn test_tool_registry_all_tools_accessible() {
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();

    // Every tool in the list should be accessible via get()
    for spec in tools {
        let tool = registry.get(&spec.name);
        assert!(tool.is_some(), "Tool {} should be accessible", spec.name);

        let retrieved_spec = tool.unwrap().spec();
        assert_eq!(retrieved_spec.name, spec.name);
    }
}

#[tokio::test]
async fn test_tool_supports_natural_language() {
    let registry = ToolRegistry::new();

    if let Some(tool) = registry.get("file_read") {
        assert!(tool.supports_natural_language());
    }

    if let Some(tool) = registry.get("web_search") {
        assert!(tool.supports_natural_language());
    }
}

#[test]
fn test_tool_input_empty_args() {
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    assert!(input.args.is_empty());
    assert!(input.context.is_none());
}

#[test]
fn test_tool_output_empty_metadata() {
    let output = ToolOutput {
        success: true,
        result: "result".to_string(),
        formatted_output: None,
        metadata: HashMap::new(),
    };

    assert!(output.metadata.is_empty());
    assert!(output.formatted_output.is_none());
}

#[test]
fn test_tool_spec_empty_examples() {
    let spec = ToolSpec {
        name: "test".to_string(),
        description: "test".to_string(),
        usage: "test".to_string(),
        examples: Vec::new(),
        input_schema: "{}".to_string(),
    };

    assert!(spec.examples.is_empty());
}

#[test]
fn test_tool_registry_has_git_tools() {
    let registry = ToolRegistry::new();

    assert!(registry.get("git_status").is_some());
    assert!(registry.get("git_commit").is_some());
}

#[test]
fn test_tool_registry_has_file_tools() {
    let registry = ToolRegistry::new();

    assert!(registry.get("file_read").is_some());
    assert!(registry.get("file_write").is_some());
    assert!(registry.get("dir_list").is_some());
}
