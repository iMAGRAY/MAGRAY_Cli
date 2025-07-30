use anyhow::Result;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Fallback embedding service для случаев когда AI модели недоступны
pub struct FallbackEmbeddingService {
    dimension: usize,
    cache: HashMap<String, Vec<f32>>,
}

impl FallbackEmbeddingService {
    pub fn new(dimension: usize) -> Self {
        warn!("🔄 Инициализация fallback embedding service (dimension: {})", dimension);
        warn!("⚠️ Используется упрощенная эмуляция embeddings - не для продакшена!");
        
        Self {
            dimension,
            cache: HashMap::new(),
        }
    }
    
    /// Генерация детерминистического "embedding" на основе hash текста
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // Проверяем кэш
        if let Some(cached) = self.cache.get(text) {
            return Ok(cached.clone());
        }
        
        // Создаем детерминистический hash
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();
        
        // Преобразуем hash в floating point vector
        let mut embedding = Vec::with_capacity(self.dimension);
        
        // Используем bytes hash для генерации псевдо-embedding
        for i in 0..self.dimension {
            let byte_idx = i % hash.len();
            let hash_byte = hash[byte_idx] as f32;
            
            // Нормализуем к диапазону [-1, 1] и добавляем вариации
            let mut value = (hash_byte - 127.5) / 127.5;
            
            // Добавляем вариации на основе позиции в тексте для большей дифференциации
            let position_factor = (i as f32 / self.dimension as f32) * 0.3;
            let text_length_factor = (text.len() as f32 / 100.0) * 0.2;
            
            value += position_factor + text_length_factor;
            
            // Финальная нормализация
            value = value.tanh(); // Ограничиваем [-1, 1]
            
            embedding.push(value);
        }
        
        // L2 нормализация для симуляции нормализованных embeddings
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        
        // Кэшируем результат
        self.cache.insert(text.to_string(), embedding.clone());
        
        Ok(embedding)
    }
    
    /// Batch embedding (просто вызывает embed для каждого элемента)
    pub fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        
        for text in texts {
            results.push(self.embed(text)?);
        }
        
        Ok(results)
    }
    
    pub fn embedding_dim(&self) -> usize {
        self.dimension
    }
    
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

/// Система graceful degradation для embedding сервиса
pub struct GracefulEmbeddingService {
    primary: Option<Box<dyn EmbeddingProvider>>,
    fallback: FallbackEmbeddingService,
    failure_count: usize,
    max_failures: usize,
    use_fallback: bool,
}

pub trait EmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
    fn embedding_dim(&self) -> usize;
    fn is_available(&self) -> bool;
}

// @component: {"k":"C","id":"graceful_embedding","t":"Fallback embedding service","m":{"cur":90,"tgt":95,"u":"%"},"f":["fallback","resilience"]}
impl GracefulEmbeddingService {
    pub fn new(
        primary: Option<Box<dyn EmbeddingProvider>>, 
        dimension: usize,
        max_failures: usize
    ) -> Self {
        info!("🛡️ Инициализация GracefulEmbeddingService");
        info!("   Primary provider: {}", 
            if primary.is_some() { "Available" } else { "None" });
        info!("   Fallback dimension: {}", dimension);
        info!("   Max failures before fallback: {}", max_failures);
        
        Self {
            primary,
            fallback: FallbackEmbeddingService::new(dimension),
            failure_count: 0,
            max_failures,
            use_fallback: false,
        }
    }
    
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        // Проверяем нужно ли использовать fallback
        if self.use_fallback || self.primary.is_none() {
            return self.fallback.embed(text);
        }
        
        // Пытаемся использовать primary provider
        if let Some(ref primary) = self.primary {
            match primary.embed(text) {
                Ok(embedding) => {
                    // Сброс счетчика ошибок при успехе
                    if self.failure_count > 0 {
                        self.failure_count = 0;
                        info!("✅ Primary embedding service recovered");
                    }
                    return Ok(embedding);
                }
                Err(e) => {
                    self.failure_count += 1;
                    error!("❌ Primary embedding failed (attempt {}/{}): {}", 
                           self.failure_count, self.max_failures, e);
                    
                    // Переключаемся на fallback если превышен лимит ошибок
                    if self.failure_count >= self.max_failures {
                        warn!("🔄 Switching to fallback embedding service after {} failures", 
                              self.failure_count);
                        self.use_fallback = true;
                    }
                }
            }
        }
        
        // Используем fallback
        warn!("⚡ Using fallback embedding for: '{}'", 
              if text.len() > 50 { &text[..50] } else { text });
        self.fallback.embed(text)
    }
    
    pub fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if self.use_fallback || self.primary.is_none() {
            return self.fallback.embed_batch(texts);
        }
        
        if let Some(ref primary) = self.primary {
            match primary.embed_batch(texts) {
                Ok(embeddings) => {
                    if self.failure_count > 0 {
                        self.failure_count = 0;
                        info!("✅ Primary embedding service recovered");
                    }
                    return Ok(embeddings);
                }
                Err(e) => {
                    self.failure_count += 1;
                    error!("❌ Primary batch embedding failed (attempt {}/{}): {}", 
                           self.failure_count, self.max_failures, e);
                    
                    if self.failure_count >= self.max_failures {
                        warn!("🔄 Switching to fallback embedding service");
                        self.use_fallback = true;
                    }
                }
            }
        }
        
        warn!("⚡ Using fallback batch embedding for {} texts", texts.len());
        self.fallback.embed_batch(texts)
    }
    
    pub fn force_fallback(&mut self) {
        warn!("🔧 Forcing fallback mode");
        self.use_fallback = true;
    }
    
    pub fn try_recover(&mut self) -> bool {
        if self.use_fallback && self.primary.is_some() {
            if let Some(ref primary) = self.primary {
                if primary.is_available() {
                    info!("🔄 Attempting to recover primary embedding service");
                    self.use_fallback = false;
                    self.failure_count = 0;
                    return true;
                }
            }
        }
        false
    }
    
    pub fn is_using_fallback(&self) -> bool {
        self.use_fallback
    }
    
    pub fn failure_count(&self) -> usize {
        self.failure_count
    }
    
    pub fn embedding_dim(&self) -> usize {
        if let Some(ref primary) = self.primary {
            primary.embedding_dim()
        } else {
            self.fallback.embedding_dim()
        }
    }
    
    pub fn status(&self) -> GracefulServiceStatus {
        GracefulServiceStatus {
            primary_available: self.primary.is_some() && 
                self.primary.as_ref().unwrap().is_available(),
            using_fallback: self.use_fallback,
            failure_count: self.failure_count,
            max_failures: self.max_failures,
            fallback_cache_size: self.fallback.cache_size(),
        }
    }
    
    #[cfg(test)]
    pub fn simulate_primary_recovery(&mut self) {
        if self.primary.is_some() {
            self.use_fallback = false;
            self.failure_count = 0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct GracefulServiceStatus {
    pub primary_available: bool,
    pub using_fallback: bool,
    pub failure_count: usize,
    pub max_failures: usize,
    pub fallback_cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fallback_embedding() {
        let mut service = FallbackEmbeddingService::new(384);
        
        let text1 = "machine learning algorithms";
        let text2 = "deep learning neural networks";
        
        let emb1 = service.embed(text1).unwrap();
        let emb2 = service.embed(text2).unwrap();
        
        assert_eq!(emb1.len(), 384);
        assert_eq!(emb2.len(), 384);
        
        // Детерминистичность - один и тот же текст должен давать одинаковый результат
        let emb1_repeat = service.embed(text1).unwrap();
        assert_eq!(emb1, emb1_repeat);
        
        // Разные тексты должны давать разные embeddings
        assert_ne!(emb1, emb2);
        
        // Проверяем нормализацию
        let norm1: f32 = emb1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm1 - 1.0).abs() < 1e-6, "Embedding should be normalized");
    }
    
    #[test]
    fn test_graceful_degradation() {
        // Создаем сервис без primary provider
        let mut service = GracefulEmbeddingService::new(None, 384, 3);
        
        // Должен сразу использовать fallback  
        let embedding = service.embed("test text").unwrap();
        assert_eq!(embedding.len(), 384);
        // Без primary provider сервис находится в состоянии fallback по умолчанию
        // но флаг use_fallback не устанавливается до первой ошибки
        assert!(service.primary.is_none());
    }
}