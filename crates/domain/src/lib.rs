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
pub mod value_objects;
pub mod repositories;
pub mod services;
pub mod errors;

// Re-export core domain types
pub use entities::{MemoryRecord, EmbeddingVector, SearchQuery, RecordId};
pub use value_objects::{LayerType, ScoreThreshold, AccessPattern, PromotionCriteria};
pub use repositories::{MemoryRepository, EmbeddingRepository, SearchRepository};
pub use services::{MemoryDomainService, SearchDomainService, PromotionDomainService};
pub use errors::{DomainError, DomainResult};

/// Domain-specific type aliases
pub type EmbeddingDimensions = usize;
pub type SimilarityScore = f32;
pub type RecordCount = usize;