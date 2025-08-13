#![cfg(feature = "extended-tests")]

use anyhow::Result;
use std::collections::HashMap;
use tempfile::TempDir;
use tools::shell_ops::ShellExec;
use tools::{Tool, ToolInput};

#[tokio::test]
async fn shell_exec_dry_run_preview() -> Result<()> {
    let tool = ShellExec::new();
    let input = ToolInput {
        command: "shell_exec".into(),
        args: HashMap::from([("command".into(), "echo hello".into())]),
        context: None,
        dry_run: true,
        timeout_ms: None,
    };
    let out = tool.execute(input).await?;
    assert!(out.success);
    assert!(out
        .formatted_output
        .expect("Test operation should succeed")
        .contains("[dry-run]"));
    Ok(())
}

#[tokio::test]
async fn shell_exec_timeout_triggers() -> Result<()> {
    let tool = ShellExec::new();
    let input = ToolInput {
        command: "shell_exec".into(),
        args: HashMap::from([("command".into(), "sleep 2".into())]),
        context: None,
        dry_run: false,
        timeout_ms: Some(200),
    };
    let out = tool.execute(input).await?;
    assert!(!out.success);
    assert!(out.result.contains("таймаут"));
    assert_eq!(
        out.metadata.get("timeout_ms").map(|s| s.as_str()),
        Some("200")
    );
    Ok(())
}

#[tokio::test]
async fn shell_exec_truncates_stdout_when_exceeds_limit() -> Result<()> {
    let tool = ShellExec::new();
    // Generate ~500KB of 'A' using /dev/zero and tr
    let cmd = "head -c 500000 /dev/zero | tr '\\0' 'A'".to_string();
    let mut args = HashMap::new();
    args.insert("command".into(), cmd);
    args.insert("max_output_kb".into(), "4".into()); // 4KB limit
    let out = tool
        .execute(ToolInput {
            command: "shell_exec".into(),
            args,
            context: None,
            dry_run: false,
            timeout_ms: Some(5000),
        })
        .await?;
    // success true (exit code 0), but truncated
    assert!(out.success);
    assert_eq!(
        out.metadata.get("stdout_truncated").map(|s| s.as_str()),
        Some("true")
    );
    Ok(())
}

#[tokio::test]
async fn shell_exec_respects_cwd() -> Result<()> {
    let tool = ShellExec::new();
    let tmp = TempDir::new().expect("Test operation should succeed");
    let cwd = tmp.path().to_string_lossy().to_string();
    let mut args = HashMap::new();
    args.insert("command".into(), "pwd".into());
    args.insert("cwd".into(), cwd.clone());
    let out = tool
        .execute(ToolInput {
            command: "shell_exec".into(),
            args,
            context: None,
            dry_run: false,
            timeout_ms: Some(5000),
        })
        .await?;
    assert!(out.success);
    // stdout may include trailing newline
    let printed = out.result.trim();
    assert!(printed.ends_with(&cwd));
    assert_eq!(
        out.metadata.get("cwd").map(|s| s.as_str()),
        Some(cwd.as_str())
    );
    Ok(())
}
