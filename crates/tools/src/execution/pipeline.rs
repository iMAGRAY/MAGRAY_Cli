// @component: {"k":"C","id":"execution_pipeline_v2","t":"Improved tool execution pipeline with resource management","m":{"cur":0,"tgt":95,"u":"%"},"f":["execution","pipeline","resource","circuit-breaker"]}

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

use super::resource_manager::{ResourceGuard, ResourceLimits, ResourceManager};
use crate::intelligent_selector::{IntelligentToolSelector, ToolConfidence, ToolSelectionContext};
use crate::registry::{SecurityLevel, ToolMetadata};
use crate::{Tool, ToolInput, ToolOutput};

/// Execution context with security and resource information
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub user_query: String,
    pub session_id: String,
    pub user_id: String,
    pub security_level: SecurityLevel,
    pub resource_limits: ResourceLimits,
    pub metadata: HashMap<String, String>,
}

/// Tool execution result with comprehensive metrics
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: ToolOutput,
    pub execution_time: Duration,
    pub attempt_count: u32,
    pub tool_id: String,
    pub strategy_used: ExecutionStrategy,
    pub resource_usage: Option<ResourceUsage>,
    pub security_events: Vec<SecurityEvent>,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_peak_mb: u64,
    pub cpu_time_ms: u64,
    pub disk_operations: u64,
    pub network_requests: u64,
}

#[derive(Debug, Clone)]
pub struct SecurityEvent {
    pub timestamp: Instant,
    pub event_type: SecurityEventType,
    pub description: String,
    pub severity: SecuritySeverity,
}

#[derive(Debug, Clone)]
pub enum SecurityEventType {
    PermissionCheck,
    ResourceLimit,
    InputValidation,
    OutputFiltering,
    Quarantine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SecuritySeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Execution strategies for different scenarios
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStrategy {
    /// Direct execution without retries
    Direct,
    /// Retry with exponential backoff  
    RetryWithBackoff,
    /// Parallel execution with fastest-wins
    ParallelFastest,
    /// Sequential fallback through alternatives
    SequentialFallback,
    /// Circuit breaker protected execution
    CircuitBreakerProtected,
    /// Resource-aware execution with throttling
    ResourceThrottled,
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout: Duration,
    pub success_threshold: u32,
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 2,
            timeout: Duration::from_secs(30),
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Circuit breaker metrics per tool
#[derive(Debug)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
    pub total_requests: u64,
    pub total_failures: u64,
}

impl Default for CircuitBreakerMetrics {
    fn default() -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            total_requests: 0,
            total_failures: 0,
        }
    }
}

/// Pipeline execution metrics
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub retried_executions: u64,
    pub circuit_breaker_trips: u64,
    pub average_execution_time: Duration,
    pub resource_violations: u64,
    pub security_violations: u64,
}

/// Improved execution pipeline with resource management
pub struct ExecutionPipeline {
    /// Available tools registry
    tools: Arc<RwLock<HashMap<String, (Arc<dyn Tool>, ToolMetadata)>>>,

    /// Resource manager for execution control
    resource_manager: Arc<ResourceManager>,

    /// Intelligent tool selector
    tool_selector: Arc<IntelligentToolSelector>,

    /// Circuit breaker metrics per tool
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreakerMetrics>>>,

    /// Circuit breaker configuration
    circuit_config: CircuitBreakerConfig,

    /// Retry configuration
    retry_config: RetryConfig,

    /// Execution metrics
    execution_metrics: Arc<Mutex<PipelineMetrics>>,
}

impl ExecutionPipeline {
    pub fn new(
        resource_limits: ResourceLimits,
        tool_selector: Arc<IntelligentToolSelector>,
    ) -> Self {
        info!("🔧 Initializing Enhanced Execution Pipeline");

        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            resource_manager: Arc::new(ResourceManager::new(resource_limits)),
            tool_selector,
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            circuit_config: CircuitBreakerConfig::default(),
            retry_config: RetryConfig::default(),
            execution_metrics: Arc::new(Mutex::new(PipelineMetrics::default())),
        }
    }

    /// Register a tool with the pipeline
    pub async fn register_tool(&self, tool: Arc<dyn Tool>, metadata: ToolMetadata) {
        let tool_id = metadata.id.clone();

        // Register with pipeline
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_id.clone(), (tool, metadata.clone()));
        }

        // Register with intelligent selector
        let tool_spec = crate::ToolSpec {
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            usage: format!("{} v{}", metadata.name, metadata.version),
            examples: metadata
                .examples
                .iter()
                .map(|e| e.description.clone())
                .collect(),
            input_schema: metadata.input_schema.to_string(),
            usage_guide: None,
            permissions: None,
            supports_dry_run: false,
        };
        let mut spec_with_guide = tool_spec.clone();
        spec_with_guide.usage_guide = Some(crate::generate_usage_guide(&tool_spec));
        self.tool_selector.register_tool(spec_with_guide).await;

        // Initialize circuit breaker
        {
            let mut breakers = self.circuit_breakers.lock().await;
            breakers.insert(tool_id.clone(), CircuitBreakerMetrics::default());
        }

        debug!("🔧 Registered tool with pipeline: {}", tool_id);
    }

    /// Execute a tool with full pipeline features
    pub async fn execute_tool(
        &self,
        tool_id: &str,
        input: ToolInput,
        context: ExecutionContext,
        strategy: ExecutionStrategy,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Update metrics
        {
            let mut metrics = self.execution_metrics.lock().await;
            metrics.total_executions += 1;
        }

        debug!("🚀 Executing tool {} with strategy {:?}", tool_id, strategy);

        // Get tool and metadata
        let (tool, metadata) = {
            let tools = self.tools.read().await;
            let (tool, metadata) = tools
                .get(tool_id)
                .ok_or_else(|| anyhow!("Tool not found: {}", tool_id))?;
            (Arc::clone(tool), metadata.clone())
        };

        // Validate security permissions
        self.validate_security_permissions(&metadata, &context)?;

        // Execute based on strategy
        let result = match strategy {
            ExecutionStrategy::Direct => {
                self.execute_direct(tool, &metadata, input, &context).await
            }
            ExecutionStrategy::RetryWithBackoff => {
                self.execute_with_retry(tool, &metadata, input, &context)
                    .await
            }
            ExecutionStrategy::ResourceThrottled => {
                self.execute_with_resource_throttling(tool, &metadata, input, &context)
                    .await
            }
            ExecutionStrategy::CircuitBreakerProtected => {
                self.execute_with_circuit_breaker(tool, &metadata, input, &context)
                    .await
            }
            _ => {
                // Fallback to direct execution for other strategies
                self.execute_direct(tool, &metadata, input, &context).await
            }
        };

        let execution_time = start_time.elapsed();

        // Update metrics and performance tracking
        self.update_execution_metrics(&result, execution_time).await;

        result
    }

    /// Execute tool using intelligent selection
    pub async fn execute_with_selection(
        &self,
        context: ExecutionContext,
        strategy: ExecutionStrategy,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        debug!(
            "🧠 Using intelligent tool selection for query: '{}'",
            context.user_query
        );

        // Create selection context
        let selection_context = ToolSelectionContext {
            user_query: context.user_query.clone(),
            session_context: context.metadata.clone(),
            previous_tools_used: Vec::new(),
            task_complexity: crate::intelligent_selector::TaskComplexity::Medium,
            urgency_level: crate::intelligent_selector::UrgencyLevel::Normal,
            user_expertise: crate::intelligent_selector::UserExpertise::Intermediate,
        };

        // Select best tool candidates
        let tool_candidates = self.tool_selector.select_tools(&selection_context).await?;

        if tool_candidates.is_empty() {
            return Err(anyhow!("No suitable tools found for query"));
        }

        // Execute based on strategy
        let result = match strategy {
            ExecutionStrategy::ParallelFastest => {
                self.execute_parallel_fastest(&tool_candidates, context)
                    .await
            }
            ExecutionStrategy::SequentialFallback => {
                self.execute_sequential_fallback(&tool_candidates, context)
                    .await
            }
            _ => {
                // Use first candidate for other strategies
                let tool_input = self.create_tool_input(&tool_candidates[0], &selection_context)?;
                self.execute_tool(&tool_candidates[0].tool_name, tool_input, context, strategy)
                    .await
            }
        };

        let execution_time = start_time.elapsed();

        // Update tool selector performance metrics
        if let Ok(ref exec_result) = result {
            self.tool_selector
                .update_tool_performance(
                    &exec_result.tool_id,
                    true,
                    execution_time,
                    Some(0.8), // Default satisfaction score
                )
                .await;
        }

        result
    }

    /// Direct execution with resource management
    async fn execute_direct(
        &self,
        tool: Arc<dyn Tool>,
        metadata: &ToolMetadata,
        input: ToolInput,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        let mut security_events = Vec::new();

        // Allocate resources
        let resource_guard = self.allocate_execution_resources(metadata, context).await?;

        // Execute tool with monitoring
        let execution_future = async {
            // Record execution start
            resource_guard
                .record_usage(super::resource_manager::ResourceUsage::default())
                .await;

            // Execute tool
            let result = tool.execute(input).await;

            // Record final resource usage
            let usage = super::resource_manager::ResourceUsage {
                memory_mb: resource_guard.memory_mb(),
                cpu_percent: 0.0, // Would be measured in real implementation
                execution_time: start_time.elapsed(),
                ..Default::default()
            };
            resource_guard.record_usage(usage).await;

            result
        };

        // Execute with timeout
        let timeout_duration = metadata
            .resource_requirements
            .max_execution_time
            .unwrap_or(Duration::from_secs(60));

        let output = match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(result) => result?,
            Err(_) => {
                security_events.push(SecurityEvent {
                    timestamp: Instant::now(),
                    event_type: SecurityEventType::ResourceLimit,
                    description: "Tool execution timed out".to_string(),
                    severity: SecuritySeverity::Warning,
                });
                return Err(anyhow!(
                    "Tool execution timed out after {:?}",
                    timeout_duration
                ));
            }
        };

        Ok(ExecutionResult {
            output,
            execution_time: start_time.elapsed(),
            attempt_count: 1,
            tool_id: metadata.id.clone(),
            strategy_used: ExecutionStrategy::Direct,
            resource_usage: Some(ResourceUsage {
                memory_peak_mb: resource_guard.memory_mb(),
                cpu_time_ms: start_time.elapsed().as_millis() as u64,
                disk_operations: 0,
                network_requests: 0,
            }),
            security_events,
        })
    }

    /// Execute with exponential backoff retry
    async fn execute_with_retry(
        &self,
        tool: Arc<dyn Tool>,
        metadata: &ToolMetadata,
        input: ToolInput,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let mut last_error = None;

        for attempt in 1..=self.retry_config.max_attempts {
            match self
                .execute_direct(Arc::clone(&tool), metadata, input.clone(), context)
                .await
            {
                Ok(mut result) => {
                    result.attempt_count = attempt;
                    result.strategy_used = ExecutionStrategy::RetryWithBackoff;

                    if attempt > 1 {
                        let mut metrics = self.execution_metrics.lock().await;
                        metrics.retried_executions += 1;
                    }

                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.retry_config.max_attempts {
                        let delay = self.calculate_retry_delay(attempt);
                        debug!("🔄 Retry attempt {} failed, waiting {:?}", attempt, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }

    /// Execute with resource throttling
    async fn execute_with_resource_throttling(
        &self,
        tool: Arc<dyn Tool>,
        metadata: &ToolMetadata,
        input: ToolInput,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        // Check system resource pressure
        let stats = self.resource_manager.get_resource_stats().await;

        if stats.is_under_pressure() {
            warn!("🚨 System under resource pressure, throttling execution");

            // Add delay based on pressure level
            let (memory_util, cpu_util) = stats.utilization_percent();
            let max_util = memory_util.max(cpu_util);

            let throttle_delay = if max_util > 95.0 {
                Duration::from_secs(5)
            } else if max_util > 85.0 {
                Duration::from_secs(2)
            } else {
                Duration::from_secs(1)
            };

            tokio::time::sleep(throttle_delay).await;
        }

        // Execute with resource monitoring
        let mut result = self.execute_direct(tool, metadata, input, context).await?;
        result.strategy_used = ExecutionStrategy::ResourceThrottled;

        Ok(result)
    }

    /// Execute with circuit breaker protection
    async fn execute_with_circuit_breaker(
        &self,
        tool: Arc<dyn Tool>,
        metadata: &ToolMetadata,
        input: ToolInput,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult> {
        let tool_id = &metadata.id;

        // Check circuit breaker state
        {
            let mut breakers = self.circuit_breakers.lock().await;
            let breaker = breakers
                .entry(tool_id.clone())
                .or_insert_with(CircuitBreakerMetrics::default);

            breaker.total_requests += 1;

            match breaker.state {
                CircuitBreakerState::Open => {
                    // Check if we should try half-open
                    if let Some(last_failure) = breaker.last_failure_time {
                        if last_failure.elapsed() > self.circuit_config.recovery_timeout {
                            breaker.state = CircuitBreakerState::HalfOpen;
                            breaker.success_count = 0;
                            info!("🔄 Circuit breaker for {} moved to half-open", tool_id);
                        } else {
                            return Err(anyhow!("Circuit breaker is open for tool: {}", tool_id));
                        }
                    }
                }
                _ => {}
            }
        }

        // Execute with monitoring
        match self.execute_direct(tool, metadata, input, context).await {
            Ok(mut result) => {
                // Success - update circuit breaker
                {
                    let mut breakers = self.circuit_breakers.lock().await;
                    if let Some(breaker) = breakers.get_mut(tool_id) {
                        match breaker.state {
                            CircuitBreakerState::HalfOpen => {
                                breaker.success_count += 1;
                                if breaker.success_count >= self.circuit_config.success_threshold {
                                    breaker.state = CircuitBreakerState::Closed;
                                    breaker.failure_count = 0;
                                    info!("✅ Circuit breaker for {} closed", tool_id);
                                }
                            }
                            CircuitBreakerState::Closed => {
                                breaker.failure_count = 0; // Reset on success
                            }
                            _ => {}
                        }
                    }
                }

                result.strategy_used = ExecutionStrategy::CircuitBreakerProtected;
                Ok(result)
            }
            Err(e) => {
                // Failure - update circuit breaker
                {
                    let mut breakers = self.circuit_breakers.lock().await;
                    if let Some(breaker) = breakers.get_mut(tool_id) {
                        breaker.failure_count += 1;
                        breaker.total_failures += 1;
                        breaker.last_failure_time = Some(Instant::now());

                        if breaker.failure_count >= self.circuit_config.failure_threshold {
                            breaker.state = CircuitBreakerState::Open;

                            let mut metrics = self.execution_metrics.lock().await;
                            metrics.circuit_breaker_trips += 1;

                            error!("🚨 Circuit breaker opened for tool: {}", tool_id);
                        }
                    }
                }

                Err(e)
            }
        }
    }

    /// Execute multiple tools in parallel, return fastest success
    async fn execute_parallel_fastest(
        &self,
        candidates: &[ToolConfidence],
        context: ExecutionContext,
    ) -> Result<ExecutionResult> {
        use tokio::select;

        let top_candidates: Vec<_> = candidates.iter().take(3).collect();

        if top_candidates.is_empty() {
            return Err(anyhow!("No candidates for parallel execution"));
        }

        // Create execution futures
        let mut futures = Vec::new();
        for candidate in &top_candidates {
            let selection_context = ToolSelectionContext {
                user_query: context.user_query.clone(),
                session_context: context.metadata.clone(),
                previous_tools_used: Vec::new(),
                task_complexity: crate::intelligent_selector::TaskComplexity::Medium,
                urgency_level: crate::intelligent_selector::UrgencyLevel::Normal,
                user_expertise: crate::intelligent_selector::UserExpertise::Intermediate,
            };

            if let Ok(tool_input) = self.create_tool_input(candidate, &selection_context) {
                let future = self.execute_tool(
                    &candidate.tool_name,
                    tool_input,
                    context.clone(),
                    ExecutionStrategy::Direct,
                );
                futures.push(Box::pin(future));
            }
        }

        if futures.is_empty() {
            return Err(anyhow!("No valid futures for parallel execution"));
        }

        // Wait for first success
        let result = match futures.len() {
            1 => futures.into_iter().next().unwrap().await,
            2 => {
                let mut iter = futures.into_iter();
                let f1 = iter.next().unwrap();
                let f2 = iter.next().unwrap();
                select! {
                    result = f1 => result,
                    result = f2 => result,
                }
            }
            3 => {
                let mut iter = futures.into_iter();
                let f1 = iter.next().unwrap();
                let f2 = iter.next().unwrap();
                let f3 = iter.next().unwrap();
                select! {
                    result = f1 => result,
                    result = f2 => result,
                    result = f3 => result,
                }
            }
            _ => return Err(anyhow!("Too many parallel candidates")),
        };

        match result {
            Ok(mut exec_result) => {
                exec_result.strategy_used = ExecutionStrategy::ParallelFastest;
                Ok(exec_result)
            }
            Err(e) => Err(e),
        }
    }

    /// Execute tools sequentially until one succeeds
    async fn execute_sequential_fallback(
        &self,
        candidates: &[ToolConfidence],
        context: ExecutionContext,
    ) -> Result<ExecutionResult> {
        let mut last_error = None;

        for candidate in candidates {
            let selection_context = ToolSelectionContext {
                user_query: context.user_query.clone(),
                session_context: context.metadata.clone(),
                previous_tools_used: Vec::new(),
                task_complexity: crate::intelligent_selector::TaskComplexity::Medium,
                urgency_level: crate::intelligent_selector::UrgencyLevel::Normal,
                user_expertise: crate::intelligent_selector::UserExpertise::Intermediate,
            };

            if let Ok(tool_input) = self.create_tool_input(candidate, &selection_context) {
                match self
                    .execute_tool(
                        &candidate.tool_name,
                        tool_input,
                        context.clone(),
                        ExecutionStrategy::Direct,
                    )
                    .await
                {
                    Ok(mut result) => {
                        result.strategy_used = ExecutionStrategy::SequentialFallback;
                        return Ok(result);
                    }
                    Err(e) => {
                        debug!("Tool {} failed, trying next candidate", candidate.tool_name);
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("All fallback tools failed")))
    }

    /// Validate security permissions for execution
    fn validate_security_permissions(
        &self,
        metadata: &ToolMetadata,
        context: &ExecutionContext,
    ) -> Result<()> {
        // Check if user can execute tools of this security level
        let required_level = &metadata.security_level;
        let user_level = &context.security_level;

        if *required_level > *user_level {
            return Err(anyhow!("Insufficient security level for tool execution"));
        }

        // Additional security checks can be added here

        Ok(())
    }

    /// Allocate resources for tool execution
    async fn allocate_execution_resources(
        &self,
        metadata: &ToolMetadata,
        context: &ExecutionContext,
    ) -> Result<ResourceGuard> {
        let requested_memory = metadata.resource_requirements.max_memory_mb.unwrap_or(512);
        let requested_cores = metadata.resource_requirements.max_cpu_cores.unwrap_or(1);
        let execution_timeout = metadata.resource_requirements.max_execution_time;

        self.resource_manager
            .allocate_resources(
                &metadata.id,
                &context.session_id,
                requested_memory,
                requested_cores,
                execution_timeout,
            )
            .await
    }

    /// Create tool input from intelligent selector candidate
    fn create_tool_input(
        &self,
        candidate: &ToolConfidence,
        context: &ToolSelectionContext,
    ) -> Result<ToolInput> {
        // This is a simplified version - in production, would use more sophisticated parsing
        Ok(ToolInput {
            command: candidate.tool_name.clone(),
            args: HashMap::new(),
            context: Some(context.user_query.clone()),
            dry_run: false,
            timeout_ms: None,
        })
    }

    /// Calculate retry delay with exponential backoff and jitter
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay_ms = self.retry_config.base_delay.as_millis() as f64;
        let backoff_delay = base_delay_ms
            * self
                .retry_config
                .backoff_multiplier
                .powi(attempt as i32 - 1);

        let delay_ms = if self.retry_config.jitter {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.75..=1.25);
            (backoff_delay * jitter_factor) as u64
        } else {
            backoff_delay as u64
        };

        let max_delay_ms = self.retry_config.max_delay.as_millis() as u64;
        Duration::from_millis(delay_ms.min(max_delay_ms))
    }

    /// Update execution metrics
    async fn update_execution_metrics(
        &self,
        result: &Result<ExecutionResult>,
        execution_time: Duration,
    ) {
        let mut metrics = self.execution_metrics.lock().await;

        match result {
            Ok(_) => {
                metrics.successful_executions += 1;
            }
            Err(_) => {
                metrics.failed_executions += 1;
            }
        }

        // Update average execution time
        let total_executions = metrics.total_executions;
        if total_executions > 0 {
            metrics.average_execution_time =
                (metrics.average_execution_time * (total_executions - 1) as u32 + execution_time)
                    / total_executions as u32;
        }
    }

    /// Get comprehensive pipeline statistics
    pub async fn get_pipeline_stats(&self) -> String {
        let metrics = self.execution_metrics.lock().await;
        let resource_stats = self.resource_manager.get_resource_stats().await;
        let breakers = self.circuit_breakers.lock().await;

        let success_rate = if metrics.total_executions > 0 {
            (metrics.successful_executions as f32 / metrics.total_executions as f32) * 100.0
        } else {
            0.0
        };

        let (memory_util, cpu_util) = resource_stats.utilization_percent();

        format!(
            "🔧 Enhanced Execution Pipeline Statistics:\n\n\
             📊 Performance Metrics:\n\
             • Total executions: {}\n\
             • Success rate: {:.1}%\n\
             • Average execution time: {:?}\n\
             • Retried executions: {}\n\
             • Circuit breaker trips: {}\n\n\
             🎯 Resource Utilization:\n\
             • Memory utilization: {:.1}% ({}/{} MB)\n\
             • CPU utilization: {:.1}% ({}/{} cores)\n\
             • Active allocations: {}\n\n\
             🔒 Security & Safety:\n\
             • Resource violations: {}\n\
             • Security violations: {}\n\
             • Circuit breakers open: {}",
            metrics.total_executions,
            success_rate,
            metrics.average_execution_time,
            metrics.retried_executions,
            metrics.circuit_breaker_trips,
            memory_util,
            resource_stats.total_memory_allocated,
            resource_stats.limits.max_memory_mb,
            cpu_util,
            resource_stats.total_cpu_cores_allocated,
            resource_stats.limits.max_cpu_cores,
            resource_stats.total_allocations,
            metrics.resource_violations,
            metrics.security_violations,
            breakers
                .values()
                .filter(|b| b.state == CircuitBreakerState::Open)
                .count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{SemanticVersion, ToolMetadata};

    #[tokio::test]
    async fn test_execution_pipeline_creation() {
        let limits = ResourceLimits::default();
        let selector = Arc::new(IntelligentToolSelector::default());
        let pipeline = ExecutionPipeline::new(limits, selector);

        let stats = pipeline.get_pipeline_stats().await;
        assert!(stats.contains("Enhanced Execution Pipeline Statistics"));
    }

    #[tokio::test]
    async fn test_security_validation() {
        let limits = ResourceLimits::default();
        let selector = Arc::new(IntelligentToolSelector::default());
        let pipeline = ExecutionPipeline::new(limits, selector);

        let metadata = ToolMetadata::new(
            "test".to_string(),
            "Test Tool".to_string(),
            SemanticVersion::new(1, 0, 0),
        );

        let context = ExecutionContext {
            user_query: "test".to_string(),
            session_id: "session1".to_string(),
            user_id: "user1".to_string(),
            security_level: SecurityLevel::Safe,
            resource_limits: ResourceLimits::default(),
            metadata: HashMap::new(),
        };

        // Should pass for Safe level
        assert!(pipeline
            .validate_security_permissions(&metadata, &context)
            .is_ok());
    }
}
