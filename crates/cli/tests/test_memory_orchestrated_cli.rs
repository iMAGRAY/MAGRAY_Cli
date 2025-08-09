#![cfg(all(feature = "extended-tests", feature = "orchestrated-search"))]

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn orchestrated_search_rerank_cli_orders_results() {
    let temp = TempDir::new().expect("temp dir");

    // add records
    let mut add1 = Command::cargo_bin("magray").expect("built");
    add1.current_dir(&temp)
        .args(["memory","add","rust ownership borrowing lifetimes","--layer","interact"]) 
        .env("CI","1").env("MAGRAY_SKIP_AUTO_INSTALL","1").env("MAGRAY_FORCE_NO_ORT","1").env("MAGRAY_CMD_TIMEOUT","20");
    add1.assert().success();

    let mut add2 = Command::cargo_bin("magray").expect("built");
    add2.current_dir(&temp)
        .args(["memory","add","python asyncio event loop","--layer","interact"]) 
        .env("CI","1").env("MAGRAY_SKIP_AUTO_INSTALL","1").env("MAGRAY_FORCE_NO_ORT","1").env("MAGRAY_CMD_TIMEOUT","20");
    add2.assert().success();

    // search with rerank
    let mut search = Command::cargo_bin("magray").expect("built");
    let assert = search.current_dir(&temp)
        .args(["memory","search","rust lifetimes borrowing","--layer","interact","--rerank","-k","2"]) 
        .env("CI","1").env("MAGRAY_SKIP_AUTO_INSTALL","1").env("MAGRAY_FORCE_NO_ORT","1").env("MAGRAY_CMD_TIMEOUT","20")
        .assert();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let pos_rust = out.find("rust ownership borrowing lifetimes");
    let pos_python = out.find("python asyncio event loop");
    if let (Some(r), Some(p)) = (pos_rust, pos_python) { assert!(r < p, "rerank should order rust-related first. Output: {}", out); } else { panic!("Expected results not found in output: {}", out); }
}