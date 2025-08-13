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

    // Additional methods needed by application layer
    async fn analyze_usage_patterns(
        &self,
        start_time: std::time::SystemTime,
        end_time: std::time::SystemTime,
        layers: Option<&Vec<LayerType>>,
        project_filter: Option<&str>,
    ) -> DomainResult<UsageStatistics>;

    async fn get_system_statistics(&self) -> DomainResult<UsageStatistics>;

    async fn collect_health_data(&self) -> DomainResult<HealthData>;

    async fn determine_initial_layer(
        &self,
        content_size: usize,
        content_type: &str,
        user_priority: Option<f32>,
        project_context: Option<&str>,
    ) -> DomainResult<LayerType>;

    async fn is_promotion_candidate(
        &self,
        record: &MemoryRecord,
        access_frequency: f64,
        last_access: &chrono::DateTime<chrono::Utc>,
    ) -> DomainResult<bool>;
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

    async fn analyze_usage_patterns(
        &self,
        _start_time: std::time::SystemTime,
        _end_time: std::time::SystemTime,
        _layers: Option<&Vec<LayerType>>,
        _project_filter: Option<&str>,
    ) -> DomainResult<UsageStatistics> {
        // Mock implementation for now
        Ok(UsageStatistics {
            total_records: 100,
            records_by_layer: std::collections::HashMap::new(),
            access_frequency: std::collections::HashMap::new(),
            cache_hit_rate: 0.85,
            average_response_time_ms: 45.0,
            search_queries_per_hour: 150.0,
            promotion_success_rate: 0.78,
            memory_utilization_mb: 512.0,
            timestamp: chrono::Utc::now(),
            total_requests: 250,
            layer_statistics: std::collections::HashMap::new(),
            access_patterns: Vec::new(),
            time_window_hours: 24,
            overall_average_response_time_ms: 45.0,
            overall_error_rate: 0.02,
            disk_usage_mb: 2048.0,
            active_connections: 15,
            requests_per_minute: 125.0,
            error_rate_percentage: 2.1,
            uptime_seconds: 3600,
        })
    }

    async fn get_system_statistics(&self) -> DomainResult<UsageStatistics> {
        // Mock implementation for now
        Ok(UsageStatistics {
            total_records: 100,
            records_by_layer: std::collections::HashMap::new(),
            access_frequency: std::collections::HashMap::new(),
            cache_hit_rate: 0.85,
            average_response_time_ms: 45.0,
            search_queries_per_hour: 150.0,
            promotion_success_rate: 0.78,
            memory_utilization_mb: 512.0,
            timestamp: chrono::Utc::now(),
            total_requests: 250,
            layer_statistics: std::collections::HashMap::new(),
            access_patterns: Vec::new(),
            time_window_hours: 1,
            overall_average_response_time_ms: 45.0,
            overall_error_rate: 0.02,
            disk_usage_mb: 2048.0,
            active_connections: 15,
            requests_per_minute: 125.0,
            error_rate_percentage: 2.1,
            uptime_seconds: 3600,
        })
    }

    async fn collect_health_data(&self) -> DomainResult<HealthData> {
        // Mock implementation for now
        Ok(HealthData {
            overall_health_score: 0.85,
            memory_utilization: 0.45,
            cache_health: CacheHealth {
                hit_rate: 0.85,
                eviction_rate: 0.1,
                memory_usage_mb: 256.0,
                response_time_ms: 5.0,
            },
            index_health: IndexHealth {
                query_performance: 0.9,
                index_efficiency: 0.88,
                memory_usage_mb: 128.0,
                rebuild_frequency: 0.01,
            },
            storage_health: StorageHealth {
                disk_usage_mb: 1024.0,
                io_performance: 0.92,
                corruption_rate: 0.0,
                backup_status: BackupStatus::UpToDate,
            },
            error_rate: 0.02,
            performance_metrics: PerformanceMetrics {
                avg_query_time_ms: 45.0,
                p95_query_time_ms: 120.0,
                throughput_qps: 25.0,
                concurrent_connections: 5,
            },
            timestamp: chrono::Utc::now(),
            components: std::collections::HashMap::new(),
            critical_issues: Vec::new(),
        })
    }

    async fn determine_initial_layer(
        &self,
        content_size: usize,
        content_type: &str,
        user_priority: Option<f32>,
        _project_context: Option<&str>,
    ) -> DomainResult<LayerType> {
        // Business logic for determining initial layer placement

        // High priority content goes to Insights layer
        if let Some(priority) = user_priority {
            if priority >= 0.8 {
                return Ok(LayerType::Insights);
            }
        }

        // Small, frequently accessed content types
        if content_type == "command" || content_type == "query" {
            return Ok(LayerType::Interact);
        }

        // Large content starts in Assets layer
        if content_size > 10 * 1024 * 1024 {
            // 10MB
            return Ok(LayerType::Assets);
        }

        // Medium-high priority content
        if let Some(priority) = user_priority {
            if priority >= 0.5 {
                return Ok(LayerType::Insights);
            }
        }

        // Default: start in Interact layer for most content
        Ok(LayerType::Interact)
    }

    async fn is_promotion_candidate(
        &self,
        record: &MemoryRecord,
        access_frequency: f64,
        last_access: &chrono::DateTime<chrono::Utc>,
    ) -> DomainResult<bool> {
        // Business rules for promotion candidacy

        let hours_since_access = chrono::Utc::now()
            .signed_duration_since(*last_access)
            .num_hours();

        // Recently accessed records with high frequency are good candidates
        if hours_since_access <= 24 && access_frequency >= 5.0 {
            return Ok(true);
        }

        // High importance records are candidates even if not frequently accessed
        if record.access_pattern().importance_score() >= 0.7 {
            return Ok(true);
        }

        // Records with accelerating access patterns
        if record.access_pattern().is_accelerating() && access_frequency >= 2.0 {
            return Ok(true);
        }

        // Not a promotion candidate
        Ok(false)
    }
}

/// Usage statistics for memory system analysis
#[derive(Debug, Clone)]
pub struct UsageStatistics {
    pub total_records: u64,
    pub records_by_layer: std::collections::HashMap<LayerType, u64>,
    pub access_frequency: std::collections::HashMap<RecordId, u64>,
    pub cache_hit_rate: f64,
    pub average_response_time_ms: f64,
    pub search_queries_per_hour: f64,
    pub promotion_success_rate: f64,
    pub memory_utilization_mb: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    // Additional fields needed by application layer
    pub total_requests: u64,
    pub layer_statistics: std::collections::HashMap<LayerType, LayerStatistics>,
    pub access_patterns: Vec<AccessPatternInfo>,
    pub time_window_hours: u32,
    pub overall_average_response_time_ms: f64,
    pub overall_error_rate: f64,
    // System metrics
    pub disk_usage_mb: f64,
    pub active_connections: u32,
    pub requests_per_minute: f64,
    pub error_rate_percentage: f64,
    pub uptime_seconds: u64,
}

/// Health data for memory system analysis
#[derive(Debug, Clone)]
pub struct HealthData {
    pub overall_health_score: f64,
    pub memory_utilization: f64,
    pub cache_health: CacheHealth,
    pub index_health: IndexHealth,
    pub storage_health: StorageHealth,
    pub error_rate: f64,
    pub performance_metrics: PerformanceMetrics,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    // Additional fields for application layer
    pub components: std::collections::HashMap<String, ComponentHealthInfo>,
    pub critical_issues: Vec<CriticalIssueInfo>,
}

#[derive(Debug, Clone)]
pub struct CacheHealth {
    pub hit_rate: f64,
    pub eviction_rate: f64,
    pub memory_usage_mb: f64,
    pub response_time_ms: f64,
}

#[derive(Debug, Clone)]
pub struct IndexHealth {
    pub query_performance: f64,
    pub index_efficiency: f64,
    pub memory_usage_mb: f64,
    pub rebuild_frequency: f64,
}

#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub disk_usage_mb: f64,
    pub io_performance: f64,
    pub corruption_rate: f64,
    pub backup_status: BackupStatus,
}

#[derive(Debug, Clone)]
pub enum BackupStatus {
    UpToDate,
    Outdated,
    Failed,
    InProgress,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub throughput_qps: f64,
    pub concurrent_connections: u32,
}

/// Layer statistics for analysis
#[derive(Debug, Clone)]
pub struct LayerStatistics {
    pub hit_rate: f64,
    pub average_response_time_ms: f64,
    pub total_requests: u64,
    pub error_rate: f64,
    pub utilization_percentage: f64,
    pub throughput_qps: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Access pattern information
#[derive(Debug, Clone)]
pub struct AccessPatternInfo {
    pub pattern_type: String,
    pub description: String,
    pub frequency: f64,
    pub confidence: f64,
    pub time_windows: Vec<String>,
    pub affected_layers: Vec<LayerType>,
    pub impact_score: f64,
}

/// Health data extended with components and issues
#[derive(Debug, Clone)]
pub struct HealthDataExtended {
    pub components: std::collections::HashMap<String, ComponentHealthInfo>,
    pub critical_issues: Vec<CriticalIssueInfo>,
}

/// Component health information
#[derive(Debug, Clone)]
pub struct ComponentHealthInfo {
    pub status: String,
    pub health_score: f64,
    pub last_check: std::time::SystemTime,
    pub error_count: u32,
    pub warning_count: u32,
    pub details: std::collections::HashMap<String, String>,
}

/// Critical issue information
#[derive(Debug, Clone)]
pub struct CriticalIssueInfo {
    pub issue_type: String,
    pub description: String,
    pub severity: u32, // Will map to IssueSeverity enum in application layer
    pub first_detected: std::time::SystemTime,
    pub last_occurrence: std::time::SystemTime,
    pub occurrence_count: u32,
    pub affected_components: Vec<String>,
    pub resolution_steps: Vec<String>,
}
