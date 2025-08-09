#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn default_policy_blocks_shell_exec() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools", "run",
        "--name", "shell_exec",
        "--command", "echo",
        "--arg", "command=echo",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "15");
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}

#[test]
fn user_policy_overrides_allow_shell_exec() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join(".magray");
    fs::create_dir_all(&home).unwrap();
    let policy_path = home.join("policy.json");
    let mut f = fs::File::create(&policy_path).unwrap();
    // Allow shell_exec by overriding default
    write!(
        f,
        r#"{{"rules":[{{"subject_kind":"Tool","subject_name":"shell_exec","when_contains_args":null,"action":"Allow","reason":"override"}}]}}"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools", "run",
        "--name", "shell_exec",
        "--command", "echo",
        "--arg", "command=echo",
    ])
    .env("CI", "1")
    .env("MAGRAY_HOME", home.to_string_lossy().to_string())
    .env("MAGRAY_CMD_TIMEOUT", "15");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn default_policy_allows_memory_backup() {
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["memory", "backup"]).env("CI", "1").env("MAGRAY_CMD_TIMEOUT", "15");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn user_policy_blocks_memory_backup() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join(".magray");
    fs::create_dir_all(&home).unwrap();
    let policy_path = home.join("policy.json");
    let mut f = fs::File::create(&policy_path).unwrap();
    // Deny memory.backup
    write!(
        f,
        r#"{{"rules":[{{"subject_kind":"Command","subject_name":"memory.backup","when_contains_args":null,"action":"Deny","reason":"no backups"}}]}}"#
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["memory", "backup"]).env("CI", "1").env("MAGRAY_CMD_TIMEOUT", "15").env("MAGRAY_HOME", home.to_string_lossy().to_string());
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}