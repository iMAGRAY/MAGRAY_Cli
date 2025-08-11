use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

fn cmd_with_temp_home() -> (Command, TempDir) {
    let temp_home = TempDir::new().expect("temp home");
    let mut cmd = Command::cargo_bin("magray").unwrap();
    cmd.env("MAGRAY_NO_ANIM", "1");
    cmd.env("HOME", temp_home.path());
    (cmd, temp_home)
}

#[test]
fn tasks_stats_empty_db() {
    let (mut cmd, _home) = cmd_with_temp_home();

    cmd.arg("tasks").arg("stats");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Статистика задач"))
        .stdout(predicate::str::contains("total: 0"));
}

#[test]
fn tasks_list_default_limit() {
    let (mut cmd, _home) = cmd_with_temp_home();

    cmd.arg("tasks").arg("list").arg("--limit").arg("5");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Готовые задачи:"));
}
