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
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools", "list"]).env("CI", "1").env("MAGRAY_CMD_TIMEOUT", "20");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn tools_policy_block_shell_exec() {
    // Attempt to run blocked tool; expect non-zero exit
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools", "run",
        "--name", "shell_exec",
        "--command", "echo",
        "--arg", "command=echo",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "20");
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}