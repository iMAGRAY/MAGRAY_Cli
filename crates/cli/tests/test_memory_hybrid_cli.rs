#![cfg(all(feature = "extended-tests", feature = "orchestrated-search"))]

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn orchestrated_search_hybrid_cli_basic() {
    let temp = TempDir::new().expect("temp dir");

    // add a few records
    for text in [
        "rust concurrency tokio",
        "python coroutines asyncio",
        "rust ownership lifetimes",
    ] {
        let mut add = Command::cargo_bin("magray").expect("built");
        add.current_dir(&temp)
            .args(["memory","add",text,"--layer","interact"]) 
            .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_SKIP_AUTO_INSTALL","1").env("MAGRAY_FORCE_NO_ORT","1").env("MAGRAY_CMD_TIMEOUT","20");
        add.assert().success();
    }

    // hybrid search should work like normal search if vector omitted
    let mut search = Command::cargo_bin("magray").expect("built");
    let assert = search.current_dir(&temp)
        .args(["memory","search","rust lifetimes","--layer","interact","--hybrid","-k","3"]) 
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_SKIP_AUTO_INSTALL","1").env("MAGRAY_FORCE_NO_ORT","1").env("MAGRAY_CMD_TIMEOUT","20")
        .assert();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(out.contains("rust ownership lifetimes"), "hybrid search output: {}", out);
}