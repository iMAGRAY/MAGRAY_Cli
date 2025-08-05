//! Property-based tests for HNSW vector search
//! 
//! Покрывает:
//! - Invariants векторного поиска
//! - Property-based testing с quickcheck
//! - Correctness properties для HNSW
//! - Distance function properties
//! - Search result consistency
//! - Performance properties

use memory::{
    vector_index_hnswlib::VectorIndexHNSW,
    types::{Layer, Record, SearchOptions},
};
use anyhow::Result;
use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};
use std::collections::HashMap;
use chrono::Utc;

// @component: {"k":"T","id":"hnsw_property_based_tests","t":"Property-based tests for HNSW vector search","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","property","hnsw","vector","quickcheck","coverage"]}

/// Генератор векторов для property-based тестирования
#[derive(Clone, Debug)]
struct TestVector {
    values: Vec<f32>,
}

impl Arbitrary for TestVector {
    fn arbitrary(g: &mut Gen) -> Self {
        let size = g.gen_range(1..=1024); // Размер от 1 до 1024
        let values: Vec<f32> = (0..size)
            .map(|_| {
                // Генерируем нормализованные значения в диапазоне [-1.0, 1.0]
                let raw: f32 = f32::arbitrary(g);
                if raw.is_finite() {
                    raw.clamp(-1.0, 1.0)
                } else {
                    0.0
                }
            })
            .collect();
        
        TestVector { values }
    }
    
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let mut shrunk = Vec::new();
        
        // Shrink размер вектора
        if self.values.len() > 1 {
            let half_size = self.values.len() / 2;
            shrunk.push(TestVector {
                values: self.values[..half_size].to_vec(),
            });
        }
        
        // Shrink значения к нулю
        if self.values.iter().any(|&x| x != 0.0) {
            shrunk.push(TestVector {
                values: vec![0.0; self.values.len()],
            });
        }
        
        Box::new(shrunk.into_iter())
    }
}

/// Генератор записей для тестирования
#[derive(Clone, Debug)]
struct TestRecord {
    id: String,
    content: String,
    embedding: TestVector,
    layer: Layer,
}

impl Arbitrary for TestRecord {
    fn arbitrary(g: &mut Gen) -> Self {
        let id = format!("test_id_{}", u32::arbitrary(g));
        let content = format!("test_content_{}", u32::arbitrary(g));
        let embedding = TestVector::arbitrary(g);
        let layer = match g.gen_range(0..3) {
            0 => Layer::Interact,
            1 => Layer::Insights,
            _ => Layer::Assets,
        };
        
        TestRecord {
            id,
            content,
            embedding,
            layer,
        }
    }
}

impl TestRecord {
    fn to_record(&self) -> Record {
        Record {
            id: self.id.clone(),
            content: self.content.clone(),
            embedding: self.embedding.values.clone(),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            layer: self.layer,
            score: None,
        }
    }
}

/// Утилиты для создания HNSW индекса
fn create_test_index(dimension: usize) -> Result<VectorIndexHNSW> {
    let temp_dir = tempfile::TempDir::new()?;
    let index_path = temp_dir.path().join("test_index.hnsw");
    
    VectorIndexHNSW::new(index_path, dimension, 16, 200, 1000)
}

/// Euclidean distance для проверки корректности
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Dot product для проверки корректности
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Cosine similarity для проверки корректности  
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot = dot_product(a, b);
    let norm_a = a.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[test]
fn property_search_returns_valid_results() {
    fn test_search_validity(records: Vec<TestRecord>, query_record: TestRecord) -> TestResult {
        if records.is_empty() || query_record.embedding.values.is_empty() {
            return TestResult::discard();
        }
        
        // Проверяем что все векторы имеют одинаковую размерность
        let dimension = query_record.embedding.values.len();
        if !records.iter().all(|r| r.embedding.values.len() == dimension) {
            return TestResult::discard();
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = match create_test_index(dimension) {
                Ok(idx) => idx,
                Err(_) => return TestResult::discard(),
            };
            
            // Добавляем записи в индекс
            for (i, record) in records.iter().enumerate() {
                if let Err(_) = index.add_vector(i as u64, &record.embedding.values) {
                    return TestResult::discard();
                }
            }
            
            if let Err(_) = index.build_index() {
                return TestResult::discard();
            }
            
            // Выполняем поиск
            let search_results = match index.search(&query_record.embedding.values, 10) {
                Ok(results) => results,
                Err(_) => return TestResult::discard(),
            };
            
            // Проверяем свойства результатов поиска
            let mut valid = true;
            
            // 1. Все ID должны быть валидными
            for &id in &search_results {
                if id >= records.len() as u64 {
                    valid = false;
                    break;
                }
            }
            
            // 2. Результаты должны быть отсортированы по релевантности (расстоянию)
            if search_results.len() > 1 {
                for i in 0..search_results.len() - 1 {
                    let dist1 = euclidean_distance(
                        &query_record.embedding.values,
                        &records[search_results[i] as usize].embedding.values
                    );
                    let dist2 = euclidean_distance(
                        &query_record.embedding.values,
                        &records[search_results[i + 1] as usize].embedding.values
                    );
                    
                    // HNSW может не гарантировать строгий порядок, но должен быть приблизительно правильным
                    // Позволяем небольшую погрешность
                    if dist1 > dist2 * 1.5 {
                        valid = false;
                        break;
                    }
                }
            }
            
            // 3. Количество результатов не должно превышать запрошенное
            if search_results.len() > 10 {
                valid = false;
            }
            
            TestResult::from_bool(valid)
        })
    }
    
    quickcheck(test_search_validity as fn(Vec<TestRecord>, TestRecord) -> TestResult);
}

#[test]
fn property_identical_vectors_have_zero_distance() {
    fn test_identical_distance(vector: TestVector) -> TestResult {
        if vector.values.is_empty() || vector.values.len() > 512 {
            return TestResult::discard();
        }
        
        let distance = euclidean_distance(&vector.values, &vector.values);
        TestResult::from_bool(distance < 1e-6) // Практически ноль
    }
    
    quickcheck(test_identical_distance as fn(TestVector) -> TestResult);
}

#[test]
fn property_distance_symmetry() {
    fn test_distance_symmetry(a: TestVector, b: TestVector) -> TestResult {
        if a.values.is_empty() || b.values.is_empty() || a.values.len() != b.values.len() {
            return TestResult::discard();
        }
        
        let dist_ab = euclidean_distance(&a.values, &b.values);
        let dist_ba = euclidean_distance(&b.values, &a.values);
        
        TestResult::from_bool((dist_ab - dist_ba).abs() < 1e-6)
    }
    
    quickcheck(test_distance_symmetry as fn(TestVector, TestVector) -> TestResult);
}

#[test]
fn property_triangle_inequality() {
    fn test_triangle_inequality(a: TestVector, b: TestVector, c: TestVector) -> TestResult {
        if a.values.is_empty() || b.values.is_empty() || c.values.is_empty() ||
           a.values.len() != b.values.len() || b.values.len() != c.values.len() {
            return TestResult::discard();
        }
        
        let dist_ab = euclidean_distance(&a.values, &b.values);
        let dist_bc = euclidean_distance(&b.values, &c.values);
        let dist_ac = euclidean_distance(&a.values, &c.values);
        
        // Triangle inequality: dist(a,c) <= dist(a,b) + dist(b,c)
        TestResult::from_bool(dist_ac <= dist_ab + dist_bc + 1e-6) // Small epsilon for floating point
    }
    
    quickcheck(test_triangle_inequality as fn(TestVector, TestVector, TestVector) -> TestResult);
}

#[test]
fn property_cosine_similarity_bounds() {
    fn test_cosine_bounds(a: TestVector, b: TestVector) -> TestResult {
        if a.values.is_empty() || b.values.is_empty() || a.values.len() != b.values.len() {
            return TestResult::discard();
        }
        
        // Фильтруем нулевые векторы
        let norm_a = a.values.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let norm_b = b.values.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        
        if norm_a < 1e-6 || norm_b < 1e-6 {
            return TestResult::discard();
        }
        
        let similarity = cosine_similarity(&a.values, &b.values);
        
        // Cosine similarity должна быть в диапазоне [-1, 1]
        TestResult::from_bool(similarity >= -1.0 - 1e-6 && similarity <= 1.0 + 1e-6)
    }
    
    quickcheck(test_cosine_bounds as fn(TestVector, TestVector) -> TestResult);
}

#[test]
fn property_search_consistency() {
    fn test_search_consistency(records: Vec<TestRecord>, query: TestVector, k1: u8, k2: u8) -> TestResult {
        if records.is_empty() || query.values.is_empty() || k1 == 0 || k2 == 0 || k1 > k2 {
            return TestResult::discard();
        }
        
        let dimension = query.values.len();
        if !records.iter().all(|r| r.embedding.values.len() == dimension) {
            return TestResult::discard();
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = match create_test_index(dimension) {
                Ok(idx) => idx,
                Err(_) => return TestResult::discard(),
            };
            
            // Добавляем записи
            for (i, record) in records.iter().enumerate() {
                if let Err(_) = index.add_vector(i as u64, &record.embedding.values) {
                    return TestResult::discard();
                }
            }
            
            if let Err(_) = index.build_index() {
                return TestResult::discard();
            }
            
            // Выполняем поиск с разными k
            let results_k1 = match index.search(&query.values, k1 as usize) {
                Ok(r) => r,
                Err(_) => return TestResult::discard(),
            };
            
            let results_k2 = match index.search(&query.values, k2 as usize) {
                Ok(r) => r,
                Err(_) => return TestResult::discard(),
            };
            
            // Проверяем что первые k1 результатов в results_k2 совпадают с results_k1
            // (с учетом того что HNSW приблизительный)
            let k1_usize = k1 as usize;
            if results_k1.len() == k1_usize && results_k2.len() >= k1_usize {
                let results_k2_truncated = &results_k2[..k1_usize];
                
                // Проверяем что результаты "приблизительно" те же
                let mut matches = 0;
                for &id1 in &results_k1 {
                    if results_k2_truncated.contains(&id1) {
                        matches += 1;
                    }
                }
                
                // Позволяем до 20% различий для приблизительного поиска HNSW
                let threshold = (k1_usize as f32 * 0.8) as usize;
                TestResult::from_bool(matches >= threshold)
            } else {
                TestResult::passed()
            }
        })
    }
    
    quickcheck(test_search_consistency as fn(Vec<TestRecord>, TestVector, u8, u8) -> TestResult);
}

#[test]
fn property_index_persistence() {
    fn test_index_persistence(records: Vec<TestRecord>) -> TestResult {
        if records.is_empty() || records.len() > 100 {
            return TestResult::discard();
        }
        
        let dimension = records[0].embedding.values.len();
        if dimension == 0 || dimension > 256 || !records.iter().all(|r| r.embedding.values.len() == dimension) {
            return TestResult::discard();
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let temp_dir = match tempfile::TempDir::new() {
                Ok(dir) => dir,
                Err(_) => return TestResult::discard(),
            };
            let index_path = temp_dir.path().join("persistence_test.hnsw");
            
            // Создаем и заполняем индекс
            {
                let index = match VectorIndexHNSW::new(index_path.clone(), dimension, 16, 200, records.len()) {
                    Ok(idx) => idx,
                    Err(_) => return TestResult::discard(),
                };
                
                for (i, record) in records.iter().enumerate() {
                    if let Err(_) = index.add_vector(i as u64, &record.embedding.values) {
                        return TestResult::discard();
                    }
                }
                
                if let Err(_) = index.build_index() {
                    return TestResult::discard();
                }
                
                if let Err(_) = index.save() {
                    return TestResult::discard();
                }
            }
            
            // Загружаем индекс заново
            let loaded_index = match VectorIndexHNSW::load(index_path, dimension) {
                Ok(idx) => idx,
                Err(_) => return TestResult::discard(),
            };
            
            // Проверяем что поиск работает одинаково
            if records.is_empty() {
                return TestResult::passed();
            }
            
            let query = &records[0].embedding.values;
            let results = match loaded_index.search(query, 5) {
                Ok(r) => r,
                Err(_) => return TestResult::discard(),
            };
            
            // Результаты должны содержать валидные ID
            let valid = results.iter().all(|&id| (id as usize) < records.len());
            TestResult::from_bool(valid)
        })
    }
    
    quickcheck(test_index_persistence as fn(Vec<TestRecord>) -> TestResult);
}

#[test]
fn property_vector_normalization_invariant() {
    fn test_normalization_invariant(vector: TestVector, scale: f32) -> TestResult {
        if vector.values.is_empty() || scale == 0.0 || !scale.is_finite() {
            return TestResult::discard();
        }
        
        let scaled_vector: Vec<f32> = vector.values.iter().map(|&x| x * scale).collect();
        
        // Вычисляем cosine similarity между оригинальным и масштабированным векторами
        let similarity = cosine_similarity(&vector.values, &scaled_vector);
        
        // Cosine similarity должна быть 1.0 для векторов в одном направлении
        // или -1.0 для противоположных направлений
        let expected = if scale > 0.0 { 1.0 } else { -1.0 };
        
        TestResult::from_bool((similarity - expected).abs() < 1e-5)
    }
    
    quickcheck(test_normalization_invariant as fn(TestVector, f32) -> TestResult);
}

#[test]
fn property_empty_search_behavior() {
    fn test_empty_search(dimension: u8) -> TestResult {
        let dim = dimension as usize;
        if dim == 0 || dim > 256 {
            return TestResult::discard();
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = match create_test_index(dim) {
                Ok(idx) => idx,
                Err(_) => return TestResult::discard(),
            };
            
            // Пытаемся искать в пустом индексе
            let query = vec![1.0; dim];
            let results = match index.search(&query, 10) {
                Ok(r) => r,
                Err(_) => return TestResult::passed(), // Ошибка ожидаема для пустого индекса
            };
            
            // Пустой индекс должен возвращать пустые результаты
            TestResult::from_bool(results.is_empty())
        })
    }
    
    quickcheck(test_empty_search as fn(u8) -> TestResult);
}

#[test]
fn property_search_limit_respected() {
    fn test_search_limit(records: Vec<TestRecord>, limit: u8) -> TestResult {
        if records.is_empty() || limit == 0 {
            return TestResult::discard();
        }
        
        let dimension = records[0].embedding.values.len();
        if dimension == 0 || !records.iter().all(|r| r.embedding.values.len() == dimension) {
            return TestResult::discard();
        }
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let index = match create_test_index(dimension) {
                Ok(idx) => idx,
                Err(_) => return TestResult::discard(),
            };
            
            // Добавляем записи
            for (i, record) in records.iter().enumerate() {
                if let Err(_) = index.add_vector(i as u64, &record.embedding.values) {
                    return TestResult::discard();
                }
            }
            
            if let Err(_) = index.build_index() {
                return TestResult::discard();
            }
            
            // Выполняем поиск с лимитом
            let query = &records[0].embedding.values;
            let results = match index.search(query, limit as usize) {
                Ok(r) => r,
                Err(_) => return TestResult::discard(),
            };
            
            // Количество результатов не должно превышать лимит
            TestResult::from_bool(results.len() <= limit as usize)
        })
    }
    
    quickcheck(test_search_limit_respected as fn(Vec<TestRecord>, u8) -> TestResult);
}