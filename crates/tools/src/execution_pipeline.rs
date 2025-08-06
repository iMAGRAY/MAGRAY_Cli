// @component: {"k":"C","id":"tool_execution_pipeline","t":"Advanced tool execution with retry logic and circuit breaker","m":{"cur":5,"tgt":90,"u":"%"},"f":["pipeline","execution","retry","circuit-breaker","resilience"]}

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn, error};

use crate::{Tool, ToolInput, ToolOutput};
use crate::intelligent_selector::{IntelligentToolSelector, ToolSelectionContext};

/// Tool execution result with detailed metrics
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub output: ToolOutput,
    pub execution_time: Duration,
    pub attempt_count: u32,
    pub tool_name: String,
    pub strategy_used: ExecutionStrategy,
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
    pub failure_threshold: u32,      // Failures before opening
    pub recovery_timeout: Duration,  // Time before trying half-open
    pub success_threshold: u32,      // Successes to close from half-open
    pub timeout: Duration,           // Request timeout
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

/// Circuit breaker state tracking
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

/// Advanced retry configuration
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

/// Tool execution pipeline with advanced features
pub struct ToolExecutionPipeline {
    /// Available tools registry
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
    
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

/// Pipeline execution metrics
#[derive(Debug, Default)]
pub struct PipelineMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub retried_executions: u64,
    pub circuit_breaker_trips: u64,
    pub average_execution_time: Duration,
    pub tool_usage_stats: HashMap<String, ToolUsageStats>,
}

/// Per-tool usage statistics
#[derive(Debug, Default, Clone)]
pub struct ToolUsageStats {
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub retries: u64,
    pub total_time: Duration,
    pub average_time: Duration,
    pub last_used: Option<Instant>,
}

impl ToolExecutionPipeline {
    pub fn new(tool_selector: Arc<IntelligentToolSelector>) -> Self {
        info!("🔧 Initializing Tool Execution Pipeline");
        
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            tool_selector,
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            circuit_config: CircuitBreakerConfig::default(),
            retry_config: RetryConfig::default(),
            execution_metrics: Arc::new(Mutex::new(PipelineMetrics::default())),
        }
    }
    
    /// Register a tool with the pipeline
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) {
        let spec = tool.spec();
        let tool_name = spec.name.clone();
        
        // Register with pipeline
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_name.clone(), tool);
        }
        
        // Register with intelligent selector
        self.tool_selector.register_tool(spec).await;
        
        // Initialize circuit breaker
        {
            let mut breakers = self.circuit_breakers.lock().await;
            breakers.insert(tool_name.clone(), CircuitBreakerMetrics::default());
        }
        
        debug!("🔧 Registered tool: {}", tool_name);
    }
    
    /// Execute a tool with full pipeline features
    pub async fn execute_tool(
        &self,
        context: &ToolSelectionContext,
        strategy: ExecutionStrategy,
    ) -> Result<ExecutionResult> {
        let _start_time = Instant::now();
        
        // Update metrics
        {
            let mut metrics = self.execution_metrics.lock().await;
            metrics.total_executions += 1;
        }
        
        debug!("🚀 Executing tool pipeline for query: '{}'", context.user_query);
        
        // Select best tool(s) using intelligent selector
        let tool_candidates = self.tool_selector.select_tools(context).await?;
        
        if tool_candidates.is_empty() {
            return Err(anyhow!("No suitable tools found for query"));
        }
        
        // Execute based on strategy
        let result = match strategy {
            ExecutionStrategy::Direct => {
                self.execute_direct(&tool_candidates[0].tool_name, context).await
            }
            ExecutionStrategy::RetryWithBackoff => {
                self.execute_with_retry(&tool_candidates[0].tool_name, context).await
            }
            ExecutionStrategy::ParallelFastest => {
                self.execute_parallel_fastest(&tool_candidates, context).await
            }
            ExecutionStrategy::SequentialFallback => {
                self.execute_sequential_fallback(&tool_candidates, context).await
            }
            ExecutionStrategy::CircuitBreakerProtected => {
                self.execute_with_circuit_breaker(&tool_candidates[0].tool_name, context).await
            }
        };
        
        let execution_time = start_time.elapsed();
        
        // Update metrics and tool selector performance
        match &result {
            Ok(exec_result) => {
                self.update_success_metrics(&exec_result.tool_name, execution_time).await;
                self.tool_selector.update_tool_performance(
                    &exec_result.tool_name,
                    true,
                    execution_time,
                    Some(0.8), // Default satisfaction score
                ).await;
                
                let mut metrics = self.execution_metrics.lock().await;
                metrics.successful_executions += 1;
            }
            Err(e) => {
                warn!("Tool execution failed: {}", e);
                if let Some(first_candidate) = tool_candidates.first() {
                    self.update_failure_metrics(&first_candidate.tool_name, execution_time).await;
                    self.tool_selector.update_tool_performance(
                        &first_candidate.tool_name,
                        false,
                        execution_time,
                        Some(0.2), // Low satisfaction for failures
                    ).await;
                }
                
                let mut metrics = self.execution_metrics.lock().await;
                metrics.failed_executions += 1;
            }
        }
        
        result
    }
    
    /// Direct execution without retries
    async fn execute_direct(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        let tool_input = self.create_tool_input(tool_name, context).await?;
        let output = self.execute_tool_internal(tool_name, tool_input).await?;
        
        Ok(ExecutionResult {
            output,
            execution_time: start_time.elapsed(),
            attempt_count: 1,
            tool_name: tool_name.to_string(),
            strategy_used: ExecutionStrategy::Direct,
        })
    }
    
    /// Execute with exponential backoff retry
    async fn execute_with_retry(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Result<ExecutionResult> {
        let _start_time = Instant::now();
        let mut last_error = None;
        
        for attempt in 1..=self.retry_config.max_attempts {
            match self.execute_direct(tool_name, context).await {
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
    
    /// Execute multiple tools in parallel, return fastest success
    async fn execute_parallel_fastest(
        &self,
        candidates: &[crate::intelligent_selector::ToolConfidence],
        context: &ToolSelectionContext,
    ) -> Result<ExecutionResult> {
        use tokio::select;
        
        let _start_time = Instant::now();
        
        // Take top 3 candidates for parallel execution
        let top_candidates: Vec<_> = candidates.iter().take(3).collect();
        
        if top_candidates.is_empty() {
            return Err(anyhow!("No candidates for parallel execution"));
        }
        
        // Create futures for parallel execution
        let mut futures = Vec::new();
        for candidate in &top_candidates {
            let future = self.execute_direct(&candidate.tool_name, context);
            futures.push(Box::pin(future));
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
            _ => unreachable!(),
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
        candidates: &[crate::intelligent_selector::ToolConfidence],
        context: &ToolSelectionContext,
    ) -> Result<ExecutionResult> {
        let _start_time = Instant::now();
        let mut last_error = None;
        
        for candidate in candidates {
            match self.execute_direct(&candidate.tool_name, context).await {
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
        
        Err(last_error.unwrap_or_else(|| anyhow!("All fallback tools failed")))
    }
    
    /// Execute with circuit breaker protection
    async fn execute_with_circuit_breaker(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Result<ExecutionResult> {
        // Check circuit breaker state
        {
            let mut breakers = self.circuit_breakers.lock().await;
            let breaker = breakers.entry(tool_name.to_string())
                .or_insert_with(CircuitBreakerMetrics::default);
            
            breaker.total_requests += 1;
            
            match breaker.state {
                CircuitBreakerState::Open => {
                    // Check if we should try half-open
                    if let Some(last_failure) = breaker.last_failure_time {
                        if last_failure.elapsed() > self.circuit_config.recovery_timeout {
                            breaker.state = CircuitBreakerState::HalfOpen;
                            breaker.success_count = 0;
                            info!("🔄 Circuit breaker for {} moved to half-open", tool_name);
                        } else {
                            return Err(anyhow!("Circuit breaker is open for tool: {}", tool_name));
                        }
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    // Allow limited requests in half-open state
                }
                CircuitBreakerState::Closed => {
                    // Normal operation
                }
            }
        }
        
        // Execute with timeout
        let timeout_duration = self.circuit_config.timeout;
        let execution_future = self.execute_direct(tool_name, context);
        
        match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(Ok(mut result)) => {
                // Success - update circuit breaker
                {
                    let mut breakers = self.circuit_breakers.lock().await;
                    if let Some(breaker) = breakers.get_mut(tool_name) {
                        match breaker.state {
                            CircuitBreakerState::HalfOpen => {
                                breaker.success_count += 1;
                                if breaker.success_count >= self.circuit_config.success_threshold {
                                    breaker.state = CircuitBreakerState::Closed;
                                    breaker.failure_count = 0;
                                    info!("✅ Circuit breaker for {} closed", tool_name);
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
            Ok(Err(e)) => {
                // Failure - update circuit breaker
                {
                    let mut breakers = self.circuit_breakers.lock().await;
                    if let Some(breaker) = breakers.get_mut(tool_name) {
                        breaker.failure_count += 1;
                        breaker.total_failures += 1;
                        breaker.last_failure_time = Some(Instant::now());
                        
                        if breaker.failure_count >= self.circuit_config.failure_threshold {
                            breaker.state = CircuitBreakerState::Open;
                            
                            let mut metrics = self.execution_metrics.lock().await;
                            metrics.circuit_breaker_trips += 1;
                            
                            error!("🚨 Circuit breaker opened for tool: {}", tool_name);
                        }
                    }
                }
                
                Err(anyhow!("Circuit breaker execution failed for tool: {} - {}", tool_name, e))
            }
            Err(_) => {
                // Timeout - update circuit breaker
                {
                    let mut breakers = self.circuit_breakers.lock().await;
                    if let Some(breaker) = breakers.get_mut(tool_name) {
                        breaker.failure_count += 1;
                        breaker.total_failures += 1;
                        breaker.last_failure_time = Some(Instant::now());
                        
                        if breaker.failure_count >= self.circuit_config.failure_threshold {
                            breaker.state = CircuitBreakerState::Open;
                            
                            let mut metrics = self.execution_metrics.lock().await;
                            metrics.circuit_breaker_trips += 1;
                            
                            error!("🚨 Circuit breaker opened for tool: {}", tool_name);
                        }
                    }
                }
                
                Err(anyhow!("Circuit breaker execution failed for tool: {}", tool_name))
            }
        }
    }
    
    /// Internal tool execution
    async fn execute_tool_internal(
        &self,
        tool_name: &str,
        input: ToolInput,
    ) -> Result<ToolOutput> {
        let tools = self.tools.read().await;
        let tool = tools.get(tool_name)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;
        
        tool.execute(input).await
    }
    
    /// Create tool input from context
    async fn create_tool_input(
        &self,
        tool_name: &str,
        context: &ToolSelectionContext,
    ) -> Result<ToolInput> {
        let tools = self.tools.read().await;
        let tool = tools.get(tool_name)
            .ok_or_else(|| anyhow!("Tool not found: {}", tool_name))?;
        
        // Use tool's natural language parser if available
        if tool.supports_natural_language() {
            tool.parse_natural_language(&context.user_query).await
        } else {
            // Create basic input
            Ok(ToolInput {
                command: tool_name.to_string(),
                args: HashMap::new(),
                context: Some(context.user_query.clone()),
            })
        }
    }
    
    /// Calculate retry delay with exponential backoff and jitter
    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay_ms = self.retry_config.base_delay.as_millis() as f64;
        let backoff_delay = base_delay_ms * self.retry_config.backoff_multiplier.powi(attempt as i32 - 1);
        
        let delay_ms = if self.retry_config.jitter {
            // Add random jitter (±25%)
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
    
    /// Update success metrics
    async fn update_success_metrics(&self, tool_name: &str, execution_time: Duration) {
        let mut metrics = self.execution_metrics.lock().await;
        let tool_stats = metrics.tool_usage_stats.entry(tool_name.to_string())
            .or_insert_with(ToolUsageStats::default);
        
        tool_stats.executions += 1;
        tool_stats.successes += 1;
        tool_stats.total_time += execution_time;
        tool_stats.average_time = tool_stats.total_time / tool_stats.executions as u32;
        tool_stats.last_used = Some(Instant::now());
    }
    
    /// Update failure metrics
    async fn update_failure_metrics(&self, tool_name: &str, execution_time: Duration) {
        let mut metrics = self.execution_metrics.lock().await;
        let tool_stats = metrics.tool_usage_stats.entry(tool_name.to_string())
            .or_insert_with(ToolUsageStats::default);
        
        tool_stats.executions += 1;
        tool_stats.failures += 1;
        tool_stats.total_time += execution_time;
        tool_stats.average_time = tool_stats.total_time / tool_stats.executions as u32;
        tool_stats.last_used = Some(Instant::now());
    }
    
    /// Get pipeline execution statistics
    pub async fn get_pipeline_stats(&self) -> String {
        let metrics = self.execution_metrics.lock().await;
        let breakers = self.circuit_breakers.lock().await;
        
        let success_rate = if metrics.total_executions > 0 {
            (metrics.successful_executions as f32 / metrics.total_executions as f32) * 100.0
        } else {
            0.0
        };
        
        let mut stats = format!(
            "🔧 Tool Execution Pipeline Statistics:\n\n\
             📊 Overall Performance:\n\
             • Total executions: {}\n\
             • Success rate: {:.1}%\n\
             • Retried executions: {}\n\
             • Circuit breaker trips: {}\n\n\
             🛠️ Tool Performance:",
            metrics.total_executions,
            success_rate,
            metrics.retried_executions,
            metrics.circuit_breaker_trips
        );
        
        // Show top tools by usage
        let mut tool_stats: Vec<_> = metrics.tool_usage_stats.iter().collect();
        tool_stats.sort_by(|a, b| b.1.executions.cmp(&a.1.executions));
        
        for (name, tool_stats) in tool_stats.iter().take(10) {
            let tool_success_rate = if tool_stats.executions > 0 {
                (tool_stats.successes as f32 / tool_stats.executions as f32) * 100.0
            } else {
                0.0
            };
            
            stats.push_str(&format!(
                "\n • {}: {} executions, {:.1}% success, {:?} avg time",
                name, tool_stats.executions, tool_success_rate, tool_stats.average_time
            ));
        }
        
        // Show circuit breaker states
        let mut breaker_info = Vec::new();
        for (name, breaker) in breakers.iter() {
            if breaker.state != CircuitBreakerState::Closed || breaker.total_failures > 0 {
                breaker_info.push(format!(
                    " • {}: {:?} ({} failures)",
                    name, breaker.state, breaker.total_failures
                ));
            }
        }
        
        if !breaker_info.is_empty() {
            stats.push_str("\n\n🚨 Circuit Breaker Status:\n");
            for info in breaker_info {
                stats.push_str(&format!("\n{}", info));
            }
        }
        
        stats
    }
}

impl Default for ToolExecutionPipeline {
    fn default() -> Self {
        let selector = Arc::new(IntelligentToolSelector::default());
        Self::new(selector)
    }
}