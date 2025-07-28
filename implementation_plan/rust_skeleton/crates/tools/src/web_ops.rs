use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct WebSearch;

impl WebSearch {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for WebSearch {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_string(),
            description: "Поиск информации в интернете".to_string(),
            usage: "web_search <запрос>".to_string(),
            examples: vec!["web_search 'Rust async best practices'".to_string()],
            input_schema: r#"{"query": "string"}"#.to_string(),
        }
    }
    
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        let query = _input.args.get("query").cloned().unwrap_or_default();
        if query.trim().is_empty() {
            return Ok(ToolOutput {
                success: false,
                result: "Пустой запрос для web_search".to_string(),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Используем DuckDuckGo Instant Answer API (не требует ключа)
        // Док: https://duckduckgo.com/api
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1",
            urlencoding::encode(&query)
        );

        let resp = reqwest::get(&url).await?;
        let status = resp.status();
        if !status.is_success() {
            return Ok(ToolOutput {
                success: false,
                result: format!("Ошибка HTTP {} при поиске", status),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Парсим json как Value чтобы не создавать отдельные структуры
        let json: serde_json::Value = resp.json().await?;

        // Пытаемся извлечь AbstractText или RelatedTopics
        let abstract_text = json["AbstractText"].as_str().unwrap_or("");
        let heading = json["Heading"].as_str().unwrap_or("");

        let mut result_text = String::new();
        if !heading.is_empty() {
            result_text.push_str(&format!("{}\n", heading));
        }
        if !abstract_text.is_empty() {
            result_text.push_str(abstract_text);
        } else {
            // Если нет абстракта, берем первые 3 RelatedTopics
            if let Some(arr) = json["RelatedTopics"].as_array() {
                for (idx, item) in arr.iter().take(3).enumerate() {
                    if let Some(text) = item["Text"].as_str() {
                        result_text.push_str(&format!("{}. {}\n", idx + 1, text));
                    }
                }
            }
        }

        if result_text.is_empty() {
            result_text = "Ничего не найдено".to_string();
        }

        Ok(ToolOutput {
            success: true,
            result: result_text.clone(),
            formatted_output: Some(result_text),
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        args.insert("query".to_string(), query.to_string());
        
        Ok(ToolInput {
            command: "web_search".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}