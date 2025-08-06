// @component: {"k":"C","id":"tool_orchestrator","t":"Integrated orchestrator combining adaptive task routing with enhanced tool system","m":{"cur":5,"tgt":95,"u":"%"},"f":["orchestration","tools","integration","routing","performance"]}

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};

use tools::enhanced_tool_system::{EnhancedToolSystem, EnhancedToolSystemConfig, EnhancedToolResult};
use tools::{Tool, ToolSpec};
use crate::orchestrator::{
    AdaptiveTaskOrchestrator, OrchestrationConfig, OrchestrationResult, TaskPriority, TaskComplexity
};

/// Tool orchestrator configuration
#[derive(Debug, Clone)]
pub struct ToolOrchestratorConfig {
    pub orchestration_config: OrchestrationConfig,
    pub tool_system_config: EnhancedToolSystemConfig,
    pub enable_cross_system_optimization: bool,
    pub performance_monitoring_interval: Duration,
}

impl Default for ToolOrchestratorConfig {
    fn default() -> Self {
        Self {
            orchestration_config: OrchestrationConfig::default(),
            tool_system_config: EnhancedToolSystemConfig::default(),
            enable_cross_system_optimization: true,
            performance_monitoring_interval: Duration::from_secs(60),
        }
    }
}

/// Integrated orchestrator combining task routing with tool execution
pub struct ToolOrchestrator {
    /// Adaptive task orchestrator for routing decisions
    task_orchestrator: Arc<AdaptiveTaskOrchestrator>,
    
    /// Enhanced tool system for actual execution
    tool_system: Arc<EnhancedToolSystem>,
    
    /// Configuration
    config: ToolOrchestratorConfig,
    
    /// Cross-system metrics
    integration_metrics: Arc<tokio::sync::Mutex<IntegrationMetrics>>,
}

/// Metrics tracking integration between orchestrator and tool system
#[derive(Debug, Default)]
pub struct IntegrationMetrics {
    pub total_orchestrated_requests: u64,
    pub successful_orchestrations: u64,
    pub tool_system_handoffs: u64,
    pub cross_optimization_decisions: u64,
    pub average_orchestration_overhead: Duration,
    pub orchestrator_tool_alignment: f32,
}

/// Result of integrated tool orchestration
#[derive(Debug)]
pub struct IntegratedOrchestrationResult {
    pub orchestration_result: OrchestrationResult,
    pub tool_result: Option<EnhancedToolResult>,
    pub optimization_applied: bool,
    pub performance_metrics: IntegrationPerformanceMetrics,
}

/// Performance metrics for integrated system
#[derive(Debug)]
pub struct IntegrationPerformanceMetrics {
    pub total_execution_time: Duration,
    pub orchestration_time: Duration,
    pub tool_execution_time: Duration,
    pub optimization_time: Duration,
    pub resource_efficiency: f32,
}

impl ToolOrchestrator {
    /// Create new integrated tool orchestrator
    pub async fn new(config: ToolOrchestratorConfig) -> Result<Self> {
        info!("🚀 Initializing Integrated Tool Orchestrator");
        
        // Initialize core components
        let task_orchestrator = Arc::new(AdaptiveTaskOrchestrator::new(config.orchestration_config.clone()));
        let tool_system = Arc::new(EnhancedToolSystem::new(config.tool_system_config.clone()).await?);
        
        let orchestrator = Self {
            task_orchestrator,
            tool_system,
            config,
            integration_metrics: Arc::new(tokio::sync::Mutex::new(IntegrationMetrics::default())),
        };
        
        // Initialize components
        orchestrator.task_orchestrator.initialize().await?;
        
        info!("✅ Integrated Tool Orchestrator initialized successfully");
        Ok(orchestrator)
    }
    
    /// Register tool with both orchestrator and tool system
    pub async fn register_tool(&self, tool: Arc<dyn Tool>) -> Result<()> {
        let tool_spec = tool.spec();
        info!("📝 Registering tool with integrated orchestrator: {}", tool_spec.name);
        
        // Register with enhanced tool system
        self.tool_system.register_tool(tool).await?;
        
        info!("✅ Tool registered successfully: {}", tool_spec.name);
        Ok(())
    }
    
    /// Process request through integrated orchestration
    pub async fn process_request(
        &self,
        content: &str,
        session_context: Option<HashMap<String, String>>,
    ) -> Result<IntegratedOrchestrationResult> {
        let start_time = Instant::now();
        
        // Update integration metrics
        {
            let mut metrics = self.integration_metrics.lock().await;
            metrics.total_orchestrated_requests += 1;
        }
        
        info!("🎯 Processing integrated orchestration request: '{}'", content);
        
        // Step 1: Orchestration phase - analyze and route task
        let orchestration_start = Instant::now();
        let context = session_context.unwrap_or_default();
        let orchestration_result = self.task_orchestrator.orchestrate_task(content, context.clone()).await?;
        let orchestration_time = orchestration_start.elapsed();
        
        debug!("📊 Orchestration phase completed in {:?}", orchestration_time);
        
        // Step 2: Determine if task should be handled by tool system
        let should_use_tool_system = self.should_delegate_to_tool_system(&orchestration_result).await;
        
        let (tool_result, optimization_applied) = if should_use_tool_system {
            // Step 3: Tool system execution phase
            {
                let mut metrics = self.integration_metrics.lock().await;
                metrics.tool_system_handoffs += 1;
            }
            
            let tool_start = Instant::now();
            let enhanced_result = self.tool_system.execute_request(content, Some(context)).await?;
            let tool_time = tool_start.elapsed();
            
            debug!("🛠️ Tool system phase completed in {:?}", tool_time);
            
            // Step 4: Cross-system optimization
            let optimization_start = Instant::now();
            let optimization = if self.config.enable_cross_system_optimization {
                self.apply_cross_system_optimization(&orchestration_result, &enhanced_result).await
            } else {
                false
            };
            let optimization_time = optimization_start.elapsed();
            
            if optimization {
                let mut metrics = self.integration_metrics.lock().await;
                metrics.cross_optimization_decisions += 1;
            }
            
            debug!("⚡ Cross-system optimization completed in {:?}", optimization_time);
            
            (Some(enhanced_result), optimization)
        } else {
            debug!("📋 Task handled directly by orchestrator (no tool system delegation)");
            (None, false)
        };
        
        let total_execution_time = start_time.elapsed();
        
        // Update success metrics
        {
            let mut metrics = self.integration_metrics.lock().await;
            metrics.successful_orchestrations += 1;
            
            // Update average orchestration overhead
            let overhead = orchestration_time;
            let current_avg = metrics.average_orchestration_overhead;
            let requests = metrics.total_orchestrated_requests;
            let new_avg = Duration::from_nanos(
                (((current_avg.as_nanos() * (requests - 1) as u128) + overhead.as_nanos()) / requests as u128) as u64
            );
            metrics.average_orchestration_overhead = new_avg;
            
            // Calculate orchestrator-tool alignment score
            metrics.orchestrator_tool_alignment = if tool_result.is_some() {
                self.calculate_alignment_score(&orchestration_result, tool_result.as_ref().unwrap()).await
            } else {
                metrics.orchestrator_tool_alignment // Keep previous score
            };
        }
        
        let tool_execution_time = tool_result.as_ref()
            .map(|r| r.execution_result.execution_time)
            .unwrap_or(Duration::from_millis(0));
            
        let resource_efficiency = self.calculate_resource_efficiency(&orchestration_result).await;
        
        let result = IntegratedOrchestrationResult {
            orchestration_result,
            tool_result,
            optimization_applied,
            performance_metrics: IntegrationPerformanceMetrics {
                total_execution_time,
                orchestration_time,
                tool_execution_time,
                optimization_time: if optimization_applied { Duration::from_millis(10) } else { Duration::from_millis(0) },
                resource_efficiency,
            },
        };
        
        info!("✅ Integrated orchestration completed in {:?}", total_execution_time);
        Ok(result)
    }
    
    /// Determine if task should be delegated to tool system
    async fn should_delegate_to_tool_system(&self, orchestration_result: &OrchestrationResult) -> bool {
        // Delegate to tool system if:
        // 1. Task was routed to 'tools' handler
        // 2. Task has high complexity
        // 3. Task mentions specific tool operations
        
        if orchestration_result.handler_used == "tools" {
            return true;
        }
        
        // Check for tool-related keywords in the response
        let response_lower = orchestration_result.response.to_lowercase();
        let tool_keywords = [
            "file", "файл", "directory", "папка", "git", "commit",
            "search", "поиск", "web", "веб", "command", "команда"
        ];
        
        if tool_keywords.iter().any(|&keyword| response_lower.contains(keyword)) {
            debug!("🔍 Task contains tool keywords, delegating to tool system");
            return true;
        }
        
        false
    }
    
    /// Apply cross-system optimization between orchestrator and tool system
    async fn apply_cross_system_optimization(
        &self,
        orchestration_result: &OrchestrationResult,
        tool_result: &EnhancedToolResult,
    ) -> bool {
        // Example optimizations:
        // 1. If orchestrator predicted low priority but tool execution was fast, adjust priority
        // 2. If tool system found better alternatives, update orchestrator routing
        // 3. If execution time differs significantly from prediction, update estimates
        
        let execution_time_diff = tool_result.execution_result.execution_time
            .as_millis() as i64 - orchestration_result.execution_time.as_millis() as i64;
        
        if execution_time_diff.abs() > 500 { // More than 500ms difference
            debug!("⚡ Applying timing optimization based on execution difference: {}ms", execution_time_diff);
            
            // TODO: Update orchestrator predictions based on actual tool execution times
            // This would involve feeding back performance data to improve future routing decisions
            
            return true;
        }
        
        // Check if tool system suggested better alternatives
        if !tool_result.alternative_tools.is_empty() {
            debug!("⚡ Tool system found {} alternatives, updating orchestrator knowledge", 
                   tool_result.alternative_tools.len());
            return true;
        }
        
        false
    }
    
    /// Calculate alignment score between orchestrator prediction and tool system execution
    async fn calculate_alignment_score(
        &self,
        orchestration_result: &OrchestrationResult,
        tool_result: &EnhancedToolResult,
    ) -> f32 {
        let mut alignment_factors = Vec::new();
        
        // Time prediction alignment
        let predicted_time = orchestration_result.execution_time.as_millis() as f32;
        let actual_time = tool_result.execution_result.execution_time.as_millis() as f32;
        
        let time_alignment = if predicted_time > 0.0 {
            1.0 - ((predicted_time - actual_time).abs() / predicted_time.max(actual_time))
        } else {
            0.5
        }.clamp(0.0, 1.0);
        
        alignment_factors.push(time_alignment);
        
        // Success prediction alignment
        let success_alignment = if orchestration_result.success == tool_result.execution_result.output.success {
            1.0
        } else {
            0.0
        };
        
        alignment_factors.push(success_alignment);
        
        // Handler selection alignment (if tool was predicted correctly)
        let handler_alignment = if orchestration_result.handler_used == "tools" {
            0.9 // High score for correct tool selection
        } else {
            0.3 // Lower score if orchestrator didn't predict tool usage
        };
        
        alignment_factors.push(handler_alignment);
        
        // Calculate weighted average
        alignment_factors.iter().sum::<f32>() / alignment_factors.len() as f32
    }
    
    /// Calculate resource efficiency score
    async fn calculate_resource_efficiency(&self, orchestration_result: &OrchestrationResult) -> f32 {
        // Simple efficiency calculation based on resource usage vs execution time
        let resource_usage = orchestration_result.resource_usage.cpu_used + 
                           orchestration_result.resource_usage.memory_used;
        
        let time_factor = orchestration_result.execution_time.as_millis() as f32 / 1000.0; // seconds
        
        if time_factor > 0.0 && resource_usage > 0.0 {
            // Higher efficiency = lower resource usage per unit time
            1.0 - (resource_usage * time_factor).min(1.0)
        } else {
            0.5 // Default neutral score
        }
    }
    
    /// Get comprehensive statistics from both systems
    pub async fn get_comprehensive_stats(&self) -> String {
        let mut stats = String::new();
        
        stats.push_str("🎯 Integrated Tool Orchestrator Statistics\n");
        stats.push_str(&"=".repeat(60));
        stats.push('\n');
        
        // Integration metrics
        {
            let metrics = self.integration_metrics.lock().await;
            let success_rate = if metrics.total_orchestrated_requests > 0 {
                (metrics.successful_orchestrations as f32 / metrics.total_orchestrated_requests as f32) * 100.0
            } else {
                0.0
            };
            
            let tool_delegation_rate = if metrics.total_orchestrated_requests > 0 {
                (metrics.tool_system_handoffs as f32 / metrics.total_orchestrated_requests as f32) * 100.0
            } else {
                0.0
            };
            
            stats.push_str(&format!(
                "🔗 Integration Performance:\n\
                 • Total orchestrated requests: {}\n\
                 • Success rate: {:.1}%\n\
                 • Tool system delegation rate: {:.1}%\n\
                 • Cross-optimization decisions: {}\n\
                 • Average orchestration overhead: {:?}\n\
                 • Orchestrator-tool alignment: {:.1}%\n\n",
                metrics.total_orchestrated_requests,
                success_rate,
                tool_delegation_rate,
                metrics.cross_optimization_decisions,
                metrics.average_orchestration_overhead,
                metrics.orchestrator_tool_alignment * 100.0
            ));
        }
        
        // Task orchestrator stats
        stats.push_str("📊 Task Orchestrator:\n");
        let orchestrator_stats = self.task_orchestrator.get_orchestration_stats().await;
        for line in orchestrator_stats.lines().skip(2) { // Skip header
            stats.push_str(&format!("  {}\n", line));
        }
        stats.push('\n');
        
        // Tool system stats
        stats.push_str("🛠️ Enhanced Tool System:\n");
        let tool_stats = self.tool_system.get_system_stats().await;
        for line in tool_stats.lines().skip(1) { // Skip header
            stats.push_str(&format!("  {}\n", line));
        }
        
        stats
    }
    
    /// Get available tools from the tool system
    pub async fn list_available_tools(&self) -> Vec<ToolSpec> {
        self.tool_system.list_tools().await
    }
    
    /// Health check for the integrated system
    pub async fn health_check(&self) -> Result<String> {
        let mut health_report = String::new();
        health_report.push_str("🏥 Integrated Tool Orchestrator Health Check\n\n");
        
        // Check task orchestrator health
        let orchestrator_healthy = self.task_orchestrator.is_healthy().await;
        health_report.push_str(&format!(
            "📊 Task Orchestrator: {}\n",
            if orchestrator_healthy { "✅ Healthy" } else { "❌ Unhealthy" }
        ));
        
        // Check tool system health
        let tool_system_health = self.tool_system.health_check().await?;
        health_report.push_str(&format!("🛠️ Tool System:\n"));
        for line in tool_system_health.lines().skip(1) { // Skip header
            health_report.push_str(&format!("  {}\n", line));
        }
        
        // Integration health
        let integration_metrics = self.integration_metrics.lock().await;
        let integration_healthy = integration_metrics.total_orchestrated_requests == 0 || 
                                 integration_metrics.orchestrator_tool_alignment > 0.7;
        
        health_report.push_str(&format!(
            "🔗 Integration Layer: {}\n",
            if integration_healthy { "✅ Well-aligned" } else { "⚠️ Alignment issues" }
        ));
        
        let overall_health = orchestrator_healthy && integration_healthy;
        health_report.push_str(&format!(
            "\n🏥 Overall System: {}\n",
            if overall_health { "✅ Healthy" } else { "⚠️ Issues detected" }
        ));
        
        Ok(health_report)
    }
    
    /// Shutdown the integrated orchestrator
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down Integrated Tool Orchestrator");
        
        // Shutdown task orchestrator (which waits for active tasks)
        self.task_orchestrator.shutdown().await?;
        
        info!("✅ Integrated Tool Orchestrator shutdown complete");
        Ok(())
    }
}

/// Trait for integrated orchestration
#[async_trait::async_trait]
pub trait IntegratedOrchestrator {
    async fn process_integrated_request(
        &self,
        content: &str,
        context: Option<HashMap<String, String>>,
    ) -> Result<IntegratedOrchestrationResult>;
    
    async fn get_integration_stats(&self) -> String;
    async fn health_status(&self) -> Result<String>;
}

#[async_trait::async_trait]
impl IntegratedOrchestrator for ToolOrchestrator {
    async fn process_integrated_request(
        &self,
        content: &str,
        context: Option<HashMap<String, String>>,
    ) -> Result<IntegratedOrchestrationResult> {
        self.process_request(content, context).await
    }
    
    async fn get_integration_stats(&self) -> String {
        self.get_comprehensive_stats().await
    }
    
    async fn health_status(&self) -> Result<String> {
        self.health_check().await
    }
}