#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn memory_help_runs() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["memory", "--help"])
        .env("MAGRAY_CMD_TIMEOUT", "30");
    assert!(cmd.status().unwrap().success());
}
