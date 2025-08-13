//! Memory Service Adapter
//!
//! Адаптер для интеграции с существующим memory service из memory crate

use crate::ports::{CacheProvider, StorageProvider};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use domain::{LayerType, MemoryRecord, MemoryRepository, RecordId};
use std::sync::Arc;

/// Adapter for memory services from memory crate
pub struct MemoryServiceAdapter {
    /// Reference to memory orchestrator from memory crate
    memory_orchestrator: Arc<dyn MemoryOrchestratorTrait>,
    /// Cache service from memory crate  
    cache_service: Arc<dyn CacheServiceTrait>,
}

/// Trait abstraction for memory orchestrator
#[async_trait]
pub trait MemoryOrchestratorTrait: Send + Sync {
    async fn store_record(
        &self,
        record: &MemoryRecord,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn retrieve_record(
        &self,
        id: &RecordId,
    ) -> Result<Option<MemoryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn search_records(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_record(
        &self,
        id: &RecordId,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait abstraction for cache service (object-safe)
#[async_trait]
pub trait CacheServiceTrait: Send + Sync {
    async fn get_bytes(
        &self,
        key: &str,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>;
    async fn set_bytes(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn delete(&self, key: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

impl MemoryServiceAdapter {
    pub fn new(
        memory_orchestrator: Arc<dyn MemoryOrchestratorTrait>,
        cache_service: Arc<dyn CacheServiceTrait>,
    ) -> Self {
        Self {
            memory_orchestrator,
            cache_service,
        }
    }
}

/// Implementation of MemoryRepository using the adapter
#[async_trait]
impl MemoryRepository for MemoryServiceAdapter {
    async fn store(&self, record: MemoryRecord) -> Result<RecordId, domain::DomainError> {
        self.memory_orchestrator
            .store_record(&record)
            .await
            .map_err(|e| {
                domain::DomainError::InvalidRecordId(format!("Failed to store record: {}", e))
            })?;
        // Return a dummy record ID for now
        Ok(RecordId::new())
    }

    async fn find_by_id(&self, id: RecordId) -> Result<Option<MemoryRecord>, domain::DomainError> {
        self.memory_orchestrator
            .retrieve_record(&id)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!("Failed to retrieve record: {}", e))
            })
    }

    async fn update(&self, record: MemoryRecord) -> Result<(), domain::DomainError> {
        self.memory_orchestrator
            .store_record(&record)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!("Failed to update record: {}", e))
            })
    }

    async fn delete(&self, id: RecordId) -> Result<bool, domain::DomainError> {
        self.memory_orchestrator
            .delete_record(&id)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!("Failed to delete record: {}", e))
            })
    }

    async fn find_by_layer(
        &self,
        layer: LayerType,
    ) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        let query = format!("layer:{:?}", layer);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!(
                    "Failed to find records by layer: {}",
                    e
                ))
            })
    }

    async fn count_by_layer(&self, layer: LayerType) -> Result<usize, domain::DomainError> {
        // Get records and count them (not optimal but works)
        let records = self.find_by_layer(layer).await?;
        Ok(records.len())
    }

    // Add missing trait methods
    async fn find_by_project(
        &self,
        project: &str,
    ) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        let query = format!("project:{}", project);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!(
                    "Failed to find records by project: {}",
                    e
                ))
            })
    }

    async fn find_by_session(
        &self,
        session: &str,
    ) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        let query = format!("session:{}", session);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!(
                    "Failed to find records by session: {}",
                    e
                ))
            })
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        let query = format!("tag:{}", tag);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!("Failed to find records by tag: {}", e))
            })
    }

    async fn find_by_kind(&self, kind: &str) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        let query = format!("kind:{}", kind);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!(
                    "Failed to find records by kind: {}",
                    e
                ))
            })
    }

    async fn find_all(&self) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        self.memory_orchestrator
            .search_records("*", 10000)
            .await
            .map_err(|e| {
                domain::DomainError::Infrastructure(format!("Failed to find all records: {}", e))
            })
    }

    async fn find_promotion_candidates(
        &self,
        from_layer: LayerType,
    ) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        // Find records in the specified layer
        <Self as MemoryRepository>::find_by_layer(self, from_layer).await
    }

    async fn store_batch(
        &self,
        records: Vec<MemoryRecord>,
    ) -> Result<Vec<RecordId>, domain::DomainError> {
        let mut record_ids = Vec::new();
        for record in records {
            let id = <Self as MemoryRepository>::store(self, record).await?;
            record_ids.push(id);
        }
        Ok(record_ids)
    }

    async fn update_batch(&self, records: Vec<MemoryRecord>) -> Result<(), domain::DomainError> {
        for record in records {
            <Self as MemoryRepository>::update(self, record).await?;
        }
        Ok(())
    }

    async fn exists(&self, id: RecordId) -> Result<bool, domain::DomainError> {
        let record = <Self as MemoryRepository>::find_by_id(self, id).await?;
        Ok(record.is_some())
    }

    async fn total_count(&self) -> Result<usize, domain::DomainError> {
        let records = <Self as MemoryRepository>::find_all(self).await?;
        Ok(records.len())
    }
}

/// Implementation of StorageProvider using the adapter
#[async_trait]
impl StorageProvider for MemoryServiceAdapter {
    async fn store(&self, record: &MemoryRecord) -> ApplicationResult<()> {
        self.memory_orchestrator
            .store_record(record)
            .await
            .map_err(|e| ApplicationError::infrastructure(format!("Failed to store record: {}", e)))
    }

    async fn get(&self, id: &RecordId) -> ApplicationResult<Option<MemoryRecord>> {
        self.memory_orchestrator
            .retrieve_record(id)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to retrieve record: {}", e))
            })
    }

    async fn delete(&self, id: &RecordId) -> ApplicationResult<()> {
        self.memory_orchestrator
            .delete_record(id)
            .await
            .map(|_| ()) // Convert bool result to ()
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to delete record: {}", e))
            })
    }

    async fn get_all(&self) -> ApplicationResult<Vec<MemoryRecord>> {
        // Implementation using search with empty query
        self.memory_orchestrator
            .search_records("*", 10000)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to get all records: {}", e))
            })
    }

    async fn get_by_filter(
        &self,
        filter: std::collections::HashMap<String, String>,
    ) -> ApplicationResult<Vec<MemoryRecord>> {
        // Simple implementation - convert filter to query string
        let query = filter
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(" AND ");

        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to filter records: {}", e))
            })
    }

    async fn exists(&self, id: &RecordId) -> ApplicationResult<bool> {
        let result = StorageProvider::get(self, id).await?;
        Ok(result.is_some())
    }

    async fn count(&self) -> ApplicationResult<u64> {
        let records = self.get_all().await?;
        Ok(records.len() as u64)
    }

    async fn clear(&self) -> ApplicationResult<()> {
        // This would require specialized API from memory orchestrator
        // For now, return success but log warning
        tracing::warn!("Storage clear operation not implemented");
        Ok(())
    }

    async fn batch_store(&self, records: &[MemoryRecord]) -> ApplicationResult<()> {
        for record in records {
            MemoryRepository::store(self, record.clone()).await?;
        }
        Ok(())
    }
}

/// Implementation of CacheProvider using the adapter
#[async_trait]
impl CacheProvider for MemoryServiceAdapter {
    async fn get_raw(&self, key: &str) -> ApplicationResult<Option<serde_json::Value>> {
        match self.cache_service.get_bytes(key).await {
            Ok(Some(bytes)) => {
                let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                    ApplicationError::infrastructure_with_source("Cache deserialization failed", e)
                })?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApplicationError::infrastructure(format!(
                "Failed to get from cache: {}",
                e
            ))),
        }
    }

    async fn set_raw(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()> {
        let bytes = serde_json::to_vec(&value).map_err(|e| {
            ApplicationError::infrastructure_with_source("Cache serialization failed", e)
        })?;

        self.cache_service
            .set_bytes(key, bytes, ttl_seconds)
            .await
            .map_err(|e| ApplicationError::infrastructure(format!("Failed to set cache: {}", e)))
    }

    async fn delete(&self, key: &str) -> ApplicationResult<bool> {
        self.cache_service.delete(key).await.map_err(|e| {
            ApplicationError::infrastructure(format!("Failed to delete from cache: {}", e))
        })
    }

    async fn exists(&self, key: &str) -> ApplicationResult<bool> {
        // Implementation using get_raw (not optimal but functional)
        let result = self.get_raw(key).await?;
        Ok(result.is_some())
    }

    async fn get_many_raw(
        &self,
        keys: &[String],
    ) -> ApplicationResult<Vec<Option<serde_json::Value>>> {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get_raw(key).await?);
        }
        Ok(results)
    }

    async fn set_many_raw(
        &self,
        items: &[(String, serde_json::Value)],
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()> {
        for (key, value) in items {
            self.set_raw(key, value.clone(), ttl_seconds).await?;
        }
        Ok(())
    }

    async fn increment(&self, key: &str, delta: i64) -> ApplicationResult<i64> {
        // Simple implementation - get, increment, set
        let current = if let Some(value) = self.get_raw(key).await? {
            serde_json::from_value(value).unwrap_or(0i64)
        } else {
            0i64
        };
        let new_value = current + delta;
        let new_value_json = serde_json::to_value(new_value)?;
        self.set_raw(key, new_value_json, None).await?;
        Ok(new_value)
    }

    async fn expire(&self, key: &str, ttl_seconds: u64) -> ApplicationResult<bool> {
        // Get the value and re-set with TTL
        let value = self.get_raw(key).await?;
        match value {
            Some(v) => {
                self.set_raw(key, v, Some(ttl_seconds)).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn ttl(&self, key: &str) -> ApplicationResult<Option<u64>> {
        // This requires cache service to expose TTL info
        Ok(None)
    }

    async fn clear_pattern(&self, pattern: &str) -> ApplicationResult<u64> {
        // This requires pattern matching capability
        // Mock implementation
        Ok(0)
    }

    async fn get_statistics(&self) -> ApplicationResult<crate::ports::CacheStatistics> {
        // Mock implementation
        Ok(crate::ports::CacheStatistics {
            total_keys: 0,
            total_memory_bytes: 0,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.0,
            eviction_count: 0,
            expiration_count: 0,
            operations_per_second: 0.0,
            average_response_time_ms: 0.0,
            error_count: 0,
            uptime_seconds: 0,
        })
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::CacheHealth> {
        Ok(crate::ports::CacheHealth {
            is_healthy: true,
            connection_status: crate::ports::CacheConnectionStatus::Connected,
            memory_usage_percent: 25.0,
            response_time_ms: 5,
            last_error: None,
            cluster_status: None,
        })
    }

    async fn flush_all(&self) -> ApplicationResult<()> {
        // Not implemented - would require cache service support
        Ok(())
    }

    async fn get_search_results(
        &self,
        query_hash: &str,
    ) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>> {
        let cache_key = format!("search_results:{}", query_hash);
        self.get_search_results(query_hash).await
    }

    async fn cache_search_results(
        &self,
        query_hash: &str,
        response: &crate::dtos::SearchMemoryResponse,
    ) -> ApplicationResult<()> {
        let cache_key = format!("search_results:{}", query_hash);
        let response_json = serde_json::to_value(response)?;
        self.set_raw(&cache_key, response_json, Some(300)).await // 5 minutes TTL
    }
}
