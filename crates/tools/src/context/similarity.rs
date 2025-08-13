// Similarity calculation utilities for embedding-based tool selection

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// AI crate integration for real embeddings
use ai::embeddings_cpu::{CpuEmbeddingService, OptimizedEmbeddingResult};
use ai::EmbeddingConfig;

/// Embedding similarity calculator
#[derive(Debug, Clone)]
pub struct SimilarityCalculator {
    /// Embedding dimension
    pub dimension: usize,

    /// Similarity threshold for filtering
    pub threshold: f64,

    /// Cached embeddings for performance (thread-safe)
    cached_embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl SimilarityCalculator {
    /// Create new similarity calculator
    pub fn new(dimension: usize, threshold: f64) -> Self {
        Self {
            dimension,
            threshold,
            cached_embeddings: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> Result<f64> {
        if a.len() != b.len() || a.len() != self.dimension {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {} and {}",
                self.dimension,
                a.len(),
                b.len()
            ));
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok((dot_product / (norm_a * norm_b)) as f64)
    }

    /// Calculate Euclidean distance
    pub fn euclidean_distance(&self, a: &[f32], b: &[f32]) -> Result<f64> {
        if a.len() != b.len() || a.len() != self.dimension {
            return Err(anyhow::anyhow!("Embedding dimension mismatch"));
        }

        let sum_squared_diff: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();

        Ok(sum_squared_diff.sqrt() as f64)
    }

    /// Calculate Manhattan distance
    pub fn manhattan_distance(&self, a: &[f32], b: &[f32]) -> Result<f64> {
        if a.len() != b.len() || a.len() != self.dimension {
            return Err(anyhow::anyhow!("Embedding dimension mismatch"));
        }

        let sum_abs_diff: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();

        Ok(sum_abs_diff as f64)
    }

    /// Normalize embedding vector
    pub fn normalize(&self, embedding: &mut [f32]) -> Result<()> {
        if embedding.len() != self.dimension {
            return Err(anyhow::anyhow!("Embedding dimension mismatch"));
        }

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        Ok(())
    }

    /// Find most similar embeddings from cached collection
    pub async fn find_most_similar(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<SimilarityMatch>> {
        let mut similarities = Vec::new();

        // Read from cache
        let cache = self.cached_embeddings.read().await;

        for (id, embedding) in cache.iter() {
            let similarity = self.cosine_similarity(query_embedding, embedding)?;

            if similarity >= self.threshold {
                similarities.push(SimilarityMatch {
                    id: id.clone(),
                    similarity_score: similarity,
                    distance_euclidean: self.euclidean_distance(query_embedding, embedding)?,
                    distance_manhattan: self.manhattan_distance(query_embedding, embedding)?,
                });
            }
        }

        // Sort by similarity score (descending)
        similarities.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top-k results
        similarities.truncate(top_k);

        Ok(similarities)
    }

    /// Find most similar embeddings from provided collection (legacy method)
    pub fn find_most_similar_from_collection(
        &self,
        query_embedding: &[f32],
        candidate_embeddings: &HashMap<String, Vec<f32>>,
        top_k: usize,
    ) -> Result<Vec<SimilarityMatch>> {
        let mut similarities = Vec::new();

        for (id, embedding) in candidate_embeddings {
            let similarity = self.cosine_similarity(query_embedding, embedding)?;

            if similarity >= self.threshold {
                similarities.push(SimilarityMatch {
                    id: id.clone(),
                    similarity_score: similarity,
                    distance_euclidean: self.euclidean_distance(query_embedding, embedding)?,
                    distance_manhattan: self.manhattan_distance(query_embedding, embedding)?,
                });
            }
        }

        // Sort by similarity score (descending)
        similarities.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Take top-k results
        similarities.truncate(top_k);

        Ok(similarities)
    }

    /// Cache embedding for repeated use
    pub async fn cache_embedding(&self, id: String, embedding: Vec<f32>) -> Result<()> {
        if embedding.len() != self.dimension {
            return Err(anyhow::anyhow!("Embedding dimension mismatch"));
        }

        let mut cache = self.cached_embeddings.write().await;
        cache.insert(id, embedding);
        Ok(())
    }

    /// Get cached embedding
    pub async fn get_cached_embedding(&self, id: &str) -> Option<Vec<f32>> {
        let cache = self.cached_embeddings.read().await;
        cache.get(id).cloned()
    }

    /// Clear embedding cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cached_embeddings.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cached_embeddings.read().await;
        CacheStats {
            cached_embeddings: cache.len(),
            total_memory_bytes: cache.len() * self.dimension * 4, // 4 bytes per f32
        }
    }
}

/// Similarity match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityMatch {
    pub id: String,
    pub similarity_score: f64,
    pub distance_euclidean: f64,
    pub distance_manhattan: f64,
}

impl SimilarityMatch {
    /// Check if match is above quality threshold
    pub fn is_high_quality(&self, threshold: f64) -> bool {
        self.similarity_score >= threshold
    }

    /// Get combined score using multiple metrics
    pub fn combined_score(&self, weights: &ScoreWeights) -> f64 {
        weights.similarity * self.similarity_score
            - weights.euclidean * self.distance_euclidean
            - weights.manhattan * self.distance_manhattan
    }
}

/// Weights for combining different similarity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub similarity: f64,
    pub euclidean: f64,
    pub manhattan: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            similarity: 1.0,
            euclidean: 0.1,
            manhattan: 0.05,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cached_embeddings: usize,
    pub total_memory_bytes: usize,
}

/// Embedding aggregator for combining multiple embeddings
#[derive(Debug, Clone)]
pub struct EmbeddingAggregator {
    pub dimension: usize,
}

impl EmbeddingAggregator {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Average multiple embeddings
    pub fn average(&self, embeddings: &[Vec<f32>]) -> Result<Vec<f32>> {
        if embeddings.is_empty() {
            return Err(anyhow::anyhow!("Cannot average empty embedding list"));
        }

        for embedding in embeddings {
            if embedding.len() != self.dimension {
                return Err(anyhow::anyhow!("Embedding dimension mismatch"));
            }
        }

        let mut result = vec![0.0; self.dimension];

        for embedding in embeddings {
            for (i, &val) in embedding.iter().enumerate() {
                result[i] += val;
            }
        }

        let count = embeddings.len() as f32;
        for val in result.iter_mut() {
            *val /= count;
        }

        Ok(result)
    }

    /// Weighted average of embeddings
    pub fn weighted_average(&self, embeddings: &[(Vec<f32>, f64)]) -> Result<Vec<f32>> {
        if embeddings.is_empty() {
            return Err(anyhow::anyhow!("Cannot average empty embedding list"));
        }

        for (embedding, _) in embeddings {
            if embedding.len() != self.dimension {
                return Err(anyhow::anyhow!("Embedding dimension mismatch"));
            }
        }

        let mut result = vec![0.0; self.dimension];
        let mut total_weight = 0.0;

        for (embedding, weight) in embeddings {
            for (i, &val) in embedding.iter().enumerate() {
                result[i] += val * (*weight as f32);
            }
            total_weight += weight;
        }

        if total_weight > 0.0 {
            for val in result.iter_mut() {
                *val /= total_weight as f32;
            }
        }

        Ok(result)
    }

    /// Concatenate embeddings
    pub fn concatenate(&self, embeddings: &[Vec<f32>]) -> Result<Vec<f32>> {
        if embeddings.is_empty() {
            return Err(anyhow::anyhow!("Cannot concatenate empty embedding list"));
        }

        let mut result = Vec::new();

        for embedding in embeddings {
            if embedding.len() != self.dimension {
                return Err(anyhow::anyhow!("Embedding dimension mismatch"));
            }
            result.extend_from_slice(embedding);
        }

        Ok(result)
    }

    /// Element-wise maximum
    pub fn element_wise_max(&self, embeddings: &[Vec<f32>]) -> Result<Vec<f32>> {
        if embeddings.is_empty() {
            return Err(anyhow::anyhow!("Cannot process empty embedding list"));
        }

        for embedding in embeddings {
            if embedding.len() != self.dimension {
                return Err(anyhow::anyhow!("Embedding dimension mismatch"));
            }
        }

        let mut result = embeddings[0].clone();

        for embedding in embeddings.iter().skip(1) {
            for (i, &val) in embedding.iter().enumerate() {
                if val > result[i] {
                    result[i] = val;
                }
            }
        }

        Ok(result)
    }
}

/// Real embedding provider using AI crate models
pub struct RealEmbeddingProvider {
    service: Arc<CpuEmbeddingService>,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    dimension: usize,
}

impl std::fmt::Debug for RealEmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RealEmbeddingProvider")
            .field(
                "cache_size",
                &self.cache.try_read().map(|c| c.len()).unwrap_or(0),
            )
            .field("dimension", &self.dimension)
            .finish_non_exhaustive()
    }
}

impl RealEmbeddingProvider {
    /// Create new real embedding provider
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let service = CpuEmbeddingService::new(config)?;
        let dimension = service.embedding_dim();

        Ok(Self {
            service: Arc::new(service),
            cache: Arc::new(RwLock::new(HashMap::new())),
            dimension,
        })
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Generate embedding for text using real AI model
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(text) {
                return Ok(cached.clone());
            }
        }

        // Generate embedding using real model
        let result = self
            .service
            .embed(text)
            .context("Failed to generate embedding")?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(text.to_string(), result.embedding.clone());
        }

        Ok(result.embedding)
    }

    /// Generate embeddings for multiple texts with batching
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Check cache for all texts
        let mut cached_embeddings = Vec::with_capacity(texts.len());
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        {
            let cache = self.cache.read().await;
            for (i, text) in texts.iter().enumerate() {
                if let Some(cached) = cache.get(text) {
                    cached_embeddings.push((i, cached.clone()));
                } else {
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }
        }

        // Generate embeddings for uncached texts
        let mut new_embeddings = Vec::new();
        if !uncached_texts.is_empty() {
            let results = self
                .service
                .embed_batch(&uncached_texts)
                .context("Failed to generate batch embeddings")?;

            new_embeddings.extend(results.into_iter().map(|r| r.embedding));

            // Cache new embeddings
            {
                let mut cache = self.cache.write().await;
                for (text, embedding) in uncached_texts.iter().zip(new_embeddings.iter()) {
                    cache.insert(text.clone(), embedding.clone());
                }
            }
        }

        // Combine cached and new embeddings in correct order
        let mut final_embeddings = vec![Vec::new(); texts.len()];

        // Fill cached embeddings
        for (i, embedding) in cached_embeddings {
            final_embeddings[i] = embedding;
        }

        // Fill new embeddings
        for (idx_in_final, embedding) in uncached_indices.into_iter().zip(new_embeddings) {
            final_embeddings[idx_in_final] = embedding;
        }

        Ok(final_embeddings)
    }

    /// Check if the embedding service is available
    pub fn is_available(&self) -> bool {
        self.service.is_available()
    }

    /// Get service statistics
    pub fn get_stats(&self) -> ai::embeddings_cpu::ServiceStats {
        self.service.get_stats()
    }

    /// Clear embedding cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        CacheStats {
            cached_embeddings: cache.len(),
            total_memory_bytes: cache.len() * self.dimension * 4, // 4 bytes per f32
        }
    }
}

/// Semantic concept clusters for better mock embeddings
#[derive(Debug, Clone, PartialEq)]
enum SemanticCluster {
    FileOperations,
    WebOperations,
    SystemOperations,
    GitOperations,
    Mixed,
}

/// Mock embedding generator for testing and development
/// Kept for backwards compatibility and testing
#[derive(Debug, Clone)]
pub struct MockEmbeddingGenerator {
    pub dimension: usize,
    pub seed: u64,
}

impl MockEmbeddingGenerator {
    pub fn new(dimension: usize, seed: u64) -> Self {
        Self { dimension, seed }
    }

    /// Generate deterministic mock embedding from text
    pub fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0f32; self.dimension];

        // Simple hash-based generation for consistent results
        let hash_base = self.hash_string(text);

        for (i, val) in embedding.iter_mut().enumerate() {
            let hash = hash_base.wrapping_add(i as u64).wrapping_mul(self.seed);
            *val = ((hash % 2000) as f32 - 1000.0) / 1000.0; // Range [-1, 1]
        }

        // Normalize the embedding
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        embedding
    }

    /// Generate embedding with semantic similarity
    /// Similar texts will have similar embeddings
    pub fn generate_semantic_embedding(&self, text: &str) -> Vec<f32> {
        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        if words.is_empty() {
            return self.generate_embedding("");
        }

        // Enhanced semantic mapping with word relationships
        let mut semantic_values = vec![0.0f32; self.dimension];

        // Process each word with semantic boosting
        for word in &words {
            let word_contribution = self.get_semantic_word_vector(word);
            for (i, &val) in word_contribution.iter().enumerate() {
                semantic_values[i] += val;
            }
        }

        // Normalize by word count
        let count = words.len() as f32;
        for val in semantic_values.iter_mut() {
            *val /= count;
        }

        // Apply semantic clustering - group similar concepts
        self.apply_semantic_clustering(&semantic_values, &words)
    }

    /// Get semantic vector for a word with concept clustering
    fn get_semantic_word_vector(&self, word: &str) -> Vec<f32> {
        let mut base_embedding = self.generate_word_embedding(word);

        // Semantic concept boosting - similar words get similar vector components
        let semantic_boost = self.get_semantic_concept_boost(word);

        // Apply boost to specific dimensions based on word category
        for (i, boost) in semantic_boost.iter().enumerate() {
            if i < base_embedding.len() {
                base_embedding[i] += boost * 0.3; // 30% semantic boost
            }
        }

        base_embedding
    }

    /// Get semantic concept boost based on word meaning
    fn get_semantic_concept_boost(&self, word: &str) -> Vec<f32> {
        let mut boost = vec![0.0f32; self.dimension];

        // File/Read concepts - boost dimensions 0-15
        if [
            "file", "read", "content", "text", "data", "load", "open", "parse", "config", "json",
            "xml", "csv",
        ]
        .contains(&word)
        {
            for item in boost.iter_mut().take(16.min(self.dimension)) {
                *item = 0.8;
            }
        }

        // Web/Network concepts - boost dimensions 16-31
        if [
            "web", "fetch", "http", "api", "request", "response", "network", "url", "json", "rest",
            "get", "post",
        ]
        .contains(&word)
        {
            for item in boost.iter_mut().take(32.min(self.dimension)).skip(16) {
                *item = 0.8;
            }
        }

        // System/Command concepts - boost dimensions 32-47
        if [
            "system", "command", "execute", "shell", "run", "process", "output", "capture", "exec",
            "bash", "cmd",
        ]
        .contains(&word)
        {
            for item in boost.iter_mut().take(48.min(self.dimension)).skip(32) {
                *item = 0.8;
            }
        }

        // Git/Version Control concepts - boost dimensions 48-63
        if [
            "git",
            "status",
            "branch",
            "commit",
            "repository",
            "version",
            "control",
            "diff",
            "merge",
            "push",
            "pull",
        ]
        .contains(&word)
        {
            for item in boost.iter_mut().take(64.min(self.dimension)).skip(48) {
                *item = 0.8;
            }
        }

        // Action verbs get distributed boost
        if [
            "read", "write", "execute", "run", "fetch", "get", "process", "analyze", "check",
            "verify",
        ]
        .contains(&word)
        {
            for i in (0..self.dimension).step_by(8) {
                if i < boost.len() {
                    boost[i] += 0.5;
                }
            }
        }

        boost
    }

    /// Apply semantic clustering to group similar concepts
    fn apply_semantic_clustering(&self, values: &[f32], words: &[&str]) -> Vec<f32> {
        let mut result = values.to_vec();

        // Determine primary concept cluster
        let cluster = self.determine_concept_cluster(words);

        // Amplify the relevant dimensions for the cluster
        match cluster {
            SemanticCluster::FileOperations => {
                for i in 0..16.min(result.len()) {
                    result[i] *= 1.4; // 40% boost for file operations
                }
            }
            SemanticCluster::WebOperations => {
                for i in 16..32.min(result.len()) {
                    result[i] *= 1.4;
                }
            }
            SemanticCluster::SystemOperations => {
                for i in 32..48.min(result.len()) {
                    result[i] *= 1.4;
                }
            }
            SemanticCluster::GitOperations => {
                for i in 48..64.min(result.len()) {
                    result[i] *= 1.4;
                }
            }
            SemanticCluster::Mixed => {
                // No specific amplification for mixed concepts
            }
        }

        // Normalize to unit vector
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in result.iter_mut() {
                *val /= norm;
            }
        }

        result
    }

    /// Determine the primary concept cluster from words
    fn determine_concept_cluster(&self, words: &[&str]) -> SemanticCluster {
        let mut file_score = 0;
        let mut web_score = 0;
        let mut system_score = 0;
        let mut git_score = 0;

        for word in words {
            if [
                "file",
                "read",
                "content",
                "text",
                "data",
                "load",
                "open",
                "parse",
                "config",
                "json",
                "xml",
                "csv",
                "configuration",
                "directory",
                "folder",
            ]
            .contains(word)
            {
                file_score += 1;
            }
            if [
                "web", "fetch", "http", "api", "request", "response", "network", "url", "json",
                "rest", "get", "post", "server", "client",
            ]
            .contains(word)
            {
                web_score += 1;
            }
            if [
                "system", "command", "execute", "shell", "run", "process", "output", "capture",
                "exec", "bash", "cmd", "terminal",
            ]
            .contains(word)
            {
                system_score += 1;
            }
            if [
                "git",
                "status",
                "branch",
                "commit",
                "repository",
                "version",
                "control",
                "diff",
                "merge",
                "push",
                "pull",
                "clone",
            ]
            .contains(word)
            {
                git_score += 1;
            }
        }

        let max_score = file_score.max(web_score).max(system_score).max(git_score);

        if max_score == 0 {
            return SemanticCluster::Mixed;
        }

        if file_score == max_score {
            SemanticCluster::FileOperations
        } else if web_score == max_score {
            SemanticCluster::WebOperations
        } else if system_score == max_score {
            SemanticCluster::SystemOperations
        } else if git_score == max_score {
            SemanticCluster::GitOperations
        } else {
            SemanticCluster::Mixed
        }
    }

    fn generate_word_embedding(&self, word: &str) -> Vec<f32> {
        self.generate_embedding(word)
    }

    fn hash_string(&self, text: &str) -> u64 {
        // Simple hash function
        text.chars()
            .enumerate()
            .map(|(i, c)| (c as u64).wrapping_mul(i as u64 + 1))
            .fold(self.seed, |acc, x| acc.wrapping_add(x).wrapping_mul(31))
    }
}

/// Unified embedding provider that can use either real or mock embeddings
pub enum EmbeddingProvider {
    Real(RealEmbeddingProvider),
    Mock(MockEmbeddingGenerator),
}

impl std::fmt::Debug for EmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Real(provider) => f
                .debug_tuple("EmbeddingProvider::Real")
                .field(provider)
                .finish(),
            Self::Mock(generator) => f
                .debug_tuple("EmbeddingProvider::Mock")
                .field(generator)
                .finish(),
        }
    }
}

impl EmbeddingProvider {
    /// Create real embedding provider
    pub fn real(config: EmbeddingConfig) -> Result<Self> {
        Ok(Self::Real(RealEmbeddingProvider::new(config)?))
    }

    /// Create mock embedding provider
    pub fn mock(dimension: usize, seed: u64) -> Self {
        Self::Mock(MockEmbeddingGenerator::new(dimension, seed))
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        match self {
            Self::Real(provider) => provider.dimension(),
            Self::Mock(generator) => generator.dimension,
        }
    }

    /// Generate single embedding
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Real(provider) => provider.generate_embedding(text).await,
            Self::Mock(generator) => Ok(generator.generate_semantic_embedding(text)),
        }
    }

    /// Generate batch of embeddings
    pub async fn generate_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        match self {
            Self::Real(provider) => provider.generate_embeddings_batch(texts).await,
            Self::Mock(generator) => Ok(texts
                .iter()
                .map(|text| generator.generate_semantic_embedding(text))
                .collect()),
        }
    }

    /// Check if provider is available
    pub fn is_available(&self) -> bool {
        match self {
            Self::Real(provider) => provider.is_available(),
            Self::Mock(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() -> Result<()> {
        let calculator = SimilarityCalculator::new(3, 0.0);

        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0];
        let vec3 = vec![1.0, 0.0, 0.0];

        let sim1 = calculator.cosine_similarity(&vec1, &vec2)?;
        let sim2 = calculator.cosine_similarity(&vec1, &vec3)?;

        assert!((sim1 - 0.0).abs() < 1e-6); // Orthogonal vectors
        assert!((sim2 - 1.0).abs() < 1e-6); // Identical vectors
        Ok(())
    }

    #[test]
    fn test_euclidean_distance() -> Result<()> {
        let calculator = SimilarityCalculator::new(2, 0.0);

        let vec1 = vec![0.0, 0.0];
        let vec2 = vec![3.0, 4.0];

        let distance = calculator.euclidean_distance(&vec1, &vec2)?;
        assert!((distance - 5.0).abs() < 1e-6); // 3-4-5 triangle
        Ok(())
    }

    #[test]
    fn test_normalize() -> Result<()> {
        let calculator = SimilarityCalculator::new(2, 0.0);
        let mut embedding = vec![3.0, 4.0];

        calculator.normalize(&mut embedding)?;

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn test_embedding_aggregator() -> Result<()> {
        let aggregator = EmbeddingAggregator::new(3);

        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];

        let average = aggregator.average(&embeddings)?;
        let expected = [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];

        for (a, e) in average.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-6);
        }
        Ok(())
    }

    #[test]
    fn test_mock_embedding_generator() {
        let generator = MockEmbeddingGenerator::new(384, 42);

        let embedding1 = generator.generate_embedding("test text");
        let embedding2 = generator.generate_embedding("test text");
        let embedding3 = generator.generate_embedding("different text");

        assert_eq!(embedding1.len(), 384);
        assert_eq!(embedding1, embedding2); // Deterministic
        assert_ne!(embedding1, embedding3); // Different inputs

        // Check normalization
        let norm: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_semantic_embedding_similarity() -> Result<()> {
        let generator = MockEmbeddingGenerator::new(128, 123);

        let embedding1 = generator.generate_semantic_embedding("read file content");
        let embedding2 = generator.generate_semantic_embedding("file read operation");
        let embedding3 = generator.generate_semantic_embedding("network connection");

        let calculator = SimilarityCalculator::new(128, 0.0);

        let sim_related = calculator.cosine_similarity(&embedding1, &embedding2)?;
        let sim_unrelated = calculator.cosine_similarity(&embedding1, &embedding3)?;

        // Related texts should have higher similarity
        assert!(sim_related > sim_unrelated);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_most_similar() -> Result<()> {
        let calculator = SimilarityCalculator::new(3, 0.5);

        // Cache some embeddings first
        calculator
            .cache_embedding("similar".to_string(), vec![0.8, 0.6, 0.0])
            .await?;
        calculator
            .cache_embedding("dissimilar".to_string(), vec![0.0, 0.0, 1.0])
            .await?;
        calculator
            .cache_embedding("identical".to_string(), vec![1.0, 0.0, 0.0])
            .await?;

        let query = vec![1.0, 0.0, 0.0];
        let matches = calculator.find_most_similar(&query, 10).await?;

        // Should find similar matches, ordered by similarity
        assert!(matches.len() <= 3);
        if !matches.is_empty() {
            assert_eq!(matches[0].id, "identical");
        }
        Ok(())
    }

    #[test]
    fn test_find_most_similar_from_collection() -> Result<()> {
        let calculator = SimilarityCalculator::new(3, 0.5);

        let query = vec![1.0, 0.0, 0.0];
        let mut candidates = HashMap::new();
        candidates.insert("similar".to_string(), vec![0.8, 0.6, 0.0]);
        candidates.insert("dissimilar".to_string(), vec![0.0, 0.0, 1.0]);
        candidates.insert("identical".to_string(), vec![1.0, 0.0, 0.0]);

        let matches = calculator.find_most_similar_from_collection(&query, &candidates, 10)?;

        // Should find similar matches, ordered by similarity
        assert!(matches.len() <= 3);
        if !matches.is_empty() {
            assert_eq!(matches[0].id, "identical");
        }
        Ok(())
    }
}
