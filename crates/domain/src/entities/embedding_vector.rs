//! EmbeddingVector - Domain representation of vector embeddings
//!
//! Pure domain entity for vector operations

use crate::errors::{DomainError, DomainResult};
use crate::{EmbeddingDimensions, SimilarityScore};
use serde::{Deserialize, Serialize};

/// Domain representation of an embedding vector
///
/// Contains business logic for vector operations
/// Independent of AI framework implementation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingVector {
    /// Vector dimensions (business constraint)
    dimensions: Vec<f32>,

    /// Expected dimension count (business invariant)
    expected_dimensions: EmbeddingDimensions,
}

impl EmbeddingVector {
    /// Create new embedding vector with validation
    pub fn new(
        dimensions: Vec<f32>,
        expected_dimensions: EmbeddingDimensions,
    ) -> DomainResult<Self> {
        if dimensions.len() != expected_dimensions {
            return Err(DomainError::EmbeddingDimensionMismatch {
                expected: expected_dimensions,
                actual: dimensions.len(),
            });
        }

        if dimensions.is_empty() {
            return Err(DomainError::InvalidEmbeddingVector(
                "Vector cannot be empty".to_string(),
            ));
        }

        // Business rule: vectors should be normalized
        if !Self::is_normalized_static(&dimensions) {
            return Err(DomainError::InvalidEmbeddingVector(
                "Vector must be normalized".to_string(),
            ));
        }

        Ok(Self {
            dimensions,
            expected_dimensions,
        })
    }

    /// Create from raw dimensions without normalization check (for infrastructure)
    pub fn from_raw(
        dimensions: Vec<f32>,
        expected_dimensions: EmbeddingDimensions,
    ) -> DomainResult<Self> {
        if dimensions.len() != expected_dimensions {
            return Err(DomainError::EmbeddingDimensionMismatch {
                expected: expected_dimensions,
                actual: dimensions.len(),
            });
        }

        Ok(Self {
            dimensions,
            expected_dimensions,
        })
    }

    /// Get vector dimensions
    pub fn dimensions(&self) -> &[f32] {
        &self.dimensions
    }

    /// Get expected dimension count
    pub fn expected_dimensions(&self) -> EmbeddingDimensions {
        self.expected_dimensions
    }

    /// Calculate cosine similarity with another vector (domain operation)
    pub fn cosine_similarity(&self, other: &EmbeddingVector) -> DomainResult<SimilarityScore> {
        if self.expected_dimensions != other.expected_dimensions {
            return Err(DomainError::EmbeddingDimensionMismatch {
                expected: self.expected_dimensions,
                actual: other.expected_dimensions,
            });
        }

        let dot_product: f32 = self
            .dimensions
            .iter()
            .zip(&other.dimensions)
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.dimensions.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.dimensions.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Calculate L2 (Euclidean) distance
    pub fn l2_distance(&self, other: &EmbeddingVector) -> DomainResult<f32> {
        if self.expected_dimensions != other.expected_dimensions {
            return Err(DomainError::EmbeddingDimensionMismatch {
                expected: self.expected_dimensions,
                actual: other.expected_dimensions,
            });
        }

        let sum_squared: f32 = self
            .dimensions
            .iter()
            .zip(&other.dimensions)
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        Ok(sum_squared.sqrt())
    }

    /// Get vector magnitude (L2 norm)
    pub fn magnitude(&self) -> f32 {
        self.dimensions.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Normalize vector to unit length (business operation)
    pub fn normalize(&mut self) -> DomainResult<()> {
        let magnitude = self.magnitude();

        if magnitude == 0.0 {
            return Err(DomainError::InvalidEmbeddingVector(
                "Cannot normalize zero vector".to_string(),
            ));
        }

        for dimension in &mut self.dimensions {
            *dimension /= magnitude;
        }

        Ok(())
    }

    /// Check if vector is normalized (business rule)
    pub fn is_normalized(&self) -> bool {
        Self::is_normalized_static(&self.dimensions)
    }

    /// Internal helper to check if dimensions are normalized
    fn is_normalized_static(dimensions: &[f32]) -> bool {
        let magnitude: f32 = dimensions.iter().map(|x| x * x).sum::<f32>().sqrt();
        (magnitude - 1.0).abs() < 1e-6
    }

    /// Create zero vector for testing/default purposes
    pub fn zero(dimensions: EmbeddingDimensions) -> Self {
        Self {
            dimensions: vec![0.0; dimensions],
            expected_dimensions: dimensions,
        }
    }

    /// Create unit vector in first dimension
    pub fn unit(dimensions: EmbeddingDimensions) -> Self {
        let mut vec = vec![0.0; dimensions];
        if dimensions > 0 {
            vec[0] = 1.0;
        }
        Self {
            dimensions: vec,
            expected_dimensions: dimensions,
        }
    }

    /// Business validation: check if vector is valid for storage
    pub fn is_valid_for_storage(&self) -> bool {
        self.dimensions.iter().all(|&x| x.is_finite()) &&
        // Check reasonable range (business rule) 
        self.dimensions.iter().all(|&x| x.abs() <= 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_vector_creation() {
        let dimensions = vec![0.6, 0.8]; // Normalized vector
        let vector = EmbeddingVector::new(dimensions, 2).unwrap();
        assert_eq!(vector.dimensions().len(), 2);
        assert!(vector.is_normalized());
    }

    #[test]
    fn test_dimension_validation() {
        let dimensions = vec![0.6, 0.8, 0.0]; // 3 dimensions
        let result = EmbeddingVector::new(dimensions, 2); // Expect 2
        assert!(result.is_err());
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = EmbeddingVector::new(vec![1.0, 0.0], 2).unwrap();
        let vec2 = EmbeddingVector::new(vec![0.0, 1.0], 2).unwrap();
        let vec3 = EmbeddingVector::new(vec![1.0, 0.0], 2).unwrap();

        // Orthogonal vectors should have similarity 0
        let sim1 = vec1.cosine_similarity(&vec2).unwrap();
        assert!((sim1 - 0.0).abs() < 1e-6);

        // Identical vectors should have similarity 1
        let sim2 = vec1.cosine_similarity(&vec3).unwrap();
        assert!((sim2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalization() {
        let mut vector = EmbeddingVector::from_raw(vec![3.0, 4.0], 2).unwrap();
        assert!(!vector.is_normalized());

        vector.normalize().unwrap();
        assert!(vector.is_normalized());
        assert!((vector.magnitude() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_storage_validation() {
        let valid_vector = EmbeddingVector::new(vec![0.6, 0.8], 2).unwrap();
        assert!(valid_vector.is_valid_for_storage());

        let invalid_vector = EmbeddingVector::from_raw(vec![f32::NAN, 0.5], 2).unwrap();
        assert!(!invalid_vector.is_valid_for_storage());
    }
}
