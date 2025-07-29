use crate::onnx_models::{Qwen3EmbeddingModel, Qwen3RerankerModel};
use crate::mock_models::{MockEmbeddingModel, MockRerankerModel};
use crate::types::{EmbedRequest, EmbedResponse, RerankRequest, RerankResponse, RerankHit};
use crate::{MemRef, MemMeta, MemSearchResult, SemanticIndex, VectorIndex};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Трейт для сервиса векторизации
#[async_trait]
pub trait Vectorizer: Send + Sync {
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse>;
}

/// Трейт для сервиса reranking
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse>;
}

/// Enum для выбора между реальной и mock моделью эмбеддингов
enum EmbeddingModelImpl {
    Real(Arc<Qwen3EmbeddingModel>),
    Mock(Arc<MockEmbeddingModel>),
}

/// Сервис для генерации эмбеддингов с fallback на mock
pub struct VectorizerService {
    model: EmbeddingModelImpl,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl VectorizerService {
    pub async fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref().to_path_buf();
        
        // Пытаемся загрузить модели в порядке приоритета: ONNX -> Mock
        let model = if let Ok(onnx_model) = Qwen3EmbeddingModel::new(model_path.clone()).await {
            tracing::info!("Loaded Qwen3 embedding model from ONNX");
            EmbeddingModelImpl::Real(Arc::new(onnx_model))
        } else {
            tracing::warn!("Failed to load real models, using mock model instead");
            let mock = MockEmbeddingModel::new(model_path).await?;
            EmbeddingModelImpl::Mock(Arc::new(mock))
        };
        
        Ok(Self {
            model,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Генерирует эмбеддинги для текстов с кэшированием
    async fn compute_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match &self.model {
            EmbeddingModelImpl::Real(model) => model.embed(texts).await,
            EmbeddingModelImpl::Mock(model) => model.embed(texts).await,
        }
    }
    
    /// Получить размерность эмбеддингов
    pub fn embedding_dim(&self) -> usize {
        match &self.model {
            EmbeddingModelImpl::Real(model) => model.embedding_dim(),
            EmbeddingModelImpl::Mock(model) => model.embedding_dim(),
        }
    }
    
    /// Ключ для кэширования
    fn cache_key(&self, text: &str) -> String {
        format!("embed:{}", blake3::hash(text.as_bytes()).to_hex())
    }
    
    /// Очистка кэша
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::debug!("Cleared embedding cache");
        Ok(())
    }
    
    /// Статистика кэша
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let entries = cache.len();
        let size_bytes = cache.iter()
            .map(|(k, v)| k.len() + v.len() * std::mem::size_of::<f32>())
            .sum();
        (entries, size_bytes)
    }
}

#[async_trait]
impl Vectorizer for VectorizerService {
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse> {
        let start_time = std::time::Instant::now();
        
        if request.texts.is_empty() {
            return Ok(EmbedResponse {
                vectors: Vec::new(),
                model: "qwen3".to_string(),
                dimensions: self.embedding_dim(),
                tokens_used: Some(0),
            });
        }
        
        let vectors = self.compute_embeddings(&request.texts).await
            .context("Failed to compute embeddings")?;
        
        let elapsed = start_time.elapsed();
        tracing::debug!("Generated {} embeddings in {:?}", vectors.len(), elapsed);
        
        // Примерная оценка токенов
        let tokens_used = request.texts.iter()
            .map(|t| (t.len() / 4) as u32)
            .sum();
        
        Ok(EmbedResponse {
            vectors,
            model: "qwen3".to_string(),
            dimensions: self.embedding_dim(),
            tokens_used: Some(tokens_used),
        })
    }
}

/// Enum для выбора между реальной и mock моделью reranker
enum RerankerModelImpl {
    Real(Arc<Qwen3RerankerModel>),
    Mock(Arc<MockRerankerModel>),
}

/// Сервис для reranking документов с fallback на mock
pub struct RerankerService {
    model: RerankerModelImpl,
    max_documents: usize,
}

impl RerankerService {
    pub async fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref().to_path_buf();
        
        // Пытаемся загрузить модели в порядке приоритета: ONNX -> Mock
        let model = if let Ok(onnx_model) = Qwen3RerankerModel::new(model_path.clone()).await {
            tracing::info!("Loaded Qwen3 reranker model from ONNX");
            RerankerModelImpl::Real(Arc::new(onnx_model))
        } else {
            tracing::warn!("Failed to load real reranker models, using mock model instead");
            let mock = MockRerankerModel::new(model_path).await?;
            RerankerModelImpl::Mock(Arc::new(mock))
        };
        
        Ok(Self {
            model,
            max_documents: 64,
        })
    }
}

#[async_trait]
impl Reranker for RerankerService {
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse> {
        let start_time = std::time::Instant::now();
        
        if request.documents.is_empty() {
            return Ok(RerankResponse {
                hits: Vec::new(),
                model: "qwen3".to_string(),
                query_time_ms: 0,
            });
        }
        
        // Ограничиваем количество документов
        let docs_to_process = if request.documents.len() > self.max_documents {
            tracing::warn!("Limiting documents from {} to {} for reranking", 
                         request.documents.len(), self.max_documents);
            &request.documents[..self.max_documents]
        } else {
            &request.documents
        };
        
        let scored_results = match &self.model {
            RerankerModelImpl::Real(model) => {
                model.rerank(&request.query, docs_to_process, request.top_k).await?
            }
            RerankerModelImpl::Mock(model) => {
                model.rerank(&request.query, docs_to_process, request.top_k).await?
            }
        };
        
        // Конвертируем результаты в формат RerankHit
        let hits: Vec<_> = scored_results.into_iter()
            .map(|(idx, score)| RerankHit {
                index: idx,
                score,
                document: docs_to_process[idx].clone(),
            })
            .collect();
        
        let elapsed = start_time.elapsed();
        tracing::debug!("Reranked {} documents in {:?}", docs_to_process.len(), elapsed);
        
        Ok(RerankResponse {
            hits,
            model: "qwen3".to_string(),
            query_time_ms: elapsed.as_millis() as u64,
        })
    }
}

/// Семантический роутер - главный интерфейс для семантического поиска
pub struct SemanticRouter {
    vectorizer: Arc<dyn Vectorizer>,
    reranker: Arc<dyn Reranker>,
    vector_index: Arc<RwLock<VectorIndex>>,
}

impl SemanticRouter {
    pub fn new(
        vectorizer: Arc<dyn Vectorizer>,
        reranker: Arc<dyn Reranker>,
    ) -> Self {
        Self {
            vectorizer,
            reranker,
            vector_index: Arc::new(RwLock::new(VectorIndex::new())),
        }
    }
    
    /// Полный цикл семантического поиска с reranking
    pub async fn semantic_search(&self, query: &str, top_k: usize, rerank_top_k: usize) -> Result<Vec<MemSearchResult>> {
        // 1. Векторизуем запрос
        let embed_req = EmbedRequest {
            texts: vec![query.to_string()],
            purpose: crate::types::EmbedPurpose::Query,
            model: Some("qwen3".to_string()),
        };
        
        let embed_resp = self.vectorizer.embed(embed_req).await?;
        let query_vector = embed_resp.vectors.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated for query"))?;
        
        // 2. Поиск похожих векторов
        let candidates = {
            let index = self.vector_index.read().await;
            index.search(&query_vector, top_k * 2)?
        };
        
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        
        // 3. Reranking если нужен
        let final_results = if rerank_top_k > 0 && candidates.len() > rerank_top_k {
            let documents: Vec<String> = candidates.iter()
                .filter_map(|r| r.snippet.clone())
                .collect();
            
            if !documents.is_empty() {
                let rerank_req = RerankRequest {
                    query: query.to_string(),
                    documents,
                    top_k: top_k.min(rerank_top_k),
                    model: Some("qwen3".to_string()),
                };
                
                let rerank_resp = self.reranker.rerank(rerank_req).await?;
                
                // Переупорядочиваем результаты согласно reranking
                rerank_resp.hits.into_iter()
                    .filter_map(|hit| {
                        candidates.get(hit.index).map(|orig| {
                            let mut result = orig.clone();
                            result.score = hit.score;
                            result
                        })
                    })
                    .collect()
            } else {
                candidates.into_iter().take(top_k).collect()
            }
        } else {
            candidates.into_iter().take(top_k).collect()
        };
        
        Ok(final_results)
    }
}

#[async_trait]
impl SemanticIndex for SemanticRouter {
    async fn ingest(&self, text: &str, mem_ref: &MemRef, meta: &MemMeta) -> Result<()> {
        // Генерируем эмбеддинг
        let embed_req = EmbedRequest {
            texts: vec![text.to_string()],
            purpose: crate::types::EmbedPurpose::Index,
            model: Some("qwen3".to_string()),
        };
        
        let embed_resp = self.vectorizer.embed(embed_req).await?;
        let vector = embed_resp.vectors.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;
        
        // Добавляем в индекс
        let mut index = self.vector_index.write().await;
        index.add(vector, mem_ref.clone(), text.to_string(), meta.clone())?;
        
        tracing::trace!("Ingested text into semantic index: {}", mem_ref.key);
        Ok(())
    }
    
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemSearchResult>> {
        self.semantic_search(query, top_k, top_k / 2).await
    }
    
    async fn remove(&self, mem_ref: &MemRef) -> Result<bool> {
        let mut index = self.vector_index.write().await;
        let removed = index.remove(mem_ref)?;
        tracing::trace!("Removed from semantic index: {}", mem_ref.key);
        Ok(removed)
    }
    
    async fn reindex(&self) -> Result<()> {
        let mut index = self.vector_index.write().await;
        index.rebuild_index()?;
        tracing::info!("Rebuilt semantic index");
        Ok(())
    }
    
    async fn vacuum(&self) -> Result<u64> {
        let mut index = self.vector_index.write().await;
        let removed = index.vacuum()?;
        tracing::info!("Vacuumed {} orphaned vectors", removed);
        Ok(removed)
    }
}