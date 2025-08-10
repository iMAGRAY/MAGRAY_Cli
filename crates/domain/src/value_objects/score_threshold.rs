//! ScoreThreshold - Business value object for similarity scoring
//!
//! Represents business rules for similarity matching

use crate::errors::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Represents a similarity score threshold for business decision making
///
/// Used to determine relevance in search operations
/// Enforces business rules about valid threshold ranges
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ScoreThreshold(f32);

impl ScoreThreshold {
    /// Create new score threshold with business validation
    pub fn new(value: f32) -> DomainResult<Self> {
        if !(0.0..=1.0).contains(&value) {
            return Err(DomainError::InvalidScoreThreshold(value));
        }
        Ok(Self(value))
    }

    /// Create very low threshold (include almost everything)
    pub fn very_low() -> Self {
        Self(0.1)
    }

    /// Create low threshold
    pub fn low() -> Self {
        Self(0.3)
    }

    /// Create medium threshold (balanced relevance)
    pub fn medium() -> Self {
        Self(0.5)
    }

    /// Create high threshold (only very relevant results)
    pub fn high() -> Self {
        Self(0.7)
    }

    /// Create very high threshold (only exact matches)
    pub fn very_high() -> Self {
        Self(0.9)
    }

    /// Get the threshold value
    pub fn value(&self) -> f32 {
        self.0
    }

    /// Check if a score meets this threshold
    pub fn meets_threshold(&self, score: f32) -> bool {
        score >= self.0
    }

    /// Get business description of this threshold level
    pub fn description(&self) -> &'static str {
        match self.0 {
            x if x <= 0.2 => "Very permissive - includes loosely related content",
            x if x <= 0.4 => "Permissive - includes somewhat related content",
            x if x <= 0.6 => "Balanced - moderate relevance required",
            x if x <= 0.8 => "Strict - high relevance required",
            _ => "Very strict - only highly relevant content",
        }
    }
}

impl Default for ScoreThreshold {
    fn default() -> Self {
        Self::medium()
    }
}

impl From<f32> for ScoreThreshold {
    fn from(value: f32) -> Self {
        Self::new(value).unwrap_or_else(|_| Self::medium())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_validation() {
        assert!(ScoreThreshold::new(0.5).is_ok());
        assert!(ScoreThreshold::new(-0.1).is_err());
        assert!(ScoreThreshold::new(1.1).is_err());
    }

    #[test]
    fn test_threshold_levels() {
        assert!(ScoreThreshold::very_low().value() < ScoreThreshold::low().value());
        assert!(ScoreThreshold::low().value() < ScoreThreshold::medium().value());
        assert!(ScoreThreshold::medium().value() < ScoreThreshold::high().value());
        assert!(ScoreThreshold::high().value() < ScoreThreshold::very_high().value());
    }

    #[test]
    fn test_meets_threshold() {
        let threshold = ScoreThreshold::medium();
        assert!(threshold.meets_threshold(0.6));
        assert!(!threshold.meets_threshold(0.4));
    }
}
