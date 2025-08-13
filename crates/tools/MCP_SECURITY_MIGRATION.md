# MCP Tools Security Migration Guide

## 🚨 CRITICAL SECURITY VULNERABILITY FIXED

**Issue**: MCP tools were bypassing ALL sandbox security policies by returning `permissions: None` and `supports_dry_run: false` in their ToolSpec.

**Impact**: MCP tools could access ANY file, ANY network resource, and execute ANY shell command without restrictions.

**Fix**: MCP tools now use SECURE-BY-DEFAULT approach with explicit permissions and dry-run support.

## 🔒 Security Changes

### Before (VULNERABLE)
```rust
// VULNERABLE CODE - bypassed all security
impl Tool for McpTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            permissions: None,        // ❌ BYPASS: No restrictions
            supports_dry_run: false, // ❌ BYPASS: No safe testing
            // ...
        }
    }
}
```

### After (SECURE)
```rust
// SECURE CODE - explicit sandbox permissions
impl Tool for McpTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            permissions: Some(self.permissions.clone()), // ✅ SECURE: Explicit restrictions
            supports_dry_run: self.supports_dry_run,     // ✅ SECURE: Safe testing support
            // ...
        }
    }
}
```

## 🛡️ Migration Steps

### 1. Update MCP Tool Registration

#### OLD (Deprecated)
```rust
// DEPRECATED - still secure but lacks explicit control
registry.register_mcp_tool(
    "my_mcp_tool",
    "python".to_string(),
    vec!["mcp_server.py".to_string()],
    "file_processor".to_string(),
    "Process files via MCP".to_string(),
);
```

#### NEW (Recommended)
```rust
// SECURE - explicit permissions required
registry.register_mcp_tool_secure(
    "my_mcp_tool",
    "python".to_string(),
    vec!["mcp_server.py".to_string()],
    "file_processor".to_string(),
    "Process files via MCP".to_string(),
    vec!["/project/input".to_string()],    // fs_read_roots
    vec!["/project/output".to_string()],   // fs_write_roots  
    vec!["api.service.com".to_string()],   // net_allowlist
    false,                                 // allow_shell
    true,                                  // supports_dry_run
);
```

### 2. Use Builder Pattern for Complex Permissions

```rust
// Fine-grained permission control
let mcp_tool = registry.register_mcp_tool_builder(
    "complex_tool",
    "node".to_string(),
    vec!["server.js".to_string()],
    "data_processor".to_string(),
    "Complex data processing tool".to_string(),
)
.with_fs_read_access(vec![
    "/data/input".to_string(),
    "/config".to_string(),
])
.with_fs_write_access(vec![
    "/data/output".to_string(),
    "/logs".to_string(),
])
.with_network_access(vec![
    "api.external.com".to_string(),
    "cache.redis.local".to_string(),
])
.with_shell_access(false)  // HIGH RISK - avoid if possible
.with_dry_run_support(true);

registry.register_mcp_tool_from_builder("complex_tool", mcp_tool);
```

## 🧪 Testing Security Fixes

### Dry-Run Testing
```rust
let input = ToolInput {
    command: "process".to_string(),
    args: HashMap::from([("file".to_string(), "data.txt".to_string())]),
    context: None,
    dry_run: true,  // Safe testing - no actual execution
    timeout_ms: Some(5000),
};

let result = mcp_tool.execute(input).await?;
// Returns detailed dry-run information instead of executing
```

### Permission Verification
```rust
let spec = mcp_tool.spec();
assert!(spec.permissions.is_some(), "MCP tools must have explicit permissions");

let perms = spec.permissions.unwrap();
// Verify only intended permissions are granted
assert_eq!(perms.fs_read_roots, vec!["/safe/input"]);
assert!(perms.fs_write_roots.is_empty());  // No write access
assert!(!perms.allow_shell);               // No shell access
```

## 🔍 Security Testing

Run the security test suite to verify fixes:

```bash
# Run MCP security tests
cargo test test_mcp_security -p tools

# Run specific security scenarios
cargo test test_mcp_tool_secure_by_default -p tools
cargo test test_mcp_tool_dry_run_mode -p tools
cargo test test_mcp_tool_precheck_integration -p tools
```

## 📊 Security Impact

| Aspect | Before | After |
|--------|--------|-------|
| File System Access | ❌ Unrestricted | ✅ Explicit paths only |
| Network Access | ❌ Any domain | ✅ Allowlist only |
| Shell Access | ❌ Unrestricted | ✅ Explicit grant required |
| Dry-Run Support | ❌ Not supported | ✅ Enabled by default |
| Policy Integration | ❌ Bypassed | ✅ Enforced via precheck |
| Security Visibility | ❌ Hidden permissions | ✅ Explicit in spec |

## ⚠️ Breaking Changes

1. **MCP Tool Constructor**: Now creates secure-by-default tools with no permissions
2. **ToolSpec.permissions**: Changed from `None` to explicit `ToolPermissions`
3. **Dry-run support**: Now enabled by default (`supports_dry_run: true`)
4. **Tool descriptions**: Now include sandbox enforcement information
5. **Registration methods**: New secure methods require explicit permission grants

## 🔧 Configuration Examples

### Read-only MCP Tool
```rust
let readonly_tool = McpTool::new(cmd, args, tool, desc)
    .with_fs_read_access(vec!["/data/readonly".to_string()])
    .with_dry_run_support(true);
```

### Network-only MCP Tool  
```rust
let api_tool = McpTool::new(cmd, args, tool, desc)
    .with_network_access(vec!["api.service.com".to_string()])
    .with_dry_run_support(true);
```

### High-risk Shell Tool (use with caution)
```rust
let shell_tool = McpTool::new(cmd, args, tool, desc)
    .with_fs_read_access(vec!["/project".to_string()])
    .with_fs_write_access(vec!["/tmp/output".to_string()])
    .with_shell_access(true)  // ⚠️ HIGH RISK
    .with_dry_run_support(true);
```

## 🎯 Best Practices

1. **Principle of Least Privilege**: Grant only minimum required permissions
2. **Always Enable Dry-run**: Use `.with_dry_run_support(true)` for testing
3. **Explicit Permissions**: Use `register_mcp_tool_secure()` for clarity
4. **Avoid Shell Access**: Set `allow_shell: false` unless absolutely necessary
5. **Test Security**: Use dry-run mode to verify behavior before production
6. **Audit Permissions**: Regularly review granted MCP tool permissions

## 🚨 Security Checklist

- [ ] All MCP tools use explicit permissions (not `None`)
- [ ] Dry-run support is enabled where possible
- [ ] File system access is limited to required paths only
- [ ] Network access is restricted to known safe domains
- [ ] Shell access is disabled unless critically required
- [ ] Security tests pass: `cargo test test_mcp_security`
- [ ] Integration with `precheck_permissions()` verified
- [ ] Tool descriptions indicate sandbox enforcement

## 📞 Support

If you encounter issues with the security migration:

1. **Check Test Suite**: Run `cargo test test_mcp_security -p tools`
2. **Review Examples**: See `test_mcp_security.rs` for patterns
3. **Verify Integration**: Ensure `precheck_permissions()` integration works
4. **Security Audit**: Review all MCP tool registrations for explicit permissions

This migration ensures MCP tools integrate properly with the sandbox security system and cannot bypass security policies.