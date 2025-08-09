use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Tool, ToolInput, ToolOutput, ToolSpec};

#[derive(Debug, Clone)]
pub struct McpTool {
    cmd: String,
    args: Vec<String>,
    remote_tool: String,
    description: String,
}

impl McpTool {
    pub fn new(cmd: String, args: Vec<String>, remote_tool: String, description: String) -> Self {
        Self { cmd, args, remote_tool, description }
    }
}

#[derive(Debug, Serialize)]
struct McpRequest {
    tool: String,
    command: String,
    args: HashMap<String, String>,
    context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    success: bool,
    result: String,
    metadata: HashMap<String, String>,
}

#[async_trait::async_trait]
impl Tool for McpTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: format!("mcp:{}", self.remote_tool),
            description: self.description.clone(),
            usage: "Proxy tool to a MCP stdio server".to_string(),
            examples: vec![format!("mcp:{}: {{\"command\":\"run\", \"args\":{{}}}}", self.remote_tool)],
            input_schema: "{command: string, args: object, context?: string}".to_string(),
            usage_guide: None,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        let mut child = Command::new(&self.cmd)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Failed to start MCP process: {}", e))?;

        let mut stdin = tokio::process::ChildStdin::from_std(child.stdin.take().unwrap())?;
        let mut stdout = tokio::process::ChildStdout::from_std(child.stdout.take().unwrap())?;

        let req = McpRequest {
            tool: self.remote_tool.clone(),
            command: input.command.clone(),
            args: input.args.clone(),
            context: input.context.clone(),
        };
        let payload = serde_json::to_vec(&req)?;

        stdin.write_all(&payload).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        // Read one line JSON response
        let mut buf = Vec::new();
        tokio::time::timeout(Duration::from_secs(30), async {
            let mut tmp = [0u8; 4096];
            loop {
                let n = stdout.read(&mut tmp).await?;
                if n == 0 { break; }
                buf.extend_from_slice(&tmp[..n]);
                if buf.contains(&b'\n') { break; }
            }
            Ok::<(), anyhow::Error>(())
        })
        .await
        .map_err(|_| anyhow!("MCP response timeout"))??;

        let line = match buf.split(|&b| b == b'\n').next() {
            Some(slice) => slice,
            None => &buf,
        };

        let resp: McpResponse = serde_json::from_slice(line)
            .map_err(|e| anyhow!("Invalid MCP JSON response: {}", e))?;

        Ok(ToolOutput {
            success: resp.success,
            result: resp.result,
            formatted_output: None,
            metadata: resp.metadata,
        })
    }

    fn supports_natural_language(&self) -> bool { false }

    async fn parse_natural_language(&self, _query: &str) -> Result<ToolInput> {
        Err(anyhow!("Natural language parsing is not supported for MCP tools"))
    }
}