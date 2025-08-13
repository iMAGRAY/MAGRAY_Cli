use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::executor::{ExecutionResult, ExecutionStatus, StepResult, StepStatus};

// Health monitoring integration
use crate::reliability::health::{HealthChecker, HealthReport, HealthStatus};

/// Result of critic evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticFeedback {
    pub evaluation_id: Uuid,
    pub execution_id: Uuid,
    pub overall_score: f64,
    pub quality_metrics: QualityMetrics,
    pub improvement_suggestions: Vec<ImprovementSuggestion>,
    pub success_indicators: Vec<SuccessIndicator>,
    pub risk_assessment: RiskAssessment,
    pub recommendations: Vec<Recommendation>,
}

/// Quality metrics for execution evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub execution_efficiency: f64,
    pub resource_utilization: f64,
    pub error_rate: f64,
    pub completion_rate: f64,
    pub response_time_score: f64,
    pub reliability_score: f64,
}

/// Improvement suggestion from critic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementSuggestion {
    pub category: ImprovementCategory,
    pub priority: Priority,
    pub description: String,
    pub suggested_action: String,
    pub expected_impact: f64,
}

/// Categories of improvements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImprovementCategory {
    Performance,
    Reliability,
    ResourceUsage,
    ErrorHandling,
    UserExperience,
    Security,
    Maintainability,
}

/// Priority levels for suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Success indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessIndicator {
    pub metric: String,
    pub actual_value: f64,
    pub expected_value: f64,
    pub passed: bool,
    pub importance: f64,
}

/// Risk assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk_level: RiskLevel,
    pub identified_risks: Vec<Risk>,
    pub mitigation_suggestions: Vec<String>,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Individual risk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub risk_type: RiskType,
    pub description: String,
    pub probability: f64,
    pub impact: f64,
    pub severity: RiskLevel,
}

/// Types of risks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskType {
    DataLoss,
    SecurityBreach,
    PerformanceDegradation,
    ResourceExhaustion,
    SystemFailure,
    UserImpact,
}

/// Recommendation from critic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub recommendation_type: RecommendationType,
    pub description: String,
    pub implementation_effort: EffortLevel,
    pub expected_benefit: f64,
    pub urgency: Priority,
}

/// Types of recommendations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationType {
    Optimize,
    Refactor,
    Monitor,
    Alert,
    Document,
    Test,
    Rollback,
    Investigate,
}

/// Effort levels for implementation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EffortLevel {
    Minimal,
    Low,
    Medium,
    High,
    Extensive,
}

/// Trait for critic functionality
#[async_trait]
pub trait CriticTrait: Send + Sync {
    /// Evaluate execution results and provide feedback
    async fn evaluate_result(&self, result: &ExecutionResult) -> Result<CriticFeedback>;

    /// Analyze step-level performance
    async fn analyze_step_performance(&self, step: &StepResult) -> Result<StepAnalysis>;

    /// Generate improvement recommendations
    async fn generate_recommendations(
        &self,
        feedback: &CriticFeedback,
    ) -> Result<Vec<Recommendation>>;

    /// Assess execution risks
    async fn assess_risks(&self, result: &ExecutionResult) -> Result<RiskAssessment>;
}

/// Step-level analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepAnalysis {
    pub step_id: Uuid,
    pub performance_score: f64,
    pub efficiency_metrics: HashMap<String, f64>,
    pub issues_found: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Critic implementation
pub struct Critic {
    agent_id: Uuid,
    performance_thresholds: HashMap<String, f64>,
    quality_weights: HashMap<String, f64>,
    // Health monitoring fields
    last_heartbeat: Arc<RwLock<Option<DateTime<Utc>>>>,
    error_count: Arc<AtomicU32>,
    start_time: Instant,
}

impl Critic {
    /// Create new Critic instance
    pub fn new() -> Self {
        let mut performance_thresholds = HashMap::new();
        performance_thresholds.insert("max_execution_time_ms".to_string(), 30000.0);
        performance_thresholds.insert("max_memory_mb".to_string(), 512.0);
        performance_thresholds.insert("max_error_rate".to_string(), 0.1);
        performance_thresholds.insert("min_completion_rate".to_string(), 0.9);

        let mut quality_weights = HashMap::new();
        quality_weights.insert("efficiency".to_string(), 0.25);
        quality_weights.insert("reliability".to_string(), 0.30);
        quality_weights.insert("resource_usage".to_string(), 0.20);
        quality_weights.insert("response_time".to_string(), 0.15);
        quality_weights.insert("error_handling".to_string(), 0.10);

        Self {
            agent_id: Uuid::new_v4(),
            performance_thresholds,
            quality_weights,
            // Health monitoring fields
            last_heartbeat: Arc::new(RwLock::new(Some(Utc::now()))),
            error_count: Arc::new(AtomicU32::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Start automatic heartbeat loop for health monitoring
    /// This prevents timeout issues by sending heartbeat every 30 seconds
    pub fn start_heartbeat_loop(&self) {
        let last_heartbeat = Arc::clone(&self.last_heartbeat);
        let agent_id = self.agent_id;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval.tick().await;

                // Update heartbeat timestamp
                {
                    let mut heartbeat = last_heartbeat.write().await;
                    *heartbeat = Some(Utc::now());
                }

                tracing::debug!(
                    agent_id = %agent_id,
                    agent_type = "Critic",
                    "Heartbeat sent"
                );
            }
        });

        tracing::info!(
            agent_id = %self.agent_id,
            agent_type = "Critic",
            "Heartbeat loop started with 30s interval"
        );
    }

    /// Calculate quality metrics from execution result
    fn calculate_quality_metrics(&self, result: &ExecutionResult) -> QualityMetrics {
        let total_steps = result.step_results.len() as f64;
        let successful_steps = result
            .step_results
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count() as f64;
        let failed_steps = result
            .step_results
            .iter()
            .filter(|s| s.status == StepStatus::Failed)
            .count() as f64;

        let completion_rate = if total_steps > 0.0 {
            successful_steps / total_steps
        } else {
            1.0
        };
        let error_rate = if total_steps > 0.0 {
            failed_steps / total_steps
        } else {
            0.0
        };

        // Calculate execution efficiency (inverse of execution time relative to estimated)
        let execution_efficiency = if result.execution_time.as_millis() > 0 {
            // Simplified calculation - would use estimated time in real implementation
            let baseline_ms = 1000.0; // 1 second baseline
            let actual_ms = result.execution_time.as_millis() as f64;
            (baseline_ms / actual_ms).min(1.0)
        } else {
            1.0
        };

        // Calculate resource utilization score
        let memory_threshold = self
            .performance_thresholds
            .get("max_memory_mb")
            .copied()
            .unwrap_or(512.0);
        let resource_utilization =
            1.0 - (result.resource_usage.memory_peak_mb as f64 / memory_threshold).min(1.0);

        // Response time score (higher is better, based on execution time)
        let max_time_threshold = self
            .performance_thresholds
            .get("max_execution_time_ms")
            .copied()
            .unwrap_or(30000.0);
        let actual_time_ms = result.execution_time.as_millis() as f64;
        let response_time_score = if actual_time_ms <= max_time_threshold {
            1.0 - (actual_time_ms / max_time_threshold)
        } else {
            0.0
        };

        // Reliability score (based on completion rate and retry patterns)
        let avg_retries = if total_steps > 0.0 {
            result
                .step_results
                .iter()
                .map(|s| s.retry_count as f64)
                .sum::<f64>()
                / total_steps
        } else {
            0.0
        };
        let reliability_score = completion_rate * (1.0 - (avg_retries * 0.1).min(0.5));

        QualityMetrics {
            execution_efficiency,
            resource_utilization,
            error_rate,
            completion_rate,
            response_time_score,
            reliability_score,
        }
    }

    /// Generate improvement suggestions based on metrics
    fn generate_improvement_suggestions(
        &self,
        metrics: &QualityMetrics,
        _result: &ExecutionResult,
    ) -> Vec<ImprovementSuggestion> {
        let mut suggestions = Vec::new();

        // Performance suggestions
        if metrics.execution_efficiency < 0.7 {
            suggestions.push(ImprovementSuggestion {
                category: ImprovementCategory::Performance,
                priority: Priority::High,
                description: "Execution time is slower than expected".to_string(),
                suggested_action:
                    "Optimize critical path operations and consider parallel execution".to_string(),
                expected_impact: 0.3,
            });
        }

        // Resource usage suggestions
        if metrics.resource_utilization < 0.5 {
            suggestions.push(ImprovementSuggestion {
                category: ImprovementCategory::ResourceUsage,
                priority: Priority::Medium,
                description: "High resource utilization detected".to_string(),
                suggested_action: "Implement resource pooling and optimize memory usage"
                    .to_string(),
                expected_impact: 0.25,
            });
        }

        // Error handling suggestions
        if metrics.error_rate > 0.1 {
            suggestions.push(ImprovementSuggestion {
                category: ImprovementCategory::ErrorHandling,
                priority: Priority::Critical,
                description: "High error rate indicates reliability issues".to_string(),
                suggested_action: "Improve error handling and retry logic".to_string(),
                expected_impact: 0.4,
            });
        }

        // Reliability suggestions
        if metrics.reliability_score < 0.8 {
            suggestions.push(ImprovementSuggestion {
                category: ImprovementCategory::Reliability,
                priority: Priority::High,
                description: "Reliability score below acceptable threshold".to_string(),
                suggested_action: "Implement circuit breakers and better fallback mechanisms"
                    .to_string(),
                expected_impact: 0.35,
            });
        }

        suggestions
    }

    /// Generate success indicators
    fn generate_success_indicators(
        &self,
        metrics: &QualityMetrics,
        _result: &ExecutionResult,
    ) -> Vec<SuccessIndicator> {
        vec![
            SuccessIndicator {
                metric: "Completion Rate".to_string(),
                actual_value: metrics.completion_rate,
                expected_value: 0.95,
                passed: metrics.completion_rate >= 0.95,
                importance: 0.9,
            },
            SuccessIndicator {
                metric: "Error Rate".to_string(),
                actual_value: metrics.error_rate,
                expected_value: 0.05,
                passed: metrics.error_rate <= 0.05,
                importance: 0.8,
            },
            SuccessIndicator {
                metric: "Execution Efficiency".to_string(),
                actual_value: metrics.execution_efficiency,
                expected_value: 0.8,
                passed: metrics.execution_efficiency >= 0.8,
                importance: 0.7,
            },
            SuccessIndicator {
                metric: "Resource Utilization".to_string(),
                actual_value: metrics.resource_utilization,
                expected_value: 0.7,
                passed: metrics.resource_utilization >= 0.7,
                importance: 0.6,
            },
        ]
    }

    /// Calculate overall score from quality metrics
    fn calculate_overall_score(&self, metrics: &QualityMetrics) -> f64 {
        let efficiency_weight = self
            .quality_weights
            .get("efficiency")
            .copied()
            .unwrap_or(0.25);
        let reliability_weight = self
            .quality_weights
            .get("reliability")
            .copied()
            .unwrap_or(0.30);
        let resource_weight = self
            .quality_weights
            .get("resource_usage")
            .copied()
            .unwrap_or(0.20);
        let response_weight = self
            .quality_weights
            .get("response_time")
            .copied()
            .unwrap_or(0.15);
        let error_weight = self
            .quality_weights
            .get("error_handling")
            .copied()
            .unwrap_or(0.10);

        let score = (metrics.execution_efficiency * efficiency_weight)
            + (metrics.reliability_score * reliability_weight)
            + (metrics.resource_utilization * resource_weight)
            + (metrics.response_time_score * response_weight)
            + ((1.0 - metrics.error_rate) * error_weight);

        score.clamp(0.0, 1.0)
    }
}

#[async_trait]
impl CriticTrait for Critic {
    async fn evaluate_result(&self, result: &ExecutionResult) -> Result<CriticFeedback> {
        tracing::debug!("Evaluating execution result for plan {}", result.plan_id);

        let quality_metrics = self.calculate_quality_metrics(result);
        let overall_score = self.calculate_overall_score(&quality_metrics);
        let improvement_suggestions =
            self.generate_improvement_suggestions(&quality_metrics, result);
        let success_indicators = self.generate_success_indicators(&quality_metrics, result);
        let risk_assessment = self.assess_risks(result).await?;

        let recommendations = vec![Recommendation {
            recommendation_type: RecommendationType::Monitor,
            description: "Continue monitoring execution patterns".to_string(),
            implementation_effort: EffortLevel::Minimal,
            expected_benefit: 0.1,
            urgency: Priority::Low,
        }];

        Ok(CriticFeedback {
            evaluation_id: Uuid::new_v4(),
            execution_id: result.plan_id,
            overall_score,
            quality_metrics,
            improvement_suggestions,
            success_indicators,
            risk_assessment,
            recommendations,
        })
    }

    async fn analyze_step_performance(&self, step: &StepResult) -> Result<StepAnalysis> {
        let mut efficiency_metrics = HashMap::new();
        let mut issues_found = Vec::new();
        let mut suggestions = Vec::new();

        // Calculate execution time efficiency
        let execution_time_ms = step.execution_time.as_millis() as f64;
        efficiency_metrics.insert("execution_time_ms".to_string(), execution_time_ms);

        // Analyze retry patterns
        if step.retry_count > 0 {
            efficiency_metrics.insert("retry_count".to_string(), step.retry_count as f64);
            issues_found.push(format!("Step required {} retries", step.retry_count));
            suggestions
                .push("Investigate root cause of failures and improve reliability".to_string());
        }

        // Performance scoring
        let base_score = match step.status {
            StepStatus::Completed => 1.0,
            StepStatus::Failed => 0.0,
            StepStatus::Skipped => 0.5,
            _ => 0.3,
        };

        let retry_penalty = (step.retry_count as f64) * 0.1;
        let performance_score = (base_score - retry_penalty).max(0.0);

        if performance_score < 0.8 {
            issues_found.push("Below optimal performance threshold".to_string());
        }

        Ok(StepAnalysis {
            step_id: step.step_id,
            performance_score,
            efficiency_metrics,
            issues_found,
            suggestions,
        })
    }

    async fn generate_recommendations(
        &self,
        feedback: &CriticFeedback,
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on overall score
        if feedback.overall_score < 0.5 {
            recommendations.push(Recommendation {
                recommendation_type: RecommendationType::Investigate,
                description: "Low overall score requires investigation".to_string(),
                implementation_effort: EffortLevel::Medium,
                expected_benefit: 0.4,
                urgency: Priority::High,
            });
        }

        // Generate recommendations based on quality metrics
        if feedback.quality_metrics.error_rate > 0.2 {
            recommendations.push(Recommendation {
                recommendation_type: RecommendationType::Refactor,
                description: "High error rate suggests need for better error handling".to_string(),
                implementation_effort: EffortLevel::High,
                expected_benefit: 0.5,
                urgency: Priority::Critical,
            });
        }

        if feedback.quality_metrics.execution_efficiency < 0.6 {
            recommendations.push(Recommendation {
                recommendation_type: RecommendationType::Optimize,
                description: "Performance optimization needed".to_string(),
                implementation_effort: EffortLevel::Medium,
                expected_benefit: 0.3,
                urgency: Priority::Medium,
            });
        }

        Ok(recommendations)
    }

    async fn assess_risks(&self, result: &ExecutionResult) -> Result<RiskAssessment> {
        let mut identified_risks = Vec::new();
        let mut mitigation_suggestions = Vec::new();

        // Assess based on execution status
        match result.status {
            ExecutionStatus::Failed => {
                identified_risks.push(Risk {
                    risk_type: RiskType::SystemFailure,
                    description: "Execution failed completely".to_string(),
                    probability: 1.0,
                    impact: 0.8,
                    severity: RiskLevel::High,
                });
                mitigation_suggestions
                    .push("Implement better error recovery mechanisms".to_string());
            }
            ExecutionStatus::Cancelled => {
                identified_risks.push(Risk {
                    risk_type: RiskType::UserImpact,
                    description: "User cancelled execution".to_string(),
                    probability: 0.3,
                    impact: 0.5,
                    severity: RiskLevel::Medium,
                });
            }
            _ => {}
        }

        // Assess resource usage risks
        if result.resource_usage.memory_peak_mb > 512 {
            identified_risks.push(Risk {
                risk_type: RiskType::ResourceExhaustion,
                description: "High memory usage detected".to_string(),
                probability: 0.6,
                impact: 0.7,
                severity: RiskLevel::Medium,
            });
            mitigation_suggestions.push("Implement memory limits and monitoring".to_string());
        }

        // Assess performance risks
        if result.execution_time.as_secs() > 60 {
            identified_risks.push(Risk {
                risk_type: RiskType::PerformanceDegradation,
                description: "Execution time exceeds acceptable limits".to_string(),
                probability: 0.7,
                impact: 0.6,
                severity: RiskLevel::Medium,
            });
            mitigation_suggestions
                .push("Optimize critical path and consider timeout mechanisms".to_string());
        }

        // Calculate overall risk level
        let overall_risk_level = if identified_risks
            .iter()
            .any(|r| r.severity == RiskLevel::Critical)
        {
            RiskLevel::Critical
        } else if identified_risks
            .iter()
            .any(|r| r.severity == RiskLevel::High)
        {
            RiskLevel::High
        } else if identified_risks
            .iter()
            .any(|r| r.severity == RiskLevel::Medium)
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(RiskAssessment {
            overall_risk_level,
            identified_risks,
            mitigation_suggestions,
        })
    }
}

impl Default for Critic {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::executor::{
        ExecutionResult, ExecutionStatus, ResourceUsage, StepResult, StepStatus,
    };

    fn create_test_execution_result() -> ExecutionResult {
        ExecutionResult {
            plan_id: Uuid::new_v4(),
            status: ExecutionStatus::Completed,
            step_results: vec![StepResult {
                step_id: Uuid::new_v4(),
                status: StepStatus::Completed,
                output: Some(serde_json::json!({"result": "success"})),
                error: None,
                execution_time: std::time::Duration::from_millis(100),
                retry_count: 0,
                metadata: HashMap::new(),
            }],
            execution_time: std::time::Duration::from_millis(500),
            resource_usage: ResourceUsage {
                cpu_time_ms: 400,
                memory_peak_mb: 128,
                disk_reads: 5,
                disk_writes: 2,
                network_requests: 1,
                tool_invocations: 1,
            },
            metadata: HashMap::new(),
            error: None,
        }
    }

    #[tokio::test]
    async fn test_evaluate_successful_result() {
        let critic = Critic::new();
        let result = create_test_execution_result();

        let feedback = critic
            .evaluate_result(&result)
            .await
            .expect("Async operation should succeed");

        assert_eq!(feedback.execution_id, result.plan_id);
        assert!(feedback.overall_score > 0.5);
        assert_eq!(feedback.quality_metrics.completion_rate, 1.0);
        assert_eq!(feedback.quality_metrics.error_rate, 0.0);
    }

    #[tokio::test]
    async fn test_analyze_step_performance() {
        let critic = Critic::new();
        let step = StepResult {
            step_id: Uuid::new_v4(),
            status: StepStatus::Completed,
            output: Some(serde_json::json!({"result": "success"})),
            error: None,
            execution_time: std::time::Duration::from_millis(100),
            retry_count: 0,
            metadata: HashMap::new(),
        };

        let analysis = critic
            .analyze_step_performance(&step)
            .await
            .expect("Async operation should succeed");

        assert_eq!(analysis.step_id, step.step_id);
        assert_eq!(analysis.performance_score, 1.0);
        assert!(analysis.issues_found.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_failed_step() {
        let critic = Critic::new();
        let step = StepResult {
            step_id: Uuid::new_v4(),
            status: StepStatus::Failed,
            output: None,
            error: Some("Tool not found".to_string()),
            execution_time: std::time::Duration::from_millis(50),
            retry_count: 2,
            metadata: HashMap::new(),
        };

        let analysis = critic
            .analyze_step_performance(&step)
            .await
            .expect("Async operation should succeed");

        assert_eq!(analysis.performance_score, 0.0);
        assert!(!analysis.issues_found.is_empty());
        assert!(!analysis.suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_assess_risks_failed_execution() {
        let critic = Critic::new();
        let mut result = create_test_execution_result();
        result.status = ExecutionStatus::Failed;

        let risk_assessment = critic
            .assess_risks(&result)
            .await
            .expect("Async operation should succeed");

        assert_eq!(risk_assessment.overall_risk_level, RiskLevel::High);
        assert!(!risk_assessment.identified_risks.is_empty());
        assert!(!risk_assessment.mitigation_suggestions.is_empty());
    }
}

/// HealthChecker implementation for Critic
#[async_trait]
impl HealthChecker for Critic {
    fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    fn agent_name(&self) -> &str {
        "Critic"
    }

    fn agent_type(&self) -> &str {
        "Critic"
    }

    async fn check_health(&self) -> Result<HealthReport> {
        let last_heartbeat = *self.last_heartbeat.read().await;
        let error_count = self.error_count.load(Ordering::Relaxed);
        let uptime = self.start_time.elapsed().as_secs();

        // Agent-specific health checks
        let thresholds_count = self.performance_thresholds.len();
        let weights_count = self.quality_weights.len();

        // Determine health status based on errors and configuration
        let status = if error_count > 25 {
            HealthStatus::Unhealthy {
                reason: format!("High error count: {}", error_count),
            }
        } else if error_count > 12 || thresholds_count == 0 {
            HealthStatus::Degraded {
                reason: format!(
                    "Moderate error count: {} or missing configuration",
                    error_count
                ),
            }
        } else {
            HealthStatus::Healthy
        };

        Ok(HealthReport {
            agent_id: self.agent_id,
            agent_name: "Critic".to_string(),
            agent_type: "Critic".to_string(),
            status,
            timestamp: Utc::now(),
            last_heartbeat,
            response_time_ms: Some(30),   // Analysis can take some time
            memory_usage_mb: Some(60),    // Estimated memory for analysis
            cpu_usage_percent: Some(8.0), // Analysis requires some CPU
            active_tasks: 0,              // Critic processes results on demand
            error_count,
            restart_count: 0, // Track restarts in future implementation
            uptime_seconds: uptime,
            metadata: serde_json::json!({
                "performance_thresholds_count": thresholds_count,
                "quality_weights_count": weights_count,
                "analysis_model_available": true // Critic uses built-in analysis
            }),
        })
    }

    async fn heartbeat(&self) -> Result<()> {
        let mut heartbeat = self.last_heartbeat.write().await;
        *heartbeat = Some(Utc::now());
        Ok(())
    }

    fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
        // Use try_read for synchronous access needed by the trait
        self.last_heartbeat.try_read().ok().and_then(|guard| *guard)
    }

    fn is_healthy(&self) -> bool {
        let error_count = self.error_count.load(Ordering::Relaxed);
        error_count <= 25 && !self.performance_thresholds.is_empty()
    }

    async fn restart(&self) -> Result<()> {
        // Reset error count and update heartbeat
        self.error_count.store(0, Ordering::Relaxed);
        {
            let mut heartbeat = self.last_heartbeat.write().await;
            *heartbeat = Some(Utc::now());
        }

        // Reinitialize configuration if needed (critic is stateless)
        // No active state to clear for Critic

        Ok(())
    }
}

/// BaseActor implementation for Critic
#[async_trait::async_trait]
impl crate::actors::BaseActor for Critic {
    fn id(&self) -> crate::actors::ActorId {
        crate::actors::ActorId::new()
    }

    fn actor_type(&self) -> &'static str {
        "Critic"
    }

    async fn handle_message(
        &mut self,
        message: crate::actors::ActorMessage,
        _context: &crate::actors::ActorContext,
    ) -> Result<(), crate::actors::ActorError> {
        match message {
            crate::actors::ActorMessage::Agent(agent_msg) => match agent_msg {
                crate::actors::AgentMessage::CritiqueResult {
                    result: _,
                    context: _,
                } => {
                    tracing::info!("Received critique request");
                    // For BaseActor implementation, just acknowledge
                    Ok(())
                }
                _ => {
                    tracing::warn!("Unsupported agent message type for Critic");
                    Ok(())
                }
            },
            _ => {
                tracing::warn!("Unsupported message type for Critic");
                Ok(())
            }
        }
    }
}
