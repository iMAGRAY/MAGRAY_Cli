use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// FileReader - чтение файлов с простым форматированием
pub struct FileReader;

impl FileReader {
    pub fn new() -> Self {
        Self
    }
    
    fn format_file_content(&self, path: &Path, content: &str) -> String {
        // Простое форматирование без syntect для надежности
        let mut formatted = String::new();
        
        // Добавляем красивый заголовок
        formatted.push_str(&format!("┌─ {} {}\n", 
            "📄",
            path.display()
        ));
        
        // Простое форматирование с номерами строк
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        let line_width = line_count.to_string().len().max(3);
        
        for (i, line) in lines.iter().enumerate() {
            let line_num = format!("{:width$}", i + 1, width = line_width);
            formatted.push_str(&format!("│ {} │ {}\n", line_num, line));
        }
        
        formatted.push_str("└");
        for _ in 0..60 {
            formatted.push('─');
        }
        formatted.push('\n');
        
        formatted
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
        let path_str = input.args.get("path")
            .ok_or_else(|| anyhow!("Требуется параметр 'path'"))?;
        let path = PathBuf::from(path_str);
        
        if !path.exists() {
            return Ok(ToolOutput {
                success: false,
                result: format!("Файл не найден: {}", path.display()),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }
        
        if path.is_dir() {
            return Ok(ToolOutput {
                success: false,
                result: format!("Это директория, не файл: {}", path.display()),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }
        
        let content = fs::read_to_string(&path)
            .map_err(|e| anyhow!("Ошибка чтения файла: {}", e))?;
            
        let formatted = self.format_file_content(&path, &content);
        
        let mut metadata = HashMap::new();
        metadata.insert("file_size".to_string(), content.len().to_string());
        metadata.insert("line_count".to_string(), content.lines().count().to_string());
        
        Ok(ToolOutput {
            success: true,
            result: content,
            formatted_output: Some(formatted),
            metadata,
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        // Простой парсинг естественного языка
        let mut args = HashMap::new();
        
        // Ищем путь в запросе
        let words: Vec<&str> = query.split_whitespace().collect();
        
        // Ищем файл с расширением или путем
        for word in &words {
            if word.contains('.') || word.starts_with('/') || word.starts_with("./") || word.starts_with("src/") {
                args.insert("path".to_string(), word.to_string());
                break;
            }
        }
        
        // Если не нашли явный путь, берем последнее слово
        if args.is_empty() && !words.is_empty() {
            args.insert("path".to_string(), words[words.len() - 1].to_string());
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
        Self
    }
}

#[async_trait::async_trait]
impl Tool for FileWriter {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "file_write".to_string(),
            description: "Записывает содержимое в файл".to_string(),
            usage: "file_write <путь> <содержимое>".to_string(),
            examples: vec![
                "file_write README.md '# My Project'".to_string(),
                "создать файл config.toml с настройками".to_string(),
            ],
            input_schema: r#"{"path": "string", "content": "string"}"#.to_string(),
        }
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let path_str = input.args.get("path")
            .ok_or_else(|| anyhow!("Требуется параметр 'path'"))?;
        let content = input.args.get("content")
            .ok_or_else(|| anyhow!("Требуется параметр 'content'"))?;
            
        let path = PathBuf::from(path_str);
        
        // Создаем директории если нужно
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| anyhow!("Ошибка создания директории: {}", e))?;
        }
        
        fs::write(&path, content)
            .map_err(|e| anyhow!("Ошибка записи файла: {}", e))?;
            
        let formatted = format!("✓ Файл успешно записан: {}\n📄 Размер: {} байт",
            path.display(),
            content.len()
        );
        
        Ok(ToolOutput {
            success: true,
            result: format!("Файл записан: {}", path.display()),
            formatted_output: Some(formatted),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Улучшенный парсинг для создания файлов
        if query.contains("создай") || query.contains("create") {
            let words: Vec<&str> = query.split_whitespace().collect();
            
            // Ищем путь (файл с расширением или без)
            let mut found_path = None;
            for word in &words {
                if word.contains('.') {
                    found_path = Some(word.to_string());
                    break;
                }
            }
            
            // Если не нашли файл с расширением, ищем просто имя файла
            if found_path.is_none() {
                for (i, word) in words.iter().enumerate() {
                    if *word == "файл" || *word == "file" {
                        if i + 1 < words.len() {
                            let mut filename = words[i + 1].to_string();
                            // Добавляем расширение если его нет
                            if !filename.contains('.') {
                                filename.push_str(".txt");
                            }
                            found_path = Some(filename);
                            break;
                        }
                    }
                }
            }
            
            let file_path = found_path.unwrap_or_else(|| "new_file.txt".to_string());
            args.insert("path".to_string(), file_path.clone());
            
            // Ищем содержимое в запросе
            let content = if query.contains("с текстом") || query.contains("с содержимым") {
                // Извлекаем текст после "с текстом" или "с содержимым"
                let content_markers = ["с текстом", "с содержимым", "содержимым"];
                let mut content = String::new();
                
                for marker in &content_markers {
                    if let Some(pos) = query.find(marker) {
                        let after_marker = &query[pos + marker.len()..].trim();
                        // Убираем кавычки если есть
                        content = after_marker.trim_matches('"').trim_matches('\'').to_string();
                        break;
                    }
                }
                
                if content.is_empty() {
                    FileWriter::generate_default_content(&file_path)
                } else {
                    content
                }
            } else {
                FileWriter::generate_default_content(&file_path)
            };
            
            args.insert("content".to_string(), content);
        }
        
        Ok(ToolInput {
            command: "file_write".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

impl FileWriter {
    fn generate_default_content(file_path: &str) -> String {
        if file_path.ends_with(".rs") {
            "fn main() {\n    println!(\"Hello, world!\");\n}".to_string()
        } else if file_path.ends_with(".md") {
            let name = file_path.replace(".md", "");
            format!("# {}\n\nОписание проекта...\n\n## Использование\n\nИнструкции по использованию...\n", name)
        } else if file_path.ends_with(".toml") {
            "[settings]\nname = \"example\"\nversion = \"1.0.0\"\n".to_string()
        } else if file_path.ends_with(".json") {
            "{\n  \"name\": \"example\",\n  \"version\": \"1.0.0\"\n}".to_string()
        } else {
            format!("# Файл: {}\n\nСодержимое файла...\n", file_path)
        }
    }
}

// DirLister - просмотр директорий
pub struct DirLister;

impl DirLister {
    pub fn new() -> Self {
        Self
    }
    
    fn format_directory_tree(&self, path: &Path) -> Result<String> {
        let mut output = String::new();
        
        output.push_str(&format!("📁 {}\n", path.display()));
        
        let walker = WalkDir::new(path)
            .max_depth(3)
            .follow_links(false);
            
        for entry in walker {
            let entry = entry.map_err(|e| anyhow!("Ошибка обхода директории: {}", e))?;
            let entry_path = entry.path();
            let depth = entry.depth();
            
            if depth == 0 { continue; }
            
            let indent = "  ".repeat(depth);
            let name = entry_path.file_name()
                .unwrap_or_default()
                .to_string_lossy();
                
            if entry_path.is_dir() {
                output.push_str(&format!("{}📁 {}\n", indent, name));
            } else {
                let icon = match entry_path.extension().and_then(|s| s.to_str()) {
                    Some("rs") => "📄",
                    Some("toml") => "📄", 
                    Some("md") => "📄",
                    Some("json") => "📄",
                    Some("txt") => "📄",
                    _ => "📄",
                };
                
                output.push_str(&format!("{}{} {}\n", indent, icon, name));
            }
        }
        
        Ok(output)
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
        let default_path = ".".to_string();
        let path_str = input.args.get("path").unwrap_or(&default_path);
        let path = PathBuf::from(path_str);
        
        if !path.exists() {
            return Ok(ToolOutput {
                success: false,
                result: format!("Директория не найдена: {}", path.display()),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }
        
        if !path.is_dir() {
            return Ok(ToolOutput {
                success: false,
                result: format!("Это файл, не директория: {}", path.display()),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }
        
        let formatted = self.format_directory_tree(&path)?;
        
        Ok(ToolOutput {
            success: true,
            result: format!("Содержимое: {}", path.display()),
            formatted_output: Some(formatted),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Ищем путь в запросе
        let words: Vec<&str> = query.split_whitespace().collect();
        
        for word in &words {
            if word.ends_with('/') || *word == "." || *word == ".." || word.starts_with("./") {
                args.insert("path".to_string(), word.to_string());
                break;
            }
        }
        
        // По умолчанию текущая директория
        if args.is_empty() {
            args.insert("path".to_string(), ".".to_string());
        }
        
        Ok(ToolInput {
            command: "dir_list".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}