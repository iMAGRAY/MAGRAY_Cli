use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    storage::VectorStore,
    types::{Layer, Record, SearchOptions},
    orchestration::{
        traits::{Coordinator, SearchCoordinator as SearchCoordinatorTrait, EmbeddingCoordinator as EmbeddingCoordinatorTrait},
        retry_handler::{RetryHandler, RetryPolicy},
        embedding_coordinator::EmbeddingCoordinator,
    },
};

/// Координатор поиска - основная логика поиска с retry и fallback
// @component: {"k":"C","id":"search_coordinator","t":"Search orchestration coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","search","coordinator"]}
pub struct SearchCoordinator {
    store: Arc<VectorStore>,
    embedding_coordinator: Arc<EmbeddingCoordinator>,
    retry_handler: RetryHandler,
    ready: std::sync::atomic::AtomicBool,
}

impl SearchCoordinator {
    pub fn new(
        store: Arc<VectorStore>,
        embedding_coordinator: Arc<EmbeddingCoordinator>,
    ) -> Self {
        Self {
            store,
            embedding_coordinator,
            retry_handler: RetryHandler::new(RetryPolicy::default()),
            ready: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl Coordinator for SearchCoordinator {
    async fn initialize(&self) -> Result<()> {
        info!("Инициализация SearchCoordinator");
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "ready": self.is_ready().await,
            "type": "search_coordinator"
        })
    }
}

#[async_trait]
impl SearchCoordinatorTrait for SearchCoordinator {
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!("Поиск в слое {:?}: '{}'", layer, query);
        
        // Получаем embedding через координатор
        let embedding = self.embedding_coordinator.get_embedding(query).await?;
        
        // Поиск с retry
        self.retry_handler
            .execute(|| async {
                self.store.search(&embedding, layer, options.top_k).await
            })
            .await
            .into_result()
    }
    
    async fn vector_search(
        &self,
        vector: &[f32],
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        self.retry_handler
            .execute(|| async {
                self.store.search(vector, layer, options.top_k).await
            })
            .await
            .into_result()
    }
    
    async fn hybrid_search(
        &self,
        query: &str,
        _vector: Option<&[f32]>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        // TODO: Реализовать гибридный поиск
        self.search(query, layer, options).await
    }
    
    async fn search_with_rerank(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
        _rerank_top_k: usize,
    ) -> Result<Vec<Record>> {
        // TODO: Реализовать reranking
        self.search(query, layer, options).await
    }
}