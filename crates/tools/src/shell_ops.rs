use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

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
        let command = input
            .args
            .get("command")
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

        // Dry-run: не выполняем, а возвращаем эхо с превью
        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] $ {}", command),
                formatted_output: Some(format!("$ {}\n[dry-run: no side effects]", command)),
                metadata: meta,
            });
        }

        // Выполняем команду с таймаутом если указан
        let run = async move {
            let output = if cfg!(target_os = "windows") {
                Command::new("cmd").args(&["/C", command]).output()
            } else {
                Command::new("sh").args(&["-c", command]).output()
            };
            output
        };

        let output_res = if let Some(ms) = input.timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(ms), run).await {
                Ok(res) => res,
                Err(_) => {
                    let mut meta = HashMap::new();
                    meta.insert("timeout_ms".into(), ms.to_string());
                    return Ok(ToolOutput {
                        success: false,
                        result: format!("Команда превысила таймаут {}ms", ms),
                        formatted_output: None,
                        metadata: meta,
                    });
                }
            }
        } else {
            run.await
        };

        match output_res {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let mut metadata = HashMap::new();
                metadata.insert(
                    "platform".to_string(),
                    if cfg!(target_os = "windows") { "windows".to_string() } else { "unix".to_string() },
                );
                metadata.insert(
                    "status_code".to_string(),
                    output.status.code().unwrap_or(-1).to_string(),
                );

                if output.status.success() {
                    Ok(ToolOutput {
                        success: true,
                        result: stdout.to_string(),
                        formatted_output: Some(format!("$ {}\n{}", command, stdout)),
                        metadata,
                    })
                } else {
                    Ok(ToolOutput {
                        success: false,
                        result: format!("Команда завершилась с ошибкой:\n{}", stderr),
                        formatted_output: Some(stdout.to_string()),
                        metadata,
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
        let command = query
            .replace("выполни команду", "")
            .replace("выполнить", "")
            .trim()
            .to_string();

        args.insert("command".to_string(), command);

        Ok(ToolInput {
            command: "shell_exec".to_string(),
            args,
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}
