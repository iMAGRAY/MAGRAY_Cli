use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct ShellExec;

impl ShellExec {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for ShellExec {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "shell_exec".to_string(),
            description: "Выполняет команды в shell".to_string(),
            usage: "shell_exec <команда>".to_string(),
            examples: vec!["shell_exec 'ls -la'".to_string()],
            input_schema: r#"{"command": "string"}"#.to_string(),
        }
    }
    
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        Ok(ToolOutput {
            success: false,
            result: "Shell выполнение будет реализовано позже".to_string(),
            formatted_output: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        args.insert("command".to_string(), query.to_string());
        
        Ok(ToolInput {
            command: "shell_exec".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}