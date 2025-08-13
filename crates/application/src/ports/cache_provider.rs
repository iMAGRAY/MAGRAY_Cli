//! Cache Provider Port
//!
//! Абстракция для кэширования данных независимо от конкретной реализации.

use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Trait для cache providers
#[async_trait]
pub trait CacheProvider: Send + Sync {
    /// Get raw value from cache as serde_json::Value
    async fn get_raw(&self, key: &str) -> ApplicationResult<Option<serde_json::Value>>;

    /// Set raw value in cache with TTL
    async fn set_raw(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()>;

    /// Delete key from cache
    async fn delete(&self, key: &str) -> ApplicationResult<bool>;

    /// Check if key exists in cache
    async fn exists(&self, key: &str) -> ApplicationResult<bool>;

    /// Get multiple raw values at once
    async fn get_many_raw(
        &self,
        keys: &[String],
    ) -> ApplicationResult<Vec<Option<serde_json::Value>>>;

    /// Set multiple raw values at once
    async fn set_many_raw(
        &self,
        items: &[(String, serde_json::Value)],
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()>;

    /// Increment numeric value (atomic operation)
    async fn increment(&self, key: &str, delta: i64) -> ApplicationResult<i64>;

    /// Expire key after specified seconds
    async fn expire(&self, key: &str, ttl_seconds: u64) -> ApplicationResult<bool>;

    /// Get TTL for key
    async fn ttl(&self, key: &str) -> ApplicationResult<Option<u64>>;

    /// Clear cache by pattern
    async fn clear_pattern(&self, pattern: &str) -> ApplicationResult<u64>;

    /// Get cache statistics
    async fn get_statistics(&self) -> ApplicationResult<CacheStatistics>;

    /// Health check for cache
    async fn health_check(&self) -> ApplicationResult<CacheHealth>;

    /// Flush all cache data
    async fn flush_all(&self) -> ApplicationResult<()>;

    /// Specialized methods for search results caching
    async fn get_search_results(
        &self,
        query_hash: &str,
    ) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>>;

    /// Cache search results
    async fn cache_search_results(
        &self,
        query_hash: &str,
        response: &crate::dtos::SearchMemoryResponse,
    ) -> ApplicationResult<()>;
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_keys: u64,
    pub total_memory_bytes: u64,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f32,
    pub eviction_count: u64,
    pub expiration_count: u64,
    pub operations_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_count: u64,
    pub uptime_seconds: u64,
}

/// Cache health status
#[derive(Debug, Clone)]
pub struct CacheHealth {
    pub is_healthy: bool,
    pub connection_status: CacheConnectionStatus,
    pub memory_usage_percent: f32,
    pub response_time_ms: u64,
    pub last_error: Option<String>,
    pub cluster_status: Option<ClusterStatus>,
}

/// Cache connection status
#[derive(Debug, Clone)]
pub enum CacheConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

/// Cluster status for distributed caches
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    pub node_count: usize,
    pub healthy_nodes: usize,
    pub leader_node: Option<String>,
    pub partition_count: usize,
    pub replication_status: ReplicationStatus,
}

/// Replication status
#[derive(Debug, Clone)]
pub enum ReplicationStatus {
    Healthy,
    Degraded,
    Failed,
    Syncing,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub provider_type: CacheProviderType,
    pub connection_string: String,
    pub default_ttl_seconds: u64,
    pub max_memory_mb: Option<u64>,
    pub max_connections: u32,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub compression: CompressionType,
    pub serialization: SerializationType,
}

/// Types of cache providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheProviderType {
    Memory,
    Redis,
    Memcached,
    DiskCache,
    Hybrid,
    Custom(String),
}

/// Compression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Lz4,
    Snappy,
    Zstd,
}

/// Serialization types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializationType {
    Json,
    Bincode,
    MessagePack,
    Protobuf,
}

/// Extension trait for typed cache operations
pub trait CacheProviderExt: CacheProvider {
    /// Get typed value from cache (convenience wrapper)
    async fn get<T>(&self, key: &str) -> ApplicationResult<Option<T>>
    where
        T: DeserializeOwned + Send,
    {
        if let Some(raw_value) = self.get_raw(key).await? {
            Ok(Some(serde_json::from_value(raw_value)?))
        } else {
            Ok(None)
        }
    }

    /// Set typed value in cache with TTL (convenience wrapper)
    async fn set<T>(&self, key: &str, value: &T, ttl_seconds: Option<u64>) -> ApplicationResult<()>
    where
        T: Serialize + Send + Sync,
    {
        let json_value = serde_json::to_value(value)?;
        self.set_raw(key, json_value, ttl_seconds).await
    }

    /// Get multiple typed values at once (convenience wrapper)
    async fn get_many<T>(&self, keys: &[String]) -> ApplicationResult<Vec<Option<T>>>
    where
        T: DeserializeOwned + Send,
    {
        let raw_results = self.get_many_raw(keys).await?;
        let mut results = Vec::new();
        for raw_value in raw_results {
            if let Some(value) = raw_value {
                results.push(Some(serde_json::from_value(value)?));
            } else {
                results.push(None);
            }
        }
        Ok(results)
    }

    /// Set multiple typed values at once (convenience wrapper)
    async fn set_many<T>(
        &self,
        items: &[(String, T)],
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()>
    where
        T: Serialize + Send + Sync,
    {
        let json_items: Result<Vec<(String, serde_json::Value)>, ApplicationError> = items
            .iter()
            .map(|(key, value)| {
                let json_value = serde_json::to_value(value)?;
                Ok::<(String, serde_json::Value), ApplicationError>((key.clone(), json_value))
            })
            .collect();
        self.set_many_raw(&json_items?, ttl_seconds).await
    }
}

// Blanket implementation for all CacheProvider types
impl<T: CacheProvider> CacheProviderExt for T {}

/// Cache key builder for consistent key generation
pub struct CacheKeyBuilder {
    prefix: String,
    separator: String,
}

impl CacheKeyBuilder {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            separator: ":".to_string(),
        }
    }

    pub fn with_separator(mut self, separator: &str) -> Self {
        self.separator = separator.to_string();
        self
    }

    pub fn key(&self, components: &[&str]) -> String {
        let mut key = self.prefix.clone();
        for component in components {
            key.push_str(&self.separator);
            key.push_str(component);
        }
        key
    }

    pub fn memory_record_key(&self, record_id: &str) -> String {
        self.key(&["memory", "record", record_id])
    }

    pub fn embedding_key(&self, text_hash: &str) -> String {
        self.key(&["embedding", text_hash])
    }

    pub fn search_results_key(&self, query_hash: &str) -> String {
        self.key(&["search", "results", query_hash])
    }

    pub fn user_session_key(&self, user_id: &str, session_id: &str) -> String {
        self.key(&["user", "session", user_id, session_id])
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub access_count: u64,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub metadata: CacheEntryMetadata,
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    pub source: String,
    pub version: String,
    pub tags: Vec<String>,
    pub size_bytes: usize,
    pub compression_ratio: Option<f32>,
    pub custom_data: std::collections::HashMap<String, serde_json::Value>,
}

/// Batch cache operations
pub struct BatchCacheOperation {
    pub operations: Vec<CacheOperation>,
    pub options: BatchOptions,
}

/// Individual cache operation
#[derive(Debug, Clone)]
pub enum CacheOperation {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    },
    Delete {
        key: String,
    },
    Increment {
        key: String,
        delta: i64,
    },
}

/// Batch operation options
#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub parallel_execution: bool,
    pub fail_fast: bool,
    pub max_concurrency: usize,
}

/// Batch operation result
#[derive(Debug, Clone)]
pub struct BatchCacheResult {
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub results: Vec<CacheOperationResult>,
    pub total_time_ms: u64,
}

/// Individual operation result
#[derive(Debug, Clone)]
pub struct CacheOperationResult {
    pub operation_index: usize,
    pub success: bool,
    pub value: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T, ttl_seconds: Option<u64>) -> Self {
        let now = chrono::Utc::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl_seconds.map(|ttl| now + chrono::Duration::seconds(ttl as i64)),
            access_count: 0,
            last_accessed: now,
            metadata: CacheEntryMetadata::default(),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = chrono::Utc::now();
    }
}

impl Default for CacheEntryMetadata {
    fn default() -> Self {
        Self {
            source: "application".to_string(),
            version: "1.0".to_string(),
            tags: vec![],
            size_bytes: 0,
            compression_ratio: None,
            custom_data: std::collections::HashMap::new(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            provider_type: CacheProviderType::Memory,
            connection_string: "memory://local".to_string(),
            default_ttl_seconds: 3600, // 1 hour
            max_memory_mb: Some(512),
            max_connections: 10,
            timeout_seconds: 5,
            retry_attempts: 3,
            compression: CompressionType::None,
            serialization: SerializationType::Json,
        }
    }
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            fail_fast: false,
            max_concurrency: 10,
        }
    }
}

/// Mock cache provider for testing
#[cfg(feature = "test-utils")]
pub struct MockCacheProvider {
    data: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<
                String,
                (serde_json::Value, Option<chrono::DateTime<chrono::Utc>>),
            >,
        >,
    >,
    operations: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

#[cfg(feature = "test-utils")]
impl MockCacheProvider {
    pub fn new() -> Self {
        Self {
            data: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            operations: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Safe lock for operations mutex
    fn safe_lock_operations(&self) -> Result<std::sync::MutexGuard<Vec<String>>, ApplicationError> {
        self.operations
            .lock()
            .map_err(|_| ApplicationError::infrastructure("Operations mutex poisoned"))
    }

    /// Safe lock for data mutex
    fn safe_lock_data(
        &self,
    ) -> Result<
        std::sync::MutexGuard<
            std::collections::HashMap<
                String,
                (serde_json::Value, Option<chrono::DateTime<chrono::Utc>>),
            >,
        >,
        ApplicationError,
    > {
        self.data
            .lock()
            .map_err(|_| ApplicationError::infrastructure("Data mutex poisoned"))
    }

    pub fn get_operations(&self) -> Vec<String> {
        self.safe_lock_operations()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| {
                tracing::error!("Failed to acquire operations lock");
                Vec::new()
            })
    }

    pub fn clear_operations(&self) {
        if let Ok(mut guard) = self.safe_lock_operations() {
            guard.clear();
        } else {
            tracing::error!("Failed to acquire operations lock for clear");
        }
    }

    fn record_operation(&self, operation: &str) {
        if let Ok(mut guard) = self.safe_lock_operations() {
            guard.push(operation.to_string());
        } else {
            tracing::error!("Failed to record operation: {}", operation);
        }
    }
}

#[cfg(feature = "test-utils")]
#[async_trait]
impl CacheProvider for MockCacheProvider {
    async fn get_raw(&self, key: &str) -> ApplicationResult<Option<serde_json::Value>> {
        self.record_operation(&format!("get_raw:{}", key));

        let data = self.data.lock().expect("Operation should succeed");
        if let Some((value, expires_at)) = data.get(key) {
            // Check expiration
            if let Some(expires) = expires_at {
                if chrono::Utc::now() > *expires {
                    return Ok(None);
                }
            }

            Ok(Some(value.clone()))
        } else {
            Ok(None)
        }
    }

    async fn set_raw(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> ApplicationResult<()> {
        self.record_operation(&format!("set_raw:{}:{:?}", key, ttl_seconds));

        let expires_at =
            ttl_seconds.map(|ttl| chrono::Utc::now() + chrono::Duration::seconds(ttl as i64));

        self.data
            .lock()
            .expect("Operation should succeed")
            .insert(key.to_string(), (value, expires_at));
        Ok(())
    }

    async fn delete(&self, key: &str) -> ApplicationResult<bool> {
        self.record_operation(&format!("delete:{}", key));
        Ok(self
            .data
            .lock()
            .expect("Operation should succeed")
            .remove(key)
            .is_some())
    }

    async fn exists(&self, key: &str) -> ApplicationResult<bool> {
        self.record_operation(&format!("exists:{}", key));
        Ok(self
            .data
            .lock()
            .expect("Operation should succeed")
            .contains_key(key))
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
        self.record_operation(&format!("increment:{}:{}", key, delta));

        let mut data = self.data.lock().expect("Operation should succeed");
        let current_value = if let Some((value, _)) = data.get(key) {
            serde_json::from_value::<i64>(value.clone()).unwrap_or(0)
        } else {
            0
        };

        let new_value = current_value + delta;
        data.insert(key.to_string(), (serde_json::json!(new_value), None));

        Ok(new_value)
    }

    async fn expire(&self, key: &str, ttl_seconds: u64) -> ApplicationResult<bool> {
        self.record_operation(&format!("expire:{}:{}", key, ttl_seconds));

        let mut data = self.data.lock().expect("Operation should succeed");
        if let Some((value, _)) = data.get(key).cloned() {
            let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);
            data.insert(key.to_string(), (value, Some(expires_at)));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn ttl(&self, key: &str) -> ApplicationResult<Option<u64>> {
        self.record_operation(&format!("ttl:{}", key));

        let data = self.data.lock().expect("Operation should succeed");
        if let Some((_, expires_at)) = data.get(key) {
            if let Some(expires) = expires_at {
                let now = chrono::Utc::now();
                if now < *expires {
                    let remaining = expires.signed_duration_since(now);
                    Ok(Some(remaining.num_seconds() as u64))
                } else {
                    Ok(Some(0))
                }
            } else {
                Ok(None) // No expiration set
            }
        } else {
            Ok(None) // Key doesn't exist
        }
    }

    async fn clear_pattern(&self, pattern: &str) -> ApplicationResult<u64> {
        self.record_operation(&format!("clear_pattern:{}", pattern));

        let mut data = self.data.lock().expect("Operation should succeed");
        let keys_to_remove: Vec<String> = data
            .keys()
            .filter(|key| key.contains(pattern))
            .cloned()
            .collect();

        let removed_count = keys_to_remove.len() as u64;
        for key in keys_to_remove {
            data.remove(&key);
        }

        Ok(removed_count)
    }

    async fn get_statistics(&self) -> ApplicationResult<CacheStatistics> {
        Ok(CacheStatistics {
            total_keys: self.data.lock().expect("Operation should succeed").len() as u64,
            total_memory_bytes: 1024, // Mock value
            hit_count: 100,
            miss_count: 10,
            hit_rate: 0.91,
            eviction_count: 5,
            expiration_count: 2,
            operations_per_second: 50.0,
            average_response_time_ms: 2.5,
            error_count: 0,
            uptime_seconds: 3600,
        })
    }

    async fn health_check(&self) -> ApplicationResult<CacheHealth> {
        Ok(CacheHealth {
            is_healthy: true,
            connection_status: CacheConnectionStatus::Connected,
            memory_usage_percent: 25.0,
            response_time_ms: 1,
            last_error: None,
            cluster_status: None,
        })
    }

    async fn flush_all(&self) -> ApplicationResult<()> {
        self.record_operation("flush_all");
        self.data.lock().expect("Operation should succeed").clear();
        Ok(())
    }

    async fn get_search_results(
        &self,
        query_hash: &str,
    ) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>> {
        self.record_operation(&format!("get_search_results:{}", query_hash));

        let cache_key = format!("search_results:{}", query_hash);
        self.get(&cache_key).await
    }

    async fn cache_search_results(
        &self,
        query_hash: &str,
        response: &crate::dtos::SearchMemoryResponse,
    ) -> ApplicationResult<()> {
        self.record_operation(&format!("cache_search_results:{}", query_hash));

        let cache_key = format!("search_results:{}", query_hash);
        self.set(&cache_key, response, Some(300)).await // Cache for 5 minutes
    }
}
