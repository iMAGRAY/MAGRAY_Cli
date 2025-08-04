use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct ShellExec;

<<<<<<< HEAD
impl Default for ShellExec {
    fn default() -> Self {
        Self::new()
    }
}

=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
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
            description: "Выполняет команды в системной оболочке (cmd на Windows, sh на Unix)".to_string(),
            usage: "shell_exec <команда>".to_string(),
            examples: vec![
                "shell_exec 'mkdir new_folder'".to_string(),
                "shell_exec 'dir' (Windows) или 'ls' (Unix)".to_string(),
                "shell_exec 'echo Hello World'".to_string(),
            ],
            input_schema: r#"{"command": "string"}"#.to_string(),
        }
    }
    
    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        use tokio::process::Command;

        let cmd_str = _input.args.get("command")
            .cloned()
            .unwrap_or_default();

        if cmd_str.trim().is_empty() {
            return Ok(ToolOutput {
                success: false,
                result: "Не указана команда для shell_exec".to_string(),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Кроссплатформенное выполнение команд
        let output = if cfg!(target_os = "windows") {
            // Windows: используем cmd.exe /C
            Command::new("cmd")
                .args(["/C", &cmd_str])
                .output()
                .await?
        } else {
            // Unix-системы: используем /bin/sh -c (более универсально чем bash)
            Command::new("/bin/sh")
                .args(["-c", &cmd_str])
                .output()
                .await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut metadata = HashMap::new();
        metadata.insert("status_code".to_string(), output.status.code().unwrap_or(-1).to_string());
        metadata.insert("platform".to_string(), if cfg!(target_os = "windows") { "windows" } else { "unix" }.to_string());

        if output.status.success() {
            Ok(ToolOutput {
                success: true,
                result: stdout.clone(),
<<<<<<< HEAD
                formatted_output: Some(format!("$ {cmd_str}\n{stdout}")),
=======
                formatted_output: Some(format!("$ {}\n{}", cmd_str, stdout)),
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                metadata,
            })
        } else {
            Ok(ToolOutput {
                success: false,
<<<<<<< HEAD
                result: format!("Команда завершилась с ошибкой:\n{stderr}"),
=======
                result: format!("Команда завершилась с ошибкой:\n{}", stderr),
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                formatted_output: None,
                metadata,
            })
        }
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