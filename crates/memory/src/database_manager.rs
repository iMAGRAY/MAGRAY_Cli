use anyhow::Result;
use parking_lot::Mutex;
use sled::{Config, Db};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Централизованный менеджер sled баз данных для предотвращения concurrent access issues
pub struct DatabaseManager {
    /// Открытые соединения с базами данных
    connections: Arc<Mutex<HashMap<PathBuf, Arc<Db>>>>,
}

impl DatabaseManager {
    /// Создать новый менеджер баз данных
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Получить глобальный экземпляр менеджера
    pub fn global() -> &'static DatabaseManager {
        static INSTANCE: std::sync::OnceLock<DatabaseManager> = std::sync::OnceLock::new();
        INSTANCE.get_or_init(DatabaseManager::new)
    }

    /// Получить или создать sled базу данных с оптимальными настройками для concurrent access
    pub fn get_database(&self, db_path: impl AsRef<Path>) -> Result<Arc<Db>> {
        let path = db_path.as_ref().to_path_buf();
        let mut connections = self.connections.lock();

        if let Some(db) = connections.get(&path) {
            debug!("Reusing existing sled connection for: {:?}", path);
            return Ok(db.clone());
        }

        debug!("Creating new sled connection for: {:?}", path);
        let db = Arc::new(self.create_optimized_database(&path)?);
        connections.insert(path, db.clone());

        Ok(db)
    }

    /// Создать оптимизированную sled базу данных
    fn create_optimized_database(&self, db_path: &Path) -> Result<Db> {
        let config = Config::new()
            .path(db_path)
            // Оптимизации для concurrent access
            .cache_capacity(64 * 1024 * 1024) // 64MB cache
            .flush_every_ms(Some(5000)) // Flush каждые 5 секунд
            // .compression_factor(6) // отключено: конфликт zstd версий из-за tantivy
            // .use_compression(true)
            // Режим для лучшей производительности
            .mode(sled::Mode::HighThroughput);

        let db = config.open()?;

        info!(
            "✅ Opened sled database: {:?} with optimized settings",
            db_path
        );
        Ok(db)
    }

    /// Создать базу данных для кэша с настройками оптимизированными для временных данных
    pub fn get_cache_database(&self, cache_path: impl AsRef<Path>) -> Result<Arc<Db>> {
        let path = cache_path.as_ref().to_path_buf();
        let mut connections = self.connections.lock();

        if let Some(db) = connections.get(&path) {
            debug!("Reusing existing cache connection for: {:?}", path);
            return Ok(db.clone());
        }

        debug!("Creating new cache connection for: {:?}", path);

        // Оптимизированные настройки для быстрого старта
        let config = Config::new()
            .path(&path)
            // Уменьшаем начальный размер кэша для быстрой инициализации
            .cache_capacity(8 * 1024 * 1024) // 8MB вместо 32MB - растет динамически
            .flush_every_ms(Some(30000)) // Flush еще реже - 30 секунд
            // .compression_factor(1) // отключено: избежание zstd конфликтов
            // .use_compression(false) // Нет сжатия = быстрее старт
            .mode(sled::Mode::HighThroughput)
            // Отключаем синхронизацию на диск при старте
            .temporary(true); // Помечаем как временную БД для оптимизаций

        let start = std::time::Instant::now();
        let db = Arc::new(config.open()?);
        let elapsed = start.elapsed();

        connections.insert(path, db.clone());

        info!(
            "✅ Opened cache database in {:?} with fast-start settings",
            elapsed
        );
        Ok(db)
    }

    /// Создать базу данных для системных данных (метрики, промотирование, etc)
    #[allow(dead_code)]
    pub fn get_system_database(&self, system_path: impl AsRef<Path>) -> Result<Arc<Db>> {
        let path = system_path.as_ref().to_path_buf();
        let mut connections = self.connections.lock();

        if let Some(db) = connections.get(&path) {
            debug!("Reusing existing system connection for: {:?}", path);
            return Ok(db.clone());
        }

        debug!("Creating new system connection for: {:?}", path);

        let config = Config::new()
            .path(&path)
            // Настройки для системных данных (высокая durability)
            .cache_capacity(16 * 1024 * 1024) // 16MB cache
            .flush_every_ms(Some(2000)) // Чаще flush для критичных данных
            // .compression_factor(8) // отключено: избежание zstd конфликтов
            // .use_compression(true)
            .mode(sled::Mode::LowSpace); // Экономия места для системных данных

        let db = Arc::new(config.open()?);
        connections.insert(path, db.clone());

        info!("✅ Opened system database with durability-optimized settings");
        Ok(db)
    }

    /// Graceful shutdown всех соединений
    pub fn shutdown(&self) -> Result<()> {
        let mut connections = self.connections.lock();
        let count = connections.len();

        for (path, db) in connections.drain() {
            debug!("Flushing and closing database: {:?}", path);
            if let Err(e) = db.flush() {
                warn!("Failed to flush database {:?}: {}", path, e);
            }
        }

        info!("✅ Closed {} sled database connections", count);
        Ok(())
    }

    /// Получить статистику всех открытых соединений
    #[allow(dead_code)] // Для мониторинга и телеметрии
    pub fn get_connection_stats(&self) -> HashMap<PathBuf, DatabaseStats> {
        let connections = self.connections.lock();
        let mut stats = HashMap::new();

        for (path, db) in connections.iter() {
            let db_stats = DatabaseStats {
                path: path.clone(),
                size_on_disk: db.size_on_disk().unwrap_or(0),
                len: db.len(),
                was_recovered: db.was_recovered(),
            };
            stats.insert(path.clone(), db_stats);
        }

        stats
    }

    /// Принудительно flush всех баз данных
    #[allow(dead_code)] // Для административных целей
    pub fn flush_all(&self) -> Result<()> {
        let connections = self.connections.lock();

        for (path, db) in connections.iter() {
            debug!("Flushing database: {:?}", path);
            db.flush()?;
        }

        info!("✅ Flushed {} databases", connections.len());
        Ok(())
    }
}

/// Статистика подключения к базе данных
#[derive(Debug, Clone)]
#[allow(dead_code)] // Для будущего мониторинга
pub struct DatabaseStats {
    pub path: PathBuf,
    pub size_on_disk: u64,
    pub len: usize,
    pub was_recovered: bool,
}

impl Default for DatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}

// Graceful shutdown при завершении процесса
impl Drop for DatabaseManager {
    fn drop(&mut self) {
        if let Err(e) = self.shutdown() {
            warn!("Error during DatabaseManager shutdown: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_manager_singleton() {
        let manager1 = DatabaseManager::global();
        let manager2 = DatabaseManager::global();

        // Должен быть тот же экземпляр
        assert!(std::ptr::eq(manager1, manager2));
    }

    #[tokio::test]
    async fn test_database_reuse() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        let manager = DatabaseManager::new();

        let db1 = manager.get_database(&db_path)?;
        let db2 = manager.get_database(&db_path)?;

        // Должна быть та же база данных
        assert!(Arc::ptr_eq(&db1, &db2));

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_access() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("concurrent.db");

        let manager = DatabaseManager::global();

        // Параллельное создание соединений
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let path = db_path.clone();
                tokio::spawn(async move {
                    let db = manager.get_database(&path).map_err(|e| {
                        anyhow::anyhow!("Failed to get database for thread {}: {}", i, e)
                    })?;
                    let tree = db.open_tree(format!("test_{}", i)).map_err(|e| {
                        anyhow::anyhow!("Failed to open tree for thread {}: {}", i, e)
                    })?;
                    tree.insert("key", "value")
                        .map_err(|e| anyhow::anyhow!("Failed to insert for thread {}: {}", i, e))?;
                    tree.flush()
                        .map_err(|e| anyhow::anyhow!("Failed to flush for thread {}: {}", i, e))?;
                    Ok::<(), anyhow::Error>(())
                })
            })
            .collect();

        // Ждем завершения всех операций
        for handle in handles {
            let _ = handle.await?;
        }

        Ok(())
    }
}
