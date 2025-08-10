// @component: {"k":"C","id":"enhanced_tool_system","t":"Complete enhanced tool system with intelligent selection and monitoring","m":{"cur":5,"tgt":95,"u":"%"},"f":["system","integration","orchestration","enhanced","production"]}

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::execution::pipeline::{ExecutionPipeline, ExecutionResult, ExecutionStrategy};
use crate::intelligent_selector::{
    IntelligentToolSelector, TaskComplexity, ToolSelectionContext, UrgencyLevel, UserExpertise,
};
use crate::performance_monitor::{MonitorConfig, ToolPerformanceMonitor};
use crate::{Tool, ToolSpec};

/// Enhanced tool system configuration
#[derive(Debug, Clone)]
pub struct EnhancedToolSystemConfig {
    pub enable_intelligent_selection: bool,
    pub enable_performance_monitoring: bool,
    pub enable_execution_pipeline: bool,
    pub default_execution_strategy: ExecutionStrategy,
    pub monitor_config: MonitorConfig,
    pub max_concurrent_executions: u32,
    pub enable_auto_optimization: bool,
}

impl Default for EnhancedToolSystemConfig {
    fn default() -> Self {
        Self {
            enable_intelligent_selection: true,
            enable_performance_monitoring: true,
            enable_execution_pipeline: true,
            default_execution_strategy: ExecutionStrategy::RetryWithBackoff,
            monitor_config: MonitorConfig::default(),
            max_concurrent_executions: 10,
            enable_auto_optimization: true,
        }
    }
}

/// Enhanced tool system result
#[derive(Debug)]
pub struct EnhancedToolResult {
    pub execution_result: ExecutionResult,
    pub selection_confidence: f32,
    pub alternative_tools: Vec<String>,
    pub performance_metrics: Option<ToolPerformanceSnapshot>,
    pub optimization_suggestions: Vec<String>,
}

/// Snapshot of tool performance for result context
#[derive(Debug, Clone)]
pub struct ToolPerformanceSnapshot {
    pub tool_name: String,
    pub execution_time: Duration,
    pub success_rate: f32,
    pub recent_usage_count: u64,
}

/// Enhanced tool system with all advanced features
pub struct EnhancedToolSystem {
    /// Core components
    intelligent_selector: Arc<IntelligentToolSelector>,
    execution_pipeline: Arc<ExecutionPipeline>,
    performance_monitor: Arc<ToolPerformanceMonitor>,

    /// System configuration
    config: EnhancedToolSystemConfig,

    /// Registered tools
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,

    /// System statistics
    system_stats: Arc<tokio::sync::Mutex<SystemStats>>, 
}

/// System-wide statistics
#[derive(Debug, Default)]
pub struct SystemStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: Duration,
    pub optimization_decisions: u64,
    pub intelligence_assists: u64,
}

impl EnhancedToolSystem {
    pub async fn new(config: EnhancedToolSystemConfig) -> Result<Self> {
        info!("🚀 Initializing Enhanced Tool System");

        // Initialize core components
        let intelligent_selector = Arc::new(IntelligentToolSelector::default());
        let execution_pipeline = Arc::new(ExecutionPipeline::new(
            crate::execution::resource_manager::ResourceLimits::default(),
            Arc::clone(&intelligent_selector),
        ));
        let performance_monitor =
            Arc::new(ToolPerformanceMonitor::new(config.monitor_config.clone()));

        let system = Self {
            intelligent_selector,
            execution_pipeline,
            performance_monitor,
            config,
            tools: Arc::new(RwLock::new(HashMap::new())),
            system_stats: Arc::new(tokio::sync::Mutex::new(SystemStats::default())),
        };

        info!(
            "✅ Enhanced Tool System initialized with {} features",
            system.count_enabled_features()
        );

        Ok(system)
    }

    /// Register a tool with all system components
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let spec = tool.spec();
        let tool_name = spec.name.clone();

        info!("📝 Registering tool: {}", tool_name);

        // Register with all components
        if self.config.enable_intelligent_selection {
            self.intelligent_selector.register_tool(spec.clone()).await;
        }

        if self.config.enable_execution_pipeline {
            self.execution_pipeline
                .register_tool(Arc::clone(&tool), crate::registry::ToolMetadata::from_spec(&spec))
                .await;
        }

        // Store in our registry
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_name.clone(), tool);
        }

        debug!("✅ Tool registered: {}", tool_name);
        Ok(())
    }

    /// Execute a tool request with full enhanced features
    pub async fn execute_request(
        &self,
        user_query: &str,
        session_context: Option<HashMap<String, String>>,
    ) -> Result<EnhancedToolResult> {
        let start_time = std::time::Instant::now();

        // Update system stats
        {
            let mut stats = self.system_stats.lock().await;
            stats.total_requests += 1;
        }

        info!("🎯 Processing enhanced tool request: '{}'", user_query);

        // Analyze query complexity and create context
        let context = self
            .create_selection_context(user_query, session_context)
            .await;

        // Determine optimal execution strategy
        let execution_strategy = if self.config.enable_auto_optimization {
            self.determine_optimal_strategy(&context).await
        } else {
            self.config.default_execution_strategy.clone()
        };

        debug!("🔧 Using execution strategy: {:?}", execution_strategy);

        // Start performance monitoring if enabled
        let execution_tracker = if self.config.enable_performance_monitoring {
            Some(
                self.performance_monitor
                    .execution_started("enhanced_system")
                    .await,
            )
        } else {
            None
        };

        // Execute through pipeline (selection inside pipeline)
        let execution_result = if self.config.enable_execution_pipeline {
            self.execution_pipeline
                .execute_with_selection(
                    crate::execution::pipeline::ExecutionContext {
                        user_query: context.user_query.clone(),
                        session_id: context
                            .session_context
                            .get("session_id")
                            .cloned()
                            .unwrap_or_else(|| "default".into()),
                        user_id: context
                            .session_context
                            .get("user_id")
                            .cloned()
                            .unwrap_or_else(|| "anonymous".into()),
                        security_level: crate::registry::SecurityLevel::Safe,
                        resource_limits: crate::execution::resource_manager::ResourceLimits::default(),
                        metadata: context.session_context.clone(),
                    },
                    execution_strategy,
                )
                .await?
        } else {
            // Fallback to direct execution
            self.execute_direct(&context).await?
        };

        let total_execution_time = start_time.elapsed();

        // Complete performance tracking
        if let Some(tracker) = execution_tracker {
            if execution_result.output.success {
                tracker
                    .success(Some("enhanced_system_execution".to_string()))
                    .await;
            } else {
                tracker
                    .failure(
                        execution_result.output.result.clone(),
                        Some("enhanced_system_execution".to_string()),
                    )
                    .await;
            }
        }

        // Get performance snapshot
        let performance_snapshot = if self.config.enable_performance_monitoring {
            self.get_performance_snapshot(&self.extract_tool_name(&execution_result))
                .await
        } else {
            None
        };

        // Generate optimization suggestions
        let optimization_suggestions = if self.config.enable_auto_optimization {
            self.generate_optimization_suggestions(&execution_result, &context)
                .await
        } else {
            Vec::new()
        };

        // Get alternative tools for context
        let alternative_tools = if self.config.enable_intelligent_selection {
            self.get_alternative_tools(&context).await
        } else {
            Vec::new()
        };

        // Update system stats
        {
            let mut stats = self.system_stats.lock().await;
            if execution_result.output.success {
                stats.successful_requests += 1;
            } else {
                stats.failed_requests += 1;
            }

            // Update average response time
            let total_time = stats.average_response_time * (stats.total_requests - 1) as u32
                + total_execution_time;
            stats.average_response_time = total_time / stats.total_requests as u32;

            if self.config.enable_auto_optimization {
                stats.optimization_decisions += 1;
            }

            if self.config.enable_intelligent_selection {
                stats.intelligence_assists += 1;
            }
        }

        let result = EnhancedToolResult {
            execution_result,
            selection_confidence: 0.8,
            alternative_tools,
            performance_metrics: performance_snapshot,
            optimization_suggestions,
        };

        info!(
            "✅ Enhanced tool request completed in {:?}",
            total_execution_time
        );
        Ok(result)
    }

    fn extract_tool_name(&self, _exec: &ExecutionResult) -> String {
        // ExecutionResult нового пайплайна не содержит явного tool_name —
        // для метрик используем ключ из контекста/монитора или оставляем placeholder.
        "unknown".to_string()
    }

    /// Create tool selection context from user query
    async fn create_selection_context(
        &self,
        user_query: &str,
        session_context: Option<HashMap<String, String>>,
    ) -> ToolSelectionContext {
        // Analyze query complexity (simplified heuristics)
        let task_complexity = self.analyze_task_complexity(user_query);
        let urgency_level = self.analyze_urgency_level(user_query);
        let user_expertise = UserExpertise::Intermediate; // Could be determined from session

        ToolSelectionContext {
            user_query: user_query.to_string(),
            session_context: session_context.unwrap_or_default(),
            previous_tools_used: Vec::new(), // Could be tracked per session
            task_complexity,
            urgency_level,
            user_expertise,
        }
    }

    /// Analyze task complexity from user query
    fn analyze_task_complexity(&self, query: &str) -> TaskComplexity {
        let query_lower = query.to_lowercase();

        // Count complexity indicators
        let complex_indicators = [
            "найди и",
            "создай и",
            "проанализируй",
            "сравни",
            "интегрируй",
            "оптимизируй",
            "рефактори",
            "настрой",
            "развернут",
            "миграц",
        ];

        let medium_indicators = [
            "создай",
            "обнови",
            "измени",
            "удали",
            "скопируй",
            "перемести",
        ];

        let complex_count = complex_indicators
            .iter()
            .filter(|&indicator| query_lower.contains(indicator))
            .count();

        let medium_count = medium_indicators
            .iter()
            .filter(|&indicator| query_lower.contains(indicator))
            .count();

        if complex_count > 0 || query.len() > 200 {
            TaskComplexity::Complex
        } else if medium_count > 0 || query.len() > 100 {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Simple
        }
    }

    /// Analyze urgency level from user query
    fn analyze_urgency_level(&self, query: &str) -> UrgencyLevel {
        let query_lower = query.to_lowercase();

        if query_lower.contains("срочно")
            || query_lower.contains("быстро")
            || query_lower.contains("критично")
            || query_lower.contains("немедленно")
        {
            UrgencyLevel::Critical
        } else if query_lower.contains("важно") || query_lower.contains("приоритет") {
            UrgencyLevel::High
        } else {
            UrgencyLevel::Normal
        }
    }

    /// Determine optimal execution strategy based on context
    async fn determine_optimal_strategy(
        &self,
        context: &ToolSelectionContext,
    ) -> ExecutionStrategy {
        match (&context.task_complexity, &context.urgency_level) {
            (TaskComplexity::Expert, _) => ExecutionStrategy::CircuitBreakerProtected,
            (TaskComplexity::Complex, UrgencyLevel::Critical) => ExecutionStrategy::ParallelFastest,
            (TaskComplexity::Complex, _) => ExecutionStrategy::SequentialFallback,
            (_, UrgencyLevel::Critical) => ExecutionStrategy::RetryWithBackoff,
            (TaskComplexity::Simple, _) => ExecutionStrategy::Direct,
            _ => ExecutionStrategy::RetryWithBackoff,
        }
    }

    /// Execute tool directly (fallback)
    async fn execute_direct(&self, _context: &ToolSelectionContext) -> Result<ExecutionResult> {
        Err(anyhow::anyhow!(
            "Direct execution not implemented - requires execution pipeline"
        ))
    }

    /// Get performance snapshot for a tool
    async fn get_performance_snapshot(&self, tool_name: &str) -> Option<ToolPerformanceSnapshot> {
        if let Some(metrics) = self.performance_monitor.get_tool_metrics(tool_name).await {
            Some(ToolPerformanceSnapshot {
                tool_name: tool_name.to_string(),
                execution_time: metrics.average_execution_time,
                success_rate: metrics.success_rate,
                recent_usage_count: metrics.total_executions,
            })
        } else {
            None
        }
    }

    /// Generate optimization suggestions based on execution
    async fn generate_optimization_suggestions(
        &self,
        execution_result: &ExecutionResult,
        context: &ToolSelectionContext,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if execution_result.execution_time > Duration::from_secs(5) {
            suggestions.push("Consider using parallel execution for complex tasks".to_string());
        }
        if execution_result.attempt_count > 1 {
            suggestions.push(
                "Tool reliability could be improved with circuit breaker protection".to_string(),
            );
        }
        if context.task_complexity == TaskComplexity::Complex
            && execution_result.strategy_used == ExecutionStrategy::Direct
        {
            suggestions.push("Complex tasks benefit from sequential fallback strategy".to_string());
        }
        if context.urgency_level == UrgencyLevel::Critical
            && execution_result.execution_time > Duration::from_secs(2)
        {
            suggestions.push("Critical tasks should use parallel fastest strategy".to_string());
        }
        suggestions
    }

    /// Get alternative tools that could handle the same request
    async fn get_alternative_tools(&self, context: &ToolSelectionContext) -> Vec<String> {
        match self.intelligent_selector.select_tools(context).await {
            Ok(candidates) => candidates
                .into_iter()
                .skip(1)
                .take(3)
                .map(|c| c.tool_name)
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Count enabled features for initialization logging
    fn count_enabled_features(&self) -> usize {
        let mut count = 0;
        if self.config.enable_intelligent_selection {
            count += 1;
        }
        if self.config.enable_performance_monitoring {
            count += 1;
        }
        if self.config.enable_execution_pipeline {
            count += 1;
        }
        if self.config.enable_auto_optimization {
            count += 1;
        }
        count
    }

    /// Get comprehensive system statistics
    pub async fn get_system_stats(&self) -> String {
        let stats = self.system_stats.lock().await;
        let pipeline_stats = if self.config.enable_execution_pipeline {
            self.execution_pipeline.get_pipeline_stats().await
        } else {
            "Execution pipeline disabled".to_string()
        };
        let selector_stats = if self.config.enable_intelligent_selection {
            self.intelligent_selector.get_selection_stats().await
        } else {
            "Intelligent selection disabled".to_string()
        };
        let monitor_stats = if self.config.enable_performance_monitoring {
            self.performance_monitor.get_performance_report().await
        } else {
            "Performance monitoring disabled".to_string()
        };
        let success_rate = if stats.total_requests > 0 {
            (stats.successful_requests as f32 / stats.total_requests as f32) * 100.0
        } else {
            0.0
        };
        format!(
            "🚀 Enhanced Tool System Statistics\n\n\
             📊 System Overview:\n\
             • Total requests: {}\n\
             • Success rate: {:.1}%\n\
             • Average response time: {:?}\n\
             • Intelligence assists: {}\n\
             • Optimization decisions: {}\n\n\
             {}\n\n{}\n\n{}",
            stats.total_requests,
            success_rate,
            stats.average_response_time,
            stats.intelligence_assists,
            stats.optimization_decisions,
            selector_stats,
            pipeline_stats,
            monitor_stats
        )
    }

    /// Get available tools list
    pub async fn list_tools(&self) -> Vec<ToolSpec> {
        let tools = self.tools.read().await;
        tools.values().map(|tool| tool.spec()).collect()
    }

    /// Check system health
    pub async fn health_check(&self) -> Result<String> {
        let stats = self.system_stats.lock().await;
        let tools_count = {
            let tools = self.tools.read().await;
            tools.len()
        };
        let health_status = if stats.total_requests > 0 {
            let success_rate =
                (stats.successful_requests as f32 / stats.total_requests as f32) * 100.0;
            match success_rate {
                rate if rate >= 95.0 => "Excellent",
                rate if rate >= 85.0 => "Good",
                rate if rate >= 70.0 => "Fair",
                _ => "Poor",
            }
        } else {
            "No data"
        };
        Ok(format!(
            "🏥 Enhanced Tool System Health Check\n\n\
             ✅ Status: {} ({})\n\
             🛠️ Registered tools: {}\n\
             📊 Total requests processed: {}\n\
             ⚡ Features enabled: {}\n\
             🔧 Configuration: {:?}",
            health_status,
            if stats.total_requests > 0 {
                format!(
                    "{:.1}% success rate",
                    (stats.successful_requests as f32 / stats.total_requests as f32) * 100.0
                )
            } else {
                "Not tested".to_string()
            },
            tools_count,
            stats.total_requests,
            self.count_enabled_features(),
            self.config.default_execution_strategy
        ))
    }
}

impl Default for EnhancedToolSystem {
    fn default() -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            Self::new(EnhancedToolSystemConfig::default())
                .await
                .unwrap()
        })
    }
}
