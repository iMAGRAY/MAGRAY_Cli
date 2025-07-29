use anyhow::Result;
use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock реализация модели эмбеддингов для тестирования
/// 
/// Генерирует псевдо-векторы на основе хеша текста
#[derive(Debug)]
pub struct MockEmbeddingModel {
    embedding_dim: usize,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl MockEmbeddingModel {
    pub async fn new<P: AsRef<Path>>(_model_path: P) -> Result<Self> {
        Ok(Self {
            embedding_dim: 1024,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }
    
    /// Генерирует псевдо-эмбеддинги на основе хеша текста
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        
        for text in texts {
            // Проверяем кэш
            let cache_key = format!("mock:{}", blake3::hash(text.as_bytes()).to_hex());
            
            let cached = {
                let cache = self.cache.read().await;
                cache.get(&cache_key).cloned()
            };
            
            let embedding = if let Some(cached) = cached {
                cached
            } else {
                // Генерируем детерминированный псевдо-вектор
                let hash = blake3::hash(text.as_bytes());
                let hash_bytes = hash.as_bytes();
                
                let mut vec = vec![0.0; self.embedding_dim];
                for (i, chunk) in hash_bytes.chunks(4).enumerate() {
                    if i >= vec.len() { break; }
                    let val = u32::from_le_bytes([
                        chunk.get(0).copied().unwrap_or(0),
                        chunk.get(1).copied().unwrap_or(0),
                        chunk.get(2).copied().unwrap_or(0),
                        chunk.get(3).copied().unwrap_or(0),
                    ]);
                    vec[i] = (val as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
                
                // Добавляем немного "семантики" на основе длины и символов
                let text_features = extract_text_features(text);
                for (i, feature) in text_features.iter().enumerate() {
                    if i + 32 < vec.len() {
                        vec[i + 32] = *feature;
                    }
                }
                
                // Нормализуем вектор
                let norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for x in &mut vec {
                        *x /= norm;
                    }
                }
                
                // Кэшируем
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(cache_key, vec.clone());
                }
                
                vec
            };
            
            embeddings.push(embedding);
        }
        
        Ok(embeddings)
    }
}

/// Mock реализация reranker модели
#[derive(Debug)]
pub struct MockRerankerModel {
    cache: Arc<RwLock<HashMap<String, f32>>>,
}

impl MockRerankerModel {
    pub async fn new<P: AsRef<Path>>(_model_path: P) -> Result<Self> {
        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Ранжирует документы относительно запроса
    pub async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        let mut scores = Vec::new();
        
        for (idx, doc) in documents.iter().enumerate() {
            let cache_key = format!("rerank:{}:{}", 
                blake3::hash(query.as_bytes()).to_hex(),
                blake3::hash(doc.as_bytes()).to_hex()
            );
            
            let cached = {
                let cache = self.cache.read().await;
                cache.get(&cache_key).copied()
            };
            
            let score = if let Some(cached) = cached {
                cached
            } else {
                // Простая эвристика для оценки релевантности
                let score = calculate_relevance_score(query, doc);
                
                // Кэшируем
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(cache_key, score);
                }
                
                score
            };
            
            scores.push((idx, score));
        }
        
        // Сортируем по убыванию релевантности
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scores.into_iter().take(top_k).collect())
    }
}

/// Извлекает простые текстовые признаки
fn extract_text_features(text: &str) -> Vec<f32> {
    let mut features = vec![0.0; 16];
    
    // Длина текста (нормализованная)
    features[0] = (text.len() as f32 / 1000.0).min(1.0);
    
    // Количество слов
    let word_count = text.split_whitespace().count();
    features[1] = (word_count as f32 / 100.0).min(1.0);
    
    // Средняя длина слова
    if word_count > 0 {
        let avg_word_len = text.chars().filter(|c| !c.is_whitespace()).count() as f32 / word_count as f32;
        features[2] = (avg_word_len / 10.0).min(1.0);
    }
    
    // Количество строк
    features[3] = (text.lines().count() as f32 / 50.0).min(1.0);
    
    // Специальные символы
    features[4] = (text.chars().filter(|c| !c.is_alphanumeric() && !c.is_whitespace()).count() as f32 / 50.0).min(1.0);
    
    // Заглавные буквы
    features[5] = (text.chars().filter(|c| c.is_uppercase()).count() as f32 / text.len() as f32).min(1.0);
    
    // Цифры
    features[6] = (text.chars().filter(|c| c.is_numeric()).count() as f32 / text.len() as f32).min(1.0);
    
    // Ключевые слова для кода
    let code_keywords = ["fn", "impl", "struct", "pub", "async", "trait", "use", "let", "mut", "self"];
    let keyword_count = code_keywords.iter()
        .filter(|kw| text.contains(*kw))
        .count();
    features[7] = (keyword_count as f32 / code_keywords.len() as f32).min(1.0);
    
    features
}

/// Простая эвристика для оценки релевантности
fn calculate_relevance_score(query: &str, document: &str) -> f32 {
    let query_lower = query.to_lowercase();
    let doc_lower = document.to_lowercase();
    
    let mut score = 0.0;
    
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
    for word in query_words {
        if word.len() >= 3 && doc_lower.contains(word) {
            score += 0.1;
        }
    }
    
    // Нормализуем оценку
    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_embedding() {
        let model = MockEmbeddingModel::new("fake_path").await.unwrap();
        
        let texts = vec![
            "Hello world".to_string(),
            "Hello world".to_string(), // Одинаковый текст должен дать одинаковый вектор
            "Goodbye world".to_string(),
        ];
        
        let embeddings = model.embed(&texts).await.unwrap();
        
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 1024);
        
        // Проверяем что одинаковые тексты дают одинаковые эмбеддинги
        assert_eq!(embeddings[0], embeddings[1]);
        
        // Проверяем что разные тексты дают разные эмбеддинги
        assert_ne!(embeddings[0], embeddings[2]);
    }
    
    #[tokio::test]
    async fn test_mock_reranker() {
        let model = MockRerankerModel::new("fake_path").await.unwrap();
        
        let query = "vector store implementation";
        let documents = vec![
            "This is about vector store and its implementation".to_string(),
            "Random text about cats".to_string(),
            "Implementation of storage system".to_string(),
        ];
        
        let results = model.rerank(query, &documents, 2).await.unwrap();
        
        assert_eq!(results.len(), 2);
        // Первый документ должен быть наиболее релевантным
        assert_eq!(results[0].0, 0);
        assert!(results[0].1 > results[1].1);
    }
}