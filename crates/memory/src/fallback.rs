use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Fallback embedding service для случаев когда AI модели недоступны
pub struct FallbackEmbeddingService {
    dimension: usize,
    cache: HashMap<String, Vec<f32>>,
}

impl FallbackEmbeddingService {
    pub fn new(dimension: usize) -> Self {
        warn!(
            "🔄 Инициализация fallback embedding service (dimension: {})",
            dimension
        );
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

        // Проверяем валидность dimension
        if self.dimension == 0 {
            return Err(anyhow::anyhow!("Invalid embedding dimension: 0"));
        }

        // Создаем детерминистический embedding
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();

        let mut embedding = Vec::with_capacity(self.dimension);
        let text_length_factor = (text.len() as f32).log2() / 10.0; // Нормализуем по длине текста

        for i in 0..self.dimension {
            // Создаем псевдо-случайное значение на основе хэша и позиции
            let hash_byte = hash[i % 32];
            let position_factor = ((i as f32 + 1.0) / self.dimension as f32).sin();

            let mut value = (hash_byte as f32 / 255.0) * 2.0 - 1.0; // [-1, 1]
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
        } else {
            // Fallback если norm слишком маленький - создаем unit vector
            warn!("Generated embedding has very small norm, using fallback unit vector");
            let default_value = 1.0 / (self.dimension as f32).sqrt();
            for val in &mut embedding {
                *val = default_value;
            }
        }

        // Кэшируем результат только если кэш не переполнен
        if self.cache.len() < 10000 {
            // Ограничиваем размер кэша
            self.cache.insert(text.to_string(), embedding.clone());
        } else {
            warn!("Fallback embedding cache is full, not caching new embeddings");
        }

        Ok(embedding)
    }

    /// Batch embedding (просто вызывает embed для каждого элемента)
    pub fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(texts.len());
        let mut failed_count = 0;

        for (i, text) in texts.iter().enumerate() {
            match self.embed(text) {
                Ok(embedding) => results.push(embedding),
                Err(e) => {
                    error!(
                        "Failed to generate fallback embedding for text {}: {}",
                        i, e
                    );
                    failed_count += 1;

                    // Создаем простой fallback embedding если основная функция failed
                    let fallback_embedding = vec![0.0; self.dimension];
                    results.push(fallback_embedding);
                }
            }
        }

        if failed_count > 0 {
            warn!(
                "Generated {} fallback embeddings out of {} total",
                failed_count,
                texts.len()
            );
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

impl GracefulEmbeddingService {
    pub fn new(
        primary: Option<Box<dyn EmbeddingProvider>>,
        dimension: usize,
        max_failures: usize,
    ) -> Self {
        info!("🛡️ Инициализация GracefulEmbeddingService");
        info!(
            "   Primary provider: {}",
            if primary.is_some() {
                "Available"
            } else {
                "None"
            }
        );
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
                    error!(
                        "❌ Primary embedding failed (attempt {}/{}): {}",
                        self.failure_count, self.max_failures, e
                    );

                    // Переключаемся на fallback если превышен лимит ошибок
                    if self.failure_count >= self.max_failures {
                        warn!(
                            "🔄 Switching to fallback embedding service after {} failures",
                            self.failure_count
                        );
                        self.use_fallback = true;
                    }
                }
            }
        }

        // Используем fallback
        warn!(
            "⚡ Using fallback embedding for: '{}'",
            if text.len() > 50 { &text[..50] } else { text }
        );
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
                    error!(
                        "❌ Primary batch embedding failed (attempt {}/{}): {}",
                        self.failure_count, self.max_failures, e
                    );

                    if self.failure_count >= self.max_failures {
                        warn!("🔄 Switching to fallback embedding service");
                        self.use_fallback = true;
                    }
                }
            }
        }

        warn!(
            "⚡ Using fallback batch embedding for {} texts",
            texts.len()
        );
        self.fallback.embed_batch(texts)
    }

    pub fn force_fallback(&mut self) {
        warn!("🔧 Forcing fallback mode");
        self.use_fallback = true;
    }

    pub fn try_recover(&mut self) -> bool {
        if self.use_fallback {
            match &self.primary {
                Some(provider) => {
                    if provider.is_available() {
                        info!("🔄 Attempting to recover primary embedding service");
                        self.use_fallback = false;
                        self.failure_count = 0;
                        return true;
                    }
                }
                None => {
                    // No primary provider to recover to
                    return false;
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
        match &self.primary {
            Some(provider) if !self.use_fallback => {
                let dim = provider.embedding_dim();
                if dim > 0 {
                    dim
                } else {
                    warn!("Primary provider returned 0 dimension, using fallback");
                    self.fallback.embedding_dim()
                }
            }
            _ => self.fallback.embedding_dim(),
        }
    }

    pub fn status(&self) -> GracefulServiceStatus {
        let primary_available = match &self.primary {
            Some(provider) => provider.is_available(),
            None => false,
        };

        GracefulServiceStatus {
            primary_available,
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
