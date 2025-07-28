use anyhow::Result;
use async_trait::async_trait;
use magray_core::{DocStore, MemLayer};
use rusqlite::{Connection, params, Row};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, error};
use serde_json::Value;

// === Legacy trait for compatibility ===
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn put(&self, layer: MemLayer, key: &str, data: &[u8]) -> Result<()>;
    async fn get(&self, layer: MemLayer, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, layer: MemLayer, key: &str) -> Result<()>;
}

// === SQLite Implementation ===

pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    blobs_dir: PathBuf,
}

impl SqliteStore {
    pub fn new(docstore: &DocStore) -> Result<Self> {
        let conn = Connection::open(&docstore.sqlite_path())?;
        
        // Initialize tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_kv (
                layer TEXT NOT NULL,
                key TEXT NOT NULL,
                data BLOB NOT NULL,
                metadata TEXT,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (layer, key)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_medium (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                layer TEXT NOT NULL DEFAULT 'medium',
                key TEXT NOT NULL UNIQUE,
                content_type TEXT,
                data BLOB,
                metadata TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_kv_layer ON memory_kv(layer)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_medium_key ON memory_medium(key)",
            [],
        )?;

        debug!("SQLite store initialized: {}", docstore.sqlite_path().display());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            blobs_dir: docstore.blobs_dir(),
        })
    }

    fn layer_to_table(&self, layer: MemLayer) -> &'static str {
        match layer {
            MemLayer::Short => "memory_kv",
            MemLayer::Medium => "memory_medium", 
            MemLayer::Long => "memory_kv", // Long использует KV + файлы
            _ => "memory_kv",
        }
    }

    fn write_blob_file(&self, key: &str, data: &[u8]) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.blobs_dir)?;
        let file_path = self.blobs_dir.join(format!("{}.blob", key));
        std::fs::write(&file_path, data)?;
        Ok(file_path)
    }

    fn read_blob_file(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let file_path = self.blobs_dir.join(format!("{}.blob", key));
        if file_path.exists() {
            Ok(Some(std::fs::read(file_path)?))
        } else {
            Ok(None)
        }
    }

    fn delete_blob_file(&self, key: &str) -> Result<bool> {
        let file_path = self.blobs_dir.join(format!("{}.blob", key));
        if file_path.exists() {
            std::fs::remove_file(file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[async_trait]
impl crate::MemoryStoreAsync for SqliteStore {
    async fn put(&self, layer: MemLayer, key: &str, data: &[u8], metadata: Option<serde_json::Value>) -> Result<()> {
        let metadata_json = metadata.map(|m| m.to_string());
        let now = chrono::Utc::now().timestamp();

        match layer {
            MemLayer::Long => {
                // Store large data as blob file, reference in DB
                let _blob_path = self.write_blob_file(key, data)?;
                let conn = self.conn.lock().unwrap();
                conn.execute(
                    "INSERT OR REPLACE INTO memory_kv (layer, key, data, metadata, created_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params!["long", key, b"", metadata_json, now],
                )?;
            },
            
            MemLayer::Medium => {
                let conn = self.conn.lock().unwrap();
                conn.execute(
                    "INSERT OR REPLACE INTO memory_medium (key, data, metadata, created_at, updated_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![key, data, metadata_json, now, now],
                )?;
            },
            
            _ => {
                // Short term and others use KV table
                let layer_name = match layer {
                    MemLayer::Short => "short",
                    MemLayer::Semantic => "semantic",
                    _ => "other",
                };
                
                let conn = self.conn.lock().unwrap();
                conn.execute(
                    "INSERT OR REPLACE INTO memory_kv (layer, key, data, metadata, created_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![layer_name, key, data, metadata_json, now],
                )?;
            }
        }

        debug!("Stored in {:?}: key={}, data_size={}", layer, key, data.len());
        Ok(())
    }

    async fn get(&self, layer: MemLayer, key: &str) -> Result<Option<Vec<u8>>> {
        match layer {
            MemLayer::Long => {
                // Try to read from blob file first
                if let Some(data) = self.read_blob_file(key)? {
                    return Ok(Some(data));
                }
                // Fallback to DB
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT data FROM memory_kv WHERE layer = 'long' AND key = ?")?;
                let mut rows = stmt.query_map([key], |row| {
                    Ok(row.get::<_, Vec<u8>>(0)?)
                })?;
                
                if let Some(row) = rows.next() {
                    Ok(Some(row?))
                } else {
                    Ok(None)
                }
            },
            
            MemLayer::Medium => {
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT data FROM memory_medium WHERE key = ?")?;
                let mut rows = stmt.query_map([key], |row| {
                    Ok(row.get::<_, Vec<u8>>(0)?)
                })?;
                
                if let Some(row) = rows.next() {
                    Ok(Some(row?))
                } else {
                    Ok(None)
                }
            },
            
            _ => {
                let layer_name = match layer {
                    MemLayer::Short => "short",
                    MemLayer::Semantic => "semantic",
                    _ => "other",
                };
                
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT data FROM memory_kv WHERE layer = ? AND key = ?")?;
                let mut rows = stmt.query_map(params![layer_name, key], |row| {
                    Ok(row.get::<_, Vec<u8>>(0)?)
                })?;
                
                if let Some(row) = rows.next() {
                    Ok(Some(row?))
                } else {
                    Ok(None)
                }
            }
        }
    }

    async fn delete(&self, layer: MemLayer, key: &str) -> Result<bool> {
        let mut deleted = false;

        match layer {
            MemLayer::Long => {
                // Delete blob file if exists
                deleted |= self.delete_blob_file(key)?;
                
                // Delete DB record
                let conn = self.conn.lock().unwrap();
                let changes = conn.execute("DELETE FROM memory_kv WHERE layer = 'long' AND key = ?", [key])?;
                deleted |= changes > 0;
            },
            
            MemLayer::Medium => {
                let conn = self.conn.lock().unwrap();
                let changes = conn.execute("DELETE FROM memory_medium WHERE key = ?", [key])?;
                deleted = changes > 0;
            },
            
            _ => {
                let layer_name = match layer {
                    MemLayer::Short => "short",
                    MemLayer::Semantic => "semantic",
                    _ => "other",
                };
                
                let conn = self.conn.lock().unwrap();
                let changes = conn.execute("DELETE FROM memory_kv WHERE layer = ? AND key = ?", params![layer_name, key])?;
                deleted = changes > 0;
            }
        }

        debug!("Delete from {:?}: key={}, deleted={}", layer, key, deleted);
        Ok(deleted)
    }

    async fn list(&self, layer: MemLayer, prefix: Option<&str>) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        match layer {
            MemLayer::Long => {
                // List from blobs directory
                if self.blobs_dir.exists() {
                    for entry in std::fs::read_dir(&self.blobs_dir)? {
                        let entry = entry?;
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".blob") {
                                let key = name.strip_suffix(".blob").unwrap();
                                if let Some(p) = prefix {
                                    if key.starts_with(p) {
                                        keys.push(key.to_string());
                                    }
                                } else {
                                    keys.push(key.to_string());
                                }
                            }
                        }
                    }
                }
                
                // Also check DB
                let conn = self.conn.lock().unwrap();
                let query = if let Some(p) = prefix {
                    format!("SELECT key FROM memory_kv WHERE layer = 'long' AND key LIKE '{}%'", p)
                } else {
                    "SELECT key FROM memory_kv WHERE layer = 'long'".to_string()
                };
                
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map([], |row| {
                    Ok(row.get::<_, String>(0)?)
                })?;
                
                for row in rows {
                    let key = row?;
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            },
            
            MemLayer::Medium => {
                let conn = self.conn.lock().unwrap();
                let query = if let Some(p) = prefix {
                    format!("SELECT key FROM memory_medium WHERE key LIKE '{}%'", p)
                } else {
                    "SELECT key FROM memory_medium".to_string()
                };
                
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map([], |row| {
                    Ok(row.get::<_, String>(0)?)
                })?;
                
                for row in rows {
                    keys.push(row?);
                }
            },
            
            _ => {
                let layer_name = match layer {
                    MemLayer::Short => "short",
                    MemLayer::Semantic => "semantic",
                    _ => "other",
                };
                
                let conn = self.conn.lock().unwrap();
                let query = if let Some(p) = prefix {
                    format!("SELECT key FROM memory_kv WHERE layer = '{}' AND key LIKE '{}%'", layer_name, p)
                } else {
                    format!("SELECT key FROM memory_kv WHERE layer = '{}'", layer_name)
                };
                
                let mut stmt = conn.prepare(&query)?;
                let rows = stmt.query_map([], |row| {
                    Ok(row.get::<_, String>(0)?)
                })?;
                
                for row in rows {
                    keys.push(row?);
                }
            }
        }

        Ok(keys)
    }

    async fn exists(&self, layer: MemLayer, key: &str) -> Result<bool> {
        match layer {
            MemLayer::Long => {
                // Check blob file first
                let blob_path = self.blobs_dir.join(format!("{}.blob", key));
                if blob_path.exists() {
                    return Ok(true);
                }
                
                // Check DB
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT 1 FROM memory_kv WHERE layer = 'long' AND key = ?")?;
                Ok(stmt.exists([key])?)
            },
            
            MemLayer::Medium => {
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT 1 FROM memory_medium WHERE key = ?")?;
                Ok(stmt.exists([key])?)
            },
            
            _ => {
                let layer_name = match layer {
                    MemLayer::Short => "short",
                    MemLayer::Semantic => "semantic",
                    _ => "other",
                };
                
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT 1 FROM memory_kv WHERE layer = ? AND key = ?")?;
                Ok(stmt.exists(params![layer_name, key])?)
            }
        }
    }
}