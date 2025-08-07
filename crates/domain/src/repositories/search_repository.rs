//! SearchRepository - Domain abstraction for search operations
//!
//! Combines memory and embedding operations for semantic search

use crate::entities::{MemoryRecord, SearchQuery};
use async_trait::async_trait;
// Re-export SimilarityResult from embedding_repository for convenience
use crate::errors::DomainResult;
pub use crate::repositories::embedding_repository::SimilarityResult;
use crate::SimilarityScore;

/// Repository abstraction for semantic search operations
///
/// Combines business logic of memory and embedding repositories
/// Orchestrates complex search scenarios
#[async_trait]
pub trait SearchRepository: Send + Sync {
    /// Execute semantic search query
    async fn search(&self, query: SearchQuery) -> DomainResult<SearchResults>;

    /// Execute text-only search (without vector similarity)
    async fn text_search(&self, query: SearchQuery) -> DomainResult<SearchResults>;

    /// Execute vector-only search (without text filtering)
    async fn vector_search(&self, query: SearchQuery) -> DomainResult<SearchResults>;

    /// Search within specific context (project + session)
    async fn context_search(
        &self,
        query: SearchQuery,
        project: &str,
        session: &str,
    ) -> DomainResult<SearchResults>;

    /// Get search suggestions based on partial query
    async fn get_suggestions(&self, partial_query: &str, limit: usize)
        -> DomainResult<Vec<String>>;

    /// Search with business rules applied (access patterns, freshness, etc.)
    async fn smart_search(&self, query: SearchQuery) -> DomainResult<SearchResults>;
}

/// Results of a search operation
#[derive(Debug, Clone)]
pub struct SearchResults {
    /// Found records with their metadata
    pub records: Vec<SearchResultRecord>,

    /// Total number of matches (before pagination)
    pub total_matches: usize,

    /// Search execution time (for performance analytics)
    pub execution_time_ms: u64,

    /// Search method used (for analytics)
    pub search_method: SearchMethod,

    /// Whether results were truncated due to limits
    pub truncated: bool,
}

/// Individual search result record
#[derive(Debug, Clone)]
pub struct SearchResultRecord {
    /// The memory record
    pub record: MemoryRecord,

    /// Similarity score (if vector search was used)
    pub similarity_score: Option<SimilarityScore>,

    /// Relevance score (business logic combination)
    pub relevance_score: f32,

    /// Ranking position in results
    pub rank: usize,

    /// Why this record matched (for explainability)
    pub match_reason: MatchReason,
}

/// Search methods used for analytics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMethod {
    /// Pure text-based search
    TextOnly,
    /// Pure vector similarity search
    VectorOnly,
    /// Hybrid text + vector search
    Hybrid,
    /// Smart search with business rules
    Smart,
}

/// Reasons why a record matched (explainable search)
#[derive(Debug, Clone)]
pub enum MatchReason {
    /// Matched by vector similarity
    VectorSimilarity { score: SimilarityScore },
    /// Matched by text content
    TextMatch { terms: Vec<String> },
    /// Matched by tags
    TagMatch { matched_tags: Vec<String> },
    /// Matched by project context
    ProjectMatch,
    /// Matched by access pattern (business logic)
    AccessPattern { reason: String },
    /// Hybrid match (multiple reasons)
    Multiple { reasons: Vec<MatchReason> },
}

impl SearchResults {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            total_matches: 0,
            execution_time_ms: 0,
            search_method: SearchMethod::TextOnly,
            truncated: false,
        }
    }

    pub fn with_records(mut self, records: Vec<SearchResultRecord>) -> Self {
        self.total_matches = records.len();
        self.records = records;
        self
    }

    pub fn with_timing(mut self, execution_time_ms: u64) -> Self {
        self.execution_time_ms = execution_time_ms;
        self
    }

    pub fn with_method(mut self, method: SearchMethod) -> Self {
        self.search_method = method;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
}

impl Default for SearchResults {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchResultRecord {
    pub fn new(record: MemoryRecord, rank: usize, match_reason: MatchReason) -> Self {
        let relevance_score = record.calculate_relevance_score();

        Self {
            record,
            similarity_score: None,
            relevance_score,
            rank,
            match_reason,
        }
    }

    pub fn with_similarity(mut self, similarity_score: SimilarityScore) -> Self {
        self.similarity_score = Some(similarity_score);
        // Combine similarity with business relevance
        self.relevance_score = (similarity_score + self.relevance_score) / 2.0;
        self
    }
}
