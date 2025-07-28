use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use console::style;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use walkdir::WalkDir;

// FileReader - чтение файлов с подсветкой синтаксиса
pub struct FileReader {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl FileReader {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
    
    fn format_file_content(&self, path: &Path, content: &str) -> String {
        let syntax = self.syntax_set
            .find_syntax_for_file(path)
            .unwrap_or_else(|_| Some(self.syntax_set.find_syntax_plain_text()))
            .unwrap();
            
        let theme = self.theme_set.themes.get("base16-ocean.dark")
            .or_else(|| self.theme_set.themes.values().next())
            .unwrap();
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        let mut formatted = String::new();
        
        // Добавляем красивый заголовок
        formatted.push_str(&format!("{}┌─ {} {}\n", 
            style("").dim(),
            style("📄").cyan(),
            style(path.display()).bright().bold()
        ));
        
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        let line_width = line_count.to_string().len().max(3);
        
        for (i, line) in LinesWithEndings::from(content).enumerate() {
            let line_num = format!("{:width$}", i + 1, width = line_width);
            
            if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
                let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
                formatted.push_str(&format!("{}│ {} │ {}", 
                    style("").dim(),
                    style(line_num).dim(),
                    escaped
                ));
            } else {
                formatted.push_str(&format!("{}│ {} │ {}", 
                    style("").dim(),
                    style(line_num).dim(),
                    line
                ));
            }
        }
        
        formatted.push_str(&format!("{}└{}\n",
            style("").dim(),
            style("─".repeat(60)).dim()
        ));
        
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
            
        let formatted = format!("{}✓ Файл успешно записан: {}\n{}📄 Размер: {} байт",
            style("").green(),
            style(path.display()).bright(),
            style("").dim(),
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
        
        // Простой парсинг для создания файлов
        if query.contains("создай") || query.contains("create") {
            let words: Vec<&str> = query.split_whitespace().collect();
            
            // Ищем путь
            for word in &words {
                if word.contains('.') {
                    args.insert("path".to_string(), word.to_string());
                    break;
                }
            }
            
            // Базовое содержимое
            let file_name = args.get("path").unwrap_or(&"new_file.txt".to_string()).clone();
            let content = if file_name.ends_with(".md") {
                format!("# {}\n\nОписание проекта...\n", file_name.replace(".md", ""))
            } else if file_name.ends_with(".toml") {
                "[settings]\nkey = \"value\"\n".to_string()
            } else {
                "# Новый файл\n".to_string()
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

// DirLister - просмотр директорий
pub struct DirLister;

impl DirLister {
    pub fn new() -> Self {
        Self
    }
    
    fn format_directory_tree(&self, path: &Path) -> Result<String> {
        let mut output = String::new();
        
        output.push_str(&format!("{}📁 {}\n", 
            style("").cyan(), 
            style(path.display()).bright().bold()
        ));
        
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
                output.push_str(&format!("{}{}📁 {}\n", 
                    style("").dim(), 
                    indent, 
                    style(name).blue()
                ));
            } else {
                let icon = match entry_path.extension().and_then(|s| s.to_str()) {
                    Some("rs") => "🦀",
                    Some("toml") => "⚙️",
                    Some("md") => "📝",
                    Some("json") => "📋",
                    Some("txt") => "📄",
                    _ => "📄",
                };
                
                output.push_str(&format!("{}{}{} {}\n", 
                    style("").dim(), 
                    indent, 
                    icon,
                    style(name).white()
                ));
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