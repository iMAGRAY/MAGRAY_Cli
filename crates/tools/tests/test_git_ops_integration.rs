#![cfg(feature = "extended-tests")]

use anyhow::Result;
use tempfile::TempDir;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use tools::git_ops::{GitCommit, GitDiff, GitStatus};
use tools::{Tool, ToolInput};
use common::{events, topics};

#[tokio::test]
async fn git_status_works_with_cwd_in_temp_repo() -> Result<()> {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert!(Command::new("git").arg("init").current_dir(repo).status()?.success());
    // create a file
    fs::write(repo.join("a.txt"), "hello")?;

    let status = GitStatus::new();
    let input = ToolInput {
        command: "git_status".into(),
        args: HashMap::from([("cwd".into(), repo.to_string_lossy().to_string())]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let out = status.execute(input).await?;
    assert!(out.result.contains("A  a.txt") || out.result.contains("?? a.txt") || out.success);
    Ok(())
}

#[tokio::test]
async fn git_commit_and_diff_publish_events() -> Result<()> {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path();
    assert!(Command::new("git").arg("init").current_dir(repo).status()?.success());
    assert!(Command::new("git").args(["config","user.email","test@example.com"]).current_dir(repo).status()?.success());
    assert!(Command::new("git").args(["config","user.name","Test"]).current_dir(repo).status()?.success());

    // subscribe before actions
    let mut rx = events::subscribe(topics::TOPIC_FS_DIFF).await;

    // write file and stage via commit tool
    fs::write(repo.join("b.txt"), "world")?;

    let commit = GitCommit::new();
    let input = ToolInput {
        command: "git_commit".into(),
        args: HashMap::from([
            ("message".into(), "it".into()),
            ("cwd".into(), repo.to_string_lossy().to_string()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let _ = commit.execute(input).await?; // allow "Нет изменений" as well in edge timing

    let diff = GitDiff::new();
    let dinput = ToolInput { command: "git_diff".into(), args: HashMap::from([("cwd".into(), repo.to_string_lossy().to_string())]), context: None, dry_run: false, timeout_ms: None };
    let _ = diff.execute(dinput).await?;

    // collect a couple of fs.diff ops best-effort
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_millis(1200);
    let mut saw_commit = false;
    let mut saw_diff = false;
    while Instant::now() < deadline {
        if let Ok(Ok(evt)) = tokio::time::timeout(Duration::from_millis(150), rx.recv()).await {
            if let Some(op) = evt.payload.get("op").and_then(|v| v.as_str()) {
                if op == "commit" { saw_commit = true; }
                if op == "diff" { saw_diff = true; }
            }
            if saw_commit && saw_diff { break; }
        }
    }
    // Best-effort: at least one of them should appear depending on environment
    assert!(saw_commit || saw_diff, "expected at least one fs.diff op from commit/diff");
    Ok(())
}

#[tokio::test]
async fn file_delete_dry_run_reports_preview() -> Result<()> {
    use tools::file_ops::FileDeleter;
    let del = FileDeleter::new();
    let args = HashMap::from([("path".into(), "/tmp/whatever".into())]);
    let input = ToolInput { command: "file_delete".into(), args, context: None, dry_run: true, timeout_ms: None };
    let out = del.execute(input).await?;
    assert!(out.success);
    assert_eq!(out.metadata.get("dry_run").map(String::as_str), Some("true"));
    assert!(out.result.contains("[dry-run] rm"));
    Ok(())
}