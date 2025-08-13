//! Common Test Utilities
//! 
//! Shared functionality для всех integration tests:
//! - Test service creation helpers
//! - Mock setup и teardown
//! - Test data generation utilities
//! - Performance measurement helpers
//! - Assertion helpers для SLA validation

use anyhow::Result;
use memory::{
    DIMemoryService,
    service_di::{default_config, MemoryServiceConfig},
    Record, Layer,
    CacheConfigType,
};
use tempfile::TempDir;
use uuid::Uuid;
use chrono::Utc;
use std::sync::Arc;
use tokio::time::Instant;

/// Test configuration builder
pub struct TestConfigBuilder {
    cache_size: usize,
    health_enabled: bool,
    ai_enabled: bool,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            cache_size: 5000,
            health_enabled: true,
            ai_enabled: true,
        }
    }
    
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }
    
    pub fn with_health(mut self, enabled: bool) -> Self {
        self.health_enabled = enabled;
        self
    }
    
    pub fn with_ai(mut self, enabled: bool) -> Self {
        self.ai_enabled = enabled;
        self
    }
    
    pub async fn build(self) -> Result<DIMemoryService> {
        let temp_dir = TempDir::new()?;
        let mut config = default_config()?;
        
        config.db_path = temp_dir.path().join("test.db");
        config.cache_path = temp_dir.path().join("test_cache");
        config.cache_config = CacheConfigType::InMemory { max_size: self.cache_size };
        config.health_enabled = self.health_enabled;
        
        std::fs::create_dir_all(&config.cache_path)?;
        
        DIMemoryService::new(config).await
    }
}

/// Test record builder
pub struct TestRecordBuilder {
    id_prefix: String,
    content_template: String,
    layer: Layer,
    tags: Vec<String>,
    project: String,
    session: String,
}

impl TestRecordBuilder {
    pub fn new() -> Self {
        Self {
            id_prefix: "test".to_string(),
            content_template: "Test record {}: sample content".to_string(),
            layer: Layer::Interact,
            tags: vec!["test".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
        }
    }
    
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.id_prefix = prefix.to_string();
        self
    }
    
    pub fn with_template(mut self, template: &str) -> Self {
        self.content_template = template.to_string();
        self
    }
    
    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }
    
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn with_project(mut self, project: &str) -> Self {
        self.project = project.to_string();
        self
    }
    
    pub fn with_session(mut self, session: &str) -> Self {
        self.session = session.to_string();
        self
    }
    
    pub fn build(&self, index: usize) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: self.content_template.replace("{}", &index.to_string()),
            embedding: vec![],
            layer: self.layer,
            kind: format!("{}_record", self.id_prefix),
            tags: self.tags.clone(),
            project: self.project.clone(),
            session: self.session.clone(),
            score: 0.75 + (index % 100) as f32 / 400.0,
            ts: Utc::now(),
            access_count: (index % 10) as u32,
            last_access: Utc::now(),
        }
    }
    
    pub fn build_batch(&self, count: usize) -> Vec<Record> {
        (0..count).map(|i| self.build(i)).collect()
    }
}

/// Performance measurement helper
pub struct PerformanceMeasurement {
    start_time: Instant,
    operation_name: String,
}

impl PerformanceMeasurement {
    pub fn start(operation_name: &str) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.to_string(),
        }
    }
    
    pub fn elapsed_ms(&self) -> f64 {
        self.start_time.elapsed().as_micros() as f64 / 1000.0
    }
    
    pub fn finish(self) -> (String, f64) {
        (self.operation_name, self.elapsed_ms())
    }
    
    pub fn finish_with_validation(self, max_time_ms: f64) -> Result<(String, f64)> {
        let elapsed = self.elapsed_ms();
        if elapsed > max_time_ms {
            anyhow::bail!("{} took {:.3}ms, exceeding limit of {:.3}ms", 
                         self.operation_name, elapsed, max_time_ms);
        }
        Ok((self.operation_name, elapsed))
    }
}

/// SLA validation helpers
pub struct SlaValidator;

impl SlaValidator {
    /// Validates search latency SLA (< 5ms average)
    pub fn validate_search_latency(times: &[f64]) -> Result<()> {
        if times.is_empty() {
            anyhow::bail!("No search times provided for SLA validation");
        }
        
        let avg_time = times.iter().sum::<f64>() / times.len() as f64;
        let mut sorted_times = times.to_vec();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).expect("Test operation should succeed"));
        
        let p95 = sorted_times[(times.len() * 95) / 100];
        let p99 = sorted_times[(times.len() * 99) / 100];
        
        if avg_time >= 5.0 {
            anyhow::bail!("Search SLA violation: average {:.3}ms >= 5ms", avg_time);
        }
        
        if p95 >= 8.0 {
            anyhow::bail!("Search P95 SLA violation: {:.3}ms >= 8ms", p95);
        }
        
        if p99 >= 15.0 {
            anyhow::bail!("Search P99 SLA violation: {:.3}ms >= 15ms", p99);
        }
        
        Ok(())
    }
    
    /// Validates throughput SLA
    pub fn validate_throughput(operations: usize, duration_secs: f64, min_ops_per_sec: f64) -> Result<()> {
        let actual_throughput = operations as f64 / duration_secs;
        
        if actual_throughput < min_ops_per_sec {
            anyhow::bail!("Throughput SLA violation: {:.1} ops/sec < {:.1} ops/sec", 
                         actual_throughput, min_ops_per_sec);
        }
        
        Ok(())
    }
    
    /// Validates success rate SLA
    pub fn validate_success_rate(successful: usize, total: usize, min_rate_percent: f64) -> Result<()> {
        let success_rate = (successful as f64 / total as f64) * 100.0;
        
        if success_rate < min_rate_percent {
            anyhow::bail!("Success rate SLA violation: {:.1}% < {:.1}%", 
                         success_rate, min_rate_percent);
        }
        
        Ok(())
    }
    
    /// Validates cache efficiency SLA
    pub fn validate_cache_efficiency(hits: u64, misses: u64, min_hit_rate_percent: f64) -> Result<()> {
        if hits + misses == 0 {
            return Ok(()); // No cache operations
        }
        
        let hit_rate = (hits as f64 / (hits + misses) as f64) * 100.0;
        
        if hit_rate < min_hit_rate_percent {
            anyhow::bail!("Cache efficiency SLA violation: {:.1}% < {:.1}%", 
                         hit_rate, min_hit_rate_percent);
        }
        
        Ok(())
    }
}

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generates realistic test content for different domains
    pub fn generate_content(domain: &str, index: usize) -> String {
        match domain {
            "technical" => format!(
                "Technical documentation {}: API endpoint specification for service integration with authentication and rate limiting",
                index
            ),
            "business" => format!(
                "Business analysis {}: Market research findings and competitive intelligence for strategic planning",
                index
            ),
            "support" => format!(
                "Customer support case {}: User inquiry about product features, pricing, and implementation guidance",
                index
            ),
            "knowledge" => format!(
                "Knowledge base article {}: Best practices and troubleshooting guide for system configuration",
                index
            ),
            "analytics" => format!(
                "Analytics report {}: User engagement metrics and conversion tracking data for optimization",
                index
            ),
            _ => format!(
                "Generic test content {}: Sample data for validation and testing purposes",
                index
            ),
        }
    }
    
    /// Generates search queries for different scenarios
    pub fn generate_search_queries(scenario: &str, count: usize) -> Vec<String> {
        let templates = match scenario {
            "performance" => vec![
                "performance optimization techniques",
                "system monitoring and alerting",
                "database query optimization",
                "cache efficiency strategies",
                "load balancing configuration",
            ],
            "technical" => vec![
                "API authentication methods",
                "microservices architecture patterns",
                "container orchestration strategies", 
                "distributed system design",
                "security best practices",
            ],
            "business" => vec![
                "market analysis findings",
                "competitive intelligence reports",
                "customer satisfaction metrics",
                "revenue optimization strategies",
                "business process automation",
            ],
            _ => vec![
                "general search query",
                "information retrieval test",
                "content discovery validation",
                "relevance ranking verification",
                "semantic similarity testing",
            ],
        };
        
        (0..count)
            .map(|i| format!("{} {}", templates[i % templates.len()], i))
            .collect()
    }
}

/// Mock service state for testing error scenarios
pub struct MockServiceState {
    pub simulate_embedding_errors: bool,
    pub simulate_search_timeouts: bool,
    pub simulate_memory_pressure: bool,
    pub error_rate_percent: f64,
}

impl Default for MockServiceState {
    fn default() -> Self {
        Self {
            simulate_embedding_errors: false,
            simulate_search_timeouts: false,
            simulate_memory_pressure: false,
            error_rate_percent: 0.0,
        }
    }
}

impl MockServiceState {
    pub fn with_embedding_errors(mut self) -> Self {
        self.simulate_embedding_errors = true;
        self
    }
    
    pub fn with_search_timeouts(mut self) -> Self {
        self.simulate_search_timeouts = true;
        self
    }
    
    pub fn with_memory_pressure(mut self) -> Self {
        self.simulate_memory_pressure = true;
        self
    }
    
    pub fn with_error_rate(mut self, rate_percent: f64) -> Self {
        self.error_rate_percent = rate_percent;
        self
    }
    
    pub fn should_simulate_error(&self, operation_index: usize) -> bool {
        if self.error_rate_percent <= 0.0 {
            return false;
        }
        
        let threshold = self.error_rate_percent / 100.0;
        let pseudo_random = ((operation_index * 7919) % 10000) as f64 / 10000.0; // Simple deterministic "random"
        
        pseudo_random < threshold
    }
}

/// Test environment setup and teardown
pub struct TestEnvironment {
    pub service: Arc<DIMemoryService>,
    pub mock_state: MockServiceState,
}

impl TestEnvironment {
    pub async fn new() -> Result<Self> {
        let service = Arc::new(TestConfigBuilder::new().build().await?);
        let mock_state = MockServiceState::default();
        
        Ok(Self { service, mock_state })
    }
    
    pub async fn with_config(config_builder: TestConfigBuilder) -> Result<Self> {
        let service = Arc::new(config_builder.build().await?);
        let mock_state = MockServiceState::default();
        
        Ok(Self { service, mock_state })
    }
    
    pub fn with_mock_state(mut self, mock_state: MockServiceState) -> Self {
        self.mock_state = mock_state;
        self
    }
    
    /// Prepares test environment with baseline data
    pub async fn prepare_baseline_data(&self, record_count: usize) -> Result<()> {
        let record_builder = TestRecordBuilder::new()
            .with_prefix("baseline")
            .with_template("Baseline test data {}: comprehensive system validation content");
        
        let records = record_builder.build_batch(record_count);
        
        for record in records {
            self.service.insert(record).await?;
        }
        
        Ok(())
    }
    
    /// Validates system health after operations
    pub async fn validate_system_health(&self) -> Result<()> {
        let health = self.service.check_health().await?;
        
        if health.overall_status != "healthy" && health.overall_status != "degraded" {
            anyhow::bail!("System health check failed: {}", health.overall_status);
        }
        
        Ok(())
    }
}