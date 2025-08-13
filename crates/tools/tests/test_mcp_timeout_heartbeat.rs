use anyhow::Result;
use serial_test::serial;
use std::collections::HashMap;
use std::env;

// Import the MCP tool module
use tools::mcp::McpTool;
use tools::{Tool, ToolInput};

/// Test MCP connection timeout configuration
#[tokio::test]
#[serial]
async fn test_mcp_connection_timeout_configuration() {
    // Clean up environment variables first
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    // Set test whitelist to allow test server
    env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "tcp://localhost:8000");

    // Test 1: Default timeout values
    let tool = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_signature_requirement(false); // Disable signature for test

    // Default values (fallback since no env vars set)
    let default_conn_timeout = 30_000; // Default 30 seconds
    let default_heartbeat = 60_000; // Default 60 seconds
    let default_max_exec = 300_000; // Default 5 minutes

    let spec = tool.spec();
    let usage = &spec.usage;

    // Verify timeout values are included in spec
    assert!(usage.contains(&format!("CONN={default_conn_timeout}ms")));
    assert!(usage.contains(&format!("HEARTBEAT={default_heartbeat}ms")));
    assert!(usage.contains(&format!("EXEC={default_max_exec}ms")));

    // Clean up
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    println!("✅ MCP timeout configuration test passed");
}

/// Test MCP connection timeout builder methods
#[tokio::test]
async fn test_mcp_timeout_builder_methods() {
    let tool = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(15_000) // 15 seconds
    .with_heartbeat_interval(30_000) // 30 seconds
    .with_max_execution_time(600_000); // 10 minutes

    let spec = tool.spec();
    let usage = &spec.usage;

    // Verify custom timeout values are reflected in spec
    assert!(usage.contains("CONN=15000ms"));
    assert!(usage.contains("HEARTBEAT=30000ms"));
    assert!(usage.contains("EXEC=600000ms"));

    println!("✅ MCP timeout builder methods test passed");
}

/// Test MCP timeout security limits enforcement
#[tokio::test]
async fn test_mcp_timeout_security_limits() {
    // Test connection timeout limits (1s to 5 minutes)
    let tool_conn_low = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(100); // Too low (should be clamped to 1000ms)

    let tool_conn_high = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(400_000); // Too high (should be clamped to 300_000ms)

    // Test heartbeat limits (10s to 10 minutes)
    let tool_hb_low = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_heartbeat_interval(5_000); // Too low (should be clamped to 10_000ms)

    let tool_hb_high = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_heartbeat_interval(700_000); // Too high (should be clamped to 600_000ms)

    // Test max execution limits (5s to 30 minutes)
    let tool_exec_low = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_max_execution_time(2_000); // Too low (should be clamped to 5_000ms)

    let tool_exec_high = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_max_execution_time(2_000_000); // Too high (should be clamped to 1_800_000ms)

    // Verify security limits are enforced
    assert!(tool_conn_low.spec().usage.contains("CONN=1000ms")); // Clamped to minimum
    assert!(tool_conn_high.spec().usage.contains("CONN=300000ms")); // Clamped to maximum

    assert!(tool_hb_low.spec().usage.contains("HEARTBEAT=10000ms")); // Clamped to minimum
    assert!(tool_hb_high.spec().usage.contains("HEARTBEAT=600000ms")); // Clamped to maximum

    assert!(tool_exec_low.spec().usage.contains("EXEC=5000ms")); // Clamped to minimum
    assert!(tool_exec_high.spec().usage.contains("EXEC=1800000ms")); // Clamped to maximum

    println!("✅ MCP timeout security limits test passed");
}

/// Test MCP timeout metadata in dry-run mode
#[tokio::test]
async fn test_mcp_timeout_dry_run_metadata() -> Result<()> {
    let tool = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(25_000)
    .with_heartbeat_interval(45_000)
    .with_max_execution_time(400_000);

    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        timeout_ms: None,
        dry_run: true,
    };

    let output = tool.execute(input).await?;

    // Verify timeout metadata is included in dry-run
    assert!(output.success);
    assert_eq!(
        output.metadata.get("connection_timeout_ms"),
        Some(&"25000".to_string())
    );
    assert_eq!(
        output.metadata.get("heartbeat_interval_ms"),
        Some(&"45000".to_string())
    );
    assert_eq!(
        output.metadata.get("max_execution_time_ms"),
        Some(&"400000".to_string())
    );
    assert_eq!(output.metadata.get("dry_run"), Some(&"true".to_string()));

    println!("✅ MCP timeout dry-run metadata test passed");
    Ok(())
}

/// Test MCP connection timeout (simulated)
/// Note: This test uses a non-existent command to trigger connection timeout
#[tokio::test]
#[serial]
async fn test_mcp_connection_timeout_failure() -> Result<()> {
    // Clean up and set test environment
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "tcp://localhost:8000");

    let tool = McpTool::new(
        "nonexistent_mcp_server_12345".to_string(), // Non-existent command
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(2_000) // 2 second timeout for fast test
    .with_signature_requirement(false); // Skip signature for test

    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        timeout_ms: None,
        dry_run: false,
    };

    // This should timeout when trying to start the non-existent process
    let result = tool.execute(input).await;

    // Should return an error due to connection timeout or process spawn failure
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Error message should indicate connection issue or process failure
    assert!(
        error_msg.contains("timeout")
            || error_msg.contains("Failed to start")
            || error_msg.contains("No such file")
            || error_msg.contains("cannot find")
            || error_msg.contains("SECURITY BLOCK") // May be blocked by security
    );

    // Clean up
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    println!("✅ MCP connection timeout failure test passed");
    Ok(())
}

/// Test MCP execution timeout with environment variables
#[tokio::test]
#[serial]
async fn test_mcp_environment_variable_configuration() {
    // Clean up environment variables first
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    // Set test environment variables
    env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "20000");
    env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "50000");
    env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "450000");
    env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "tcp://localhost:8000");

    let tool = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_signature_requirement(false); // Disable signature for test

    let spec = tool.spec();
    let usage = &spec.usage;

    // Verify environment variables are used
    assert!(usage.contains("CONN=20000ms"));
    assert!(usage.contains("HEARTBEAT=50000ms"));
    assert!(usage.contains("EXEC=450000ms"));

    // Clean up environment variables
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");

    println!("✅ MCP environment variable configuration test passed");
}

/// Test MCP timeout integration with sandbox config
#[tokio::test]
#[serial]
async fn test_mcp_sandbox_config_timeout_integration() {
    // Clean up environment variables first
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");

    // Set sandbox environment variables for MCP timeouts
    env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "35000");
    env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "70000");
    env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "500000");

    // Create sandbox config from environment
    let sandbox_config = common::sandbox_config::SandboxConfig::from_env();

    // Verify sandbox config picks up timeout values
    assert_eq!(sandbox_config.mcp.connection_timeout_ms, 35000);
    assert_eq!(sandbox_config.mcp.heartbeat_interval_ms, 70000);
    assert_eq!(sandbox_config.mcp.max_execution_time_ms, 500000);

    // Clean up
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");

    println!("✅ MCP sandbox config timeout integration test passed");
}

/// Test MCP timeout security enforcement with invalid environment values
#[tokio::test]
#[serial]
async fn test_mcp_timeout_invalid_environment_values() {
    // Clean up environment variables first
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");

    // Set invalid environment variables
    env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "invalid_value");
    env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "-5000");
    env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "2000000"); // Too high

    let sandbox_config = common::sandbox_config::SandboxConfig::from_env();

    // Invalid values should fall back to defaults or be clamped
    assert_eq!(sandbox_config.mcp.connection_timeout_ms, 30000); // Default (invalid string)
    assert_eq!(sandbox_config.mcp.heartbeat_interval_ms, 60000); // Default (negative value)
    assert_eq!(sandbox_config.mcp.max_execution_time_ms, 1800000); // Clamped to maximum

    // Clean up
    env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");

    println!("✅ MCP timeout invalid environment values test passed");
}

/// Test MCP heartbeat functionality (simulation)
/// Note: This test simulates heartbeat behavior since we can't easily create a real MCP process for testing
#[tokio::test]
async fn test_mcp_heartbeat_simulation() {
    let tool = McpTool::new(
        "test_cmd".to_string(),
        vec![],
        "test_tool".to_string(),
        "Test MCP tool with heartbeat".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_heartbeat_interval(1000) // 1 second for fast test (will be clamped to 10s minimum)
    .with_connection_timeout(5000);

    // Verify heartbeat interval is clamped to security minimum
    let spec = tool.spec();
    assert!(spec.usage.contains("HEARTBEAT=10000ms")); // Should be clamped to 10s minimum

    println!("✅ MCP heartbeat simulation test passed");
}

/// Integration test: MCP timeout with all security features
#[tokio::test]
async fn test_mcp_timeout_complete_integration() -> Result<()> {
    let tool = McpTool::new(
        "echo".to_string(), // Use echo command for cross-platform compatibility
        vec!["test".to_string()],
        "echo_tool".to_string(),
        "Echo test tool for timeout integration".to_string(),
        "tcp://localhost:8000".to_string(),
    )
    .with_connection_timeout(10_000)
    .with_heartbeat_interval(20_000)
    .with_max_execution_time(30_000)
    .with_signature_requirement(false) // Skip signature for test
    .with_declared_capabilities(vec!["read-only".to_string()])
    .with_max_allowed_capabilities(vec!["read-only".to_string(), "computation".to_string()]);

    // Test dry-run mode with complete timeout/heartbeat metadata
    let dry_run_input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        timeout_ms: Some(25_000), // Should be limited by max_execution_time_ms
        dry_run: true,
    };

    let dry_run_output = tool.execute(dry_run_input).await?;
    assert!(dry_run_output.success);

    // Verify all timeout metadata is present
    assert!(dry_run_output
        .metadata
        .contains_key("connection_timeout_ms"));
    assert!(dry_run_output
        .metadata
        .contains_key("heartbeat_interval_ms"));
    assert!(dry_run_output
        .metadata
        .contains_key("max_execution_time_ms"));
    assert_eq!(
        dry_run_output.metadata.get("dry_run"),
        Some(&"true".to_string())
    );

    println!("✅ MCP complete timeout integration test passed");
    Ok(())
}
