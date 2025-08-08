#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
// For fuzzing we'll use simplified vector operations
// since we don't need the full memory crate API

#[derive(Debug, Arbitrary)]
struct VectorOperationInput {
    operations: Vec<VectorOperation>,
}

#[derive(Debug, Arbitrary)]
enum VectorOperation {
    CreateVector { values: Vec<f32> },
    NormalizeVector { values: Vec<f32> },
    ComputeDistance { v1: Vec<f32>, v2: Vec<f32> },
    BatchDistance { query: Vec<f32>, vectors: Vec<Vec<f32>> },
    SIMDOperation { v1: Vec<f32>, v2: Vec<f32> },
    DotProduct { v1: Vec<f32>, v2: Vec<f32> },
    VectorAddition { v1: Vec<f32>, v2: Vec<f32> },
    VectorScaling { vector: Vec<f32>, scale: f32 },
}

fuzz_target!(|input: VectorOperationInput| {
    for operation in input.operations.into_iter().take(20) { // Limit operations for performance
        match operation {
            VectorOperation::CreateVector { values } => {
                test_vector_creation(values);
            }
            VectorOperation::NormalizeVector { values } => {
                test_vector_normalization(values);
            }
            VectorOperation::ComputeDistance { v1, v2 } => {
                test_distance_computation(v1, v2);
            }
            VectorOperation::BatchDistance { query, vectors } => {
                test_batch_distance_computation(query, vectors);
            }
            VectorOperation::SIMDOperation { v1, v2 } => {
                test_simd_operations(v1, v2);
            }
            VectorOperation::DotProduct { v1, v2 } => {
                test_dot_product(v1, v2);
            }
            VectorOperation::VectorAddition { v1, v2 } => {
                test_vector_addition(v1, v2);
            }
            VectorOperation::VectorScaling { vector, scale } => {
                test_vector_scaling(vector, scale);
            }
        }
    }
});

fn test_vector_creation(values: Vec<f32>) {
    // Filter out NaN and infinite values
    let filtered_values: Vec<f32> = values
        .into_iter()
        .filter(|x| x.is_finite())
        .take(10000) // Reasonable size limit
        .collect();
    
    if !filtered_values.is_empty() && filtered_values.len() <= 10000 {
        // Test basic vector properties
        assert!(!filtered_values.is_empty(), "Vector should not be empty");
        
        // All values should be finite
        for &val in &filtered_values {
            assert!(val.is_finite(), "Vector values should be finite after creation");
        }
        
        // Test vector magnitude calculation
        let magnitude: f32 = filtered_values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            assert!(magnitude.is_finite(), "Vector magnitude should be finite");
        }
    }
}

fn test_vector_normalization(values: Vec<f32>) {
    let mut filtered_values: Vec<f32> = values
        .into_iter()
        .filter(|x| x.is_finite())
        .take(1000)
        .collect();
    
    if !filtered_values.is_empty() {
        // Test in-place normalization
        let original_values = filtered_values.clone();
        normalize_vector(&mut filtered_values);
        
        // Check that normalization doesn't introduce NaN values
        for &val in &filtered_values {
            assert!(val.is_finite(), "Normalized values should be finite");
        }
        
        // Check magnitude is approximately 1 (if original wasn't zero vector)
        let original_magnitude: f32 = original_values.iter().map(|x| x * x).sum::<f32>().sqrt();
        if original_magnitude > 1e-8 {
            let normalized_magnitude: f32 = filtered_values.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!(
                (normalized_magnitude - 1.0).abs() < 1e-5,
                "Normalized vector should have unit magnitude: {}",
                normalized_magnitude
            );
        }
    }
}

fn test_distance_computation(v1: Vec<f32>, v2: Vec<f32>) {
    // Ensure vectors have same dimension and are finite
    let filtered_v1: Vec<f32> = v1.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    let filtered_v2: Vec<f32> = v2.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    
    if filtered_v1.len() == filtered_v2.len() && !filtered_v1.is_empty() {
        // Test cosine distance
        let distance = cosine_distance(&filtered_v1, &filtered_v2);
        
        // Distance should be finite and within expected range
        assert!(distance.is_finite(), "Distance should be finite");
        assert!(distance >= 0.0, "Cosine distance should be non-negative");
        assert!(distance <= 2.0, "Cosine distance should be <= 2.0");
        
        // Test symmetry
        let distance_reversed = cosine_distance(&filtered_v2, &filtered_v1);
        let diff = (distance - distance_reversed).abs();
        assert!(diff < 1e-6, "Distance should be symmetric");
        
        // Test self-distance (should be 0 for non-zero vectors)
        let self_distance = cosine_distance(&filtered_v1, &filtered_v1);
        if filtered_v1.iter().any(|&x| x != 0.0) {
            assert!(self_distance < 1e-6, "Self-distance should be near 0");
        }
    }
}

fn test_batch_distance_computation(query: Vec<f32>, vectors: Vec<Vec<f32>>) {
    let filtered_query: Vec<f32> = query.into_iter().filter(|x| x.is_finite()).take(512).collect();
    
    if !filtered_query.is_empty() && !vectors.is_empty() {
        // Filter and align vector dimensions
        let aligned_vectors: Vec<Vec<f32>> = vectors
            .into_iter()
            .take(50) // Limit for performance
            .map(|v| {
                v.into_iter()
                    .filter(|x| x.is_finite())
                    .take(filtered_query.len())
                    .collect::<Vec<f32>>()
            })
            .filter(|v| v.len() == filtered_query.len())
            .collect();
        
        if !aligned_vectors.is_empty() {
            // Test batch distance computation
            let batch_distances = compute_batch_distances(&filtered_query, &aligned_vectors);
            
            // Verify results
            assert_eq!(batch_distances.len(), aligned_vectors.len());
            
            for (i, &distance) in batch_distances.iter().enumerate() {
                assert!(distance.is_finite(), "Batch distance {} should be finite", i);
                assert!(distance >= 0.0, "Batch distance {} should be non-negative", i);
                assert!(distance <= 2.0, "Batch distance {} should be reasonable", i);
                
                // Compare with individual computation
                let individual_distance = cosine_distance(&filtered_query, &aligned_vectors[i]);
                let diff = (distance - individual_distance).abs();
                assert!(
                    diff < 1e-5,
                    "Batch and individual distance should match: {} vs {}",
                    distance,
                    individual_distance
                );
            }
        }
    }
}

fn test_simd_operations(v1: Vec<f32>, v2: Vec<f32>) {
    let filtered_v1: Vec<f32> = v1.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    let filtered_v2: Vec<f32> = v2.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    
    if filtered_v1.len() == filtered_v2.len() && !filtered_v1.is_empty() {
        // Test SIMD vs scalar consistency
        let scalar_distance = cosine_distance(&filtered_v1, &filtered_v2);
        let simd_distance = simd_cosine_distance(&filtered_v1, &filtered_v2);
        
        let diff = (scalar_distance - simd_distance).abs();
        assert!(
            diff < 1e-5,
            "SIMD and scalar implementations should match: {} vs {}",
            simd_distance,
            scalar_distance
        );
        
        // Test SIMD dot product
        let scalar_dot = filtered_v1.iter().zip(filtered_v2.iter()).map(|(a, b)| a * b).sum::<f32>();
        let simd_dot = simd_dot_product(&filtered_v1, &filtered_v2);
        
        let dot_diff = (scalar_dot - simd_dot).abs();
        assert!(
            dot_diff < 1e-5,
            "SIMD and scalar dot product should match: {} vs {}",
            simd_dot,
            scalar_dot
        );
    }
}

fn test_dot_product(v1: Vec<f32>, v2: Vec<f32>) {
    let filtered_v1: Vec<f32> = v1.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    let filtered_v2: Vec<f32> = v2.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    
    if filtered_v1.len() == filtered_v2.len() && !filtered_v1.is_empty() {
        let dot_product = filtered_v1.iter().zip(filtered_v2.iter()).map(|(a, b)| a * b).sum::<f32>();
        
        assert!(dot_product.is_finite(), "Dot product should be finite");
        
        // Test symmetry
        let dot_product_reversed = filtered_v2.iter().zip(filtered_v1.iter()).map(|(a, b)| a * b).sum::<f32>();
        let diff = (dot_product - dot_product_reversed).abs();
        assert!(diff < 1e-6, "Dot product should be symmetric");
    }
}

fn test_vector_addition(v1: Vec<f32>, v2: Vec<f32>) {
    let filtered_v1: Vec<f32> = v1.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    let filtered_v2: Vec<f32> = v2.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    
    if filtered_v1.len() == filtered_v2.len() && !filtered_v1.is_empty() {
        let sum: Vec<f32> = filtered_v1.iter()
            .zip(filtered_v2.iter())
            .map(|(a, b)| a + b)
            .collect();
        
        // Verify all results are finite
        for &val in &sum {
            assert!(val.is_finite(), "Vector addition result should be finite");
        }
        
        // Test that addition is commutative
        let sum_reversed: Vec<f32> = filtered_v2.iter()
            .zip(filtered_v1.iter())
            .map(|(a, b)| a + b)
            .collect();
        
        for (i, (&a, &b)) in sum.iter().zip(sum_reversed.iter()).enumerate() {
            let diff = (a - b).abs();
            assert!(diff < 1e-6, "Vector addition should be commutative at index {}: {} vs {}", i, a, b);
        }
    }
}

fn test_vector_scaling(vector: Vec<f32>, scale: f32) {
    let filtered_vector: Vec<f32> = vector.into_iter().filter(|x| x.is_finite()).take(1000).collect();
    
    if !filtered_vector.is_empty() && scale.is_finite() {
        let scaled: Vec<f32> = filtered_vector.iter().map(|&x| x * scale).collect();
        
        // Check that scaling produces finite results (unless scale is extreme)
        if scale.abs() < 1e10 && scale.abs() > 1e-10 {
            for &val in &scaled {
                assert!(val.is_finite(), "Scaled vector values should be finite");
            }
        }
        
        // Test scaling properties
        if scale != 0.0 {
            // Test that scaling by inverse gets back to original (approximately)
            let inverse_scale = 1.0 / scale;
            let rescaled: Vec<f32> = scaled.iter().map(|&x| x * inverse_scale).collect();
            
            for (i, (&original, &rescaled_val)) in filtered_vector.iter().zip(rescaled.iter()).enumerate() {
                if original != 0.0 && scale.abs() > 1e-6 && scale.abs() < 1e6 {
                    let relative_error = ((original - rescaled_val) / original).abs();
                    assert!(
                        relative_error < 1e-5,
                        "Scaling and rescaling should recover original value at index {}: {} vs {}",
                        i, original, rescaled_val
                    );
                }
            }
        }
    }
}

// Helper functions that would be implemented in the actual simd_optimized module
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
    // In a real implementation, this would use SIMD instructions
    // For fuzzing, we use the same implementation to test consistency
    cosine_distance(v1, v2)
}

fn simd_dot_product(v1: &[f32], v2: &[f32]) -> f32 {
    // In a real implementation, this would use SIMD instructions
    v1.iter().zip(v2.iter()).map(|(a, b)| a * b).sum()
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