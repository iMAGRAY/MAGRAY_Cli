use async_trait::async_trait;
use anyhow::Result;

pub struct Prompt(String);
pub struct LLMOutput { pub text: String }

#[async_trait]
pub trait LLMClient {
    async fn complete(&self, prompt: &Prompt) -> Result<LLMOutput>;
}
