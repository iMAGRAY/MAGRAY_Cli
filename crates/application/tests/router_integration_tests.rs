#![allow(unused_imports)]
#![allow(unused_attributes)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::absurd_extreme_comparisons)]
#![allow(unused_comparisons)]
#![allow(unused_variables)]
use anyhow::Result;
use application::services::router_application_service::RouterApplicationService;
use application::use_cases::router_use_cases::{
    GetRouterStatusRequest, RouteRequestRequest, RouterUseCases, RunRouterBenchmarkRequest,
};

/// Integration tests for Router functionality
///
/// These tests verify the complete Router integration from CLI commands
/// through Application Services down to Use Cases.

#[tokio::test]
async fn test_router_application_service_integration() {
    let service = RouterApplicationService::new();

    // Test basic service creation and health
    assert!(service
        .is_healthy()
        .await
        .expect("Test operation should succeed"));
}

#[tokio::test]
async fn test_full_routing_workflow() {
    let service = RouterApplicationService::new();

    // Test complete routing workflow
    let response = service
        .route_user_request(
            "read the configuration file for the project".to_string(),
            Some("development environment".to_string()),
            false, // Execute the route
            true,  // Include detailed analysis
        )
        .await
        .expect("Test operation should succeed");

    // Verify routing decision
    assert!(!response.selected_route.is_empty());
    assert!(response.confidence > 0.0);
    assert!(response.execution_attempted);
    assert!(response.execution_results.is_some());

    // Verify analysis was included
    let analysis = response.analysis.expect("Test operation should succeed");
    assert_eq!(analysis.detected_intent, "file_operation");
    assert!(!analysis.suggested_modules.is_empty());
    assert!(!analysis.required_capabilities.is_empty());

    // Should suggest tools module for file operations
    let tools_suggestion = analysis
        .suggested_modules
        .iter()
        .find(|s| s.module == "tools");
    assert!(tools_suggestion.is_some());
    assert!(
        tools_suggestion
            .expect("Test operation should succeed")
            .confidence
            > 0.5
    );
}

#[tokio::test]
async fn test_dry_run_analysis() {
    let service = RouterApplicationService::new();

    // Test analysis without execution (dry run)
    let response = service
        .analyze_request(
            "search for files containing 'config' in the current directory".to_string(),
        )
        .await
        .expect("Test operation should succeed");

    // Verify no execution occurred
    assert!(!response.execution_attempted);
    assert!(response.execution_results.is_none());

    // But analysis should be present
    assert!(response.analysis.is_some());
    assert!(!response.alternative_routes.is_empty());
}

#[tokio::test]
async fn test_quick_routing_performance() {
    let service = RouterApplicationService::new();

    let start_time = std::time::Instant::now();

    // Test quick routing (minimal analysis)
    let response = service
        .quick_route("memory search for recent documents".to_string())
        .await
        .expect("Test operation should succeed");

    let elapsed = start_time.elapsed();

    // Should be fast (under 100ms for mock implementation)
    assert!(elapsed.as_millis() < 100);

    // Should have basic routing info but no detailed analysis
    assert!(!response.selected_route.is_empty());
    assert!(response.analysis.is_none());
    assert!(!response.execution_attempted);

    // Should route to memory for memory-related queries
    assert!(response.selected_route.contains("memory"));
}

#[tokio::test]
async fn test_router_status_detailed() {
    let service = RouterApplicationService::new();

    // Test detailed status retrieval
    let status = service
        .get_router_status(true)
        .await
        .expect("Test operation should succeed");

    // Verify complete status information
    assert!(status.active);
    assert!(status.total_routes_processed > 0);
    assert!(!status.active_policies.is_empty());
    assert!(status.performance_stats.is_some());
    assert!(!status.agent_availability.is_empty());

    // Verify performance stats are included
    let perf_stats = status
        .performance_stats
        .expect("Test operation should succeed");
    assert!(perf_stats.avg_routing_time_ms > 0.0);
    assert!(perf_stats.success_rate > 0.0);
    assert!(perf_stats.last_24h_requests > 0);

    // Verify agent availability
    assert!(status.agent_availability.contains_key("llm"));
    assert!(status.agent_availability.contains_key("tools"));
    assert!(status.agent_availability.contains_key("memory"));
}

#[tokio::test]
async fn test_router_benchmarking() {
    let service = RouterApplicationService::new();

    // Test router benchmarking
    let results = service
        .run_benchmark(
            20,    // Small number for test speed
            false, // Sequential processing
            vec!["file_ops".to_string(), "memory_search".to_string()],
        )
        .await
        .expect("Test operation should succeed");

    // Verify benchmark results
    assert!(results.total_time_ms > 0);
    assert!(results.avg_time_per_request_ms > 0.0);
    assert!(results.requests_per_second > 0.0);
    assert!(results.success_rate > 0.0);
    assert_eq!(results.scenario_results.len(), 2);

    // Verify scenario-specific results
    let file_ops_result = results
        .scenario_results
        .get("file_ops")
        .expect("Test operation should succeed");
    assert_eq!(file_ops_result.scenario, "file_ops");
    assert!(file_ops_result.requests_processed > 0);
    assert!(file_ops_result.successes > 0);
}

#[tokio::test]
async fn test_request_validation() {
    let service = RouterApplicationService::new();

    // Test valid request
    assert!(service.validate_request("list files").await.is_ok());

    // Test empty request
    assert!(service.validate_request("").await.is_err());
    assert!(service.validate_request("   ").await.is_err());

    // Test too long request
    let long_request = "a".repeat(10001);
    assert!(service.validate_request(&long_request).await.is_err());
}

#[tokio::test]
async fn test_multiple_request_types() {
    let service = RouterApplicationService::new();

    let test_cases = vec![
        ("read file config.json", "file_operation", "tools"),
        ("remember this information", "memory_operation", "memory"),
        ("search for documents", "search_operation", "memory"),
        ("what is the weather like", "general_query", "llm"),
    ];

    for (request, expected_intent, expected_module) in test_cases {
        let response = service
            .analyze_request(request.to_string())
            .await
            .expect("Test operation should succeed");

        let analysis = response.analysis.expect("Test operation should succeed");
        assert_eq!(analysis.detected_intent, expected_intent);

        // Check that the expected module is suggested
        let suggested = analysis
            .suggested_modules
            .iter()
            .find(|s| s.module == expected_module);
        assert!(
            suggested.is_some(),
            "Expected module '{}' not suggested for request '{}'",
            expected_module,
            request
        );
    }
}

#[tokio::test]
async fn test_router_statistics() {
    let service = RouterApplicationService::new();

    // Get statistics
    let stats = service
        .get_statistics()
        .await
        .expect("Test operation should succeed");
    assert!(stats.is_some());

    let stats = stats.expect("Test operation should succeed");
    assert!(stats.avg_routing_time_ms > 0.0);
    assert!(stats.success_rate >= 0.0 && stats.success_rate <= 1.0);
    assert!(stats.last_24h_requests >= 0);
}

#[tokio::test]
async fn test_router_use_cases_direct() {
    let use_cases = RouterUseCases::new();

    // Test route request use case directly
    let route_request = RouteRequestRequest {
        user_request: "execute shell command ls -la".to_string(),
        context: Some("unix shell".to_string()),
        dry_run: false,
        include_analysis: true,
    };

    let route_response = use_cases
        .route_request
        .execute(route_request)
        .await
        .expect("Test operation should succeed");

    assert!(!route_response.selected_route.is_empty());
    assert!(route_response.confidence > 0.0);
    assert!(route_response.execution_attempted);
    assert!(route_response.analysis.is_some());

    // Test status use case directly
    let status_request = GetRouterStatusRequest {
        include_details: true,
    };

    let status_response = use_cases
        .get_status
        .execute(status_request)
        .await
        .expect("Test operation should succeed");

    assert!(status_response.active);
    assert!(!status_response.active_policies.is_empty());

    // Test benchmark use case directly
    let benchmark_request = RunRouterBenchmarkRequest {
        num_requests: 5,
        parallel: true,
        scenarios: vec!["test".to_string()],
    };

    let benchmark_response = use_cases
        .run_benchmark
        .execute(benchmark_request)
        .await
        .expect("Test operation should succeed");

    assert!(benchmark_response.total_time_ms > 0);
    assert!(benchmark_response.requests_per_second > 0.0);
    assert!(!benchmark_response.scenario_results.is_empty());
}

#[tokio::test]
async fn test_concurrent_routing_requests() {
    let service = RouterApplicationService::new();

    // Test concurrent requests to ensure thread safety
    let mut handles = Vec::new();

    for i in 0..10 {
        let service = RouterApplicationService::new(); // Each task gets its own service
        let request = format!("process file number {}", i);

        let handle = tokio::spawn(async move { service.quick_route(request).await });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    // Verify all requests completed successfully
    for result in results {
        let response = result
            .expect("Test operation should succeed")
            .expect("Test operation should succeed");
        assert!(!response.selected_route.is_empty());
        assert!(response.confidence > 0.0);
    }
}

#[tokio::test]
async fn test_error_handling_in_routing() {
    let service = RouterApplicationService::new();

    // Test routing with potentially problematic input
    let problematic_inputs = vec![
        "a".repeat(100),                                             // Very long single word
        "special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?".to_string(), // Special characters
        "unicode: 🚀🔥💯🎯".to_string(),                             // Unicode characters
        "tabs\tand\nnewlines".to_string(),                           // Control characters
    ];

    for input in problematic_inputs {
        let result = service.analyze_request(input.clone()).await;

        // Should handle gracefully - either succeed or fail with proper error
        match result {
            Ok(response) => {
                assert!(!response.selected_route.is_empty());
                assert!(response.analysis.is_some());
            }
            Err(e) => {
                // Error should have meaningful message
                assert!(!e.to_string().is_empty());
            }
        }
    }
}
