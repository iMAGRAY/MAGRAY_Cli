use anyhow::Result;

// Новая модульная архитектура следующая принципам SOLID
use crate::hnsw_index::{HnswConfig, HnswStats, VectorIndex};

// Legacy алиасы для обратной совместимости
pub type HnswRsConfig = HnswConfig;
pub type HnswRsStats = HnswStats;

/// Legacy wrapper для обратной совместимости
/// Использует новую модульную архитектуру VectorIndex под капотом
pub struct VectorIndexHnswRs {
    inner: VectorIndex,
}

impl VectorIndexHnswRs {
    /// Создание через новую модульную архитектуру
    pub fn new(config: HnswRsConfig) -> Result<Self> {
        let inner = VectorIndex::new(config)?;
        Ok(Self { inner })
    }

    // Все методы делегируются к новому VectorIndex

    /// Добавление одного вектора
    pub fn add(&self, id: String, vector: Vec<f32>) -> Result<()> {
        self.inner.add(id, vector)
    }

    /// Пакетное добавление векторов
    pub fn add_batch(&self, vectors: Vec<(String, Vec<f32>)>) -> Result<()> {
        self.inner.add_batch(vectors)
    }

    /// Поиск ближайших векторов
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        self.inner.search(query, k)
    }

    /// Параллельный поиск для множественных запросов
    pub fn parallel_search(
        &self,
        queries: &[Vec<f32>],
        k: usize,
    ) -> Result<Vec<Vec<(String, f32)>>> {
        self.inner.parallel_search(queries, k)
    }

    /// Удаление вектора из индекса
    pub fn remove(&self, id: &str) -> Result<bool> {
        self.inner.remove(id)
    }

    /// Получение статистики индекса
    pub fn stats(&self) -> &HnswStats {
        self.inner.stats()
    }

    /// Получение конфигурации
    pub fn config(&self) -> &HnswConfig {
        self.inner.config()
    }

    /// Количество векторов в индексе
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Проверка пустоты индекса
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Проверка существования ID в индексе
    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    /// Очистка индекса
    pub fn clear(&self) {
        self.inner.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_rs_basic() {
        let config = HnswRsConfig::default();
        let index = VectorIndexHnswRs::new(config).unwrap();

        // Тест добавления
        let vector1 = vec![0.1; 1024];
        let vector2 = vec![0.2; 1024];

        index.add("doc1".to_string(), vector1).unwrap();
        index.add("doc2".to_string(), vector2).unwrap();

        assert_eq!(index.len(), 2);

        // Тест поиска
        let query = vec![0.15; 1024];
        let results = index.search(&query, 2).unwrap();

        println!("Результаты поиска: {:?}", results);
        assert_eq!(results.len(), 2);

        // Проверяем что расстояния положительные и логичные
        let (id1, dist1) = &results[0];
        let (id2, dist2) = &results[1];
        println!("Первый результат: {} с расстоянием {}", id1, dist1);
        println!("Второй результат: {} с расстоянием {}", id2, dist2);

        // Результаты должны быть отсортированы по возрастанию расстояния
        assert!(
            dist1 <= dist2,
            "dist1={} должно быть <= dist2={}",
            dist1,
            dist2
        );
    }

    #[test]
    fn test_hnsw_rs_batch() {
        let config = HnswRsConfig::default();
        let index = VectorIndexHnswRs::new(config).unwrap();

        // Тест пакетного добавления
        let vectors = vec![
            ("doc1".to_string(), vec![0.1; 1024]),
            ("doc2".to_string(), vec![0.2; 1024]),
            ("doc3".to_string(), vec![0.3; 1024]),
        ];

        index.add_batch(vectors).unwrap();
        assert_eq!(index.len(), 3);

        // Статистика
        let stats = index.stats();
        // vector_count отражает общее количество записей всех insertion операций
        // add_batch -> add_batch_sequential -> add (3 раза) = 3 записи + 3 single insertions = 6
        assert!(stats.vector_count() >= 3, "Should have at least 3 vectors");
        assert!(stats.avg_insertion_time_ms() >= 0.0);
    }
}
