use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

// @component: {"k":"C","id":"shell_exec","t":"Shell command execution tool","m":{"cur":85,"tgt":95,"u":"%"},"f":["tools","shell","execution"]}
pub struct ShellExec;

impl ShellExec {
    pub fn new() -> Self {
        ShellExec
    }
}

impl Default for ShellExec {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for ShellExec {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "shell_exec".to_string(),
            description: "Выполняет shell команду".to_string(),
            usage: "shell_exec <команда>".to_string(),
            examples: vec![
                "shell_exec \"ls -la\"".to_string(),
                "выполни команду pwd".to_string(),
            ],
            input_schema: r#"{"command": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let command = input.args.get("command")
            .ok_or_else(|| anyhow::anyhow!("Отсутствует параметр 'command'"))?;

        // Парсим команду для выполнения
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(ToolOutput {
                success: false,
                result: "Пустая команда".to_string(),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        let (_cmd, _args) = parts.split_first().unwrap();

        // Выполняем команду
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", command])
                .output()
        } else {
            Command::new("sh")
                .args(&["-c", command])
                .output()
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    Ok(ToolOutput {
                        success: true,
                        result: stdout.to_string(),
                        formatted_output: None,
                        metadata: HashMap::new(),
                    })
                } else {
                    Ok(ToolOutput {
                        success: false,
                        result: format!("Команда завершилась с ошибкой:\n{}", stderr),
                        formatted_output: Some(stdout.to_string()),
                        metadata: HashMap::new(),
                    })
                }
            }
            Err(e) => Ok(ToolOutput {
                success: false,
                result: format!("Не удалось выполнить команду: {}", e),
                formatted_output: None,
                metadata: HashMap::new(),
            }),
        }
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Простое извлечение команды
        let command = query.replace("выполни команду", "")
            .replace("выполнить", "")
            .trim()
            .to_string();
        
        args.insert("command".to_string(), command);
        
        Ok(ToolInput {
            command: "shell_exec".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}