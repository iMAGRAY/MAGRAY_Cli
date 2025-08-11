#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn env_policy_allows_web_search() {
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_search","when_contains_args":null,"action":"Allow","reason":"env-allow"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools",
        "run",
        "--name",
        "web_search",
        "--command",
        "search",
        "--arg",
        "query=rust",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "20")
    .env("MAGRAY_POLICY_JSON", policy);
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn env_policy_blocks_web_fetch_by_domain() {
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_fetch","when_contains_args":{"domain":"example.com"},"action":"Deny","reason":"blocked-domain"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools",
        "run",
        "--name",
        "web_fetch",
        "--command",
        "fetch",
        "--arg",
        "url=https://example.com/path?q=1",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "20")
    .env("MAGRAY_POLICY_JSON", policy);
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}

#[test]
fn env_policy_blocks_web_search_with_keyword_secret() {
    // Tools handler enriches args with keyword=secret when query contains it
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_search","when_contains_args":{"keyword":"secret"},"action":"Deny","reason":"no secrets"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools",
        "run",
        "--name",
        "web_search",
        "--command",
        "search",
        "--arg",
        "query=find secret docs",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "20")
    .env("MAGRAY_POLICY_JSON", policy);
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}

#[test]
fn env_policy_ask_with_autoapprove_succeeds() {
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_search","when_contains_args":null,"action":"Ask","reason":"medium"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools",
        "run",
        "--name",
        "web_search",
        "--command",
        "search",
        "--arg",
        "query=rust",
    ])
    .env("CI", "1")
    .env("MAGRAY_CMD_TIMEOUT", "20")
    .env("MAGRAY_POLICY_JSON", policy)
    .env("MAGRAY_AUTO_APPROVE_ASK", "true");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn env_policy_ask_memory_backup_noninteractive_fails_autoapprove_passes() {
    let ask_backup = r#"{"rules":[{"subject_kind":"Command","subject_name":"memory.backup","when_contains_args":null,"action":"Ask","reason":"medium"}]}"#;
    // Non-interactive should fail
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["memory", "backup"])
        .env("CI", "1")
        .env("MAGRAY_CMD_TIMEOUT", "20")
        .env("MAGRAY_NONINTERACTIVE", "true")
        .env("MAGRAY_POLICY_JSON", ask_backup);
    let status = cmd.status().expect("run ok");
    assert!(!status.success());

    // Auto-approve should pass
    let mut cmd2 = Command::cargo_bin("magray").expect("binary built");
    cmd2.args(["memory", "backup"])
        .env("CI", "1")
        .env("MAGRAY_CMD_TIMEOUT", "20")
        .env("MAGRAY_AUTO_APPROVE_ASK", "true")
        .env("MAGRAY_POLICY_JSON", ask_backup);
    let status2 = cmd2.status().expect("run ok");
    assert!(status2.success());
}

#[test]
fn env_policy_ask_memory_restore_noninteractive_fails_autoapprove_passes() {
    let ask_restore = r#"{"rules":[{"subject_kind":"Command","subject_name":"memory.restore","when_contains_args":null,"action":"Ask","reason":"high"}]}"#;
    // Non-interactive should fail
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["memory", "restore", "/dev/null"])
        .env("CI", "1")
        .env("MAGRAY_CMD_TIMEOUT", "20")
        .env("MAGRAY_NONINTERACTIVE", "true")
        .env("MAGRAY_POLICY_JSON", ask_restore);
    let status = cmd.status().expect("run ok");
    assert!(!status.success());

    // Auto-approve should pass
    let mut cmd2 = Command::cargo_bin("magray").expect("binary built");
    cmd2.args(["memory", "restore", "/dev/null"])
        .env("CI", "1")
        .env("MAGRAY_CMD_TIMEOUT", "20")
        .env("MAGRAY_AUTO_APPROVE_ASK", "true")
        .env("MAGRAY_POLICY_JSON", ask_restore);
    let status2 = cmd2.status().expect("run ok");
    assert!(status2.success());
}
