use crate::{LayerStats, MemMeta, MemoryStore, MemRef, MemLayer};
use crate::VectorIndex;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Запись в векторном хранилище
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorRecord {
    key: String,
    data: Vec<u8>,
    meta: MemMeta,
    vector: Vec<f32>,
    created_at: DateTime<Utc>,
    accessed_at: DateTime<Utc>,
    access_count: u64,
}

/// Универсальное векторное хранилище для M1/M2
/// 
/// Заменяет SQL-based хранилища на векторное для эффективного семантического поиска
#[derive(Debug)]
pub struct VectorStore {
    /// Идентификатор слоя
    layer: MemLayer,
    /// Векторный индекс
    vector_index: Arc<RwLock<VectorIndex>>,
    /// Хранилище данных (key -> record)
    storage: Arc<RwLock<HashMap<String, VectorRecord>>>,
    /// Путь для персистентности
    persist_path: PathBuf,
    /// Кэш векторов для быстрого доступа
    vector_cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl VectorStore {
    pub async fn new<P: AsRef<Path>>(layer: MemLayer, persist_path: P) -> Result<Self> {
        let persist_path = persist_path.as_ref().to_path_buf();
        
        // Создаем директорию если не существует
        if let Some(parent) = persist_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context("Failed to create persist directory")?;
        }
        
        // Загружаем существующие данные если есть
        let storage: HashMap<String, VectorRecord> = if persist_path.exists() {
            let data = tokio::fs::read(&persist_path).await
                .context("Failed to read persisted data")?;
            serde_json::from_slice(&data)
                .context("Failed to deserialize persisted data")?
        } else {
            HashMap::new()
        };
        
        // Создаем векторный индекс и заполняем его
        let mut vector_index = VectorIndex::new();
        let mut vector_cache = HashMap::new();
        
        for (key, record) in &storage {
            let mem_ref = MemRef::new(layer, key.clone());
            vector_index.add(
                record.vector.clone(),
                mem_ref,
                std::str::from_utf8(&record.data).unwrap_or("<binary>").to_string(),
                record.meta.clone(),
            )?;
            vector_cache.insert(key.clone(), record.vector.clone());
        }
        
        tracing::info!("Initialized vector store for layer {:?} with {} records", layer, storage.len());
        
        Ok(Self {
            layer,
            vector_index: Arc::new(RwLock::new(vector_index)),
            storage: Arc::new(RwLock::new(storage)),
            persist_path,
            vector_cache: Arc::new(RwLock::new(vector_cache)),
        })
    }
    
    /// Сохранить изменения на диск
    async fn persist(&self) -> Result<()> {
        let storage = self.storage.read().await;
        let data = serde_json::to_vec(&*storage)
            .context("Failed to serialize storage")?;
        
        // Атомарная запись через временный файл
        let temp_path = self.persist_path.with_extension("tmp");
        tokio::fs::write(&temp_path, data).await
            .context("Failed to write temporary file")?;
        tokio::fs::rename(&temp_path, &self.persist_path).await
            .context("Failed to rename temporary file")?;
        
        Ok(())
    }
    
    /// Получить вектор для ключа
    pub async fn get_vector(&self, key: &str) -> Option<Vec<f32>> {
        let cache = self.vector_cache.read().await;
        cache.get(key).cloned()
    }
    
    /// Поиск похожих записей по вектору
    pub async fn search_similar(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<(String, f32)>> {
        let index = self.vector_index.read().await;
        let results = index.search(query_vector, top_k)?;
        
        Ok(results.into_iter()
            .map(|r| (r.mem_ref.key, r.score))
            .collect())
    }
    
    /// Получить записи для промоушена
    pub async fn get_promotion_candidates(
        &self,
        min_access_count: u64,
        max_age_seconds: u64,
    ) -> Result<Vec<(String, Vec<u8>, MemMeta)>> {
        let storage = self.storage.read().await;
        let cutoff_time = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        
        let mut candidates: Vec<_> = storage
            .values()
            .filter(|r| {
                r.access_count >= min_access_count && 
                r.created_at <= cutoff_time
            })
            .map(|r| (r.key.clone(), r.data.clone(), r.meta.clone()))
            .collect();
        
        // Сортируем по количеству обращений (убывание)
        candidates.sort_by(|a, b| b.2.access_count.cmp(&a.2.access_count));
        
        Ok(candidates)
    }
    
    /// Очистка устаревших записей
    pub async fn cleanup(&self, max_age_seconds: u64, min_access_count: u64) -> Result<u64> {
        let mut storage = self.storage.write().await;
        let mut index = self.vector_index.write().await;
        let mut cache = self.vector_cache.write().await;
        
        let cutoff_time = Utc::now() - chrono::Duration::seconds(max_age_seconds as i64);
        let initial_count = storage.len();
        
        // Фильтруем записи
        storage.retain(|key, record| {
            let should_keep = record.created_at > cutoff_time || 
                             record.access_count >= min_access_count;
            
            if !should_keep {
                // Удаляем из индекса и кэша
                let mem_ref = MemRef::new(self.layer, key.clone());
                let _ = index.remove(&mem_ref);
                cache.remove(key);
            }
            
            should_keep
        });
        
        let removed = initial_count - storage.len();
        
        // Сохраняем изменения
        drop(storage);
        drop(index);
        drop(cache);
        
        if removed > 0 {
            self.persist().await?;
        }
        
        Ok(removed as u64)
    }
}

#[async_trait]
impl MemoryStore for VectorStore {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        // Проверяем есть ли вектор в метаданных
        let vector = if let Some(vec_value) = meta.extra.get("vector") {
            serde_json::from_value::<Vec<f32>>(vec_value.clone())
                .context("Failed to deserialize vector from metadata")?
        } else {
            // Если нет вектора, создаем нулевой вектор
            // В реальности здесь должна быть векторизация через VectorizerService
            vec![0.0; 1024] // Qwen3 embedding size
        };
        
        let record = VectorRecord {
            key: key.to_string(),
            data: data.to_vec(),
            meta: meta.clone(),
            vector: vector.clone(),
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
        };
        
        // Обновляем хранилище
        {
            let mut storage = self.storage.write().await;
            storage.insert(key.to_string(), record.clone());
        }
        
        // Обновляем векторный индекс
        {
            let mut index = self.vector_index.write().await;
            let mem_ref = MemRef::new(self.layer, key.to_string());
            index.add(
                vector.clone(),
                mem_ref,
                std::str::from_utf8(data).unwrap_or("<binary>").to_string(),
                meta.clone(),
            )?;
        }
        
        // Обновляем кэш векторов
        {
            let mut cache = self.vector_cache.write().await;
            cache.insert(key.to_string(), vector);
        }
        
        // Сохраняем на диск
        self.persist().await?;
        
        tracing::trace!("Stored {} bytes in vector store {:?}: {}", data.len(), self.layer, key);
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        let mut storage = self.storage.write().await;
        
        if let Some(record) = storage.get_mut(key) {
            // Обновляем статистику доступа
            record.access_count += 1;
            record.accessed_at = Utc::now();
            
            let mut meta = record.meta.clone();
            meta.access_count = record.access_count;
            meta.last_accessed = record.accessed_at;
            
            let data = record.data.clone();
            
            // Сохраняем изменения
            drop(storage);
            self.persist().await?;
            
            tracing::trace!("Retrieved {} bytes from vector store {:?}: {}", data.len(), self.layer, key);
            Ok(Some((data, meta)))
        } else {
            tracing::trace!("Key not found in vector store {:?}: {}", self.layer, key);
            Ok(None)
        }
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let mut storage = self.storage.write().await;
        let mut index = self.vector_index.write().await;
        let mut cache = self.vector_cache.write().await;
        
        let removed = storage.remove(key).is_some();
        
        if removed {
            // Удаляем из индекса
            let mem_ref = MemRef::new(self.layer, key.to_string());
            index.remove(&mem_ref)?;
            
            // Удаляем из кэша
            cache.remove(key);
            
            // Сохраняем изменения
            drop(storage);
            drop(index);
            drop(cache);
            self.persist().await?;
            
            tracing::trace!("Deleted key from vector store {:?}: {}", self.layer, key);
        }
        
        Ok(removed)
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let storage = self.storage.read().await;
        Ok(storage.contains_key(key))
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let storage = self.storage.read().await;
        let mut keys: Vec<_> = storage.keys().cloned().collect();
        
        // Сортируем по дате создания (новые первые)
        let records = &*storage;
        keys.sort_by(|a, b| {
            let a_created = records.get(a).map(|r| r.created_at).unwrap_or_default();
            let b_created = records.get(b).map(|r| r.created_at).unwrap_or_default();
            b_created.cmp(&a_created)
        });
        
        Ok(keys)
    }
    
    async fn stats(&self) -> Result<LayerStats> {
        let storage = self.storage.read().await;
        
        let total_items = storage.len() as u64;
        let total_size_bytes: u64 = storage.values()
            .map(|r| r.data.len() as u64)
            .sum();
        
        let oldest_item = storage.values()
            .map(|r| r.created_at)
            .min();
        
        let newest_item = storage.values()
            .map(|r| r.created_at)
            .max();
        
        let avg_access_count = if total_items > 0 {
            storage.values()
                .map(|r| r.access_count as f64)
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
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_vector_store_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_vector_store.json");
        let store = VectorStore::new(MemLayer::Short, &store_path).await.unwrap();
        
        let key = "test_key";
        let data = b"test data";
        let mut meta = MemMeta::default();
        
        // Добавляем вектор в метаданные
        meta.extra.insert(
            "vector".to_string(),
            serde_json::json!(vec![0.1; 1024]),
        );
        
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
        
        // Test vector retrieval
        let vector = store.get_vector(key).await;
        assert!(vector.is_some());
        assert_eq!(vector.unwrap().len(), 1024);
        
        // Test persistence
        drop(store);
        
        // Reload from disk
        let store2 = VectorStore::new(MemLayer::Short, &store_path).await.unwrap();
        assert!(store2.exists(key).await.unwrap());
        
        // Test delete
        assert!(store2.delete(key).await.unwrap());
        assert!(!store2.exists(key).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_vector_store_similarity_search() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("test_similarity.json");
        let store = VectorStore::new(MemLayer::Short, &store_path).await.unwrap();
        
        // Добавляем несколько записей с разными векторами
        for i in 0..5 {
            let key = format!("doc_{}", i);
            let data = format!("Document {}", i).into_bytes();
            let mut meta = MemMeta::default();
            
            // Создаем вектор с разными значениями
            let mut vector = vec![0.0; 1024];
            vector[i] = 1.0; // Разные позиции для разных документов
            
            meta.extra.insert(
                "vector".to_string(),
                serde_json::to_value(&vector).unwrap(),
            );
            
            store.put(&key, &data, &meta).await.unwrap();
        }
        
        // Ищем похожие на doc_2
        let query_vector = store.get_vector("doc_2").await.unwrap();
        let similar = store.search_similar(&query_vector, 3).await.unwrap();
        
        assert!(!similar.is_empty());
        assert_eq!(similar[0].0, "doc_2"); // Самый похожий - он сам
        assert_eq!(similar[0].1, 1.0); // Идеальное совпадение
    }
}