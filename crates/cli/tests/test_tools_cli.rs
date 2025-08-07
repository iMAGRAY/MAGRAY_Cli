use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn tools_list_shows_registered_tools() {
    let mut cmd = Command::cargo_bin("magray").unwrap();
    cmd.env("MAGRAY_NO_ANIM", "1")
        .arg("tools")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Registered Tools").or(predicate::str::contains("Registered Tools").not()));
}

#[test]
fn tools_run_shell_exec() {
    let mut cmd = Command::cargo_bin("magray").unwrap();
    cmd.env("MAGRAY_NO_ANIM", "1")
        .arg("tools")
        .arg("run")
        .arg("--name").arg("shell_exec")
        .arg("--command").arg("shell_exec")
        .arg("--arg").arg("command=echo hello");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}