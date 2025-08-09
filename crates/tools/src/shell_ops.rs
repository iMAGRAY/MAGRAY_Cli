use crate::{Tool, ToolInput, ToolOutput, ToolSpec};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::process::Command;
use tokio::io::{AsyncReadExt};

pub struct ShellExec;

impl ShellExec {
    pub fn new() -> Self { ShellExec }
}

impl Default for ShellExec { fn default() -> Self { Self::new() } }

#[async_trait::async_trait]
impl Tool for ShellExec {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "shell_exec".to_string(),
            description: "Выполняет shell команду (с таймаутом, лимитом вывода и sandbox env)".to_string(),
            usage: "shell_exec <команда> [cwd?]".to_string(),
            examples: vec![
                "shell_exec \"ls -la\"".to_string(),
                "выполни команду pwd".to_string(),
            ],
            input_schema: r#"{"command": "string", "cwd": "string?", "max_output_kb": "number?"}"#.to_string(),
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let command = input
            .args
            .get("command")
            .ok_or_else(|| anyhow!("Отсутствует параметр 'command'"))?
            .to_string();

        // Пустая команда
        if command.trim().is_empty() {
            return Ok(ToolOutput { success: false, result: "Пустая команда".to_string(), formatted_output: None, metadata: HashMap::new() });
        }

        // Dry-run preview
        if input.dry_run {
            let mut meta = HashMap::new();
            meta.insert("dry_run".into(), "true".into());
            if let Some(cwd) = input.args.get("cwd") { meta.insert("cwd".into(), cwd.clone()); }
            return Ok(ToolOutput {
                success: true,
                result: format!("[dry-run] $ {}", command),
                formatted_output: Some(format!("$ {}\n[dry-run: no side effects]", command)),
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

        // Build command
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", &command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", &command]);
            c
        };

        if let Some(ref dir) = cwd { cmd.current_dir(dir); }

        // Sanitize environment: start clean, allow only PATH
        cmd.env_clear();
        if let Ok(path) = std::env::var("PATH") { cmd.env("PATH", path); }

        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let start = std::time::Instant::now();
        let mut child = cmd.spawn()?;

        let mut stdout = child.stdout.take().ok_or_else(|| anyhow!("stdout pipe unavailable"))?;
        let mut stderr = child.stderr.take().ok_or_else(|| anyhow!("stderr pipe unavailable"))?;

        // Readers with caps
        let mut out_buf: Vec<u8> = Vec::with_capacity(8192);
        let mut err_buf: Vec<u8> = Vec::with_capacity(4096);

        let read_future = async {
            let mut tmp_out = [0u8; 4096];
            let mut tmp_err = [0u8; 4096];
            loop {
                let mut progress = false;
                tokio::select! {
                    read = stdout.read(&mut tmp_out) => {
                        let n = read?; if n>0 {
                            let take = n.min(max_bytes.saturating_sub(out_buf.len()));
                            if take>0 { out_buf.extend_from_slice(&tmp_out[..take]); }
                            progress = true;
                        }
                    }
                    read = stderr.read(&mut tmp_err) => {
                        let n = read?; if n>0 {
                            let take = n.min(max_bytes.saturating_sub(err_buf.len()));
                            if take>0 { err_buf.extend_from_slice(&tmp_err[..take]); }
                            progress = true;
                        }
                    }
                }
                if out_buf.len() >= max_bytes { break; }
                if !progress { break; }
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
                    if let Some(dir) = cwd { meta.insert("cwd".into(), dir); }
                    return Ok(ToolOutput { success: false, result: format!("Команда превысила таймаут {}ms", ms), formatted_output: None, metadata: meta });
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
                metadata.insert("platform".into(), if cfg!(target_os = "windows") { "windows".into() } else { "unix".into() });
                metadata.insert("status_code".into(), exit.code().unwrap_or(-1).to_string());
                metadata.insert("runtime_ms".into(), duration_ms.to_string());
                if let Some(dir) = cwd { metadata.insert("cwd".into(), dir); }
                if out_buf.len() >= max_bytes { metadata.insert("stdout_truncated".into(), "true".into()); }
                if err_buf.len() >= max_bytes { metadata.insert("stderr_truncated".into(), "true".into()); }
                metadata.insert("max_output_kb".into(), max_output_kb.to_string());

                if exit.success() {
                    Ok(ToolOutput { success: true, result: stdout_s.clone(), formatted_output: Some(format!("$ {}\n{}", command, stdout_s)), metadata })
                } else {
                    Ok(ToolOutput { success: false, result: format!("Команда завершилась с ошибкой:\n{}", stderr_s), formatted_output: Some(stdout_s), metadata })
                }
            }
            Err(e) => Ok(ToolOutput { success: false, result: format!("Не удалось выполнить команду: {}", e), formatted_output: None, metadata: HashMap::new() }),
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
        Ok(ToolInput { command: "shell_exec".to_string(), args, context: Some(query.to_string()), dry_run: false, timeout_ms: None })
    }
}
