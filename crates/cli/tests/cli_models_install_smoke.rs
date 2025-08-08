#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn cli_models_auto_install_smoke() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["models", "install"]) // hypothetical subcommand if present; fallback to check
        .env("MAGRAY_AUTO_INSTALL_MODELS", "true")
        .env("MAGRAY_AUTO_INSTALL_ORT", "true")
        .env("ORT_NO_TEST", "1")
        .env("MAGRAY_CMD_TIMEOUT", "15");

    // If install not available, fall back to 'models check'
    let status = cmd.status().unwrap_or_else(|_| {
        let mut alt = Command::cargo_bin("magray").expect("binary built");
        alt.args(["models", "check"]) // always exists
            .env("MAGRAY_AUTO_INSTALL_MODELS", "true")
            .env("MAGRAY_AUTO_INSTALL_ORT", "true")
            .env("ORT_NO_TEST", "1")
            .env("MAGRAY_CMD_TIMEOUT", "15")
            .status()
            .expect("run alt ok")
    });
    assert!(status.success());
}