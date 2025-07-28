use crate::{LayerStats, MemMeta, MemoryStore};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// M0: Ephemeral - RAM хранилище для временных данных шага/сессии
/// 
/// Особенности:
/// - Данные хранятся только в памяти
/// - Автоматическая очистка по TTL
/// - Быстрый доступ без I/O
/// - Теряется при перезапуске процесса
#[derive(Debug)]
pub struct EphemeralStore {
    data: Arc<RwLock<HashMap<String, EphemeralItem>>>,
}

#[derive(Debug, Clone)]
struct EphemeralItem {
    data: Vec<u8>,
    meta: MemMeta,
    created_at: DateTime<Utc>,
}

impl EphemeralStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Очистка устаревших записей по TTL
    pub async fn cleanup_expired(&self) -> Result<u64> {
        let mut data = self.data.write().await;
        let now = Utc::now();
        let mut removed_count = 0;
        
        data.retain(|_key, item| {
            if let Some(ttl_seconds) = item.meta.ttl_seconds {
                let expires_at = item.created_at + chrono::Duration::seconds(ttl_seconds as i64);
                if now > expires_at {
                    removed_count += 1;
                    return false;
                }
            }
            true
        });
        
        tracing::debug!("Cleaned up {} expired ephemeral items", removed_count);
        Ok(removed_count)
    }
    
    /// Получить все ключи для миграции в другие слои
    pub async fn get_all_items(&self) -> Result<Vec<(String, Vec<u8>, MemMeta)>> {
        let data = self.data.read().await;
        let items = data
            .iter()
            .map(|(key, item)| (key.clone(), item.data.clone(), item.meta.clone()))
            .collect();
        Ok(items)
    }
    
    /// Очистить все данные (для тестов или сброса сессии)
    pub async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        data.clear();
        tracing::debug!("Cleared all ephemeral data");
        Ok(())
    }
}

impl Default for EphemeralStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStore for EphemeralStore {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        let mut store = self.data.write().await;
        
        let item = EphemeralItem {
            data: data.to_vec(),
            meta: meta.clone(),
            created_at: Utc::now(),
        };
        
        store.insert(key.to_string(), item);
        
        tracing::trace!("Stored {} bytes in ephemeral layer with key: {}", data.len(), key);
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        let mut store = self.data.write().await;
        
        if let Some(item) = store.get_mut(key) {
            // Проверяем TTL
            if let Some(ttl_seconds) = item.meta.ttl_seconds {
                let expires_at = item.created_at + chrono::Duration::seconds(ttl_seconds as i64);
                if Utc::now() > expires_at {
                    // Удаляем устаревшую запись
                    store.remove(key);
                    tracing::trace!("Ephemeral item expired and removed: {}", key);
                    return Ok(None);
                }
            }
            
            // Обновляем статистику доступа
            item.meta.access_count += 1;
            item.meta.last_accessed = Utc::now();
            
            let result = (item.data.clone(), item.meta.clone());
            tracing::trace!("Retrieved {} bytes from ephemeral layer: {}", result.0.len(), key);
            Ok(Some(result))
        } else {
            tracing::trace!("Key not found in ephemeral layer: {}", key);
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let mut store = self.data.write().await;
        let removed = store.remove(key).is_some();
        
        if removed {
            tracing::trace!("Deleted key from ephemeral layer: {}", key);
        }
        
        Ok(removed)
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let store = self.data.read().await;
        let exists = store.contains_key(key);
        
        // Проверяем TTL для существующих ключей
        if exists {
            if let Some(item) = store.get(key) {
                if let Some(ttl_seconds) = item.meta.ttl_seconds {
                    let expires_at = item.created_at + chrono::Duration::seconds(ttl_seconds as i64);
                    if Utc::now() > expires_at {
                        return Ok(false);
                    }
                }
            }
        }
        
        Ok(exists)
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let store = self.data.read().await;
        let keys = store.keys().cloned().collect();
        Ok(keys)
    }
    
    async fn stats(&self) -> Result<LayerStats> {
        let store = self.data.read().await;
        
        let total_items = store.len() as u64;
        let total_size_bytes = store
            .values()
            .map(|item| item.data.len() as u64)
            .sum();
        
        let oldest_item = store
            .values()
            .map(|item| item.created_at)
            .min();
        
        let newest_item = store
            .values()
            .map(|item| item.created_at)
            .max();
        
        let avg_access_count = if total_items > 0 {
            store
                .values()
                .map(|item| item.meta.access_count as f64)
                .sum::<f64>() / total_items as f64
        } else {
            0.0
        };
        
        Ok(LayerStats {
            total_items,
            total_size_bytes,
            oldest_item,
            newest_item,
            avg_access_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tokio_test;
    
    #[tokio::test]
    async fn test_ephemeral_basic_operations() {
        let store = EphemeralStore::new();
        let key = "test_key";
        let data = b"test data";
        let meta = MemMeta::default();
        
        // Test put
        store.put(key, data, &meta).await.unwrap();
        
        // Test exists
        assert!(store.exists(key).await.unwrap());
        
        // Test get
        let result = store.get(key).await.unwrap();
        assert!(result.is_some());
        let (retrieved_data, retrieved_meta) = result.unwrap();
        assert_eq!(retrieved_data, data);
        assert_eq!(retrieved_meta.access_count, 1);
        
        // Test delete
        assert!(store.delete(key).await.unwrap());
        assert!(!store.exists(key).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_ephemeral_ttl() {
        let store = EphemeralStore::new();
        let key = "ttl_test";
        let data = b"ttl data";
        let mut meta = MemMeta::default();
        meta.ttl_seconds = Some(1); // 1 секунда TTL
        
        store.put(key, data, &meta).await.unwrap();
        assert!(store.exists(key).await.unwrap());
        
        // Ждём истечения TTL
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Данные должны быть недоступны
        assert!(!store.exists(key).await.unwrap());
        assert!(store.get(key).await.unwrap().is_none());
    }
    
    #[tokio::test]
    async fn test_ephemeral_cleanup() {
        let store = EphemeralStore::new();
        let mut meta = MemMeta::default();
        meta.ttl_seconds = Some(1);
        
        // Добавляем несколько записей с коротким TTL
        for i in 0..5 {
            let key = format!("key_{}", i);
            store.put(&key, b"data", &meta).await.unwrap();
        }
        
        // Ждём истечения TTL
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Запускаем очистку
        let removed = store.cleanup_expired().await.unwrap();
        assert_eq!(removed, 5);
        
        // Проверяем статистику
        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_items, 0);
    }
}