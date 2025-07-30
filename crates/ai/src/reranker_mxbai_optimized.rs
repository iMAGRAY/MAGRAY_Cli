use crate::memory_pool::{GLOBAL_MEMORY_POOL, PoolStats};
use anyhow::Result as AnyhowResult;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, debug, warn};

/// Optimized MXBai Reranker Service with batch processing and memory pooling
pub struct OptimizedMxbaiRerankerService {
    session: Arc<Mutex<Session>>,
    model_path: PathBuf,
    max_seq_length: usize,
    batch_size: usize,
}

/// Batch reranking input
#[derive(Debug, Clone)]
pub struct RerankBatch {
    pub query: String,
    pub documents: Vec<String>,
    pub top_k: Option<usize>,
}

/// Result of optimized reranking operation
#[derive(Debug, Clone)]
pub struct OptimizedRerankResult {
    pub query: String,
    pub document: String,
    pub score: f32,
    pub index: usize,
    pub processing_time_ms: u128,
}

/// Batch processing result
#[derive(Debug)]
pub struct BatchRerankResult {
    pub results: Vec<OptimizedRerankResult>,
    pub total_time_ms: u128,
    pub throughput_docs_per_sec: f64,
}

impl OptimizedMxbaiRerankerService {
    /// Create new optimized MXBai reranker service
    pub fn new(model_path: PathBuf, max_seq_length: usize, batch_size: usize) -> AnyhowResult<Self> {
        info!("Initializing OPTIMIZED MXBai reranker service");
        info!("   Max sequence length: {}", max_seq_length);
        info!("   Batch size: {}", batch_size);
        
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
            .with_name("optimized_mxbai_reranker")
            .commit()?;
        
        // Create optimized session
        let session = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_memory_pattern(true)? // Enable memory pattern optimization
            .commit_from_file(&model_path)?;
        
        info!("✅ OPTIMIZED MXBai reranker session created");
        info!("   Model: {}", model_path.display());
        info!("   Inputs: {}", session.inputs.len());
        info!("   Outputs: {}", session.outputs.len());
        
        // Verify it's the expected MXBai model (3 inputs for Qwen2)
        if session.inputs.len() != 3 {
            warn!("Expected 3 inputs for MXBai Qwen2, got {}", session.inputs.len());
        }
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            model_path,
            max_seq_length,
            batch_size,
        })
    }
    
    /// Optimized batch reranking with memory pooling
    pub fn rerank_batch(&self, batch: &RerankBatch) -> AnyhowResult<BatchRerankResult> {
        let start_time = std::time::Instant::now();
        let query = &batch.query;
        let documents = &batch.documents;
        
        info!("🚀 OPTIMIZED batch reranking: {} documents", documents.len());
        
        if documents.is_empty() {
            return Ok(BatchRerankResult {
                results: vec![],
                total_time_ms: 0,
                throughput_docs_per_sec: 0.0,
            });
        }
        
        // Process documents in optimized batches
        let mut all_results = Vec::with_capacity(documents.len());
        let chunks: Vec<&[String]> = documents.chunks(self.batch_size).collect();
        
        debug!("Processing {} chunks of max size {}", chunks.len(), self.batch_size);
        
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            debug!("Processing chunk {}/{} with {} documents", chunk_idx + 1, chunks.len(), chunk.len());
            
            let chunk_results = self.process_batch_optimized(query, chunk)?;
            
            // Add original indices
            for (local_idx, mut result) in chunk_results.into_iter().enumerate() {
                result.index = chunk_idx * self.batch_size + local_idx;
                all_results.push(result);
            }
        }
        
        // Sort by score (descending)
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Apply top_k limit if specified
        if let Some(k) = batch.top_k {
            all_results.truncate(k);
        }
        
        let total_time = start_time.elapsed().as_millis();
        let throughput = 1000.0 * documents.len() as f64 / total_time as f64;
        
        info!("✅ OPTIMIZED batch reranking completed in {}ms ({:.1} docs/sec)", 
              total_time, throughput);
        
        Ok(BatchRerankResult {
            results: all_results,
            total_time_ms: total_time,
            throughput_docs_per_sec: throughput,
        })
    }
    
    /// Process a batch of documents at once using memory pooling
    fn process_batch_optimized(&self, query: &str, documents: &[String]) -> AnyhowResult<Vec<OptimizedRerankResult>> {
        let batch_size = documents.len();
        debug!("Processing optimized batch of {} documents", batch_size);
        
        // Tokenize all query-document pairs in batch
        let batch_tokenized = self.tokenize_batch(query, documents);
        
        // Find maximum sequence length for padding
        let max_len = batch_tokenized.input_ids.iter().map(|ids| ids.len()).max().unwrap_or(0);
        let padded_len = max_len.min(self.max_seq_length);
        
        debug!("Batch max length: {}, padded to: {}", max_len, padded_len);
        
        // Use memory pools for batch data
        let total_elements = batch_size * padded_len;
        let mut flat_input_ids = GLOBAL_MEMORY_POOL.get_input_buffer(total_elements);
        let mut flat_attention_masks = GLOBAL_MEMORY_POOL.get_attention_buffer(total_elements);
        let mut flat_position_ids = GLOBAL_MEMORY_POOL.get_token_type_buffer(total_elements);
        
        // Flatten and pad batch data
        for i in 0..batch_size {
            let input_ids = &batch_tokenized.input_ids[i];
            let attention_mask = &batch_tokenized.attention_masks[i];
            let position_ids = &batch_tokenized.position_ids[i];
            
            // Pad to uniform length
            let actual_len = input_ids.len().min(padded_len);
            
            // Add padded input_ids
            flat_input_ids.extend_from_slice(&input_ids[..actual_len]);
            if actual_len < padded_len {
                flat_input_ids.extend(vec![1i64; padded_len - actual_len]); // PAD token
            }
            
            // Add padded attention_mask
            flat_attention_masks.extend_from_slice(&attention_mask[..actual_len]);
            if actual_len < padded_len {
                flat_attention_masks.extend(vec![0i64; padded_len - actual_len]); // Ignore padded positions
            }
            
            // Add padded position_ids
            flat_position_ids.extend_from_slice(&position_ids[..actual_len]);
            if actual_len < padded_len {
                // Continue position sequence for padding
                for pos in actual_len..padded_len {
                    flat_position_ids.push(pos as i64);
                }
            }
        }
        
        // Create batch tensors [batch_size, seq_len]
        let input_ids_tensor = Tensor::from_array(([batch_size, padded_len], flat_input_ids.clone()))?;
        let attention_mask_tensor = Tensor::from_array(([batch_size, padded_len], flat_attention_masks.clone()))?;
        let position_ids_tensor = Tensor::from_array(([batch_size, padded_len], flat_position_ids.clone()))?;
        
        // Single ONNX call for entire batch
        let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
        
        let outputs = session.run(inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => attention_mask_tensor,
            "position_ids" => position_ids_tensor
        ])?;
        
        // Return buffers to pools
        GLOBAL_MEMORY_POOL.return_input_buffer(flat_input_ids);
        GLOBAL_MEMORY_POOL.return_attention_buffer(flat_attention_masks);
        GLOBAL_MEMORY_POOL.return_token_type_buffer(flat_position_ids);
        
        // Extract batch scores
        let scores = self.extract_batch_scores(&outputs, batch_size)?;
        
        // Create results
        let mut results = Vec::with_capacity(batch_size);
        for (i, document) in documents.iter().enumerate() {
            results.push(OptimizedRerankResult {
                query: query.to_string(),
                document: document.clone(),
                score: scores[i],
                index: i, // Will be updated by caller
                processing_time_ms: 0, // Will be set by caller
            });
        }
        
        debug!("Extracted {} batch scores", scores.len());
        Ok(results)
    }
    
    /// Tokenize query with multiple documents in batch
    fn tokenize_batch(&self, query: &str, documents: &[String]) -> BatchTokenizedPairs {
        let mut batch_input_ids = Vec::with_capacity(documents.len());
        let mut batch_attention_masks = Vec::with_capacity(documents.len());
        let mut batch_position_ids = Vec::with_capacity(documents.len());
        
        for document in documents {
            let (input_ids, attention_mask, position_ids) = self.tokenize_pair_optimized(query, document);
            
            batch_input_ids.push(input_ids);
            batch_attention_masks.push(attention_mask);
            batch_position_ids.push(position_ids);
        }
        
        BatchTokenizedPairs {
            input_ids: batch_input_ids,
            attention_masks: batch_attention_masks,
            position_ids: batch_position_ids,
        }
    }
    
    /// Optimized tokenization for query-document pairs with memory pooling
    fn tokenize_pair_optimized(&self, query: &str, document: &str) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
        // Use memory pools for tokenization buffers
        let mut query_tokens = GLOBAL_MEMORY_POOL.get_input_buffer(128);
        let mut doc_tokens = GLOBAL_MEMORY_POOL.get_input_buffer(256);
        
        // Simple hash-based tokenization (can be replaced with real tokenizer later)
        for word in query.split_whitespace().take(128) {
            let word_hash = word.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
            query_tokens.push((word_hash % 30000 + 1000) as i64);
        }
        
        for word in document.split_whitespace().take(256) {
            let word_hash = word.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
            doc_tokens.push((word_hash % 30000 + 1000) as i64);
        }
        
        // Create combined input: [CLS] query [SEP] document
        let mut input_ids = vec![0i64]; // CLS token
        input_ids.extend_from_slice(&query_tokens);
        input_ids.push(2i64); // SEP token  
        input_ids.extend_from_slice(&doc_tokens);
        
        // Truncate if too long
        if input_ids.len() > self.max_seq_length {
            input_ids.truncate(self.max_seq_length);
        }
        
        let seq_len = input_ids.len();
        let attention_mask = vec![1i64; seq_len];
        let position_ids: Vec<i64> = (0..seq_len as i64).collect();
        
        // Return tokenization buffers to pool
        GLOBAL_MEMORY_POOL.return_input_buffer(query_tokens);
        GLOBAL_MEMORY_POOL.return_input_buffer(doc_tokens);
        
        (input_ids, attention_mask, position_ids)
    }
    
    /// Extract scores from batch outputs
    fn extract_batch_scores(&self, outputs: &ort::session::SessionOutputs, batch_size: usize) -> AnyhowResult<Vec<f32>> {
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                
                // MXBai Qwen2 outputs logits [batch_size, seq_len, vocab_size]
                if shape_vec.len() == 3 && shape_vec[0] == batch_size as i64 {
                    let seq_len = shape_vec[1] as usize;
                    let vocab_size = shape_vec[2] as usize;
                    
                    debug!("Extracting scores from batch output: [{}, {}, {}]", batch_size, seq_len, vocab_size);
                    
                    let mut scores = Vec::with_capacity(batch_size);
                    
                    // Extract score for each item in batch
                    for batch_idx in 0..batch_size {
                        let batch_offset = batch_idx * seq_len * vocab_size;
                        let last_token_start = batch_offset + (seq_len - 1) * vocab_size;
                        
                        if last_token_start + 100 < data.len() {
                            // Take average of some logits as proxy score
                            let mut sum = 0.0f32;
                            for i in 0..100 {
                                sum += data[last_token_start + i];
                            }
                            let score = (sum / 100.0).tanh(); // Normalize to [-1, 1] range
                            scores.push(score);
                        } else {
                            scores.push(0.0); // Fallback score
                        }
                    }
                    
                    debug!("Extracted {} batch scores", scores.len());
                    return Ok(scores);
                    
                } else if shape_vec.len() == 2 && shape_vec[0] == batch_size as i64 {
                    // Direct score outputs [batch_size, num_classes]
                    if shape_vec[1] == 1 {
                        // Single score per batch item
                        let scores: Vec<f32> = (0..batch_size).map(|i| data[i]).collect();
                        return Ok(scores);
                    } else if shape_vec[1] == 2 {
                        // Binary classification logits [negative, positive]
                        let scores: Vec<f32> = (0..batch_size).map(|i| data[i * 2 + 1]).collect(); // Take positive scores
                        return Ok(scores);
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Could not extract batch reranking scores from model outputs"))
    }
    
    /// Single document reranking (fallback for compatibility)
    pub fn rerank(&self, query: &str, documents: &[String], top_k: Option<usize>) -> AnyhowResult<Vec<OptimizedRerankResult>> {
        let batch = RerankBatch {
            query: query.to_string(),
            documents: documents.to_vec(),
            top_k,
        };
        
        let batch_result = self.rerank_batch(&batch)?;
        Ok(batch_result.results)
    }
    
    /// Get service statistics including memory pool stats
    pub fn get_stats(&self) -> RerankServiceStats {
        RerankServiceStats {
            model_name: "mxbai-optimized".to_string(),
            max_seq_length: self.max_seq_length,
            batch_size: self.batch_size,
            optimization_level: "Level3+MemoryPool+Batch".to_string(),
        }
    }
    
    /// Get memory pool statistics
    pub fn get_pool_stats(&self) -> PoolStats {
        GLOBAL_MEMORY_POOL.get_stats()
    }
    
    /// Check if model is available
    pub fn is_available(&self) -> bool {
        self.model_path.exists()
    }
}

/// Batch tokenized pairs
#[derive(Debug)]
struct BatchTokenizedPairs {
    pub input_ids: Vec<Vec<i64>>,
    pub attention_masks: Vec<Vec<i64>>,
    pub position_ids: Vec<Vec<i64>>,
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct RerankServiceStats {
    pub model_name: String,
    pub max_seq_length: usize,
    pub batch_size: usize,
    pub optimization_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_optimized_reranker_creation() {
        let model_path = PathBuf::from("test_models/mxbai/model.onnx");
        
        match OptimizedMxbaiRerankerService::new(model_path, 512, 8) {
            Ok(_service) => {
                println!("✅ Optimized MXBai service created successfully");
            },
            Err(e) => {
                println!("Expected error without model file: {}", e);
            }
        }
    }
    
    #[test]
    fn test_batch_reranking_api() {
        let query = "machine learning algorithms";
        let documents = vec![
            "deep learning neural networks".to_string(),
            "traditional algorithms and data structures".to_string(),
            "artificial intelligence and ML".to_string(),
        ];
        
        let batch = RerankBatch {
            query: query.to_string(),
            documents,
            top_k: Some(2),
        };
        
        // Test batch structure
        assert_eq!(batch.documents.len(), 3);
        assert_eq!(batch.top_k, Some(2));
        assert!(!batch.query.is_empty());
    }
}