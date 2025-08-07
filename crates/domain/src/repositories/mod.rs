//! Repository Abstractions - Ports for Infrastructure Layer
//!
//! Определяет contracts между Domain и Infrastructure слоями
//! Следует Dependency Inversion Principle

mod embedding_repository;
mod memory_repository;
pub mod search_repository;

pub use embedding_repository::{EmbeddingRepository, SimilarityResult};
pub use memory_repository::{
    AccessPatternType, MemoryMetadataRepository, MemoryRepository, RepositoryStatistics,
};
pub use search_repository::{
    MatchReason, SearchMethod, SearchRepository, SearchResultRecord, SearchResults,
};
