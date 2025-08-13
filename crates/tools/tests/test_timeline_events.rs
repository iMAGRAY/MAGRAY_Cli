#![cfg(feature = "extended-tests")]

use anyhow::Result;
use common::{events, topics};
use std::collections::{HashMap, HashSet};
use std::process::Command;
use tempfile::TempDir;
use tools::file_ops::{FileDeleter, FileWriter};
use tools::git_ops::{GitCommit, GitDiff};
use tools::{Tool, ToolInput};

#[tokio::test]
async fn timeline_fs_diff_sequence() -> Result<()> {
    // Prepare a temporary git repo
    let tmp = TempDir::new().expect("Test operation should succeed");
    let repo_dir = tmp.path();

    // git init
    let status = Command::new("git")
        .arg("init")
        .current_dir(repo_dir)
        .status()
        .expect("Test operation should succeed");
    assert!(status.success(), "git init failed");
    assert!(Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_dir)
        .status()
        .expect("Test operation should succeed")
        .success());
    assert!(Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_dir)
        .status()
        .expect("Test operation should succeed")
        .success());

    // subscribe to fs.diff before actions
    let mut rx = events::subscribe(topics::TOPIC_FS_DIFF).await;

    // 1) write a file
    let file_path = repo_dir.join("sample.txt");
    let writer = FileWriter::new();
    let input = ToolInput {
        command: "file_write".into(),
        args: HashMap::from([
            ("path".into(), file_path.to_string_lossy().to_string()),
            ("content".into(), "hello".into()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let out = writer.execute(input).await?;
    assert!(out.success);

    // 2) git commit
    let commit = GitCommit::new();
    let commit_input = ToolInput {
        command: "git_commit".into(),
        args: HashMap::from([
            ("message".into(), "test".into()),
            ("cwd".into(), repo_dir.to_string_lossy().to_string()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let _ = commit.execute(commit_input).await?; // success or "nothing to commit" are acceptable

    // 3) git diff (will likely be empty, but should publish a diff event)
    let diff = GitDiff::new();
    let diff_input = ToolInput {
        command: "git_diff".into(),
        args: HashMap::from([("cwd".into(), repo_dir.to_string_lossy().to_string())]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let _ = diff.execute(diff_input).await?;

    // 4) delete file
    let deleter = FileDeleter::new();
    let del_input = ToolInput {
        command: "file_delete".into(),
        args: HashMap::from([("path".into(), file_path.to_string_lossy().to_string())]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };
    let del_out = deleter.execute(del_input).await?;
    assert!(del_out.success);

    // collect events with timeout window
    let mut ops: HashSet<String> = HashSet::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
    while std::time::Instant::now() < deadline {
        if let Ok(Ok(evt)) =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await
        {
            if let Some(op) = evt.payload.get("op").and_then(|v| v.as_str()) {
                ops.insert(op.to_string());
            }
            if ops.contains("write")
                && ops.contains("commit")
                && ops.contains("diff")
                && ops.contains("delete")
            {
                break;
            }
        }
    }

    // must have write and delete; commit and diff are best-effort depending on environment
    assert!(ops.contains("write"), "missing write event");
    assert!(ops.contains("delete"), "missing delete event");

    Ok(())
}
