use async_trait::async_trait;
use anyhow::Result;

pub struct EmbedRequest { pub texts: Vec<String>, pub purpose: Purpose }
pub enum Purpose { Index, Query, ToolSpec }
pub struct EmbedResponse { pub vectors: Vec<Vec<f32>>, pub model: String }

#[async_trait]
pub trait Vectorizer {
    async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
}
