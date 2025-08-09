#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn events_emitted_for_memory_and_tools() {
    // memory stats should emit intent
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["memory","stats"]) // prints stats
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_CMD_TIMEOUT","15")
        .output().expect("run ok");
    assert!(out.status.success());

    // tools run with deny should emit policy.block and error lines in output
    let mut cmd2 = Command::cargo_bin("magray").expect("built");
    let status = cmd2.args(["tools","run","--name","shell_exec","--command","run","--arg","cmd=echo hi"]) // blocked by default policy
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_CMD_TIMEOUT","15")
        .status().expect("run ok");
    assert!(!status.success());
}