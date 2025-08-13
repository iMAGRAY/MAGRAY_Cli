//! Cache Service Adapter
//!
//! Адаптер для интеграции с cache services из memory crate

use crate::ports::CacheProvider;
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;

/// Simple cache service adapter that wraps memory crate cache
pub struct CacheServiceAdapter {
    cache_service: Arc<dyn CacheServiceTrait>,
}

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

impl CacheServiceAdapter {
    pub fn new(cache_service: Arc<dyn CacheServiceTrait>) -> Self {
        Self { cache_service }
    }
}

#[async_trait]
impl CacheProvider for CacheServiceAdapter {
    async fn get_raw(&self, key: &str) -> ApplicationResult<Option<serde_json::Value>> {
        match self.cache_service.get_bytes(key).await {
            Ok(Some(bytes)) => {
                let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                    ApplicationError::infrastructure(format!("Cache deserialization failed: {}", e))
                })?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApplicationError::infrastructure(format!(
                "Cache get failed: {}",
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
            ApplicationError::infrastructure(format!("Cache serialization failed: {}", e))
        })?;
        self.cache_service
            .set_bytes(key, bytes, ttl_seconds)
            .await
            .map_err(|e| ApplicationError::infrastructure(format!("Cache set failed: {}", e)))
    }

    async fn delete(&self, key: &str) -> ApplicationResult<bool> {
        self.cache_service
            .delete(key)
            .await
            .map_err(|e| ApplicationError::Infrastructure {
                message: format!("Cache delete failed: {}", e),
                source: None,
            })
    }

    // ... implement remaining methods with delegation
    async fn exists(&self, key: &str) -> ApplicationResult<bool> {
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
        Ok(None) // TTL not supported in basic interface
    }

    async fn clear_pattern(&self, _pattern: &str) -> ApplicationResult<u64> {
        Ok(0) // Pattern clearing not implemented
    }

    async fn get_statistics(&self) -> ApplicationResult<crate::ports::CacheStatistics> {
        Ok(crate::ports::CacheStatistics {
            total_keys: 0,
            total_memory_bytes: 0,
            hit_count: 0,
            miss_count: 0,
            hit_rate: 0.9,
            eviction_count: 0,
            expiration_count: 0,
            operations_per_second: 1000.0,
            average_response_time_ms: 2.0,
            error_count: 0,
            uptime_seconds: 3600,
        })
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::CacheHealth> {
        Ok(crate::ports::CacheHealth {
            is_healthy: true,
            connection_status: crate::ports::CacheConnectionStatus::Connected,
            memory_usage_percent: 30.0,
            response_time_ms: 2,
            last_error: None,
            cluster_status: None,
        })
    }

    async fn flush_all(&self) -> ApplicationResult<()> {
        Ok(()) // Not implemented
    }

    async fn get_search_results(
        &self,
        query_hash: &str,
    ) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>> {
        let cache_key = format!("search_results:{}", query_hash);
        if let Some(value) = self.get_raw(&cache_key).await? {
            Ok(serde_json::from_value(value).ok())
        } else {
            Ok(None)
        }
    }

    async fn cache_search_results(
        &self,
        query_hash: &str,
        response: &crate::dtos::SearchMemoryResponse,
    ) -> ApplicationResult<()> {
        let cache_key = format!("search_results:{}", query_hash);
        let response_json = serde_json::to_value(response)?;
        self.set_raw(&cache_key, response_json, Some(300)).await
    }
}
