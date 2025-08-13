# Critic API Documentation

## Overview

The `Critic` agent analyzes execution results and provides improvement feedback with quality metrics and actionable recommendations.

## API Contract

### CriticTrait

```rust
#[async_trait]
pub trait CriticTrait: Send + Sync {
    /// Evaluate execution result and generate feedback
    async fn evaluate_result(&self, result: &ExecutionResult) -> Result<CriticFeedback>;
    
    /// Analyze individual step performance
    async fn analyze_step_performance(&self, step: &StepResult) -> Result<StepAnalysis>;
    
    /// Generate improvement recommendations
    async fn generate_recommendations(&self, feedback: &CriticFeedback) -> Result<Vec<Recommendation>>;
    
    /// Assess risks in execution results
    async fn assess_risks(&self, result: &ExecutionResult) -> Result<RiskAssessment>;
}
```

## Key Data Structures

### CriticFeedback

```rust
pub struct CriticFeedback {
    pub result_id: Uuid,
    pub overall_score: f64,              // 0.0-1.0 quality score
    pub quality_metrics: QualityMetrics,
    pub success_indicators: Vec<SuccessIndicator>,
    pub improvement_suggestions: Vec<ImprovementSuggestion>,
    pub risk_assessment: RiskAssessment,
}
```

### QualityMetrics

```rust
pub struct QualityMetrics {
    pub efficiency: f64,           // Resource usage efficiency
    pub reliability: f64,          // Error rate and stability
    pub performance: f64,          // Speed and responsiveness
    pub resource_utilization: f64, // Resource optimization
    pub user_satisfaction: f64,    // User experience quality
}
```

## Usage Examples

### Basic Result Evaluation

```rust
use orchestrator::agents::{Critic, CriticTrait};

#[tokio::main]
async fn main() -> Result<()> {
    let critic = Critic::new()
        .with_quality_thresholds(quality_config)
        .with_improvement_engine(improvement_config);
    
    // Analyze execution result
    let feedback = critic.evaluate_result(&execution_result).await?;
    
    println!("Overall Quality Score: {:.2}", feedback.overall_score);
    
    // Display quality metrics
    let metrics = &feedback.quality_metrics;
    println!("Quality Breakdown:");
    println!("  Efficiency: {:.2}", metrics.efficiency);
    println!("  Reliability: {:.2}", metrics.reliability);
    println!("  Performance: {:.2}", metrics.performance);
    
    // Show improvement suggestions
    for suggestion in &feedback.improvement_suggestions {
        println!("💡 {}: {}", suggestion.category, suggestion.description);
        println!("   Priority: {:?}", suggestion.priority);
    }
    
    Ok(())
}
```

For complete documentation, see the source code in `crates/orchestrator/src/agents/critic.rs`.