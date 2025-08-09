#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;

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

#[test]
fn tools_select_json_contains_permissions_and_dryrun_breakdown() {
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd
        .args(["tools","select","--query","скачай страницу","--json"]) // json
        .env("CI","1").env("MAGRAY_NO_ANIM","1")
        .output().expect("run ok");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("\"breakdown\""));
    assert!(s.contains("permissions_adjust"));
    assert!(s.contains("dry_run_bonus"));
}

#[test]
fn fs_sandbox_blocks_outside_root() {
    let tmp = TempDir::new().unwrap();
    let allowed = tmp.path().canonicalize().unwrap();
    // Try write into /tmp (allowed) and into / (blocked) — we check blocked path deterministically using parent dir
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd
        .args(["tools","run","--name","file_write","--command","write","--arg","path=/etc/shadow","--arg","content=oops"]) // obviously forbidden
        .env("CI","1").env("MAGRAY_NO_ANIM","1")
        .env("MAGRAY_NONINTERACTIVE","true")
        .env("MAGRAY_FS_SANDBOX","1")
        .env("MAGRAY_FS_ROOTS", allowed.to_string_lossy().to_string())
        .output().expect("run ok");
    assert!(!out.status.success());
    let s = String::from_utf8_lossy(&out.stderr);
    assert!(s.contains("песочницы"));
}

#[test]
fn net_sandbox_blocks_and_allows() {
    // Disallow all
    let mut cmd = Command::cargo_bin("magray").expect("built");
    let out = cmd
        .args(["tools","run","--name","web_fetch","--command","get","--arg","url=https://example.com"]) // network
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_NONINTERACTIVE","true")
        .env("MAGRAY_NET_ALLOW","") // no allowlist
        .output().expect("run ok");
    assert!(!out.status.success());
    let s = String::from_utf8_lossy(&out.stderr);
    assert!(s.to_lowercase().contains("сеть запрещена"));

    // Allow example.com
    let mut cmd2 = Command::cargo_bin("magray").expect("built");
    let out2 = cmd2
        .args(["tools","run","--name","web_fetch","--command","get","--arg","url=https://example.com"]) // network
        .env("CI","1").env("MAGRAY_NO_ANIM","1").env("MAGRAY_NONINTERACTIVE","true")
        .env("MAGRAY_NET_ALLOW","example.com")
        .output().expect("run ok");
    // It may fail due to network unavailability in test env, but should not be blocked by sandbox. Accept success=false but not the sandbox error message.
    let s2e = String::from_utf8_lossy(&out2.stderr).to_lowercase();
    assert!(!s2e.contains("сеть запрещена"));
}