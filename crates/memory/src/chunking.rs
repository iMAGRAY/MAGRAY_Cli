use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Типы чанков для разных видов контента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    /// Код с контекстом (функция, класс, модуль)
    Code {
        language: String,
        entity_type: CodeEntityType,
        file_path: String,
        line_start: usize,
        line_end: usize,
    },
    /// Документация (markdown, комментарии)
    Documentation {
        format: DocFormat,
        file_path: String,
        section: Option<String>,
    },
    /// Структурированные данные (JSON, TOML, YAML)
    StructuredData {
        format: String,
        schema: Option<String>,
        path: String,
    },
    /// Произвольный текст
    PlainText {
        source: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeEntityType {
    Function,
    Class,
    Module,
    Trait,
    Impl,
    Struct,
    Enum,
    Constant,
    Import,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocFormat {
    Markdown,
    RustDoc,
    JavaDoc,
    PlainText,
}

/// Чанк контента с метаданными
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    /// Уникальный идентификатор чанка
    pub id: String,
    /// Тип чанка
    pub chunk_type: ChunkType,
    /// Содержимое чанка
    pub content: String,
    /// Контекст (например, сигнатура функции для тела функции)
    pub context: Option<String>,
    /// Родительский чанк (для иерархии)
    pub parent_id: Option<String>,
    /// Связанные чанки (например, тесты для функции)
    pub related_ids: Vec<String>,
    /// Теги для фильтрации
    pub tags: Vec<String>,
    /// Размер в токенах (для LLM контекста)
    pub token_count: usize,
    /// Хеш содержимого для дедупликации
    pub content_hash: String,
}

/// Стратегия чанкинга
#[derive(Debug, Clone)]
pub struct ChunkingStrategy {
    /// Максимальный размер чанка в токенах
    pub max_tokens: usize,
    /// Минимальный размер чанка в токенах
    pub min_tokens: usize,
    /// Перекрытие между чанками (для контекста)
    pub overlap_tokens: usize,
    /// Включать ли контекст родителя
    pub include_parent_context: bool,
    /// Максимальная глубина для вложенных структур
    pub max_depth: usize,
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        Self {
            max_tokens: 512,      // Оптимально для эмбеддингов
            min_tokens: 50,       // Не создаем слишком мелкие чанки
            overlap_tokens: 50,   // Перекрытие для сохранения контекста
            include_parent_context: true,
            max_depth: 3,         // Класс -> Метод -> Вложенная функция
        }
    }
}

/// Чанкер для кода на Rust
pub struct RustCodeChunker {
    strategy: ChunkingStrategy,
}

impl RustCodeChunker {
    pub fn new(strategy: ChunkingStrategy) -> Self {
        Self { strategy }
    }

    /// Разбить Rust файл на семантические чанки
    pub async fn chunk_file(&self, file_path: &Path, content: &str) -> Result<Vec<ContentChunk>> {
        let mut chunks = Vec::new();
        
        // TODO: Использовать syn для парсинга AST
        // Пока простая реализация на основе строк
        
        let file_path_str = file_path.to_string_lossy().to_string();
        let lines: Vec<&str> = content.lines().collect();
        
        // Ищем функции
        let mut i = 0;
        while i < lines.len() {
            if let Some(func_chunk) = self.extract_function(&lines, i, &file_path_str)? {
                i = func_chunk.line_end;
                chunks.push(func_chunk.chunk);
            } else {
                i += 1;
            }
        }
        
        Ok(chunks)
    }

    /// Извлечь функцию как чанк
    fn extract_function(&self, lines: &[&str], start: usize, file_path: &str) -> Result<Option<FunctionChunk>> {
        // Простая эвристика для поиска функций
        let line = lines[start].trim();
        
        if line.starts_with("pub fn") || line.starts_with("fn") || 
           line.starts_with("pub async fn") || line.starts_with("async fn") {
            
            // Находим конец функции по балансу фигурных скобок
            let mut brace_count = 0;
            let mut end = start;
            let mut in_function = false;
            
            for (i, line) in lines[start..].iter().enumerate() {
                let idx = start + i;
                
                for ch in line.chars() {
                    match ch {
                        '{' => {
                            brace_count += 1;
                            in_function = true;
                        }
                        '}' => {
                            brace_count -= 1;
                            if brace_count == 0 && in_function {
                                end = idx;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                
                if brace_count == 0 && in_function {
                    break;
                }
            }
            
            if end > start {
                let content = lines[start..=end].join("\n");
                let signature = lines[start].to_string();
                
                let chunk = ContentChunk {
                    id: format!("{}:{}:{}", file_path, start + 1, end + 1),
                    chunk_type: ChunkType::Code {
                        language: "rust".to_string(),
                        entity_type: CodeEntityType::Function,
                        file_path: file_path.to_string(),
                        line_start: start + 1,
                        line_end: end + 1,
                    },
                    content: content.clone(),
                    context: Some(signature),
                    parent_id: None,
                    related_ids: vec![],
                    tags: vec!["rust".to_string(), "function".to_string()],
                    token_count: 0, // TODO: подсчитать реальное количество токенов
                    content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
                };
                
                return Ok(Some(FunctionChunk {
                    chunk,
                    line_end: end + 1,
                }));
            }
        }
        
        Ok(None)
    }
}

struct FunctionChunk {
    chunk: ContentChunk,
    line_end: usize,
}

/// Чанкер для Markdown документов
pub struct MarkdownChunker {
    strategy: ChunkingStrategy,
}

impl MarkdownChunker {
    pub fn new(strategy: ChunkingStrategy) -> Self {
        Self { strategy }
    }

    /// Разбить Markdown на семантические секции
    pub async fn chunk_document(&self, file_path: &Path, content: &str) -> Result<Vec<ContentChunk>> {
        let mut chunks = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Разбиваем по заголовкам
        let mut current_section = String::new();
        let mut current_heading = None;
        let mut section_start = 0;
        
        for (i, line) in content.lines().enumerate() {
            if line.starts_with('#') {
                // Сохраняем предыдущую секцию
                if !current_section.trim().is_empty() {
                    let chunk = ContentChunk {
                        id: format!("{}:{}", file_path_str, section_start),
                        chunk_type: ChunkType::Documentation {
                            format: DocFormat::Markdown,
                            file_path: file_path_str.clone(),
                            section: current_heading.clone(),
                        },
                        content: current_section.clone(),
                        context: current_heading.clone(),
                        parent_id: None,
                        related_ids: vec![],
                        tags: vec!["markdown".to_string(), "documentation".to_string()],
                        token_count: 0, // TODO: подсчитать
                        content_hash: blake3::hash(current_section.as_bytes()).to_hex().to_string(),
                    };
                    chunks.push(chunk);
                }
                
                // Начинаем новую секцию
                current_section = line.to_string() + "\n";
                current_heading = Some(line.trim_start_matches('#').trim().to_string());
                section_start = i;
            } else {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }
        
        // Сохраняем последнюю секцию
        if !current_section.trim().is_empty() {
            let chunk = ContentChunk {
                id: format!("{}:{}", file_path_str, section_start),
                chunk_type: ChunkType::Documentation {
                    format: DocFormat::Markdown,
                    file_path: file_path_str.clone(),
                    section: current_heading,
                },
                content: current_section.clone(),
                context: None,
                parent_id: None,
                related_ids: vec![],
                tags: vec!["markdown".to_string(), "documentation".to_string()],
                token_count: 0,
                content_hash: blake3::hash(current_section.as_bytes()).to_hex().to_string(),
            };
            chunks.push(chunk);
        }
        
        Ok(chunks)
    }
}

/// Универсальный чанкер
pub struct UniversalChunker {
    rust_chunker: RustCodeChunker,
    markdown_chunker: MarkdownChunker,
    strategy: ChunkingStrategy,
}

impl UniversalChunker {
    pub fn new(strategy: ChunkingStrategy) -> Self {
        let rust_chunker = RustCodeChunker::new(strategy.clone());
        let markdown_chunker = MarkdownChunker::new(strategy.clone());
        
        Self {
            rust_chunker,
            markdown_chunker,
            strategy,
        }
    }

    /// Разбить файл на чанки в зависимости от типа
    pub async fn chunk_file(&self, file_path: &Path) -> Result<Vec<ContentChunk>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        
        match file_path.extension().and_then(|e| e.to_str()) {
            Some("rs") => self.rust_chunker.chunk_file(file_path, &content).await,
            Some("md") => self.markdown_chunker.chunk_document(file_path, &content).await,
            _ => {
                // Для остальных файлов - простое разбиение по строкам
                self.chunk_plain_text(file_path, &content).await
            }
        }
    }

    /// Простое разбиение текста
    async fn chunk_plain_text(&self, file_path: &Path, content: &str) -> Result<Vec<ContentChunk>> {
        let mut chunks = Vec::new();
        let file_path_str = file_path.to_string_lossy().to_string();
        
        // Разбиваем по параграфам или фиксированному размеру
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        
        for (i, para) in paragraphs.iter().enumerate() {
            if !para.trim().is_empty() {
                let chunk = ContentChunk {
                    id: format!("{}:para:{}", file_path_str, i),
                    chunk_type: ChunkType::PlainText {
                        source: file_path_str.clone(),
                    },
                    content: para.to_string(),
                    context: None,
                    parent_id: None,
                    related_ids: vec![],
                    tags: vec!["plaintext".to_string()],
                    token_count: 0,
                    content_hash: blake3::hash(para.as_bytes()).to_hex().to_string(),
                };
                chunks.push(chunk);
            }
        }
        
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rust_chunking() {
        let chunker = RustCodeChunker::new(ChunkingStrategy::default());
        
        let code = r#"
use std::io;

fn main() {
    println!("Hello, world!");
}

pub async fn process_data(input: &str) -> Result<String> {
    let processed = input.to_uppercase();
    Ok(processed)
}
"#;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        
        let chunks = chunker.chunk_file(&file_path, code).await.unwrap();
        
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].content.contains("fn main"));
        assert!(chunks[1].content.contains("async fn process_data"));
    }

    #[tokio::test]
    async fn test_markdown_chunking() {
        let chunker = MarkdownChunker::new(ChunkingStrategy::default());
        
        let markdown = r#"
# Introduction

This is the introduction section.

## Overview

Some overview content here.

# Main Content

The main content goes here.
"#;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        
        let chunks = chunker.chunk_document(&file_path, markdown).await.unwrap();
        
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].content.contains("# Introduction"));
        assert!(chunks[1].content.contains("## Overview"));
        assert!(chunks[2].content.contains("# Main Content"));
    }
}