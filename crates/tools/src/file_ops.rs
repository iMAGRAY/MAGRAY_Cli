use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

// FileReader - чтение файлов с простым форматированием
pub struct FileReader;

impl FileReader {
    pub fn new() -> Self {
        FileReader
    }
}

impl Default for FileReader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileReader {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_read".to_string(),
            description: "Читает содержимое файлов с красивой подсветкой синтаксиса".to_string(),
            usage: "file_read <путь>".to_string(),
            examples: vec![
                "file_read src/main.rs".to_string(),
                "file_read README.md".to_string(),
                "показать содержимое config.toml".to_string(),
            ],
            input_schema: r#"{"path": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input.args.get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;
        
        let content = fs::read_to_string(path)?;
        
        // Простое форматирование с заголовком
        let mut formatted = String::new();
        formatted.push_str(&format!("\n📄 Файл: {}\n", path));
        formatted.push_str(&"─".repeat(60));
        formatted.push('\n');
        formatted.push_str(&content);
        formatted.push('\n');
        formatted.push_str(&"─".repeat(60));
        formatted.push('\n');
        
        Ok(ToolOutput {
            success: true,
            result: content,
            formatted_output: Some(formatted),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем путь из запроса
        if let Some(path) = extract_path_from_query(query) {
            args.insert("path".to_string(), path);
        } else {
            args.insert("path".to_string(), query.to_string());
        }
        
        Ok(ToolInput {
            command: "file_read".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

// FileWriter - запись файлов
pub struct FileWriter;

impl FileWriter {
    pub fn new() -> Self {
        FileWriter
    }
}

impl Default for FileWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileWriter {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_write".to_string(),
            description: "Создаёт или перезаписывает файл с указанным содержимым".to_string(),
            usage: "file_write <путь> <содержимое>".to_string(),
            examples: vec![
                "file_write test.txt Hello World".to_string(),
                "создать файл config.json с содержимым {...}".to_string(),
            ],
            input_schema: r#"{"path": "string", "content": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input.args.get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;
        let content = input.args.get("content")
            .map(|s| s.as_str())
            .unwrap_or("");

        fs::write(path, content)?;
        
        Ok(ToolOutput {
            success: true,
            result: format!("✅ Файл '{}' успешно создан", path),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Простой парсинг для создания файла
        let parts: Vec<&str> = query.split(" с содержимым ").collect();
        if parts.len() == 2 {
            args.insert("path".to_string(), parts[0].trim().to_string());
            args.insert("content".to_string(), parts[1].trim().to_string());
        } else {
            return Err(anyhow!("Не удалось распарсить запрос на создание файла"));
        }
        
        Ok(ToolInput {
            command: "file_write".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

// DirLister - просмотр директорий
pub struct DirLister;

impl DirLister {
    pub fn new() -> Self {
        DirLister
    }
}

impl Default for DirLister {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for DirLister {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "dir_list".to_string(),
            description: "Показывает содержимое директории в виде красивого дерева".to_string(),
            usage: "dir_list <путь>".to_string(),
            examples: vec![
                "dir_list .".to_string(),
                "dir_list src/".to_string(),
                "показать содержимое папки".to_string(),
            ],
            input_schema: r#"{"path": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path = input.args.get("path")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'path'"))?;
        
        let path = Path::new(path);
        if !path.is_dir() {
            return Err(anyhow!("'{}' не является директорией", path.display()));
        }
        
        let mut output = String::new();
        output.push_str(&format!("\n📁 Директория: {}\n", path.display()));
        output.push_str(&"─".repeat(60));
        output.push('\n');
        
        // Собираем entries
        let mut entries: Vec<_> = fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .collect();
        
        // Сортируем: сначала директории, потом файлы
        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            b_is_dir.cmp(&a_is_dir).then_with(|| a.file_name().cmp(&b.file_name()))
        });
        
        for entry in entries {
            let entry_path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            
            if entry_path.is_dir() {
                output.push_str(&format!("📁 {}/\n", name_str));
            } else {
                let icon = "📄";
                let size = entry.metadata()
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|_| "?".to_string());
                output.push_str(&format!("{} {} ({})\n", icon, name_str, size));
            }
        }
        
        output.push_str(&"─".repeat(60));
        output.push('\n');
        
        Ok(ToolOutput {
            success: true,
            result: output.clone(),
            formatted_output: Some(output),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем путь из запроса
        if let Some(path) = extract_path_from_query(query) {
            args.insert("path".to_string(), path);
        } else {
            args.insert("path".to_string(), ".".to_string());
        }
        
        Ok(ToolInput {
            command: "dir_list".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

// FileSearcher - поиск файлов
pub struct FileSearcher;

impl FileSearcher {
    pub fn new() -> Self {
        FileSearcher
    }
}

impl Default for FileSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for FileSearcher {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_search".to_string(),
            description: "Ищет файлы по имени или расширению".to_string(),
            usage: "file_search <паттерн> [путь]".to_string(),
            examples: vec![
                "file_search *.rs".to_string(),
                "file_search main.rs src/".to_string(),
                "найти все файлы .toml".to_string(),
            ],
            input_schema: r#"{"pattern": "string", "path": "string?"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let pattern = input.args.get("pattern")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'pattern'"))?;
        let search_path = input.args.get("path")
            .map(|s| s.as_str())
            .unwrap_or(".");
        
        let mut results = Vec::new();
        let pattern_lower = pattern.to_lowercase();
        
        for entry in WalkDir::new(search_path)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            // Простой поиск по паттерну
            let matches = if pattern.contains('*') {
                // Простая поддержка wildcard
                let pattern_parts: Vec<&str> = pattern.split('*').collect();
                if pattern_parts.len() == 2 {
                    if pattern.starts_with('*') {
                        file_name.to_lowercase().ends_with(&pattern_parts[1].to_lowercase())
                    } else if pattern.ends_with('*') {
                        file_name.to_lowercase().starts_with(&pattern_parts[0].to_lowercase())
                    } else {
                        file_name.to_lowercase().starts_with(&pattern_parts[0].to_lowercase()) &&
                        file_name.to_lowercase().ends_with(&pattern_parts[1].to_lowercase())
                    }
                } else {
                    file_name.to_lowercase().contains(&pattern_lower)
                }
            } else {
                file_name.to_lowercase().contains(&pattern_lower)
            };
            
            if matches {
                results.push(path.display().to_string());
            }
        }
        
        let mut output = String::new();
        output.push_str(&format!("\n🔍 Поиск: {} в {}\n", pattern, search_path));
        output.push_str(&"─".repeat(60));
        output.push('\n');
        
        if results.is_empty() {
            output.push_str("Файлы не найдены\n");
        } else {
            output.push_str(&format!("Найдено {} файлов:\n", results.len()));
            for result in results.iter().take(100) {
                output.push_str(&format!("  📄 {}\n", result));
            }
            if results.len() > 100 {
                output.push_str(&format!("  ... и ещё {} файлов\n", results.len() - 100));
            }
        }
        
        output.push_str(&"─".repeat(60));
        output.push('\n');
        
        Ok(ToolOutput {
            success: true,
            result: output.clone(),
            formatted_output: Some(output),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем паттерн поиска
        if query.contains("файлы") || query.contains("файл") {
            // Пытаемся найти расширение файла
            if let Some(ext_start) = query.find('.') {
                let ext_end = query[ext_start..].find(' ').unwrap_or(query.len() - ext_start);
                let pattern = format!("*{}", &query[ext_start..ext_start + ext_end]);
                args.insert("pattern".to_string(), pattern);
            } else {
                args.insert("pattern".to_string(), "*".to_string());
            }
        } else {
            args.insert("pattern".to_string(), query.to_string());
        }
        
        Ok(ToolInput {
            command: "file_search".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

// Вспомогательная функция для форматирования размера файла
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

// Вспомогательная функция для извлечения пути из запроса
fn extract_path_from_query(query: &str) -> Option<String> {
    // Простой поиск путей в запросе
    let words: Vec<&str> = query.split_whitespace().collect();
    for word in words {
        if word.contains('/') || word.contains('\\') || word.ends_with(".rs") || word.ends_with(".md") || word.ends_with(".toml") {
            return Some(word.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_file_reader_creation() {
        let reader = FileReader::new();
        let spec = reader.spec();
        
        assert_eq!(spec.name, "file_read");
        assert!(spec.description.contains("Читает содержимое файлов"));
        assert!(!spec.examples.is_empty());
    }

    #[tokio::test]
    async fn test_file_reader_default() {
        let reader1 = FileReader::default();
        let reader2 = FileReader::new();
        
        assert_eq!(reader1.spec().name, reader2.spec().name);
    }

    #[tokio::test]
    async fn test_file_reader_nonexistent_file() {
        let reader = FileReader::new();
        let mut input_args = HashMap::new();
        input_args.insert("path".to_string(), "/definitely/nonexistent/file.txt".to_string());
        
        let input = ToolInput {
            command: "file_read".to_string(),
            args: input_args,
            context: None,
        };
        
        let result = reader.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_reader_existing_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_content = "Hello, World!";
        
        fs::write(&file_path, test_content).unwrap();
        
        let reader = FileReader::new();
        let mut input_args = HashMap::new();
        input_args.insert("path".to_string(), file_path.to_string_lossy().to_string());
        
        let input = ToolInput {
            command: "file_read".to_string(),
            args: input_args,
            context: None,
        };
        
        let result = reader.execute(input).await?;
        assert!(result.success);
        assert_eq!(result.result, test_content);
        assert!(result.formatted_output.is_some());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_reader_natural_language() -> Result<()> {
        let reader = FileReader::new();
        
        // Test with path in query
        let input = reader.parse_natural_language("показать содержимое src/main.rs").await?;
        assert_eq!(input.command, "file_read");
        assert_eq!(input.args.get("path").unwrap(), "src/main.rs");
        
        // Test without recognizable path
        let input = reader.parse_natural_language("показать файл").await?;
        assert_eq!(input.args.get("path").unwrap(), "показать файл");
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_writer_creation() {
        let writer = FileWriter::new();
        let spec = writer.spec();
        
        assert_eq!(spec.name, "file_write");
        assert!(spec.description.contains("Создаёт или перезаписывает файл"));
        assert!(!spec.examples.is_empty());
    }

    #[tokio::test]
    async fn test_file_writer_default() {
        let writer1 = FileWriter::default();
        let writer2 = FileWriter::new();
        
        assert_eq!(writer1.spec().name, writer2.spec().name);
    }

    #[tokio::test]
    async fn test_file_writer_write_file() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_output.txt");
        let test_content = "Test content";
        
        let writer = FileWriter::new();
        let mut input_args = HashMap::new();
        input_args.insert("path".to_string(), file_path.to_string_lossy().to_string());
        input_args.insert("content".to_string(), test_content.to_string());
        
        let input = ToolInput {
            command: "file_write".to_string(),
            args: input_args,
            context: None,
        };
        
        let result = writer.execute(input).await?;
        assert!(result.success);
        assert!(result.result.contains("успешно создан"));
        
        // Verify file was actually created
        let written_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written_content, test_content);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_file_writer_missing_path() {
        let writer = FileWriter::new();
        let input_args = HashMap::new(); // Missing path
        
        let input = ToolInput {
            command: "file_write".to_string(),
            args: input_args,
            context: None,
        };
        
        let result = writer.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_writer_natural_language() -> Result<()> {
        let writer = FileWriter::new();
        
        let input = writer.parse_natural_language("создать файл test.txt с содержимым Hello World").await?;
        assert_eq!(input.command, "file_write");
        assert_eq!(input.args.get("path").unwrap(), "создать файл test.txt");
        assert_eq!(input.args.get("content").unwrap(), "Hello World");
        
        // Test invalid format
        let result = writer.parse_natural_language("создать файл test.txt").await;
        assert!(result.is_err());
        
        Ok(())
    }

    #[test]
    fn test_extract_path_from_query() {
        // Test with .rs file
        let path = extract_path_from_query("показать содержимое src/main.rs");
        assert_eq!(path, Some("src/main.rs".to_string()));
        
        // Test with .md file
        let path = extract_path_from_query("прочитать README.md");
        assert_eq!(path, Some("README.md".to_string()));
        
        // Test with path containing slash
        let path = extract_path_from_query("открыть file/path.txt");
        assert_eq!(path, Some("file/path.txt".to_string()));
        
        // Test with backslash (Windows style)
        let path = extract_path_from_query("показать file\\path.txt");
        assert_eq!(path, Some("file\\path.txt".to_string()));
        
        // Test with .toml file
        let path = extract_path_from_query("config.toml file");
        assert_eq!(path, Some("config.toml".to_string()));
        
        // Test with no recognizable path
        let path = extract_path_from_query("показать файл");
        assert_eq!(path, None);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(100), "100 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2048 * 1024 * 1024), "2.0 GB");
    }
}