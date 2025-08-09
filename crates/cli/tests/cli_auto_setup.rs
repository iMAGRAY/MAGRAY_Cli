#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn cli_auto_setup_status_smoke() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["status"]) // run a safe subcommand that triggers startup hooks
        .env("MAGRAY_NO_ANIM", "1")
        .env("MAGRAY_CMD_TIMEOUT", "20")
        .env("CI", "1") // prefer non-interactive defaults
        .env("MAGRAY_AUTO_INSTALL_MODELS", "true")
        .env("MAGRAY_AUTO_INSTALL_ORT", "true")
        .env("ORT_NO_TEST", "1");

    let status = cmd.status().expect("run ok");
    assert!(status.success());
}