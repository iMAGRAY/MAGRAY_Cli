#![cfg(all(feature = "extended-tests", feature = "legacy-tests"))]

use anyhow::Result;
use std::collections::HashMap;
use tools::shell_ops::ShellExec;
use tools::{Tool, ToolInput};

#[tokio::test]
async fn test_shell_exec_spec() {
    let shell_exec = ShellExec::new();
    let spec = shell_exec.spec();

    assert_eq!(spec.name, "shell_exec");
    assert!(spec.description.contains("Выполняет команды"));
    assert!(spec.usage.contains("shell_exec"));
    assert!(!spec.examples.is_empty());
    assert!(spec.input_schema.contains("command"));
}

#[tokio::test]
async fn test_shell_exec_natural_language_parsing() -> Result<()> {
    let shell_exec = ShellExec::new();
    let input = shell_exec
        .parse_natural_language("echo hello world")
        .await?;

    assert_eq!(input.command, "shell_exec");
    assert!(input.args.contains_key("command"));
    assert_eq!(
        input.args.get("command"),
        Some(&"echo hello world".to_string())
    );
    assert!(input.context.is_some());

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_empty_command() -> Result<()> {
    let shell_exec = ShellExec::new();
    let input = ToolInput {
        command: "shell_exec".to_string(),
        args: HashMap::new(), // No command provided
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    assert!(!result.success);
    assert!(result.result.contains("Не указана команда"));
    assert!(result.formatted_output.is_none());

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_empty_command_string() -> Result<()> {
    let shell_exec = ShellExec::new();
    let mut args = HashMap::new();
    args.insert("command".to_string(), "   ".to_string()); // Empty/whitespace command

    let input = ToolInput {
        command: "shell_exec".to_string(),
        args,
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    assert!(!result.success);
    assert!(result.result.contains("Не указана команда"));

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_supports_natural_language() {
    let shell_exec = ShellExec::new();
    assert!(shell_exec.supports_natural_language());
}

#[tokio::test]
async fn test_shell_exec_echo_command() -> Result<()> {
    let shell_exec = ShellExec::new();
    let mut args = HashMap::new();
    args.insert("command".to_string(), "echo test_output".to_string());

    let input = ToolInput {
        command: "shell_exec".to_string(),
        args,
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    // Echo should generally work on all platforms
    assert!(result.success);
    assert!(result.result.contains("test_output"));
    assert!(result.formatted_output.is_some());

    // Check metadata
    assert!(result.metadata.contains_key("status_code"));
    assert!(result.metadata.contains_key("platform"));
    assert_eq!(result.metadata.get("status_code"), Some(&"0".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_formatted_output() -> Result<()> {
    let shell_exec = ShellExec::new();
    let mut args = HashMap::new();
    args.insert("command".to_string(), "echo formatted_test".to_string());

    let input = ToolInput {
        command: "shell_exec".to_string(),
        args,
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    if result.success {
        assert!(result.formatted_output.is_some());
        let formatted = result.formatted_output.unwrap();
        assert!(formatted.starts_with("$ echo formatted_test"));
        assert!(formatted.contains("formatted_test"));
    }

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_invalid_command() -> Result<()> {
    let shell_exec = ShellExec::new();
    let mut args = HashMap::new();
    args.insert(
        "command".to_string(),
        "nonexistent_command_123456".to_string(),
    );

    let input = ToolInput {
        command: "shell_exec".to_string(),
        args,
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    // Should fail but not panic
    assert!(!result.success);
    assert!(result.result.contains("ошибкой") || result.result.contains("error"));
    assert!(result.formatted_output.is_none());

    // Should still have metadata
    assert!(result.metadata.contains_key("status_code"));
    assert!(result.metadata.contains_key("platform"));

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_platform_metadata() -> Result<()> {
    let shell_exec = ShellExec::new();
    let mut args = HashMap::new();
    args.insert("command".to_string(), "echo platform_test".to_string());

    let input = ToolInput {
        command: "shell_exec".to_string(),
        args,
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let result = shell_exec.execute(input).await?;

    assert!(result.metadata.contains_key("platform"));
    let platform = result.metadata.get("platform").unwrap();
    assert!(platform == "windows" || platform == "unix");

    Ok(())
}

#[tokio::test]
async fn test_shell_exec_multiple_examples() {
    let shell_exec = ShellExec::new();
    let spec = shell_exec.spec();

    // Should have multiple examples
    assert!(spec.examples.len() >= 3);

    // Examples should contain relevant commands
    let examples_str = spec.examples.join(" ");
    assert!(examples_str.contains("mkdir") || examples_str.contains("echo"));
}

#[tokio::test]
async fn test_shell_exec_cross_platform_spec() {
    let shell_exec = ShellExec::new();
    let spec = shell_exec.spec();

    // Description should mention both Windows and Unix
    assert!(spec.description.contains("Windows") || spec.description.contains("cmd"));
    assert!(spec.description.contains("Unix") || spec.description.contains("sh"));
}

#[tokio::test]
async fn test_shell_exec_natural_language_context() -> Result<()> {
    let shell_exec = ShellExec::new();
    let command = "ls -la";
    let input = shell_exec.parse_natural_language(command).await?;

    assert_eq!(input.context, Some(command.to_string()));
    assert_eq!(input.args.get("command"), Some(&command.to_string()));

    Ok(())
}
