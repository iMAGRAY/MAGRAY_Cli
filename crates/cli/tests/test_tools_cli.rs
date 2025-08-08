#![cfg(all(feature = "extended-tests", feature = "legacy-tests"))]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn tools_help_runs() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools"]).env("MAGRAY_CMD_TIMEOUT", "30");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}