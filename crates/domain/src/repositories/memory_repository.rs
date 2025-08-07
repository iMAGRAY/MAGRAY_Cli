//! MemoryRepository - Domain abstraction for memory persistence
//!
//! Defines the contract for memory storage without infrastructure concerns

use crate::entities::{MemoryRecord, RecordId};
use crate::errors::DomainResult;
use crate::value_objects::LayerType;
use async_trait::async_trait;

/// Repository abstraction for memory record persistence
///
/// Follows Repository Pattern - domain defines the interface,
/// infrastructure provides the implementation
#[async_trait]
pub trait MemoryRepository: Send + Sync {
    /// Store a new memory record
    async fn store(&self, record: MemoryRecord) -> DomainResult<RecordId>;

    /// Update an existing memory record
    async fn update(&self, record: MemoryRecord) -> DomainResult<()>;

    /// Retrieve record by ID
    async fn find_by_id(&self, id: RecordId) -> DomainResult<Option<MemoryRecord>>;

    /// Find records by layer
    async fn find_by_layer(&self, layer: LayerType) -> DomainResult<Vec<MemoryRecord>>;

    /// Find records by project
    async fn find_by_project(&self, project: &str) -> DomainResult<Vec<MemoryRecord>>;

    /// Find records by session
    async fn find_by_session(&self, session: &str) -> DomainResult<Vec<MemoryRecord>>;

    /// Find records by tag
    async fn find_by_tag(&self, tag: &str) -> DomainResult<Vec<MemoryRecord>>;

    /// Find records by content type/kind
    async fn find_by_kind(&self, kind: &str) -> DomainResult<Vec<MemoryRecord>>;

    /// Delete record by ID
    async fn delete(&self, id: RecordId) -> DomainResult<bool>;

    /// Count records in layer
    async fn count_by_layer(&self, layer: LayerType) -> DomainResult<usize>;

    /// Get all records (for bulk operations)
    /// WARNING: Use with caution on large datasets
    async fn find_all(&self) -> DomainResult<Vec<MemoryRecord>>;

    /// Find records needing promotion based on business criteria
    async fn find_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> DomainResult<Vec<MemoryRecord>>;

    /// Batch store multiple records (performance optimization)
    async fn store_batch(&self, records: Vec<MemoryRecord>) -> DomainResult<Vec<RecordId>>;

    /// Batch update multiple records
    async fn update_batch(&self, records: Vec<MemoryRecord>) -> DomainResult<()>;

    /// Check if record exists
    async fn exists(&self, id: RecordId) -> DomainResult<bool>;

    /// Get total record count
    async fn total_count(&self) -> DomainResult<usize>;
}

/// Repository abstraction for record metadata operations
///
/// Separated from main repository to follow Interface Segregation Principle
#[async_trait]
pub trait MemoryMetadataRepository: Send + Sync {
    /// Update record access pattern (for ML tracking)
    async fn record_access(&self, id: RecordId) -> DomainResult<()>;

    /// Get records by access pattern (hot/warm/cold)
    async fn find_by_access_pattern(
        &self,
        pattern_type: AccessPatternType,
    ) -> DomainResult<Vec<RecordId>>;

    /// Clean up expired records based on TTL rules
    async fn cleanup_expired(&self) -> DomainResult<usize>;

    /// Get storage statistics for business analytics
    async fn get_statistics(&self) -> DomainResult<RepositoryStatistics>;
}

/// Types of access patterns for querying
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPatternType {
    Hot,  // Frequently accessed recently
    Warm, // Moderately accessed
    Cold, // Rarely accessed
}

/// Repository statistics for business intelligence
#[derive(Debug, Clone)]
pub struct RepositoryStatistics {
    pub total_records: usize,
    pub records_by_layer: Vec<(LayerType, usize)>,
    pub hot_records: usize,
    pub warm_records: usize,
    pub cold_records: usize,
    pub avg_access_count: f32,
    pub most_active_projects: Vec<(String, usize)>,
}

impl RepositoryStatistics {
    pub fn new() -> Self {
        Self {
            total_records: 0,
            records_by_layer: Vec::new(),
            hot_records: 0,
            warm_records: 0,
            cold_records: 0,
            avg_access_count: 0.0,
            most_active_projects: Vec::new(),
        }
    }
}

impl Default for RepositoryStatistics {
    fn default() -> Self {
        Self::new()
    }
}
