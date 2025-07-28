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
        Ok(ToolOutput {
            success: false,
            result: "Веб поиск будет реализован позже".to_string(),
            formatted_output: None,
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