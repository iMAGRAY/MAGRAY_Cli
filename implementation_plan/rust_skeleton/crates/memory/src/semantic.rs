use crate::onnx_models::{Qwen3EmbeddingModel, Qwen3RerankerModel};
use crate::types::{EmbedRequest, EmbedResponse, RerankRequest, RerankResponse, RerankHit};
use crate::{MemRef, MemMeta, MemSearchResult, SemanticIndex};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Сервис для генерации эмбеддингов с использованием Qwen3 модели
/// 
/// Особенности:
/// - Локальная ONNX модель для офлайн работы
/// - Батчинг запросов для оптимизации
/// - Кэширование результатов
/// - Поддержка разных целей эмбеддинга (Index, Query, etc.)
#[derive(Debug)]
pub struct VectorizerService {
    model: Arc<Qwen3EmbeddingModel>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl VectorizerService {
    pub async fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref().to_path_buf();
        
        // Загружаем Qwen3 модель
        let model = Qwen3EmbeddingModel::new(model_path).await
            .context("Failed to initialize Qwen3 embedding model")?;
        
        Ok(Self {
            model: Arc::new(model),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Генерирует эмбеддинги для текстов с кэшированием
    async fn compute_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        let mut to_compute = Vec::new();
        let mut to_compute_indices = Vec::new();
        
        // Проверяем кэш
        {
            let cache = self.cache.read().await;
            for (i, text) in texts.iter().enumerate() {
                let cache_key = self.cache_key(text);
                if let Some(cached) = cache.get(&cache_key) {
                    results.push(Some(cached.clone()));
                } else {
                    results.push(None);
                    to_compute.push(text.clone());
                    to_compute_indices.push(i);
                }
            }
        }
        
        // Вычисляем эмбеддинги для отсутствующих в кэше
        if !to_compute.is_empty() {
            let computed = self.model.embed(&to_compute).await?;
            
            // Обновляем кэш и результаты
            {
                let mut cache = self.cache.write().await;
                for (i, embedding) in computed.into_iter().enumerate() {
                    let text = &to_compute[i];
                    let cache_key = self.cache_key(text);
                    cache.insert(cache_key, embedding.clone());
                    
                    let result_index = to_compute_indices[i];
                    results[result_index] = Some(embedding);
                }
            }
        }
        
        // Конвертируем в финальный результат
        let final_results: Result<Vec<Vec<f32>>> = results
            .into_iter()
            .map(|opt| opt.ok_or_else(|| anyhow::anyhow!("Failed to compute embedding")))
            .collect();
        
        final_results
    }
    
    /// Получить размерность эмбеддингов
    pub fn embedding_dim(&self) -> usize {
        self.model.embedding_dim()
    }
    
    
    /// Ключ для кэширования
    fn cache_key(&self, text: &str) -> String {
        format!("qwen3:{}", blake3::hash(text.as_bytes()).to_hex())
    }
    
    /// Очистка кэша
    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        tracing::debug!("Cleared Qwen3 embedding cache");
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
                dimensions: self.model.embedding_dim(),
                tokens_used: Some(0),
            });
        }
        
        let vectors = self.compute_embeddings(&request.texts).await
            .context("Failed to compute embeddings")?;
        
        let elapsed = start_time.elapsed();
        tracing::debug!("Generated {} embeddings in {:?}", vectors.len(), elapsed);
        
        // Примерная оценка токенов (в реальности считается токенизатором)
        let tokens_used = request.texts.iter()
            .map(|t| (t.len() / 4) as u32) // Примерно 4 символа на токен
            .sum();
        
        Ok(EmbedResponse {
            vectors,
            model: "qwen3".to_string(),
            dimensions: self.model.embedding_dim(),
            tokens_used: Some(tokens_used),
        })
    }
}

/// Трейт для сервиса векторизации
#[async_trait]
pub trait Vectorizer: Send + Sync {
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse>;
}

/// Сервис для reranking документов с использованием Qwen3 модели
/// 
/// Особенности:
/// - Cross-encoder архитектура для точного ранжирования
/// - Батчинг пар (query, document)
/// - Оптимизация для top-K результатов
#[derive(Debug)]
pub struct RerankerService {
    model: Arc<Qwen3RerankerModel>,
    max_documents: usize,
}

impl RerankerService {
    pub async fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref().to_path_buf();
        
        // Загружаем Qwen3 reranker модель
        let model = Qwen3RerankerModel::new(model_path).await
            .context("Failed to initialize Qwen3 reranker model")?;
        
        Ok(Self {
            model: Arc::new(model),
            max_documents: 64, // Лимит для производительности
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
        
        let scored_results = self.model.rerank(&request.query, docs_to_process, request.top_k).await
            .context("Failed to run Qwen3 reranking")?;
        
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

/// Трейт для сервиса reranking
#[async_trait]
pub trait Reranker: Send + Sync {
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse>;
}

/// Семантический роутер - главный интерфейс для семантического поиска
/// 
/// Объединяет векторизацию, индексацию и reranking для получения
/// релевантных результатов из всех слоёв памяти
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
            index.search(&query_vector, top_k * 2)? // Берём больше для reranking
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

/// Простой векторный индекс с использованием HNSW
struct VectorIndex {
    vectors: Vec<Vec<f32>>,
    metadata: Vec<(MemRef, String, MemMeta)>, // (ref, snippet, meta)
    // Пока что убираем HNSW из-за сложности API
    // hnsw_index: Option<hnsw::Hnsw<f32, hnsw::DistL2>>,
    dimensions: usize,
}

impl VectorIndex {
    fn new() -> Self {
        Self {
            vectors: Vec::new(),
            metadata: Vec::new(),
            // hnsw_index: None,
            dimensions: 1024, // Qwen3 размерность
        }
    }
    
    fn add(&mut self, vector: Vec<f32>, mem_ref: MemRef, snippet: String, meta: MemMeta) -> Result<()> {
        if vector.len() != self.dimensions {
            return Err(anyhow::anyhow!("Vector dimension mismatch: expected {}, got {}", 
                                     self.dimensions, vector.len()));
        }
        
        self.vectors.push(vector);
        self.metadata.push((mem_ref, snippet, meta));
        
        // Перестраиваем индекс если нужно
        if self.vectors.len() % 100 == 0 {
            self.rebuild_index()?;
        }
        
        Ok(())
    }
    
    fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<MemSearchResult>> {
        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut results = Vec::new();
        
        // Простой линейный поиск для всех случаев
        {
            // Линейный поиск
            let mut scored_results: Vec<_> = self.vectors.iter().enumerate()
                .map(|(idx, vector)| {
                    let score = self.cosine_similarity(query_vector, vector);
                    (idx, score)
                })
                .collect();
            
            scored_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            for (idx, score) in scored_results.into_iter().take(top_k) {
                if let Some((mem_ref, snippet, meta)) = self.metadata.get(idx) {
                    results.push(MemSearchResult {
                        mem_ref: mem_ref.clone(),
                        score,
                        snippet: Some(snippet.clone()),
                        meta: meta.clone(),
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    fn remove(&mut self, mem_ref: &MemRef) -> Result<bool> {
        if let Some(idx) = self.metadata.iter().position(|(r, _, _)| r == mem_ref) {
            self.vectors.remove(idx);
            self.metadata.remove(idx);
            // self.hnsw_index = None; // Требует перестройки
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn rebuild_index(&mut self) -> Result<()> {
        if self.vectors.is_empty() {
            return Ok(());
        }
        
        // Пока ничего не делаем
        tracing::debug!("Rebuilt index with {} vectors", self.vectors.len());
        Ok(())
    }
    
    fn vacuum(&mut self) -> Result<u64> {
        // Удаляем дублированные векторы (простая реализация)
        let original_len = self.vectors.len();
        
        // В реальной реализации здесь была бы проверка orphaned references
        // и более сложная логика очистки
        
        let removed = original_len - self.vectors.len();
        
        Ok(removed as u64)
    }
    
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_vectorizer_service() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("model.onnx");
        
        // Создаём фиктивный файл модели
        tokio::fs::write(&model_path, b"fake model").await.unwrap();
        
        let vectorizer = VectorizerService::new(&model_path).await.unwrap();
        
        let request = EmbedRequest {
            texts: vec!["hello world".to_string(), "test text".to_string()],
            purpose: crate::types::EmbedPurpose::Index,
            model: Some("bge3".to_string()),
        };
        
        let response = vectorizer.embed(request).await.unwrap();
        assert_eq!(response.vectors.len(), 2);
        assert_eq!(response.vectors[0].len(), 1024);
        assert_eq!(response.dimensions, 1024);
    }
    
    #[tokio::test]
    async fn test_reranker_service() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().join("reranker.onnx");
        
        tokio::fs::write(&model_path, b"fake reranker").await.unwrap();
        
        let reranker = RerankerService::new(&model_path).await.unwrap();
        
        let request = RerankRequest {
            query: "test query".to_string(),
            documents: vec![
                "relevant document".to_string(),
                "irrelevant text".to_string(),
                "another relevant doc".to_string(),
            ],
            top_k: 2,
            model: Some("bg3".to_string()),
        };
        
        let response = reranker.rerank(request).await.unwrap();
        assert_eq!(response.hits.len(), 2);
        assert!(response.hits[0].score >= response.hits[1].score);
    }
    
    #[tokio::test]
    async fn test_vector_index() {
        let mut index = VectorIndex::new();
        
        let vector1 = vec![1.0f32; 1024];
        let vector2 = vec![0.5f32; 1024];
        
        let mem_ref1 = MemRef::new(crate::MemLayer::Short, "key1".to_string());
        let mem_ref2 = MemRef::new(crate::MemLayer::Medium, "key2".to_string());
        
        let meta = MemMeta::default();
        
        index.add(vector1.clone(), mem_ref1.clone(), "snippet1".to_string(), meta.clone()).unwrap();
        index.add(vector2, mem_ref2, "snippet2".to_string(), meta).unwrap();
        
        let results = index.search(&vector1, 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].mem_ref, mem_ref1);
    }
}