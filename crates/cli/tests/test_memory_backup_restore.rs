#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs;

#[test]
fn cli_memory_backup_creates_file() {
    let temp = TempDir::new().unwrap();
    let backups_dir = temp.path().join("backups");
    fs::create_dir_all(&backups_dir).unwrap();

    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.current_dir(&temp)
        .args(["memory","backup"]) // default name
        .env("CI","1")
        .env("MAGRAY_SKIP_AUTO_INSTALL","1")
        .env("ORT_DYLIB_PATH","scripts/onnxruntime/lib/libonnxruntime.so")
        .env("MAGRAY_FORCE_NO_ORT","1")
        .env("MAGRAY_CMD_TIMEOUT","20");
    let assert = cmd.assert();
    assert.success();

    let entries = fs::read_dir(&backups_dir).unwrap();
    let mut any = false;
    for e in entries.flatten() { if e.file_type().unwrap().is_file() { any = true; break; } }
    assert!(any, "backup file should be created");
}

#[tokio::test]
async fn api_backup_and_restore_roundtrip() {
    use memory::api::{UnifiedMemoryAPI, MemoryServiceTrait};
    use memory::di::UnifiedContainer;
    use memory::Layer;

    let svc = UnifiedContainer::new();
    let api = UnifiedMemoryAPI::new(std::sync::Arc::new(svc));

    let _ = api.remember("hello world".to_string(), memory::api::MemoryContext::new("test").with_layer(Layer::Interact));
    let _ = api.remember("rust language".to_string(), memory::api::MemoryContext::new("test").with_layer(Layer::Insights));

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("back.json");
    let count = api.backup_to_path(&path).await.expect("backup ok");
    assert!(count >= 0);

    let restored = api.restore_from_path(&path).await.expect("restore ok");
    assert!(restored >= 0);
}