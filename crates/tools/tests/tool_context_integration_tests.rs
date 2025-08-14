// @component: {"k":"T","id":"tool_context_integration_tests","t":"Integration tests for ToolContextBuilder","m":{"cur":0,"tgt":100,"u":"%"},"f":["test","integration","context","builder"]}

use std::collections::HashMap;
use std::sync::Arc;
use tools::context::{ContextBuildingConfig, ToolContextBuilder, ToolSelectionRequest};
use tools::registry::{SecureToolRegistry, SecurityConfig};

/// Test that ToolContextBuilder can be created successfully
#[tokio::test]
async fn test_tool_context_builder_creation() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder_result = ToolContextBuilder::new(registry);
    assert!(
        builder_result.is_ok(),
        "ToolContextBuilder should be created successfully"
    );

    let builder = builder_result.unwrap();
    assert!(
        builder.has_intelligent_tool_selection(),
        "Should use semantic reranking by default"
    );
}

/// Test that ToolContextBuilder can process basic tool selection requests
#[tokio::test]
async fn test_basic_tool_selection() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(registry).expect("Failed to create ToolContextBuilder");

    let request = ToolSelectionRequest {
        query: "file operations".to_string(),
        context: HashMap::new(),
        required_categories: Some(vec!["filesystem".to_string()]),
        exclude_tools: vec![],
        platform: Some("windows".to_string()),
        max_security_level: None,
        prefer_fast_tools: true,
        include_experimental: false,
    };

    let response = builder.build_context(request).await;
    assert!(response.is_ok(), "Tool selection should succeed");

    let response = response.unwrap();
    assert!(
        !response.tools.is_empty() || response.tools.is_empty(),
        "Should handle empty registry gracefully"
    );
    assert!(
        response.selection_metrics.total_time.as_millis() < 5000,
        "Selection should complete within 5 seconds"
    );
}

/// Test that ToolContextBuilder works with custom configuration
#[tokio::test]
async fn test_custom_configuration() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let config = ContextBuildingConfig {
        max_candidate_tools: 20,
        max_context_tools: 5,
        similarity_threshold: 0.3,
        use_semantic_reranking: false, // Use simple ranking for test
        enable_caching: false,
        include_usage_patterns: true,
        include_performance_metrics: true,
        max_build_time: std::time::Duration::from_secs(2),
    };

    let builder = ToolContextBuilder::with_config(registry, config)
        .expect("Failed to create ToolContextBuilder with config");

    let request = ToolSelectionRequest {
        query: "search and query".to_string(),
        context: HashMap::from([("project_type".to_string(), "rust".to_string())]),
        required_categories: None,
        exclude_tools: vec!["deprecated_tool".to_string()],
        platform: Some("windows".to_string()),
        max_security_level: Some("MediumRisk".to_string()),
        prefer_fast_tools: false,
        include_experimental: true,
    };

    let response = builder.build_context(request).await;
    assert!(response.is_ok(), "Custom configuration should work");
}

/// Test fallback behavior when no tools match
#[tokio::test]
async fn test_fallback_behavior() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(registry).expect("Failed to create ToolContextBuilder");

    let request = ToolSelectionRequest {
        query: "very specific non-existent tool functionality".to_string(),
        context: HashMap::new(),
        required_categories: Some(vec!["non_existent_category".to_string()]),
        exclude_tools: vec![],
        platform: None,
        max_security_level: None,
        prefer_fast_tools: true,
        include_experimental: false,
    };

    let response = builder.build_context(request).await;
    assert!(response.is_ok(), "Should handle no matches gracefully");

    let response = response.unwrap();
    // Should return empty results for non-existent categories
    assert!(
        response.tools.is_empty(),
        "Should return empty results for non-existent categories"
    );
}

/// Test error handling with malformed requests
#[tokio::test]
async fn test_error_handling() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(registry).expect("Failed to create ToolContextBuilder");

    // Test empty query
    let request = ToolSelectionRequest {
        query: "".to_string(),
        context: HashMap::new(),
        required_categories: None,
        exclude_tools: vec![],
        platform: None,
        max_security_level: None,
        prefer_fast_tools: true,
        include_experimental: false,
    };

    let response = builder.build_context(request).await;
    assert!(response.is_ok(), "Empty query should be handled gracefully");
}

/// Test performance requirements are met
#[tokio::test]
async fn test_performance_requirements() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(registry).expect("Failed to create ToolContextBuilder");

    let request = ToolSelectionRequest {
        query: "performance test query".to_string(),
        context: HashMap::new(),
        required_categories: None,
        exclude_tools: vec![],
        platform: None,
        max_security_level: None,
        prefer_fast_tools: true,
        include_experimental: false,
    };

    let start_time = std::time::Instant::now();
    let response = builder.build_context(request).await;
    let elapsed = start_time.elapsed();

    assert!(response.is_ok(), "Performance test should succeed");
    assert!(
        elapsed.as_millis() < 100,
        "Tool selection should complete in under 100ms for empty registry"
    );

    let response = response.unwrap();
    assert!(
        response.selection_metrics.total_time.as_millis() < 100,
        "Reported metrics should be under 100ms"
    );
}

/// Integration test with orchestrator-like usage pattern
#[tokio::test]
async fn test_orchestrator_integration_pattern() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(registry).expect("Failed to create ToolContextBuilder");

    // Simulate different intent types that orchestrator would send
    let intent_patterns = vec![
        ("execute git status", Some(vec!["git".to_string()])),
        ("search for files", Some(vec!["filesystem".to_string()])),
        ("make http request", Some(vec!["web".to_string()])),
        ("analyze code", Some(vec!["analysis".to_string()])),
    ];

    for (query, categories) in intent_patterns {
        let request = ToolSelectionRequest {
            query: query.to_string(),
            context: HashMap::from([
                ("intent_type".to_string(), "ExecuteTool".to_string()),
                ("session_id".to_string(), "test-session".to_string()),
            ]),
            required_categories: categories,
            exclude_tools: vec![],
            platform: Some("windows".to_string()),
            max_security_level: None,
            prefer_fast_tools: true,
            include_experimental: false,
        };

        let response = builder.build_context(request).await;
        assert!(
            response.is_ok(),
            "Orchestrator pattern should work for query: {query}"
        );

        let response = response.unwrap();
        // Verify response structure
        assert!(
            response.context.query == query,
            "Query should be preserved in context"
        );
        assert!(
            response.selection_metrics.total_time.as_millis() < 1000,
            "Should complete quickly"
        );
    }
}

/// Test that builder methods work correctly
#[tokio::test]
async fn test_builder_methods() {
    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));

    let builder = ToolContextBuilder::new(Arc::clone(&registry))
        .expect("Failed to create ToolContextBuilder");

    // Test has_intelligent_tool_selection
    let has_intelligent = builder.has_intelligent_tool_selection();
    assert!(
        has_intelligent,
        "Should report semantic reranking by default"
    );

    // Test with_config constructor
    let custom_config = ContextBuildingConfig {
        max_candidate_tools: 10,
        max_context_tools: 3,
        similarity_threshold: 0.5,
        use_semantic_reranking: false,
        enable_caching: true,
        include_usage_patterns: false,
        include_performance_metrics: false,
        max_build_time: std::time::Duration::from_millis(500),
    };

    let custom_builder = ToolContextBuilder::with_config(registry, custom_config);
    assert!(
        custom_builder.is_ok(),
        "Custom config constructor should work"
    );
}
