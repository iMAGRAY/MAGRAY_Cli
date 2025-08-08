//! # Application Layer
//!
//! Реализует Clean Architecture Application Layer с:
//! - Use Cases для бизнес-логики workflows
//! - Application Services для координации
//! - DTOs для передачи данных
//! - Ports для абстракций Infrastructure слоя
//! - Command/Query Separation (CQRS)
//!
//! ## Architecture Design Principles
//!
//! 1. **Use Cases** - инкапсулируют бизнес workflows
//! 2. **Application Services** - координируют domain services
//! 3. **DTOs** - изолируют от domain entities 
//! 4. **Ports** - абстракции для infrastructure
//! 5. **CQRS** - разделение команд и запросов
//!
//! ## Dependency Direction
//!
//! ```
//! Application Layer → Domain Layer (entities, services, repositories)
//! Infrastructure → Application Layer (implements ports)
//! ```

pub mod dtos;
pub mod errors;
pub mod ports;
pub mod services;
pub mod use_cases;
pub mod cqrs;
pub mod adapters;

// Re-export common types for convenience
pub use errors::ApplicationError;

/// Application layer result type
pub type ApplicationResult<T> = Result<T, ApplicationError>;

/// Request context for tracing and metrics
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: uuid::Uuid,
    pub correlation_id: String,
    pub user_id: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub source: RequestSource,
}

#[derive(Debug, Clone)]
pub enum RequestSource {
    Cli,
    Api,
    Internal,
    System,
}

impl RequestContext {
    pub fn new(source: RequestSource) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4(),
            correlation_id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            timestamp: std::time::SystemTime::now(),
            source,
        }
    }

    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = correlation_id;
        self
    }
}