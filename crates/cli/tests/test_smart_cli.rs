use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
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
fn smart_runs_shell_and_creates_task_artifact() {
    let (mut cmd, home) = cmd_with_temp_home();

    // Задача, распознаваемая как shell_exec
    cmd.arg("smart").arg("выполни команду echo from-smart");

    let assert = cmd.assert().success();
    assert
        .stdout(predicate::str::contains("Smart планировщик"))
        .stdout(predicate::str::contains("Выполнение завершено"))
        .stdout(predicate::str::contains("from-smart"));

    // Проверим, что создались директории ~/.magray и ~/.magray/artifacts
    let magray_home = home.path().join(".magray");
    assert!(magray_home.exists());
    let artifacts_dir = magray_home.join("artifacts");
    assert!(artifacts_dir.exists());

    // Должен быть хотя бы один артефакт .txt
    let mut txt_found = false;
    if let Ok(entries) = fs::read_dir(&artifacts_dir) {
        for e in entries.flatten() {
            if let Some(ext) = e.path().extension() {
                if ext == "txt" {
                    txt_found = true;
                    let content = fs::read_to_string(e.path()).unwrap_or_default();
                    assert!(content.contains("from-smart"));
                    break;
                }
            }
        }
    }
    assert!(txt_found, "Ожидался текстовый артефакт шага");
}
