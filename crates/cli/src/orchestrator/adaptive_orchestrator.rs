// @component: {"k":"C","id":"adaptive_task_orchestrator","t":"Intelligent task distribution orchestrator","m":{"cur":5,"tgt":95,"u":"%"},"f":["orchestration","adaptive","async","priority","load-balancing"]}

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// use crate::agent_traits::*; // Не используется пока
use super::{ResourceManager, StrategySelector, TaskAnalyzer};

/// Priority levels for task orchestration
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

/// Task complexity categories for intelligent routing
#[derive(Debug, Clone, PartialEq)]
pub enum TaskComplexity {
    Simple,  // Single operation, low resource usage
    Medium,  // Multiple steps, moderate resource usage
    Complex, // Complex logic, high resource usage
    Expert,  // Advanced reasoning, maximum resource usage
}

/// Resource requirements for task execution
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub cpu_intensity: f32, // 0.0 to 1.0
    pub memory_usage: f32,  // 0.0 to 1.0
    pub io_operations: f32, // 0.0 to 1.0
    pub network_usage: f32, // 0.0 to 1.0
    pub estimated_duration: Duration,
}

/// Task context with all necessary information for orchestration
#[derive(Debug, Clone)]
pub struct OrchestrationTask {
    pub id: Uuid,
    pub content: String,
    pub priority: TaskPriority,
    pub complexity: TaskComplexity,
    pub resource_requirements: ResourceRequirements,
    pub context: HashMap<String, String>,
    pub deadline: Option<Instant>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: Instant,
}

/// Result of task orchestration
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    pub task_id: Uuid,
    pub response: String,
    pub execution_time: Duration,
    pub handler_used: String,
    pub resource_usage: ResourceUsageStats,
    pub success: bool,
    pub retry_count: u32,
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceUsageStats {
    pub cpu_used: f32,
    pub memory_used: f32,
    pub io_operations: u64,
    pub network_bytes: u64,
}

/// Orchestration-wide metrics
#[derive(Debug, Clone)]
pub struct OrchestrationMetrics {
    pub total_tasks_completed: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub avg_task_duration: Duration,
    pub resource_utilization: f32,
}

impl Default for OrchestrationMetrics {
    fn default() -> Self {
        Self {
            total_tasks_completed: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            avg_task_duration: Duration::from_millis(0),
            resource_utilization: 0.0,
        }
    }
}

/// Handler availability and performance metrics
#[derive(Debug, Clone)]
pub struct HandlerMetrics {
    pub total_tasks: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time: Duration,
    pub current_load: f32, // 0.0 to 1.0
    pub availability: bool,
    pub last_health_check: Instant,
}

impl Default for HandlerMetrics {
    fn default() -> Self {
        Self {
            total_tasks: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_execution_time: Duration::from_millis(0),
            current_load: 0.0,
            availability: true,
            last_health_check: Instant::now(),
        }
    }
}

/// Adaptive Task Orchestrator - интеллектуальный распределитель задач
pub struct AdaptiveTaskOrchestrator {
    // Core components
    task_analyzer: Arc<TaskAnalyzer>,
    resource_manager: Arc<ResourceManager>,
    strategy_selector: Arc<Mutex<StrategySelector>>,

    // Handler registry with metrics
    chat_handler_metrics: Arc<Mutex<HandlerMetrics>>,
    tools_handler_metrics: Arc<Mutex<HandlerMetrics>>,
    memory_handler_metrics: Arc<Mutex<HandlerMetrics>>,
    admin_handler_metrics: Arc<Mutex<HandlerMetrics>>,

    // Task queue and execution tracking
    active_tasks: Arc<RwLock<HashMap<Uuid, OrchestrationTask>>>,
    task_history: Arc<Mutex<Vec<OrchestrationResult>>>,

    // Performance optimization
    #[allow(dead_code)] // Кеш производительности для оптимизации
    performance_cache: Arc<Mutex<HashMap<String, Duration>>>, // task_hash -> avg_execution_time
    load_balancer: Arc<Mutex<LoadBalancer>>,

    // Configuration
    config: OrchestrationConfig,

    // Metrics and monitoring
    total_tasks_processed: Arc<Mutex<u64>>,
    avg_response_time: Arc<Mutex<Duration>>,
    #[allow(dead_code)] // Метрики оркестрации
    orchestration_metrics: Arc<Mutex<OrchestrationMetrics>>,
}

/// Load balancer for distributing tasks across handlers
#[derive(Debug)]
struct LoadBalancer {
    handler_loads: HashMap<String, f32>,
    #[allow(dead_code)] // Индекс для round-robin балансировки
    round_robin_index: usize,
    adaptive_weights: HashMap<String, f32>,
}

impl LoadBalancer {
    fn new() -> Self {
        let mut handler_loads = HashMap::new();
        let mut adaptive_weights = HashMap::new();

        // Initialize handler loads
        for handler in &["chat", "tools", "memory", "admin"] {
            handler_loads.insert(handler.to_string(), 0.0);
            adaptive_weights.insert(handler.to_string(), 1.0);
        }

        Self {
            handler_loads,
            round_robin_index: 0,
            adaptive_weights,
        }
    }

    /// Select optimal handler based on load and task requirements
    fn select_handler(
        &mut self,
        task: &OrchestrationTask,
        available_handlers: &[String],
    ) -> String {
        if available_handlers.is_empty() {
            return "chat".to_string(); // fallback
        }

        // Calculate handler scores based on load and task complexity
        let mut handler_scores = HashMap::new();

        for handler in available_handlers {
            let load = self.handler_loads.get(handler).copied().unwrap_or(0.0);
            let weight = self.adaptive_weights.get(handler).copied().unwrap_or(1.0);
            let load_penalty = load * 2.0; // Higher load = lower score
            let weight_bonus = weight; // Higher weight = better score

            let base_score = match handler.as_str() {
                "chat" => match task.complexity {
                    TaskComplexity::Simple | TaskComplexity::Medium => 0.8,
                    _ => 0.3,
                },
                "tools" => match task.complexity {
                    TaskComplexity::Complex | TaskComplexity::Expert => 0.9,
                    TaskComplexity::Medium => 0.7,
                    _ => 0.4,
                },
                "memory" => 0.5, // Neutral for all tasks
                "admin" => match task.priority {
                    TaskPriority::Critical | TaskPriority::Emergency => 0.9,
                    _ => 0.2,
                },
                _ => 0.1,
            };

            let final_score = (base_score + weight_bonus) - load_penalty;
            handler_scores.insert(handler.clone(), final_score);
        }

        // Select handler with highest score
        handler_scores
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(handler, _)| handler)
            .unwrap_or_else(|| available_handlers[0].clone())
    }

    /// Update handler load after task completion
    fn update_load(&mut self, handler: &str, execution_time: Duration, success: bool) {
        let current_load = self.handler_loads.get(handler).copied().unwrap_or(0.0);
        let time_factor = (execution_time.as_millis() as f32) / 1000.0; // seconds
        let load_delta = if success {
            time_factor * 0.1
        } else {
            time_factor * 0.3
        };

        let new_load = (current_load + load_delta).min(1.0);
        self.handler_loads.insert(handler.to_string(), new_load);

        // Decay load over time (simulate recovery)
        let decay_factor = 0.95;
        let decayed_load = new_load * decay_factor;
        self.handler_loads.insert(handler.to_string(), decayed_load);

        // Update adaptive weights based on success rate
        let current_weight = self.adaptive_weights.get(handler).copied().unwrap_or(1.0);
        let weight_delta = if success { 0.05 } else { -0.1 };
        let new_weight = (current_weight + weight_delta).clamp(0.1, 2.0);
        self.adaptive_weights
            .insert(handler.to_string(), new_weight);
    }
}

/// Configuration for orchestration behavior
#[derive(Debug, Clone)]
pub struct OrchestrationConfig {
    pub max_concurrent_tasks: usize,
    pub task_timeout: Duration,
    pub health_check_interval: Duration,
    pub performance_cache_ttl: Duration,
    pub enable_load_balancing: bool,
    pub enable_adaptive_routing: bool,
    pub retry_strategy: RetryStrategy,
    pub max_retries: u32,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            task_timeout: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            performance_cache_ttl: Duration::from_secs(300),
            enable_load_balancing: true,
            enable_adaptive_routing: true,
            retry_strategy: RetryStrategy::ExponentialBackoff {
                base_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(10),
                max_attempts: 3,
            },
            max_retries: 3,
        }
    }
}

/// Retry strategies for failed tasks
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    None,
    Fixed {
        delay: Duration,
        max_attempts: u32,
    },
    ExponentialBackoff {
        base_delay: Duration,
        max_delay: Duration,
        max_attempts: u32,
    },
}

impl AdaptiveTaskOrchestrator {
    /// Create a new Adaptive Task Orchestrator
    pub fn new(config: OrchestrationConfig) -> Self {
        info!("🎯 Инициализация Adaptive Task Orchestrator");

        Self {
            task_analyzer: Arc::new(TaskAnalyzer::new()),
            resource_manager: Arc::new(ResourceManager::new()),
            strategy_selector: Arc::new(Mutex::new(StrategySelector::new())),

            chat_handler_metrics: Arc::new(Mutex::new(HandlerMetrics::default())),
            tools_handler_metrics: Arc::new(Mutex::new(HandlerMetrics::default())),
            memory_handler_metrics: Arc::new(Mutex::new(HandlerMetrics::default())),
            admin_handler_metrics: Arc::new(Mutex::new(HandlerMetrics::default())),

            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_history: Arc::new(Mutex::new(Vec::new())),

            performance_cache: Arc::new(Mutex::new(HashMap::new())),
            load_balancer: Arc::new(Mutex::new(LoadBalancer::new())),

            config,

            total_tasks_processed: Arc::new(Mutex::new(0)),
            avg_response_time: Arc::new(Mutex::new(Duration::from_millis(0))),
            orchestration_metrics: Arc::new(Mutex::new(OrchestrationMetrics::default())),
        }
    }

    /// Orchestrate task execution with intelligent routing
    pub async fn orchestrate_task(
        &self,
        content: &str,
        context: HashMap<String, String>,
    ) -> Result<OrchestrationResult> {
        let start_time = Instant::now();
        let task_id = Uuid::new_v4();

        info!("🎯 Начало оркестрации задачи {}: {}", task_id, content);

        // Step 1: Analyze task
        let (priority, complexity) = self.task_analyzer.analyze_task(content, &context).await?;
        let resource_requirements = self
            .task_analyzer
            .estimate_resources(&complexity, &priority)
            .await;

        let orchestration_task = OrchestrationTask {
            id: task_id,
            content: content.to_string(),
            priority,
            complexity,
            resource_requirements: resource_requirements.clone(),
            context: context.clone(),
            deadline: None,
            retry_count: 0,
            max_retries: match self.config.retry_strategy {
                RetryStrategy::None => 0,
                RetryStrategy::Fixed { max_attempts, .. } => max_attempts,
                RetryStrategy::ExponentialBackoff { max_attempts, .. } => max_attempts,
            },
            created_at: Instant::now(),
        };

        // Step 2: Add to active tasks
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id, orchestration_task.clone());
        }

        // Step 3: Check resource availability
        let can_execute = self
            .resource_manager
            .can_allocate_resources(&resource_requirements)
            .await?;
        if !can_execute {
            return Err(anyhow::anyhow!(
                "Insufficient resources available for task execution"
            ));
        }

        // Step 4: Select optimal execution strategy and handler
        let available_handlers = self.get_available_handlers().await;
        let selected_handler = {
            let mut balancer = self.load_balancer.lock().await;
            balancer.select_handler(&orchestration_task, &available_handlers)
        };

        info!(
            "📋 Task {} routed to handler: {}",
            task_id, selected_handler
        );

        // Step 5: Execute task
        let execution_result = self
            .execute_task_on_handler(&orchestration_task, &selected_handler)
            .await;

        let execution_time = start_time.elapsed();

        // Step 6: Update metrics and cleanup
        let success = execution_result.is_ok();
        self.update_handler_metrics(&selected_handler, execution_time, success)
            .await;

        {
            let mut balancer = self.load_balancer.lock().await;
            balancer.update_load(&selected_handler, execution_time, success);
        }

        // Remove from active tasks
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.remove(&task_id);
        }

        let result = match execution_result {
            Ok(response) => {
                info!(
                    "✅ Task {} completed successfully in {:?}",
                    task_id, execution_time
                );
                OrchestrationResult {
                    task_id,
                    response,
                    execution_time,
                    handler_used: selected_handler,
                    resource_usage: ResourceUsageStats {
                        cpu_used: resource_requirements.cpu_intensity,
                        memory_used: resource_requirements.memory_usage,
                        io_operations: 0,
                        network_bytes: 0,
                    },
                    success: true,
                    retry_count: orchestration_task.retry_count,
                }
            }
            Err(e) => {
                error!("❌ Task {} failed: {}", task_id, e);
                OrchestrationResult {
                    task_id,
                    response: format!("Task failed: {}", e),
                    execution_time,
                    handler_used: selected_handler,
                    resource_usage: ResourceUsageStats {
                        cpu_used: 0.0,
                        memory_used: 0.0,
                        io_operations: 0,
                        network_bytes: 0,
                    },
                    success: false,
                    retry_count: orchestration_task.retry_count,
                }
            }
        };

        // Update statistics
        {
            let mut total_tasks = self.total_tasks_processed.lock().await;
            *total_tasks += 1;

            let mut avg_time = self.avg_response_time.lock().await;
            let tasks_count = *total_tasks as f64;
            let current_avg = avg_time.as_millis() as f64;
            let new_avg = (current_avg * (tasks_count - 1.0) + execution_time.as_millis() as f64)
                / tasks_count;
            *avg_time = Duration::from_millis(new_avg as u64);
        }

        // Store in history
        {
            let mut history = self.task_history.lock().await;
            history.push(result.clone());

            // Keep only last 1000 results
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        Ok(result)
    }

    /// Execute task on specified handler (placeholder - would integrate with actual handlers)
    async fn execute_task_on_handler(
        &self,
        task: &OrchestrationTask,
        handler: &str,
    ) -> Result<String> {
        debug!("🔄 Executing task {} on handler {}", task.id, handler);

        // This would be replaced with actual handler execution
        // For now, simulate execution based on task complexity
        let execution_time = match task.complexity {
            TaskComplexity::Simple => Duration::from_millis(50),
            TaskComplexity::Medium => Duration::from_millis(200),
            TaskComplexity::Complex => Duration::from_millis(800),
            TaskComplexity::Expert => Duration::from_millis(2000),
        };

        tokio::time::sleep(execution_time).await;

        Ok(format!(
            "Task '{}' executed by {} handler",
            task.content, handler
        ))
    }

    /// Get list of available handlers based on health checks
    async fn get_available_handlers(&self) -> Vec<String> {
        let mut available = Vec::new();

        // Check each handler's availability
        if self.is_handler_available("chat").await {
            available.push("chat".to_string());
        }
        if self.is_handler_available("tools").await {
            available.push("tools".to_string());
        }
        if self.is_handler_available("memory").await {
            available.push("memory".to_string());
        }
        if self.is_handler_available("admin").await {
            available.push("admin".to_string());
        }

        available
    }

    /// Check if specific handler is available and healthy
    async fn is_handler_available(&self, handler: &str) -> bool {
        let metrics = match handler {
            "chat" => self.chat_handler_metrics.lock().await,
            "tools" => self.tools_handler_metrics.lock().await,
            "memory" => self.memory_handler_metrics.lock().await,
            "admin" => self.admin_handler_metrics.lock().await,
            _ => return false,
        };

        metrics.availability && metrics.current_load < 0.9
    }

    /// Update handler metrics after task completion
    async fn update_handler_metrics(&self, handler: &str, execution_time: Duration, success: bool) {
        let metrics_lock = match handler {
            "chat" => &self.chat_handler_metrics,
            "tools" => &self.tools_handler_metrics,
            "memory" => &self.memory_handler_metrics,
            "admin" => &self.admin_handler_metrics,
            _ => return,
        };

        let mut metrics = metrics_lock.lock().await;
        metrics.total_tasks += 1;

        if success {
            metrics.successful_tasks += 1;
        } else {
            metrics.failed_tasks += 1;
        }

        // Update average execution time
        let total = metrics.total_tasks as f64;
        let current_avg = metrics.average_execution_time.as_millis() as f64;
        let new_time = execution_time.as_millis() as f64;
        let new_avg = (current_avg * (total - 1.0) + new_time) / total;
        metrics.average_execution_time = Duration::from_millis(new_avg as u64);

        // Update current load (simplified calculation)
        metrics.current_load = (metrics.failed_tasks as f32) / (metrics.total_tasks as f32);
        metrics.last_health_check = Instant::now();

        debug!(
            "📊 Updated metrics for {}: {} tasks, {:.1}% success rate",
            handler,
            metrics.total_tasks,
            (metrics.successful_tasks as f32 / metrics.total_tasks as f32) * 100.0
        );
    }

    /// Submit task for orchestration and return execution strategy
    pub async fn submit_task(
        &self,
        task_id: &str,
        content: &str,
        context: HashMap<String, String>,
    ) -> Result<super::ExecutionStrategy> {
        debug!("📥 Submitting task {} for orchestration", task_id);

        // Analyze task complexity and priority
        let (priority, complexity) = self.task_analyzer.analyze_task(content, &context).await?;
        let resource_requirements = self
            .task_analyzer
            .estimate_resources(&complexity, &priority)
            .await;

        // Check if resources can be allocated
        if !self
            .resource_manager
            .can_allocate_resources(&resource_requirements)
            .await?
        {
            return Err(anyhow::anyhow!("Insufficient resources available for task"));
        }

        // Reserve resources
        self.resource_manager
            .reserve_resources(task_id, resource_requirements.clone())
            .await?;

        // Get system resource status
        let resource_status = self.resource_manager.get_resource_status().await;

        // Create selection criteria for strategy selector
        let selection_criteria = super::SelectionCriteria {
            task_complexity: complexity.clone(),
            task_priority: priority.clone(),
            resource_availability: 1.0 - resource_status.cpu_usage,
            system_load: resource_status.cpu_usage,
            queue_depth: self.active_tasks.read().await.len(),
            similar_tasks_count: 0, // TODO: implement task similarity analysis
            estimated_duration: resource_requirements.estimated_duration,
        };

        // Select execution strategy
        let strategy = self
            .strategy_selector
            .lock()
            .await
            .select_strategy(&selection_criteria)
            .await?;

        // Create orchestration task
        let task_uuid =
            Uuid::parse_str(&task_id.replace("task_", "")).unwrap_or_else(|_| Uuid::new_v4());
        let orchestration_task = OrchestrationTask {
            id: task_uuid,
            content: content.to_string(),
            priority,
            complexity,
            resource_requirements,
            context,
            deadline: None,
            retry_count: 0,
            max_retries: self.config.max_retries,
            created_at: Instant::now(),
        };

        // Add to active tasks
        self.active_tasks
            .write()
            .await
            .insert(task_uuid, orchestration_task);

        info!(
            "✅ Task {} orchestrated with strategy: {:?}",
            task_id, strategy
        );
        Ok(strategy)
    }

    /// Complete task and update metrics
    pub async fn complete_task(
        &self,
        task_id: &str,
        success: bool,
        duration: std::time::Duration,
    ) -> Result<()> {
        debug!("🏁 Completing task: {} (success: {})", task_id, success);

        // Parse task_id to get UUID
        let task_uuid = match Uuid::parse_str(&task_id.replace("task_", "")) {
            Ok(uuid) => uuid,
            Err(_) => {
                // Fallback for invalid UUIDs
                warn!("Invalid task UUID: {}", task_id);
                return Ok(());
            }
        };

        // Remove from active tasks
        let mut tasks = self.active_tasks.write().await;
        if let Some(_task) = tasks.remove(&task_uuid) {
            debug!("✅ Task {} removed from active tasks", task_id);
        }

        // Update metrics
        let mut total_tasks = self.total_tasks_processed.lock().await;
        *total_tasks += 1;

        let mut avg_time = self.avg_response_time.lock().await;
        let new_avg = (avg_time.as_millis() + duration.as_millis()) / 2;
        *avg_time = Duration::from_millis(new_avg as u64);

        Ok(())
    }

    /// Shutdown orchestrator gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Adaptive Task Orchestrator");

        // Wait for active tasks to complete (with timeout)
        let timeout_duration = std::time::Duration::from_secs(30);
        let start_time = std::time::Instant::now();

        while !self.active_tasks.read().await.is_empty() && start_time.elapsed() < timeout_duration
        {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let remaining_tasks = self.active_tasks.read().await.len();
        if remaining_tasks > 0 {
            warn!(
                "⚠️ Orchestrator shutdown with {} active tasks remaining",
                remaining_tasks
            );
        }

        info!("✅ Adaptive Task Orchestrator shutdown complete");
        Ok(())
    }

    /// Get comprehensive orchestration statistics
    pub async fn get_orchestration_stats(&self) -> String {
        let mut stats = String::new();
        stats.push_str("🎯 Adaptive Task Orchestrator Statistics\n\n");

        let total_tasks = *self.total_tasks_processed.lock().await;
        let avg_time = *self.avg_response_time.lock().await;
        let active_tasks_count = self.active_tasks.read().await.len();

        stats.push_str("📊 Overall Performance:\n");
        stats.push_str(&format!("  • Total tasks processed: {}\n", total_tasks));
        stats.push_str(&format!("  • Average response time: {:?}\n", avg_time));
        stats.push_str(&format!(
            "  • Currently active tasks: {}\n",
            active_tasks_count
        ));
        stats.push_str(&format!(
            "  • Max concurrent tasks: {}\n",
            self.config.max_concurrent_tasks
        ));

        stats.push_str("\n🔧 Handler Performance:\n");

        // Chat handler stats
        {
            let metrics = self.chat_handler_metrics.lock().await;
            stats.push_str(&format!(
                "  • Chat Handler: {} tasks ({:.1}% success, {:?} avg)\n",
                metrics.total_tasks,
                if metrics.total_tasks > 0 {
                    (metrics.successful_tasks as f32 / metrics.total_tasks as f32) * 100.0
                } else {
                    0.0
                },
                metrics.average_execution_time
            ));
        }

        // Tools handler stats
        {
            let metrics = self.tools_handler_metrics.lock().await;
            stats.push_str(&format!(
                "  • Tools Handler: {} tasks ({:.1}% success, {:?} avg)\n",
                metrics.total_tasks,
                if metrics.total_tasks > 0 {
                    (metrics.successful_tasks as f32 / metrics.total_tasks as f32) * 100.0
                } else {
                    0.0
                },
                metrics.average_execution_time
            ));
        }

        // Memory handler stats
        {
            let metrics = self.memory_handler_metrics.lock().await;
            stats.push_str(&format!(
                "  • Memory Handler: {} tasks ({:.1}% success, {:?} avg)\n",
                metrics.total_tasks,
                if metrics.total_tasks > 0 {
                    (metrics.successful_tasks as f32 / metrics.total_tasks as f32) * 100.0
                } else {
                    0.0
                },
                metrics.average_execution_time
            ));
        }

        // Admin handler stats
        {
            let metrics = self.admin_handler_metrics.lock().await;
            stats.push_str(&format!(
                "  • Admin Handler: {} tasks ({:.1}% success, {:?} avg)\n",
                metrics.total_tasks,
                if metrics.total_tasks > 0 {
                    (metrics.successful_tasks as f32 / metrics.total_tasks as f32) * 100.0
                } else {
                    0.0
                },
                metrics.average_execution_time
            ));
        }

        // Load balancer stats
        {
            let balancer = self.load_balancer.lock().await;
            stats.push_str("\n⚖️ Load Balancer Status:\n");
            for (handler, load) in &balancer.handler_loads {
                let weight = balancer
                    .adaptive_weights
                    .get(handler)
                    .copied()
                    .unwrap_or(1.0);
                stats.push_str(&format!(
                    "  • {}: load={:.2}, weight={:.2}\n",
                    handler, load, weight
                ));
            }
        }

        stats
    }

    /// Alias for get_orchestration_stats for backward compatibility
    pub async fn get_statistics(&self) -> String {
        self.get_orchestration_stats().await
    }

    /// Initialize the orchestrator and all its components
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Initializing Adaptive Task Orchestrator");

        // Initialize performance monitoring
        // Here we could add specific initialization for components if needed

        // Verify all components are healthy
        if !self.is_healthy().await {
            return Err(anyhow::anyhow!(
                "Some orchestrator components are unhealthy"
            ));
        }

        info!("✅ Adaptive Task Orchestrator initialized successfully");
        Ok(())
    }

    /// Check if the orchestrator is healthy and ready to process tasks
    pub async fn is_healthy(&self) -> bool {
        // Check if all handlers are available
        let available_handlers = self.get_available_handlers().await;

        // Must have at least one handler available
        if available_handlers.is_empty() {
            return false;
        }

        // Check resource manager availability
        let _resource_status = self.resource_manager.get_resource_status().await;
        // Resource manager doesn't return Result, so we assume it's always available

        // Check active tasks aren't overloaded
        let active_count = self.active_tasks.read().await.len();
        if active_count >= self.config.max_concurrent_tasks {
            return false;
        }

        true
    }
}

// This would be implemented by UnifiedAgentV2 or similar
#[async_trait]
pub trait TaskOrchestrator {
    async fn orchestrate(&self, task: &str, context: HashMap<String, String>) -> Result<String>;
    async fn get_orchestration_metrics(&self) -> String;
}
