//! Common testing utilities and mock infrastructure for MAGRAY CLI
//!
//! Provides reusable test fixtures, mocks, and utilities across all crates.
//! Focuses on creating bulletproof testing infrastructure that catches real bugs.

use rand::prelude::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Mock HTTP client for testing external API interactions
#[derive(Debug, Clone)]
pub struct MockHttpClient {
    responses: Arc<RwLock<HashMap<String, MockResponse>>>,
    request_log: Arc<Mutex<Vec<MockRequest>>>,
    delay_simulation: Arc<RwLock<Option<Duration>>>,
    failure_rate: Arc<RwLock<f64>>,
}

#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub delay: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct MockRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timestamp: Instant,
}

impl MockHttpClient {
    /// Create a new mock HTTP client
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            request_log: Arc::new(Mutex::new(Vec::new())),
            delay_simulation: Arc::new(RwLock::new(None)),
            failure_rate: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Set up a mock response for a specific URL pattern
    pub fn expect_request(&self, url_pattern: &str) -> MockResponseBuilder {
        MockResponseBuilder::new(self.clone(), url_pattern.to_string())
    }

    /// Set global network delay simulation
    pub fn set_network_delay(&self, delay: Duration) {
        *self.delay_simulation.write().unwrap() = Some(delay);
    }

    /// Set failure rate (0.0 to 1.0) for simulating network issues
    pub fn set_failure_rate(&self, rate: f64) {
        *self.failure_rate.write().unwrap() = rate.clamp(0.0, 1.0);
    }

    /// Get all logged requests for verification
    pub fn get_requests(&self) -> Vec<MockRequest> {
        self.request_log.lock().unwrap().clone()
    }

    /// Clear request history
    pub fn clear_history(&self) {
        self.request_log.lock().unwrap().clear();
    }

    /// Simulate making an HTTP request
    pub async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    ) -> Result<MockResponse, String> {
        // Log the request
        let request = MockRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            body,
            timestamp: Instant::now(),
        };
        self.request_log.lock().unwrap().push(request);

        // Simulate network delay
        if let Some(delay) = *self.delay_simulation.read().unwrap() {
            tokio::time::sleep(delay).await;
        }

        // Simulate random failures
        let failure_rate = *self.failure_rate.read().unwrap();
        if rand::random::<f64>() < failure_rate {
            return Err("Simulated network failure".to_string());
        }

        // Find matching response
        let responses = self.responses.read().unwrap();
        for (pattern, response) in responses.iter() {
            if url.contains(pattern) {
                // Simulate response-specific delay
                if let Some(delay) = response.delay {
                    tokio::time::sleep(delay).await;
                }
                return Ok(response.clone());
            }
        }

        Err(format!("No mock response configured for URL: {}", url))
    }
}

impl Default for MockHttpClient { fn default() -> Self { Self::new() } }

/// Builder for mock HTTP responses
pub struct MockResponseBuilder {
    client: MockHttpClient,
    url_pattern: String,
    status: u16,
    body: String,
    headers: HashMap<String, String>,
    delay: Option<Duration>,
}

impl MockResponseBuilder {
    fn new(client: MockHttpClient, url_pattern: String) -> Self {
        Self {
            client,
            url_pattern,
            status: 200,
            body: String::new(),
            headers: HashMap::new(),
            delay: None,
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_json_body<T: serde::Serialize>(mut self, data: &T) -> Self {
        self.body = serde_json::to_string(data).unwrap();
        self.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn build(self) {
        let response = MockResponse {
            status: self.status,
            body: self.body,
            headers: self.headers,
            delay: self.delay,
        };

        self.client
            .responses
            .write()
            .unwrap()
            .insert(self.url_pattern, response);
    }
}

/// Mock database for testing data persistence
#[derive(Debug, Clone)]
pub struct MockDatabase {
    data: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    operation_log: Arc<Mutex<Vec<DatabaseOperation>>>,
    latency_simulation: Arc<RwLock<Option<Duration>>>,
    failure_scenarios: Arc<RwLock<Vec<FailureScenario>>>,
}

#[derive(Debug, Clone)]
pub struct DatabaseOperation {
    pub operation_type: DatabaseOperationType,
    pub table: String,
    pub key: Option<String>,
    pub timestamp: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DatabaseOperationType {
    Insert,
    Update,
    Delete,
    Select,
    CreateTable,
    DropTable,
}

#[derive(Debug, Clone)]
pub struct FailureScenario {
    pub operation_pattern: String,
    pub failure_rate: f64,
    pub error_message: String,
}

impl MockDatabase {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            operation_log: Arc::new(Mutex::new(Vec::new())),
            latency_simulation: Arc::new(RwLock::new(None)),
            failure_scenarios: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set database operation latency simulation
    pub fn set_latency(&self, latency: Duration) {
        *self.latency_simulation.write().unwrap() = Some(latency);
    }

    /// Add failure scenario for specific operations
    pub fn add_failure_scenario(&self, pattern: &str, failure_rate: f64, error: &str) {
        let scenario = FailureScenario {
            operation_pattern: pattern.to_string(),
            failure_rate,
            error_message: error.to_string(),
        };
        self.failure_scenarios.write().unwrap().push(scenario);
    }

    /// Simulate database insert operation
    pub async fn insert(
        &self,
        table: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        let start = Instant::now();
        self.simulate_operation_delay().await;
        self.check_failure_scenarios("insert", table).await?;

        self.data
            .write()
            .unwrap()
            .insert(format!("{}:{}", table, key), value);

        self.log_operation(DatabaseOperation {
            operation_type: DatabaseOperationType::Insert,
            table: table.to_string(),
            key: Some(key.to_string()),
            timestamp: start,
            duration: start.elapsed(),
        });

        Ok(())
    }

    /// Simulate database select operation
    pub async fn select(
        &self,
        table: &str,
        key: &str,
    ) -> Result<Option<serde_json::Value>, String> {
        let start = Instant::now();
        self.simulate_operation_delay().await;
        self.check_failure_scenarios("select", table).await?;

        let result = self
            .data
            .read()
            .unwrap()
            .get(&format!("{}:{}", table, key))
            .cloned();

        self.log_operation(DatabaseOperation {
            operation_type: DatabaseOperationType::Select,
            table: table.to_string(),
            key: Some(key.to_string()),
            timestamp: start,
            duration: start.elapsed(),
        });

        Ok(result)
    }

    /// Simulate database update operation
    pub async fn update(
        &self,
        table: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<bool, String> {
        let start = Instant::now();
        self.simulate_operation_delay().await;
        self.check_failure_scenarios("update", table).await?;

        let full_key = format!("{}:{}", table, key);
        let existed = self.data.read().unwrap().contains_key(&full_key);

        if existed {
            self.data.write().unwrap().insert(full_key, value);
        }

        self.log_operation(DatabaseOperation {
            operation_type: DatabaseOperationType::Update,
            table: table.to_string(),
            key: Some(key.to_string()),
            timestamp: start,
            duration: start.elapsed(),
        });

        Ok(existed)
    }

    /// Simulate database delete operation
    pub async fn delete(&self, table: &str, key: &str) -> Result<bool, String> {
        let start = Instant::now();
        self.simulate_operation_delay().await;
        self.check_failure_scenarios("delete", table).await?;

        let full_key = format!("{}:{}", table, key);
        let existed = self.data.write().unwrap().remove(&full_key).is_some();

        self.log_operation(DatabaseOperation {
            operation_type: DatabaseOperationType::Delete,
            table: table.to_string(),
            key: Some(key.to_string()),
            timestamp: start,
            duration: start.elapsed(),
        });

        Ok(existed)
    }

    /// Get all logged operations
    pub fn get_operations(&self) -> Vec<DatabaseOperation> {
        self.operation_log.lock().unwrap().clone()
    }

    /// Get operation statistics
    pub fn get_operation_stats(&self) -> DatabaseStats {
        let ops = self.operation_log.lock().unwrap();
        let total_ops = ops.len();

        if total_ops == 0 {
            return DatabaseStats::default();
        }

        let avg_duration = ops.iter().map(|op| op.duration).sum::<Duration>() / total_ops as u32;

        let operations_by_type = ops.iter().fold(HashMap::new(), |mut acc, op| {
            *acc.entry(op.operation_type.clone()).or_insert(0) += 1;
            acc
        });

        DatabaseStats {
            total_operations: total_ops,
            average_duration: avg_duration,
            operations_by_type,
        }
    }

    async fn simulate_operation_delay(&self) {
        let latency = *self.latency_simulation.read().unwrap();
        if let Some(latency) = latency { tokio::time::sleep(latency).await; }
    }

    async fn check_failure_scenarios(&self, operation: &str, table: &str) -> Result<(), String> {
        let scenarios: Vec<FailureScenario> = self.failure_scenarios.read().unwrap().clone();
        for scenario in scenarios.iter() {
            let pattern = &scenario.operation_pattern;
            if (pattern == operation || pattern == "all" || pattern == table)
                && rand::random::<f64>() < scenario.failure_rate
            {
                return Err(scenario.error_message.clone());
            }
        }
        Ok(())
    }

    fn log_operation(&self, operation: DatabaseOperation) {
        self.operation_log.lock().unwrap().push(operation);
    }
}

impl Default for MockDatabase { fn default() -> Self { Self::new() } }

#[derive(Debug, Default)]
pub struct DatabaseStats {
    pub total_operations: usize,
    pub average_duration: Duration,
    pub operations_by_type: HashMap<DatabaseOperationType, usize>,
}

/// Mock event system for testing async communication
#[derive(Debug)]
pub struct MockEventSystem {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<MockEvent>>>>,
    event_log: Arc<Mutex<Vec<MockEvent>>>,
    subscribers: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

#[derive(Debug, Clone)]
pub struct MockEvent {
    pub id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: Instant,
    pub source: String,
}

impl MockEventSystem {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            event_log: Arc::new(Mutex::new(Vec::new())),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new event channel
    pub fn create_channel(&self, name: &str) -> broadcast::Receiver<MockEvent> {
        let (tx, rx) = broadcast::channel(1000);
        self.channels.write().unwrap().insert(name.to_string(), tx);
        rx
    }

    /// Publish event to a channel
    pub async fn publish(
        &self,
        channel: &str,
        event_type: &str,
        payload: serde_json::Value,
        source: &str,
    ) -> Result<(), String> {
        let event = MockEvent {
            id: Uuid::new_v4(),
            event_type: event_type.to_string(),
            payload,
            timestamp: Instant::now(),
            source: source.to_string(),
        };

        // Log the event
        self.event_log.lock().unwrap().push(event.clone());

        // Send to subscribers
        if let Some(tx) = self.channels.read().unwrap().get(channel) {
            tx.send(event)
                .map_err(|e| format!("Failed to send event: {}", e))?;
        } else {
            return Err(format!("Channel '{}' not found", channel));
        }

        Ok(())
    }

    /// Subscribe to a channel
    pub fn subscribe(&self, channel: &str, subscriber_id: &str) -> broadcast::Receiver<MockEvent> {
        self.subscribers
            .lock()
            .unwrap()
            .entry(channel.to_string())
            .or_default()
            .push(subscriber_id.to_string());

        self.create_channel(channel)
    }

    /// Get all events from log
    pub fn get_events(&self) -> Vec<MockEvent> {
        self.event_log.lock().unwrap().clone()
    }

    /// Get events by type
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<MockEvent> {
        self.event_log
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Get subscriber count for channel
    pub fn get_subscriber_count(&self, channel: &str) -> usize {
        self.subscribers
            .lock()
            .unwrap()
            .get(channel)
            .map(|subs| subs.len())
            .unwrap_or(0)
    }
}

impl Default for MockEventSystem { fn default() -> Self { Self::new() } }

/// Test data generators for realistic test scenarios
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate realistic text data of various lengths and content types
    pub fn generate_text_samples(count: usize) -> Vec<String> {
        let templates = [
            "User query: {}",
            "Technical documentation about {} and {} integration",
            "Error message: Failed to process {} due to {} constraints",
            "Performance report: {} operations completed in {} milliseconds",
            "Configuration update: {} parameter changed from {} to {}",
        ];

        let words = vec![
            "system",
            "database",
            "network",
            "authentication",
            "authorization",
            "memory",
            "cpu",
            "storage",
            "cache",
            "index",
            "query",
            "transaction",
            "microservice",
            "api",
            "endpoint",
            "middleware",
            "pipeline",
            "queue",
            "embedding",
            "tokenization",
            "model",
            "inference",
            "training",
            "optimization",
        ];

        (0..count)
            .map(|i| {
                let template = &templates[i % templates.len()];
                let word1 = words[thread_rng().gen_range(0..words.len())];
                let word2 = words[thread_rng().gen_range(0..words.len())];
                let value = thread_rng().gen_range(1..10000);

                match i % 5 {
                    0 => template.replace("{}", word1),
                    1 => {
                        let mut result = template.replace("{}", word1);
                        if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, word2);
                        }
                        result
                    }
                    2 => {
                        let mut result = template.replace("{}", word1);
                        if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, word2);
                        }
                        result
                    }
                    3 => {
                        let mut result = template.replace("{}", &value.to_string());
                        if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, &(value * 10).to_string());
                        }
                        result
                    }
                    4 => {
                        let mut result = template.replace("{}", word1);
                        if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, &value.to_string());
                        }
                        if let Some(pos) = result.find("{}") {
                            result.replace_range(pos..pos + 2, &(value + 1).to_string());
                        }
                        result
                    }
                    _ => unreachable!(),
                }
            })
            .collect()
    }

    /// Generate realistic vector embeddings for testing
    pub fn generate_embeddings(count: usize, dimensions: usize) -> Vec<Vec<f32>> {
        (0..count)
            .map(|_| {
                (0..dimensions)
                    .map(|_| thread_rng().gen::<f32>() * 2.0 - 1.0) // Range [-1, 1]
                    .collect()
            })
            .collect()
    }

    /// Generate test configuration objects
    pub fn generate_configs<T>() -> Vec<T>
    where
        T: Default + Clone,
    {
        // In a real implementation, this would generate various config permutations
        vec![T::default(); 5]
    }

    /// Generate realistic error scenarios for testing
    pub fn generate_error_scenarios() -> Vec<ErrorScenario> {
        vec![
            ErrorScenario {
                name: "network_timeout".to_string(),
                description: "Network request timeout after 30 seconds".to_string(),
                expected_error_type: "TimeoutError".to_string(),
                trigger_condition: "network_delay > 30s".to_string(),
            },
            ErrorScenario {
                name: "memory_exhaustion".to_string(),
                description: "Out of memory when processing large batch".to_string(),
                expected_error_type: "OutOfMemoryError".to_string(),
                trigger_condition: "batch_size > memory_limit".to_string(),
            },
            ErrorScenario {
                name: "invalid_input".to_string(),
                description: "Invalid input data format".to_string(),
                expected_error_type: "ValidationError".to_string(),
                trigger_condition: "input validation fails".to_string(),
            },
            ErrorScenario {
                name: "concurrent_access".to_string(),
                description: "Race condition during concurrent access".to_string(),
                expected_error_type: "ConcurrencyError".to_string(),
                trigger_condition: "multiple writers to same resource".to_string(),
            },
            ErrorScenario {
                name: "resource_unavailable".to_string(),
                description: "Required resource is temporarily unavailable".to_string(),
                expected_error_type: "ResourceUnavailableError".to_string(),
                trigger_condition: "resource locked or busy".to_string(),
            },
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ErrorScenario {
    pub name: String,
    pub description: String,
    pub expected_error_type: String,
    pub trigger_condition: String,
}

/// Mock metrics collector for testing observability
#[derive(Debug)]
pub struct MockMetricsCollector {
    metrics: Arc<RwLock<HashMap<String, MetricValue>>>,
    history: Arc<Mutex<Vec<MetricEvent>>>,
}

#[derive(Debug, Clone)]
pub struct MetricEvent {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: Instant,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Timer(Duration),
}

impl MockMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn increment_counter(&self, name: &str, value: u64, labels: HashMap<String, String>) {
        let metric_value = MetricValue::Counter(value);
        self.record_metric(name, metric_value, labels);
    }

    pub fn set_gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        let metric_value = MetricValue::Gauge(value);
        self.record_metric(name, metric_value, labels);
    }

    pub fn record_histogram(&self, name: &str, values: Vec<f64>, labels: HashMap<String, String>) {
        let metric_value = MetricValue::Histogram(values);
        self.record_metric(name, metric_value, labels);
    }

    pub fn record_timer(&self, name: &str, duration: Duration, labels: HashMap<String, String>) {
        let metric_value = MetricValue::Timer(duration);
        self.record_metric(name, metric_value, labels);
    }

    fn record_metric(&self, name: &str, value: MetricValue, labels: HashMap<String, String>) {
        // Update current metrics
        self.metrics
            .write()
            .unwrap()
            .insert(name.to_string(), value.clone());

        // Add to history
        let event = MetricEvent {
            name: name.to_string(),
            value,
            timestamp: Instant::now(),
            labels,
        };
        self.history.lock().unwrap().push(event);
    }

    pub fn get_metric(&self, name: &str) -> Option<MetricValue> {
        self.metrics.read().unwrap().get(name).cloned()
    }

    pub fn get_history(&self) -> Vec<MetricEvent> {
        self.history.lock().unwrap().clone()
    }

    pub fn get_metrics_by_label(&self, label_key: &str, label_value: &str) -> Vec<MetricEvent> {
        self.history
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event.labels.get(label_key) == Some(&label_value.to_string()))
            .cloned()
            .collect()
    }
}

impl Default for MockMetricsCollector { fn default() -> Self { Self::new() } }

// Common test utilities
/// Test timing utility for performance assertions
pub struct TestTimer {
    start: Instant,
    name: String,
}

impl TestTimer {
    pub fn start(name: &str) -> Self {
        Self {
            start: Instant::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn assert_faster_than(&self, max_duration: Duration) {
        let elapsed = self.elapsed();
        assert!(
            elapsed < max_duration,
            "{} took {:?}, expected faster than {:?}",
            self.name,
            elapsed,
            max_duration
        );
    }

    pub fn assert_slower_than(&self, min_duration: Duration) {
        let elapsed = self.elapsed();
        assert!(
            elapsed > min_duration,
            "{} took {:?}, expected slower than {:?}",
            self.name,
            elapsed,
            min_duration
        );
    }
}

/// Memory usage tracker for testing memory efficiency
pub struct MemoryTracker {
    initial_usage: usize,
    name: String,
}

impl MemoryTracker {
    pub fn start(name: &str) -> Self {
        Self {
            initial_usage: get_memory_usage(),
            name: name.to_string(),
        }
    }

    pub fn current_usage(&self) -> usize {
        get_memory_usage()
    }

    pub fn memory_delta(&self) -> isize {
        self.current_usage() as isize - self.initial_usage as isize
    }

    pub fn assert_memory_increase_less_than(&self, max_increase: usize) {
        let delta = self.memory_delta();
        assert!(
            delta >= 0 && (delta as usize) < max_increase,
            "{} increased memory by {} bytes, expected less than {}",
            self.name,
            delta,
            max_increase
        );
    }
}

fn get_memory_usage() -> usize {
    // Simplified memory tracking - in real implementation would use proper system calls
    // For now, return a mock value
    thread_rng().gen_range(1024..1024 * 1024)
}
