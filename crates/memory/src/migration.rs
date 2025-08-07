use crate::types::{Layer, Record};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use tracing::{error, info, warn};

/// Версия схемы базы данных
const CURRENT_SCHEMA_VERSION: u32 = 2;
const SCHEMA_VERSION_KEY: &str = "_schema_version";

/// Старая структура записи (для миграции)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldStoredRecord {
    record: OldRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldRecord {
    pub id: String, // Раньше было строкой
    pub text: String,
    pub embedding: Vec<f32>,
    pub layer: Layer,
    pub kind: String,
    pub tags: Vec<String>,
    pub project: String,
    pub session: String,
    pub ts: DateTime<Utc>,
    pub last_access: DateTime<Utc>,
    pub score: f32,
    pub access_count: u32,
}

/// Менеджер миграций для базы данных
pub struct MigrationManager {
    db: Db,
}

impl MigrationManager {
    /// Открывает sled БД для migration с crash recovery
    fn open_migration_database(db_path: impl AsRef<Path>) -> Result<Db> {
        use sled::Config;

        let config = Config::new()
            .path(db_path.as_ref())
            .mode(sled::Mode::HighThroughput)
            .flush_every_ms(Some(500)) // Migration нужно частое сохранение
            .use_compression(true)
            .compression_factor(19);

        let db = config.open()?;
        info!("Migration database opened with crash recovery");
        Ok(db)
    }

    pub fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db = Self::open_migration_database(db_path)?;
        Ok(Self { db })
    }

    /// Проверить и выполнить необходимые миграции
    pub async fn migrate(&self) -> Result<()> {
        let current_version = self.get_schema_version()?;

        if current_version < CURRENT_SCHEMA_VERSION {
            info!(
                "Начинаем миграцию с версии {} на {}",
                current_version, CURRENT_SCHEMA_VERSION
            );

            // Выполняем миграции последовательно
            if current_version < 1 {
                self.migrate_v0_to_v1().await?;
            }

            if current_version < 2 {
                self.migrate_v1_to_v2().await?;
            }

            // Обновляем версию схемы
            self.set_schema_version(CURRENT_SCHEMA_VERSION)?;

            info!("Миграция завершена успешно");
        } else {
            info!("База данных уже на последней версии схемы");
        }

        Ok(())
    }

    /// Получить текущую версию схемы
    fn get_schema_version(&self) -> Result<u32> {
        if let Some(version_bytes) = self.db.get(SCHEMA_VERSION_KEY)? {
            let version = u32::from_le_bytes(version_bytes.as_ref().try_into()?);
            Ok(version)
        } else {
            Ok(0) // Если версии нет, значит это старая БД
        }
    }

    /// Установить версию схемы
    fn set_schema_version(&self, version: u32) -> Result<()> {
        self.db.insert(SCHEMA_VERSION_KEY, &version.to_le_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    /// Миграция с версии 0 на версию 1: Очистка некорректных данных
    async fn migrate_v0_to_v1(&self) -> Result<()> {
        info!("Выполняем миграцию v0 -> v1: Очистка некорректных данных");

        let mut cleaned_count = 0;
        let mut error_count = 0;

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let tree = self.db.open_tree(layer.table_name())?;
            let mut keys_to_remove = Vec::new();

            for result in tree.iter() {
                match result {
                    Ok((key, value)) => {
                        // Проверяем, можем ли десериализовать запись
                        if bincode::deserialize::<OldStoredRecord>(&value).is_err() {
                            keys_to_remove.push(key);
                            error_count += 1;
                        }
                    }
                    Err(e) => {
                        error!("Ошибка при чтении записи: {}", e);
                        error_count += 1;
                    }
                }
            }

            // Удаляем некорректные записи
            for key in keys_to_remove {
                tree.remove(key)?;
                cleaned_count += 1;
            }
        }

        info!(
            "Очищено {} некорректных записей, {} ошибок",
            cleaned_count, error_count
        );

        Ok(())
    }

    /// Миграция с версии 1 на версию 2: Преобразование ID из строки в UUID
    async fn migrate_v1_to_v2(&self) -> Result<()> {
        info!("Выполняем миграцию v1 -> v2: Преобразование ID в UUID");

        let mut migrated_count = 0;
        let mut skipped_count = 0;

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let tree = self.db.open_tree(layer.table_name())?;
            let mut batch = sled::Batch::default();
            let mut keys_to_remove = Vec::new();

            for result in tree.iter() {
                match result {
                    Ok((key, value)) => {
                        // Пытаемся десериализовать старую запись
                        match bincode::deserialize::<OldStoredRecord>(&value) {
                            Ok(old_stored) => {
                                // Преобразуем в новый формат
                                match self.convert_old_record(old_stored.record) {
                                    Ok(new_record) => {
                                        // Сохраняем с новым ключом (UUID bytes)
                                        let new_stored = crate::storage::StoredRecord {
                                            record: new_record.clone(),
                                        };
                                        let new_value = bincode::serialize(&new_stored)?;
                                        batch.insert(new_record.id.as_bytes(), new_value);

                                        // Помечаем старый ключ для удаления
                                        keys_to_remove.push(key);
                                        migrated_count += 1;
                                    }
                                    Err(e) => {
                                        warn!("Не удалось преобразовать запись: {}", e);
                                        skipped_count += 1;
                                    }
                                }
                            }
                            Err(_) => {
                                // Возможно, это уже новая запись, пропускаем
                                skipped_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Ошибка при чтении записи: {}", e);
                        skipped_count += 1;
                    }
                }
            }

            // Применяем изменения
            tree.apply_batch(batch)?;

            // Удаляем старые ключи
            for key in keys_to_remove {
                tree.remove(key)?;
            }
        }

        info!(
            "Мигрировано {} записей, пропущено {}",
            migrated_count, skipped_count
        );

        Ok(())
    }

    /// Преобразовать старую запись в новую
    fn convert_old_record(&self, old: OldRecord) -> Result<Record> {
        // Пытаемся парсить ID как UUID
        let uuid = if let Ok(uuid) = uuid::Uuid::parse_str(&old.id) {
            uuid
        } else {
            // Если не получается, генерируем новый
            warn!("Не удалось парсить UUID из '{}', генерируем новый", old.id);
            uuid::Uuid::new_v4()
        };

        Ok(Record {
            id: uuid,
            text: old.text,
            embedding: old.embedding,
            layer: old.layer,
            kind: old.kind,
            tags: old.tags,
            project: old.project,
            session: old.session,
            ts: old.ts,
            last_access: old.last_access,
            score: old.score,
            access_count: old.access_count,
        })
    }

    /// Очистить все данные (для тестирования)
    pub async fn clear_all_data(&self) -> Result<()> {
        warn!("Очистка всех данных из базы!");

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let tree = self.db.open_tree(layer.table_name())?;
            tree.clear()?;
        }

        self.db.flush()?;
        info!("Все данные очищены");

        Ok(())
    }

    /// Получить статистику базы данных
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let mut stats = DatabaseStats::default();

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let tree = self.db.open_tree(layer.table_name())?;
            let layer_stats = self.get_layer_stats(&tree, layer)?;

            match layer {
                Layer::Interact => stats.interact = layer_stats,
                Layer::Insights => stats.insights = layer_stats,
                Layer::Assets => stats.assets = layer_stats,
            }
        }

        stats.total_size_bytes = self.db.size_on_disk()?;
        stats.schema_version = self.get_schema_version()?;

        Ok(stats)
    }

    /// Получить статистику для слоя
    fn get_layer_stats(&self, tree: &sled::Tree, layer: Layer) -> Result<LayerStats> {
        let mut stats = LayerStats {
            layer,
            record_count: 0,
            total_size_bytes: 0,
            corrupted_count: 0,
            avg_embedding_dim: 0.0,
        };

        let mut total_dim = 0;
        let mut valid_embeddings = 0;

        for result in tree.iter() {
            match result {
                Ok((_, value)) => {
                    stats.record_count += 1;
                    stats.total_size_bytes += value.len() as u64;

                    // Пытаемся десериализовать для проверки
                    if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value)
                    {
                        total_dim += stored.record.embedding.len();
                        valid_embeddings += 1;
                    } else {
                        stats.corrupted_count += 1;
                    }
                }
                Err(_) => {
                    stats.corrupted_count += 1;
                }
            }
        }

        if valid_embeddings > 0 {
            stats.avg_embedding_dim = total_dim as f64 / valid_embeddings as f64;
        }

        Ok(stats)
    }
}

/// Статистика базы данных
#[derive(Debug, Default)]
pub struct DatabaseStats {
    pub interact: LayerStats,
    pub insights: LayerStats,
    pub assets: LayerStats,
    pub total_size_bytes: u64,
    pub schema_version: u32,
}

/// Статистика слоя
#[derive(Debug)]
pub struct LayerStats {
    pub layer: Layer,
    pub record_count: usize,
    pub total_size_bytes: u64,
    pub corrupted_count: usize,
    pub avg_embedding_dim: f64,
}

impl Default for LayerStats {
    fn default() -> Self {
        Self {
            layer: Layer::Interact,
            record_count: 0,
            total_size_bytes: 0,
            corrupted_count: 0,
            avg_embedding_dim: 0.0,
        }
    }
}
