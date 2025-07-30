use anyhow::{Context, Result};
use ndarray::Array2;
use ort::{inputs, session::{builder::GraphOptimizationLevel, Session}, value::Tensor};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokenizers::tokenizer::Tokenizer;
use tracing::{debug, info};

/// Configuration for ORT 2.0 embedding service
#[derive(Debug, Clone)]
pub struct OrtEmbeddingConfig {
    pub model_name: String,
    pub max_length: usize,
    pub normalize: bool,
    pub pooling_method: String,
    pub num_threads: usize,
}

impl Default for OrtEmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            normalize: true,
            pooling_method: "mean".to_string(),
            num_threads: 4,
        }
    }
}

/// ONNX Runtime 2.0 based embedding service
pub struct OrtEmbeddingService {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<Tokenizer>,
    config: OrtEmbeddingConfig,
}

impl OrtEmbeddingService {
    /// Create a new embedding service with ONNX model for BGE-M3
    pub fn new(
        model_path: impl AsRef<Path>,
        tokenizer_path: impl AsRef<Path>,
        config: OrtEmbeddingConfig,
    ) -> Result<Self> {
        let model_path = model_path.as_ref();
        let tokenizer_path = tokenizer_path.as_ref();
        
        info!("Initializing ORT 2.0 embedding service for BGE-M3");
        info!("Model: {}", model_path.display());
        info!("Tokenizer: {}", tokenizer_path.display());
        
        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
            use std::path::PathBuf;
            let possible_paths = vec![
                std::env::current_dir().unwrap().join("scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("./scripts/onnxruntime/lib/onnxruntime.dll"),
            ];
            
            for dll_path in possible_paths {
                if dll_path.exists() {
                    info!("Found ORT library at: {}", dll_path.display());
                    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
                    break;
                }
            }
        }
        
        // Initialize ONNX Runtime
        ort::init()
            .with_name("ort_bge_m3_embeddings")
            .commit()
            .context("Failed to initialize ONNX Runtime")?;
        
        // Create session with ORT 2.0 API
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(config.num_threads)?
            .commit_from_file(model_path)
            .context("Failed to load ONNX model")?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        info!("ORT embedding service initialized successfully");
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            config,
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
        let (input_ids, attention_mask) = if input_ids.len() > self.config.max_length {
            debug!("Truncating input from {} to {} tokens", input_ids.len(), self.config.max_length);
            (&input_ids[..self.config.max_length], &attention_mask[..self.config.max_length])
        } else {
            (input_ids, attention_mask)
        };
        
        // Convert to i64 for ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&id| id as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&m| m as i64).collect();
        let token_type_ids_i64: Vec<i64> = vec![0i64; input_ids_i64.len()]; // BGE-M3 needs token_type_ids
        
        // Create arrays
        let n_tokens = input_ids_i64.len();
        let input_ids_array = Array2::from_shape_vec((1, n_tokens), input_ids_i64)
            .context("Failed to create input_ids array")?;
        let attention_mask_array = Array2::from_shape_vec((1, n_tokens), attention_mask_i64)
            .context("Failed to create attention_mask array")?;
        let token_type_ids_array = Array2::from_shape_vec((1, n_tokens), token_type_ids_i64)
            .context("Failed to create token_type_ids array")?;
        
        // Create tensors from arrays - ort 2.0 needs tuple format
        let input_ids_shape = input_ids_array.shape().to_vec();
        let input_ids_data = input_ids_array.into_raw_vec();
        let input_ids_tensor = Tensor::from_array((input_ids_shape, input_ids_data))
            .context("Failed to create input_ids tensor")?;
            
        let attention_mask_shape = attention_mask_array.shape().to_vec();
        let attention_mask_data = attention_mask_array.into_raw_vec();
        let attention_mask_tensor = Tensor::from_array((attention_mask_shape, attention_mask_data))
            .context("Failed to create attention_mask tensor")?;
            
        let token_type_ids_shape = token_type_ids_array.shape().to_vec();
        let token_type_ids_data = token_type_ids_array.into_raw_vec();
        let token_type_ids_tensor = Tensor::from_array((token_type_ids_shape, token_type_ids_data))
            .context("Failed to create token_type_ids tensor")?;
        
        // Run inference using ORT 2.0 inputs! macro for BGE-M3 (3 inputs)
        let mut session = self.session.lock().unwrap();
        let outputs = session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "token_type_ids" => token_type_ids_tensor
        ])
        .context("ONNX inference failed")?;
        
        // Extract embeddings from output
        let embeddings = self.extract_embeddings(&outputs)?;
        
        Ok(embeddings)
    }
    
    /// Extract embeddings from session outputs
    fn extract_embeddings(&self, outputs: &ort::session::SessionOutputs) -> Result<Vec<f32>> {
        // Get the first output (usually "last_hidden_state" or similar)
        let output = outputs.iter().next()
            .ok_or_else(|| anyhow::anyhow!("No output from model"))?
            .1;  // Get the value from (name, value) tuple
        
        // Extract raw tensor (shape, data) - more reliable in ort 2.0
        let (shape, data) = output.try_extract_tensor::<f32>()
            .context("Failed to extract raw tensor")?;
        
        // Apply pooling based on raw tensor shape and data
        let embeddings = self.apply_pooling_raw(shape, data)?;
        
        // Normalize if requested
        let embeddings = if self.config.normalize {
            normalize_vector(embeddings)
        } else {
            embeddings
        };
        
        Ok(embeddings)
    }
    
    
    /// Apply pooling to raw tensor data
    fn apply_pooling_raw(&self, shape: &ort::tensor::Shape, data: &[f32]) -> Result<Vec<f32>> {
        // Convert shape to slice - Shape might be a Vec or array
        let shape_vec: Vec<usize> = (0..shape.len())
            .map(|i| shape[i] as usize)
            .collect();
        let shape_slice = shape_vec.as_slice();
        
        match shape_slice.len() {
            3 => {
                // [batch, sequence, hidden] - need pooling
                let batch_size = shape_slice[0];
                let seq_len = shape_slice[1];
                let hidden_size = shape_slice[2];
                
                if batch_size != 1 {
                    return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
                }
                
                match self.config.pooling_method.as_str() {
                    "mean" => {
                        // Mean pooling over sequence dimension
                        let mut pooled = vec![0.0f32; hidden_size];
                        for seq_idx in 0..seq_len {
                            for hidden_idx in 0..hidden_size {
                                let idx = seq_idx * hidden_size + hidden_idx;
                                pooled[hidden_idx] += data[idx];
                            }
                        }
                        
                        // Average
                        for val in &mut pooled {
                            *val /= seq_len as f32;
                        }
                        
                        Ok(pooled)
                    },
                    "cls" => {
                        // Use first token (CLS token)
                        Ok(data[..hidden_size].to_vec())
                    },
                    _ => {
                        // Default to mean pooling
                        self.mean_pooling_raw(data, seq_len, hidden_size)
                    }
                }
            },
            2 => {
                // [batch, hidden] - already pooled
                let batch_size = shape_slice[0];
                if batch_size != 1 {
                    return Err(anyhow::anyhow!("Batch size must be 1, got {}", batch_size));
                }
                
                Ok(data.to_vec())
            },
            _ => {
                Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape_slice))
            }
        }
    }
    
    /// Helper for mean pooling on raw data
    fn mean_pooling_raw(&self, data: &[f32], seq_len: usize, hidden_size: usize) -> Result<Vec<f32>> {
        let mut pooled = vec![0.0f32; hidden_size];
        for seq_idx in 0..seq_len {
            for hidden_idx in 0..hidden_size {
                let idx = seq_idx * hidden_size + hidden_idx;
                pooled[hidden_idx] += data[idx];
            }
        }
        for val in &mut pooled {
            *val /= seq_len as f32;
        }
        Ok(pooled)
    }
    
    /// Batch embedding generation
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        texts.iter()
            .map(|text| self.embed(text))
            .collect()
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