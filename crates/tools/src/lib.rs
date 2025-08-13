#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

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
pub mod intelligent_selector;
pub mod performance_monitor;

// MCP integration
pub mod mcp;

// Capability system (P1.2.3)
pub mod capabilities;

// Tool manifest validation (P1.2.2)
pub mod manifest;

// WASM runtime integration (P1.2.1.b)
#[cfg(feature = "wasm-runtime")]
pub mod wasm_runtime;

// WASM sandbox system (P1.2.4.a)
pub mod sandbox;

// Tool Context Builder system (P1.3.1)
pub mod context;

// Re-export WASM runtime types for public API
#[cfg(feature = "wasm-runtime")]
pub use wasm_runtime::{
    WasmExecutionResult, WasmInstance, WasmModule, WasmRuntime, WasmRuntimeConfig,
    WasmRuntimeError, WasmValue,
};

// Re-export sandbox types for public API
pub use sandbox::{
    resource_limits::ResourceLimits,
    sandbox_violations::{SandboxViolation, ViolationType},
    wasi_config::WasiSandboxConfig,
    wasm_sandbox::{SandboxedModule, WasmSandbox},
    SandboxConfig, SandboxError,
};

// Re-export context builder types for public API
pub use context::{
    ContextBuildingConfig, ExtractedMetadata, QwenToolReranker, RerankingRequest,
    RerankingResponse, ToolContext, ToolContextBuilder, ToolContextError, ToolMetadataExtractor,
    ToolRankingResult, ToolReranker, ToolSelectionRequest, ToolSelectionResponse,
};

// Базовые типы для системы инструментов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub command: String,
    pub args: HashMap<String, String>,
    pub context: Option<String>,
    pub dry_run: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub result: String,
    pub formatted_output: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageGuide {
    pub usage_title: String,
    pub usage_summary: String,
    pub preconditions: Vec<String>,
    pub arguments_brief: HashMap<String, String>,
    pub good_for: Vec<String>,
    pub not_for: Vec<String>,
    pub constraints: Vec<String>,
    pub examples: Vec<String>,
    pub platforms: Vec<String>,
    pub cost_class: String,
    pub latency_class: String,
    pub side_effects: Vec<String>,
    pub risk_score: u8,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolPermissions {
    pub fs_read_roots: Vec<String>,
    pub fs_write_roots: Vec<String>,
    pub net_allowlist: Vec<String>,
    pub allow_shell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub examples: Vec<String>,
    pub input_schema: String,
    pub usage_guide: Option<UsageGuide>,
    // Permissions and dry-run capability
    pub permissions: Option<ToolPermissions>,
    pub supports_dry_run: bool,
}

impl ToolSpec {
    pub fn with_usage_guide(mut self, guide: UsageGuide) -> Self {
        self.usage_guide = Some(guide);
        self
    }
}

pub fn generate_usage_guide(spec: &ToolSpec) -> UsageGuide {
    let mut args_brief = HashMap::new();
    if spec.input_schema.contains("url") {
        args_brief.insert("url".into(), "HTTP/HTTPS URL".into());
    }
    if spec.input_schema.contains("path") {
        args_brief.insert("path".into(), "File path".into());
    }
    if spec.input_schema.contains("cmd") {
        args_brief.insert("cmd".into(), "Shell command".into());
    }

    let examples = if !spec.examples.is_empty() {
        spec.examples.clone()
    } else {
        vec![format!("{} --help", spec.name)]
    };

    UsageGuide {
        usage_title: spec.name.clone(),
        usage_summary: spec.description.clone(),
        preconditions: vec![],
        arguments_brief: args_brief,
        good_for: vec!["general".into()],
        not_for: vec![],
        constraints: vec![],
        examples,
        platforms: vec!["linux".into(), "mac".into(), "win".into()],
        cost_class: "free".into(),
        latency_class: "fast".into(),
        side_effects: vec![],
        risk_score: 1,
        capabilities: vec![],
        tags: vec![],
    }
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
    #[allow(dead_code)]
    security_enforcer: Option<fn(&str, &ToolInput) -> bool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
            security_enforcer: None,
        };
        // Регистрируем базовые инструменты
        registry.register("file_read", Box::new(file_ops::FileReader::new()));
        registry.register("file_write", Box::new(file_ops::FileWriter::new()));
        registry.register("file_delete", Box::new(file_ops::FileDeleter::new()));
        registry.register("dir_list", Box::new(file_ops::DirLister::new()));
        registry.register("file_search", Box::new(file_ops::FileSearcher::new()));
        registry.register("git_status", Box::new(git_ops::GitStatus::new()));
        registry.register("git_commit", Box::new(git_ops::GitCommit::new()));
        registry.register("git_diff", Box::new(git_ops::GitDiff::new()));
        registry.register("web_search", Box::new(web_ops::WebSearch::new()));
        registry.register("web_fetch", Box::new(web_ops::WebFetch::new()));
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
        self.tools
            .values()
            .map(|tool| {
                let mut spec = tool.spec();
                if spec.usage_guide.is_none() {
                    spec.usage_guide = Some(generate_usage_guide(&spec));
                }
                spec
            })
            .collect()
    }

    /// DEPRECATED: Use register_mcp_tool_secure() instead for explicit security control
    #[deprecated(note = "Use register_mcp_tool_secure() for explicit sandbox permissions")]
    pub fn register_mcp_tool(
        &mut self,
        name: &str,
        cmd: String,
        args: Vec<String>,
        remote_tool: String,
        description: String,
    ) {
        // SECURE BY DEFAULT: No permissions granted
        let tool = mcp::McpTool::new(
            cmd,
            args,
            remote_tool,
            description,
            "unknown://localhost".to_string(),
        );
        self.register(name, Box::new(tool));
    }

    /// Register MCP tool with explicit sandbox permissions - SECURE BY DEFAULT
    /// This method requires explicit permission grants, preventing security bypasses
    #[allow(clippy::too_many_arguments)]
    pub fn register_mcp_tool_secure(
        &mut self,
        name: &str,
        cmd: String,
        args: Vec<String>,
        remote_tool: String,
        description: String,
        server_url: String,
        fs_read_roots: Vec<String>,
        fs_write_roots: Vec<String>,
        net_allowlist: Vec<String>,
        allow_shell: bool,
        supports_dry_run: bool,
    ) {
        let tool = mcp::McpTool::new(cmd, args, remote_tool, description, server_url)
            .with_fs_read_access(fs_read_roots)
            .with_fs_write_access(fs_write_roots)
            .with_network_access(net_allowlist)
            .with_shell_access(allow_shell)
            .with_dry_run_support(supports_dry_run);

        self.register(name, Box::new(tool));
    }

    /// Register MCP tool with builder pattern for fine-grained control
    /// Returns McpTool builder for chaining permission methods
    pub fn register_mcp_tool_builder(
        &mut self,
        _name: &str,
        cmd: String,
        args: Vec<String>,
        remote_tool: String,
        description: String,
        server_url: String,
    ) -> mcp::McpTool {
        mcp::McpTool::new(cmd, args, remote_tool, description, server_url)
    }

    /// Register pre-configured MCP tool from builder
    pub fn register_mcp_tool_from_builder(&mut self, name: &str, tool: mcp::McpTool) {
        self.register(name, Box::new(tool));
    }

    /// Опционально установить внешний проверяющий хук безопасности
    pub fn with_security_enforcer(mut self, f: fn(&str, &ToolInput) -> bool) -> Self {
        self.security_enforcer = Some(f);
        self
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
