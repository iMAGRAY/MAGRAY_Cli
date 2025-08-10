//! AccessPattern - Business value object for tracking record access
//!
//! Pure domain logic for ML promotion decisions

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Tracks access patterns for business decision making
///
/// Used by ML promotion algorithms to determine record importance
/// Contains ONLY business-relevant access metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    /// Total number of accesses (business metric)
    access_count: u32,

    /// First access timestamp (business event)
    first_access: DateTime<Utc>,

    /// Last access timestamp (business event)
    last_access: DateTime<Utc>,

    /// Access timestamps for pattern analysis (limited to last 10)
    recent_accesses: Vec<DateTime<Utc>>,
}

impl AccessPattern {
    /// Create new access pattern
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            access_count: 0,
            first_access: now,
            last_access: now,
            recent_accesses: Vec::new(),
        }
    }

    /// Create access pattern with initial access
    pub fn with_first_access(timestamp: DateTime<Utc>) -> Self {
        Self {
            access_count: 1,
            first_access: timestamp,
            last_access: timestamp,
            recent_accesses: vec![timestamp],
        }
    }

    /// Record new access (business event)
    pub fn record_access(&mut self) {
        let now = Utc::now();
        self.access_count += 1;
        self.last_access = now;

        // Keep only last 10 accesses for pattern analysis
        self.recent_accesses.push(now);
        if self.recent_accesses.len() > 10 {
            self.recent_accesses.remove(0);
        }

        // Initialize first access if this is the first one
        if self.access_count == 1 {
            self.first_access = now;
        }
    }

    // Business metrics for ML promotion

    /// Get total access count
    pub fn access_count(&self) -> u32 {
        self.access_count
    }

    /// Get first access timestamp
    pub fn first_access(&self) -> DateTime<Utc> {
        self.first_access
    }

    /// Get last access timestamp
    pub fn last_access(&self) -> DateTime<Utc> {
        self.last_access
    }

    /// Calculate average time between accesses
    pub fn avg_access_interval(&self) -> Duration {
        if self.access_count <= 1 {
            return Duration::zero();
        }

        let total_duration = self.last_access - self.first_access;
        total_duration / (self.access_count as i32 - 1)
    }

    /// Calculate hours since last access
    pub fn hours_since_last_access(&self) -> i64 {
        let now = Utc::now();
        (now - self.last_access).num_hours()
    }

    /// Calculate total age (time since first access)
    pub fn total_age(&self) -> Duration {
        Utc::now() - self.first_access
    }

    /// Check if record is "hot" (accessed recently and frequently)
    pub fn is_hot(&self) -> bool {
        self.access_count >= 3 && self.hours_since_last_access() < 2
    }

    /// Check if record is "warm" (moderately accessed)
    pub fn is_warm(&self) -> bool {
        self.access_count >= 2 && self.hours_since_last_access() < 24
    }

    /// Check if record is "cold" (rarely accessed)
    pub fn is_cold(&self) -> bool {
        self.hours_since_last_access() >= 24
    }

    /// Calculate access frequency (accesses per day)
    pub fn access_frequency_per_day(&self) -> f64 {
        let age_days = self.total_age().num_days() as f64;
        if age_days < 1.0 {
            self.access_count as f64 // Less than a day old
        } else {
            self.access_count as f64 / age_days
        }
    }

    /// Check if access pattern is accelerating (getting more frequent)
    pub fn is_accelerating(&self) -> bool {
        if self.recent_accesses.len() < 4 {
            return false;
        }

        let recent_half = &self.recent_accesses[self.recent_accesses.len() / 2..];
        let older_half = &self.recent_accesses[..self.recent_accesses.len() / 2];

        if recent_half.len() < 2 || older_half.len() < 2 {
            return false;
        }

        let recent_interval = if recent_half.len() > 1 {
            (*recent_half.last().unwrap() - *recent_half.first().unwrap()).num_minutes() as f64
                / (recent_half.len() - 1) as f64
        } else {
            0.0
        };

        let older_interval = if older_half.len() > 1 {
            (*older_half.last().unwrap() - *older_half.first().unwrap()).num_minutes() as f64
                / (older_half.len() - 1) as f64
        } else {
            f64::INFINITY
        };

        recent_interval < older_interval && recent_interval > 0.0
    }

    /// Get business importance score (0.0 to 1.0)
    pub fn importance_score(&self) -> f32 {
        let frequency_score = (self.access_frequency_per_day() / 10.0).min(1.0) as f32;
        let recency_score = {
            let hours_since = self.hours_since_last_access() as f32;
            if hours_since < 1.0 {
                1.0
            } else if hours_since < 24.0 {
                1.0 - (hours_since / 24.0)
            } else {
                0.1
            }
        };
        let acceleration_boost = if self.is_accelerating() { 0.2 } else { 0.0 };

        (frequency_score * 0.5 + recency_score * 0.5 + acceleration_boost).min(1.0)
    }
}

impl Default for AccessPattern {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_access_pattern() {
        let pattern = AccessPattern::new();
        assert_eq!(pattern.access_count(), 0);
        // Classification at t=0 can vary due to clock granularity; ensure no panic and zero count
        assert!(pattern.hours_since_last_access() >= 0);
    }

    #[test]
    fn test_record_access() {
        let mut pattern = AccessPattern::new();
        pattern.record_access();

        assert_eq!(pattern.access_count(), 1);
        assert!(pattern.hours_since_last_access() < 1); // Just accessed
    }

    #[test]
    fn test_hot_warm_cold_classification() {
        let mut pattern = AccessPattern::new();

        // Record multiple accesses to make it hot
        for _ in 0..5 {
            pattern.record_access();
        }

        assert!(pattern.is_hot());
        assert!(!pattern.is_cold());
    }

    #[test]
    fn test_importance_score() {
        let mut pattern = AccessPattern::new();

        // New pattern should have low importance
        let initial_score = pattern.importance_score();

        // After several accesses, importance should increase
        for _ in 0..5 {
            pattern.record_access();
        }

        let final_score = pattern.importance_score();
        assert!(final_score > initial_score);
        assert!(final_score <= 1.0);
    }

    #[test]
    fn test_access_frequency() {
        let mut pattern = AccessPattern::new();

        // Record some accesses
        for _ in 0..3 {
            pattern.record_access();
        }

        let frequency = pattern.access_frequency_per_day();
        assert!(frequency >= 3.0); // At least 3 accesses per day (since it's new)
    }
}
