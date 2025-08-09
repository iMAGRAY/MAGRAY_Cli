use assert_cmd::prelude::*; // Add methods on commands
use std::process::Command;

#[test]
fn cli_models_check_exits_quickly() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["models", "check"]) // use lowercase subcommand
        .env("MAGRAY_AUTO_INSTALL_MODELS", "true")
        .env("MAGRAY_AUTO_INSTALL_ORT", "true")
        .env("ORT_NO_TEST", "1")
        .env("MAGRAY_CMD_TIMEOUT", "30");

    let status = cmd.status().expect("run ok");
    assert!(status.success(), "cli models check should succeed");
}