use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

/// Абстракция над различными версиями ONNX Runtime
/// 
/// Позволяет использовать разные версии ORT без изменения основного кода
#[async_trait]
pub trait OrtBackend: Send + Sync {
    /// Название и версия backend'а
    fn name(&self) -> &str;
    
    /// Создать векторизатор из модели
    async fn create_vectorizer(&self, model_path: &Path) -> Result<Arc<dyn Vectorizer>>;
    
    /// Создать reranker из модели  
    async fn create_reranker(&self, model_path: &Path) -> Result<Arc<dyn Reranker>>;
}

/// Интерфейс для векторизации текста
#[async_trait]
pub trait Vectorizer: Send + Sync {
    /// Получить размерность эмбеддингов
    fn embedding_dim(&self) -> usize;
    
    /// Векторизовать список текстов
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    
    /// Очистить кэш
    async fn clear_cache(&self) -> Result<()>;
    
    /// Получить статистику кэша
    async fn cache_stats(&self) -> (usize, usize);
}

/// Интерфейс для ранжирования документов
#[async_trait]
pub trait Reranker: Send + Sync {
    /// Ранжировать документы относительно запроса
    async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>>;
}

/// Backend для ORT версии 1.16
pub struct OrtV1Backend;

impl OrtV1Backend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OrtBackend for OrtV1Backend {
    fn name(&self) -> &str {
        "ONNX Runtime v1.16"
    }
    
    async fn create_vectorizer(&self, model_path: &Path) -> Result<Arc<dyn Vectorizer>> {
        #[cfg(feature = "use_real_onnx")]
        {
            use crate::onnx_models::Qwen3EmbeddingModel;
            let model = Qwen3EmbeddingModel::new(model_path.to_path_buf()).await?;
            Ok(Arc::new(OrtV1Vectorizer(model)))
        }
        
        #[cfg(not(feature = "use_real_onnx"))]
        {
            use crate::onnx_models::Qwen3EmbeddingModel;
            let model = Qwen3EmbeddingModel::new(model_path.to_path_buf()).await?;
            Ok(Arc::new(SimplifiedVectorizer(model)))
        }
    }
    
    async fn create_reranker(&self, model_path: &Path) -> Result<Arc<dyn Reranker>> {
        #[cfg(feature = "use_real_onnx")]
        {
            use crate::onnx_models::Qwen3RerankerModel;
            let model = Qwen3RerankerModel::new(model_path.to_path_buf()).await?;
            Ok(Arc::new(OrtV1Reranker(model)))
        }
        
        #[cfg(not(feature = "use_real_onnx"))]
        {
            use crate::onnx_models::Qwen3RerankerModel;
            let model = Qwen3RerankerModel::new(model_path.to_path_buf()).await?;
            Ok(Arc::new(SimplifiedReranker(model)))
        }
    }
}

/// Обёртка для векторизатора ORT v1.16
#[cfg(feature = "use_real_onnx")]
struct OrtV1Vectorizer(crate::onnx_models::Qwen3EmbeddingModel);

#[cfg(feature = "use_real_onnx")]
#[async_trait]
impl Vectorizer for OrtV1Vectorizer {
    fn embedding_dim(&self) -> usize {
        self.0.embedding_dim()
    }
    
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.0.embed(texts).await
    }
    
    async fn clear_cache(&self) -> Result<()> {
        self.0.clear_cache().await
    }
    
    async fn cache_stats(&self) -> (usize, usize) {
        self.0.cache_stats().await
    }
}

/// Обёртка для reranker ORT v1.16
#[cfg(feature = "use_real_onnx")]
struct OrtV1Reranker(crate::onnx_models::Qwen3RerankerModel);

#[cfg(feature = "use_real_onnx")]
#[async_trait]
impl Reranker for OrtV1Reranker {
    async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        self.0.rerank(query, documents, top_k).await
    }
}

/// Обёртка для упрощённого векторизатора
#[cfg(not(feature = "use_real_onnx"))]
struct SimplifiedVectorizer(crate::onnx_models::Qwen3EmbeddingModel);

#[cfg(not(feature = "use_real_onnx"))]
#[async_trait]
impl Vectorizer for SimplifiedVectorizer {
    fn embedding_dim(&self) -> usize {
        self.0.embedding_dim()
    }
    
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.0.embed(texts).await
    }
    
    async fn clear_cache(&self) -> Result<()> {
        self.0.clear_cache().await
    }
    
    async fn cache_stats(&self) -> (usize, usize) {
        self.0.cache_stats().await
    }
}

/// Обёртка для упрощённого reranker
#[cfg(not(feature = "use_real_onnx"))]
struct SimplifiedReranker(crate::onnx_models::Qwen3RerankerModel);

#[cfg(not(feature = "use_real_onnx"))]
#[async_trait]
impl Reranker for SimplifiedReranker {
    async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        self.0.rerank(query, documents, top_k).await
    }
}

/// Mock backend для тестирования
pub struct MockBackend;

impl MockBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl OrtBackend for MockBackend {
    fn name(&self) -> &str {
        "Mock Backend (no ONNX)"
    }
    
    async fn create_vectorizer(&self, _model_path: &Path) -> Result<Arc<dyn Vectorizer>> {
        Ok(Arc::new(MockVectorizer::new()))
    }
    
    async fn create_reranker(&self, _model_path: &Path) -> Result<Arc<dyn Reranker>> {
        Ok(Arc::new(MockReranker::new()))
    }
}

/// Mock векторизатор для тестирования
struct MockVectorizer {
    embedding_dim: usize,
}

impl MockVectorizer {
    fn new() -> Self {
        Self { embedding_dim: 1024 }
    }
}

#[async_trait]
impl Vectorizer for MockVectorizer {
    fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }
    
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // Генерируем простые эмбеддинги на основе хеша текста
        Ok(texts.iter().map(|text| {
            let mut vec = vec![0.0f32; self.embedding_dim];
            let hash = blake3::hash(text.as_bytes());
            let bytes = hash.as_bytes();
            
            for (i, &byte) in bytes.iter().enumerate() {
                if i < self.embedding_dim {
                    vec[i] = (byte as f32 / 255.0) * 2.0 - 1.0;
                }
            }
            
            // Нормализация
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for val in &mut vec {
                    *val /= norm;
                }
            }
            
            vec
        }).collect())
    }
    
    async fn clear_cache(&self) -> Result<()> {
        Ok(())
    }
    
    async fn cache_stats(&self) -> (usize, usize) {
        (0, 0)
    }
}

/// Mock reranker для тестирования
struct MockReranker;

impl MockReranker {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Reranker for MockReranker {
    async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        // Простое ранжирование по совпадению слов
        let query_lower = query.to_lowercase();
        let query_words: std::collections::HashSet<_> = 
            query_lower.split_whitespace().collect();
        
        let mut scores: Vec<(usize, f32)> = documents.iter().enumerate().map(|(idx, doc)| {
            let doc_lower = doc.to_lowercase();
            let doc_words: std::collections::HashSet<_> = 
                doc_lower.split_whitespace().collect();
            
            let intersection = query_words.intersection(&doc_words).count();
            let union = query_words.union(&doc_words).count();
            
            let score = if union > 0 {
                intersection as f32 / union as f32
            } else {
                0.0
            };
            
            (idx, score)
        }).collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        
        Ok(scores)
    }
}

/// Фабрика для создания подходящего backend'а
pub struct BackendFactory;

impl BackendFactory {
    /// Создать backend на основе доступных возможностей
    pub async fn create_best_available() -> Arc<dyn OrtBackend> {
        // Проверяем переменную окружения
        if std::env::var("MAGRAY_USE_MOCK_MODELS").unwrap_or_default() == "true" {
            tracing::info!("Using mock backend (forced by environment variable)");
            return Arc::new(MockBackend::new());
        }
        
        // Пытаемся создать ORT v1.16 backend
        match Self::try_create_ort_v1().await {
            Ok(backend) => {
                tracing::info!("Using {}", backend.name());
                backend
            }
            Err(e) => {
                tracing::warn!("Failed to create ORT backend: {}, falling back to mock", e);
                Arc::new(MockBackend::new())
            }
        }
    }
    
    async fn try_create_ort_v1() -> Result<Arc<dyn OrtBackend>> {
        // Проверяем, что ORT библиотека доступна
        #[cfg(feature = "use_real_onnx")]
        {
            // Простая проверка через создание Session
            use ort::session::Session;
            // В ORT 2.0 Environment управляется автоматически
            // Проверяем доступность создав пустой builder
            let _ = Session::builder()
                .map_err(|e| anyhow::anyhow!("ORT not available: {}", e))?;
        }
        
        Ok(Arc::new(OrtV1Backend::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_backend() {
        let backend = MockBackend::new();
        assert_eq!(backend.name(), "Mock Backend (no ONNX)");
        
        let vectorizer = backend.create_vectorizer(Path::new("fake")).await.unwrap();
        assert_eq!(vectorizer.embedding_dim(), 1024);
        
        let texts = vec!["Hello".to_string(), "World".to_string()];
        let embeddings = vectorizer.embed(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 1024);
    }
    
    #[tokio::test]
    async fn test_mock_reranker() {
        let backend = MockBackend::new();
        let reranker = backend.create_reranker(Path::new("fake")).await.unwrap();
        
        let query = "hello world";
        let docs = vec![
            "hello world example".to_string(),
            "goodbye moon".to_string(),
            "world peace".to_string(),
        ];
        
        let results = reranker.rerank(query, &docs, 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0); // Первый документ наиболее релевантный
    }
}