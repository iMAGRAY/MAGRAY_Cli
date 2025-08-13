/// MCP Security Tests - Critical P0 Security Vulnerability Fix Verification
///
/// This test suite verifies that the MCP tools security bypass vulnerability is fixed.
/// All tests MUST pass to ensure sandbox security is enforced.
use serial_test::serial;
use std::collections::HashMap;
use tools::{mcp::McpTool, Tool, ToolInput};

#[tokio::test]
async fn test_mcp_tool_secure_by_default() {
    // SECURITY TEST: MCP tools should have NO permissions by default
    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "test.server.com".to_string(),
    );

    let spec = mcp_tool.spec();

    // CRITICAL: permissions must NOT be None anymore
    assert!(
        spec.permissions.is_some(),
        "MCP tool permissions must be explicit, not None"
    );

    let perms = spec.permissions.expect("Test operation should succeed");

    // SECURE BY DEFAULT: No file system access
    assert!(
        perms.fs_read_roots.is_empty(),
        "MCP tool should have no FS read access by default"
    );
    assert!(
        perms.fs_write_roots.is_empty(),
        "MCP tool should have no FS write access by default"
    );

    // SECURE BY DEFAULT: No network access
    assert!(
        perms.net_allowlist.is_empty(),
        "MCP tool should have no network access by default"
    );

    // SECURE BY DEFAULT: No shell access
    assert!(
        !perms.allow_shell,
        "MCP tool should not have shell access by default"
    );

    // SECURE BY DEFAULT: Supports dry-run
    assert!(
        spec.supports_dry_run,
        "MCP tool should support dry-run by default for security testing"
    );
}

// ============================================================================
// CRITICAL P0.2.4: MCP SERVER WHITELIST/BLACKLIST TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_mcp_server_whitelist_success() {
    // SERVER FILTERING TEST: Whitelisted servers should be allowed
    // Clean up environment variables first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");

    std::env::set_var(
        "MAGRAY_MCP_SERVER_WHITELIST",
        "trusted.example.com,safe-mcp.org",
    );

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("whitelist_test_tool");
    tokio::fs::write(&mock_binary, b"whitelist test binary")
        .await
        .expect("Test operation should succeed");

    let mcp_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "whitelist_test_tool".to_string(),
        "Tool for whitelist testing".to_string(),
        "trusted.example.com".to_string(), // Whitelisted server
    )
    .with_signature_requirement(false); // Disable signature requirement for this test

    // Server validation should succeed
    let result = mcp_tool.validate_server();
    assert!(
        result.is_ok(),
        "Whitelisted server should pass validation: {:?}",
        result.err()
    );

    // Tool spec should show server information
    let spec = mcp_tool.spec();
    assert!(
        spec.description.contains("server=trusted.example.com"),
        "Description should show server URL: {}",
        spec.description
    );

    assert!(
        spec.usage.contains("SERVER=trusted.example.com"),
        "Usage should show server URL: {}",
        spec.usage
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

#[tokio::test]
#[serial]
async fn test_mcp_server_blacklist_blocking() {
    // SERVER FILTERING TEST: Blacklisted servers should ALWAYS be blocked
    // Clean up environment variables first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");

    std::env::set_var(
        "MAGRAY_MCP_SERVER_WHITELIST",
        "trusted.example.com,malicious.badsite.com",
    );
    std::env::set_var(
        "MAGRAY_MCP_SERVER_BLACKLIST",
        "malicious.badsite.com,evil.hacker.net",
    );

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("blacklist_test_tool");
    tokio::fs::write(&mock_binary, b"blacklist test binary")
        .await
        .expect("Test operation should succeed");

    let blacklisted_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "blacklisted_tool".to_string(),
        "Tool connecting to blacklisted server".to_string(),
        "malicious.badsite.com".to_string(), // Blacklisted server (even though whitelisted)
    )
    .with_signature_requirement(false); // Disable signature requirement for this test

    // Server validation should fail due to blacklist (higher priority)
    let result = blacklisted_tool.validate_server();
    assert!(
        result.is_err(),
        "Blacklisted server should fail validation even if whitelisted"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("BLACKLISTED") || error_msg.contains("blocked for security"),
        "Error should indicate blacklisted server: {error_msg}"
    );

    // Execution should be blocked
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: false,
        timeout_ms: Some(1000),
    };

    let exec_result = blacklisted_tool.execute(input).await;
    assert!(
        exec_result.is_err(),
        "Execution should be blocked for blacklisted server"
    );

    let exec_error = exec_result.unwrap_err().to_string();
    assert!(
        exec_error.contains("SECURITY BLOCK") && exec_error.contains("server validation failure"),
        "Execution error should indicate server validation failure: {exec_error}"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");
}

#[tokio::test]
#[serial]
async fn test_mcp_server_empty_whitelist_blocks_all() {
    // SERVER FILTERING TEST: Empty whitelist should block ALL servers (secure by default)
    // Clean up any environment variables first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("empty_whitelist_tool");
    tokio::fs::write(&mock_binary, b"empty whitelist test")
        .await
        .expect("Test operation should succeed");

    let blocked_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "blocked_by_default".to_string(),
        "Tool blocked by empty whitelist".to_string(),
        "any.server.com".to_string(), // Any server should be blocked
    )
    .with_signature_requirement(false); // Disable signature requirement for this test

    // Server validation should fail due to empty whitelist
    let result = blocked_tool.validate_server();
    assert!(
        result.is_err(),
        "Empty whitelist should block all servers by default"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("No MCP servers whitelisted")
            || error_msg.contains("default security policy"),
        "Error should indicate no servers whitelisted: {error_msg}"
    );
}

#[tokio::test]
#[serial]
async fn test_mcp_server_not_in_whitelist_blocked() {
    // SERVER FILTERING TEST: Servers not in whitelist should be blocked
    // Clean up environment variables first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");

    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "trusted1.com,trusted2.org");

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("not_whitelisted_tool");
    tokio::fs::write(&mock_binary, b"not whitelisted test")
        .await
        .expect("Test operation should succeed");

    let not_whitelisted_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "not_whitelisted".to_string(),
        "Tool connecting to non-whitelisted server".to_string(),
        "untrusted.random.com".to_string(), // Not in whitelist
    )
    .with_signature_requirement(false); // Disable signature requirement for this test

    // Server validation should fail
    let result = not_whitelisted_tool.validate_server();
    assert!(result.is_err(), "Non-whitelisted server should be blocked");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not in whitelist") && error_msg.contains("untrusted.random.com"),
        "Error should indicate server not in whitelist: {error_msg}"
    );

    assert!(
        error_msg.contains("trusted1.com") && error_msg.contains("trusted2.org"),
        "Error should show allowed servers list: {error_msg}"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

#[tokio::test]
#[serial]
async fn test_mcp_server_dry_run_includes_server_info() {
    // SERVER FILTERING TEST: Dry-run output should include server information
    // Clean up environment variables first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::remove_var("MAGRAY_MCP_SERVER_BLACKLIST");

    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "test-server.example.com");

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("dry_run_server_tool");
    tokio::fs::write(&mock_binary, b"dry run server test")
        .await
        .expect("Test operation should succeed");

    let server_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "server_dry_run_test".to_string(),
        "Tool for server dry-run testing".to_string(),
        "test-server.example.com".to_string(),
    )
    .with_signature_requirement(false); // Disable signature requirement for this test

    // Dry-run execution should include server information
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: true,
        timeout_ms: Some(1000),
    };

    let result = server_tool.execute(input).await;
    match &result {
        Ok(_) => println!("✅ Dry-run succeeded"),
        Err(e) => println!("❌ Dry-run failed: {e}"),
    }
    assert!(
        result.is_ok(),
        "Dry-run should succeed for whitelisted server: {:?}",
        result.err()
    );

    let output = result.expect("Test operation should succeed");
    assert!(
        output
            .result
            .contains("MCP Server: test-server.example.com"),
        "Dry-run output should include server URL: {}",
        output.result
    );

    // Metadata should include server URL
    assert!(
        output.metadata.contains_key("server_url"),
        "Metadata should contain server_url"
    );
    assert_eq!(
        output
            .metadata
            .get("server_url")
            .expect("Test operation should succeed"),
        "test-server.example.com",
        "Metadata server_url should match"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

// ============================================================================
// CRITICAL P0.2.5: MCP TIMEOUT AND HEARTBEAT TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_mcp_connection_timeout_configuration() {
    // TIMEOUT TEST: Connection timeout should be configurable and enforced
    // Clean up environment first
    std::env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    std::env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    std::env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    // Set environment variables for timeout configuration
    std::env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "5000"); // 5 seconds
    std::env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "15000"); // 15 seconds
    std::env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "60000"); // 60 seconds
    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "test.example.com");

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("timeout_config_tool");
    tokio::fs::write(&mock_binary, b"timeout config test")
        .await
        .expect("Test operation should succeed");

    let timeout_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "timeout_config_test".to_string(),
        "Tool for timeout configuration testing".to_string(),
        "test.example.com".to_string(),
    )
    .with_signature_requirement(false); // Disable signature for test

    // Tool should pick up environment timeout settings
    let spec = timeout_tool.spec();
    assert!(
        spec.usage.contains("CONN=5000ms"),
        "Tool spec should show connection timeout: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("HEARTBEAT=15000ms"),
        "Tool spec should show heartbeat interval: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("EXEC=60000ms"),
        "Tool spec should show max execution time: {}",
        spec.usage
    );

    // Dry-run should include timeout information
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: true,
        timeout_ms: Some(30000),
    };

    let result = timeout_tool.execute(input).await;
    assert!(
        result.is_ok(),
        "Dry-run should succeed with timeout config: {:?}",
        result.err()
    );

    let output = result.expect("Test operation should succeed");
    // Check metadata includes timeout values
    assert!(
        output.metadata.contains_key("connection_timeout_ms"),
        "Metadata should contain connection_timeout_ms"
    );
    assert_eq!(
        output
            .metadata
            .get("connection_timeout_ms")
            .expect("Test operation should succeed"),
        "5000",
        "Connection timeout should match configured value"
    );
    assert_eq!(
        output
            .metadata
            .get("heartbeat_interval_ms")
            .expect("Test operation should succeed"),
        "15000",
        "Heartbeat interval should match configured value"
    );
    assert_eq!(
        output
            .metadata
            .get("max_execution_time_ms")
            .expect("Test operation should succeed"),
        "60000",
        "Max execution time should match configured value"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    std::env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    std::env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

#[tokio::test]
#[serial]
async fn test_mcp_timeout_builder_methods() {
    // TIMEOUT TEST: Builder methods should configure timeout values with security limits
    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("builder_timeout_tool");
    tokio::fs::write(&mock_binary, b"builder timeout test")
        .await
        .expect("Test operation should succeed");

    // Test with extreme values (should be clamped to security limits)
    let timeout_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "builder_timeout_test".to_string(),
        "Tool for builder timeout testing".to_string(),
        "test.example.com".to_string(),
    )
    .with_signature_requirement(false)
    .with_connection_timeout(500) // Too low - should be clamped to 1000ms
    .with_heartbeat_interval(5_000) // Too low - should be clamped to 10_000ms
    .with_max_execution_time(2_000_000); // Too high - should be clamped to 1_800_000ms

    let spec = timeout_tool.spec();

    // Check that values are clamped to security limits
    assert!(
        spec.usage.contains("CONN=1000ms"), // Minimum 1 second
        "Connection timeout should be clamped to minimum: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("HEARTBEAT=10000ms"), // Minimum 10 seconds
        "Heartbeat interval should be clamped to minimum: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("EXEC=1800000ms"), // Maximum 30 minutes
        "Max execution time should be clamped to maximum: {}",
        spec.usage
    );
}

#[tokio::test]
#[serial]
async fn test_mcp_default_timeout_values() {
    // TIMEOUT TEST: Default timeout values should be secure and reasonable
    // Clean up environment to test defaults
    std::env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    std::env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    std::env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "default.example.com");

    // Create mock binary
    let temp_dir = tempfile::tempdir().expect("Test operation should succeed");
    let mock_binary = temp_dir.path().join("default_timeout_tool");
    tokio::fs::write(&mock_binary, b"default timeout test")
        .await
        .expect("Test operation should succeed");

    let default_tool = McpTool::new(
        mock_binary.to_string_lossy().to_string(),
        vec![],
        "default_timeout_test".to_string(),
        "Tool for default timeout testing".to_string(),
        "default.example.com".to_string(),
    )
    .with_signature_requirement(false);

    let spec = default_tool.spec();

    // Check for secure default values
    assert!(
        spec.usage.contains("CONN=30000ms"), // Default 30 seconds
        "Default connection timeout should be 30s: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("HEARTBEAT=60000ms"), // Default 60 seconds
        "Default heartbeat interval should be 60s: {}",
        spec.usage
    );
    assert!(
        spec.usage.contains("EXEC=300000ms"), // Default 5 minutes
        "Default max execution time should be 5 minutes: {}",
        spec.usage
    );

    // Dry-run should show default timeout values
    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: true,
        timeout_ms: None, // Use default
    };

    let result = default_tool.execute(input).await;
    assert!(result.is_ok(), "Dry-run with defaults should succeed");

    let output = result.expect("Test operation should succeed");
    assert_eq!(
        output
            .metadata
            .get("connection_timeout_ms")
            .expect("Test operation should succeed"),
        "30000",
        "Default connection timeout should be 30000ms"
    );
    assert_eq!(
        output
            .metadata
            .get("heartbeat_interval_ms")
            .expect("Test operation should succeed"),
        "60000",
        "Default heartbeat interval should be 60000ms"
    );
    assert_eq!(
        output
            .metadata
            .get("max_execution_time_ms")
            .expect("Test operation should succeed"),
        "300000",
        "Default max execution time should be 300000ms"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}
