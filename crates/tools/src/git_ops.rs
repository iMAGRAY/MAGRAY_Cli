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
        let message = input
            .args
            .get("message")
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
        let add = Command::new("git").args(&["add", "."]).output()?;

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
            if let Some(end) = query[start + 1..].find('"') {
                let message = query[start + 1..start + 1 + end].to_string();
                args.insert("message".to_string(), message);
            }
        } else {
            // Пытаемся использовать весь текст после ключевых слов
            let lower = query.to_lowercase();
            if let Some(pos) = lower.find("сообщением") {
                args.insert("message".to_string(), query[pos + 11..].trim().to_string());
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
        let output = Command::new("git").args(&["diff"]).output()?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_creation() {
        let git_status = GitStatus::new();
        let spec = git_status.spec();

        assert_eq!(spec.name, "git_status");
        assert!(spec.description.contains("статус git"));
        assert_eq!(spec.usage, "git_status");
        assert!(!spec.examples.is_empty());
    }

    #[test]
    fn test_git_status_default() {
        let git_status1 = GitStatus::default();
        let git_status2 = GitStatus::new();

        assert_eq!(git_status1.spec().name, git_status2.spec().name);
    }

    #[tokio::test]
    async fn test_git_status_natural_language_parsing() -> Result<()> {
        let git_status = GitStatus::new();

        let input = git_status
            .parse_natural_language("показать статус git")
            .await?;
        assert_eq!(input.command, "git_status");
        assert!(input.args.is_empty());
        assert_eq!(input.context, Some("показать статус git".to_string()));

        Ok(())
    }

    #[test]
    fn test_git_commit_creation() {
        let git_commit = GitCommit::new();
        let spec = git_commit.spec();

        assert_eq!(spec.name, "git_commit");
        assert!(spec.description.contains("Создаёт коммит"));
        assert!(!spec.examples.is_empty());
    }

    #[test]
    fn test_git_commit_default() {
        let git_commit1 = GitCommit::default();
        let git_commit2 = GitCommit::new();

        assert_eq!(git_commit1.spec().name, git_commit2.spec().name);
    }

    #[tokio::test]
    async fn test_git_commit_missing_message() {
        let git_commit = GitCommit::new();
        let input_args = HashMap::new(); // Missing message

        let input = ToolInput {
            command: "git_commit".to_string(),
            args: input_args,
            context: None,
        };

        let result = git_commit.execute(input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_git_commit_natural_language_parsing() -> Result<()> {
        let git_commit = GitCommit::new();

        // Test valid format with quotes
        let input = git_commit
            .parse_natural_language("закоммитить changes with message \"Fix bug\"")
            .await?;
        assert_eq!(input.command, "git_commit");
        assert_eq!(input.args.get("message").unwrap(), "Fix bug");

        // Test format without quotes (should use entire text)
        let input = git_commit.parse_natural_language("просто commit").await?;
        assert_eq!(input.command, "git_commit");
        assert_eq!(input.args.get("message").unwrap(), "просто commit");

        Ok(())
    }

    #[test]
    fn test_git_diff_creation() {
        let git_diff = GitDiff::new();
        let spec = git_diff.spec();

        assert_eq!(spec.name, "git_diff");
        assert!(spec.description.contains("Показывает изменения"));
        assert!(!spec.examples.is_empty());
    }

    #[test]
    fn test_git_diff_default() {
        let git_diff1 = GitDiff::default();
        let git_diff2 = GitDiff::new();

        assert_eq!(git_diff1.spec().name, git_diff2.spec().name);
    }

    #[tokio::test]
    async fn test_git_diff_natural_language_parsing() -> Result<()> {
        let git_diff = GitDiff::new();

        let input = git_diff
            .parse_natural_language("показать различия в git")
            .await?;
        assert_eq!(input.command, "git_diff");
        assert!(input.args.is_empty());
        assert_eq!(input.context, Some("показать различия в git".to_string()));

        Ok(())
    }
}
