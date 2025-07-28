use anyhow::Result;
use async_trait::async_trait;
use magray_core::{DocStore, MemLayer, MemRef, TodoItem, TaskState};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use tracing::{info, debug, error};

pub mod todo;
pub mod store;

// Re-export main types
pub use todo::TodoService;
pub use store::{MemoryStore, SqliteStore};

// === Memory Store Trait ===

#[async_trait]
pub trait MemoryStoreAsync: Send + Sync {
    async fn put(&self, layer: MemLayer, key: &str, data: &[u8], metadata: Option<serde_json::Value>) -> Result<()>;
    async fn get(&self, layer: MemLayer, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, layer: MemLayer, key: &str) -> Result<bool>;
    async fn list(&self, layer: MemLayer, prefix: Option<&str>) -> Result<Vec<String>>;
    async fn exists(&self, layer: MemLayer, key: &str) -> Result<bool>;
}

// === Memory Coordinator ===

pub struct MemoryCoordinator {
    store: Arc<dyn MemoryStoreAsync>,
    ephemeral: Arc<Mutex<HashMap<String, Vec<u8>>>>, // M0 - RAM cache
}

impl MemoryCoordinator {
    pub fn new(store: Arc<dyn MemoryStoreAsync>) -> Self {
        Self {
            store,
            ephemeral: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn put(&self, layer: MemLayer, key: &str, data: &[u8], metadata: Option<serde_json::Value>) -> Result<MemRef> {
        let mut mem_ref = MemRef::new(layer.clone(), key.to_string());
        
        // Сначала обрабатываем metadata
        if let Some(ref meta) = metadata {
            if let Some(obj) = meta.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        mem_ref.metadata.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        match layer {
            MemLayer::Ephemeral => {
                let mut cache = self.ephemeral.lock().unwrap();
                cache.insert(key.to_string(), data.to_vec());
                debug!("Stored in M0 (Ephemeral): key={}, size={}", key, data.len());
            },
            _ => {
                self.store.put(layer.clone(), key, data, metadata).await?;
                debug!("Stored in {:?}: key={}, size={}", layer, key, data.len());
            }
        }

        Ok(mem_ref)
    }

    pub async fn get(&self, mem_ref: &MemRef) -> Result<Option<Vec<u8>>> {
        match mem_ref.layer {
            MemLayer::Ephemeral => {
                let cache = self.ephemeral.lock().unwrap();
                Ok(cache.get(&mem_ref.key).cloned())
            },
            _ => {
                self.store.get(mem_ref.layer.clone(), &mem_ref.key).await
            }
        }
    }

    pub async fn delete(&self, mem_ref: &MemRef) -> Result<bool> {
        match mem_ref.layer {
            MemLayer::Ephemeral => {
                let mut cache = self.ephemeral.lock().unwrap();
                Ok(cache.remove(&mem_ref.key).is_some())
            },
            _ => {
                self.store.delete(mem_ref.layer.clone(), &mem_ref.key).await
            }
        }
    }

    // #INCOMPLETE: Семантический поиск через M4 будет добавлен позже
    pub async fn semantic_search(&self, _query: &str, _top_k: usize) -> Result<Vec<MemRef>> {
        info!("Semantic search requested but not yet implemented");
        Ok(Vec::new())
    }

    // #INCOMPLETE: Promotion между слоями памяти
    pub async fn promote(&self, _from_ref: &MemRef, _to_layer: MemLayer) -> Result<MemRef> {
        todo!("Memory promotion between layers")
    }
}
