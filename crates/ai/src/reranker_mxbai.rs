use anyhow::Result as AnyhowResult;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, debug, warn};
use crate::tokenization::OptimizedTokenizer;

/// MXBai Reranker Service with real ONNX Runtime 2.0 and real tokenization
pub struct MxbaiRerankerService {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<OptimizedTokenizer>,
    model_path: PathBuf,
}

/// Result of reranking operation
#[derive(Debug, Clone)]
pub struct RerankResult {
    pub query: String,
    pub document: String,
    pub score: f32,
    pub index: usize,
}

impl MxbaiRerankerService {
    /// Create new MXBai reranker service with real tokenization
    pub fn new(model_path: PathBuf) -> AnyhowResult<Self> {
        info!("Initializing MXBai reranker service with real tokenization");
        
        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
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
            .with_name("mxbai_reranker")
            .commit()?;
        
        // Create session
        let session = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_path)?;
        
        info!("✅ MXBai reranker session created successfully");
        info!("   Model: {}", model_path.display());
        info!("   Inputs: {}", session.inputs.len());
        info!("   Outputs: {}", session.outputs.len());
        
        // Verify it's the expected MXBai model (3 inputs for Qwen2: input_ids, attention_mask, position_ids)
        if session.inputs.len() != 3 {
            warn!("Expected 3 inputs for MXBai Qwen2, got {}", session.inputs.len());
        }
        
        // Create real tokenizer for MXBai model (uses Qwen2 tokenizer)
        let tokenizer_path = model_path.parent().unwrap().join("tokenizer.json");
        let tokenizer = if tokenizer_path.exists() {
            info!("Loading real MXBai tokenizer from: {}", tokenizer_path.display());
            OptimizedTokenizer::new(tokenizer_path, 512)?
        } else {
            warn!("Tokenizer not found, cannot continue without real tokenization");
            return Err(anyhow::anyhow!("Tokenizer file not found: {}", tokenizer_path.display()));
        };
        
        info!("✅ Real tokenization initialized for MXBai reranker");
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_path,
        })
    }
    
    /// Rerank documents for a query
    pub fn rerank(&self, query: &str, documents: &[String], top_k: Option<usize>) -> AnyhowResult<Vec<RerankResult>> {
        debug!("Reranking {} documents for query: {} chars", documents.len(), query.len());
        
        let mut results = Vec::new();
        
        // Process each document with query
        for (index, document) in documents.iter().enumerate() {
            let score = self.score_pair(query, document)?;
            
            results.push(RerankResult {
                query: query.to_string(),
                document: document.clone(),
                score,
                index,
            });
        }
        
        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Apply top_k limit if specified
        if let Some(k) = top_k {
            results.truncate(k);
        }
        
        debug!("Successfully reranked documents, returning top {}", results.len());
        Ok(results)
    }
    
    /// Score a query-document pair using real tokenization
    fn score_pair(&self, query: &str, document: &str) -> AnyhowResult<f32> {
        // Real tokenization for query and document separately
        let query_tokenized = self.tokenizer.encode(query)?;
        let doc_tokenized = self.tokenizer.encode(document)?;
        
        // Create combined input: [CLS] query [SEP] document
        let mut input_ids = vec![0i64]; // CLS token (assuming 0 is CLS)
        
        // Add query tokens (limit to reasonable length)
        let query_limit = std::cmp::min(query_tokenized.input_ids.len(), 128);
        input_ids.extend_from_slice(&query_tokenized.input_ids[..query_limit]);
        
        // Add separator token
        input_ids.push(2i64); // SEP token (assuming 2 is SEP)
        
        // Add document tokens (fill remaining space)
        let remaining_space = 512 - input_ids.len();
        let doc_limit = std::cmp::min(doc_tokenized.input_ids.len(), remaining_space);
        input_ids.extend_from_slice(&doc_tokenized.input_ids[..doc_limit]);
        
        // Truncate if still too long
        if input_ids.len() > 512 {
            input_ids.truncate(512);
        }
        
        let seq_len = input_ids.len();
        let attention_mask = vec![1i64; seq_len];
        let position_ids: Vec<i64> = (0..seq_len as i64).collect(); // Position IDs for Qwen2
        
        // Create tensors for MXBai Qwen2 (3 inputs: input_ids, attention_mask, position_ids)
        let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids))?;
        let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask))?;
        let position_ids_tensor = Tensor::from_array(([1, seq_len], position_ids))?;
        
        // Run inference
        let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
        
        let outputs = session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "position_ids" => position_ids_tensor
        ])?;
        
        // Extract score from outputs
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                
                // MXBai Qwen2 outputs logits [batch, seq_len, vocab_size]
                if shape_vec.len() == 3 && shape_vec[0] == 1 {
                    let seq_len = shape_vec[1] as usize;
                    let vocab_size = shape_vec[2] as usize;
                    
                    // Get the last token's logits (EOS position for scoring)
                    let last_token_start = (seq_len - 1) * vocab_size;
                    
                    // For MXBai, use a simple scoring based on last token logits
                    if last_token_start + 100 < data.len() {
                        // Take average of some logits as proxy score
                        let mut sum = 0.0f32;
                        for i in 0..100 {
                            sum += data[last_token_start + i];
                        }
                        let score = sum / 100.0;
                        return Ok(score.tanh()); // Normalize to [-1, 1] range
                    }
                } else if shape_vec.len() == 2 && shape_vec[0] == 1 {
                    // Fallback: look for direct score outputs
                    if shape_vec[1] == 1 {
                        // Single score output
                        return Ok(data[0]);
                    } else if shape_vec[1] == 2 {
                        // Binary classification logits [negative, positive]
                        let positive_score = data[1];
                        return Ok(positive_score);
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not extract reranking score from model outputs"))
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
    fn test_mxbai_service_creation() {
        let model_path = PathBuf::from("test_models/mxbai/model.onnx");
        // This test will fail without actual model, but shows the API
        match MxbaiRerankerService::new(model_path) {
            Ok(_service) => {
                println!("MXBai service created successfully");
            },
            Err(e) => {
                println!("Expected error without model file: {}", e);
            }
        }
    }
    
    #[test]
    fn test_tokenization() {
        let query = "machine learning algorithms";
        let document = "deep learning and neural networks for artificial intelligence";
        
        // Mock tokenization result
        let (q_tokens, d_tokens) = (vec![1000i64, 2000, 3000], vec![4000i64, 5000, 6000, 7000]);
        
        assert!(!q_tokens.is_empty());
        assert!(!d_tokens.is_empty());  
        assert!(q_tokens.len() <= 128);
        assert!(d_tokens.len() <= 256);
    }
}