#![cfg(feature = "extended-tests")]
use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn memory_search_rerank_fallback_cli() {
    let temp = TempDir::new().unwrap();
    // Prepare environment
    let mut add = Command::cargo_bin("magray").expect("binary built");
    add.current_dir(&temp)
        .args(["memory","add","rust tokio async runtimes","--layer","interact"]) // 1
        .env("MAGRAY_SKIP_AUTO_INSTALL","1")
        .env("MAGRAY_FORCE_NO_ORT","1")
        .env("CI","1");
    add.assert().success();

    let mut add2 = Command::cargo_bin("magray").expect("binary built");
    add2.current_dir(&temp)
        .args(["memory","add","python asyncio event loop","--layer","interact"]) // 2
        .env("MAGRAY_SKIP_AUTO_INSTALL","1")
        .env("MAGRAY_FORCE_NO_ORT","1")
        .env("CI","1");
    add2.assert().success();

    let mut add3 = Command::cargo_bin("magray").expect("binary built");
    add3.current_dir(&temp)
        .args(["memory","add","rust ownership borrowing lifetimes","--layer","interact"]) // 3
        .env("MAGRAY_SKIP_AUTO_INSTALL","1")
        .env("MAGRAY_FORCE_NO_ORT","1")
        .env("CI","1");
    add3.assert().success();

    // Run search with rerank flag
    let mut search = Command::cargo_bin("magray").expect("binary built");
    let assert = search.current_dir(&temp)
        .args(["memory","search","rust lifetimes borrowing","--layer","interact","--rerank","-k","3"]) // top-3
        .env("MAGRAY_SKIP_AUTO_INSTALL","1")
        .env("MAGRAY_FORCE_NO_ORT","1")
        .env("CI","1")
        .assert();
    let output = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    // We do not rely on exact formatting (colors/indices); verify ordering of results heuristically
    // Heuristic: expect the "rust ownership borrowing lifetimes" appears before python
    let pos_rust = output.find("rust ownership borrowing lifetimes");
    let pos_python = output.find("python asyncio event loop");
    if let (Some(r), Some(p)) = (pos_rust, pos_python) { assert!(r < p); } else { panic!("Expected target texts not found in output: {}", output); }
}