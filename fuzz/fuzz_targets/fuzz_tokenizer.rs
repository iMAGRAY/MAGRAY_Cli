#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use ai::{
    tokenizer::{TokenizeConfig, TokenizerManager, TokenizerType},
    tokenization::simple_qwen3::SimpleQwen3Tokenizer,
};

#[derive(Debug, Arbitrary)]
struct TokenizerFuzzInput {
    text: String,
    config: TokenizeConfigFuzz,
    tokenizer_type: TokenizerTypeFuzz,
}

#[derive(Debug, Arbitrary)]
struct TokenizeConfigFuzz {
    max_length: Option<u16>, // Reduced range for fuzzing
    truncation: bool,
    padding: bool,
    add_special_tokens: bool,
    return_attention_mask: bool,
    return_token_type_ids: bool,
}

#[derive(Debug, Arbitrary)]
enum TokenizerTypeFuzz {
    Qwen3,
    BgeM3,
}

impl From<TokenizeConfigFuzz> for TokenizeConfig {
    fn from(fuzz_config: TokenizeConfigFuzz) -> Self {
        TokenizeConfig {
            max_length: fuzz_config.max_length.map(|x| x as usize),
            truncation: fuzz_config.truncation,
            padding: fuzz_config.padding,
            add_special_tokens: fuzz_config.add_special_tokens,
            return_attention_mask: fuzz_config.return_attention_mask,
            return_token_type_ids: fuzz_config.return_token_type_ids,
        }
    }
}

impl From<TokenizerTypeFuzz> for TokenizerType {
    fn from(fuzz_type: TokenizerTypeFuzz) -> Self {
        match fuzz_type {
            TokenizerTypeFuzz::Qwen3 => TokenizerType::Qwen3,
            TokenizerTypeFuzz::BgeM3 => TokenizerType::BgeM3,
        }
    }
}

fuzz_target!(|input: TokenizerFuzzInput| {
    // Create a simple runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    
    rt.block_on(async {
        // Test different tokenizer creation patterns
        match input.tokenizer_type {
            TokenizerTypeFuzz::Qwen3 => {
                // Test SimpleQwen3Tokenizer directly
                if let Ok(tokenizer) = SimpleQwen3Tokenizer::new() {
                    let config = TokenizeConfig::from(input.config);
                    
                    // Fuzz tokenization - should never panic
                    let _ = tokenizer.tokenize_with_config(&input.text, Some(config));
                    
                    // Fuzz basic encode/decode operations
                    if let Ok(encoded) = tokenizer.encode(&input.text) {
                        // Encoded tokens should be reasonable
                        if !encoded.is_empty() && encoded.len() < 10000 {
                            let _ = tokenizer.decode(&encoded);
                        }
                    }
                    
                    // Test basic tokenize
                    let _ = tokenizer.tokenize(&input.text);
                }
            }
            TokenizerTypeFuzz::BgeM3 => {
                // Test TokenizerManager with different types
                if let Ok(tokenizer) = TokenizerManager::new(TokenizerType::BgeM3).await {
                    let _ = tokenizer.tokenize(&input.text);
                }
            }
        }
    });
});

// Additional fuzz target for testing tokenizer edge cases
#[derive(Debug, Arbitrary)]
struct TokenizerEdgeCaseInput {
    operations: Vec<TokenizerOperation>,
}

#[derive(Debug, Arbitrary)]
enum TokenizerOperation {
    Tokenize { text: String },
    Encode { text: String },
    Decode { tokens: Vec<u32> },
    BatchTokenize { texts: Vec<String> },
}

fuzz_target!(|input: TokenizerEdgeCaseInput| {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    
    rt.block_on(async {
        if let Ok(tokenizer) = SimpleQwen3Tokenizer::new() {
            for operation in input.operations.into_iter().take(10) { // Limit operations
                match operation {
                    TokenizerOperation::Tokenize { text } => {
                        let _ = tokenizer.tokenize(&text);
                    }
                    TokenizerOperation::Encode { text } => {
                        let _ = tokenizer.encode(&text);
                    }
                    TokenizerOperation::Decode { tokens } => {
                        // Filter tokens to reasonable values
                        let vocab_size = tokenizer.get_vocab_size();
                        let filtered_tokens: Vec<u32> = tokens
                            .into_iter()
                            .filter(|&token| (token as usize) < vocab_size)
                            .take(1000) // Limit token count
                            .collect();
                        
                        if !filtered_tokens.is_empty() {
                            let _ = tokenizer.decode(&filtered_tokens);
                        }
                    }
                    TokenizerOperation::BatchTokenize { texts } => {
                        // Test batch processing
                        let limited_texts: Vec<String> = texts
                            .into_iter()
                            .take(5) // Limit batch size
                            .collect();
                        
                        for text in limited_texts {
                            let _ = tokenizer.tokenize(&text);
                        }
                    }
                }
            }
        }
    });
});