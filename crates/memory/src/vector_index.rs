use crate::{MemRef, MemMeta, MemSearchResult};
use anyhow::Result;
use std::collections::HashMap;

/// Простой векторный индекс в памяти
/// 
/// В будущем можно заменить на более эффективную реализацию
/// например, используя HNSW или другие алгоритмы
#[derive(Debug)]
pub struct VectorIndex {
    /// Векторы (id -> vector)
    vectors: HashMap<String, Vec<f32>>,
    /// Метаданные (id -> (mem_ref, snippet, meta))
    metadata: HashMap<String, (MemRef, String, MemMeta)>,
    /// Размерность векторов
    dimension: Option<usize>,
}

impl VectorIndex {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
            metadata: HashMap::new(),
            dimension: None,
        }
    }
    
    /// Добавить вектор в индекс
    pub fn add(
        &mut self,
        vector: Vec<f32>,
        mem_ref: MemRef,
        snippet: String,
        meta: MemMeta,
    ) -> Result<()> {
        // Проверяем размерность
        if let Some(dim) = self.dimension {
            if vector.len() != dim {
                return Err(anyhow::anyhow!(
                    "Vector dimension mismatch: expected {}, got {}",
                    dim,
                    vector.len()
                ));
            }
        } else {
            self.dimension = Some(vector.len());
        }
        
        let id = format!("{}:{}", mem_ref.layer as u8, mem_ref.key);
        self.vectors.insert(id.clone(), vector);
        self.metadata.insert(id, (mem_ref, snippet, meta));
        
        Ok(())
    }
    
    /// Удалить вектор из индекса
    pub fn remove(&mut self, mem_ref: &MemRef) -> Result<bool> {
        let id = format!("{}:{}", mem_ref.layer as u8, mem_ref.key);
        let removed_vector = self.vectors.remove(&id).is_some();
        let removed_meta = self.metadata.remove(&id).is_some();
        
        Ok(removed_vector && removed_meta)
    }
    
    /// Поиск K ближайших соседей
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<MemSearchResult>> {
        if self.vectors.is_empty() {
            return Ok(Vec::new());
        }
        
        // Проверяем размерность
        if let Some(dim) = self.dimension {
            if query.len() != dim {
                return Err(anyhow::anyhow!(
                    "Query dimension mismatch: expected {}, got {}",
                    dim,
                    query.len()
                ));
            }
        }
        
        // Вычисляем косинусное сходство со всеми векторами
        let mut scores: Vec<(String, f32)> = self.vectors
            .iter()
            .map(|(id, vector)| {
                let similarity = cosine_similarity(query, vector);
                (id.clone(), similarity)
            })
            .collect();
        
        // Сортируем по убыванию сходства
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Берем топ K
        let results: Vec<MemSearchResult> = scores
            .into_iter()
            .take(k)
            .filter_map(|(id, score)| {
                self.metadata.get(&id).map(|(mem_ref, snippet, meta)| {
                    MemSearchResult {
                        mem_ref: mem_ref.clone(),
                        score,
                        snippet: Some(snippet.clone()),
                        meta: meta.clone(),
                    }
                })
            })
            .collect();
        
        Ok(results)
    }
    
    /// Получить количество векторов в индексе
    pub fn len(&self) -> usize {
        self.vectors.len()
    }
    
    /// Проверить пустой ли индекс
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
    
    /// Очистить индекс
    pub fn clear(&mut self) {
        self.vectors.clear();
        self.metadata.clear();
        self.dimension = None;
    }
    
    /// Перестроить индекс (заглушка для будущей оптимизации)
    pub fn rebuild_index(&mut self) -> Result<()> {
        // В будущем здесь может быть построение HNSW или другого индекса
        Ok(())
    }
    
    /// Очистить устаревшие векторы (заглушка)
    pub fn vacuum(&mut self) -> Result<u64> {
        // В будущем здесь может быть логика очистки
        Ok(0)
    }
}

/// Вычислить косинусное сходство между двумя векторами
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    
    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    #[test]
    fn test_vector_index_basic() {
        let mut index = VectorIndex::new();
        
        // Добавляем векторы
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0];
        let vec3 = vec![0.0, 0.0, 1.0];
        
        let ref1 = MemRef::new(crate::MemLayer::Short, "doc1".to_string());
        let ref2 = MemRef::new(crate::MemLayer::Short, "doc2".to_string());
        let ref3 = MemRef::new(crate::MemLayer::Short, "doc3".to_string());
        
        index.add(vec1.clone(), ref1.clone(), "Document 1".to_string(), MemMeta::default()).unwrap();
        index.add(vec2.clone(), ref2.clone(), "Document 2".to_string(), MemMeta::default()).unwrap();
        index.add(vec3.clone(), ref3.clone(), "Document 3".to_string(), MemMeta::default()).unwrap();
        
        assert_eq!(index.len(), 3);
        
        // Поиск похожих на vec1
        let results = index.search(&vec1, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].mem_ref.key, "doc1");
        assert_eq!(results[0].score, 1.0); // Точное совпадение
        
        // Удаление
        assert!(index.remove(&ref1).unwrap());
        assert_eq!(index.len(), 2);
    }
    
    #[test]
    fn test_cosine_similarity() {
        // Одинаковые векторы
        let v1 = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&v1, &v1), 1.0);
        
        // Ортогональные векторы
        let v2 = vec![1.0, 0.0];
        let v3 = vec![0.0, 1.0];
        assert_eq!(cosine_similarity(&v2, &v3), 0.0);
        
        // Противоположные векторы
        let v4 = vec![1.0, 0.0];
        let v5 = vec![-1.0, 0.0];
        assert_eq!(cosine_similarity(&v4, &v5), -1.0);
    }
}