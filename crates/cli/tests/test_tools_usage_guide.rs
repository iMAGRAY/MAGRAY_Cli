#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn tools_list_has_usage_guide() {
    // Use tools list to print specs; we don't have JSON command, so sanity check by stdout content
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["tools","list"]) // human output
        .env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Registered Tools"));
}