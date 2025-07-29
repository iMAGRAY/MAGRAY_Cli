use anyhow::Result;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Упрощенная mock реализация Qwen3 модели для эмбеддингов
/// 
/// Эта реализация используется как временная заглушка пока
/// не будет исправлена интеграция с ONNX Runtime
#[derive(Debug)]
pub struct Qwen3EmbeddingModel {
    embedding_dim: usize,
    max_length: usize,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl Qwen3EmbeddingModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        info!("Loading Qwen3 Embedding model from: {} (using simplified implementation)", model_path.display());
        
        // Проверяем что директория существует
        if !model_path.exists() {
            warn!("Model directory not found: {}, using mock implementation", model_path.display());
        }

        Ok(Self {
            embedding_dim: 1024,
            max_length: 512,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Генерирует эмбеддинги для списка текстов
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(texts.len());

        for text in texts {
            let embedding = self.compute_single_embedding(text).await?;
            results.push(embedding);
        }

        Ok(results)
    }

    async fn compute_single_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Проверяем кэш
        let cache_key = self.cache_key(text);
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Генерируем псевдо-эмбеддинг на основе текста
        let mut embedding = vec![0.0f32; self.embedding_dim];
        
        // Используем хеш текста для генерации детерминированного вектора
        let hash = blake3::hash(text.as_bytes());
        let hash_bytes = hash.as_bytes();
        
        for (i, chunk) in hash_bytes.chunks(4).enumerate() {
            if i >= embedding.len() / 32 { break; }
            let val = u32::from_le_bytes([
                chunk.get(0).copied().unwrap_or(0),
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
                chunk.get(3).copied().unwrap_or(0),
            ]);
            
            // Распределяем значения по вектору
            for j in 0..32 {
                let idx = i * 32 + j;
                if idx < embedding.len() {
                    embedding[idx] = ((val >> j) & 1) as f32 * 0.1 - 0.05;
                }
            }
        }

        // Добавляем текстовые признаки
        let text_len = text.len() as f32;
        let word_count = text.split_whitespace().count() as f32;
        
        embedding[0] = (text_len / 1000.0).tanh();
        embedding[1] = (word_count / 100.0).tanh();
        embedding[2] = (text.chars().filter(|c| c.is_uppercase()).count() as f32 / text_len).tanh();
        embedding[3] = (text.chars().filter(|c| c.is_numeric()).count() as f32 / text_len).tanh();

        // L2 нормализация
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        // Кэшируем результат
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, embedding.clone());
        }

        debug!("Generated embedding for text: {:.50}...", text);
        Ok(embedding)
    }

    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    fn cache_key(&self, text: &str) -> String {
        format!("qwen3:{}", blake3::hash(text.as_bytes()).to_hex())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Cleared Qwen3 embedding cache");
        Ok(())
    }

    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let entries = cache.len();
        let size_bytes = cache.iter()
            .map(|(k, v)| k.len() + v.len() * std::mem::size_of::<f32>())
            .sum();
        (entries, size_bytes)
    }
}

/// Упрощенная mock реализация Qwen3 модели для reranking
#[derive(Debug)]
pub struct Qwen3RerankerModel {
    cache: Arc<RwLock<HashMap<String, f32>>>,
}

impl Qwen3RerankerModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        info!("Loading Qwen3 Reranker model from: {} (using simplified implementation)", model_path.display());
        
        if !model_path.exists() {
            warn!("Model directory not found: {}, using mock implementation", model_path.display());
        }

        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Ранжирует документы относительно запроса
    pub async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_docs = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            let score = self.compute_relevance_score(query, doc).await?;
            scored_docs.push((idx, score));
        }

        // Сортируем по убыванию скора
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Возвращаем top-k результатов
        scored_docs.truncate(top_k);
        
        debug!("Reranked {} documents, returning top {}", documents.len(), scored_docs.len());
        Ok(scored_docs)
    }

    async fn compute_relevance_score(&self, query: &str, document: &str) -> Result<f32> {
        // Проверяем кэш
        let cache_key = format!("{}:{}", 
            blake3::hash(query.as_bytes()).to_hex(),
            blake3::hash(document.as_bytes()).to_hex()
        );
        
        {
            let cache = self.cache.read().await;
            if let Some(&score) = cache.get(&cache_key) {
                return Ok(score);
            }
        }

        // Простая эвристика для оценки релевантности
        let query_lower = query.to_lowercase();
        let doc_lower = document.to_lowercase();
        
        let mut score = 0.0f32;
        
        // Точное совпадение
        if doc_lower.contains(&query_lower) {
            score += 0.5;
        }
        
        // Совпадение слов
        let query_words: std::collections::HashSet<_> = query_lower.split_whitespace().collect();
        let doc_words: std::collections::HashSet<_> = doc_lower.split_whitespace().collect();
        
        let intersection = query_words.intersection(&doc_words).count();
        let union = query_words.union(&doc_words).count();
        
        if union > 0 {
            score += 0.3 * (intersection as f32 / union as f32);
        }
        
        // Совпадение подстрок
        for word in &query_words {
            if word.len() >= 3 && doc_lower.contains(word) {
                score += 0.1;
            }
        }
        
        // Нормализуем оценку
        score = score.min(1.0);
        
        // Кэшируем результат
        {
            let mut cache = self.cache.write().await;
            cache.insert(cache_key, score);
        }
        
        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_embedding_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().to_path_buf();
        
        let model = Qwen3EmbeddingModel::new(model_path).await.unwrap();
        
        let texts = vec![
            "Hello world".to_string(),
            "Test embedding".to_string(),
        ];
        
        let embeddings = model.embed(&texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 1024);
        
        // Проверяем что векторы нормализованы
        let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_reranker_model() {
        let temp_dir = TempDir::new().unwrap();
        let model_path = temp_dir.path().to_path_buf();
        
        let model = Qwen3RerankerModel::new(model_path).await.unwrap();
        
        let query = "vector database";
        let documents = vec![
            "This is about vector database systems".to_string(),
            "Random text about cats".to_string(),
            "Database vectors and embeddings".to_string(),
        ];
        
        let results = model.rerank(query, &documents, 2).await.unwrap();
        
        assert_eq!(results.len(), 2);
        // Первый документ должен быть наиболее релевантным
        assert!(results[0].1 > results[1].1);
    }
}