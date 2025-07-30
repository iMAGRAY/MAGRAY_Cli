use anyhow::{Context, Result};
use ndarray::Array2;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    inputs,
};
use std::path::Path;
use std::sync::Arc;
use tokenizers::tokenizer::Tokenizer;
use tracing::{debug, info};

/// Extended embedding configuration for real service
#[derive(Debug, Clone)]
pub struct RealEmbeddingConfig {
    pub model_name: String,
    pub max_length: usize,
    pub normalize: bool,
    pub pooling_method: String,
    pub num_threads: usize,
}

impl Default for RealEmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "Qwen3-Embedding-0.6B".to_string(),
            max_length: 512,
            normalize: true,
            pooling_method: "mean".to_string(),
            num_threads: 4,
        }
    }
}

/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub embedding_dim: usize,
    pub max_tokens: usize,
    pub supports_batch: bool,
}

/// Embedding result
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
    pub tokens_used: usize,
}

/// Real ONNX-based embedding service using ort 2.0 API
pub struct RealEmbeddingService {
    session: Arc<Session>,
    tokenizer: Arc<Tokenizer>,
    config: RealEmbeddingConfig,
    model_info: ModelInfo,
}

impl RealEmbeddingService {
    /// Create a new embedding service with real ONNX model
    pub fn new(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        config: RealEmbeddingConfig,
    ) -> Result<Self> {
        let model_path = model_path.as_ref();
        let tokenizer_path = tokenizer_path.as_ref();
        
        info!("Initializing real ONNX embedding service");
        info!("Model: {}", model_path.display());
        info!("Tokenizer: {}", tokenizer_path.display());
        
        // Initialize ONNX Runtime session
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads)?
            .build_from_file(model_path)
            .context("Failed to load ONNX model")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        // Get model info from session
        let model_info = extract_model_info(&session, &config);
        
        info!("ONNX embedding service initialized successfully");
        info!("Model dimensions: {}", model_info.embedding_dim);
        
        Ok(Self {
            session: Arc::new(session),
            tokenizer: Arc::new(tokenizer),
            config,
            model_info,
        })
    }
    
    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        debug!("Generating embedding for text: {} chars", text.len());
        
        // Tokenize text
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        
        // Truncate if necessary
        let max_len = self.config.max_length;
        let (input_ids, attention_mask) = if input_ids.len() > max_len {
            debug!("Truncating input from {} to {} tokens", input_ids.len(), max_len);
            (&input_ids[..max_len], &attention_mask[..max_len])
        } else {
            (input_ids, attention_mask)
        };
        
        // Convert to i64 for ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&m| m as i64).collect();
        
        // Create ONNX tensors
        let n_tokens = input_ids_i64.len();
        let input_ids_array = Array2::from_shape_vec(
            (1, n_tokens),
            input_ids_i64
        ).context("Failed to create input_ids array")?;
        
        let attention_mask_array = Array2::from_shape_vec(
            (1, n_tokens),
            attention_mask_i64
        ).context("Failed to create attention_mask array")?;
        
        // Run inference using ort::inputs! macro
        let outputs = self.session.run(inputs! {
            "input_ids" => input_ids_array,
            "attention_mask" => attention_mask_array
        }?)
            .context("ONNX inference failed")?;
        
        // Extract embeddings from output
        let embeddings = extract_embeddings_v2(&outputs, &self.config)?;
        
        Ok(EmbeddingResult {
            embedding: embeddings,
            model: self.config.model_name.clone(),
            dimensions: self.model_info.embedding_dim,
            tokens_used: n_tokens,
        })
    }
    
    /// Get model information
    pub fn model_info(&self) -> &ModelInfo {
        &self.model_info
    }
    
    /// Batch embedding generation
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        texts.iter()
            .map(|text| self.embed(text))
            .collect()
    }
}

/// Extract model information from ONNX session
fn extract_model_info(_session: &Session, config: &RealEmbeddingConfig) -> ModelInfo {
    // For now, we'll use fixed values since the API doesn't expose output shapes easily
    // The Qwen3-Embedding-0.6B model outputs 768-dimensional embeddings
    ModelInfo {
        name: config.model_name.clone(),
        version: "1.0.0".to_string(),
        embedding_dim: 768,
        max_tokens: config.max_length,
        supports_batch: true,
    }
}

/// Extract embeddings from ONNX output (v2 API)
fn extract_embeddings_v2(outputs: &ort::session::SessionOutputs, config: &RealEmbeddingConfig) -> Result<Vec<f32>> {
    // Get the first output value - for embedding models it's usually called "last_hidden_state" or similar
    let output = outputs.iter().next()
        .ok_or_else(|| anyhow::anyhow!("No output from model"))?
        .1;
    
    // Extract tensor data as ArrayView - returns a tuple of (shape, data)
    let (shape, tensor_data) = output.try_extract_raw::<f32>()
        .context("Failed to extract embeddings tensor")?;
    
    let shape_slice = shape.as_slice();
    
    // Handle different output formats
    let embeddings = match shape_slice.len() {
        3 => {
            // [batch, sequence, hidden] - use pooling
            let batch_size = shape_slice[0];
            let seq_len = shape_slice[1];
            let hidden_size = shape_slice[2];
            
            if batch_size != 1 {
                return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
            }
            
            match config.pooling_method.as_str() {
                "mean" => {
                    // Mean pooling over sequence dimension
                    let mut pooled = vec![0.0f32; hidden_size];
                    for seq_idx in 0..seq_len {
                        for hidden_idx in 0..hidden_size {
                            let idx = seq_idx * hidden_size + hidden_idx;
                            pooled[hidden_idx] += tensor_data[idx];
                        }
                    }
                    // Average
                    for val in &mut pooled {
                        *val /= seq_len as f32;
                    }
                    pooled
                },
                "cls" => {
                    // Use first token (CLS token)
                    tensor_data[..hidden_size].to_vec()
                },
                _ => {
                    // Default to mean pooling
                    let mut pooled = vec![0.0f32; hidden_size];
                    for seq_idx in 0..seq_len {
                        for hidden_idx in 0..hidden_size {
                            let idx = seq_idx * hidden_size + hidden_idx;
                            pooled[hidden_idx] += tensor_data[idx];
                        }
                    }
                    for val in &mut pooled {
                        *val /= seq_len as f32;
                    }
                    pooled
                }
            }
        },
        2 => {
            // [batch, hidden] - already pooled
            let batch_size = shape_slice[0];
            if batch_size != 1 {
                return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
            }
            tensor_data.to_vec()
        },
        _ => {
            return Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape_slice));
        }
    };
    
    // Normalize if requested
    let embeddings = if config.normalize {
        normalize_vector(embeddings)
    } else {
        embeddings
    };
    
    Ok(embeddings)
}

/// Normalize vector to unit length
fn normalize_vector(mut vec: Vec<f32>) -> Vec<f32> {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in &mut vec {
            *val /= norm;
        }
    }
    vec
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_normalize_vector() {
        let vec = vec![3.0, 4.0];
        let normalized = normalize_vector(vec);
        
        // Should have unit length
        let norm: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
        
        // Check values
        assert!((normalized[0] - 0.6).abs() < 1e-6);
        assert!((normalized[1] - 0.8).abs() < 1e-6);
    }
}