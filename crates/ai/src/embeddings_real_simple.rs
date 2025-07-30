use anyhow::{Context, Result};
use ndarray::{Array2, ArrayViewD};
use ort::{inputs, session::{Session, SessionOutputs}};
use std::path::Path;
use std::sync::Arc;
use tokenizers::tokenizer::Tokenizer;
use tracing::{debug, info};

/// Simple real embedding service using ort 2.0
pub struct RealEmbeddingServiceSimple {
    session: Arc<Session>,
    tokenizer: Arc<Tokenizer>,
    max_length: usize,
    normalize: bool,
}

impl RealEmbeddingServiceSimple {
    /// Create a new embedding service
    pub fn new(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let model_path = model_path.as_ref();
        let tokenizer_path = tokenizer_path.as_ref();
        
        info!("Initializing real ONNX embedding service (simple)");
        info!("Model: {}", model_path.display());
        info!("Tokenizer: {}", tokenizer_path.display());
        
        // Initialize ONNX Runtime session
        let session = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)
            .context("Failed to load ONNX model")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        info!("ONNX embedding service initialized successfully");
        
        Ok(Self {
            session: Arc::new(session),
            tokenizer: Arc::new(tokenizer),
            max_length: 512,
            normalize: true,
        })
    }
    
    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text: {} chars", text.len());
        
        // Tokenize text
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        
        // Truncate if necessary
        let (input_ids, attention_mask) = if input_ids.len() > self.max_length {
            debug!("Truncating input from {} to {} tokens", input_ids.len(), self.max_length);
            (&input_ids[..self.max_length], &attention_mask[..self.max_length])
        } else {
            (input_ids, attention_mask)
        };
        
        // Convert to i64 for ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&m| m as i64).collect();
        
        // Create ONNX tensors
        let n_tokens = input_ids_i64.len();
        let input_ids_array = Array2::from_shape_vec((1, n_tokens), input_ids_i64)
            .context("Failed to create input_ids array")?;
        let attention_mask_array = Array2::from_shape_vec((1, n_tokens), attention_mask_i64)
            .context("Failed to create attention_mask array")?;
        
        // Run inference
        let outputs = self.session.run(inputs! {
            "input_ids" => input_ids_array,
            "attention_mask" => attention_mask_array
        }?)
        .context("ONNX inference failed")?;
        
        // Extract embeddings from output
        let embeddings = self.extract_embeddings(&outputs)?;
        
        Ok(embeddings)
    }
    
    /// Extract embeddings from output
    fn extract_embeddings(&self, outputs: &SessionOutputs) -> Result<Vec<f32>> {
        // Get the first output
        let output = outputs.iter().next()
            .ok_or_else(|| anyhow::anyhow!("No output from model"))?
            .1;
        
        // Extract tensor data
        let tensor_view = output.try_extract_tensor::<f32>()
            .context("Failed to extract embeddings tensor")?;
        
        let embeddings = self.process_tensor_output(tensor_view)?;
        
        // Normalize if requested
        let embeddings = if self.normalize {
            normalize_vector(embeddings)
        } else {
            embeddings
        };
        
        Ok(embeddings)
    }
    
    /// Process tensor output based on shape
    fn process_tensor_output(&self, tensor: ArrayViewD<f32>) -> Result<Vec<f32>> {
        let shape = tensor.shape();
        
        match shape.len() {
            3 => {
                // [batch, sequence, hidden] - use mean pooling
                let batch_size = shape[0];
                let seq_len = shape[1];
                let hidden_size = shape[2];
                
                if batch_size != 1 {
                    return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
                }
                
                // Mean pooling over sequence dimension
                let mut pooled = vec![0.0f32; hidden_size];
                for seq_idx in 0..seq_len {
                    for hidden_idx in 0..hidden_size {
                        pooled[hidden_idx] += tensor[[0, seq_idx, hidden_idx]];
                    }
                }
                
                // Average
                for val in &mut pooled {
                    *val /= seq_len as f32;
                }
                
                Ok(pooled)
            },
            2 => {
                // [batch, hidden] - already pooled
                let batch_size = shape[0];
                if batch_size != 1 {
                    return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
                }
                
                // Extract as vector
                let hidden_size = shape[1];
                let mut result = vec![0.0f32; hidden_size];
                for i in 0..hidden_size {
                    result[i] = tensor[[0, i]];
                }
                Ok(result)
            },
            _ => {
                Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape))
            }
        }
    }
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