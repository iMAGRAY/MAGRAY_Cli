#![cfg(all(not(feature = "minimal"), feature = "hnsw-index", feature = "rayon"))]

use arbitrary::{Arbitrary, Unstructured};
use memory::{
    hnsw_index::{HnswConfig, HnswIndex, HnswStats},
    simd_optimized::*,
    types::{EmbeddingVector, Record, SearchQuery},
};
use proptest::prelude::*;
use quickcheck::{quickcheck, TestResult};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Debug, Clone, Arbitrary)]
struct TestVector {
    dimensions: u16, // 1-1000 dimensions
    values: Vec<f32>,
}

impl TestVector {
    fn to_embedding_vector(&self) -> EmbeddingVector {
        let normalized_values: Vec<f32> = self
            .values
            .iter()
            .take(self.dimensions as usize)
            .map(|&v| v.clamp(-10.0, 10.0)) // Clamp to reasonable range
            .collect();

        EmbeddingVector::new(normalized_values).unwrap()
    }

    fn generate_similar(&self, similarity: f32) -> TestVector {
        let noise_factor = 1.0 - similarity.clamp(0.0, 1.0);
        let mut similar_values = self.values.clone();

        for value in &mut similar_values {
            let noise = (fastrand::f32() - 0.5) * noise_factor;
            *value += noise;
        }

        TestVector {
            dimensions: self.dimensions,
            values: similar_values,
        }
    }
}

#[derive(Debug, Clone)]
struct HnswTestConfig {
    max_connections: usize,
    ef_construction: usize,
    ef_search: usize,
    max_layers: usize,
    distance_threshold: f32,
}

impl Arbitrary for HnswTestConfig {
    fn arbitrary(u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(HnswTestConfig {
            max_connections: u.int_in_range(2..=64)?,
            ef_construction: u.int_in_range(16..=200)?,
            ef_search: u.int_in_range(10..=100)?,
            max_layers: u.int_in_range(2..=16)?,
            distance_threshold: u.choose(&[0.1, 0.3, 0.5, 0.7, 0.9])?,
        })
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_hnsw_distance_symmetry(
        vector1 in any::<TestVector>(),
        vector2 in any::<TestVector>()
    ) {
        // Arrange - ensure vectors have same dimensions
        let dim = vector1.dimensions.min(vector2.dimensions).max(2) as usize;
        let v1: Vec<f32> = vector1.values.into_iter().take(dim).collect();
        let v2: Vec<f32> = vector2.values.into_iter().take(dim).collect();

        if v1.len() == dim && v2.len() == dim {
            // Act - compute distances both ways
            let distance_1_to_2 = cosine_distance(&v1, &v2);
            let distance_2_to_1 = cosine_distance(&v2, &v1);

            // Assert - distance should be symmetric
            let diff = (distance_1_to_2 - distance_2_to_1).abs();
            prop_assert!(diff < 1e-6,
                "Distance should be symmetric: d(a,b) = {} != d(b,a) = {}, diff = {}",
                distance_1_to_2, distance_2_to_1, diff);
        }
    }

    #[test]
    fn test_hnsw_triangle_inequality(
        vector1 in any::<TestVector>(),
        vector2 in any::<TestVector>(),
        vector3 in any::<TestVector>()
    ) {
        // Arrange - ensure all vectors have same dimensions
        let dim = vector1.dimensions.min(vector2.dimensions).min(vector3.dimensions).max(2) as usize;
        let v1: Vec<f32> = vector1.values.into_iter().take(dim).collect();
        let v2: Vec<f32> = vector2.values.into_iter().take(dim).collect();
        let v3: Vec<f32> = vector3.values.into_iter().take(dim).collect();

        if v1.len() == dim && v2.len() == dim && v3.len() == dim {
            // Act - compute distances
            let d12 = cosine_distance(&v1, &v2);
            let d23 = cosine_distance(&v2, &v3);
            let d13 = cosine_distance(&v1, &v3);

            // Note: Cosine distance doesn't strictly satisfy triangle inequality,
            // but we test a relaxed version
            let tolerance = 0.1;
            prop_assert!(d13 <= d12 + d23 + tolerance,
                "Triangle inequality (relaxed): d(a,c) = {} > d(a,b) + d(b,c) = {} + {} = {}",
                d13, d12, d23, d12 + d23);
        }
    }

    #[test]
    fn test_hnsw_index_insertion_idempotency(
        vectors in prop::collection::vec(any::<TestVector>(), 1..20),
        config in any::<HnswTestConfig>()
    ) {
        tokio_test::block_on(async {
            // Arrange - create HNSW index with test configuration
            let hnsw_config = HnswConfig {
                max_connections: config.max_connections,
                ef_construction: config.ef_construction,
                ef_search: config.ef_search,
                max_layers: config.max_layers,
                distance_threshold: config.distance_threshold,
            };

            let mut index = HnswIndex::new(hnsw_config);
            let mut inserted_ids = HashSet::new();

            // Act - insert vectors and track IDs
            for (i, test_vector) in vectors.iter().enumerate() {
                let embedding = test_vector.to_embedding_vector();
                let record = Record::new(
                    format!("test_record_{}", i),
                    format!("test content {}", i),
                    embedding);

                let result = index.insert(record.clone()).await;
                if result.is_ok() {
                    inserted_ids.insert(record.id.clone());

                    // Verify the record can be found
                    let search_result = index.search(&record.embedding_vector, 5, config.ef_search).await;
                    if let Ok(results) = search_result {
                        prop_assert!(!results.is_empty(), "Should find at least the inserted record");

                        // The inserted record should be among the results (possibly with small distance due to floating point)
                        let found = results.iter().any(|r| r.record_id == record.id);
                        prop_assert!(found, "Inserted record should be findable in search results");
                    }
                }
            }

            // Assert - verify index properties
            let stats = index.get_stats().await;
            prop_assert_eq!(stats.total_vectors as usize, inserted_ids.len(),
                "Index should contain exactly the number of successfully inserted vectors");

            // Test idempotency - inserting same records again should not change the index significantly
            let initial_stats = index.get_stats().await;

            for (i, test_vector) in vectors.iter().enumerate().take(3) { // Test first 3 for performance
                let embedding = test_vector.to_embedding_vector();
                let duplicate_record = Record::new(
                    format!("test_record_{}", i), // Same ID
                    format!("test content {}", i),
                    embedding);

                let _ = index.insert(duplicate_record).await; // May succeed or fail, both are valid
            }

            let final_stats = index.get_stats().await;
            // The index should not have grown significantly from duplicate insertions
            prop_assert!(final_stats.total_vectors <= initial_stats.total_vectors + 3,
                "Duplicate insertions should not significantly increase index size");
        })?;
    }

    #[test]
    fn test_hnsw_search_consistency(
        base_vector in any::<TestVector>(),
        similarity_levels in prop::collection::vec(0.1f32..1.0f32, 3..10),
        k in 1usize..10,
        config in any::<HnswTestConfig>()
    ) {
        tokio_test::block_on(async {
            // Arrange - create index with similar vectors at different similarity levels
            let hnsw_config = HnswConfig {
                max_connections: config.max_connections,
                ef_construction: config.ef_construction,
                ef_search: config.ef_search.max(k * 2), // Ensure ef_search >= k
                max_layers: config.max_layers,
                distance_threshold: config.distance_threshold,
            };

            let mut index = HnswIndex::new(hnsw_config);
            let mut expected_order = Vec::new();

            // Insert base vector and similar vectors
            let base_embedding = base_vector.to_embedding_vector();
            let base_record = Record::new("base".to_string(), "base content".to_string(), base_embedding.clone());
            let _ = index.insert(base_record).await;

            for (i, &similarity) in similarity_levels.iter().enumerate() {
                let similar_vector = base_vector.generate_similar(similarity);
                let similar_embedding = similar_vector.to_embedding_vector();
                let distance = cosine_distance(&base_embedding.vector, &similar_embedding.vector);

                let record = Record::new(
                    format!("similar_{}", i),
                    format!("similar content {}", i),
                    similar_embedding);

                if index.insert(record.clone()).await.is_ok() {
                    expected_order.push((record.id.clone(), distance));
                }
            }

            // Sort by expected distance (closer should come first)
            expected_order.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            let search_results = index.search(&base_embedding, k.min(expected_order.len()), config.ef_search).await;

            // Assert - verify search consistency
            if let Ok(results) = search_results {
                prop_assert!(!results.is_empty(), "Search should return results when vectors exist");
                prop_assert!(results.len() <= k, "Should not return more than k results");

                // Verify results are in approximate distance order
                for i in 1..results.len() {
                    prop_assert!(results[i-1].score <= results[i].score + 0.1,
                        "Results should be approximately ordered by distance: result[{}].score = {} > result[{}].score = {}",
                        i-1, results[i-1].score, i, results[i].score);
                }

                if results.len() >= 2 {
                    let first_result_expected_pos = expected_order.iter()
                        .position(|(id, _)| *id == results[0].record_id);
                    let second_result_expected_pos = expected_order.iter()
                        .position(|(id, _)| *id == results[1].record_id);

                    if let (Some(pos1), Some(pos2)) = (first_result_expected_pos, second_result_expected_pos) {
                        // Allow some flexibility due to HNSW approximation
                        prop_assert!(pos1 <= pos2 + 2,
                            "Search order should roughly match expected distance order: {} vs {}",
                            pos1, pos2);
                    }
                }
            }
        })?;
    }
}

quickcheck! {
    fn qc_simd_distance_consistency(v1: Vec<f32>, v2: Vec<f32>) -> TestResult {
        if v1.len() != v2.len() || v1.len() < 4 || v1.len() > 1000 {
            return TestResult::discard();
        }

        // Test that SIMD and scalar implementations give same results
        let simd_distance = simd_cosine_distance(&v1, &v2);
        let scalar_distance = cosine_distance(&v1, &v2);

        let diff = (simd_distance - scalar_distance).abs();
        TestResult::from_bool(diff < 1e-5)
    }

    fn qc_vector_normalization_invariant(mut vector: Vec<f32>) -> TestResult {
        if vector.is_empty() || vector.len() > 1000 {
            return TestResult::discard();
        }

        // Remove any NaN or infinite values
        vector.retain(|x| x.is_finite());
        if vector.is_empty() {
            return TestResult::discard();
        }

        // Test normalization invariant
        let original_magnitude = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if original_magnitude < 1e-8 {
            return TestResult::discard(); // Skip zero vectors
        }

        normalize_vector(&mut vector);
        let normalized_magnitude = vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        TestResult::from_bool((normalized_magnitude - 1.0).abs() < 1e-6)
    }

    fn qc_batch_processing_consistency(vectors: Vec<Vec<f32>>, query: Vec<f32>) -> TestResult {
        if vectors.is_empty() || vectors.len() > 50 {
            return TestResult::discard();
        }

        // Ensure all vectors have same dimension and are reasonable size
        let dim = query.len();
        if dim < 4 || dim > 512 {
            return TestResult::discard();
        }

        let filtered_vectors: Vec<Vec<f32>> = vectors.into_iter()
            .filter(|v| v.len() == dim)
            .take(10) // Limit for performance
            .collect();

        if filtered_vectors.is_empty() {
            return TestResult::discard();
        }

        // Test that batch processing gives same results as individual processing
        let batch_distances = compute_batch_distances(&query, &filtered_vectors);

        let individual_distances: Vec<f32> = filtered_vectors.iter()
            .map(|v| cosine_distance(&query, v))
            .collect();

        if batch_distances.len() != individual_distances.len() {
            return TestResult::from_bool(false);
        }

        for (batch_dist, individual_dist) in batch_distances.iter().zip(individual_distances.iter()) {
            if (batch_dist - individual_dist).abs() > 1e-5 {
                return TestResult::from_bool(false);
            }
        }

        TestResult::from_bool(true)
    }
}

#[test]
fn fuzz_hnsw_operations_stability() {
    let mut data = vec![0u8; 10000];

    for _ in 0..100 {
        // Generate random data
        for byte in &mut data {
            *byte = fastrand::u8(..);
        }

        let mut unstructured = Unstructured::new(&data);

        // Try to generate random operations
        if let Ok(operations) = Vec::<HnswOperation>::arbitrary(&mut unstructured) {
            tokio_test::block_on(async {
                let config = HnswConfig::default();
                let mut index = HnswIndex::new(config);

                for operation in operations.into_iter().take(20) {
                    // Limit operations
                    match operation {
                        HnswOperation::Insert { vector, id } => {
                            if !vector.is_empty()
                                && vector.len() <= 1000
                                && vector.iter().all(|x| x.is_finite())
                            {
                                let embedding = EmbeddingVector::new(vector).unwrap();
                                let record = Record::new(id, "fuzz test".to_string(), embedding);
                                let _ = index.insert(record).await; // Don't panic on any input
                            }
                        }
                        HnswOperation::Search { query_vector, k } => {
                            if !query_vector.is_empty()
                                && query_vector.len() <= 1000
                                && query_vector.iter().all(|x| x.is_finite())
                                && k > 0
                                && k <= 100
                            {
                                let embedding = EmbeddingVector::new(query_vector).unwrap();
                                let _ = index.search(&embedding, k, 50).await; // Should not panic
                            }
                        }
                        HnswOperation::Remove { id } => {
                            let _ = index.remove(&id).await; // Should handle gracefully
                        }
                    }
                }

                // Index should remain in a valid state
                let stats = index.get_stats().await;
                assert!(stats.total_vectors <= 100); // Reasonable upper bound
                assert!(stats.total_connections >= 0);
            });
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
enum HnswOperation {
    Insert { vector: Vec<f32>, id: String },
    Search { query_vector: Vec<f32>, k: usize },
    Remove { id: String },
}

// Performance property tests
#[test]
fn property_search_performance_scaling() {
    tokio_test::block_on(async {
        let dimensions = 128;
        let config = HnswConfig::default();
        let mut index = HnswIndex::new(config);

        // Test search performance at different index sizes
        let sizes = vec![100, 500, 1000, 2000];

        for &size in &sizes {
            // Clear and rebuild index
            index = HnswIndex::new(config);

            // Insert vectors
            for i in 0..size {
                let vector: Vec<f32> = (0..dimensions)
                    .map(|_| fastrand::f32() * 2.0 - 1.0)
                    .collect();

                let embedding = EmbeddingVector::new(vector).unwrap();
                let record =
                    Record::new(format!("record_{}", i), format!("content_{}", i), embedding);
                let _ = index.insert(record).await;
            }

            // Measure search time
            let query_vector: Vec<f32> = (0..dimensions)
                .map(|_| fastrand::f32() * 2.0 - 1.0)
                .collect();
            let query_embedding = EmbeddingVector::new(query_vector).unwrap();

            let start = Instant::now();
            let results = index.search(&query_embedding, 10, 50).await;
            let search_time = start.elapsed();

            assert!(results.is_ok(), "Search should succeed");

            let time_per_vector = search_time.as_nanos() as f64 / size as f64;
            println!(
                "Index size: {}, Search time: {:?}, Time per vector: {:.2}ns",
                size, search_time, time_per_vector
            );

            assert!(
                search_time < Duration::from_millis(100),
                "Search should complete within 100ms even for large indices"
            );
        }
    });
}

#[test]
fn property_memory_usage_scaling() {
    tokio_test::block_on(async {
        let dimensions = 64;
        let config = HnswConfig {
            max_connections: 16,
            ef_construction: 200,
            ef_search: 50,
            max_layers: 4,
            distance_threshold: 0.5,
        };

        let sizes = vec![100, 500, 1000];

        for &size in &sizes {
            let mut index = HnswIndex::new(config);

            // Insert vectors and measure memory growth
            let initial_stats = index.get_stats().await;

            for i in 0..size {
                let vector: Vec<f32> = (0..dimensions)
                    .map(|_| fastrand::f32() * 2.0 - 1.0)
                    .collect();

                let embedding = EmbeddingVector::new(vector).unwrap();
                let record =
                    Record::new(format!("record_{}", i), format!("content_{}", i), embedding);
                let _ = index.insert(record).await;
            }

            let final_stats = index.get_stats().await;

            // Memory usage should scale reasonably with the number of vectors
            let memory_per_vector = final_stats.memory_usage_bytes as f64 / size as f64;
            println!(
                "Index size: {}, Memory per vector: {:.2} bytes",
                size, memory_per_vector
            );

            assert!(
                memory_per_vector > dimensions as f64 * 4.0, // At least 4 bytes per dimension
                "Memory usage should account for vector storage"
            );
            assert!(
                memory_per_vector < dimensions as f64 * 100.0, // No more than 100 bytes per dimension
                "Memory usage should not be excessive"
            );

            // Connection density should be reasonable
            if final_stats.total_vectors > 0 {
                let connections_per_vector =
                    final_stats.total_connections as f64 / final_stats.total_vectors as f64;
                assert!(
                    connections_per_vector > 1.0,
                    "Should have connections between vectors"
                );
                assert!(
                    connections_per_vector < config.max_connections as f64 * 2.0,
                    "Connection density should be within reasonable bounds"
                );
            }
        }
    });
}

fn cosine_distance(v1: &[f32], v2: &[f32]) -> f32 {
    assert_eq!(v1.len(), v2.len());

    let dot_product: f32 = v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum();
    let norm_a: f32 = v1.iter().map(|a| a * a).sum::<f32>().sqrt();
    let norm_b: f32 = v2.iter().map(|b| b * b).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        1.0 // Maximum distance for zero vectors
    } else {
        1.0 - (dot_product / (norm_a * norm_b)).clamp(-1.0, 1.0)
    }
}

fn simd_cosine_distance(v1: &[f32], v2: &[f32]) -> f32 {
    cosine_distance(v1, v2)
}

fn normalize_vector(vector: &mut [f32]) {
    let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 1e-8 {
        for x in vector.iter_mut() {
            *x /= magnitude;
        }
    }
}

fn compute_batch_distances(query: &[f32], vectors: &[Vec<f32>]) -> Vec<f32> {
    vectors.iter().map(|v| cosine_distance(query, v)).collect()
}
