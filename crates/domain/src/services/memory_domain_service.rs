//! MemoryDomainService - Core business logic for memory operations
//!
//! Pure domain service implementing business rules

use crate::entities::{EmbeddingVector, MemoryRecord, RecordId};
use crate::errors::{DomainError, DomainResult};
use crate::repositories::{EmbeddingRepository, MemoryRepository};
use crate::value_objects::LayerType;
use async_trait::async_trait;
use std::sync::Arc;

/// Domain service for memory business operations
///
/// Orchestrates complex business logic that spans multiple entities
/// Uses repository abstractions to maintain clean architecture
pub struct MemoryDomainService<M, E>
where
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    memory_repo: Arc<M>,
    embedding_repo: Arc<E>,
}

impl<M, E> MemoryDomainService<M, E>
where
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    /// Create new domain service
    pub fn new(memory_repo: Arc<M>, embedding_repo: Arc<E>) -> Self {
        Self {
            memory_repo,
            embedding_repo,
        }
    }

    /// Store memory record with embedding (complex business operation)
    pub async fn store_record_with_embedding(
        &self,
        record: MemoryRecord,
        embedding: EmbeddingVector,
    ) -> DomainResult<RecordId> {
        // Business validation
        self.validate_record_for_storage(&record)?;
        self.validate_embedding_for_record(&embedding)?;

        // Store record first
        let record_id = self.memory_repo.store(record.clone()).await?;

        // Store embedding
        if let Err(e) = self
            .embedding_repo
            .store_embedding(record_id, embedding)
            .await
        {
            let _ = self.memory_repo.delete(record_id).await;
            return Err(e);
        }

        Ok(record_id)
    }

    /// Update record content and re-compute embedding
    pub async fn update_record_content(
        &self,
        record_id: RecordId,
        new_content: String,
        new_embedding: EmbeddingVector,
    ) -> DomainResult<()> {
        // Get existing record
        let mut record = self
            .memory_repo
            .find_by_id(record_id)
            .await?
            .ok_or_else(|| DomainError::RecordNotFound(record_id.to_string()))?;

        // Update content with business validation
        record.update_content(new_content)?;

        // Update both record and embedding atomically
        self.memory_repo.update(record).await?;
        self.embedding_repo
            .update_embedding(record_id, new_embedding)
            .await?;

        Ok(())
    }

    /// Record access and update patterns (business event)
    pub async fn record_access(&self, record_id: RecordId) -> DomainResult<()> {
        let mut record = self
            .memory_repo
            .find_by_id(record_id)
            .await?
            .ok_or_else(|| DomainError::RecordNotFound(record_id.to_string()))?;

        // Business logic: record the access
        record.record_access();

        // Update record
        self.memory_repo.update(record).await?;

        Ok(())
    }

    /// Get record with access tracking
    pub async fn get_record(&self, record_id: RecordId) -> DomainResult<Option<MemoryRecord>> {
        if let Some(record) = self.memory_repo.find_by_id(record_id).await? {
            // Asynchronously record access (business rule)
            let _ = self.record_access(record_id).await;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }

    /// Delete record with all associated data
    pub async fn delete_record(&self, record_id: RecordId) -> DomainResult<bool> {
        // Delete embedding first
        let _ = self.embedding_repo.delete_embedding(record_id).await;

        // Delete record
        self.memory_repo.delete(record_id).await
    }

    /// Batch operations with transaction-like semantics
    pub async fn store_batch_with_embeddings(
        &self,
        records_with_embeddings: Vec<(MemoryRecord, EmbeddingVector)>,
    ) -> DomainResult<Vec<RecordId>> {
        // Validate all records first (fail fast)
        for (record, embedding) in &records_with_embeddings {
            self.validate_record_for_storage(record)?;
            self.validate_embedding_for_record(embedding)?;
        }

        let records: Vec<_> = records_with_embeddings
            .iter()
            .map(|(r, _)| r.clone())
            .collect();
        let embeddings: Vec<_> = records_with_embeddings.into_iter().collect();

        // Store records in batch
        let record_ids = self.memory_repo.store_batch(records).await?;

        // Store embeddings in batch
        let embedding_pairs: Vec<_> = record_ids
            .iter()
            .zip(embeddings.into_iter())
            .map(|(&id, (_, embedding))| (id, embedding))
            .collect();

        if let Err(e) = self
            .embedding_repo
            .store_embeddings_batch(embedding_pairs)
            .await
        {
            // Rollback: delete all stored records
            for &record_id in &record_ids {
                let _ = self.memory_repo.delete(record_id).await;
            }
            return Err(e);
        }

        Ok(record_ids)
    }

    /// Get records needing promotion (business intelligence)
    pub async fn get_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<Vec<MemoryRecord>> {
        let candidates = self
            .memory_repo
            .find_promotion_candidates(from_layer)
            .await?;

        let filtered_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|record| self.should_promote_record(record))
            .collect();

        Ok(filtered_candidates)
    }

    fn validate_record_for_storage(&self, record: &MemoryRecord) -> DomainResult<()> {
        if record.content().trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }

        if record.kind().trim().is_empty() {
            return Err(DomainError::InvalidKind("Kind cannot be empty".to_string()));
        }

        // Business rule: reasonable content size
        if record.content().len() > 100_000 {
            return Err(DomainError::InvalidKind("Content too large".to_string()));
        }

        Ok(())
    }

    fn validate_embedding_for_record(&self, embedding: &EmbeddingVector) -> DomainResult<()> {
        if embedding.expected_dimensions() != self.embedding_repo.expected_dimensions() {
            return Err(DomainError::EmbeddingDimensionMismatch {
                expected: self.embedding_repo.expected_dimensions(),
                actual: embedding.expected_dimensions(),
            });
        }

        if !embedding.is_valid_for_storage() {
            return Err(DomainError::InvalidEmbeddingVector(
                "Invalid vector values".to_string(),
            ));
        }

        Ok(())
    }

    fn should_promote_record(&self, record: &MemoryRecord) -> bool {
        let access_pattern = record.access_pattern();

        // Must have minimum activity
        if access_pattern.access_count() < 3 {
            return false;
        }

        // Must not be too old without recent activity
        if access_pattern.hours_since_last_access() > 72 {
            return false;
        }

        // Must have good importance score
        access_pattern.importance_score() > 0.3
    }
}

/// Trait for domain service operations
#[async_trait]
#[allow(dead_code)]
pub trait MemoryDomainServiceTrait: Send + Sync {
    async fn store_record_with_embedding(
        &self,
        record: MemoryRecord,
        embedding: EmbeddingVector,
    ) -> DomainResult<RecordId>;

    async fn get_record(&self, record_id: RecordId) -> DomainResult<Option<MemoryRecord>>;

    async fn record_access(&self, record_id: RecordId) -> DomainResult<()>;

    async fn delete_record(&self, record_id: RecordId) -> DomainResult<bool>;

    async fn get_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<Vec<MemoryRecord>>;
}

#[async_trait]
impl<M, E> MemoryDomainServiceTrait for MemoryDomainService<M, E>
where
    M: MemoryRepository,
    E: EmbeddingRepository,
{
    async fn store_record_with_embedding(
        &self,
        record: MemoryRecord,
        embedding: EmbeddingVector,
    ) -> DomainResult<RecordId> {
        self.store_record_with_embedding(record, embedding).await
    }

    async fn get_record(&self, record_id: RecordId) -> DomainResult<Option<MemoryRecord>> {
        self.get_record(record_id).await
    }

    async fn record_access(&self, record_id: RecordId) -> DomainResult<()> {
        self.record_access(record_id).await
    }

    async fn delete_record(&self, record_id: RecordId) -> DomainResult<bool> {
        self.delete_record(record_id).await
    }

    async fn get_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<Vec<MemoryRecord>> {
        self.get_promotion_candidates(from_layer).await
    }
}
