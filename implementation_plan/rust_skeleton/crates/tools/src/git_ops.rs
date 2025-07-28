use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct GitStatus;

impl GitStatus {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for GitStatus {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "git_status".to_string(),
            description: "Показывает статус Git репозитория".to_string(),
            usage: "git_status".to_string(),
            examples: vec!["git status".to_string()],
            input_schema: r#"{}"#.to_string(),
        }
    }
    
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        Ok(ToolOutput {
            success: false,
            result: "Git инструменты будут реализованы позже".to_string(),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: "git_status".to_string(),
            args: HashMap::new(),
            context: Some(query.to_string()),
        })
    }
}

pub struct GitCommit;

impl GitCommit {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for GitCommit {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "git_commit".to_string(),
            description: "Создает Git коммит".to_string(),
            usage: "git_commit <сообщение>".to_string(),
            examples: vec!["git commit -m 'fix: исправлена ошибка'".to_string()],
            input_schema: r#"{"message": "string"}"#.to_string(),
        }
    }
    
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        Ok(ToolOutput {
            success: false,
            result: "Git инструменты будут реализованы позже".to_string(),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: "git_commit".to_string(),
            args: HashMap::new(),
            context: Some(query.to_string()),
        })
    }
}