use anyhow::Result;
use std::sync::Arc;

use crate::use_cases::{
    GetRouterStatusRequest, GetRouterStatusResponse, GetRouterStatusUseCase, RouteRequestRequest,
    RouteRequestResponse, RouteRequestUseCase, RouterUseCases, RunRouterBenchmarkRequest,
    RunRouterBenchmarkResponse, RunRouterBenchmarkUseCase,
};

/// Application Service for Router operations
///
/// This service provides a high-level interface for router functionality,
/// orchestrating between different use cases and managing the overall router workflow.
///
/// It acts as a facade for the router domain, hiding the complexity of individual
/// use cases and providing a simplified API for the presentation layer (CLI).
pub struct RouterApplicationService {
    use_cases: RouterUseCases,
}

impl RouterApplicationService {
    /// Create a new Router Application Service
    pub fn new() -> Self {
        Self {
            use_cases: RouterUseCases::new(),
        }
    }

    /// Route a user request to the appropriate handler
    ///
    /// This is the main entry point for request routing. It analyzes the user's
    /// natural language request and determines the best route for execution.
    ///
    /// # Arguments
    /// * `user_request` - The user's natural language request
    /// * `context` - Optional context information
    /// * `dry_run` - If true, only analyze without executing
    /// * `include_analysis` - If true, include detailed analysis in response
    ///
    /// # Returns
    /// A `RouteRequestResponse` containing routing decisions and optionally execution results
    pub async fn route_user_request(
        &self,
        user_request: String,
        context: Option<String>,
        dry_run: bool,
        include_analysis: bool,
    ) -> Result<RouteRequestResponse> {
        let request = RouteRequestRequest {
            user_request,
            context,
            dry_run,
            include_analysis,
        };

        self.use_cases.route_request.execute(request).await
    }

    /// Get the current status of the router system
    ///
    /// Returns information about router health, performance, and configuration.
    ///
    /// # Arguments
    /// * `include_details` - If true, include detailed performance metrics
    ///
    /// # Returns
    /// A `GetRouterStatusResponse` with system status information
    pub async fn get_router_status(
        &self,
        include_details: bool,
    ) -> Result<GetRouterStatusResponse> {
        let request = GetRouterStatusRequest { include_details };

        self.use_cases.get_status.execute(request).await
    }

    /// Run router performance benchmarks
    ///
    /// Tests the router's performance with various scenarios and load patterns.
    ///
    /// # Arguments
    /// * `num_requests` - Number of test requests to process
    /// * `parallel` - Whether to run requests in parallel
    /// * `scenarios` - Specific scenarios to test (empty for default scenarios)
    ///
    /// # Returns
    /// A `RunRouterBenchmarkResponse` with benchmark results
    pub async fn run_benchmark(
        &self,
        num_requests: u32,
        parallel: bool,
        scenarios: Vec<String>,
    ) -> Result<RunRouterBenchmarkResponse> {
        let request = RunRouterBenchmarkRequest {
            num_requests,
            parallel,
            scenarios,
        };

        self.use_cases.run_benchmark.execute(request).await
    }

    /// Analyze a request without routing it
    ///
    /// This is a convenience method for getting detailed analysis of a request
    /// without actually executing any routing.
    ///
    /// # Arguments
    /// * `user_request` - The request to analyze
    ///
    /// # Returns
    /// A `RouteRequestResponse` with analysis but no execution
    pub async fn analyze_request(&self, user_request: String) -> Result<RouteRequestResponse> {
        self.route_user_request(user_request, None, true, true)
            .await
    }

    /// Quick route determination for simple requests
    ///
    /// Returns just the routing decision without detailed analysis or execution.
    /// This is optimized for performance when only the route is needed.
    ///
    /// # Arguments
    /// * `user_request` - The request to route
    ///
    /// # Returns
    /// A `RouteRequestResponse` with basic routing information
    pub async fn quick_route(&self, user_request: String) -> Result<RouteRequestResponse> {
        self.route_user_request(user_request, None, true, false)
            .await
    }

    /// Execute a request through the determined route
    ///
    /// This combines routing analysis with actual execution.
    ///
    /// # Arguments
    /// * `user_request` - The request to route and execute
    /// * `context` - Optional context information
    ///
    /// # Returns
    /// A `RouteRequestResponse` with routing decisions and execution results
    pub async fn route_and_execute(
        &self,
        user_request: String,
        context: Option<String>,
    ) -> Result<RouteRequestResponse> {
        self.route_user_request(user_request, context, false, true)
            .await
    }

    /// Check if the router system is healthy
    ///
    /// Quick health check that returns basic system status.
    ///
    /// # Returns
    /// `true` if the router system is operational, `false` otherwise
    pub async fn is_healthy(&self) -> Result<bool> {
        let status = self.get_router_status(false).await?;
        Ok(status.active)
    }

    /// Get routing statistics
    ///
    /// Returns performance and usage statistics for the router system.
    ///
    /// # Returns
    /// Performance statistics if available
    pub async fn get_statistics(&self) -> Result<Option<crate::use_cases::RouterPerformanceStats>> {
        let status = self.get_router_status(true).await?;
        Ok(status.performance_stats)
    }

    /// Validate a routing request
    ///
    /// Checks if a request can be properly routed before attempting execution.
    ///
    /// # Arguments
    /// * `user_request` - The request to validate
    ///
    /// # Returns
    /// `Ok(())` if the request can be routed, `Err` with details if not
    pub async fn validate_request(&self, user_request: &str) -> Result<()> {
        if user_request.trim().is_empty() {
            return Err(anyhow::anyhow!("Request cannot be empty"));
        }

        if user_request.len() > 10000 {
            return Err(anyhow::anyhow!("Request too long (max 10000 characters)"));
        }

        // Additional validation could be added here
        // For now, we'll do a quick routing check
        let response = self.quick_route(user_request.to_string()).await?;

        if response.confidence < 0.1 {
            return Err(anyhow::anyhow!("Request cannot be confidently routed"));
        }

        Ok(())
    }
}

impl Default for RouterApplicationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_application_service_creation() {
        let service = RouterApplicationService::new();

        // Test that the service was created successfully
        let is_healthy = service
            .is_healthy()
            .await
            .expect("Operation should succeed");
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_route_user_request() {
        let service = RouterApplicationService::new();

        let response = service
            .route_user_request(
                "read the config file".to_string(),
                None,
                false, // don't dry run - actually execute
                true,  // include analysis
            )
            .await
            .expect("Operation should succeed");

        assert!(!response.selected_route.is_empty());
        assert!(response.confidence > 0.0);
        assert!(response.analysis.is_some());
        assert!(response.execution_attempted);
        assert!(response.execution_results.is_some());
    }

    #[tokio::test]
    async fn test_analyze_request() {
        let service = RouterApplicationService::new();

        let response = service
            .analyze_request("search for files".to_string())
            .await
            .expect("Operation should succeed");

        assert!(!response.selected_route.is_empty());
        assert!(response.analysis.is_some());
        assert!(!response.execution_attempted); // Should be false for analysis
        assert!(response.execution_results.is_none());
    }

    #[tokio::test]
    async fn test_quick_route() {
        let service = RouterApplicationService::new();

        let response = service
            .quick_route("memory search".to_string())
            .await
            .expect("Operation should succeed");

        assert!(!response.selected_route.is_empty());
        assert!(response.analysis.is_none()); // Should be None for quick route
        assert!(!response.execution_attempted);
    }

    #[tokio::test]
    async fn test_get_router_status() {
        let service = RouterApplicationService::new();

        let status = service
            .get_router_status(true)
            .await
            .expect("Operation should succeed");

        assert!(status.active);
        assert!(!status.active_policies.is_empty());
        assert!(status.performance_stats.is_some());
    }

    #[tokio::test]
    async fn test_run_benchmark() {
        let service = RouterApplicationService::new();

        let results = service
            .run_benchmark(5, false, vec![])
            .await
            .expect("Operation should succeed");

        assert!(results.total_time_ms > 0);
        assert!(results.requests_per_second > 0.0);
        assert!(!results.scenario_results.is_empty());
    }

    #[tokio::test]
    async fn test_validate_request() {
        let service = RouterApplicationService::new();

        // Valid request
        let result = service.validate_request("read file").await;
        assert!(result.is_ok());

        // Empty request
        let result = service.validate_request("").await;
        assert!(result.is_err());

        // Too long request
        let long_request = "a".repeat(10001);
        let result = service.validate_request(&long_request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_is_healthy() {
        let service = RouterApplicationService::new();

        let healthy = service
            .is_healthy()
            .await
            .expect("Operation should succeed");
        assert!(healthy);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let service = RouterApplicationService::new();

        let stats = service
            .get_statistics()
            .await
            .expect("Operation should succeed");
        assert!(stats.is_some());

        let stats = stats.expect("Operation should succeed");
        assert!(stats.avg_routing_time_ms > 0.0);
        assert!(stats.success_rate > 0.0);
    }

    #[tokio::test]
    async fn test_route_and_execute() {
        let service = RouterApplicationService::new();

        let response = service
            .route_and_execute(
                "list files in directory".to_string(),
                Some("working directory context".to_string()),
            )
            .await
            .expect("Operation should succeed");

        assert!(!response.selected_route.is_empty());
        assert!(response.execution_attempted);
        assert!(response.analysis.is_some());
        assert!(response.execution_results.is_some());
    }
}
