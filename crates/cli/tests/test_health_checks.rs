#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn health_runs() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["health"]).env("MAGRAY_CMD_TIMEOUT", "60");
    assert!(cmd.status().unwrap().success());
}
