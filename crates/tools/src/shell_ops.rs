use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub struct ShellExec;

impl ShellExec {
    /// Whitelist разрешенных базовых команд (SECURITY: предотвращает command injection)
    const ALLOWED_COMMANDS: &'static [&'static str] = &[
        "ls", "pwd", "echo", "cat", "head", "tail", "find", "grep", "awk", "sed", "sort", "uniq",
        "wc", "du", "df", "ps", "top", "date", "whoami", "id", "uname", "which", "whereis", "file",
        "stat", "chmod", "chown", "mkdir", "rmdir", "touch", "cp", "mv", "ln", "tar", "gzip",
        "gunzip", "zip", "unzip", "curl", "wget", "ping", "netstat", "ss", "lsof", "tree", "diff",
        "patch", "git", "cargo", "rustc", "node", "npm", "python", "pip", "go", "javac", "java",
    ];

    /// Опасные символы и паттерны (SECURITY: предотвращает command injection)
    const DANGEROUS_PATTERNS: &'static [&'static str] = &[
        ";",
        "&&",
        "||",
        "|",
        ">",
        ">>",
        "<",
        "`",
        "$",
        "(",
        ")",
        "{",
        "}",
        "rm -rf",
        "sudo",
        "su",
        "/etc/",
        "/root/",
        "/home/",
        "passwd",
        "shadow",
        "eval",
        "exec",
        "sh -c",
        "bash -c",
        "cmd.exe",
        "powershell",
        "nc",
        "netcat",
    ];

    pub fn new() -> Self {
        ShellExec
    }

    /// Валидация команды на предмет безопасности (SECURITY)
    fn validate_command(&self, command: &str) -> Result<Vec<String>> {
        let trimmed = command.trim();

        if trimmed.is_empty() {
            return Err(anyhow!("Пустая команда не допускается"));
        }

        // Проверка на опасные паттерны
        for &pattern in Self::DANGEROUS_PATTERNS {
            if trimmed.contains(pattern) {
                return Err(anyhow!(
                    "Команда содержит опасный паттерн: '{}'. Доступны только базовые команды.",
                    pattern
                ));
            }
        }

        // Парсинг аргументов (простая реализация для безопасности)
        let parts: Vec<String> = trimmed.split_whitespace().map(|s| s.to_string()).collect();

        if parts.is_empty() {
            return Err(anyhow!("Не удалось распарсить команду"));
        }

        let base_command = &parts[0];

        // Проверка whitelist базовых команд
        if !Self::ALLOWED_COMMANDS.contains(&base_command.as_str()) {
            return Err(anyhow!(
                "Команда '{}' не разрешена. Разрешенные команды: {:?}",
                base_command,
                Self::ALLOWED_COMMANDS
            ));
        }

        // Валидация аргументов
        for arg in &parts[1..] {
            // Проверка на path traversal в аргументах
            if arg.contains("..") {
                return Err(anyhow!(
                    "Аргумент содержит path traversal паттерн: '{}'",
                    arg
                ));
            }

            // Проверка на потенциально опасные символы в аргументах
            if arg.contains("$") || arg.contains("`") || arg.contains("'") {
                return Err(anyhow!(
                    "Аргумент содержит потенциально опасные символы: '{}'",
                    arg
                ));
            }
        }

        Ok(parts)
    }

    /// Валидация рабочей директории (SECURITY: предотвращает path traversal)
    fn validate_cwd(&self, cwd: &str) -> Result<PathBuf> {
        let path = PathBuf::from(cwd);

        // Проверка на absolute path (более безопасно)
        if !path.is_absolute() {
            return Err(anyhow!("Рабочая директория должна быть абсолютным путем"));
        }

        // Нормализация пути для предотвращения path traversal
        let canonical = path
            .canonicalize()
            .map_err(|e| anyhow!("Не удалось нормализовать путь '{}': {}", cwd, e))?;

        // Проверка что путь существует и это директория
        if !canonical.is_dir() {
            return Err(anyhow!("Путь '{}' не является директорией", cwd));
        }

        Ok(canonical)
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
            description: "Выполняет shell команду (с таймаутом, лимитом вывода и sandbox env)"
                .to_string(),
            usage: "shell_exec <команда> [cwd?]".to_string(),
            examples: vec![
                "shell_exec \"ls -la\"".to_string(),
                "выполни команду pwd".to_string(),
            ],
            input_schema: r#"{"command": "string", "cwd": "string?", "max_output_kb": "number?"}"#
                .to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: true,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let command = input
            .args
            .get("command")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'command'"))?
            .to_string();

        // SECURITY: Валидация команды на предмет безопасности
        let validated_parts = match self.validate_command(&command) {
            Ok(parts) => parts,
            Err(e) => {
                return Ok(ToolOutput {
                    success: false,
                    result: format!("🔒 SECURITY ERROR: {}", e),
                    formatted_output: None,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("security_error".into(), "true".into());
                        meta.insert("original_command".into(), command);
                        meta
                    },
                });
            }
        };

        // Dry-run preview
        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            if let Some(cwd) = input.args.get("cwd") {
                meta.insert("cwd".into(), cwd.clone());
            }
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] $ {}", command),
                formatted_output: Some(format!("$ {}\n[dry-run]", command)),
                metadata: meta,
            });
        }

        let cwd = input.args.get("cwd").cloned();
        let max_output_kb: usize = input
            .args
            .get("max_output_kb")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(256);
        let max_bytes = max_output_kb.saturating_mul(1024);

        // SECURITY: Валидация рабочей директории если указана
        let validated_cwd = if let Some(ref cwd_str) = cwd {
            match self.validate_cwd(cwd_str) {
                Ok(path) => Some(path),
                Err(e) => {
                    return Ok(ToolOutput {
                        success: false,
                        result: format!("🔒 SECURITY ERROR (CWD): {}", e),
                        formatted_output: None,
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("security_error".into(), "true".into());
                            meta.insert("invalid_cwd".into(), cwd_str.clone());
                            meta
                        },
                    });
                }
            }
        } else {
            None
        };

        // SECURITY: Безопасное построение команды - используем прямое выполнение вместо shell
        let mut cmd = Command::new(&validated_parts[0]);
        if validated_parts.len() > 1 {
            cmd.args(&validated_parts[1..]);
        }

        // Установка рабочей директории если валидирована
        if let Some(ref validated_dir) = validated_cwd {
            cmd.current_dir(validated_dir);
        }

        // Sanitize environment: start clean, allow only PATH
        cmd.env_clear();
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }

        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let start = std::time::Instant::now();
        let mut child = cmd.spawn()?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("stdout pipe unavailable"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| anyhow!("stderr pipe unavailable"))?;

        // Readers with caps
        let mut out_buf: Vec<u8> = Vec::with_capacity(8192);
        let mut err_buf: Vec<u8> = Vec::with_capacity(4096);

        let read_future = async {
            let mut tmp_out = [0u8; 4096];
            let mut tmp_err = [0u8; 4096];
            let mut out_closed = false;
            let mut err_closed = false;
            let mut cap_out = false;
            let mut cap_err = false;
            while !(out_closed && err_closed) {
                tokio::select! {
                    read = stdout.read(&mut tmp_out), if !out_closed => {
                        let n = read?;
                        if n == 0 { out_closed = true; } else if !cap_out {
                            let take = n.min(max_bytes.saturating_sub(out_buf.len()));
                            if take>0 { out_buf.extend_from_slice(&tmp_out[..take]); }
                            if out_buf.len() >= max_bytes { cap_out = true; }
                        }
                    }
                    read = stderr.read(&mut tmp_err), if !err_closed => {
                        let n = read?;
                        if n == 0 { err_closed = true; } else if !cap_err {
                            let take = n.min(max_bytes.saturating_sub(err_buf.len()));
                            if take>0 { err_buf.extend_from_slice(&tmp_err[..take]); }
                            if err_buf.len() >= max_bytes { cap_err = true; }
                        }
                    }
                }
            }
            Ok::<(), std::io::Error>(())
        };

        let child_wait = async {
            let _ = read_future.await; // best-effort
            child.wait().await
        };

        let status = if let Some(ms) = input.timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(ms), child_wait).await {
                Ok(res) => res,
                Err(_) => {
                    // Kill on timeout
                    let _ = child.kill().await;
                    let mut meta = HashMap::new();
                    meta.insert("timeout_ms".into(), ms.to_string());
                    meta.insert("runtime_ms".into(), start.elapsed().as_millis().to_string());
                    if let Some(dir) = cwd {
                        meta.insert("cwd".into(), dir);
                    }
                    return Ok(ToolOutput {
                        success: false,
                        result: format!("Команда превысила таймаут {}ms", ms),
                        formatted_output: None,
                        metadata: meta,
                    });
                }
            }
        } else {
            child_wait.await
        };

        let duration_ms = start.elapsed().as_millis();

        match status {
            Ok(exit) => {
                let stdout_s = String::from_utf8_lossy(&out_buf).to_string();
                let stderr_s = String::from_utf8_lossy(&err_buf).to_string();
                let mut metadata = HashMap::new();
                metadata.insert(
                    "platform".into(),
                    if cfg!(target_os = "windows") {
                        "windows".into()
                    } else {
                        "unix".into()
                    },
                );
                metadata.insert("status_code".into(), exit.code().unwrap_or(-1).to_string());
                metadata.insert("runtime_ms".into(), duration_ms.to_string());
                if let Some(dir) = cwd {
                    metadata.insert("cwd".into(), dir);
                }
                if out_buf.len() >= max_bytes {
                    metadata.insert("stdout_truncated".into(), "true".into());
                }
                if err_buf.len() >= max_bytes {
                    metadata.insert("stderr_truncated".into(), "true".into());
                }
                metadata.insert("max_output_kb".into(), max_output_kb.to_string());

                if exit.success() {
                    Ok(ToolOutput {
                        success: true,
                        result: stdout_s.clone(),
                        formatted_output: Some(format!(
                            "$ {}\n{}",
                            validated_parts.join(" "),
                            stdout_s
                        )),
                        metadata,
                    })
                } else {
                    Ok(ToolOutput {
                        success: false,
                        result: format!("Команда завершилась с ошибкой:\n{}", stderr_s),
                        formatted_output: Some(stdout_s),
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
