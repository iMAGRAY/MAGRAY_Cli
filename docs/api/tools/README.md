# Tools Platform 2.0 - API Reference

## 📚 Overview

Tools Platform 2.0 предоставляет secure, extensible framework для выполнения инструментов с поддержкой WASM sandboxing, MCP integration, и intelligent tool selection с AI embeddings.

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────┐
│                Tool Registry                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Native    │  │    WASM     │  │     MCP     │  │
│  │    Tools    │  │   Modules   │  │   Servers   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│              Execution Pipeline                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │ Security    │  │ Sandbox     │  │ Resource    │  │
│  │ Validation  │  │ Execution   │  │ Management  │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│           Tool Context Builder                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │ AI Tool     │  │ Usage Guide │  │ Performance │  │
│  │ Selection   │  │ Generation  │  │ Analytics   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
└─────────────────────────────────────────────────────┘
```

## 📖 Core API Components

| Component | Purpose | Implementation Status |
|-----------|---------|----------------------|
| **ToolRegistry** | Central tool management | ✅ Production Ready |
| **ExecutionPipeline** | Secure tool execution | ✅ Production Ready |  
| **WasmRuntime** | WASM module execution | ⚠️  90% Complete |
| **SandboxSystem** | Resource/security limits | ⚠️  85% Complete |
| **McpIntegration** | MCP server connectivity | ⚠️  80% Complete |
| **CapabilitySystem** | Permission validation | ⚠️  90% Complete |
| **ContextBuilder** | AI-powered tool selection | ⚠️  75% Complete |

## 🔗 API Reference Documents

### Core Platform APIs
- [**registry-api.md**](registry-api.md) - Tool Registry management и discovery
- [**execution-api.md**](execution-api.md) - Execution Pipeline и security enforcement
- [**context-builder-api.md**](context-builder-api.md) - AI-powered tool selection и context

### Runtime & Sandboxing
- [**wasm-runtime-api.md**](wasm-runtime-api.md) - WASM module loading и execution
- [**sandbox-api.md**](sandbox-api.md) - Resource limits и security isolation
- [**capabilities-api.md**](capabilities-api.md) - Permission system и capability validation

### Integration APIs  
- [**mcp-api.md**](mcp-api.md) - MCP server integration и communication
- [**manifest-api.md**](manifest-api.md) - Tool manifest validation и schema

### Development Guides
- [**tool-development-guide.md**](../guides/tool-development.md) - Creating new tools
- [**security-configuration.md**](../guides/security-configuration.md) - Security setup
- [**integration-patterns.md**](../guides/integration-patterns.md) - Common usage patterns

## 🚀 Quick Start

### Basic Tool Registration

```rust
use tools::{ToolRegistry, ToolSpec, Tool, ToolInput, ToolOutput};

// Create registry with built-in tools
let mut registry = ToolRegistry::new();

// Register custom tool
registry.register("custom_analyzer", Box::new(CustomAnalyzer::new()));

// List available tools
let tools = registry.list_tools();
for tool_spec in tools {
    println!("Tool: {} - {}", tool_spec.name, tool_spec.description);
}
```

### Tool Execution с Security

```rust
use tools::{ToolRegistry, ToolInput};
use common::policy::{PolicyEngine, PolicySubjectKind};

let registry = ToolRegistry::new();
let policy_engine = PolicyEngine::new();

// Validate tool permission
let decision = policy_engine.evaluate(
    PolicySubjectKind::Tool,
    "file_write",
    &[("path", "/tmp/output.txt")]
);

if decision.allowed {
    let tool = registry.get("file_write").unwrap();
    let input = ToolInput {
        command: "write".to_string(),
        args: [("path", "/tmp/output.txt"), ("content", "Hello World")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        context: None,
        dry_run: false,
        timeout_ms: Some(30000),
    };
    
    let result = tool.execute(input).await?;
    println!("Result: {}", result.result);
}
```

### AI-Powered Tool Selection

```rust
use tools::{ToolContextBuilder, ToolSelectionConfig, PerformancePriority};

// Create context builder with real AI embeddings
let context_builder = ToolContextBuilder::new_with_real_embeddings(&registry).await?;

// Configure selection preferences
let config = ToolSelectionConfig {
    max_tools: 3,
    performance_priority: PerformancePriority::Speed,
    include_experimental: false,
    require_dry_run: true,
};

// Select best tools for task
let selection = context_builder
    .select_tools("analyze log files for errors")
    .with_config(config)
    .await?;

for selected_tool in selection.selected_tools {
    println!("Selected: {} (confidence: {:.2})", 
        selected_tool.spec.name,
        selected_tool.metadata.confidence_score
    );
}
```

### WASM Tool Loading

```rust
use tools::wasm_runtime::{WasmRuntime, WasmRuntimeConfig};
use tools::sandbox::{WasmSandbox, ResourceLimits, SandboxConfig};

// Configure WASM runtime with security
let runtime_config = WasmRuntimeConfig {
    memory_limit: 64 * 1024 * 1024, // 64MB
    execution_timeout: std::time::Duration::from_secs(30),
    enable_debugging: false,
};

let runtime = WasmRuntime::new(runtime_config).await?;

// Load and sandbox WASM module
let module_bytes = std::fs::read("tools/analyzer.wasm")?;
let module = runtime.load_module(&module_bytes).await?;

let sandbox_config = SandboxConfig {
    resource_limits: ResourceLimits {
        max_memory: 32 * 1024 * 1024,
        max_execution_time: std::time::Duration::from_secs(15),
        max_open_files: 10,
    },
    allowed_imports: vec!["env.log".to_string()],
    filesystem_access: vec!["/tmp".to_string()],
};

let sandbox = WasmSandbox::new(sandbox_config);
let instance = sandbox.instantiate(module).await?;

// Execute with safety guarantees
let result = instance.call("analyze", &["input.log"]).await?;
```

### MCP Server Integration

```rust
use tools::mcp::{McpClient, McpServerConfig, McpTool};

// Configure MCP server connection
let server_config = McpServerConfig {
    server_url: "http://localhost:8080".to_string(),
    connection_timeout: std::time::Duration::from_secs(10),
    heartbeat_interval: std::time::Duration::from_secs(30),
    max_retries: 3,
};

let client = McpClient::connect(server_config).await?;

// Discover remote tools
let remote_tools = client.list_tools().await?;
for tool in remote_tools {
    println!("Remote tool: {} - {}", tool.name, tool.description);
}

// Register MCP tool with explicit security
registry.register_mcp_tool_secure(
    "remote_analyzer",
    "analyze".to_string(),
    vec!["--format", "json"],
    "text_analyzer".to_string(),
    "Advanced text analysis tool".to_string(),
    "http://localhost:8080".to_string(),
    vec!["/tmp".to_string()],      // fs_read_roots
    vec![],                        // fs_write_roots  
    vec!["*.analysis.com".to_string()], // net_allowlist
    false,                         // allow_shell
    true,                          // supports_dry_run
);
```

## ⚙️ Configuration

### Environment Variables

```bash
# Tool Platform Configuration
MAGRAY_TOOL_TIMEOUT=60000          # Default tool timeout (ms)
MAGRAY_MAX_CONCURRENT_TOOLS=10     # Max concurrent executions
MAGRAY_TOOL_CACHE_SIZE=1000        # Tool result cache size

# WASM Runtime
MAGRAY_WASM_MEMORY_LIMIT=104857600 # 100MB default limit
MAGRAY_WASM_EXECUTION_TIMEOUT=30   # Execution timeout (seconds)
MAGRAY_WASM_DEBUG_MODE=false       # Enable debugging

# MCP Integration
MAGRAY_MCP_SERVER_TIMEOUT=15000    # MCP server timeout (ms)
MAGRAY_MCP_HEARTBEAT_INTERVAL=30000 # Heartbeat interval (ms)
MAGRAY_MCP_MAX_RETRIES=3           # Connection retry count

# Security & Sandboxing
MAGRAY_SANDBOX_ENABLED=true        # Enable sandbox by default
MAGRAY_SANDBOX_MEMORY_LIMIT=67108864 # 64MB sandbox limit
MAGRAY_AUDIT_TOOL_EXECUTION=true   # Audit all tool executions
```

### Feature Flags

```toml
[features]
default = ["native-tools", "security-enforcer"]
wasm-runtime = ["wasmtime"]
mcp-integration = ["tokio-tungstenite", "serde_json"]
ai-tool-selection = ["candle", "tokenizers"]
sandbox-isolation = ["resource-limits", "filesystem-jail"]
audit-logging = ["structured-logging", "event-bus"]
performance-monitoring = ["metrics", "profiling"]
```

## 🔒 Security Features

### Permission System
- **Explicit Capabilities**: Tools must declare required permissions
- **Principle of Least Privilege**: Minimal default permissions
- **Runtime Validation**: Permission checks on every execution
- **Audit Trail**: All permission grants/denials logged

### Sandboxing
- **Resource Limits**: Memory, CPU, file descriptor limits
- **Filesystem Isolation**: Read/write root restrictions
- **Network Controls**: Domain-based allowlists
- **Execution Timeouts**: Prevent runaway processes

### MCP Security
- **Server Authentication**: Certificate-based validation
- **Message Validation**: Schema enforcement for all messages
- **Rate Limiting**: Protection against DoS attacks
- **Capability Negotiation**: Explicit capability exchange

## 📊 Performance Characteristics

### Native Tools
- **Latency**: < 1ms execution overhead
- **Throughput**: 1000+ ops/sec per tool
- **Memory**: Minimal overhead (< 1MB per tool)
- **CPU**: Near-zero overhead when idle

### WASM Tools  
- **Startup**: 5-50ms module instantiation
- **Execution**: 10-100% overhead vs native
- **Memory**: Isolated heap (configurable)
- **Security**: Complete isolation from host

### MCP Tools
- **Latency**: Network + protocol overhead (10-100ms)
- **Reliability**: Auto-retry with exponential backoff
- **Discovery**: Cached tool metadata
- **Scalability**: Connection pooling support

## 🐛 Error Handling

### Tool Execution Errors
```rust
use tools::{ToolError, ToolExecutionError};

match tool.execute(input).await {
    Ok(output) => println!("Success: {}", output.result),
    Err(ToolError::Execution(exec_error)) => match exec_error {
        ToolExecutionError::Timeout => eprintln!("Tool timed out"),
        ToolExecutionError::PermissionDenied => eprintln!("Access denied"),
        ToolExecutionError::ResourceExhausted => eprintln!("Resource limit exceeded"),
        ToolExecutionError::InvalidInput(msg) => eprintln!("Invalid input: {}", msg),
        ToolExecutionError::ToolCrashed(msg) => eprintln!("Tool crashed: {}", msg),
    },
    Err(ToolError::Registry(reg_error)) => eprintln!("Registry error: {}", reg_error),
    Err(ToolError::Security(sec_error)) => eprintln!("Security error: {}", sec_error),
}
```

### Recovery Strategies
- **Automatic Retry**: Transient failures with exponential backoff
- **Graceful Degradation**: Fallback to alternative tools
- **Resource Recovery**: Cleanup after failures
- **User Notification**: Clear error messages with remediation steps

## 📈 Monitoring & Observability

### Metrics Collection
```rust
use tools::performance_monitor::{ToolMetrics, MetricsCollector};

let collector = MetricsCollector::new();

// Metrics are collected automatically during tool execution
let metrics = collector.get_tool_metrics("file_analyzer").await?;

println!("Tool Performance:");
println!("  Avg Latency: {:.2}ms", metrics.average_latency.as_millis());
println!("  Success Rate: {:.2}%", metrics.success_rate * 100.0);
println!("  Total Executions: {}", metrics.execution_count);
println!("  Error Rate: {:.2}%", metrics.error_rate * 100.0);
```

### Event Publishing
```rust
use common::event_bus::{EventBus, ToolEvent};

// Tool events are automatically published to EventBus
let event_bus = EventBus::new().await?;
event_bus.subscribe("tool.execution.*", |event: ToolEvent| {
    match event {
        ToolEvent::Started { tool_name, .. } => 
            info!("Tool {} started", tool_name),
        ToolEvent::Completed { tool_name, duration, .. } => 
            info!("Tool {} completed in {:?}", tool_name, duration),
        ToolEvent::Failed { tool_name, error, .. } => 
            warn!("Tool {} failed: {}", tool_name, error),
    }
}).await?;
```

## 🔧 Troubleshooting

### Common Issues

**Tool Not Found**
```
Error: ToolNotFound("my_custom_tool")
Solution: Check tool registration and spelling
```

**Permission Denied**
```
Error: PermissionDenied("filesystem write access required")  
Solution: Update tool manifest or grant explicit permissions
```

**WASM Module Load Failed**
```
Error: InvalidWasmModule("invalid magic number")
Solution: Verify WASM file integrity and compatibility
```

**MCP Connection Failed**
```
Error: McpConnectionFailed("connection refused")
Solution: Check server status and network configuration
```

### Debug Mode
```bash
# Enable verbose tool execution logging
export RUST_LOG=tools=debug
export MAGRAY_TOOL_DEBUG=true

# Run with debug output
magray-cli --tool my_tool --debug
```

## 📋 Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_execution() {
        let registry = ToolRegistry::new();
        let tool = registry.get("file_read").unwrap();
        
        let input = ToolInput {
            command: "read".to_string(),
            args: [("path", "test.txt")].iter().cloned().collect(),
            dry_run: true,
            ..Default::default()
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_end_to_end_tool_pipeline() {
    let registry = ToolRegistry::new();
    let context_builder = ToolContextBuilder::new(&registry).await?;
    
    // Test AI tool selection
    let selection = context_builder
        .select_tools("create and analyze log file")
        .await?;
        
    assert!(!selection.selected_tools.is_empty());
    
    // Test tool execution pipeline
    for selected_tool in selection.selected_tools {
        let result = selected_tool.execute(test_input()).await?;
        assert!(result.success);
    }
}
```

## 🔗 Related Documentation

- [Multi-Agent Integration](../agents/integration-guide.md) - Integration с agent system
- [Security Configuration](../guides/security-configuration.md) - Security best practices
- [Memory Integration](../memory/README.md) - Memory system integration
- [Policy Configuration](../security/policy-api.md) - Policy engine integration

---

**API Version**: 2.0  
**Implementation Status**: 88% Complete  
**Production Readiness**: Near Ready (pending security audit)  
**Next Milestone**: Full MCP integration и performance optimization