use tools::git_ops::{GitStatus, GitCommit};
use tools::{Tool, ToolInput};
use std::collections::HashMap;
use anyhow::Result;

#[tokio::test]
async fn test_git_status_spec() {
    let git_status = GitStatus::new();
    let spec = git_status.spec();
    
    assert_eq!(spec.name, "git_status");
    assert!(spec.description.contains("статус"));
    assert!(spec.usage.contains("git_status"));
    assert!(!spec.examples.is_empty());
}

#[tokio::test]
async fn test_git_status_natural_language_parsing() -> Result<()> {
    let git_status = GitStatus::new();
    let input = git_status.parse_natural_language("покажи статус репозитория").await?;
    
    assert_eq!(input.command, "git_status");
    assert!(input.context.is_some());
    assert_eq!(input.context.unwrap(), "покажи статус репозитория");
    
    Ok(())
}

#[tokio::test]
async fn test_git_status_execute() -> Result<()> {
    let git_status = GitStatus::new();
    let input = ToolInput {
        command: "git_status".to_string(),
        args: HashMap::new(),
        context: None,
    };
    
    // Execute git status (may fail if not in git repo, but shouldn't panic)
    let result = git_status.execute(input).await;
    assert!(result.is_ok());
    
    let output = result.unwrap();
    // Result should have some content regardless of success/failure
    assert!(!output.result.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_git_commit_spec() {
    let git_commit = GitCommit::new();
    let spec = git_commit.spec();
    
    assert_eq!(spec.name, "git_commit");
    assert!(spec.description.contains("коммит"));
    assert!(spec.usage.contains("git_commit"));
    assert!(!spec.examples.is_empty());
}

#[tokio::test]
async fn test_git_commit_natural_language_parsing() -> Result<()> {
    let git_commit = GitCommit::new();
    let input = git_commit.parse_natural_language("создай коммит с сообщением fix bug").await?;
    
    assert_eq!(input.command, "git_commit");
    assert!(input.context.is_some());
    assert_eq!(input.context.unwrap(), "создай коммит с сообщением fix bug");
    
    Ok(())
}

#[tokio::test]
async fn test_git_commit_with_message() -> Result<()> {
    let git_commit = GitCommit::new();
    let mut args = HashMap::new();
    args.insert("message".to_string(), "test commit message".to_string());
    
    let input = ToolInput {
        command: "git_commit".to_string(),
        args,
        context: None,
    };
    
    // Execute git commit (may fail if nothing to commit, but shouldn't panic)
    let result = git_commit.execute(input).await;
    assert!(result.is_ok());
    
    let output = result.unwrap();
    // Result should have some content regardless of success/failure
    assert!(!output.result.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_git_commit_default_message() -> Result<()> {
    let git_commit = GitCommit::new();
    let input = ToolInput {
        command: "git_commit".to_string(),
        args: HashMap::new(), // No message provided
        context: None,
    };
    
    // Execute git commit with default message
    let result = git_commit.execute(input).await;
    assert!(result.is_ok());
    
    let output = result.unwrap();
    // Should use default message
    assert!(!output.result.is_empty());
    
    Ok(())
}

#[tokio::test] 
async fn test_git_tools_support_natural_language() {
    let git_status = GitStatus::new();
    let git_commit = GitCommit::new();
    
    assert!(git_status.supports_natural_language());
    assert!(git_commit.supports_natural_language());
}

#[tokio::test]
async fn test_git_status_formatted_output() -> Result<()> {
    let git_status = GitStatus::new();
    let input = ToolInput {
        command: "git_status".to_string(),
        args: HashMap::new(),
        context: None,
    };
    
    let result = git_status.execute(input).await?;
    
    // Should have formatted output for successful operations
    if result.success {
        assert!(result.formatted_output.is_some());
        let formatted = result.formatted_output.unwrap();
        assert!(formatted.contains("📂"));
        assert!(formatted.contains("Текущий статус"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_git_commit_metadata() -> Result<()> {
    let git_commit = GitCommit::new();
    let mut args = HashMap::new();
    args.insert("message".to_string(), "test metadata".to_string());
    
    let input = ToolInput {
        command: "git_commit".to_string(),
        args,
        context: None,
    };
    
    let result = git_commit.execute(input).await?;
    
    // Should include message in metadata for successful commits
    if result.success {
        assert!(result.metadata.contains_key("message"));
        assert_eq!(result.metadata.get("message"), Some(&"test metadata".to_string()));
    }
    
    Ok(())
}