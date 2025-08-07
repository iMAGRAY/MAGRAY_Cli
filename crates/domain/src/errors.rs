//! Domain Errors - Business rule violations
//!
//! Contains ONLY business logic errors, не infrastructure errors

use crate::value_objects::LayerType;
use thiserror::Error;

/// Domain-specific errors representing business rule violations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    /// Business validation: content cannot be empty
    #[error("Content cannot be empty")]
    EmptyContent,

    /// Business validation: invalid record ID format
    #[error("Invalid record ID: {0}")]
    InvalidRecordId(String),

    /// Business validation: invalid layer type
    #[error("Invalid layer type: {0}")]
    InvalidLayerType(String),

    /// Business validation: invalid kind
    #[error("Invalid kind: {0}")]
    InvalidKind(String),

    /// Business validation: invalid tag
    #[error("Invalid tag: {0}")]
    InvalidTag(String),

    /// Business rule: duplicate tag not allowed
    #[error("Duplicate tag: {0}")]
    DuplicateTag(String),

    /// Business rule: promotion not allowed
    #[error("Promotion not allowed from {from:?} to {to:?}")]
    PromotionNotAllowed { from: LayerType, to: LayerType },

    /// Business validation: invalid score threshold
    #[error("Invalid score threshold: {0}, must be between 0.0 and 1.0")]
    InvalidScoreThreshold(f32),

    /// Business validation: invalid search query
    #[error("Invalid search query: {0}")]
    InvalidSearchQuery(String),

    /// Business rule: embedding dimension mismatch
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    EmbeddingDimensionMismatch { expected: usize, actual: usize },

    /// Business rule: invalid embedding vector
    #[error("Invalid embedding vector: {0}")]
    InvalidEmbeddingVector(String),

    /// Business validation: record not found
    #[error("Record not found with ID: {0}")]
    RecordNotFound(String),

    /// Business rule: search limit exceeded
    #[error("Search limit exceeded: {requested}, maximum allowed: {max_allowed}")]
    SearchLimitExceeded {
        requested: usize,
        max_allowed: usize,
    },

    /// Business validation: invalid project name
    #[error("Invalid project name: {0}")]
    InvalidProjectName(String),

    /// Business validation: invalid session name  
    #[error("Invalid session name: {0}")]
    InvalidSessionName(String),

    /// Business rule: operation not supported for layer
    #[error("Operation {operation} not supported for layer {layer:?}")]
    OperationNotSupportedForLayer { operation: String, layer: LayerType },

    /// Business rule: concurrent modification detected
    #[error("Concurrent modification detected for record {record_id}")]
    ConcurrentModification { record_id: String },

    /// Business validation: invalid promotion criteria
    #[error("Invalid promotion criteria: {0}")]
    InvalidPromotionCriteria(String),
}

/// Domain result type
pub type DomainResult<T> = Result<T, DomainError>;

impl DomainError {
    /// Check if error is a business validation error
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            DomainError::EmptyContent
                | DomainError::InvalidRecordId(_)
                | DomainError::InvalidLayerType(_)
                | DomainError::InvalidKind(_)
                | DomainError::InvalidTag(_)
                | DomainError::InvalidScoreThreshold(_)
                | DomainError::InvalidSearchQuery(_)
                | DomainError::InvalidEmbeddingVector(_)
                | DomainError::InvalidProjectName(_)
                | DomainError::InvalidSessionName(_)
                | DomainError::InvalidPromotionCriteria(_)
        )
    }

    /// Check if error is a business rule violation
    pub fn is_business_rule_error(&self) -> bool {
        matches!(
            self,
            DomainError::DuplicateTag(_)
                | DomainError::PromotionNotAllowed { .. }
                | DomainError::EmbeddingDimensionMismatch { .. }
                | DomainError::SearchLimitExceeded { .. }
                | DomainError::OperationNotSupportedForLayer { .. }
                | DomainError::ConcurrentModification { .. }
        )
    }

    /// Check if error indicates missing data
    pub fn is_not_found_error(&self) -> bool {
        matches!(self, DomainError::RecordNotFound(_))
    }

    /// Get error category for business logic
    pub fn category(&self) -> ErrorCategory {
        if self.is_validation_error() {
            ErrorCategory::Validation
        } else if self.is_business_rule_error() {
            ErrorCategory::BusinessRule
        } else if self.is_not_found_error() {
            ErrorCategory::NotFound
        } else {
            ErrorCategory::Other
        }
    }
}

/// Categories of domain errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Input validation errors
    Validation,
    /// Business rule violations
    BusinessRule,
    /// Resource not found
    NotFound,
    /// Other domain errors
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let validation_error = DomainError::EmptyContent;
        assert!(validation_error.is_validation_error());
        assert_eq!(validation_error.category(), ErrorCategory::Validation);

        let business_rule_error = DomainError::PromotionNotAllowed {
            from: LayerType::Assets,
            to: LayerType::Interact,
        };
        assert!(business_rule_error.is_business_rule_error());
        assert_eq!(business_rule_error.category(), ErrorCategory::BusinessRule);

        let not_found_error = DomainError::RecordNotFound("test-id".to_string());
        assert!(not_found_error.is_not_found_error());
        assert_eq!(not_found_error.category(), ErrorCategory::NotFound);
    }

    #[test]
    fn test_error_messages() {
        let error = DomainError::InvalidScoreThreshold(1.5);
        assert!(error.to_string().contains("1.5"));
        assert!(error.to_string().contains("between 0.0 and 1.0"));
    }
}
