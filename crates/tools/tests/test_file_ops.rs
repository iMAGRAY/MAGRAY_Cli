#![cfg(all(feature = "extended-tests", feature = "legacy-tests"))]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tools::file_ops::{DirLister, FileReader, FileWriter};
use tools::{Tool, ToolInput};

#[tokio::test]
async fn test_file_reader() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let test_content = "Hello, World!\nThis is a test file.\nLine 3";
    fs::write(&test_file, test_content).unwrap();

    let reader = FileReader::new();

    // Тест spec
    let spec = reader.spec();
    assert_eq!(spec.name, "file_read");
    assert!(!spec.description.is_empty());

    // Тест чтения файла
    let input = ToolInput {
        command: "file_read".to_string(),
        args: HashMap::from([("path".to_string(), test_file.to_str().unwrap().to_string())]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = reader.execute(input).await.unwrap();
    assert!(output.success);
    assert!(output.result.contains("Hello, World!"));
    assert!(output.result.contains("Line 3"));

    // Проверка форматированного вывода
    assert!(output.formatted_output.is_some());
    let formatted = output.formatted_output.unwrap();
    assert!(formatted.contains("📄"));
    assert!(formatted.contains("│ 1 │"));
}

#[tokio::test]
async fn test_file_reader_nonexistent() {
    let reader = FileReader::new();

    let input = ToolInput {
        command: "file_read".to_string(),
        args: HashMap::from([("path".to_string(), "/nonexistent/file.txt".to_string())]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = reader.execute(input).await;
    assert!(output.is_err());
}

#[tokio::test]
async fn test_file_reader_natural_language() {
    let reader = FileReader::new();

    // Тест парсинга natural language
    let queries = vec![
        "read file test.txt",
        "show me the contents of test.txt",
        "cat test.txt",
        "open test.txt",
    ];

    for query in queries {
        let input = reader.parse_natural_language(query).await.unwrap();
        assert_eq!(input.command, "file_read");
        assert!(input.args.contains_key("path"));
        assert!(input.args.get("path").unwrap().contains("test.txt"));
    }
}

#[tokio::test]
async fn test_file_writer() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("output.txt");

    let writer = FileWriter::new();

    // Тест spec
    let spec = writer.spec();
    assert_eq!(spec.name, "file_write");

    // Тест записи файла
    let input = ToolInput {
        command: "file_write".to_string(),
        args: HashMap::from([
            ("path".to_string(), test_file.to_str().unwrap().to_string()),
            ("content".to_string(), "Test content\nLine 2".to_string()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = writer.execute(input).await.unwrap();
    assert!(output.success);
    assert!(output.result.contains("байт"));

    // Проверяем что файл создан
    assert!(test_file.exists());
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Test content\nLine 2");
}

#[tokio::test]
async fn test_file_writer_overwrite() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("existing.txt");
    fs::write(&test_file, "Old content").unwrap();

    let writer = FileWriter::new();

    // Перезапись файла
    let input = ToolInput {
        command: "file_write".to_string(),
        args: HashMap::from([
            ("path".to_string(), test_file.to_str().unwrap().to_string()),
            ("content".to_string(), "New content".to_string()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = writer.execute(input).await.unwrap();
    assert!(output.success);

    // Проверяем что содержимое обновлено
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "New content");
}

#[tokio::test]
async fn test_file_writer_natural_language() {
    let writer = FileWriter::new();

    let queries = vec![
        "write 'Hello' to test.txt",
        "save 'Hello' to test.txt",
        "create file test.txt with content 'Hello'",
    ];

    for query in queries {
        let input = writer.parse_natural_language(query).await.unwrap();
        assert_eq!(input.command, "file_write");
        assert!(input.args.contains_key("path"));
        assert!(input.args.contains_key("content"));
        assert!(input.args.get("content").unwrap().contains("Hello"));
    }
}

#[tokio::test]
async fn test_dir_lister() {
    let temp_dir = TempDir::new().unwrap();

    // Создаем тестовую структуру
    fs::create_dir(temp_dir.path().join("subdir")).unwrap();
    fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
    fs::write(temp_dir.path().join("file2.rs"), "content2").unwrap();
    fs::write(temp_dir.path().join("subdir/file3.txt"), "content3").unwrap();

    let lister = DirLister::new();

    // Тест spec
    let spec = lister.spec();
    assert_eq!(spec.name, "dir_list");

    // Тест листинга директории
    let input = ToolInput {
        command: "dir_list".to_string(),
        args: HashMap::from([(
            "path".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        )]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = lister.execute(input).await.unwrap();
    assert!(output.success);
    assert!(output.result.contains("file1.txt"));
    assert!(output.result.contains("file2.rs"));
    assert!(output.result.contains("subdir"));

    // Проверка форматированного вывода
    assert!(output.formatted_output.is_some());
    let formatted = output.formatted_output.unwrap();
    assert!(formatted.contains("📁") || formatted.contains("📄"));
}

#[tokio::test]
async fn test_dir_lister_with_pattern() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("test1.txt"), "").unwrap();
    fs::write(temp_dir.path().join("test2.rs"), "").unwrap();
    fs::write(temp_dir.path().join("other.md"), "").unwrap();

    let lister = DirLister::new();

    // Тест с паттерном
    let input = ToolInput {
        command: "dir_list".to_string(),
        args: HashMap::from([
            (
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            ),
            ("pattern".to_string(), "test*".to_string()),
        ]),
        context: None,
        dry_run: false,
        timeout_ms: None,
    };

    let output = lister.execute(input).await.unwrap();
    assert!(output.success);
    assert!(output.result.contains("test1.txt"));
    assert!(output.result.contains("test2.rs"));
    assert!(!output.result.contains("other.md"));
}

#[tokio::test]
async fn test_dir_lister_natural_language() {
    let lister = DirLister::new();

    let queries = vec![
        "list files in /tmp",
        "show directory contents of /tmp",
        "ls /tmp",
        "dir /tmp",
    ];

    for query in queries {
        let input = lister.parse_natural_language(query).await.unwrap();
        assert_eq!(input.command, "dir_list");
        assert!(input.args.contains_key("path"));
        assert!(input.args.get("path").unwrap().contains("/tmp"));
    }
}

#[tokio::test]
async fn test_dir_lister_recursive() {
    let temp_dir = TempDir::new().unwrap();

    // Создаем вложенную структуру
    fs::create_dir_all(temp_dir.path().join("a/b/c")).unwrap();
    fs::write(temp_dir.path().join("root.txt"), "").unwrap();
    fs::write(temp_dir.path().join("a/file_a.txt"), "").unwrap();
    fs::write(temp_dir.path().join("a/b/file_b.txt"), "").unwrap();
    fs::write(temp_dir.path().join("a/b/c/file_c.txt"), "").unwrap();

    let lister = DirLister::new();

    // Тест рекурсивного листинга
    let input = ToolInput {
        command: "dir_list".to_string(),
        args: HashMap::from([
            (
                "path".to_string(),
                temp_dir.path().to_str().unwrap().to_string(),
            ),
            ("recursive".to_string(), "true".to_string()),
        ]),
        context: None,
    };

    let output = lister.execute(input).await.unwrap();
    assert!(output.success);
    assert!(output.result.contains("root.txt"));
    assert!(output.result.contains("file_a.txt"));
    assert!(output.result.contains("file_b.txt"));
    assert!(output.result.contains("file_c.txt"));
}

#[test]
fn test_file_ops_coverage() {
    // Дополнительный тест для увеличения покрытия
    let reader = FileReader::new();
    let writer = FileWriter::new();
    let lister = DirLister::new();

    // Проверяем что все инструменты поддерживают natural language
    assert!(reader.supports_natural_language());
    assert!(writer.supports_natural_language());
    assert!(lister.supports_natural_language());
}
