//! Repository Abstractions - Ports for Infrastructure Layer
//!
//! Определяет contracts между Domain и Infrastructure слоями
//! Следует Dependency Inversion Principle

mod memory_repository;
mod embedding_repository;
pub mod search_repository;

pub use memory_repository::{MemoryRepository, MemoryMetadataRepository, AccessPatternType, RepositoryStatistics};
pub use embedding_repository::{EmbeddingRepository, SimilarityResult};
pub use search_repository::{SearchRepository, SearchResults, SearchMethod, SearchResultRecord, MatchReason};