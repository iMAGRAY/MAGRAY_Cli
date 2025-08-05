use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

use crate::{
    storage::VectorStore,
    types::{Layer, Record},
    backup::{BackupMetadata, LayerInfo},
};

// @component: {"k":"C","id":"incremental_backup","t":"Incremental backup with delta compression","m":{"cur":0,"tgt":95,"u":"%"},"f":["backup","delta","compression"]}

/// Метаданные инкрементального backup
#[derive(Debug, Serialize, Deserialize)]
pub struct IncrementalBackupMetadata {
    pub base_metadata: BackupMetadata,
    pub backup_type: BackupType,
    pub parent_backup: Option<String>, // Path к предыдущему backup
    pub delta_info: DeltaInfo,
    pub compression_ratio: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BackupType {
    Full,
    Incremental { since: DateTime<Utc> },
    Differential { base: String }, // Path к full backup
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeltaInfo {
    pub added_records: usize,
    pub modified_records: usize,
    pub deleted_records: usize,
    pub layer_deltas: HashMap<String, LayerDelta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayerDelta {
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
    pub checksum_changes: Vec<String>, // Checksums измененных записей
}

/// Snapshot состояния для сравнения
#[derive(Debug, Serialize, Deserialize)]
pub struct LayerSnapshot {
    pub layer: Layer,
    pub timestamp: DateTime<Utc>,
    pub record_checksums: HashMap<String, String>, // UUID -> SHA256
    pub total_records: usize,
}

/// Менеджер инкрементального backup
pub struct IncrementalBackupManager {
    base_path: PathBuf,
    snapshots_path: PathBuf,
}

impl IncrementalBackupManager {
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let snapshots_path = base_path.join("snapshots");
        
        // Создаём директории если не существуют
        for path in [&base_path, &snapshots_path] {
            if !path.exists() {
                fs::create_dir_all(path)?;
            }
        }
        
        Ok(Self {
            base_path,
            snapshots_path,
        })
    }

    /// Создать полный backup и snapshot
    pub async fn create_full_backup(
        &self,
        store: Arc<VectorStore>,
        backup_name: Option<String>,
    ) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = backup_name.unwrap_or_else(|| format!("full_backup_{timestamp}"));
        let backup_path = self.base_path.join(format!("{backup_name}.tar.gz"));
        
        info!("🔄 Creating full backup: {:?}", backup_path);
        
        // Создаём snapshot состояния
        let snapshot = self.create_snapshot(&store).await?;
        self.save_snapshot(&backup_name, &snapshot).await?;
        
        // Создаём обычный backup
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        let mut total_records = 0;
        let mut layers_info = Vec::new();
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let layer_file = temp_path.join(format!("{}_records.json", layer.as_str()));
            let (count, size, _checksum) = self.export_layer_full(&store, layer, &layer_file).await?;
            
            layers_info.push(LayerInfo {
                layer,
                record_count: count,
                size_bytes: size,
            });
            
            total_records += count;
        }
        
        // Создаём метаданные
        let metadata = IncrementalBackupMetadata {
            base_metadata: BackupMetadata {
                version: 1,
                created_at: Utc::now(),
                magray_version: env!("CARGO_PKG_VERSION").to_string(),
                layers: layers_info,
                total_records,
                index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
                checksum: None,
                layer_checksums: None,
            },
            backup_type: BackupType::Full,
            parent_backup: None,
            delta_info: DeltaInfo {
                added_records: total_records,
                modified_records: 0,
                deleted_records: 0,
                layer_deltas: HashMap::new(),
            },
            compression_ratio: 1.0,
        };
        
        // Сохраняем метаданные
        let metadata_path = temp_path.join("incremental_metadata.json");
        let metadata_file = File::create(&metadata_path)?;
        serde_json::to_writer_pretty(metadata_file, &metadata)?;
        
        // Создаём архив с высокой компрессией
        let tar_gz = File::create(&backup_path)?;
        let encoder = GzEncoder::new(tar_gz, Compression::best());
        let mut tar = tar::Builder::new(encoder);
        
        tar.append_dir_all(".", temp_path)?;
        tar.finish()?;
        
        let file_size = backup_path.metadata()?.len();
        let compression_ratio = file_size as f64 / (total_records * 1024) as f64; // Приблизительно
        
        info!("✅ Full backup created: {} records, {:.1} MB, compression ratio: {:.2}", 
              total_records, file_size as f64 / 1024.0 / 1024.0, compression_ratio);
        
        Ok(backup_path)
    }

    /// Создать инкрементальный backup
    pub async fn create_incremental_backup(
        &self,
        store: Arc<VectorStore>,
        base_backup_name: &str,
        backup_name: Option<String>,
    ) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = backup_name.unwrap_or_else(|| format!("incr_backup_{timestamp}"));
        let backup_path = self.base_path.join(format!("{backup_name}.tar.gz"));
        
        info!("🔄 Creating incremental backup: {:?}", backup_path);
        
        // Загружаем предыдущий snapshot
        let base_snapshot = self.load_snapshot(base_backup_name).await?;
        
        // Создаём текущий snapshot
        let current_snapshot = self.create_snapshot(&store).await?;
        
        // Вычисляем дельту
        let delta_info = self.calculate_delta(&base_snapshot, &current_snapshot).await?;
        
        if delta_info.added_records == 0 && delta_info.modified_records == 0 && delta_info.deleted_records == 0 {
            info!("📝 No changes detected, skipping incremental backup");
            return Err(anyhow!("No changes to backup"));
        }
        
        info!("📊 Delta: +{} ±{} -{} records", 
              delta_info.added_records, delta_info.modified_records, delta_info.deleted_records);
        
        // Создаём временную директорию для delta файлов
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Экспортируем только изменения
        let total_delta_records = self.export_delta(&store, &base_snapshot, &current_snapshot, temp_path).await?;
        
        // Создаём метаданные
        let metadata = IncrementalBackupMetadata {
            base_metadata: BackupMetadata {
                version: 1,
                created_at: Utc::now(),
                magray_version: env!("CARGO_PKG_VERSION").to_string(),
                layers: Vec::new(), // Заполним позже
                total_records: total_delta_records,
                index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
                checksum: None,
                layer_checksums: None,
            },
            backup_type: BackupType::Incremental { 
                since: base_snapshot[0].timestamp // Берём timestamp первого слоя
            },
            parent_backup: Some(base_backup_name.to_string()),
            delta_info,
            compression_ratio: 1.0, // Будет обновлено после сжатия
        };
        
        // Сохраняем метаданные
        let metadata_path = temp_path.join("incremental_metadata.json");
        let metadata_file = File::create(&metadata_path)?;
        serde_json::to_writer_pretty(metadata_file, &metadata)?;
        
        // Сохраняем новый snapshot
        self.save_snapshot(&backup_name, &current_snapshot).await?;
        
        // Создаём архив с максимальной компрессией (delta данные сжимаются лучше)
        let tar_gz = File::create(&backup_path)?;
        let encoder = GzEncoder::new(tar_gz, Compression::best());
        let mut tar = tar::Builder::new(encoder);
        
        tar.append_dir_all(".", temp_path)?;
        tar.finish()?;
        
        let file_size = backup_path.metadata()?.len();
        info!("✅ Incremental backup created: {} delta records, {:.1} KB", 
              total_delta_records, file_size as f64 / 1024.0);
        
        Ok(backup_path)
    }

    /// Построить цепочку backup'ов от базового до текущего (без рекурсии)
    async fn build_backup_chain(&self, backup_id: &str) -> Result<Vec<String>> {
        let mut chain = Vec::new();
        let mut current_id = backup_id.to_string();
        
        // Идём от текущего backup'а к базовому, собирая цепочку
        loop {
            let backup_path = self.base_path.join(format!("{current_id}.tar.gz"));
            if !backup_path.exists() {
                return Err(anyhow!("Backup not found in chain: {}", current_id));
            }
            
            let metadata = self.read_incremental_metadata(&backup_path)?;
            
            // Добавляем в начало цепочки для правильного порядка восстановления
            chain.insert(0, current_id.clone());
            
            // Проверяем, есть ли родительский backup
            match metadata.backup_type {
                BackupType::Full => {
                    // Достигли полного backup'а - это база цепочки
                    break;
                },
                BackupType::Incremental { .. } => {
                    if let Some(parent) = metadata.parent_backup {
                        current_id = parent;
                    } else {
                        // Incremental без parent - ошибка
                        return Err(anyhow!("Incremental backup {} has no parent", current_id));
                    }
                },
                _ => {
                    return Err(anyhow!("Unsupported backup type in chain"));
                }
            }
            
            // Защита от бесконечных циклов
            if chain.len() > 100 {
                return Err(anyhow!("Backup chain too long, possible cycle detected"));
            }
        }
        
        Ok(chain)
    }

    /// Восстановить из инкрементального backup
    pub async fn restore_incremental_backup(
        &self,
        store: Arc<VectorStore>,
        backup_path: impl AsRef<Path>,
    ) -> Result<IncrementalBackupMetadata> {
        let backup_path = backup_path.as_ref();
        
        // Читаем метаданные
        let metadata = self.read_incremental_metadata(backup_path)?;
        
        match &metadata.backup_type {
            BackupType::Full => {
                info!("📦 Restoring from full backup");
                self.restore_full_backup_data(&store, backup_path).await?;
            },
            BackupType::Incremental { since } => {
                info!("📦 Restoring from incremental backup (since: {})", since);
                
                // Восстанавливаем цепочку backup'ов без рекурсии
                if let Some(ref parent) = metadata.parent_backup {
                    // Собираем всю цепочку backup'ов от базового до текущего
                    let chain = self.build_backup_chain(parent).await?;
                    
                    info!("🔗 Found backup chain with {} elements", chain.len());
                    
                    // Восстанавливаем каждый backup в правильном порядке
                    for backup_id in chain {
                        let backup_path = self.base_path.join(format!("{backup_id}.tar.gz"));
                        if !backup_path.exists() {
                            return Err(anyhow!("Backup in chain not found: {}", backup_id));
                        }
                        
                        let backup_metadata = self.read_incremental_metadata(&backup_path)?;
                        
                        match backup_metadata.backup_type {
                            BackupType::Full => {
                                info!("  📦 Restoring full backup: {}", backup_id);
                                self.restore_full_backup_data(&store, &backup_path).await?;
                            },
                            BackupType::Incremental { .. } => {
                                info!("  🔄 Applying incremental changes: {}", backup_id);
                                self.apply_delta_changes(&store, &backup_path).await?;
                            },
                            _ => {
                                warn!("  ⚠️ Skipping unsupported backup type in chain: {}", backup_id);
                            }
                        }
                    }
                }
                
                // Применяем delta изменения
                self.apply_delta_changes(&store, backup_path).await?;
            },
            BackupType::Differential { base } => {
                info!("📦 Restoring from differential backup (base: {})", base);
                // Реализация differential restore
                return Err(anyhow!("Differential restore not implemented yet"));
            }
        }
        
        info!("✅ Incremental restore completed");
        Ok(metadata)
    }

    /// Создать snapshot текущего состояния всех слоев
    async fn create_snapshot(&self, store: &VectorStore) -> Result<Vec<LayerSnapshot>> {
        let mut snapshots = Vec::new();
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let mut record_checksums = HashMap::new();
            let mut count = 0;
            
            let iter = store.iter_layer(layer).await?;
            for (_key, value) in iter.flatten() {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    let record_json = serde_json::to_string(&stored.record)?;
                    let mut hasher = Sha256::new();
                    hasher.update(record_json.as_bytes());
                    let checksum = format!("{:x}", hasher.finalize());
                    
                    let id = stored.record.id.to_string();
                    record_checksums.insert(id, checksum);
                    count += 1;
                }
            }
            
            snapshots.push(LayerSnapshot {
                layer,
                timestamp: Utc::now(),
                record_checksums,
                total_records: count,
            });
            
            debug!("📸 Snapshot created for layer {:?}: {} records", layer, count);
        }
        
        Ok(snapshots)
    }

    /// Сохранить snapshot
    async fn save_snapshot(&self, backup_name: &str, snapshots: &[LayerSnapshot]) -> Result<()> {
        let snapshot_path = self.snapshots_path.join(format!("{backup_name}_snapshot.json"));
        let file = File::create(snapshot_path)?;
        serde_json::to_writer_pretty(file, snapshots)?;
        Ok(())
    }

    /// Загрузить snapshot
    async fn load_snapshot(&self, backup_name: &str) -> Result<Vec<LayerSnapshot>> {
        let snapshot_path = self.snapshots_path.join(format!("{backup_name}_snapshot.json"));
        if !snapshot_path.exists() {
            return Err(anyhow!("Snapshot not found: {:?}", snapshot_path));
        }
        
        let file = File::open(snapshot_path)?;
        let snapshots = serde_json::from_reader(BufReader::new(file))?;
        Ok(snapshots)
    }

    /// Вычислить дельту между двумя snapshots
    async fn calculate_delta(
        &self, 
        base: &[LayerSnapshot], 
        current: &[LayerSnapshot]
    ) -> Result<DeltaInfo> {
        let mut total_added = 0;
        let mut total_modified = 0;
        let mut total_deleted = 0;
        let mut layer_deltas = HashMap::new();
        
        for (base_snapshot, current_snapshot) in base.iter().zip(current.iter()) {
            if base_snapshot.layer != current_snapshot.layer {
                return Err(anyhow!("Snapshot layer mismatch"));
            }
            
            let base_ids: HashSet<_> = base_snapshot.record_checksums.keys().collect();
            let current_ids: HashSet<_> = current_snapshot.record_checksums.keys().collect();
            
            // Новые записи
            let added: Vec<_> = current_ids.difference(&base_ids).collect();
            let added_count = added.len();
            
            // Удаленные записи  
            let deleted: Vec<_> = base_ids.difference(&current_ids).collect();
            let deleted_count = deleted.len();
            
            // Измененные записи (общие ID с разными checksums)
            let mut modified_count = 0;
            let mut checksum_changes = Vec::new();
            
            for id in base_ids.intersection(&current_ids) {
                if let (Some(base_checksum), Some(current_checksum)) = (
                    base_snapshot.record_checksums.get(*id),
                    current_snapshot.record_checksums.get(*id)
                ) {
                    if base_checksum != current_checksum {
                        modified_count += 1;
                        checksum_changes.push(current_checksum.clone());
                    }
                }
            }
            
            total_added += added_count;
            total_modified += modified_count;
            total_deleted += deleted_count;
            
            layer_deltas.insert(
                base_snapshot.layer.as_str().to_string(),
                LayerDelta {
                    added: added_count,
                    modified: modified_count,
                    deleted: deleted_count,
                    checksum_changes,
                }
            );
            
            info!("📊 Layer {:?} delta: +{} ±{} -{}", 
                  base_snapshot.layer, added_count, modified_count, deleted_count);
        }
        
        Ok(DeltaInfo {
            added_records: total_added,
            modified_records: total_modified,
            deleted_records: total_deleted,
            layer_deltas,
        })
    }

    /// Экспортировать только изменения
    async fn export_delta(
        &self,
        store: &VectorStore,
        base_snapshots: &[LayerSnapshot],
        current_snapshots: &[LayerSnapshot],
        output_dir: &Path,
    ) -> Result<usize> {
        let mut total_exported = 0;
        
        for (base_snapshot, current_snapshot) in base_snapshots.iter().zip(current_snapshots.iter()) {
            let layer = current_snapshot.layer;
            
            // Определяем какие записи нужно экспортировать
            let base_checksums = &base_snapshot.record_checksums;
            let current_checksums = &current_snapshot.record_checksums;
            
            let mut records_to_export = Vec::new();
            let iter = store.iter_layer(layer).await?;
            
            for (_key, value) in iter.flatten() {
                if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                    let id = stored.record.id.to_string();
                    
                    // Экспортируем если:
                    // 1. Новая запись (не было в base)
                    // 2. Измененная запись (разные checksums)
                    let should_export = if let Some(current_checksum) = current_checksums.get(&id) {
                        match base_checksums.get(&id) {
                            None => true, // Новая запись
                            Some(base_checksum) => base_checksum != current_checksum, // Изменена
                        }
                    } else {
                        false // Запись была удалена
                    };
                    
                    if should_export {
                        records_to_export.push(stored.record);
                    }
                }
            }
            
            // Сохраняем delta записи
            if !records_to_export.is_empty() {
                let delta_file = output_dir.join(format!("{}_delta.json", layer.as_str()));
                self.save_records_to_file(&records_to_export, &delta_file)?;
                total_exported += records_to_export.len();
                
                info!("💾 Exported {} delta records for layer {:?}", 
                      records_to_export.len(), layer);
            }
        }
        
        Ok(total_exported)
    }

    /// Применить delta изменения
    async fn apply_delta_changes(&self, store: &VectorStore, backup_path: &Path) -> Result<()> {
        // Распаковываем backup
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(temp_path)?;
        
        // Применяем delta файлы
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let delta_file = temp_path.join(format!("{}_delta.json", layer.as_str()));
            if delta_file.exists() {
                let records = self.load_records_from_file(&delta_file)?;
                if !records.is_empty() {
                    let refs: Vec<&Record> = records.iter().collect();
                    store.insert_batch_atomic(&refs).await?;
                    info!("✅ Applied {} delta records to layer {:?}", records.len(), layer);
                }
            }
        }
        
        Ok(())
    }

    /// Вспомогательные методы
    async fn export_layer_full(&self, store: &VectorStore, layer: Layer, output_path: &Path) -> Result<(usize, usize, String)> {
        let mut count = 0;
        let mut records = Vec::new();
        let mut hasher = Sha256::new();
        
        let iter = store.iter_layer(layer).await?;
        for (_, value) in iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                let record_json = serde_json::to_string(&stored.record)?;
                hasher.update(record_json.as_bytes());
                records.push(stored.record);
                count += 1;
            }
        }
        
        self.save_records_to_file(&records, output_path)?;
        let size = output_path.metadata()?.len() as usize;
        let checksum = format!("{:x}", hasher.finalize());
        
        Ok((count, size, checksum))
    }

    async fn restore_full_backup_data(&self, store: &VectorStore, backup_path: &Path) -> Result<()> {
        info!("🔄 Restoring full backup data from: {:?}", backup_path);
        
        // Распаковываем backup
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(temp_path)?;
        
        // Восстанавливаем каждый слой
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let layer_file = temp_path.join(format!("{}_records.json", layer.as_str()));
            if layer_file.exists() {
                let records = self.load_records_from_file(&layer_file)?;
                if !records.is_empty() {
                    let refs: Vec<&Record> = records.iter().collect();
                    store.insert_batch_atomic(&refs).await?;
                    info!("✅ Restored {} records to layer {:?}", records.len(), layer);
                }
            }
        }
        
        Ok(())
    }

    fn save_records_to_file(&self, records: &[Record], path: &Path) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        
        for record in records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        
        writer.flush()?;
        Ok(())
    }

    fn load_records_from_file(&self, path: &Path) -> Result<Vec<Record>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        
        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            if let Ok(record) = serde_json::from_str::<Record>(&line) {
                records.push(record);
            }
        }
        
        Ok(records)
    }

    fn read_incremental_metadata(&self, backup_path: &Path) -> Result<IncrementalBackupMetadata> {
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        
        for entry in tar.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            
            if path.file_name() == Some(std::ffi::OsStr::new("incremental_metadata.json")) {
                let metadata: IncrementalBackupMetadata = serde_json::from_reader(entry)?;
                return Ok(metadata);
            }
        }
        
        Err(anyhow!("Incremental metadata not found in backup"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_incremental_backup_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = IncrementalBackupManager::new(temp_dir.path()).unwrap();
        
        // Тест создания snapshot
        let store_dir = TempDir::new().unwrap();
        let store = VectorStore::new(store_dir.path()).await.unwrap();
        
        let snapshots = manager.create_snapshot(&store).await.unwrap();
        assert_eq!(snapshots.len(), 3); // 3 layers
        
        // Тест сохранения/загрузки snapshot
        manager.save_snapshot("test", &snapshots).await.unwrap();
        let loaded = manager.load_snapshot("test").await.unwrap();
        assert_eq!(loaded.len(), snapshots.len());
    }
}