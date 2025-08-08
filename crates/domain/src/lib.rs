//! Domain Layer - MAGRAY CLI Business Logic
//!
//! Содержит ТОЛЬКО чистую business logic без dependencies на:
//! - Infrastructure (databases, file systems, networks)
//! - Frameworks (web, CLI, UI)
//! - External systems (AI services, third-party APIs)
//!
//! Принципы Clean Architecture:
//! - Entities: core business objects (MemoryRecord, SearchQuery, etc.)
//! - Value Objects: immutable data structures (LayerType, ScoreThreshold)
//! - Repository Abstractions: interfaces для persistence
//! - Business Rules: чистая business logic без side effects

pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

// Re-export core domain types
pub use entities::{EmbeddingVector, MemoryRecord, RecordId, SearchQuery};
pub use errors::{DomainError, DomainResult};
pub use repositories::{EmbeddingRepository, MemoryRepository, SearchRepository};
pub use services::{MemoryDomainService, PromotionDomainService, SearchDomainService};
pub use value_objects::{AccessPattern, LayerType, PromotionCriteria, ScoreThreshold};

/// Domain-specific type aliases
pub type EmbeddingDimensions = usize;
pub type SimilarityScore = f32;
pub type RecordCount = usize;
