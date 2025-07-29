use anyhow::Result;
use std::path::Path;
use tokenizers::Tokenizer;
use serde_json::Value;

/// Загружает токенизатор из различных форматов с полной поддержкой Qwen3
pub async fn load_tokenizer<P: AsRef<Path>>(model_path: P) -> Result<Tokenizer> {
    let model_path = model_path.as_ref();
    
    // Сначала пробуем tokenizer.json (если есть)
    let tokenizer_json = model_path.join("tokenizer.json");
    if tokenizer_json.exists() {
        return Tokenizer::from_file(&tokenizer_json)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer.json: {}", e));
    }
    
    // Для Qwen3 моделей создаем токенизатор из vocab.json и merges.txt
    let vocab_file = model_path.join("vocab.json");
    let merges_file = model_path.join("merges.txt");
    let tokenizer_config_file = model_path.join("tokenizer_config.json");
    let special_tokens_file = model_path.join("special_tokens_map.json");
    
    if vocab_file.exists() && merges_file.exists() {
        tracing::info!("Loading Qwen3 tokenizer from vocab.json and merges.txt");
        
        // Загружаем конфигурацию токенизатора
        let tokenizer_config = if tokenizer_config_file.exists() {
            let config_content = tokio::fs::read_to_string(&tokenizer_config_file).await?;
            serde_json::from_str::<Value>(&config_content)?
        } else {
            Value::Null
        };
        
        // Загружаем специальные токены
        let special_tokens = if special_tokens_file.exists() {
            let special_content = tokio::fs::read_to_string(&special_tokens_file).await?;
            serde_json::from_str::<Value>(&special_content)?
        } else {
            Value::Null
        };
        
        // Создаем BPE модель из файлов
        use tokenizers::models::bpe::BPE;
        let bpe = BPE::from_file(
            vocab_file.to_str().ok_or_else(|| anyhow::anyhow!("Invalid vocab path"))?,
            merges_file.to_str().ok_or_else(|| anyhow::anyhow!("Invalid merges path"))?
        )
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build BPE model: {:?}", e))?;
        
        let mut tokenizer = Tokenizer::new(bpe);
        
        // Настройка нормализации (Qwen3 использует минимальную нормализацию)
        use tokenizers::normalizers::{Sequence, Replace};
        let normalizers = vec![
            // Заменяем некоторые проблемные символы
            Replace::new("``", "\"".to_string()).unwrap().into(),
            Replace::new("''", "\"".to_string()).unwrap().into(),
        ];
        tokenizer.with_normalizer(Some(Sequence::new(normalizers)));
        
        // Используем простой whitespace pre-tokenizer (ByteLevel недоступен в публичном API)
        use tokenizers::pre_tokenizers::whitespace::Whitespace;
        tokenizer.with_pre_tokenizer(Some(Whitespace {}));
        
        // Добавляем специальные токены из конфигурации
        if let Value::Object(config) = &tokenizer_config {
            let mut special_tokens_to_add = Vec::new();
            
            // Добавляем токены из added_tokens_decoder
            if let Some(Value::Object(added_tokens)) = config.get("added_tokens_decoder") {
                for (token_id, token_data) in added_tokens {
                    if let Some(token_obj) = token_data.as_object() {
                        if let Some(Value::String(content)) = token_obj.get("content") {
                            if let Ok(_id) = token_id.parse::<u32>() {
                                let is_special = token_obj.get("special")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                
                                special_tokens_to_add.push((content.clone(), is_special));
                            }
                        }
                    }
                }
            }
            
            // Добавляем special tokens в tokenizer
            use tokenizers::AddedToken;
            for (token_str, is_special) in special_tokens_to_add {
                let added_token = AddedToken::from(token_str.as_str(), is_special);
                tokenizer.add_tokens(&[added_token]);
            }
        }
        
        // Используем BPE декодер через DecoderWrapper  
        use tokenizers::DecoderWrapper;
        tokenizer.with_decoder(Some(DecoderWrapper::BPE(Default::default())));
        
        tracing::info!("Successfully created Qwen3 tokenizer with {} special tokens", 
                      tokenizer_config.get("added_tokens_decoder")
                          .and_then(|v| v.as_object())
                          .map(|o| o.len())
                          .unwrap_or(0));
        
        return Ok(tokenizer);
    }
    
    // Если ничего не найдено, создаем базовый токенизатор для тестирования
    tracing::warn!("No tokenizer files found, creating a basic tokenizer for testing");
    
    use tokenizers::models::bpe::BPE;
    let bpe = BPE::default();
    let mut tokenizer = Tokenizer::new(bpe);
    
    // Добавляем базовую функциональность
    use tokenizers::normalizers::Lowercase;
    use tokenizers::NormalizerWrapper;
    tokenizer.with_normalizer(Some(NormalizerWrapper::from(Lowercase)));
    
    use tokenizers::pre_tokenizers::whitespace::Whitespace;
    tokenizer.with_pre_tokenizer(Some(Whitespace {}));
    
    Ok(tokenizer)
}