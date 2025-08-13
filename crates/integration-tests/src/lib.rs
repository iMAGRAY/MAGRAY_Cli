//! Comprehensive Integration Testing Suite for MAGRAY CLI
//!
//! This crate provides comprehensive integration tests that validate the interaction
//! between completed components:
//!
//! ## Integration Testing Scope
//!
//! ### 1. Multi-Component Integration Tests
//! - **Tool Context Builder + AI Embeddings** (P1.3.2): Real AI embedding workflows with fallback
//! - **Config Profiles + Security Integration** (P2.3.7): Dev/prod profile security workflows  
//! - **Multi-Agent Orchestration Integration**: Agent coordination and EventBus flows
//!
//! ### 2. End-to-End Workflows  
//! - **Complete tool selection pipeline**: Query → Embedding → Context → Selection
//! - **Security policy evaluation**: Request → Policy → Capability → Decision
//! - **Configuration management**: Load → Validate → Apply → Switch profiles
//! - **Agent coordination**: Intent → Plan → Execute → Review cycle
//!
//! ### 3. Performance Integration Testing
//! - **Load testing**: High-volume tool requests and agent coordination
//! - **Stress testing**: Memory pressure and concurrent operations
//! - **Failover testing**: Component failure handling and recovery
//!
//! ### 4. Security Integration Validation
//! - **Policy enforcement**: Cross-component security validation
//! - **Capability validation**: Permission checking across components
//! - **Audit trail**: Security event logging and monitoring
//!
//! ## Usage
//!
//! ### Running Integration Tests
//! ```bash
//! # Run all integration tests
//! cargo test --package integration-tests
//!
//! # Run specific test suite
//! cargo test --package integration-tests multi_component_integration
//!
//! # Run with real AI models (requires models)
//! cargo test --package integration-tests --features real_ai
//!
//! # Run performance tests
//! cargo test --package integration-tests performance_integration
//! ```
//!
//! ### Running Benchmarks
//! ```bash
//! # Run integration benchmarks
//! cargo bench --package integration-tests
//!
//! # Run with flamegraph profiling
//! cargo bench --package integration-tests --features stress_testing
//! ```
//!
//! ## Test Environment Setup
//!
//! Tests are designed to run in isolated environments with:
//! - Temporary directories for file system tests
//! - Mock external dependencies by default
//! - Real component integration validation
//! - Performance measurement and reporting

pub mod common;
pub mod fixtures;
// Optional modules (placeholder implementations - not implemented yet)
// pub mod performance;
// pub mod security;
// pub mod utils;

/// Test environment configuration
#[derive(Debug, Clone)]
pub struct TestEnvironment {
    pub temp_dir: std::path::PathBuf,
    pub use_real_ai: bool,
    pub use_real_network: bool,
    pub log_level: tracing::Level,
    pub test_timeout_secs: u64,
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("magray_integration_tests"),
            use_real_ai: cfg!(feature = "real_ai"),
            use_real_network: false, // Safe default
            log_level: tracing::Level::INFO,
            test_timeout_secs: 60,
        }
    }
}

impl TestEnvironment {
    /// Create test environment with temporary directory
    pub async fn setup() -> anyhow::Result<Self> {
        let env = Self::default();

        // Create temporary directory
        tokio::fs::create_dir_all(&env.temp_dir).await?;

        // Initialize logging for tests
        let _ = tracing_subscriber::fmt()
            .with_max_level(env.log_level)
            .with_test_writer()
            .try_init();

        tracing::info!("Integration test environment initialized");
        tracing::info!("Temp dir: {}", env.temp_dir.display());
        tracing::info!("Use real AI: {}", env.use_real_ai);
        tracing::info!("Test timeout: {}s", env.test_timeout_secs);

        Ok(env)
    }

    /// Clean up test environment - best effort cleanup on Windows
    pub async fn cleanup(&self) -> anyhow::Result<()> {
        if self.temp_dir.exists() {
            // On Windows, try multiple times with delay for file handles to close
            for attempt in 0..3 {
                match tokio::fs::remove_dir_all(&self.temp_dir).await {
                    Ok(_) => {
                        tracing::info!("Cleaned up test environment");
                        return Ok(());
                    }
                    Err(e) if attempt < 2 => {
                        tracing::warn!(
                            "Cleanup attempt {} failed: {}, retrying...",
                            attempt + 1,
                            e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Final cleanup attempt failed: {}. Directory may remain: {}",
                            e,
                            self.temp_dir.display()
                        );
                        // Don't fail the test due to cleanup issues
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// Get path in temp directory
    pub fn temp_path(&self, path: &str) -> std::path::PathBuf {
        self.temp_dir.join(path)
    }
}

/// Integration test result with metrics
#[derive(Debug, Clone)]
pub struct IntegrationTestResult {
    pub test_name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub memory_usage_mb: f64,
    pub components_tested: Vec<String>,
    pub metrics: std::collections::HashMap<String, f64>,
    pub errors: Vec<String>,
}

impl IntegrationTestResult {
    pub fn new(test_name: String) -> Self {
        Self {
            test_name,
            passed: false,
            duration_ms: 0,
            memory_usage_mb: 0.0,
            components_tested: Vec::new(),
            metrics: std::collections::HashMap::new(),
            errors: Vec::new(),
        }
    }

    pub fn with_component(mut self, component: &str) -> Self {
        self.components_tested.push(component.to_string());
        self
    }

    pub fn with_metric(mut self, name: &str, value: f64) -> Self {
        self.metrics.insert(name.to_string(), value);
        self
    }

    pub fn with_error(mut self, error: &str) -> Self {
        self.errors.push(error.to_string());
        self
    }

    pub fn success(mut self, duration_ms: u64) -> Self {
        self.passed = true;
        self.duration_ms = duration_ms;
        self
    }

    pub fn failure(mut self, duration_ms: u64, error: &str) -> Self {
        self.passed = false;
        self.duration_ms = duration_ms;
        self.errors.push(error.to_string());
        self
    }
}

/// Test suite runner with metrics collection
pub struct IntegrationTestRunner {
    env: TestEnvironment,
    results: Vec<IntegrationTestResult>,
}

impl IntegrationTestRunner {
    pub async fn new() -> anyhow::Result<Self> {
        let env = TestEnvironment::setup().await?;
        Ok(Self {
            env,
            results: Vec::new(),
        })
    }

    pub fn environment(&self) -> &TestEnvironment {
        &self.env
    }

    pub async fn run_test<F, Fut>(&mut self, test_name: &str, test_fn: F) -> anyhow::Result<()>
    where
        F: FnOnce(TestEnvironment) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<IntegrationTestResult>>,
    {
        tracing::info!("Running integration test: {}", test_name);
        let start = std::time::Instant::now();

        match test_fn(self.env.clone()).await {
            Ok(mut result) => {
                result.duration_ms = start.elapsed().as_millis() as u64;
                tracing::info!("Test {} completed in {}ms", test_name, result.duration_ms);
                self.results.push(result);
            }
            Err(e) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let result = IntegrationTestResult::new(test_name.to_string())
                    .failure(duration_ms, &e.to_string());
                tracing::error!("Test {} failed after {}ms: {}", test_name, duration_ms, e);
                self.results.push(result);
                return Err(e);
            }
        }

        Ok(())
    }

    pub fn get_results(&self) -> &[IntegrationTestResult] {
        &self.results
    }

    pub async fn cleanup(self) -> anyhow::Result<()> {
        self.env.cleanup().await
    }

    /// Generate comprehensive test report
    pub fn generate_report(&self) -> String {
        let total_tests = self.results.len();
        let passed_tests = self.results.iter().filter(|r| r.passed).count();
        let failed_tests = total_tests - passed_tests;

        let total_duration: u64 = self.results.iter().map(|r| r.duration_ms).sum();
        let avg_duration = if total_tests > 0 {
            total_duration / total_tests as u64
        } else {
            0
        };

        let mut report = String::new();
        report.push_str("# Integration Test Report\n\n");
        report.push_str(&format!("**Total Tests**: {}\n", total_tests));
        report.push_str(&format!("**Passed**: {} ✅\n", passed_tests));
        report.push_str(&format!("**Failed**: {} ❌\n", failed_tests));
        report.push_str(&format!(
            "**Success Rate**: {:.1}%\n",
            if total_tests > 0 {
                (passed_tests as f64 / total_tests as f64) * 100.0
            } else {
                0.0
            }
        ));
        report.push_str(&format!("**Total Duration**: {}ms\n", total_duration));
        report.push_str(&format!("**Average Duration**: {}ms\n\n", avg_duration));

        // Components coverage
        let mut components: std::collections::HashSet<String> = std::collections::HashSet::new();
        for result in &self.results {
            for component in &result.components_tested {
                components.insert(component.clone());
            }
        }
        report.push_str("## Components Tested\n");
        for component in components.iter() {
            report.push_str(&format!("- {}\n", component));
        }
        report.push_str("\n");

        // Detailed results
        report.push_str("## Test Results\n\n");
        for result in &self.results {
            let status = if result.passed {
                "✅ PASS"
            } else {
                "❌ FAIL"
            };
            report.push_str(&format!("### {} - {}\n", result.test_name, status));
            report.push_str(&format!("- **Duration**: {}ms\n", result.duration_ms));
            report.push_str(&format!(
                "- **Components**: {}\n",
                result.components_tested.join(", ")
            ));

            if !result.metrics.is_empty() {
                report.push_str("- **Metrics**:\n");
                for (name, value) in &result.metrics {
                    report.push_str(&format!("  - {}: {:.2}\n", name, value));
                }
            }

            if !result.errors.is_empty() {
                report.push_str("- **Errors**:\n");
                for error in &result.errors {
                    report.push_str(&format!("  - {}\n", error));
                }
            }
            report.push_str("\n");
        }

        report
    }
}
