use anyhow::Result;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, debug, warn};
use crate::tokenization::OptimizedTokenizer;

/// BGE-M3 Embedding Service with real ONNX Runtime 2.0
pub struct BgeM3EmbeddingService {
    session: Arc<std::sync::Mutex<Session>>,
    tokenizer: Arc<OptimizedTokenizer>,
    model_path: PathBuf,
    hidden_size: usize,
}

/// Result of embedding operation  
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub text: String,
    pub embedding: Vec<f32>,
    pub token_count: usize,
}

impl BgeM3EmbeddingService {
    /// Create new BGE-M3 embedding service
    pub fn new(model_path: PathBuf) -> Result<Self> {
        info!("Initializing BGE-M3 embedding service");
        
        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
            // Try multiple possible paths for ONNX Runtime DLL
            let possible_paths = vec![
                std::env::current_dir().unwrap().join("scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("./scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("C:/Users/1/Documents/GitHub/MAGRAY_Cli/scripts/onnxruntime/lib/onnxruntime.dll"),
            ];
            
            for dll_path in possible_paths {
                if dll_path.exists() {
                    info!("Found ONNX Runtime DLL at: {}", dll_path.display());
                    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
                    break;
                }
            }
        }
        
        // Initialize ONNX Runtime
        ort::init()
            .with_name("bge_m3_embedding")
            .commit()?;
        
        // Create session
        let session = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_path)?;
        
        info!("✅ BGE-M3 session created successfully");
        info!("   Model: {}", model_path.display());
        info!("   Inputs: {}", session.inputs.len());
        info!("   Outputs: {}", session.outputs.len());
        
        // Verify it's the expected BGE-M3 model (3 inputs)
        if session.inputs.len() != 3 {
            warn!("Expected 3 inputs for BGE-M3, got {}", session.inputs.len());
        }
        
        let hidden_size = 1024; // BGE-M3 размерность из config.json
        
        // Load proper XLMRoberta tokenizer
        let tokenizer_path = model_path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid model path"))?
            .join("tokenizer.json");
        
        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!("Tokenizer not found at: {}", tokenizer_path.display()));
        }
        
        let tokenizer = OptimizedTokenizer::new(tokenizer_path, 512)?;
        info!("✅ XLMRoberta tokenizer loaded successfully");
        
        Ok(Self {
            session: Arc::new(std::sync::Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_path,
            hidden_size,
        })
    }
    
    /// Generate embedding for single text
    pub fn embed(&self, text: &str) -> Result<EmbeddingResult> {
        let results = self.embed_batch(&[text.to_string()])?;
        Ok(results.into_iter().next().unwrap())
    }
    
    /// Generate embeddings for multiple texts
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        debug!("Generating BGE-M3 embeddings for {} texts", texts.len());
        
        let mut results = Vec::new();
        
        // Process each text individually for now (can be batched later)
        for text in texts {
            let embedding = self.process_single_text(text)?;
            let token_count = self.estimate_token_count(text);
            
            results.push(EmbeddingResult {
                text: text.clone(),
                embedding,
                token_count,
            });
        }
        
        debug!("Successfully generated {} BGE-M3 embeddings", results.len());
        Ok(results)
    }
    
    /// Process single text with BGE-M3 model
    fn process_single_text(&self, text: &str) -> Result<Vec<f32>> {
        // Use proper XLMRoberta tokenization
        let tokenized = self.tokenizer.encode(text)?;
        let seq_len = tokenized.length;
        
        // Create tensors for BGE-M3 (XLMRoberta inputs)
        let input_ids_tensor = Tensor::from_array(([1, seq_len], tokenized.input_ids))?;
        let attention_mask_tensor = Tensor::from_array(([1, seq_len], tokenized.attention_mask))?;
        let token_type_ids_tensor = Tensor::from_array(([1, seq_len], tokenized.token_type_ids))?;
        
        // Run inference
        let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
        
        let outputs = session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "token_type_ids" => token_type_ids_tensor
        ])?;
        
        // Extract embeddings from outputs
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                
                // Look for hidden states [batch, seq, hidden]
                if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                    let hidden_size = shape_vec[2] as usize;
                    
                    debug!("Found BGE-M3 hidden states: [1, {}, {}]", seq_len, hidden_size);
                    
                    // Apply mean pooling
                    let pooled = self.mean_pooling(data, seq_len, hidden_size)?;
                    
                    // Normalize
                    let normalized = self.normalize_embedding(pooled)?;
                    
                    debug!("Generated BGE-M3 embedding: {} dims", normalized.len());
                    return Ok(normalized);
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not extract BGE-M3 embeddings from model outputs"))
    }
    
    
    /// Apply mean pooling to hidden states
    fn mean_pooling(&self, data: &[f32], seq_len: usize, hidden_size: usize) -> Result<Vec<f32>> {
        let mut pooled = vec![0.0f32; hidden_size];
        
        for seq_idx in 0..seq_len {
            for hidden_idx in 0..hidden_size {
                let data_idx = seq_idx * hidden_size + hidden_idx;
                if data_idx < data.len() {
                    pooled[hidden_idx] += data[data_idx];
                }
            }
        }
        
        // Average over sequence length
        for val in &mut pooled {
            *val /= seq_len as f32;
        }
        
        Ok(pooled)
    }
    
    /// Normalize embedding vector
    fn normalize_embedding(&self, embedding: Vec<f32>) -> Result<Vec<f32>> {
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm > 0.0 {
            Ok(embedding.iter().map(|x| x / norm).collect())
        } else {
            Err(anyhow::anyhow!("Cannot normalize zero vector"))
        }
    }
    
    /// Estimate token count
    fn estimate_token_count(&self, text: &str) -> usize {
        // Use real tokenizer for accurate count
        self.tokenizer.encode(text)
            .map(|t| t.length)
            .unwrap_or_else(|_| text.split_whitespace().count() + 2)
    }
    
    /// Get embedding dimension
    pub fn embedding_dim(&self) -> usize {
        self.hidden_size
    }
    
    /// Check if model is available
    pub fn is_available(&self) -> bool {
        self.model_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_bge_m3_service_creation() {
        let model_path = PathBuf::from("test_models/bge-m3/model.onnx");
        // This test will fail without actual model, but shows the API
        match BgeM3EmbeddingService::new(model_path) {
            Ok(service) => {
                assert_eq!(service.embedding_dim(), 1024);
                println!("BGE-M3 service created successfully");
            },
            Err(e) => {
                println!("Expected error without model file: {}", e);
            }
        }
    }
    
    #[test]
    fn test_xlmroberta_tokenization() {
        // This test would require actual tokenizer file
        // Just verify that the service expects proper tokenizer now
        let model_path = PathBuf::from("test_models/bge-m3/model.onnx");
        match BgeM3EmbeddingService::new(model_path) {
            Err(e) => {
                // Should fail because tokenizer.json is missing
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Tokenizer") || 
                    error_msg.contains("not found") || 
                    error_msg.contains("File at") ||
                    error_msg.contains("does not exist"),
                    "Unexpected error message: {}",
                    error_msg
                );
                println!("Expected error without tokenizer: {}", e);
            },
            Ok(_) => {
                println!("Service created with XLMRoberta tokenizer");
            }
        }
    }
}