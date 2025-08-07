//! EmbeddingRepository - Domain abstraction for embedding operations
//!
//! Handles vector operations independently of AI framework

use crate::entities::{EmbeddingVector, RecordId};
use crate::errors::DomainResult;
use crate::{EmbeddingDimensions, SimilarityScore};
use async_trait::async_trait;

/// Repository abstraction for embedding vector operations
///
/// Separates vector operations from AI framework specifics
/// Follows Domain-Driven Design principles
#[async_trait]
pub trait EmbeddingRepository: Send + Sync {
    /// Store embedding vector for a record
    async fn store_embedding(
        &self,
        record_id: RecordId,
        embedding: EmbeddingVector,
    ) -> DomainResult<()>;

    /// Retrieve embedding vector by record ID
    async fn get_embedding(&self, record_id: RecordId) -> DomainResult<Option<EmbeddingVector>>;

    /// Find similar embeddings using vector search
    async fn find_similar(
        &self,
        query_vector: &EmbeddingVector,
        limit: usize,
        min_similarity: SimilarityScore,
    ) -> DomainResult<Vec<SimilarityResult>>;

    /// Batch store multiple embeddings
    async fn store_embeddings_batch(
        &self,
        embeddings: Vec<(RecordId, EmbeddingVector)>,
    ) -> DomainResult<()>;

    /// Delete embedding by record ID
    async fn delete_embedding(&self, record_id: RecordId) -> DomainResult<bool>;

    /// Update existing embedding
    async fn update_embedding(
        &self,
        record_id: RecordId,
        embedding: EmbeddingVector,
    ) -> DomainResult<()>;

    /// Get embedding count
    async fn count_embeddings(&self) -> DomainResult<usize>;

    /// Check if embedding exists
    async fn embedding_exists(&self, record_id: RecordId) -> DomainResult<bool>;

    /// Get expected embedding dimensions
    fn expected_dimensions(&self) -> EmbeddingDimensions;
}

/// Result of similarity search
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    /// Record ID
    pub record_id: RecordId,

    /// Similarity score (0.0 to 1.0)
    pub similarity_score: SimilarityScore,

    /// Distance metric (if applicable)
    pub distance: Option<f32>,
}

impl SimilarityResult {
    pub fn new(record_id: RecordId, similarity_score: SimilarityScore) -> Self {
        Self {
            record_id,
            similarity_score,
            distance: None,
        }
    }

    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = Some(distance);
        self
    }
}
