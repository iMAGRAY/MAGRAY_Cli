use async_trait::async_trait;
use anyhow::Result;

#[derive(Clone, Copy, Debug)]
pub enum MemLayer { Ephemeral, Short, Medium, Long, Semantic }

#[derive(Clone, Debug)]
pub struct MemRef { pub layer: MemLayer, pub key: String }

#[async_trait]
pub trait MemoryStore {
    async fn put(&self, layer: MemLayer, key: &str, data: &[u8]) -> Result<()>;
    async fn get(&self, layer: MemLayer, key: &str) -> Result<Option<Vec<u8>>>;
    async fn delete(&self, layer: MemLayer, key: &str) -> Result<()>;
}
