/// MCP Timeout/Heartbeat Tests - Critical P0.2.5 Implementation Verification
///
/// This test suite verifies that MCP timeout and heartbeat mechanisms work correctly.
/// These tests ensure DoS prevention and proper resource cleanup.
use serial_test::serial;
use std::collections::HashMap;
use tools::{mcp::McpTool, Tool, ToolInput};

// ============================================================================
// CRITICAL P0.2.5: MCP TIMEOUT/HEARTBEAT TESTS
// ============================================================================

#[tokio::test]
#[serial]
async fn test_mcp_timeout_configuration() {
    // TIMEOUT TEST: MCP tools should respect timeout configurations
    std::env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "15000"); // 15 seconds
    std::env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "30000"); // 30 seconds
    std::env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "120000"); // 2 minutes

    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "timeout_test_tool".to_string(),
        "Tool for timeout testing".to_string(),
        "test.example.com".to_string(),
    )
    .with_signature_requirement(false); // Disable signature for testing

    let spec = mcp_tool.spec();

    // Spec should show timeout information
    assert!(
        spec.usage.contains("CONN=15000ms"),
        "Usage should show connection timeout: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("HEARTBEAT=30000ms"),
        "Usage should show heartbeat interval: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("EXEC=120000ms"),
        "Usage should show max execution time: {}",
        spec.usage
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    std::env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    std::env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
}

#[tokio::test]
#[serial]
async fn test_mcp_timeout_security_limits() {
    // SECURITY TEST: Timeout values should be clamped to secure limits
    // Clean up environment first
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "test.example.com"); // Allow test server

    std::env::set_var("MAGRAY_MCP_CONNECTION_TIMEOUT", "600000"); // 10 minutes - too high
    std::env::set_var("MAGRAY_MCP_HEARTBEAT_INTERVAL", "5000"); // 5 seconds - too low
    std::env::set_var("MAGRAY_MCP_MAX_EXECUTION_TIME", "3600000"); // 1 hour - too high

    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "timeout_clamp_test".to_string(),
        "Tool for timeout clamping test".to_string(),
        "test.example.com".to_string(),
    )
    .with_signature_requirement(false); // Disable signature for testing

    let spec = mcp_tool.spec();

    // Connection timeout should be clamped to max 5 minutes (300000ms)
    assert!(
        spec.usage.contains("CONN=300000ms"),
        "Connection timeout should be clamped to 300000ms: {}",
        spec.usage
    );

    // Heartbeat should be clamped to min 10 seconds (10000ms)
    assert!(
        spec.usage.contains("HEARTBEAT=10000ms"),
        "Heartbeat should be clamped to 10000ms: {}",
        spec.usage
    );

    // Max execution should be clamped to max 30 minutes (1800000ms)
    assert!(
        spec.usage.contains("EXEC=1800000ms"),
        "Max execution should be clamped to 1800000ms: {}",
        spec.usage
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_CONNECTION_TIMEOUT");
    std::env::remove_var("MAGRAY_MCP_HEARTBEAT_INTERVAL");
    std::env::remove_var("MAGRAY_MCP_MAX_EXECUTION_TIME");
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

#[tokio::test]
async fn test_mcp_timeout_builder_methods() {
    // BUILDER TEST: Builder methods should respect security limits
    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "builder_timeout_test".to_string(),
        "Tool for builder timeout test".to_string(),
        "test.example.com".to_string(),
    )
    .with_signature_requirement(false) // Disable signature for testing
    .with_connection_timeout(500) // Too low - should be clamped to 1000ms
    .with_heartbeat_interval(5000) // Too low - should be clamped to 10000ms
    .with_max_execution_time(2000000); // Too high - should be clamped to 1800000ms

    let spec = mcp_tool.spec();

    // All values should be clamped to secure limits
    assert!(
        spec.usage.contains("CONN=1000ms"),
        "Connection timeout should be clamped to 1000ms minimum: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("HEARTBEAT=10000ms"),
        "Heartbeat should be clamped to 10000ms minimum: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("EXEC=1800000ms"),
        "Max execution should be clamped to 1800000ms maximum: {}",
        spec.usage
    );
}

#[tokio::test]
#[serial]
async fn test_mcp_timeout_metadata_in_dry_run() {
    // METADATA TEST: Dry-run should include timeout information in metadata
    std::env::set_var("MAGRAY_MCP_SERVER_WHITELIST", "test-timeout.example.com");

    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "dry_run_timeout_test".to_string(),
        "Tool for dry-run timeout metadata test".to_string(),
        "test-timeout.example.com".to_string(),
    )
    .with_signature_requirement(false) // Disable signature for testing
    .with_connection_timeout(25000)
    .with_heartbeat_interval(45000)
    .with_max_execution_time(180000);

    let input = ToolInput {
        command: "test".to_string(),
        args: HashMap::new(),
        context: None,
        dry_run: true,
        timeout_ms: Some(1000),
    };

    let result = mcp_tool.execute(input).await;
    assert!(result.is_ok(), "Dry-run should succeed: {:?}", result.err());

    let output = result.expect("Test operation should succeed");

    // Check timeout metadata is present
    assert!(
        output.metadata.contains_key("connection_timeout_ms"),
        "Metadata should contain connection_timeout_ms"
    );
    assert_eq!(
        output
            .metadata
            .get("connection_timeout_ms")
            .expect("Test operation should succeed"),
        "25000",
        "Connection timeout metadata should match configured value"
    );

    assert!(
        output.metadata.contains_key("heartbeat_interval_ms"),
        "Metadata should contain heartbeat_interval_ms"
    );
    assert_eq!(
        output
            .metadata
            .get("heartbeat_interval_ms")
            .expect("Test operation should succeed"),
        "45000",
        "Heartbeat interval metadata should match configured value"
    );

    assert!(
        output.metadata.contains_key("max_execution_time_ms"),
        "Metadata should contain max_execution_time_ms"
    );
    assert_eq!(
        output
            .metadata
            .get("max_execution_time_ms")
            .expect("Test operation should succeed"),
        "180000",
        "Max execution time metadata should match configured value"
    );

    // Clean up environment
    std::env::remove_var("MAGRAY_MCP_SERVER_WHITELIST");
}

#[tokio::test]
async fn test_mcp_default_timeouts() {
    // DEFAULT TEST: MCP tools should have secure default timeouts
    let mcp_tool = McpTool::new(
        "echo".to_string(),
        vec![],
        "default_timeout_test".to_string(),
        "Tool for default timeout test".to_string(),
        "default.example.com".to_string(),
    )
    .with_signature_requirement(false);

    let spec = mcp_tool.spec();

    // Should have secure defaults without environment variables
    assert!(
        spec.usage.contains("CONN=30000ms"),
        "Should use default connection timeout of 30000ms: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("HEARTBEAT=60000ms"),
        "Should use default heartbeat interval of 60000ms: {}",
        spec.usage
    );

    assert!(
        spec.usage.contains("EXEC=300000ms"),
        "Should use default max execution time of 300000ms: {}",
        spec.usage
    );
}
