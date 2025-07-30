use anyhow::{Context, Result};
use ort::{
    environment::Environment,
    session::{builder::{GraphOptimizationLevel, SessionBuilder}, Session},
    value::Value,
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

/// Real ONNX-based embedding service
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
        
        // Initialize ONNX Runtime environment
        let environment = Environment::builder()
            .with_name("magray_embeddings")
            .with_log_level(ort::environment::LoggingLevel::Warning)
            .build()
            .context("Failed to create ONNX Runtime environment")?;
        
        // Create session with optimizations
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads as i16)?
            .with_model_from_file(model_path)
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
        let input_ids_array = ndarray::Array2::from_shape_vec(
            (1, n_tokens),
            input_ids_i64
        ).context("Failed to create input_ids array")?;
        
        let attention_mask_array = ndarray::Array2::from_shape_vec(
            (1, n_tokens),
            attention_mask_i64
        ).context("Failed to create attention_mask array")?;
        
        // Create input values
        let input_ids_value = Value::from_array(self.session.allocator(), &input_ids_array)
            .context("Failed to create input_ids tensor")?;
        let attention_mask_value = Value::from_array(self.session.allocator(), &attention_mask_array)
            .context("Failed to create attention_mask tensor")?;
        
        // Run inference
        let outputs = self.session.run(vec![input_ids_value, attention_mask_value])
            .context("ONNX inference failed")?;
        
        // Extract embeddings from output
        let embeddings = extract_embeddings(&outputs, &self.config)?;
        
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
fn extract_model_info(session: &Session, config: &RealEmbeddingConfig) -> ModelInfo {
    // Get output shape to determine embedding dimension
    let outputs = session.outputs();
    let embedding_dim = if let Some(output) = outputs.first() {
        // Try to get last dimension from shape
        if let Some(shape) = output.dimensions() {
            shape.last().copied().unwrap_or(768) as usize
        } else {
            768 // Default dimension
        }
    } else {
        768
    };
    
    ModelInfo {
        name: config.model_name.clone(),
        version: "1.0.0".to_string(),
        embedding_dim,
        max_tokens: config.max_length,
        supports_batch: true,
    }
}

/// Extract embeddings from ONNX output
fn extract_embeddings(outputs: &[Value], config: &RealEmbeddingConfig) -> Result<Vec<f32>> {
    // Get the first output (should be embeddings)
    let output = outputs.first()
        .ok_or_else(|| anyhow::anyhow!("No output from model"))?;
    
    // Extract tensor data
    let tensor = output.try_extract::<f32>()
        .context("Failed to extract embeddings tensor")?;
    
    let tensor_view = tensor.view();
    let shape = tensor_view.shape();
    
    // Handle different output formats
    let embeddings = match shape.len() {
        3 => {
            // [batch, sequence, hidden] - use pooling
            let batch_size = shape[0];
            let seq_len = shape[1];
            let hidden_size = shape[2];
            
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
                            pooled[hidden_idx] += tensor_view.as_slice().unwrap()[idx];
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
                    tensor_view.as_slice().unwrap()[..hidden_size].to_vec()
                },
                _ => {
                    // Default to mean pooling
                    let mut pooled = vec![0.0f32; hidden_size];
                    for seq_idx in 0..seq_len {
                        for hidden_idx in 0..hidden_size {
                            let idx = seq_idx * hidden_size + hidden_idx;
                            pooled[hidden_idx] += tensor_view.as_slice().unwrap()[idx];
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
            let batch_size = shape[0];
            if batch_size != 1 {
                return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
            }
            tensor_view.as_slice().unwrap().to_vec()
        },
        _ => {
            return Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape));
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
    use tempfile::TempDir;
    
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