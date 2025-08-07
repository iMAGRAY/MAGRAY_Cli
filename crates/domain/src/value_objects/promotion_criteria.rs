//! PromotionCriteria - Business rules for layer promotion
//!
//! Encapsulates business logic for determining when records should be promoted

use crate::errors::{DomainError, DomainResult};
use crate::value_objects::LayerType;
use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Business criteria for promoting records between layers
///
/// Encapsulates the business rules that determine when a record
/// should be promoted from one layer to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionCriteria {
    /// Minimum access count required for promotion
    min_access_count: u32,

    /// Maximum time between accesses (for frequency check)
    max_access_interval: Duration,

    /// Minimum age before promotion (prevents premature promotion)
    min_age: Duration,

    /// Minimum importance score (0.0 to 1.0)
    min_importance_score: f32,

    /// Require accelerating access pattern
    require_acceleration: bool,
}

impl PromotionCriteria {
    /// Create custom promotion criteria with validation
    pub fn new(
        min_access_count: u32,
        max_access_interval: Duration,
        min_age: Duration,
        min_importance_score: f32,
        require_acceleration: bool,
    ) -> DomainResult<Self> {
        if min_importance_score < 0.0 || min_importance_score > 1.0 {
            return Err(DomainError::InvalidPromotionCriteria(format!(
                "Importance score must be between 0.0 and 1.0, got {}",
                min_importance_score
            )));
        }

        if min_access_count == 0 {
            return Err(DomainError::InvalidPromotionCriteria(
                "Minimum access count must be greater than 0".to_string(),
            ));
        }

        Ok(Self {
            min_access_count,
            max_access_interval,
            min_age,
            min_importance_score,
            require_acceleration,
        })
    }

    /// Default criteria for Interact → Insights promotion
    pub fn interact_to_insights() -> Self {
        Self {
            min_access_count: 5,
            max_access_interval: Duration::hours(4),
            min_age: Duration::hours(1),
            min_importance_score: 0.3,
            require_acceleration: false,
        }
    }

    /// Default criteria for Insights → Assets promotion
    pub fn insights_to_assets() -> Self {
        Self {
            min_access_count: 10,
            max_access_interval: Duration::days(1),
            min_age: Duration::days(7),
            min_importance_score: 0.5,
            require_acceleration: false,
        }
    }

    /// Strict criteria (higher requirements)
    pub fn strict_for_layers(from: LayerType, to: LayerType) -> DomainResult<Self> {
        match (from, to) {
            (LayerType::Interact, LayerType::Insights) => Ok(Self {
                min_access_count: 10,
                max_access_interval: Duration::hours(2),
                min_age: Duration::hours(4),
                min_importance_score: 0.5,
                require_acceleration: true,
            }),
            (LayerType::Insights, LayerType::Assets) => Ok(Self {
                min_access_count: 20,
                max_access_interval: Duration::hours(12),
                min_age: Duration::days(14),
                min_importance_score: 0.7,
                require_acceleration: true,
            }),
            _ => Err(DomainError::InvalidPromotionCriteria(format!(
                "Invalid promotion path: {:?} → {:?}",
                from, to
            ))),
        }
    }

    /// Lenient criteria (lower requirements)
    pub fn lenient_for_layers(from: LayerType, to: LayerType) -> DomainResult<Self> {
        match (from, to) {
            (LayerType::Interact, LayerType::Insights) => Ok(Self {
                min_access_count: 3,
                max_access_interval: Duration::hours(8),
                min_age: Duration::minutes(30),
                min_importance_score: 0.2,
                require_acceleration: false,
            }),
            (LayerType::Insights, LayerType::Assets) => Ok(Self {
                min_access_count: 5,
                max_access_interval: Duration::days(2),
                min_age: Duration::days(3),
                min_importance_score: 0.3,
                require_acceleration: false,
            }),
            _ => Err(DomainError::InvalidPromotionCriteria(format!(
                "Invalid promotion path: {:?} → {:?}",
                from, to
            ))),
        }
    }

    // Getters for criteria values
    pub fn min_access_count(&self) -> u32 {
        self.min_access_count
    }

    pub fn max_access_interval(&self) -> Duration {
        self.max_access_interval
    }

    pub fn min_age(&self) -> Duration {
        self.min_age
    }

    pub fn min_importance_score(&self) -> f32 {
        self.min_importance_score
    }

    pub fn require_acceleration(&self) -> bool {
        self.require_acceleration
    }

    /// Get business description of these criteria
    pub fn description(&self) -> String {
        format!(
            "Requires: {} accesses, max {} interval, min {} age, importance {:.1}, acceleration: {}",
            self.min_access_count,
            format_duration(self.max_access_interval),
            format_duration(self.min_age),
            self.min_importance_score,
            self.require_acceleration
        )
    }
}

impl Default for PromotionCriteria {
    fn default() -> Self {
        Self::interact_to_insights()
    }
}

/// Helper function to format duration for display
fn format_duration(duration: Duration) -> String {
    let hours = duration.num_hours();
    let days = duration.num_days();

    if days > 0 {
        format!("{}d", days)
    } else if hours > 0 {
        format!("{}h", hours)
    } else {
        format!("{}m", duration.num_minutes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criteria_creation() {
        let criteria =
            PromotionCriteria::new(5, Duration::hours(2), Duration::hours(1), 0.5, false).unwrap();

        assert_eq!(criteria.min_access_count(), 5);
        assert_eq!(criteria.min_importance_score(), 0.5);
    }

    #[test]
    fn test_criteria_validation() {
        // Invalid importance score
        let result = PromotionCriteria::new(
            5,
            Duration::hours(2),
            Duration::hours(1),
            1.5, // Invalid
            false,
        );
        assert!(result.is_err());

        // Zero access count
        let result = PromotionCriteria::new(
            0, // Invalid
            Duration::hours(2),
            Duration::hours(1),
            0.5,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_default_criteria() {
        let interact_criteria = PromotionCriteria::interact_to_insights();
        let assets_criteria = PromotionCriteria::insights_to_assets();

        // Assets promotion should be stricter
        assert!(assets_criteria.min_access_count() > interact_criteria.min_access_count());
        assert!(assets_criteria.min_importance_score() > interact_criteria.min_importance_score());
    }

    #[test]
    fn test_strict_vs_lenient() {
        let strict =
            PromotionCriteria::strict_for_layers(LayerType::Interact, LayerType::Insights).unwrap();
        let lenient =
            PromotionCriteria::lenient_for_layers(LayerType::Interact, LayerType::Insights)
                .unwrap();

        assert!(strict.min_access_count() > lenient.min_access_count());
        assert!(strict.min_importance_score() > lenient.min_importance_score());
    }
}
