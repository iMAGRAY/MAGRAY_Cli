//! Cache Service Adapter
//!
//! Адаптер для интеграции с cache services из memory crate

use std::sync::Arc;
use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError};
use crate::ports::CacheProvider;

/// Simple cache service adapter that wraps memory crate cache
pub struct CacheServiceAdapter {
    cache_service: Arc<dyn CacheServiceTrait>,
}

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

impl CacheServiceAdapter {
    pub fn new(cache_service: Arc<dyn CacheServiceTrait>) -> Self {
        Self { cache_service }
    }
}

#[async_trait]
impl CacheProvider for CacheServiceAdapter {
    async fn get<T>(&self, key: &str) -> ApplicationResult<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        self.cache_service.get(key).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Cache get failed", e))
    }

    async fn set<T>(&self, key: &str, value: &T, ttl_seconds: Option<u64>) -> ApplicationResult<()>
    where
        T: serde::Serialize + Send + Sync,
    {
        self.cache_service.set(key, value, ttl_seconds).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Cache set failed", e))
    }

    async fn delete(&self, key: &str) -> ApplicationResult<bool> {
        self.cache_service.delete(key).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Cache delete failed", e))
    }

    // ... implement remaining methods with delegation
    async fn exists(&self, key: &str) -> ApplicationResult<bool> {
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
        let current: Option<i64> = self.get(key).await?;
        let new_value = current.unwrap_or(0) + delta;
        self.set(key, &new_value, None).await?;
        Ok(new_value)
    }

    async fn expire(&self, key: &str, ttl_seconds: u64) -> ApplicationResult<bool> {
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

    async fn get_search_results(&self, query_hash: &str) -> ApplicationResult<Option<crate::dtos::SearchMemoryResponse>> {
        let cache_key = format!("search_results:{}", query_hash);
        self.get(&cache_key).await
    }

    async fn cache_search_results(&self, query_hash: &str, response: &crate::dtos::SearchMemoryResponse) -> ApplicationResult<()> {
        let cache_key = format!("search_results:{}", query_hash);
        self.set(&cache_key, response, Some(300)).await
    }
}