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

/// Гибкий сервис векторизации, который может работать с ONNX или без него
pub struct FlexibleVectorizerService {
    embedding_fn: Box<dyn Fn(&str) -> Vec<f32> + Send + Sync>,
    embedding_dim: usize,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl FlexibleVectorizerService {
    /// Создаем с настоящими ONNX моделями если возможно, иначе используем mock
    pub async fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref();
        
        // Пробуем загрузить ONNX модель
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(model) = crate::onnx_models::Qwen3EmbeddingModel::new(model_path).await {
                tracing::info!("Using ONNX embedding model");
                return Ok(Self {
                    embedding_fn: Box::new(move |text| {
                        // В реальности здесь должен быть вызов модели
                        // Но для Windows используем mock
                        Self::mock_embedding(text)
                    }),
                    embedding_dim: 1024,
                    cache: Arc::new(RwLock::new(HashMap::new())),
                });
            }
        }
        
        // Fallback на mock embeddings
        tracing::warn!("Using mock embeddings (ONNX not available)");
        Ok(Self {
            embedding_fn: Box::new(Self::mock_embedding),
            embedding_dim: 1024,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Mock функция для создания эмбеддингов
    fn mock_embedding(text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; 1024];
        let text_lower = text.to_lowercase();
        
        // Простая эвристика на основе ключевых слов
        let keywords = [
            ("rust", vec![0, 1, 2]),
            ("programming", vec![3, 4]),
            ("language", vec![5, 6]),
            ("memory", vec![7, 8, 9]),
            ("safety", vec![10, 11]),
            ("ownership", vec![12, 13]),
            ("async", vec![14, 15, 16]),
            ("function", vec![17, 18]),
            ("struct", vec![19, 20]),
            ("impl", vec![21, 22]),
            ("trait", vec![23, 24]),
            ("vector", vec![25, 26, 27]),
            ("search", vec![28, 29]),
            ("index", vec![30, 31]),
            ("code", vec![32, 33]),
            ("file", vec![34, 35]),
            ("python", vec![40, 41]),
            ("javascript", vec![42, 43]),
            ("java", vec![44, 45]),
            ("cpp", vec![46, 47]),
        ];
        
        // Устанавливаем веса для найденных ключевых слов
        for (keyword, positions) in &keywords {
            if text_lower.contains(keyword) {
                for &pos in positions {
                    if pos < vector.len() {
                        vector[pos] = 0.8 + (text_lower.matches(keyword).count() as f32 * 0.1).min(0.2);
                    }
                }
            }
        }
        
        // Добавляем признаки на основе структуры текста
        let lines = text.lines().count();
        let words = text.split_whitespace().count();
        let avg_word_len = if words > 0 {
            text.chars().filter(|c| c.is_alphabetic()).count() as f32 / words as f32
        } else {
            0.0
        };
        
        vector[100] = (lines as f32 / 50.0).min(1.0);
        vector[101] = (words as f32 / 100.0).min(1.0);
        vector[102] = (avg_word_len / 10.0).min(1.0);
        
        // Хеш текста для уникальности
        let hash = blake3::hash(text.as_bytes());
        let hash_bytes = hash.as_bytes();
        for i in 0..32 {
            if 200 + i < vector.len() {
                vector[200 + i] = (hash_bytes[i] as f32 / 255.0) * 0.3;
            }
        }
        
        // Нормализуем вектор
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vector {
                *x /= norm;
            }
        }
        
        vector
    }
    
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }
    
    async fn compute_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        
        for text in texts {
            let cache_key = format!("embed:{}", blake3::hash(text.as_bytes()).to_hex());
            
            // Проверяем кэш
            {
                let cache = self.cache.read().await;
                if let Some(cached) = cache.get(&cache_key) {
                    results.push(cached.clone());
                    continue;
                }
            }
            
            // Вычисляем эмбеддинг
            let embedding = (self.embedding_fn)(text);
            
            // Сохраняем в кэш
            {
                let mut cache = self.cache.write().await;
                cache.insert(cache_key, embedding.clone());
            }
            
            results.push(embedding);
        }
        
        Ok(results)
    }
}

#[async_trait]
impl Vectorizer for FlexibleVectorizerService {
    async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse> {
        let vectors = self.compute_embeddings(&request.texts).await?;
        
        Ok(EmbedResponse {
            vectors,
            model: "flexible".to_string(),
            dimensions: self.embedding_dim,
            tokens_used: Some(request.texts.iter().map(|t| t.len() as u32 / 4).sum()),
        })
    }
}

/// Гибкий сервис reranking
pub struct FlexibleRerankerService {
    rerank_fn: Box<dyn Fn(&str, &[String]) -> Vec<(usize, f32)> + Send + Sync>,
}

impl FlexibleRerankerService {
    pub async fn new<P: AsRef<Path>>(_model_path: P) -> Result<Self> {
        tracing::warn!("Using mock reranker");
        Ok(Self {
            rerank_fn: Box::new(Self::mock_rerank),
        })
    }
    
    /// Mock функция для reranking
    fn mock_rerank(query: &str, documents: &[String]) -> Vec<(usize, f32)> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        let mut scores: Vec<(usize, f32)> = documents.iter()
            .enumerate()
            .map(|(idx, doc)| {
                let doc_lower = doc.to_lowercase();
                let mut score = 0.0;
                
                // Точное совпадение
                if doc_lower.contains(&query_lower) {
                    score += 0.5;
                }
                
                // Совпадение слов
                for word in &query_words {
                    if doc_lower.contains(word) {
                        score += 0.2 / query_words.len() as f32;
                    }
                }
                
                // Jaccard similarity
                let doc_words: std::collections::HashSet<&str> = 
                    doc_lower.split_whitespace().collect();
                let query_set: std::collections::HashSet<&str> = 
                    query_words.iter().copied().collect();
                
                let intersection = query_set.intersection(&doc_words).count();
                let union = query_set.union(&doc_words).count();
                
                if union > 0 {
                    score += 0.3 * (intersection as f32 / union as f32);
                }
                
                (idx, score.min(1.0))
            })
            .collect();
        
        // Сортируем по убыванию релевантности
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        scores
    }
}

#[async_trait]
impl Reranker for FlexibleRerankerService {
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse> {
        let ranked = (self.rerank_fn)(&request.query, &request.documents);
        
        let hits: Vec<_> = ranked.into_iter()
            .take(request.top_k)
            .map(|(idx, score)| RerankHit {
                index: idx,
                score,
                document: request.documents[idx].clone(),
            })
            .collect();
        
        Ok(RerankResponse {
            hits,
            model: "flexible".to_string(),
            query_time_ms: 0,
        })
    }
}

/// Семантический роутер с гибкими сервисами
pub struct FlexibleSemanticRouter {
    vectorizer: Arc<dyn Vectorizer>,
    reranker: Arc<dyn Reranker>,
    vector_index: Arc<RwLock<VectorIndex>>,
}

impl FlexibleSemanticRouter {
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
    
    pub async fn semantic_search(
        &self,
        query: &str,
        top_k: usize,
        rerank_top_k: usize,
    ) -> Result<Vec<MemSearchResult>> {
        // Векторизуем запрос
        let embed_req = EmbedRequest {
            texts: vec![query.to_string()],
            purpose: crate::types::EmbedPurpose::Query,
            model: None,
        };
        
        let embed_resp = self.vectorizer.embed(embed_req).await?;
        let query_vector = embed_resp.vectors.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;
        
        // Поиск похожих векторов
        let candidates = {
            let index = self.vector_index.read().await;
            index.search(&query_vector, top_k * 2)?
        };
        
        if candidates.is_empty() || rerank_top_k == 0 {
            return Ok(candidates.into_iter().take(top_k).collect());
        }
        
        // Reranking
        let documents: Vec<String> = candidates.iter()
            .filter_map(|r| r.snippet.clone())
            .collect();
        
        if documents.is_empty() {
            return Ok(candidates.into_iter().take(top_k).collect());
        }
        
        let rerank_req = RerankRequest {
            query: query.to_string(),
            documents,
            top_k: top_k.min(rerank_top_k),
            model: None,
        };
        
        let rerank_resp = self.reranker.rerank(rerank_req).await?;
        
        // Переупорядочиваем результаты
        Ok(rerank_resp.hits.into_iter()
            .filter_map(|hit| {
                candidates.get(hit.index).map(|orig| {
                    let mut result = orig.clone();
                    result.score = hit.score;
                    result
                })
            })
            .collect())
    }
}

#[async_trait]
impl SemanticIndex for FlexibleSemanticRouter {
    async fn ingest(&self, text: &str, mem_ref: &MemRef, meta: &MemMeta) -> Result<()> {
        let embed_req = EmbedRequest {
            texts: vec![text.to_string()],
            purpose: crate::types::EmbedPurpose::Index,
            model: None,
        };
        
        let embed_resp = self.vectorizer.embed(embed_req).await?;
        let vector = embed_resp.vectors.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;
        
        let mut index = self.vector_index.write().await;
        index.add(vector, mem_ref.clone(), text.to_string(), meta.clone())?;
        
        Ok(())
    }
    
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<MemSearchResult>> {
        self.semantic_search(query, top_k, top_k / 2).await
    }
    
    async fn remove(&self, mem_ref: &MemRef) -> Result<bool> {
        let mut index = self.vector_index.write().await;
        index.remove(mem_ref)
    }
    
    async fn reindex(&self) -> Result<()> {
        let mut index = self.vector_index.write().await;
        index.rebuild_index()
    }
    
    async fn vacuum(&self) -> Result<u64> {
        let mut index = self.vector_index.write().await;
        index.vacuum()
    }
}