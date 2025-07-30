use anyhow::Result;
use tokenizers::Tokenizer;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, debug};

/// Optimized tokenizer with buffer reuse and batch processing
pub struct OptimizedTokenizer {
    inner: Arc<Tokenizer>,
    max_length: usize,
    model_name: String,
}

/// Tokenization result with all necessary data
#[derive(Debug, Clone)]
pub struct TokenizedInput {
    pub input_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
    pub token_type_ids: Vec<i64>,
    pub length: usize,
}

/// Batch tokenization result
#[derive(Debug)]
pub struct BatchTokenized {
    pub input_ids: Vec<Vec<i64>>,
    pub attention_masks: Vec<Vec<i64>>,
    pub token_type_ids: Vec<Vec<i64>>,
    pub lengths: Vec<usize>,
    pub max_length: usize,
}

impl OptimizedTokenizer {
    /// Create new optimized tokenizer
    pub fn new(tokenizer_path: impl AsRef<Path>, max_length: usize) -> Result<Self> {
        let tokenizer_path = tokenizer_path.as_ref();
        info!("Loading optimized tokenizer from: {}", tokenizer_path.display());
        
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
            
        info!("✅ Optimized tokenizer loaded successfully");
        info!("   Vocab size: {}", tokenizer.get_vocab_size(true));
        info!("   Max length: {}", max_length);
        
        Ok(Self {
            inner: Arc::new(tokenizer),
            max_length,
            model_name: "bge-m3".to_string(),
        })
    }
    
    /// Tokenize single text with proper XLMRoberta tokenization
    pub fn encode(&self, text: &str) -> Result<TokenizedInput> {
        debug!("Tokenizing text: {} chars", text.len());
        
        // Use real tokenizer instead of hash-based approach
        let encoding = self.inner
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;
        
        let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mut attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();
        
        // Truncate if necessary
        if input_ids.len() > self.max_length {
            debug!("Truncating from {} to {} tokens", input_ids.len(), self.max_length);
            input_ids.truncate(self.max_length);
            attention_mask.truncate(self.max_length);
            
            // Ensure we end with EOS token for BGE-M3
            if let Some(eos_id) = self.get_eos_token_id() {
                if !input_ids.is_empty() {
                    let last_idx = input_ids.len() - 1;
                    input_ids[last_idx] = eos_id;
                }
            }
        }
        
        // BGE-M3 uses XLMRoberta which needs token_type_ids (all zeros for single sequence)
        let token_type_ids = vec![0i64; input_ids.len()];
        let length = input_ids.len();
        
        debug!("Tokenized to {} tokens", length);
        
        Ok(TokenizedInput {
            input_ids,
            attention_mask,
            token_type_ids,
            length,
        })
    }
    
    /// Batch tokenization - much more efficient than one-by-one
    pub fn encode_batch(&self, texts: &[&str]) -> Result<BatchTokenized> {
        if texts.is_empty() {
            return Ok(BatchTokenized {
                input_ids: vec![],
                attention_masks: vec![],
                token_type_ids: vec![],
                lengths: vec![],
                max_length: 0,
            });
        }
        
        debug!("Batch tokenizing {} texts", texts.len());
        
        // Batch tokenization is much faster than individual calls
        let encodings = self.inner
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| anyhow::anyhow!("Batch tokenization failed: {}", e))?;
        
        let mut batch_input_ids = Vec::with_capacity(texts.len());
        let mut batch_attention_masks = Vec::with_capacity(texts.len());
        let mut batch_token_type_ids = Vec::with_capacity(texts.len());
        let mut batch_lengths = Vec::with_capacity(texts.len());
        let mut max_seq_len = 0;
        
        for encoding in encodings {
            let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
            let mut attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();
            
            // Truncate if necessary
            if input_ids.len() > self.max_length {
                input_ids.truncate(self.max_length);
                attention_mask.truncate(self.max_length);
                
                // Ensure EOS token
                if let Some(eos_id) = self.get_eos_token_id() {
                    if !input_ids.is_empty() {
                        let last_idx = input_ids.len() - 1;
                        input_ids[last_idx] = eos_id;
                    }
                }
            }
            
            let token_type_ids = vec![0i64; input_ids.len()];
            let length = input_ids.len();
            
            max_seq_len = max_seq_len.max(length);
            
            batch_input_ids.push(input_ids);
            batch_attention_masks.push(attention_mask);
            batch_token_type_ids.push(token_type_ids);
            batch_lengths.push(length);
        }
        
        debug!("Batch tokenized: {} texts, max_len: {}", texts.len(), max_seq_len);
        
        Ok(BatchTokenized {
            input_ids: batch_input_ids,
            attention_masks: batch_attention_masks,
            token_type_ids: batch_token_type_ids,
            lengths: batch_lengths,
            max_length: max_seq_len,
        })
    }
    
    /// Pad batch to uniform length for efficient ONNX processing
    pub fn pad_batch(&self, batch: &mut BatchTokenized, target_length: Option<usize>) -> Result<()> {
        let pad_length = target_length.unwrap_or(batch.max_length);
        let pad_token_id = self.get_pad_token_id().unwrap_or(1); // Use pad token or 1
        
        debug!("Padding batch to length: {}", pad_length);
        
        for i in 0..batch.input_ids.len() {
            let current_len = batch.input_ids[i].len();
            
            if current_len < pad_length {
                let pad_count = pad_length - current_len;
                
                // Pad input_ids with pad token
                batch.input_ids[i].extend(vec![pad_token_id; pad_count]);
                
                // Pad attention_mask with zeros (ignore padded tokens)
                batch.attention_masks[i].extend(vec![0i64; pad_count]);
                
                // Pad token_type_ids with zeros
                batch.token_type_ids[i].extend(vec![0i64; pad_count]);
            }
        }
        
        batch.max_length = pad_length;
        Ok(())
    }
    
    /// Get special token IDs
    fn get_eos_token_id(&self) -> Option<i64> {
        self.inner.token_to_id("</s>").map(|id| id as i64)
    }
    
    fn get_pad_token_id(&self) -> Option<i64> {
        self.inner.token_to_id("<pad>").map(|id| id as i64)
    }
    
    
    /// Get tokenizer info
    pub fn vocab_size(&self) -> usize {
        self.inner.get_vocab_size(true)
    }
    
    pub fn max_length(&self) -> usize {
        self.max_length
    }
    
    pub fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Helper function to create optimized tokenizer for BGE-M3
pub fn create_bge_m3_tokenizer(models_dir: impl AsRef<Path>) -> Result<OptimizedTokenizer> {
    let tokenizer_path = models_dir.as_ref().join("bge-m3").join("tokenizer.json");
    OptimizedTokenizer::new(tokenizer_path, 512)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_optimized_tokenizer() {
        // This test requires the actual tokenizer file
        let tokenizer_path = PathBuf::from("../memory/models/bge-m3/tokenizer.json");
        
        if tokenizer_path.exists() {
            let tokenizer = OptimizedTokenizer::new(tokenizer_path, 512).unwrap();
            
            let result = tokenizer.encode("Hello world, this is a test").unwrap();
            
            assert!(!result.input_ids.is_empty());
            assert_eq!(result.input_ids.len(), result.attention_mask.len());
            assert_eq!(result.input_ids.len(), result.token_type_ids.len());
            assert_eq!(result.length, result.input_ids.len());
            
            println!("Tokenized: {:?}", result);
        }
    }
    
    #[test]
    fn test_batch_tokenization() {
        let tokenizer_path = PathBuf::from("../memory/models/bge-m3/tokenizer.json");
        
        if tokenizer_path.exists() {
            let tokenizer = OptimizedTokenizer::new(tokenizer_path, 512).unwrap();
            
            let texts = vec![
                "First test text",
                "Second longer test text with more words",
                "Third text"
            ];
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            
            let mut batch = tokenizer.encode_batch(&text_refs).unwrap();
            
            assert_eq!(batch.input_ids.len(), 3);
            assert_eq!(batch.attention_masks.len(), 3);
            assert_eq!(batch.token_type_ids.len(), 3);
            
            // Test padding
            tokenizer.pad_batch(&mut batch, Some(20)).unwrap();
            
            for ids in &batch.input_ids {
                assert_eq!(ids.len(), 20);
            }
            
            println!("Batch tokenized and padded successfully");
        }
    }
}