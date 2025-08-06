use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

pub struct GitStatus;

impl GitStatus {
    pub fn new() -> Self {
        GitStatus
    }
}

impl Default for GitStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for GitStatus {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "git_status".to_string(),
            description: "Показывает статус git репозитория".to_string(),
            usage: "git_status".to_string(),
            examples: vec!["git_status".to_string()],
            input_schema: r#"{}"#.to_string(),
        }
    }

    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        let output = Command::new("git")
            .args(&["status", "--porcelain"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolOutput {
                success: true,
                result: stdout.to_string(),
                formatted_output: Some(format!("Git status:\n{}", stdout)),
                metadata: HashMap::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolOutput {
                success: false,
                result: format!("Git error: {}", stderr),
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

impl GitCommit {
    pub fn new() -> Self {
        GitCommit
    }
}

impl Default for GitCommit {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for GitCommit {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "git_commit".to_string(),
            description: "Создаёт коммит с указанным сообщением".to_string(),
            usage: "git_commit <сообщение>".to_string(),
            examples: vec![
                "git_commit \"Добавил новую функцию\"".to_string(),
                "создать коммит с сообщением \"исправил баг\"".to_string(),
            ],
            input_schema: r#"{"message": "string"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let message = input.args.get("message")
            .ok_or_else(|| anyhow::anyhow!("Отсутствует параметр 'message'"))?;

        // Сначала проверяем, есть ли что коммитить
        let status = Command::new("git")
            .args(&["status", "--porcelain"])
            .output()?;

        if status.status.success() && status.stdout.is_empty() {
            return Ok(ToolOutput {
                success: true,
                result: "Нет изменений для коммита".to_string(),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Добавляем все изменения
        let add = Command::new("git")
            .args(&["add", "."])
            .output()?;

        if !add.status.success() {
            let stderr = String::from_utf8_lossy(&add.stderr);
            return Ok(ToolOutput {
                success: false,
                result: format!("Ошибка git add: {}", stderr),
                formatted_output: None,
                metadata: HashMap::new(),
            });
        }

        // Создаем коммит
        let commit = Command::new("git")
            .args(&["commit", "-m", message])
            .output()?;

        if commit.status.success() {
            let stdout = String::from_utf8_lossy(&commit.stdout);
            Ok(ToolOutput {
                success: true,
                result: stdout.to_string(),
                formatted_output: Some(format!("✅ Создан коммит:\n{}", stdout)),
                metadata: HashMap::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&commit.stderr);
            Ok(ToolOutput {
                success: false,
                result: format!("Ошибка коммита: {}", stderr),
                formatted_output: None,
                metadata: HashMap::new(),
            })
        }
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        let mut args = HashMap::new();
        
        // Извлекаем сообщение коммита
        if let Some(start) = query.find('"') {
            if let Some(end) = query[start+1..].find('"') {
                let message = query[start+1..start+1+end].to_string();
                args.insert("message".to_string(), message);
            }
        } else {
            // Пытаемся использовать весь текст после ключевых слов
            let lower = query.to_lowercase();
            if let Some(pos) = lower.find("сообщением") {
                args.insert("message".to_string(), query[pos+11..].trim().to_string());
            } else {
                args.insert("message".to_string(), query.to_string());
            }
        }
        
        Ok(ToolInput {
            command: "git_commit".to_string(),
            args,
            context: Some(query.to_string()),
        })
    }
}

pub struct GitDiff;

impl GitDiff {
    pub fn new() -> Self {
        GitDiff
    }
}

impl Default for GitDiff {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Tool for GitDiff {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "git_diff".to_string(),
            description: "Показывает изменения в файлах".to_string(),
            usage: "git_diff".to_string(),
            examples: vec!["git_diff".to_string()],
            input_schema: r#"{}"#.to_string(),
        }
    }

    async fn execute(&self, _input: ToolInput) -> Result<ToolOutput> {
        let output = Command::new("git")
            .args(&["diff"])
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(ToolOutput {
                success: true,
                result: stdout.to_string(),
                formatted_output: Some(if stdout.is_empty() {
                    "Нет незакоммиченных изменений".to_string()
                } else {
                    format!("Git diff:\n{}", stdout)
                }),
                metadata: HashMap::new(),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(ToolOutput {
                success: false,
                result: format!("Git error: {}", stderr),
                formatted_output: None,
                metadata: HashMap::new(),
            })
        }
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        Ok(ToolInput {
            command: "git_diff".to_string(),
            args: HashMap::new(),
            context: Some(query.to_string()),
        })
    }
}