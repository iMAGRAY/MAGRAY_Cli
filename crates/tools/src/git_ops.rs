use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;

pub struct GitStatus;

impl Default for GitStatus {
    fn default() -> Self {
        Self::new()
    }
}

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
        use tokio::process::Command;

        // Выполняем `git status --short --branch` для компактного вывода
        let output = Command::new("git")
            .args(["status", "--short", "--branch"])
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(ToolOutput {
                success: true,
                result: stdout.clone(),
                formatted_output: Some(format!("\n📂 Текущий статус репозитория:\n{stdout}")),
                metadata: HashMap::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(ToolOutput {
                success: false,
                result: format!("Ошибка выполнения git status: {stderr}"),
                formatted_output: None,
                metadata: HashMap::new(),
            })
        }
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

impl Default for GitCommit {
    fn default() -> Self {
        Self::new()
    }
}

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
        use tokio::process::Command;

        let message = _input.args.get("message").cloned().unwrap_or_else(|| "commit via MAGRAY CLI".to_string());

        // Добавляем изменения
        let add_status = Command::new("git")
            .args(["add", "-A"])
            .output()
            .await?;

        if !add_status.status.success() {
            let err = String::from_utf8_lossy(&add_status.stderr).to_string();
            return Ok(ToolOutput {
                success: false,
                result: format!("Ошибка git add: {err}"),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Создаем коммит
        let commit_status = Command::new("git")
            .args(["commit", "-m", &message])
            .output()
            .await?;

        if commit_status.status.success() {
            let stdout = String::from_utf8_lossy(&commit_status.stdout).to_string();
            Ok(ToolOutput {
                success: true,
                result: stdout.clone(),
                formatted_output: Some(format!("\n✓ Создан коммит:\n{stdout}")),
                metadata: HashMap::from([("message".to_string(), message)]),
            })
        } else {
            let stderr = String::from_utf8_lossy(&commit_status.stderr).to_string();
            Ok(ToolOutput {
                success: false,
                result: format!("Ошибка git commit: {stderr}"),
                formatted_output: None,
                metadata: HashMap::new(),
            })
        }
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: "git_commit".to_string(),
            args: HashMap::new(),
            context: Some(query.to_string()),
        })
    }
}