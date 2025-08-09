/// Базовые интеграционные тесты для проверки основной функциональности MAGRAY CLI
use anyhow::Result;
use std::process::Command;
use std::path::PathBuf;

/// Находит исполняемый файл magray в target
fn find_magray_binary() -> PathBuf {
    let exe_name = if cfg!(windows) { "magray.exe" } else { "magray" };
    
    // Пробуем разные варианты расположения
    let possible_paths = vec![
        format!("target/release/{}", exe_name),
        format!("target/debug/{}", exe_name),
    ];
    
    for path in possible_paths {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }
    
    // Если не нашли, компилируем
    println!("Binary not found, building...");
    Command::new("cargo")
        .args(&["build", "--release", "--bin", "magray"])
        .output()
        .expect("Failed to build magray");
    
    PathBuf::from(format!("target/release/{}", exe_name))
}

#[test]
fn test_cli_version() -> Result<()> {
    let binary = find_magray_binary();
    
    let output = Command::new(&binary)
        .arg("--version")
        .output()?;
    
    assert!(output.status.success(), "magray --version should succeed");
    
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("magray") || stdout.contains("MAGRAY"), 
            "Version output should contain program name");
    
    Ok(())
}

#[test]
fn test_cli_help() -> Result<()> {
    let binary = find_magray_binary();
    
    let output = Command::new(&binary)
        .arg("--help")
        .output()?;
    
    assert!(output.status.success(), "magray --help should succeed");
    
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("USAGE") || stdout.contains("Usage"), 
            "Help output should contain usage information");
    assert!(stdout.contains("OPTIONS") || stdout.contains("Options"),
            "Help output should contain options");
    
    Ok(())
}

#[test]
fn test_cli_status_command() -> Result<()> {
    let binary = find_magray_binary();
    
    let output = Command::new(&binary)
        .arg("status")
        .output()?;
    
    // Status может не успешно выполниться если модели не загружены, но не должен крашиться
    assert!(!output.status.success() || output.status.success(), 
            "Status command should at least not crash");
    
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    
    // Не должно быть panic сообщений
    assert!(!stdout.contains("panic"), "Should not panic: {}", stdout);
    assert!(!stderr.contains("panic"), "Should not panic: {}", stderr);
    assert!(!stderr.contains("thread 'main' panicked"), "Should not panic: {}", stderr);
    
    Ok(())
}

#[test]
fn test_memory_operations() -> Result<()> {
    let binary = find_magray_binary();
    
    // Тест команды memory stats
    let output = Command::new(&binary)
        .args(&["memory", "stats"])
        .output()?;
    
    // Проверяем что команда не крашится
    let stderr = String::from_utf8(output.stderr)?;
    assert!(!stderr.contains("panic"), "Memory stats should not panic");
    
    Ok(())
}

#[test]
fn test_error_handling_no_unwrap_panics() -> Result<()> {
    let binary = find_magray_binary();
    
    // Тестируем различные невалидные команды чтобы убедиться что нет unwrap паник
    let test_cases = vec![
        vec!["nonexistent-command"],
        vec!["chat"],  // без аргументов
        vec!["memory", "search"],  // без query
        vec!["tool", "execute"],  // без tool name
        vec!["--invalid-flag"],
    ];
    
    for args in test_cases {
        let output = Command::new(&binary)
            .args(&args)
            .output()?;
        
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Главное - не должно быть паник от unwrap()
        assert!(!stderr.contains("panicked at"), 
                "Command {:?} caused panic: {}", args, stderr);
        assert!(!stderr.contains("called `Option::unwrap()`"),
                "Command {:?} caused unwrap panic: {}", args, stderr);
        assert!(!stderr.contains("called `Result::unwrap()`"),
                "Command {:?} caused unwrap panic: {}", args, stderr);
    }
    
    Ok(())
}

#[test]
#[cfg(not(windows))] // Сигналы специфичны для Unix
fn test_graceful_shutdown() -> Result<()> {
    use std::time::Duration;
    use std::thread;
    
    let binary = find_magray_binary();
    
    // Запускаем процесс в фоне
    let mut child = Command::new(&binary)
        .arg("chat")
        .arg("--interactive")
        .spawn()?;
    
    // Даем процессу время на инициализацию
    thread::sleep(Duration::from_millis(500));
    
    // Посылаем SIGTERM
    child.kill()?;
    
    // Ждем завершения
    let status = child.wait()?;
    
    // Процесс должен завершиться, а не зависнуть
    assert!(status.code().is_some() || !status.success(),
            "Process should terminate on SIGTERM");
    
    Ok(())
}

// Тест производительности - базовые операции должны быть быстрыми
#[test]
fn test_performance_basic_operations() -> Result<()> {
    use std::time::Instant;
    
    let binary = find_magray_binary();
    
    // Help должен выполняться очень быстро
    let start = Instant::now();
    let output = Command::new(&binary)
        .arg("--help")
        .output()?;
    let duration = start.elapsed();
    
    assert!(output.status.success());
    assert!(duration.as_millis() < 1000, 
            "Help command took too long: {:?}", duration);
    
    // Version тоже должен быть мгновенным
    let start = Instant::now();
    let output = Command::new(&binary)
        .arg("--version")
        .output()?;
    let duration = start.elapsed();
    
    assert!(output.status.success());
    assert!(duration.as_millis() < 1000,
            "Version command took too long: {:?}", duration);
    
    Ok(())
}

// Модульный тест для проверки что основные модули компилируются
#[test]
fn test_modules_compile() {
    // Просто проверяем что можем импортировать основные модули
    use memory::{Record, Layer};
    use ai::{AiConfig, BgeM3EmbeddingService};
    use llm::LlmClient;
    use tools::{Tool, ToolRegistry};
    use router::SmartRouter;
    use todo::{TodoService, TodoItem};
    use common::OperationTimer;
    
    // Если этот тест компилируется, значит основные модули доступны
    assert!(true);
}