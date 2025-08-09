#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn tools_list_basic_and_details_and_json() {
    // human list
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["tools","list"]) // human output
        .env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("Registered Tools"));

    // details
    let mut cmd2 = Command::cargo_bin("magray").expect("built");
    let out2 = cmd2.args(["tools","list","--details"]).env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out2.status.success());
    let s2 = String::from_utf8_lossy(&out2.stdout);
    // Should show usage lines
    assert!(s2.contains("usage:"));

    // json
    let mut cmd3 = Command::cargo_bin("magray").expect("built");
    let out3 = cmd3.args(["tools","list","--json"]).env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out3.status.success());
    let s3 = String::from_utf8_lossy(&out3.stdout);
    // Validate basic JSON structure
    assert!(s3.trim_start().starts_with("["));
    assert!(s3.contains("\"name\""));
    assert!(s3.contains("\"usage_guide\""));
}

#[test]
fn tools_run_file_delete_triggers_dynamic_ask_non_interactive() {
    // file_delete marked as high-risk with side effects -> dynamic Ask should trigger and fail in non-interactive
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["tools","run","--name","file_delete","--command","delete","--arg","path=/tmp/should_not_exist.txt"]) 
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_NONINTERACTIVE","true")
        .output().expect("run ok");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("requires confirmation") || stderr.contains("Отменено пользователем"));
}

#[test]
fn tools_metrics_json_shape() {
    // Ask metrics snapshot JSON
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["tools","metrics","--json"]).env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("\"tools\""));
}

#[test]
fn tools_select_json_structure() {
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd.args(["tools","select","--query","скачай страницу","--json"]).env("CI","1").env("MAGRAY_NO_ANIM","1").output().expect("run ok");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    // Expect array of explanations with keys
    assert!(s.contains("tool_name"));
    assert!(s.contains("confidence_score"));
    assert!(s.contains("breakdown"));
}