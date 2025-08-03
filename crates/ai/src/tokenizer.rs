use crate::{AiError, Result};
use tokenizers::Tokenizer;
use std::path::Path;
use tracing::{info, debug};

/// Tokenizer wrapper for ONNX models
pub struct TokenizerService {
    tokenizer: Tokenizer,
    max_length: usize,
}

impl TokenizerService {
    /// Load tokenizer from file
    pub fn from_file(tokenizer_path: impl AsRef<Path>, max_length: usize) -> Result<Self> {
        let tokenizer_path = tokenizer_path.as_ref();
        
        if !tokenizer_path.exists() {
            return Err(AiError::TokenizerError(format!(
                "Tokenizer file not found: {}",
                tokenizer_path.display()
            )));
        }
        
        debug!("Loading tokenizer from: {}", tokenizer_path.display());
        
        let tokenizer = Tokenizer::from_file(tokenizer_path)?;
        
        info!("Successfully loaded tokenizer with max_length: {}", max_length);
        
        Ok(Self {
            tokenizer,
            max_length,
        })
    }
    
    /// Create a default tokenizer for testing
    pub fn new_default(max_length: usize) -> Result<Self> {
        use tokenizers::models::bpe::BPE;
        use tokenizers::normalizers::BertNormalizer;
        use tokenizers::pre_tokenizers::bert::BertPreTokenizer;
        use tokenizers::processors::template::TemplateProcessing;
        
        let mut tokenizer = Tokenizer::new(BPE::default());
        
        // Basic normalizer
        tokenizer.with_normalizer(Some(BertNormalizer::default()));
        
        // Basic pre-tokenizer
        tokenizer.with_pre_tokenizer(Some(BertPreTokenizer));
        
        // Basic post-processor
        let special_tokens = vec![
            ("[CLS]".to_string(), 101),
            ("[SEP]".to_string(), 102),
            ("[PAD]".to_string(), 0),
            ("[UNK]".to_string(), 100),
        ];
        
        let post_processor = TemplateProcessing::builder()
            .try_single("[CLS] $A [SEP]")
            .unwrap()
            .try_pair("[CLS] $A [SEP] $B:1 [SEP]:1")
            .unwrap()
            .special_tokens(special_tokens)
            .build()
            .unwrap();
            
        tokenizer.with_post_processor(Some(post_processor));
        
        info!("Created default tokenizer for testing");
        
        Ok(Self {
            tokenizer,
            max_length,
        })
    }
    
    /// Tokenize a single text
    pub fn encode(&self, text: &str) -> Result<TokenizedInput> {
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| AiError::TokenizerError(e.to_string()))?;
        
        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();
        
        // Truncate if too long
        if input_ids.len() > self.max_length {
            input_ids.truncate(self.max_length);
            attention_mask.truncate(self.max_length);
        }
        
        // Pad if too short
        while input_ids.len() < self.max_length {
            input_ids.push(0); // PAD token
            attention_mask.push(0);
        }
        
        Ok(TokenizedInput {
            input_ids,
            attention_mask,
            length: encoding.len(),
        })
    }
    
    /// Tokenize multiple texts in batch
    pub fn encode_batch(&self, texts: &[String]) -> Result<Vec<TokenizedInput>> {
        texts.iter()
            .map(|text| self.encode(text))
            .collect()
    }
    
    /// Get vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.tokenizer.get_vocab_size(true)
    }
    
    /// Get special tokens
    pub fn get_special_tokens(&self) -> SpecialTokens {
        let vocab = self.tokenizer.get_vocab(true);
        
        SpecialTokens {
            pad_token_id: vocab.get("[PAD]").copied().unwrap_or(0),
            cls_token_id: vocab.get("[CLS]").copied().unwrap_or(101),
            sep_token_id: vocab.get("[SEP]").copied().unwrap_or(102),
            unk_token_id: vocab.get("[UNK]").copied().unwrap_or(100),
        }
    }
}

/// Tokenized input ready for ONNX model
#[derive(Debug, Clone)]
pub struct TokenizedInput {
    pub input_ids: Vec<u32>,
    pub attention_mask: Vec<u32>,
    pub length: usize,
}

/// Special token IDs
#[derive(Debug, Clone)]
pub struct SpecialTokens {
    pub pad_token_id: u32,
    pub cls_token_id: u32,
    pub sep_token_id: u32,
    pub unk_token_id: u32,
}