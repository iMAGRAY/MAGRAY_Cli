use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use common::policy::{get_policy_engine_with_eventbus, PolicyAction, SimpleToolPermissions};
use common::sandbox_config::SandboxConfig;
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
        "gunzip", "zip", "unzip", "ping", "netstat", "ss", "lsof", "tree", "diff", "patch", "git",
        "cargo", "rustc", "node", "npm", "python", "pip", "go", "javac", "java",
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
            description: "🔒 SECURITY: Executes shell commands with mandatory policy checks and sandbox restrictions. Requires explicit permission via policy configuration."
                .to_string(),
            usage: "shell_exec <команда> [cwd?] - REQUIRES SHELL POLICY PERMISSION".to_string(),
            examples: vec![
                "shell_exec \"ls -la\" --dry-run  # Preview command safely".to_string(),
                "выполни команду pwd (requires policy allow)".to_string(),
            ],
            input_schema: r#"{"command": "string", "cwd": "string?", "max_output_kb": "number?"}"#
                .to_string(),
            usage_guide: Some(crate::UsageGuide {
                usage_title: "🔒 SECURITY-PROTECTED SHELL EXECUTION".to_string(),
                usage_summary: "Executes shell commands with mandatory policy checks and sandbox restrictions. Requires explicit permission.".to_string(),
                preconditions: vec![
                    "Policy must explicitly allow shell_exec (default: DENIED)".to_string(),
                    "Sandbox config must set MAGRAY_SHELL_ALLOW=true".to_string(),
                    "Commands restricted to approved whitelist only".to_string(),
                ],
                arguments_brief: {
                    let mut args = HashMap::new();
                    args.insert("command".to_string(), "Shell command to execute (whitelist restricted)".to_string());
                    args.insert("cwd".to_string(), "Working directory (optional, validated)".to_string());
                    args.insert("max_output_kb".to_string(), "Maximum output size in KB (default: 256)".to_string());
                    args
                },
                good_for: vec![
                    "Running approved development tools (git, cargo, npm)".to_string(),
                    "File system operations (ls, find, grep)".to_string(),
                    "System information commands (ps, df, date)".to_string(),
                ],
                not_for: vec![
                    "System administration commands (sudo, su)".to_string(),
                    "Network services (nc, netcat, ssh servers)".to_string(),
                    "Dangerous operations (rm -rf, format, fdisk)".to_string(),
                    "Shell injection or command chaining".to_string(),
                ],
                constraints: vec![
                    format!("Whitelist: {:?}", Self::ALLOWED_COMMANDS),
                    "No dangerous patterns: ;, &&, ||, |, >, <, `, $, ()".to_string(),
                    "No path traversal: .. patterns blocked".to_string(),
                    "No system directories: /etc/, /root/, C:\\Windows\\".to_string(),
                ],
                examples: vec![
                    "shell_exec \"ls -la\" --dry-run".to_string(),
                    "shell_exec \"git status\"".to_string(),
                    "shell_exec \"cargo check\"".to_string(),
                ],
                platforms: vec!["Windows".to_string(), "Linux".to_string(), "macOS".to_string()],
                cost_class: "FREE".to_string(),
                latency_class: "MEDIUM".to_string(),
                side_effects: vec![
                    "Executes shell commands on host system".to_string(),
                    "May modify files or system state".to_string(),
                    "All executions logged for security audit".to_string(),
                ],
                risk_score: 8, // HIGH RISK: Shell execution
                capabilities: vec![
                    "shell_execution".to_string(),
                    "policy_enforcement".to_string(),
                    "sandbox_restrictions".to_string(),
                ],
                tags: vec![
                    "shell".to_string(),
                    "security".to_string(),
                    "policy".to_string(),
                    "restricted".to_string(),
                ],
            }),
            permissions: Some(crate::ToolPermissions {
                fs_read_roots: vec![],
                fs_write_roots: vec![],
                net_allowlist: vec![],
                allow_shell: true, // Mandatory: This tool requires shell access
            }),
            supports_dry_run: true,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let command = input
            .args
            .get("command")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'command'"))?
            .to_string();

        // SECURITY P0.1.5: MANDATORY POLICY ENGINE CHECK
        // Shell execution MUST be explicitly allowed - no bypassing!
        let policy_engine = get_policy_engine_with_eventbus();
        let mut args_for_policy = HashMap::new();
        args_for_policy.insert("command".to_string(), command.clone());
        if let Some(cwd) = input.args.get("cwd") {
            args_for_policy.insert("cwd".to_string(), cwd.clone());
        }

        let policy_decision = policy_engine.evaluate_tool("shell_exec", &args_for_policy);

        match policy_decision.action {
            PolicyAction::Deny => {
                return Ok(ToolOutput {
                    success: false,
                    result: format!(
                        "🔒 SECURITY POLICY VIOLATION: Shell execution denied by policy.\n\
                        Command: '{}'\n\
                        Reason: {:?}\n\
                        Risk Level: {:?}\n\
                        To enable shell execution, update your policy configuration.",
                        command,
                        policy_decision
                            .matched_rule
                            .as_ref()
                            .and_then(|r| r.reason.as_ref()),
                        policy_decision.risk
                    ),
                    formatted_output: None,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("policy_violation".into(), "shell_exec_denied".into());
                        meta.insert("command".into(), command.clone());
                        meta.insert("risk_level".into(), format!("{:?}", policy_decision.risk));
                        if let Some(rule) = policy_decision.matched_rule {
                            if let Some(reason) = rule.reason {
                                meta.insert("policy_reason".into(), reason);
                            }
                        }
                        meta
                    },
                });
            }
            PolicyAction::Ask => {
                // SECURITY: Require explicit confirmation before executing shell commands
                if !input.dry_run {
                    return Ok(ToolOutput {
                        success: false,
                        result: format!(
                            "🔒 SECURITY CONFIRMATION REQUIRED: Shell command requires explicit permission.\n\
                            Command: '{}'\n\
                            Risk Level: {:?}\n\
                            Use --dry-run to preview or update policy to allow shell execution.",
                            command,
                            policy_decision.risk
                        ),
                        formatted_output: Some(format!("$ {command}\n[REQUIRES PERMISSION]")),
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("policy_action".into(), "ask_permission".into());
                            meta.insert("command".into(), command.clone());
                            meta.insert("risk_level".into(), format!("{:?}", policy_decision.risk));
                            meta.insert("hint".into(), "Use --dry-run to preview command".into());
                            meta
                        },
                    });
                }
                // Continue with dry-run execution for preview
            }
            PolicyAction::Allow => {
                // Policy explicitly allows - continue with execution
            }
        }

        // SECURITY: Additional sandbox precheck
        let shell_perms = SimpleToolPermissions {
            fs_read_roots: vec![],
            fs_write_roots: vec![],
            net_allowlist: vec![],
            allow_shell: true, // This tool requires shell access
        };
        let sandbox_config = SandboxConfig::from_env();

        if let Some(precheck_decision) =
            common::policy::precheck_permissions("shell_exec", &shell_perms, &sandbox_config)
        {
            if !precheck_decision.allowed {
                return Ok(ToolOutput {
                    success: false,
                    result: format!(
                        "🔒 SANDBOX RESTRICTION: Shell execution blocked by sandbox configuration.\n\
                        Command: '{command}'\n\
                        Configure MAGRAY_SHELL_ALLOW=true to enable shell execution."
                    ),
                    formatted_output: None,
                    metadata: {
                        let mut meta = HashMap::new();
                        meta.insert("sandbox_violation".into(), "shell_not_allowed".into());
                        meta.insert("command".into(), command.clone());
                        meta.insert("hint".into(), "Set MAGRAY_SHELL_ALLOW=true".into());
                        meta
                    },
                });
            }
        }

        // SECURITY: Валидация команды на предмет безопасности
        let validated_parts = match self.validate_command(&command) {
            Ok(parts) => parts,
            Err(e) => {
                return Ok(ToolOutput {
                    success: false,
                    result: format!("🔒 SECURITY ERROR: {e}"),
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
                result: format!("[dry-run] $ {command}"),
                formatted_output: Some(format!("$ {command}\n[dry-run]")),
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
                        result: format!("🔒 SECURITY ERROR (CWD): {e}"),
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
                        result: format!("Команда превысила таймаут {ms}ms"),
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
                        result: format!("Команда завершилась с ошибкой:\n{stderr_s}"),
                        formatted_output: Some(stdout_s),
                        metadata,
                    })
                }
            }
            Err(e) => Ok(ToolOutput {
                success: false,
                result: format!("Не удалось выполнить команду: {e}"),
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
