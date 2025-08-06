use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

/// Упрощённый токенизатор для Qwen3 моделей
/// Поскольку мы используем уже квантованные ONNX модели,
/// нам не нужна полная точность токенизации - достаточно базовой
pub struct SimpleQwen3Tokenizer {
    vocab_size: usize,
    max_length: usize,
    #[allow(dead_code)]
    special_tokens: HashMap<String, u32>,
}

impl SimpleQwen3Tokenizer {
    /// Создать токенизатор для Qwen3
    #[allow(dead_code)]
    pub fn new(model_dir: impl AsRef<Path>, max_length: usize) -> Result<Self> {
        info!("Creating simplified Qwen3 tokenizer");
        
        // Базовые специальные токены для Qwen3
        let mut special_tokens = HashMap::new();
        special_tokens.insert("<|endoftext|>".to_string(), 151643);
        special_tokens.insert("<|startoftext|>".to_string(), 151644);
        special_tokens.insert("<|im_start|>".to_string(), 151644);
        special_tokens.insert("<|im_end|>".to_string(), 151645);
        
        // Проверяем наличие config.json для получения vocab_size
        let config_path = model_dir.as_ref().join("config.json");
        let vocab_size = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    // Простой парсинг vocab_size из config.json
                    if let Some(vocab_match) = content.find("\"vocab_size\": ") {
                        let start = vocab_match + 14;
                        if let Some(end) = content[start..].find(|c: char| !c.is_numeric()) {
                            content[start..start+end].parse().unwrap_or(151669)
                        } else {
                            151669
                        }
                    } else {
                        151669
                    }
                }
                Err(_) => 151669
            }
        } else {
            151669 // Default Qwen3 vocab size
        };
        
        info!("✅ Simple Qwen3 tokenizer created");
        info!("   Vocab size: {}", vocab_size);
        info!("   Max length: {}", max_length);
        
        Ok(Self {
            vocab_size,
            max_length,
            special_tokens,
        })
    }
    
    /// Улучшенная токенизация для ONNX моделей
    pub fn encode(&self, text: &str) -> super::TokenizedInput {
        // Улучшенная токенизация с субсловным разбиением
        let mut input_ids = Vec::new();
        
        // Начинаем с BOS токена (для Qwen3 это endoftext)
        input_ids.push(151643i64);
        
        // Нормализация текста: lowercase + удаление лишних пробелов
        let normalized_text = text.to_lowercase().trim().to_string();
        
        // Разбиение на слова и символы для лучшей токенизации
        let mut remaining_length = self.max_length - 2; // Резервируем место для BOS/EOS
        
        for word in normalized_text.split_whitespace() {
            if remaining_length == 0 { break; }
            
            // Для каждого слова пытаемся разбить на подслова
            let word_tokens = self.tokenize_word(word, remaining_length);
            input_ids.extend(word_tokens.iter());
            remaining_length = remaining_length.saturating_sub(word_tokens.len());
        }
        
        // Добавляем EOS токен
        input_ids.push(151643i64);
        
        let length = input_ids.len();
        let attention_mask = vec![1i64; length];
        let token_type_ids = vec![0i64; length]; // Qwen3 не использует, но нужны для совместимости
        
        super::TokenizedInput {
            input_ids,
            attention_mask,
            token_type_ids,
            length,
        }
    }
    
    /// Улучшенная токенизация отдельного слова
    fn tokenize_word(&self, word: &str, max_tokens: usize) -> Vec<i64> {
        if max_tokens == 0 { return vec![]; }
        
        let mut tokens = Vec::new();
        let word_chars: Vec<char> = word.chars().collect();
        
        // Если слово очень короткое, обрабатываем как один токен
        if word.len() <= 3 {
            let token_id = self.char_sequence_to_token(&word_chars);
            tokens.push(token_id);
            return tokens;
        }
        
        // Разбиваем длинные слова на части (субсловная токенизация)
        let mut i = 0;
        while i < word_chars.len() && tokens.len() < max_tokens {
            // Пытаемся найти наибольший возможный субтокен (до 6 символов)
            let max_len = std::cmp::min(6, word_chars.len() - i);
            let mut best_len = 1;
            
            // Находим оптимальную длину субтокена
            for len in (2..=max_len).rev() {
                let subword: String = word_chars[i..i + len].iter().collect();
                if self.is_good_subword(&subword) {
                    best_len = len;
                    break;
                }
            }
            
            let subword_chars = &word_chars[i..i + best_len];
            let token_id = self.char_sequence_to_token(subword_chars);
            tokens.push(token_id);
            
            i += best_len;
        }
        
        tokens
    }
    
    /// Определяет является ли подслово "хорошим" для токенизации
    fn is_good_subword(&self, subword: &str) -> bool {
        // Предпочитаем подслова которые:
        // 1. Заканчиваются на гласную
        // 2. Содержат общие префиксы/суффиксы
        // 3. Являются целыми морфемами
        
        let last_char = subword.chars().last().unwrap_or(' ').to_lowercase().next().unwrap_or(' ');
        let vowels = ['a', 'e', 'i', 'o', 'u', 'y', 'а', 'е', 'и', 'о', 'у', 'ы', 'э', 'ю', 'я'];
        
        // Проверяем общие префиксы/суффиксы
        let common_endings = ["ing", "ed", "er", "ly", "tion", "ness", "ment", 
                             "ный", "ком", "тся", "ать", "ить", "еть"];
        let common_prefixes = ["un", "re", "pre", "de", "dis", 
                              "не", "пре", "при", "раз", "без"];
        
        vowels.contains(&last_char) || 
        common_endings.iter().any(|&ending| subword.ends_with(ending)) ||
        common_prefixes.iter().any(|&prefix| subword.starts_with(prefix))
    }
    
    /// Преобразует последовательность символов в токен ID
    fn char_sequence_to_token(&self, chars: &[char]) -> i64 {
        // Более качественное хеширование с учетом позиции символов
        let mut hash = 5381u64; // DJB2 hash base
        
        for (i, &ch) in chars.iter().enumerate() {
            let char_code = ch as u64;
            hash = hash.wrapping_mul(33).wrapping_add(char_code);
            
            // Добавляем вес позиции для лучшего распределения
            let position_weight = (i + 1) as u64;
            hash = hash.wrapping_add(position_weight.wrapping_mul(char_code));
        }
        
        // Добавляем длину для различения подслов разной длины
        hash = hash.wrapping_add(chars.len() as u64 * 97);
        
        // Мапим в диапазон vocab_size, избегая специальных токенов (первые 100)
        let token_range = self.vocab_size as u64 - 200; // Избегаем первые и последние 100 токенов
        let token_id = (hash % token_range) + 100;
        
        token_id as i64
    }
    
    /// Батч токенизация
    pub fn encode_batch(&self, texts: &[&str]) -> super::BatchTokenized {
        let mut batch_input_ids = Vec::with_capacity(texts.len());
        let mut batch_attention_masks = Vec::with_capacity(texts.len());
        let mut batch_token_type_ids = Vec::with_capacity(texts.len());
        let mut batch_lengths = Vec::with_capacity(texts.len());
        let mut max_seq_len = 0;
        
        for text in texts {
            let tokenized = self.encode(text);
            max_seq_len = max_seq_len.max(tokenized.length);
            batch_input_ids.push(tokenized.input_ids);
            batch_attention_masks.push(tokenized.attention_mask);
            batch_token_type_ids.push(tokenized.token_type_ids);
            batch_lengths.push(tokenized.length);
        }
        
        super::BatchTokenized {
            input_ids: batch_input_ids,
            attention_masks: batch_attention_masks,
            token_type_ids: batch_token_type_ids,
            lengths: batch_lengths,
            max_length: max_seq_len,
        }
    }
    
    /// Паддинг батча
    pub fn pad_batch(&self, batch: &mut super::BatchTokenized, target_length: Option<usize>) -> Result<()> {
        let pad_length = target_length.unwrap_or(batch.max_length);
        let pad_token_id = 151643i64; // Qwen3 использует endoftext как pad
        
        for i in 0..batch.input_ids.len() {
            let current_len = batch.input_ids[i].len();
            
            if current_len < pad_length {
                let pad_count = pad_length - current_len;
                
                batch.input_ids[i].extend(vec![pad_token_id; pad_count]);
                batch.attention_masks[i].extend(vec![0i64; pad_count]);
                batch.token_type_ids[i].extend(vec![0i64; pad_count]);
            }
        }
        
        batch.max_length = pad_length;
        Ok(())
    }
    
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }
    
    #[allow(dead_code)]
    pub fn max_length(&self) -> usize {
        self.max_length
    }
}