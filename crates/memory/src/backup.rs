use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::Builder as TarBuilder;
use tracing::{debug, info, warn, error};
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use flate2::Compression;

use crate::{
    storage::VectorStore,
    types::{Layer, Record},
    vector_index_hnswlib::HnswRsConfig,
};

/// Версия формата backup для совместимости
const BACKUP_FORMAT_VERSION: u32 = 1;

/// Метаданные backup
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub magray_version: String,
    pub layers: Vec<LayerInfo>,
    pub total_records: usize,
    pub index_config: HnswRsConfig,
    /// SHA256 контрольная сумма всех данных backup'а
    pub checksum: Option<String>,
    /// Детальные контрольные суммы по слоям
    pub layer_checksums: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayerInfo {
    pub layer: Layer,
    pub record_count: usize,
    pub size_bytes: usize,
}

/// Менеджер резервного копирования
pub struct BackupManager {
    base_path: PathBuf,
}

impl BackupManager {
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Создаём директорию для backup если не существует
        if !base_path.exists() {
            fs::create_dir_all(&base_path)?;
        }
        
        Ok(Self { base_path })
    }

    /// Создать полный backup системы памяти
    pub async fn create_backup(
        &self,
        store: Arc<VectorStore>,
        backup_name: Option<String>,
    ) -> Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
<<<<<<< HEAD
        let backup_name = backup_name.unwrap_or_else(|| format!("backup_{timestamp}"));
        let backup_path = self.base_path.join(format!("{backup_name}.tar.gz"));
=======
        let backup_name = backup_name.unwrap_or_else(|| format!("backup_{}", timestamp));
        let backup_path = self.base_path.join(format!("{}.tar.gz", backup_name));
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        
        info!("Creating backup: {:?}", backup_path);
        
        // Создаём временную директорию для сбора файлов
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Собираем статистику по слоям
        let mut layers_info = Vec::new();
        let mut total_records = 0;
        
        // Экспортируем данные каждого слоя с вычислением контрольных сумм
        let mut layer_checksums = std::collections::HashMap::new();
        let mut master_hasher = Sha256::new();
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let layer_file = temp_path.join(format!("{}_records.json", layer.as_str()));
            let (count, size, layer_checksum) = self.export_layer_with_checksum(&store, layer, &layer_file).await?;
            
            layers_info.push(LayerInfo {
                layer,
                record_count: count,
                size_bytes: size as usize,
            });
            
            // Сохраняем контрольную сумму слоя
            let layer_name = layer.as_str().to_string();
            layer_checksums.insert(layer_name.clone(), layer_checksum.clone());
            
            // Добавляем в master checksum для общей контрольной суммы
            master_hasher.update(layer_checksum.as_bytes());
            master_hasher.update(count.to_le_bytes());
            
            total_records += count;
            info!("✅ Exported {} records from layer {:?} (checksum: {}...)", 
                  count, layer, &layer_checksum[..16]);
        }
        
        // Вычисляем общую контрольную сумму
        let master_checksum = format!("{:x}", master_hasher.finalize());
        
        // Создаём метаданные с контрольными суммами
        let metadata = BackupMetadata {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: layers_info,
            total_records,
            index_config: HnswRsConfig::default(), // Получаем из первого слоя
            checksum: Some(master_checksum.clone()),
            layer_checksums: Some(layer_checksums),
        };
        
        info!("🔒 Backup integrity: master checksum {}", &master_checksum[..16]);
        
        // Сохраняем метаданные
        let metadata_path = temp_path.join("metadata.json");
        let metadata_file = File::create(&metadata_path)?;
        serde_json::to_writer_pretty(metadata_file, &metadata)?;
        
        // Создаём tar.gz архив
        let tar_gz = File::create(&backup_path)?;
        let encoder = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = TarBuilder::new(encoder);
        
        // Добавляем все файлы в архив
        tar.append_dir_all(".", temp_path)?;
        
        // Финализируем архив
        tar.finish()?;
        
        info!("Backup created successfully: {:?} ({} records)", backup_path, total_records);
        Ok(backup_path)
    }

    /// Восстановить из backup
    pub async fn restore_backup(
        &self,
        store: Arc<VectorStore>,
        backup_path: impl AsRef<Path>,
    ) -> Result<BackupMetadata> {
        let backup_path = backup_path.as_ref();
        
        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found: {:?}", backup_path));
        }
        
        info!("Restoring from backup: {:?}", backup_path);
        
        // Создаём временную директорию для распаковки
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Распаковываем архив
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(temp_path)?;
        
        // Читаем метаданные
        let metadata_path = temp_path.join("metadata.json");
        let metadata_file = File::open(&metadata_path)?;
        let metadata: BackupMetadata = serde_json::from_reader(BufReader::new(metadata_file))?;
        
        info!("📦 Restoring backup: version {}, {} total records", 
              metadata.version, metadata.total_records);
        
        // Проверяем контрольные суммы если доступны
        if let Some(ref expected_checksum) = metadata.checksum {
            info!("🔒 Verifying backup integrity (checksum: {}...)", &expected_checksum[..16]);
            let verified = self.verify_backup_integrity(temp_path, &metadata).await?;
            if !verified {
                return Err(anyhow!("❌ Backup integrity check FAILED - corrupted data detected"));
            }
            info!("✅ Backup integrity verified successfully");
        } else {
            warn!("⚠️ No checksum found - skipping integrity verification");
        }
        
        // Проверяем версию формата
        if metadata.version != BACKUP_FORMAT_VERSION {
            return Err(anyhow!(
                "Incompatible backup format version: {} (expected {})",
                metadata.version,
                BACKUP_FORMAT_VERSION
            ));
        }
        
        info!("Backup metadata: {} records from {}", 
              metadata.total_records, 
              metadata.created_at);
        
        // Восстанавливаем каждый слой
        let mut restored_count = 0;
        
        for layer_info in &metadata.layers {
            let layer_file = temp_path.join(format!("{}_records.json", layer_info.layer.as_str()));
            
            if layer_file.exists() {
                let count = self.import_layer(&store, layer_info.layer, &layer_file).await?;
                restored_count += count;
                info!("Restored {} records to layer {:?}", count, layer_info.layer);
            } else {
                warn!("Layer file not found: {:?}", layer_file);
            }
        }
        
        info!("Restore completed: {} records restored", restored_count);
        
        Ok(metadata)
    }

    /// Экспортировать слой в файл
    /// Экспорт слоя с вычислением контрольной суммы
    async fn export_layer_with_checksum(
        &self,
        store: &Arc<VectorStore>,
        layer: Layer,
        output_path: &Path,
    ) -> Result<(usize, u64, String)> {
        let mut count = 0;
        let mut records = Vec::new();
        let mut hasher = Sha256::new();
        
        // Итерируемся по всем записям слоя
        let iter = store.iter_layer(layer).await?;
        
        for item in iter {
            match item {
                Ok((key, value)) => {
                    // Десериализуем запись
                    if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                        // Добавляем запись в hash для контрольной суммы
                        let record_json = serde_json::to_string(&stored.record)?;
                        hasher.update(record_json.as_bytes());
                        
                        records.push(stored.record);
                        count += 1;
                        
                        // Периодически сохраняем для экономии памяти
                        if records.len() >= 1000 {
<<<<<<< HEAD
                            self.append_records_to_file(output_path, &records)?;
=======
                            self.append_records_to_file(&output_path, &records)?;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                            records.clear();
                        }
                    } else {
                        debug!("Failed to deserialize record with key: {:?}", key);
                    }
                }
                Err(e) => {
                    error!("Error iterating layer {}: {}", layer.as_str(), e);
                }
            }
        }
        
        // Сохраняем оставшиеся записи
        if !records.is_empty() {
<<<<<<< HEAD
            self.append_records_to_file(output_path, &records)?;
=======
            self.append_records_to_file(&output_path, &records)?;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        }
        
        let file_size = if output_path.exists() {
            output_path.metadata()?.len()
        } else {
            0
        };
        
        let checksum = format!("{:x}", hasher.finalize());
        
        Ok((count, file_size, checksum))
    }
    
    /// Legacy метод для обратной совместимости  
    #[allow(dead_code)]
    async fn export_layer(
        &self,
        store: &Arc<VectorStore>,
        layer: Layer,
        output_path: &Path,
    ) -> Result<(usize, usize)> {
        let (count, file_size, _checksum) = self.export_layer_with_checksum(store, layer, output_path).await?;
        Ok((count, file_size as usize))
    }
    
    /// Проверяет целостность backup'а по контрольным суммам
    async fn verify_backup_integrity(&self, backup_dir: &Path, metadata: &BackupMetadata) -> Result<bool> {
        info!("🔍 Starting backup integrity verification...");
        
        let mut master_hasher = Sha256::new();
        let mut verified_layers = 0;
        
        // Проверяем контрольные суммы по слоям если доступны
        if let Some(ref layer_checksums) = metadata.layer_checksums {
            for layer_info in &metadata.layers {
                let layer_name = layer_info.layer.as_str();
<<<<<<< HEAD
                let layer_file = backup_dir.join(format!("{layer_name}_records.json"));
=======
                let layer_file = backup_dir.join(format!("{}_records.json", layer_name));
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
                
                if !layer_file.exists() {
                    warn!("⚠️ Layer file missing: {:?}", layer_file);
                    continue;
                }
                
                // Вычисляем контрольную сумму слоя
                let actual_checksum = self.calculate_file_checksum(&layer_file).await?;
                
                if let Some(expected_checksum) = layer_checksums.get(layer_name) {
                    if &actual_checksum == expected_checksum {
                        debug!("✅ Layer {} checksum verified", layer_name);
                        verified_layers += 1;
                        
                        // Добавляем в master checksum
                        master_hasher.update(actual_checksum.as_bytes());
                        master_hasher.update(layer_info.record_count.to_le_bytes());
                    } else {
                        error!("❌ Layer {} checksum mismatch: expected {}, got {}", 
                               layer_name, expected_checksum, actual_checksum);
                        return Ok(false);
                    }
                } else {
                    warn!("⚠️ No checksum found for layer {}", layer_name);
                }
            }
        } else {
            warn!("⚠️ No layer checksums available for verification");
        }
        
        // Проверяем общую контрольную сумму
        if let Some(ref expected_master) = metadata.checksum {
            let actual_master = format!("{:x}", master_hasher.finalize());
            
            if &actual_master == expected_master {
                info!("✅ Master checksum verified: {}", &actual_master[..16]);
                return Ok(true);
            } else {
                error!("❌ Master checksum mismatch: expected {}, got {}", 
                       expected_master, actual_master);
                return Ok(false);
            }
        }
        
        // Если нет master checksum, но есть layer checksums
        if verified_layers > 0 {
            info!("✅ {} layer checksums verified (no master checksum)", verified_layers);
            return Ok(true);
        }
        
        // Никаких checksums нет
        warn!("⚠️ No checksums available for verification");
        Ok(true) // Считаем валидным если нет данных для проверки
    }
    
    /// Вычисляет SHA256 контрольную сумму JSON файла
    async fn calculate_file_checksum(&self, file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        
        // Читаем и обрабатываем файл как JSON для консистентности
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Парсим JSON чтобы убрать форматирование differences
        let records: Vec<crate::types::Record> = serde_json::from_str(&contents)?;
        
        // Пересериализуем в консистентном формате
        for record in records {
            let normalized_json = serde_json::to_string(&record)?;
            hasher.update(normalized_json.as_bytes());
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Импортировать слой из файла
    async fn import_layer(
        &self,
        store: &Arc<VectorStore>,
        layer: Layer,
        input_path: &Path,
    ) -> Result<usize> {
        let file = File::open(input_path)?;
        let reader = BufReader::new(file);
        
        let mut count = 0;
        let mut batch = Vec::new();
        
        // Читаем записи построчно (каждая строка - JSON объект)
        for line in std::io::BufRead::lines(reader) {
            let line = line?;
            
            if let Ok(mut record) = serde_json::from_str::<Record>(&line) {
                // Убеждаемся что запись в правильном слое
                record.layer = layer;
                batch.push(record);
                
                // Batch insert для производительности
                if batch.len() >= 100 {
                    let refs: Vec<&Record> = batch.iter().collect();
                    store.insert_batch_atomic(&refs).await?;
                    count += batch.len();
                    batch.clear();
                }
            } else {
                debug!("Failed to parse record: {}", line);
            }
        }
        
        // Вставляем оставшиеся записи
        if !batch.is_empty() {
            let refs: Vec<&Record> = batch.iter().collect();
            store.insert_batch_atomic(&refs).await?;
            count += batch.len();
        }
        
        Ok(count)
    }

    /// Добавить записи в файл (для потоковой записи)
    fn append_records_to_file(&self, path: &Path, records: &[Record]) -> Result<()> {
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        let mut writer = BufWriter::new(file);
        
        for record in records {
            serde_json::to_writer(&mut writer, record)?;
            writer.write_all(b"\n")?;
        }
        
        writer.flush()?;
        Ok(())
    }

    /// Получить список доступных backup файлов
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                if let Ok(metadata) = self.read_backup_metadata(&path) {
                    let file_size = entry.metadata()?.len();
                    
                    backups.push(BackupInfo {
                        path,
                        metadata,
                        size_bytes: file_size,
                    });
                }
            }
        }
        
        // Сортируем по дате создания (новые первые)
        backups.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));
        
        Ok(backups)
    }

    /// Прочитать метаданные backup без полной распаковки
<<<<<<< HEAD
    pub fn read_backup_metadata(&self, backup_path: &Path) -> Result<BackupMetadata> {
=======
    fn read_backup_metadata(&self, backup_path: &Path) -> Result<BackupMetadata> {
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        
        // Ищем файл metadata.json в архиве
        for entry in tar.entries()? {
            let entry = entry?;
            let path = entry.path()?;
            
            if path.file_name() == Some(std::ffi::OsStr::new("metadata.json")) {
                let metadata: BackupMetadata = serde_json::from_reader(entry)?;
                return Ok(metadata);
            }
        }
        
        Err(anyhow!("Metadata not found in backup"))
    }

    /// Удалить старые backup файлы
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        let backups = self.list_backups()?;
        let mut deleted = 0;
        
        // Оставляем только последние keep_count backup файлов
        for (i, backup_info) in backups.iter().enumerate() {
            if i >= keep_count {
                fs::remove_file(&backup_info.path)?;
                deleted += 1;
                info!("Deleted old backup: {:?}", backup_info.path);
            }
        }
        
        Ok(deleted)
    }
<<<<<<< HEAD

    /// Проверить целостность backup файла
    pub async fn verify_backup(&self, backup_path: impl AsRef<Path>) -> Result<bool> {
        let backup_path = backup_path.as_ref();
        
        if !backup_path.exists() {
            return Ok(false);
        }
        
        // Читаем метаданные для получения контрольных сумм
        let metadata = self.read_backup_metadata(backup_path)?;
        
        // Проверяем есть ли контрольные суммы
        if metadata.checksum.is_none() && metadata.layer_checksums.is_none() {
            warn!("Backup не содержит контрольных сумм для верификации: {:?}", backup_path);
            return Ok(true); // Считаем валидным если контрольных сумм нет
        }
        
        // Создаём временную директорию для проверки
        let temp_dir = tempfile::TempDir::new()?;
        let temp_path = temp_dir.path();
        
        // Распаковываем архив
        let tar_gz = File::open(backup_path)?;
        let decoder = GzDecoder::new(tar_gz);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(temp_path)?;
        
        // Проверяем контрольные суммы слоёв если есть
        if let Some(layer_checksums) = &metadata.layer_checksums {
            for (layer_name, expected_checksum) in layer_checksums {
                let layer_file = temp_path.join(format!("{}_records.json", layer_name));
                if layer_file.exists() {
                    let actual_checksum = self.calculate_file_checksum(&layer_file).await?;
                    if actual_checksum != *expected_checksum {
                        warn!("Контрольная сумма слоя {} не совпадает: ожидается {}, получено {}", 
                              layer_name, expected_checksum, actual_checksum);
                        return Ok(false);
                    }
                }
            }
        }
        
        info!("Backup прошёл проверку целостности: {:?}", backup_path);
        Ok(true)
    }

=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

/// Информация о backup файле
#[derive(Debug)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub metadata: BackupMetadata,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_backup_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = BackupManager::new(temp_dir.path()).unwrap();
        
        // Проверяем что директория создана
        assert!(temp_dir.path().exists());
        
        // Проверяем список backup (должен быть пустой)
        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = BackupMetadata {
            version: BACKUP_FORMAT_VERSION,
            created_at: Utc::now(),
            magray_version: "0.1.0".to_string(),
            layers: vec![
                LayerInfo {
                    layer: Layer::Interact,
                    record_count: 100,
                    size_bytes: 1024,
                }
            ],
            total_records: 100,
            index_config: HnswRsConfig::default(),
            checksum: None,
            layer_checksums: None,
        };
        
        let json = serde_json::to_string_pretty(&metadata).unwrap();
        let parsed: BackupMetadata = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.version, metadata.version);
        assert_eq!(parsed.total_records, metadata.total_records);
    }
}