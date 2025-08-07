use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// New secure registry system
pub mod registry;

// New execution system with security and resource management
pub mod execution;

// Plugin system with WASM and external process support
pub mod plugins;

// Tool implementations
pub mod file_ops;
pub mod git_ops;
pub mod shell_ops;
pub mod web_ops;

// Advanced features (legacy - being replaced by execution module)
pub mod enhanced_tool_system;
pub mod execution_pipeline;
pub mod intelligent_selector;
pub mod performance_monitor;

// MCP integration
pub mod mcp;

// Базовые типы для системы инструментов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub command: String,
    pub args: HashMap<String, String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub result: String,
    pub formatted_output: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub examples: Vec<String>,
    pub input_schema: String,
}

// Трейт для всех инструментов
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput>;
    fn supports_natural_language(&self) -> bool {
        true
    }
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput>;
}

// Реестр инструментов
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // Регистрируем базовые инструменты
        registry.register("file_read", Box::new(file_ops::FileReader::new()));
        registry.register("file_write", Box::new(file_ops::FileWriter::new()));
        registry.register("dir_list", Box::new(file_ops::DirLister::new()));
        registry.register("git_status", Box::new(git_ops::GitStatus::new()));
        registry.register("git_commit", Box::new(git_ops::GitCommit::new()));
        registry.register("web_search", Box::new(web_ops::WebSearch::new()));
        registry.register("shell_exec", Box::new(shell_ops::ShellExec::new()));

        registry
    }

    pub fn register(&mut self, name: &str, tool: Box<dyn Tool>) {
        self.tools.insert(name.to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list_tools(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|tool| tool.spec()).collect()
    }

    /// Зарегистрировать MCP tool, проксирующий удалённый MCP сервер/процесс по stdio
    pub fn register_mcp_tool(
        &mut self,
        name: &str,
        cmd: String,
        args: Vec<String>,
        remote_tool: String,
        description: String,
    ) {
        let tool = mcp::McpTool::new(cmd, args, remote_tool, description);
        self.register(name, Box::new(tool));
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
