use crate::{LayerStats, MemMeta, MemoryStore};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use walkdir::WalkDir;

/// M3: LongTerm - файловое хранилище для больших артефактов и архивов
/// 
/// Особенности:
/// - Хранение больших файлов на диске
/// - Организация по хешам для дедупликации
/// - Сжатие и архивирование старых данных
/// - Метаданные в отдельном индексе
#[derive(Debug)]
pub struct LongTermStore {
    base_path: PathBuf,
    index: tokio::sync::RwLock<HashMap<String, BlobMeta>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BlobMeta {
    file_path: PathBuf,
    content_hash: String,
    size_bytes: u64,
    compressed: bool,
    mem_meta: MemMeta,
    created_at: DateTime<Utc>,
}

impl LongTermStore {
    pub async fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Создаём структуру директорий
        fs::create_dir_all(&base_path).await
            .context("Failed to create long-term storage directory")?;
        
        let blobs_dir = base_path.join("blobs");
        fs::create_dir_all(&blobs_dir).await
            .context("Failed to create blobs directory")?;
        
        let archive_dir = base_path.join("archive");
        fs::create_dir_all(&archive_dir).await
            .context("Failed to create archive directory")?;
        
        // Загружаем индекс из файла
        let index_path = base_path.join("index.json");
        let index = if index_path.exists() {
            let index_data = fs::read_to_string(&index_path).await
                .context("Failed to read index file")?;
            serde_json::from_str(&index_data)
                .context("Failed to parse index file")?
        } else {
            HashMap::new()
        };
        
        tracing::info!("Initialized long-term memory store at: {}", base_path.display());
        
        Ok(Self {
            base_path,
            index: tokio::sync::RwLock::new(index),
        })
    }
    
    /// Сохранить индекс на диск
    async fn save_index(&self) -> Result<()> {
        let index = self.index.read().await;
        let index_data = serde_json::to_string_pretty(&*index)
            .context("Failed to serialize index")?;
        
        let index_path = self.base_path.join("index.json");
        fs::write(&index_path, index_data).await
            .context("Failed to write index file")?;
        
        Ok(())
    }
    
    /// Вычислить хеш содержимого для дедупликации
    fn compute_hash(data: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    /// Получить путь к файлу по хешу
    fn get_blob_path(&self, hash: &str) -> PathBuf {
        // Организуем файлы по первым символам хеша для лучшей производительности ФС
        let prefix = &hash[..2];
        let suffix = &hash[2..];
        self.base_path.join("blobs").join(prefix).join(suffix)
    }
    
    /// Сжать данные если они большие
    async fn maybe_compress(&self, data: &[u8]) -> Result<(Vec<u8>, bool)> {
        const COMPRESSION_THRESHOLD: usize = 1024; // 1KB
        
        if data.len() > COMPRESSION_THRESHOLD {
            // Простое сжатие (в реальности можно использовать zstd или lz4)
            let compressed = self.compress_data(data)?;
            if compressed.len() < data.len() {
                tracing::trace!("Compressed {} bytes to {} bytes", data.len(), compressed.len());
                return Ok((compressed, true));
            }
        }
        
        Ok((data.to_vec(), false))
    }
    
    /// Простое сжатие данных (заглушка)
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // В реальной реализации здесь был бы zstd или другой алгоритм
        // Пока что просто возвращаем исходные данные
        Ok(data.to_vec())
    }
    
    /// Распаковать данные
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // В реальной реализации здесь была бы распаковка
        Ok(data.to_vec())
    }
    
    /// Архивировать старые файлы
    pub async fn archive_old_files(&self, max_age_days: u64) -> Result<u64> {
        let cutoff_time = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut archived_count = 0;
        
        let mut index = self.index.write().await;
        let mut to_archive = Vec::new();
        
        for (key, meta) in index.iter() {
            if meta.created_at < cutoff_time && meta.mem_meta.access_count < 5 {
                to_archive.push(key.clone());
            }
        }
        
        for key in to_archive {
            if let Some(meta) = index.remove(&key) {
                // Перемещаем файл в архив
                let archive_path = self.base_path.join("archive").join(&meta.content_hash);
                if let Some(parent) = archive_path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                
                if meta.file_path.exists() {
                    fs::rename(&meta.file_path, &archive_path).await
                        .context("Failed to move file to archive")?;
                    archived_count += 1;
                    tracing::debug!("Archived file: {} -> {}", key, archive_path.display());
                }
            }
        }
        
        if archived_count > 0 {
            self.save_index().await?;
        }
        
        tracing::info!("Archived {} old files", archived_count);
        Ok(archived_count)
    }
    
    /// Очистить архивные файлы старше определённого возраста
    pub async fn cleanup_archive(&self, max_archive_age_days: u64) -> Result<u64> {
        let archive_dir = self.base_path.join("archive");
        if !archive_dir.exists() {
            return Ok(0);
        }
        
        let cutoff_time = Utc::now() - chrono::Duration::days(max_archive_age_days as i64);
        let mut deleted_count = 0;
        
        for entry in WalkDir::new(&archive_dir) {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(created) = metadata.created() {
                        let created_dt: DateTime<Utc> = created.into();
                        if created_dt < cutoff_time {
                            fs::remove_file(entry.path()).await?;
                            deleted_count += 1;
                            tracing::debug!("Deleted archived file: {}", entry.path().display());
                        }
                    }
                }
            }
        }
        
        tracing::info!("Deleted {} archived files", deleted_count);
        Ok(deleted_count)
    }
    
    /// Получить статистику использования диска
    pub async fn get_disk_usage(&self) -> Result<(u64, u64, u64)> {
        let mut blobs_size = 0;
        let mut archive_size = 0;
        let mut total_files = 0;
        
        // Размер активных блобов
        let blobs_dir = self.base_path.join("blobs");
        if blobs_dir.exists() {
            for entry in WalkDir::new(&blobs_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        blobs_size += metadata.len();
                        total_files += 1;
                    }
                }
            }
        }
        
        // Размер архива
        let archive_dir = self.base_path.join("archive");
        if archive_dir.exists() {
            for entry in WalkDir::new(&archive_dir) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        archive_size += metadata.len();
                    }
                }
            }
        }
        
        Ok((blobs_size, archive_size, total_files))
    }
}

#[async_trait]
impl MemoryStore for LongTermStore {
    async fn put(&self, key: &str, data: &[u8], meta: &MemMeta) -> Result<()> {
        let content_hash = Self::compute_hash(data);
        let blob_path = self.get_blob_path(&content_hash);
        
        // Создаём директорию если не существует
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).await
                .context("Failed to create blob directory")?;
        }
        
        // Сжимаем данные если нужно
        let (final_data, compressed) = self.maybe_compress(data).await?;
        
        // Записываем файл
        let mut file = fs::File::create(&blob_path).await
            .context("Failed to create blob file")?;
        file.write_all(&final_data).await
            .context("Failed to write blob data")?;
        file.sync_all().await
            .context("Failed to sync blob file")?;
        
        // Обновляем индекс
        let blob_meta = BlobMeta {
            file_path: blob_path,
            content_hash: content_hash.clone(),
            size_bytes: data.len() as u64,
            compressed,
            mem_meta: meta.clone(),
            created_at: Utc::now(),
        };
        
        {
            let mut index = self.index.write().await;
            index.insert(key.to_string(), blob_meta);
        }
        
        // Сохраняем индекс
        self.save_index().await?;
        
        tracing::trace!("Stored {} bytes in long-term layer: {} (hash: {})", data.len(), key, content_hash);
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<(Vec<u8>, MemMeta)>> {
        let blob_meta = {
            let mut index = self.index.write().await;
            if let Some(meta) = index.get_mut(key) {
                // Обновляем статистику доступа
                meta.mem_meta.access_count += 1;
                meta.mem_meta.last_accessed = Utc::now();
                meta.clone()
            } else {
                return Ok(None);
            }
        };
        
        // Читаем файл
        if !blob_meta.file_path.exists() {
            tracing::warn!("Blob file not found: {}", blob_meta.file_path.display());
            return Ok(None);
        }
        
        let mut file = fs::File::open(&blob_meta.file_path).await
            .context("Failed to open blob file")?;
        
        let mut data = Vec::new();
        file.read_to_end(&mut data).await
            .context("Failed to read blob file")?;
        
        // Распаковываем если нужно
        let final_data = if blob_meta.compressed {
            self.decompress_data(&data)?
        } else {
            data
        };
        
        // Сохраняем обновлённую статистику
        self.save_index().await?;
        
        tracing::trace!("Retrieved {} bytes from long-term layer: {}", final_data.len(), key);
        Ok(Some((final_data, blob_meta.mem_meta)))
    }
    
    async fn delete(&self, key: &str) -> Result<bool> {
        let blob_meta = {
            let mut index = self.index.write().await;
            index.remove(key)
        };
        
        if let Some(meta) = blob_meta {
            // Удаляем файл
            if meta.file_path.exists() {
                fs::remove_file(&meta.file_path).await
                    .context("Failed to delete blob file")?;
                tracing::trace!("Deleted blob file: {}", meta.file_path.display());
            }
            
            // Сохраняем индекс
            self.save_index().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        let index = self.index.read().await;
        Ok(index.contains_key(key))
    }
    
    async fn list_keys(&self) -> Result<Vec<String>> {
        let index = self.index.read().await;
        Ok(index.keys().cloned().collect())
    }
    
    async fn stats(&self) -> Result<LayerStats> {
        let index = self.index.read().await;
        
        let total_items = index.len() as u64;
        let total_size_bytes = index.values().map(|meta| meta.size_bytes).sum();
        
        let oldest_item = index.values()
            .map(|meta| meta.created_at)
            .min();
        
        let newest_item = index.values()
            .map(|meta| meta.created_at)
            .max();
        
        let avg_access_count = if total_items > 0 {
            index.values()
                .map(|meta| meta.mem_meta.access_count as f64)
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
    async fn test_long_term_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = LongTermStore::new(temp_dir.path()).await.unwrap();
        
        let key = "test_blob";
        let data = b"This is a test blob with some content";
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
    async fn test_long_term_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let store = LongTermStore::new(temp_dir.path()).await.unwrap();
        
        let key = "large_file";
        let data = vec![42u8; 10000]; // 10KB файл
        let meta = MemMeta::default();
        
        store.put(key, &data, &meta).await.unwrap();
        
        let result = store.get(key).await.unwrap();
        assert!(result.is_some());
        let (retrieved_data, _) = result.unwrap();
        assert_eq!(retrieved_data, data);
    }
    
    #[tokio::test]
    async fn test_long_term_disk_usage() {
        let temp_dir = TempDir::new().unwrap();
        let store = LongTermStore::new(temp_dir.path()).await.unwrap();
        
        // Добавляем несколько файлов
        for i in 0..3 {
            let key = format!("file_{}", i);
            let data = vec![i as u8; 1000];
            let meta = MemMeta::default();
            store.put(&key, &data, &meta).await.unwrap();
        }
        
        let (blobs_size, archive_size, total_files) = store.get_disk_usage().await.unwrap();
        assert!(blobs_size > 0);
        assert_eq!(archive_size, 0); // Пока нет архивных файлов
        assert_eq!(total_files, 3);
    }
}