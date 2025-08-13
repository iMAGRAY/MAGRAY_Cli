//! PromotionDomainService - Business logic for layer promotion
//!
//! Handles complex ML promotion decisions with business rules

use crate::entities::MemoryRecord;
use crate::errors::{DomainError, DomainResult};
use crate::repositories::MemoryRepository;
use crate::value_objects::{LayerType, PromotionCriteria};
use async_trait::async_trait;
use std::sync::Arc;

/// Domain service for promotion business operations
pub struct PromotionDomainService<M>
where
    M: MemoryRepository,
{
    memory_repo: Arc<M>,
    default_criteria: PromotionCriteria,
}

impl<M> PromotionDomainService<M>
where
    M: MemoryRepository,
{
    pub fn new(memory_repo: Arc<M>) -> Self {
        Self {
            memory_repo,
            default_criteria: PromotionCriteria::default(),
        }
    }

    pub fn with_criteria(memory_repo: Arc<M>, criteria: PromotionCriteria) -> Self {
        Self {
            memory_repo,
            default_criteria: criteria,
        }
    }

    /// Execute promotion analysis for layer
    pub async fn analyze_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<PromotionAnalysis> {
        let candidates = self
            .memory_repo
            .find_promotion_candidates(from_layer)
            .await?;

        let mut analysis = PromotionAnalysis::new(from_layer);

        for record in candidates {
            let recommendation = self.analyze_record_promotion(&record, from_layer)?;
            analysis.add_recommendation(recommendation);
        }

        Ok(analysis)
    }

    /// Execute promotion with business rules
    pub async fn execute_promotion(
        &self,
        record: &mut MemoryRecord,
        target_layer: LayerType,
    ) -> DomainResult<PromotionResult> {
        // Business validation
        if !record.layer().can_promote_to(target_layer) {
            return Err(DomainError::PromotionNotAllowed {
                from: record.layer(),
                to: target_layer,
            });
        }

        let from_layer = record.layer();

        // Check promotion criteria
        if !self.meets_promotion_criteria(record, target_layer)? {
            return Ok(PromotionResult::rejected(
                record.id(),
                from_layer,
                "Does not meet promotion criteria".to_string(),
            ));
        }

        // Execute promotion
        record.promote_to_layer(target_layer)?;

        let from_layer = record.layer();

        // Update in repository
        self.memory_repo.update(record.clone()).await?;

        Ok(PromotionResult::success(
            record.id(),
            from_layer,
            target_layer,
        ))
    }

    /// Batch promotion with business rules
    pub async fn execute_batch_promotion(
        &self,
        from_layer: LayerType,
        criteria: Option<PromotionCriteria>,
    ) -> DomainResult<BatchPromotionResult> {
        let promotion_criteria = criteria.unwrap_or(self.default_criteria.clone());
        let candidates = self
            .memory_repo
            .find_promotion_candidates(from_layer)
            .await?;

        let mut results = BatchPromotionResult::new();
        let target_layer = from_layer
            .next_layer()
            .ok_or(DomainError::PromotionNotAllowed {
                from: from_layer,
                to: from_layer, // Invalid target
            })?;

        let mut records_to_update = Vec::new();

        for mut record in candidates {
            if self.meets_promotion_criteria_with(&record, target_layer, &promotion_criteria)? {
                match record.promote_to_layer(target_layer) {
                    Ok(()) => {
                        records_to_update.push(record.clone());
                        results.add_success(record.id(), target_layer);
                    }
                    Err(e) => {
                        results.add_failure(record.id(), e.to_string());
                    }
                }
            } else {
                results.add_skipped(record.id(), "Does not meet criteria".to_string());
            }
        }

        // Batch update records
        if !records_to_update.is_empty() {
            self.memory_repo.update_batch(records_to_update).await?;
        }

        Ok(results)
    }

    /// Get promotion statistics for business intelligence
    pub async fn get_promotion_statistics(&self) -> DomainResult<PromotionStatistics> {
        let interact_count = self.memory_repo.count_by_layer(LayerType::Interact).await?;
        let insights_count = self.memory_repo.count_by_layer(LayerType::Insights).await?;
        let assets_count = self.memory_repo.count_by_layer(LayerType::Assets).await?;

        let interact_candidates = self
            .memory_repo
            .find_promotion_candidates(LayerType::Interact)
            .await?;
        let insights_candidates = self
            .memory_repo
            .find_promotion_candidates(LayerType::Insights)
            .await?;

        Ok(PromotionStatistics {
            total_records: interact_count + insights_count + assets_count,
            records_per_layer: vec![
                (LayerType::Interact, interact_count),
                (LayerType::Insights, insights_count),
                (LayerType::Assets, assets_count),
            ],
            promotion_candidates: vec![
                (LayerType::Interact, interact_candidates.len()),
                (LayerType::Insights, insights_candidates.len()),
            ],
        })
    }

    // Private helper methods

    fn analyze_record_promotion(
        &self,
        record: &MemoryRecord,
        from_layer: LayerType,
    ) -> DomainResult<PromotionRecommendation> {
        let target_layer = from_layer
            .next_layer()
            .ok_or(DomainError::PromotionNotAllowed {
                from: from_layer,
                to: from_layer,
            })?;

        let meets_criteria = self.meets_promotion_criteria(record, target_layer)?;
        let confidence = self.calculate_promotion_confidence(record);

        Ok(PromotionRecommendation {
            record_id: record.id(),
            from_layer,
            to_layer: target_layer,
            recommended: meets_criteria && confidence > 0.7,
            confidence,
            reason: self.get_promotion_reason(record, meets_criteria),
        })
    }

    fn meets_promotion_criteria(
        &self,
        record: &MemoryRecord,
        target_layer: LayerType,
    ) -> DomainResult<bool> {
        self.meets_promotion_criteria_with(record, target_layer, &self.default_criteria)
    }

    fn meets_promotion_criteria_with(
        &self,
        record: &MemoryRecord,
        _target_layer: LayerType,
        criteria: &PromotionCriteria,
    ) -> DomainResult<bool> {
        let access_pattern = record.access_pattern();

        // Check access count
        if access_pattern.access_count() < criteria.min_access_count() {
            return Ok(false);
        }

        // Check importance score
        if access_pattern.importance_score() < criteria.min_importance_score() {
            return Ok(false);
        }

        // Check age requirements
        if access_pattern.total_age() < criteria.min_age() {
            return Ok(false);
        }

        if criteria.require_acceleration() && !access_pattern.is_accelerating() {
            return Ok(false);
        }

        Ok(true)
    }

    fn calculate_promotion_confidence(&self, record: &MemoryRecord) -> f32 {
        let access_pattern = record.access_pattern();
        let base_score = access_pattern.importance_score();

        // Boost confidence based on business factors
        let access_boost = (access_pattern.access_count() as f32 / 20.0).min(0.3);
        let recency_boost = if access_pattern.hours_since_last_access() < 24 {
            0.2
        } else {
            0.0
        };
        let acceleration_boost = if access_pattern.is_accelerating() {
            0.1
        } else {
            0.0
        };

        (base_score + access_boost + recency_boost + acceleration_boost).min(1.0)
    }

    fn get_promotion_reason(&self, record: &MemoryRecord, meets_criteria: bool) -> String {
        if meets_criteria {
            let access_pattern = record.access_pattern();
            format!(
                "High activity: {} accesses, importance {:.2}, {} since last access",
                access_pattern.access_count(),
                access_pattern.importance_score(),
                if access_pattern.hours_since_last_access() < 1 {
                    "< 1 hour".to_string()
                } else {
                    format!("{} hours", access_pattern.hours_since_last_access())
                }
            )
        } else {
            "Does not meet promotion criteria".to_string()
        }
    }
}

/// Results and data structures for promotion operations

#[derive(Debug, Clone)]
pub struct PromotionAnalysis {
    pub from_layer: LayerType,
    pub recommendations: Vec<PromotionRecommendation>,
}

impl PromotionAnalysis {
    pub fn new(from_layer: LayerType) -> Self {
        Self {
            from_layer,
            recommendations: Vec::new(),
        }
    }

    pub fn add_recommendation(&mut self, recommendation: PromotionRecommendation) {
        self.recommendations.push(recommendation);
    }

    pub fn recommended_count(&self) -> usize {
        self.recommendations
            .iter()
            .filter(|r| r.recommended)
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct PromotionRecommendation {
    pub record_id: crate::entities::RecordId,
    pub from_layer: LayerType,
    pub to_layer: LayerType,
    pub recommended: bool,
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct PromotionResult {
    pub record_id: crate::entities::RecordId,
    pub success: bool,
    pub from_layer: LayerType,
    pub target_layer: Option<LayerType>,
    pub reason: String,
}

impl PromotionResult {
    pub fn success(
        record_id: crate::entities::RecordId,
        from_layer: LayerType,
        target_layer: LayerType,
    ) -> Self {
        Self {
            record_id,
            success: true,
            from_layer,
            target_layer: Some(target_layer),
            reason: "Successfully promoted".to_string(),
        }
    }

    pub fn rejected(
        record_id: crate::entities::RecordId,
        from_layer: LayerType,
        reason: String,
    ) -> Self {
        Self {
            record_id,
            success: false,
            from_layer,
            target_layer: None,
            reason,
        }
    }

    pub fn record_id(&self) -> &crate::entities::RecordId {
        &self.record_id
    }

    pub fn from_layer(&self) -> LayerType {
        self.from_layer
    }

    pub fn to_layer(&self) -> Option<LayerType> {
        self.target_layer
    }
}

#[derive(Debug, Clone)]
pub struct BatchPromotionResult {
    pub successful: Vec<(crate::entities::RecordId, LayerType)>,
    pub failed: Vec<(crate::entities::RecordId, String)>,
    pub skipped: Vec<(crate::entities::RecordId, String)>,
}

impl BatchPromotionResult {
    pub fn new() -> Self {
        Self {
            successful: Vec::new(),
            failed: Vec::new(),
            skipped: Vec::new(),
        }
    }

    pub fn add_success(&mut self, record_id: crate::entities::RecordId, layer: LayerType) {
        self.successful.push((record_id, layer));
    }

    pub fn add_failure(&mut self, record_id: crate::entities::RecordId, reason: String) {
        self.failed.push((record_id, reason));
    }

    pub fn add_skipped(&mut self, record_id: crate::entities::RecordId, reason: String) {
        self.skipped.push((record_id, reason));
    }

    pub fn total_processed(&self) -> usize {
        self.successful.len() + self.failed.len() + self.skipped.len()
    }
}

impl Default for BatchPromotionResult {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PromotionStatistics {
    pub total_records: usize,
    pub records_per_layer: Vec<(LayerType, usize)>,
    pub promotion_candidates: Vec<(LayerType, usize)>,
}

/// Trait for promotion domain service operations
#[async_trait]
#[allow(dead_code)]
pub trait PromotionDomainServiceTrait: Send + Sync {
    async fn analyze_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<PromotionAnalysis>;
    async fn execute_batch_promotion(
        &self,
        from_layer: LayerType,
        criteria: Option<PromotionCriteria>,
    ) -> DomainResult<BatchPromotionResult>;
    async fn get_promotion_statistics(&self) -> DomainResult<PromotionStatistics>;
    async fn promote_record(
        &self,
        record_id: crate::entities::RecordId,
        target_layer: LayerType,
    ) -> DomainResult<PromotionResult>;

    async fn find_promotion_candidates(
        &self,
        criteria: &[PromotionCriteria],
        max_candidates: Option<usize>,
        dry_run: bool,
    ) -> DomainResult<Vec<PromotionRecommendation>>;

    async fn analyze_promotion_opportunities(
        &self,
        criteria: &PromotionCriteria,
        layers: Option<&[LayerType]>,
        time_window_hours: u64,
    ) -> DomainResult<PromotionAnalysis>;

    async fn force_promote_records(
        &self,
        record_ids: &[crate::entities::RecordId],
        target_layer: LayerType,
    ) -> DomainResult<Vec<PromotionResult>>;

    async fn run_promotion_cycle(&self) -> DomainResult<PromotionCycleResults>;
}

#[async_trait]
impl<M> PromotionDomainServiceTrait for PromotionDomainService<M>
where
    M: MemoryRepository,
{
    async fn analyze_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<PromotionAnalysis> {
        self.analyze_promotion_candidates(from_layer).await
    }

    async fn execute_batch_promotion(
        &self,
        from_layer: LayerType,
        criteria: Option<PromotionCriteria>,
    ) -> DomainResult<BatchPromotionResult> {
        self.execute_batch_promotion(from_layer, criteria).await
    }

    async fn get_promotion_statistics(&self) -> DomainResult<PromotionStatistics> {
        self.get_promotion_statistics().await
    }

    async fn promote_record(
        &self,
        record_id: crate::entities::RecordId,
        target_layer: LayerType,
    ) -> DomainResult<PromotionResult> {
        // Find record by ID
        let mut record =
            self.memory_repo
                .find_by_id(record_id)
                .await?
                .ok_or(DomainError::InvalidRecordId(format!(
                    "Record not found: {record_id:?}"
                )))?;

        // Execute promotion using existing method
        self.execute_promotion(&mut record, target_layer).await
    }

    async fn find_promotion_candidates(
        &self,
        _criteria: &[PromotionCriteria],
        max_candidates: Option<usize>,
        _dry_run: bool,
    ) -> DomainResult<Vec<PromotionRecommendation>> {
        // For now, find candidates for Interact layer
        let analysis = self
            .analyze_promotion_candidates(LayerType::Interact)
            .await?;

        let mut candidates = analysis.recommendations;
        if let Some(limit) = max_candidates {
            candidates.truncate(limit);
        }

        Ok(candidates)
    }

    async fn analyze_promotion_opportunities(
        &self,
        _criteria: &PromotionCriteria,
        layers: Option<&[LayerType]>,
        _time_window_hours: u64,
    ) -> DomainResult<PromotionAnalysis> {
        // For now, analyze first layer or default to Interact
        let layer = layers
            .and_then(|l| l.first())
            .copied()
            .unwrap_or(LayerType::Interact);
        self.analyze_promotion_candidates(layer).await
    }

    async fn force_promote_records(
        &self,
        record_ids: &[crate::entities::RecordId],
        target_layer: LayerType,
    ) -> DomainResult<Vec<PromotionResult>> {
        let mut results = Vec::new();

        for &record_id in record_ids {
            let result = self.promote_record(record_id, target_layer).await;
            match result {
                Ok(promotion_result) => results.push(promotion_result),
                Err(e) => results.push(PromotionResult::rejected(
                    record_id,
                    target_layer,
                    e.to_string(),
                )),
            }
        }

        Ok(results)
    }

    async fn run_promotion_cycle(&self) -> DomainResult<PromotionCycleResults> {
        let start_time = chrono::Utc::now();

        // Run batch promotion from Interact to Insights layer
        let batch_result = self
            .execute_batch_promotion(LayerType::Interact, None)
            .await?;

        let end_time = chrono::Utc::now();

        Ok(PromotionCycleResults {
            cycle_id: format!("cycle_{}", chrono::Utc::now().timestamp()),
            start_time,
            end_time,
            total_candidates: batch_result.total_processed() as u64,
            successful_promotions: batch_result.successful.len() as u64,
            failed_promotions: batch_result.failed.len() as u64,
            skipped_promotions: batch_result.skipped.len() as u64,
            promotion_details: Vec::new(),
            overall_success_rate: if batch_result.total_processed() > 0 {
                batch_result.successful.len() as f64 / batch_result.total_processed() as f64
            } else {
                1.0
            },
            efficiency_score: 0.8, // TODO: Calculate actual efficiency
            analysis_time_ms: 100, // Mock value
            execution_time_ms: (end_time - start_time).num_milliseconds() as u64,
            performance_impact: PerformanceImpact {
                query_latency_change_ms: 0.0,
                throughput_change_qps: 0.0,
                memory_usage_change_mb: 0.0,
                cache_hit_rate_change: 0.0,
            },
            recommendations: Vec::new(),
        })
    }
}

/// Results from a complete promotion cycle
#[derive(Debug, Clone)]
pub struct PromotionCycleResults {
    pub cycle_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: chrono::DateTime<chrono::Utc>,
    pub total_candidates: u64,
    pub successful_promotions: u64,
    pub failed_promotions: u64,
    pub skipped_promotions: u64,
    pub promotion_details: Vec<PromotionDetail>,
    pub overall_success_rate: f64,
    pub efficiency_score: f64,
    pub analysis_time_ms: u64,
    pub execution_time_ms: u64,
    pub performance_impact: PerformanceImpact,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PromotionDetail {
    pub record_id: String,
    pub from_layer: LayerType,
    pub to_layer: LayerType,
    pub promotion_score: f64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl PromotionDetail {
    pub fn record_id(&self) -> &str {
        &self.record_id
    }

    pub fn from_layer(&self) -> LayerType {
        self.from_layer
    }

    pub fn to_layer(&self) -> LayerType {
        self.to_layer
    }

    pub fn score(&self) -> f32 {
        self.promotion_score as f32
    }

    pub fn reason(&self) -> &str {
        self.error_message
            .as_deref()
            .unwrap_or("Successful promotion")
    }

    pub fn estimated_benefit(&self) -> f32 {
        if self.success {
            self.promotion_score as f32 * 0.8
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceImpact {
    pub query_latency_change_ms: f64,
    pub throughput_change_qps: f64,
    pub memory_usage_change_mb: f64,
    pub cache_hit_rate_change: f64,
}
