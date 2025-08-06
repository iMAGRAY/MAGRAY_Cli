use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct WebSearch;

impl WebSearch {
    pub fn new() -> Self {
        WebSearch
    }
}

impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for WebSearch {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description: "Поиск информации в интернете (mock)".to_string(),
            usage: "web_search <запрос>".to_string(),
            examples: vec![
                "web_search \"Rust программирование\"".to_string(),
                "найди информацию о машинном обучении".to_string(),
            ],
            input_schema: r#"{"query": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let query = input.args.get("query")
            .ok_or_else(|| anyhow::anyhow!("Отсутствует параметр 'query'"))?;

        // Mock implementation
        Ok(ToolOutput {
            success: true,
            result: format!("🔍 Поиск: '{}'\n\n[Mock результаты поиска]\n1. Результат 1\n2. Результат 2\n3. Результат 3", query),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем поисковый запрос
        let query_clean = query.replace("найди информацию о", "")
            .replace("найти ", "")
            .replace("поиск ", "")
            .trim()
            .to_string();
        
        args.insert("query".to_string(), query_clean);
        
        Ok(ToolInput {
            command: "web_search".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

pub struct WebFetch;

impl WebFetch {
    pub fn new() -> Self {
        WebFetch
    }
}

impl Default for WebFetch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for WebFetch {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_fetch".to_string(),
            description: "Загружает содержимое веб-страницы (mock)".to_string(),
            usage: "web_fetch <url>".to_string(),
            examples: vec![
                "web_fetch https://example.com".to_string(),
                "загрузи страницу rust-lang.org".to_string(),
            ],
            input_schema: r#"{"url": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let url = input.args.get("url")
            .ok_or_else(|| anyhow::anyhow!("Отсутствует параметр 'url'"))?;

        // Mock implementation
        Ok(ToolOutput {
            success: true,
            result: format!("📄 Содержимое страницы: {}\n\n[Mock содержимое]\n<html>\n  <body>\n    <h1>Заголовок страницы</h1>\n    <p>Текст страницы...</p>\n  </body>\n</html>", url),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем URL из запроса
        let words: Vec<&str> = query.split_whitespace().collect();
        for word in words {
            if word.starts_with("http://") || word.starts_with("https://") || word.contains(".com") || word.contains(".org") {
                args.insert("url".to_string(), word.to_string());
                break;
            }
        }
        
        // Если URL не найден, используем весь запрос
        if !args.contains_key("url") {
            args.insert("url".to_string(), query.to_string());
        }
        
        Ok(ToolInput {
            command: "web_fetch".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}