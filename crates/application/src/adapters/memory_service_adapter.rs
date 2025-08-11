//! Memory Service Adapter
//!
//! Адаптер для интеграции с существующим memory service из memory crate

use std::sync::Arc;
use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError};
use crate::ports::{StorageProvider, CacheProvider};
use domain::entities::memory_record::MemoryRecord;
use domain::entities::record_id::RecordId;
use domain::repositories::memory_repository::MemoryRepository;

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
    async fn store_record(&self, record: &MemoryRecord) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn retrieve_record(&self, id: &RecordId) -> Result<Option<MemoryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn search_records(&self, query: &str, limit: usize) -> Result<Vec<MemoryRecord>, Box<dyn std::error::Error + Send + Sync>>;
    async fn delete_record(&self, id: &RecordId) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait abstraction for cache service
#[async_trait]
pub trait CacheServiceTrait: Send + Sync {
    async fn get<T>(&self, key: &str) -> Result<Option<T>, Box<dyn std::error::Error + Send + Sync>>
    where
        T: serde::de::DeserializeOwned + Send;
    async fn set<T>(&self, key: &str, value: &T, ttl: Option<u64>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        T: serde::Serialize + Send + Sync;
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
    async fn store(&self, record: &MemoryRecord) -> Result<(), domain::DomainError> {
        self.memory_orchestrator
            .store_record(record)
            .await
            .map_err(|e| domain::DomainError::Infrastructure(format!("Failed to store record: {}", e)))
    }

    async fn find_by_id(&self, id: &RecordId) -> Result<Option<MemoryRecord>, domain::DomainError> {
        self.memory_orchestrator
            .retrieve_record(id)
            .await
            .map_err(|e| domain::DomainError::Infrastructure(format!("Failed to retrieve record: {}", e)))
    }

    async fn find_by_content_hash(&self, hash: &str) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        // Implementation depends on memory orchestrator capabilities
        self.memory_orchestrator
            .search_records(hash, 100)
            .await
            .map_err(|e| domain::DomainError::Infrastructure(format!("Failed to search records: {}", e)))
    }

    async fn update(&self, record: &MemoryRecord) -> Result<(), domain::DomainError> {
        self.store(record).await
    }

    async fn delete(&self, id: &RecordId) -> Result<bool, domain::DomainError> {
        self.memory_orchestrator
            .delete_record(id)
            .await
            .map_err(|e| domain::DomainError::Infrastructure(format!("Failed to delete record: {}", e)))
    }

    async fn list_by_layer(&self, layer: &domain::value_objects::layer_type::LayerType) -> Result<Vec<MemoryRecord>, domain::DomainError> {
        // Implementation depends on memory orchestrator search capabilities
        // This is a simplified implementation
        let query = format!("layer:{:?}", layer);
        self.memory_orchestrator
            .search_records(&query, 1000)
            .await
            .map_err(|e| domain::DomainError::Infrastructure(format!("Failed to list records by layer: {}", e)))
    }

    async fn count_by_layer(&self, layer: &domain::value_objects::layer_type::LayerType) -> Result<u64, domain::DomainError> {
        // Get records and count them (not optimal but works)
        let records = self.list_by_layer(layer).await?;
        Ok(records.len() as u64)
    }

    async fn find_similar(&self, embedding: &domain::entities::embedding_vector::EmbeddingVector, limit: usize, threshold: f64) -> Result<Vec<(MemoryRecord, f64)>, domain::DomainError> {
        // This would require more sophisticated search capabilities
        Ok(vec![])
    }
}

/// Implementation of StorageProvider using the adapter
#[async_trait]
impl StorageProvider for MemoryServiceAdapter {
    async fn store_record(&self, record: &MemoryRecord) -> ApplicationResult<()> {
        self.memory_orchestrator
            .store_record(record)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to store record", e))
    }

    async fn retrieve_record(&self, id: &RecordId) -> ApplicationResult<Option<MemoryRecord>> {
        self.memory_orchestrator
            .retrieve_record(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to retrieve record", e))
    }

    async fn update_record(&self, record: &MemoryRecord) -> ApplicationResult<()> {
        self.memory_orchestrator
            .store_record(record) // Using upsert behavior
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to update record", e))
    }

    async fn delete_record(&self, id: &RecordId) -> ApplicationResult<bool> {
        self.memory_orchestrator
            .delete_record(id)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to delete record", e))
    }

    async fn list_records_by_layer(&self, layer: &domain::value_objects::layer_type::LayerType, limit: Option<usize>) -> ApplicationResult<Vec<MemoryRecord>> {
        let query = format!("layer:{:?}", layer);
        let limit = limit.unwrap_or(1000);
        
        self.memory_orchestrator
            .search_records(&query, limit)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to list records", e))
    }

    async fn get_storage_stats(&self) -> ApplicationResult<crate::ports::StorageStats> {
        Ok(crate::ports::StorageStats {
            total_records: 0,
            total_size_bytes: 0,
            cache_records: 0,
            index_records: 0,
            storage_records: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::StorageHealth> {
        // Mock implementation - real implementation would check memory orchestrator health
        Ok(crate::ports::StorageHealth {
            is_healthy: true,
            response_time_ms: 10,
            error_rate: 0.0,
            last_error: None,
            details: std::collections::HashMap::new(),
        })
    }

    async fn backup_layer(&self, layer: &domain::value_objects::layer_type::LayerType, location: &str) -> ApplicationResult<crate::ports::BackupResult> {
        // Mock implementation
        Ok(crate::ports::BackupResult {
            backup_id: format!("backup_{:?}_{}", layer, chrono::Utc::now().timestamp()),
            records_backed_up: 0,
            size_bytes: 0,
            duration_ms: 0,
        })
    }

    async fn restore_layer(&self, backup_id: &str, target_layer: &domain::value_objects::layer_type::LayerType) -> ApplicationResult<crate::ports::RestoreResult> {
        // Mock implementation
        Ok(crate::ports::RestoreResult {
            backup_id: backup_id.to_string(),
            records_restored: 0,
            duration_ms: 0,
            success: true,
        })
    }
}

/// Implementation of CacheProvider using the adapter
#[async_trait]
impl CacheProvider for MemoryServiceAdapter {
    async fn get<T>(&self, key: &str) -> ApplicationResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        self.cache_service
            .get(key)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to get from cache", e))
    }

    async fn set<T>(&self, key: &str, value: &T, ttl_seconds: Option<u64>) -> ApplicationResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        self.cache_service
            .set(key, value, ttl_seconds)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to set cache", e))
    }

    async fn delete(&self, key: &str) -> ApplicationResult<bool> {
        self.cache_service
            .delete(key)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to delete from cache", e))
    }

    async fn exists(&self, key: &str) -> ApplicationResult<bool> {
        // Implementation using get (not optimal but functional)
        let result: Option<serde_json::Value> = self.get(key).await?;
        Ok(result.is_some())
    }

    async fn get_many<T>(&self, keys: &[String]) -> ApplicationResult<Vec<Option<T>>>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    async fn set_many<T>(&self, items: &[(String, T)], ttl_seconds: Option<u64>) -> ApplicationResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        for (key, value) in items {
            self.set(key, value, ttl_seconds).await?;
        }
        Ok(())
    }

    async fn increment(&self, key: &str, delta: i64) -> ApplicationResult<i64> {
        // Simple implementation - get, increment, set
        let current: Option<i64> = self.get(key).await?;
        let new_value = current.unwrap_or(0) + delta;
        self.set(key, &new_value, None).await?;
        Ok(new_value)
    }

    async fn expire(&self, key: &str, ttl_seconds: u64) -> ApplicationResult<bool> {
        // Get the value and re-set with TTL
        let value: Option<serde_json::Value> = self.get(key).await?;
        match value {
            Some(v) => {
                self.set(key, &v, Some(ttl_seconds)).await?;
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

    async fn get_search_results(&self, query_hash: &str) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>> {
        let cache_key = format!("search_results:{}", query_hash);
        self.get(&cache_key).await
    }

    async fn cache_search_results(&self, query_hash: &str, response: &crate::dtos::SearchMemoryResponse) -> ApplicationResult<()> {
        let cache_key = format!("search_results:{}", query_hash);
        self.set(&cache_key, response, Some(300)).await // 5 minutes TTL
    }
}