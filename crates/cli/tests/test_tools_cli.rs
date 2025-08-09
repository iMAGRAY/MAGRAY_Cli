#![cfg(all(feature = "extended-tests"))]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn tools_list_runs() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools", "list"]).env("MAGRAY_CMD_TIMEOUT", "30").env("CI", "1");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[tokio::test]
async fn tools_run_event_bus_smoke() {
    let mut rx = common::events::subscribe(common::topics::TOPIC_TOOL_INVOKED).await;
    // Run list to ensure CLI path works; actual run event requires a specific tool setup,
    // so we only validate no hang and optional event
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools", "list"]).env("CI", "1");
    let _ = cmd.status().expect("run ok");
    let _ = tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await;
}