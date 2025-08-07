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

        // Check promotion criteria
        if !self.meets_promotion_criteria(record, target_layer)? {
            return Ok(PromotionResult::rejected(
                record.id(),
                "Does not meet promotion criteria".to_string(),
            ));
        }

        // Execute promotion
        record.promote_to_layer(target_layer)?;

        // Update in repository
        self.memory_repo.update(record.clone()).await?;

        Ok(PromotionResult::success(record.id(), target_layer))
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
        let target_layer =
            from_layer
                .next_layer()
                .ok_or_else(|| DomainError::PromotionNotAllowed {
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
        let target_layer =
            from_layer
                .next_layer()
                .ok_or_else(|| DomainError::PromotionNotAllowed {
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

        // Check acceleration if required
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
    pub target_layer: Option<LayerType>,
    pub reason: String,
}

impl PromotionResult {
    pub fn success(record_id: crate::entities::RecordId, layer: LayerType) -> Self {
        Self {
            record_id,
            success: true,
            target_layer: Some(layer),
            reason: "Successfully promoted".to_string(),
        }
    }

    pub fn rejected(record_id: crate::entities::RecordId, reason: String) -> Self {
        Self {
            record_id,
            success: false,
            target_layer: None,
            reason,
        }
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

#[derive(Debug, Clone)]
pub struct PromotionStatistics {
    pub total_records: usize,
    pub records_per_layer: Vec<(LayerType, usize)>,
    pub promotion_candidates: Vec<(LayerType, usize)>,
}

/// Trait for promotion domain service operations
#[async_trait]
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
}
