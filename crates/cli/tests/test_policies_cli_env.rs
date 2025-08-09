#![cfg(feature = "extended-tests")]

use assert_cmd::prelude::*;
use std::process::Command;

#[test]
fn env_policy_allows_web_search() {
    // Allow web_search via env JSON
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_search","when_contains_args":null,"action":"Allow","reason":"env-allow"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools","run","--name","web_search","--command","search","--arg","query=rust"])
        .env("CI","1")
        .env("MAGRAY_CMD_TIMEOUT","20")
        .env("MAGRAY_POLICY_JSON", policy);
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}

#[test]
fn env_policy_blocks_web_fetch_by_domain() {
    // Deny web_fetch for domain example.com using arg enrichment
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_fetch","when_contains_args":{"domain":"example.com"},"action":"Deny","reason":"blocked-domain"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args([
        "tools","run","--name","web_fetch","--command","fetch",
        "--arg","url=https://example.com/path?q=1"
    ])
    .env("CI","1")
    .env("MAGRAY_CMD_TIMEOUT","20")
    .env("MAGRAY_POLICY_JSON", policy);
    let status = cmd.status().expect("run ok");
    assert!(!status.success());
}

#[test]
fn env_policy_ask_with_autoapprove_succeeds() {
    // Ask for web_search; auto-approve via env
    let policy = r#"{"rules":[{"subject_kind":"Tool","subject_name":"web_search","when_contains_args":null,"action":"Ask","reason":"medium"}]}"#;
    let mut cmd = Command::cargo_bin("magray").expect("binary built");
    cmd.args(["tools","run","--name","web_search","--command","search","--arg","query=rust"])
        .env("CI","1")
        .env("MAGRAY_CMD_TIMEOUT","20")
        .env("MAGRAY_POLICY_JSON", policy)
        .env("MAGRAY_AUTO_APPROVE_ASK","true");
    let status = cmd.status().expect("run ok");
    assert!(status.success());
}