use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct WebSearch;

<<<<<<< HEAD
impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}

=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
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
<<<<<<< HEAD
                result: format!("Ошибка HTTP {status} при поиске"),
=======
                result: format!("Ошибка HTTP {} при поиске", status),
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
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
<<<<<<< HEAD
            result_text.push_str(&format!("{heading}\n"));
=======
            result_text.push_str(&format!("{}\n", heading));
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
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