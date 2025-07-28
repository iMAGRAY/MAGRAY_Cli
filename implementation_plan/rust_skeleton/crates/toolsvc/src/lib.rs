use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;

pub struct ToolSpec { pub id: String, pub name: String, pub desc: String }

#[async_trait]
pub trait Tool {
    fn spec(&self) -> ToolSpec;
    async fn invoke(&self, input: Value) -> Result<Value>;
}
