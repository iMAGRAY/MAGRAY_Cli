// @component: {"k":"C","id":"strategy_selector","t":"Intelligent execution strategy selector","m":{"cur":5,"tgt":90,"u":"%"},"f":["strategy","selection","optimization","routing","adaptive"]}

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

use super::{TaskComplexity, TaskPriority};

/// Execution strategy types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionStrategy {
    /// Direct execution without optimization
    Direct,
    /// Batch similar tasks together
    Batched,
    /// Execute with priority queueing
    Prioritized,
    /// Use parallel execution where possible
    Parallel,
    /// Use adaptive load balancing
    LoadBalanced,
    /// Execute with full resource optimization
    Optimized,
}

/// Strategy selection criteria
#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub task_complexity: TaskComplexity,
    pub task_priority: TaskPriority,
    pub resource_availability: f32, // 0.0 to 1.0
    pub system_load: f32,           // 0.0 to 1.0
    pub queue_depth: usize,
    pub similar_tasks_count: usize,
    pub estimated_duration: Duration,
}

/// Strategy performance metrics
#[derive(Debug, Default, Clone)]
pub struct StrategyMetrics {
    pub total_selections: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub avg_execution_time_ms: f32,
    pub resource_efficiency: f32,
    pub strategy_usage: HashMap<ExecutionStrategy, u64>,
}

/// Intelligent strategy selector for task execution optimization
pub struct StrategySelector {
    strategy_weights: HashMap<ExecutionStrategy, f32>,
    performance_history: StrategyMetrics,
    adaptive_learning: bool,
}

impl StrategySelector {
    pub fn new() -> Self {
        let mut strategy_weights = HashMap::new();

        // Initial weights based on general performance characteristics
        strategy_weights.insert(ExecutionStrategy::Direct, 1.0);
        strategy_weights.insert(ExecutionStrategy::Batched, 1.2);
        strategy_weights.insert(ExecutionStrategy::Prioritized, 1.1);
        strategy_weights.insert(ExecutionStrategy::Parallel, 1.4);
        strategy_weights.insert(ExecutionStrategy::LoadBalanced, 1.3);
        strategy_weights.insert(ExecutionStrategy::Optimized, 1.5);

        info!("🧠 Initializing StrategySelector with adaptive learning enabled");

        Self {
            strategy_weights,
            performance_history: StrategyMetrics::default(),
            adaptive_learning: true,
        }
    }

    /// Select optimal execution strategy based on current conditions
    pub async fn select_strategy(
        &mut self,
        criteria: &SelectionCriteria,
    ) -> Result<ExecutionStrategy> {
        debug!(
            "🎯 Selecting execution strategy for task: complexity={:?}, priority={:?}, load={:.1}%",
            criteria.task_complexity,
            criteria.task_priority,
            criteria.system_load * 100.0
        );

        self.performance_history.total_selections += 1;

        let strategy = self.analyze_optimal_strategy(criteria).await;

        // Update usage statistics
        let count = self
            .performance_history
            .strategy_usage
            .entry(strategy.clone())
            .or_insert(0);
        *count += 1;

        info!(
            "✅ Selected strategy: {:?} (confidence: {:.1}%)",
            strategy,
            self.calculate_confidence(&strategy, criteria) * 100.0
        );

        Ok(strategy)
    }

    /// Analyze and determine optimal strategy based on current conditions
    async fn analyze_optimal_strategy(&self, criteria: &SelectionCriteria) -> ExecutionStrategy {
        let mut scores = HashMap::new();

        scores.insert(
            ExecutionStrategy::Direct,
            self.score_direct_strategy(criteria),
        );
        scores.insert(
            ExecutionStrategy::Batched,
            self.score_batched_strategy(criteria),
        );
        scores.insert(
            ExecutionStrategy::Prioritized,
            self.score_prioritized_strategy(criteria),
        );
        scores.insert(
            ExecutionStrategy::Parallel,
            self.score_parallel_strategy(criteria),
        );
        scores.insert(
            ExecutionStrategy::LoadBalanced,
            self.score_load_balanced_strategy(criteria),
        );
        scores.insert(
            ExecutionStrategy::Optimized,
            self.score_optimized_strategy(criteria),
        );

        if self.adaptive_learning {
            for (strategy, score) in scores.iter_mut() {
                if let Some(weight) = self.strategy_weights.get(strategy) {
                    *score *= weight;
                }
            }
        }

        // Find strategy with highest score
        scores
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(strategy, _)| strategy)
            .unwrap_or(ExecutionStrategy::Direct)
    }

    /// Score direct execution strategy
    fn score_direct_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        match criteria.task_complexity {
            TaskComplexity::Simple => score += 0.5,
            TaskComplexity::Medium => score += 0.2,
            _ => score -= 0.3,
        }

        if criteria.system_load < 0.3 {
            score += 0.3;
        }

        if criteria.queue_depth < 3 {
            score += 0.2;
        }

        score
    }

    /// Score batched execution strategy
    fn score_batched_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        // Strongly favor batching when there are similar tasks
        if criteria.similar_tasks_count > 2 {
            score += 1.0 + (criteria.similar_tasks_count as f32 * 0.2);
        } else {
            score -= 0.5; // Penalize if no similar tasks
        }

        match criteria.task_complexity {
            TaskComplexity::Medium => score += 0.3,
            TaskComplexity::Complex => score += 0.2,
            _ => score -= 0.1,
        }

        score
    }

    /// Score prioritized execution strategy
    fn score_prioritized_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        match criteria.task_priority {
            TaskPriority::Emergency => score += 2.0,
            TaskPriority::Critical => score += 1.5,
            TaskPriority::High => score += 1.0,
            TaskPriority::Normal => score += 0.2,
            TaskPriority::Low => score -= 0.5,
        }

        // Favor when queue has mixed priorities
        if criteria.queue_depth > 5 {
            score += 0.5;
        }

        score
    }

    /// Score parallel execution strategy
    fn score_parallel_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        match criteria.task_complexity {
            TaskComplexity::Complex => score += 0.8,
            TaskComplexity::Expert => score += 1.2,
            TaskComplexity::Medium => score += 0.4,
            _ => score -= 0.3,
        }

        // Favor when resources are available
        if criteria.resource_availability > 0.6 {
            score += 0.6;
        } else if criteria.resource_availability < 0.3 {
            score -= 0.8;
        }

        if criteria.estimated_duration > Duration::from_millis(500)
            && criteria.estimated_duration < Duration::from_secs(30)
        {
            score += 0.4;
        }

        score
    }

    /// Score load balanced execution strategy
    fn score_load_balanced_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        // Strongly favor when system load is high
        if criteria.system_load > 0.7 {
            score += 1.0;
        } else if criteria.system_load > 0.5 {
            score += 0.5;
        }

        // Favor when queue is deep
        if criteria.queue_depth > 8 {
            score += 0.7;
        }

        match criteria.task_complexity {
            TaskComplexity::Complex | TaskComplexity::Expert => score += 0.3,
            _ => score += 0.1,
        }

        score
    }

    /// Score optimized execution strategy
    fn score_optimized_strategy(&self, criteria: &SelectionCriteria) -> f32 {
        let mut score = 1.0;

        match criteria.task_complexity {
            TaskComplexity::Expert => score += 1.5,
            TaskComplexity::Complex => score += 0.8,
            TaskComplexity::Medium => score += 0.3,
            _ => score -= 0.2,
        }

        match criteria.task_priority {
            TaskPriority::Critical | TaskPriority::Emergency => score += 0.8,
            TaskPriority::High => score += 0.4,
            _ => score += 0.1,
        }

        if criteria.resource_availability > 0.5 {
            score += 0.5;
        } else {
            score -= 0.3;
        }

        score
    }

    /// Calculate confidence level for strategy selection
    fn calculate_confidence(
        &self,
        strategy: &ExecutionStrategy,
        criteria: &SelectionCriteria,
    ) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Increase confidence based on clear indicators
        match (strategy, &criteria.task_priority, &criteria.task_complexity) {
            (ExecutionStrategy::Direct, _, TaskComplexity::Simple) => confidence += 0.3,
            (ExecutionStrategy::Batched, _, _) if criteria.similar_tasks_count > 3 => {
                confidence += 0.4
            }
            (ExecutionStrategy::Prioritized, TaskPriority::Emergency, _) => confidence += 0.4,
            (ExecutionStrategy::Parallel, _, TaskComplexity::Expert)
                if criteria.resource_availability > 0.7 =>
            {
                confidence += 0.4
            }
            (ExecutionStrategy::LoadBalanced, _, _) if criteria.system_load > 0.8 => {
                confidence += 0.3
            }
            (ExecutionStrategy::Optimized, TaskPriority::Critical, TaskComplexity::Expert) => {
                confidence += 0.5
            }
            _ => confidence += 0.1,
        }

        if self.performance_history.total_selections > 10 {
            let success_rate = self.performance_history.successful_executions as f32
                / self.performance_history.total_selections as f32;
            confidence = (confidence + success_rate) / 2.0;
        }

        confidence.min(1.0)
    }

    /// Update performance metrics after task execution
    pub async fn update_metrics(
        &mut self,
        strategy: &ExecutionStrategy,
        success: bool,
        execution_time: Duration,
    ) {
        if success {
            self.performance_history.successful_executions += 1;
        } else {
            self.performance_history.failed_executions += 1;
            warn!("❌ Strategy {:?} execution failed", strategy);
        }

        // Update average execution time
        let new_time_ms = execution_time.as_millis() as f32;
        let total_executions = self.performance_history.successful_executions
            + self.performance_history.failed_executions;

        if total_executions > 1 {
            self.performance_history.avg_execution_time_ms =
                ((self.performance_history.avg_execution_time_ms * (total_executions - 1) as f32)
                    + new_time_ms)
                    / total_executions as f32;
        } else {
            self.performance_history.avg_execution_time_ms = new_time_ms;
        }

        // Adaptive learning: adjust strategy weights based on performance
        if self.adaptive_learning && total_executions.is_multiple_of(10) {
            self.adjust_strategy_weights(strategy, success, execution_time)
                .await;
        }

        debug!(
            "📊 Updated metrics for strategy {:?}: success={}, time={:.1}ms",
            strategy, success, new_time_ms
        );
    }

    /// Adjust strategy weights based on performance (adaptive learning)
    async fn adjust_strategy_weights(
        &mut self,
        strategy: &ExecutionStrategy,
        success: bool,
        execution_time: Duration,
    ) {
        let current_weight = self.strategy_weights.get(strategy).cloned().unwrap_or(1.0);
        let adjustment = if success {
            if execution_time < Duration::from_millis(100) {
                0.05 // Small boost for fast successful executions
            } else if execution_time < Duration::from_secs(1) {
                0.02 // Tiny boost for reasonable executions
            } else {
                -0.01 // Slight penalty for slow executions
            }
        } else {
            -0.1 // Penalty for failures
        };

        let new_weight = (current_weight + adjustment).clamp(0.1, 2.0); // Clamp between 0.1 and 2.0
        self.strategy_weights.insert(strategy.clone(), new_weight);

        debug!(
            "🧠 Adaptive learning: adjusted {:?} weight from {:.2} to {:.2}",
            strategy, current_weight, new_weight
        );
    }

    /// Get strategy selection statistics
    pub fn get_strategy_stats(&self) -> String {
        let total = self.performance_history.total_selections;
        let success_rate = if total > 0 {
            (self.performance_history.successful_executions as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let mut stats = format!(
            "🧠 Strategy Selector Statistics:\n\n\
             📊 Performance Overview:\n\
             • Total selections: {}\n\
             • Success rate: {:.1}%\n\
             • Avg execution time: {:.1}ms\n\
             • Adaptive learning: {}\n\n\
             📈 Strategy Usage:",
            total,
            success_rate,
            self.performance_history.avg_execution_time_ms,
            if self.adaptive_learning {
                "Enabled"
            } else {
                "Disabled"
            }
        );

        // Sort strategies by usage
        let mut usage_vec: Vec<_> = self.performance_history.strategy_usage.iter().collect();
        usage_vec.sort_by(|a, b| b.1.cmp(a.1));

        let usage_vec_clone = usage_vec.clone();

        for (strategy, count) in usage_vec {
            let percentage = if total > 0 {
                (*count as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            let weight = self.strategy_weights.get(strategy).unwrap_or(&1.0);
            stats.push_str(&format!(
                "\n • {:?}: {} ({:.1}%) [weight: {:.2}]",
                strategy, count, percentage, weight
            ));
        }

        if self.adaptive_learning && total > 50 {
            stats.push_str("\n\n🚀 Optimization Recommendations:");

            let best_strategy = usage_vec_clone
                .first()
                .map(|(s, _)| s)
                .unwrap_or(&&ExecutionStrategy::Direct);

            stats.push_str(&format!("\n • Most successful: {:?}", best_strategy));

            if success_rate < 80.0 {
                stats.push_str("\n • Consider reviewing task complexity analysis");
            }

            if self.performance_history.avg_execution_time_ms > 1000.0 {
                stats.push_str("\n • High execution times detected - review resource allocation");
            }
        }

        stats
    }

    /// Reset performance metrics (for testing or maintenance)
    pub fn reset_metrics(&mut self) {
        self.performance_history = StrategyMetrics::default();
        info!("🔄 Strategy selector metrics reset");
    }

    /// Enable or disable adaptive learning
    pub fn set_adaptive_learning(&mut self, enabled: bool) {
        self.adaptive_learning = enabled;
        info!(
            "🧠 Adaptive learning {}",
            if enabled { "enabled" } else { "disabled" }
        );
    }
}

impl Default for StrategySelector {
    fn default() -> Self {
        Self::new()
    }
}
