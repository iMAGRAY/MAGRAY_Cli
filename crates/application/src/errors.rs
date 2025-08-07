//! Application Layer Errors
//!
//! Определяет все ошибки Application Layer с четкой категоризацией
//! и mapping на Domain Layer errors.

use thiserror::Error;
use domain::errors::DomainError;

/// Основные ошибки Application Layer
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Domain layer errors
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    /// Validation errors
    #[error("Validation failed: {message}")]
    Validation { message: String },

    /// Infrastructure errors (port implementations)
    #[error("Infrastructure error: {message}")]
    Infrastructure { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    /// Resource not found
    #[error("Resource not found: {resource_type} with ID '{id}'")]
    NotFound { resource_type: String, id: String },

    /// Conflict errors
    #[error("Conflict: {message}")]
    Conflict { message: String },

    /// Authorization/Permission errors
    #[error("Access denied: {message}")]
    AccessDenied { message: String },

    /// Rate limiting
    #[error("Rate limit exceeded: {message}")]
    RateLimited { message: String },

    /// External service errors
    #[error("External service error: {service} - {message}")]
    ExternalService { service: String, message: String },

    /// Timeout errors
    #[error("Operation timed out: {operation}")]
    Timeout { operation: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

impl ApplicationError {
    /// Create validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation { message: message.into() }
    }

    /// Create infrastructure error with source
    pub fn infrastructure<S: Into<String>>(message: S) -> Self {
        Self::Infrastructure { 
            message: message.into(), 
            source: None 
        }
    }

    /// Create infrastructure error with cause
    pub fn infrastructure_with_source<S: Into<String>, E>(message: S, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Infrastructure { 
            message: message.into(), 
            source: Some(Box::new(source))
        }
    }

    /// Create not found error
    pub fn not_found<R: Into<String>, I: Into<String>>(resource_type: R, id: I) -> Self {
        Self::NotFound { 
            resource_type: resource_type.into(), 
            id: id.into() 
        }
    }

    /// Create conflict error
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict { message: message.into() }
    }

    /// Create access denied error
    pub fn access_denied<S: Into<String>>(message: S) -> Self {
        Self::AccessDenied { message: message.into() }
    }

    /// Create timeout error
    pub fn timeout<S: Into<String>>(operation: S) -> Self {
        Self::Timeout { operation: operation.into() }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Domain(e) => e.is_retryable(),
            Self::Infrastructure { .. } => true,
            Self::ExternalService { .. } => true,
            Self::Timeout { .. } => true,
            Self::RateLimited { .. } => true,
            _ => false,
        }
    }

    /// Get error category for metrics
    pub fn category(&self) -> &'static str {
        match self {
            Self::Domain(_) => "domain",
            Self::Validation { .. } => "validation",
            Self::Infrastructure { .. } => "infrastructure",
            Self::NotFound { .. } => "not_found",
            Self::Conflict { .. } => "conflict",
            Self::AccessDenied { .. } => "access_denied",
            Self::RateLimited { .. } => "rate_limited",
            Self::ExternalService { .. } => "external_service",
            Self::Timeout { .. } => "timeout",
            Self::Configuration { .. } => "configuration",
        }
    }
}