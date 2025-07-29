use anyhow::{Context, Result};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Value,
};
use std::path::PathBuf;
use tokenizers::Tokenizer;
use tracing::{debug, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use crate::tokenizer_utils::load_tokenizer;

/// Настоящая Qwen3 модель для эмбеддингов через ONNX Runtime
/// 
/// Эта реализация использует реальные ONNX модели и токенизатор
/// для генерации высококачественных эмбеддингов текста
#[derive(Debug)]
pub struct Qwen3EmbeddingModel {
    session: Arc<Mutex<Session>>,
    tokenizer: Tokenizer,
    embedding_dim: usize,
    max_length: usize,
    pad_token_id: u32,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    use_kv_cache: bool,
    num_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
}

impl Qwen3EmbeddingModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        // Инициализируем ORT при первом использовании
        crate::onnx_init::ensure_ort_initialized()?;
        
        info!("Loading Qwen3 Embedding model from: {}", model_path.display());
        
        // Проверяем что директория и файлы существуют
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Model directory not found: {}", model_path.display()));
        }

        // Проверяем разные варианты имен файлов
        let model_file = if model_path.join("model_fp16.onnx").exists() {
            model_path.join("model_fp16.onnx")
        } else if model_path.join("model.onnx").exists() {
            model_path.join("model.onnx")
        } else {
            return Err(anyhow::anyhow!("Model file not found in: {}", model_path.display()));
        };

        // Загружаем конфигурацию модели
        let config_file = model_path.join("config.json");
        let config_content = tokio::fs::read_to_string(&config_file).await
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;
        let config: serde_json::Value = serde_json::from_str(&config_content)
            .context("Failed to parse config.json")?;
        
        let hidden_size = config["hidden_size"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("hidden_size not found in config"))? as usize;
        let max_position_embeddings = config["max_position_embeddings"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("max_position_embeddings not found in config"))? as usize;
        
        // Читаем параметры для KV-cache
        let use_kv_cache = config["use_cache"].as_bool().unwrap_or(false);
        let num_layers = config["num_hidden_layers"].as_u64().unwrap_or(28) as usize;
        let num_attention_heads = config["num_attention_heads"].as_u64().unwrap_or(16) as usize;
        let num_key_value_heads = config["num_key_value_heads"].as_u64().unwrap_or(8) as usize;
        let head_dim = config["head_dim"].as_u64().unwrap_or(128) as usize;

        // В ORT 2.0 инициализация происходит автоматически
        // Загружаем ONNX сессию напрямую
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_file)
            .with_context(|| format!("Failed to load ONNX model: {}", model_file.display()))?;

        // Загружаем токенизатор (поддерживаем разные форматы)
        let tokenizer = load_tokenizer(&model_path).await
            .with_context(|| format!("Failed to load tokenizer from: {}", model_path.display()))?;

        // Для Qwen3 pad_token это "<|endoftext|>" с ID 151643
        let pad_token_id = 151643u32;

        info!("Successfully loaded Qwen3 embedding model:");
        info!("  - Hidden size: {}", hidden_size);
        info!("  - Max length: {}", max_position_embeddings);
        info!("  - Pad token ID: {}", pad_token_id);
        info!("  - Use KV-cache: {}", use_kv_cache);
        info!("  - Layers: {}, Heads: {}, KV heads: {}, Head dim: {}", 
              num_layers, num_attention_heads, num_key_value_heads, head_dim);

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer,
            embedding_dim: hidden_size,
            max_length: max_position_embeddings.min(32768), // Ограничиваем для производительности
            pad_token_id,
            cache: Arc::new(RwLock::new(HashMap::new())),
            use_kv_cache,
            num_layers,
            num_attention_heads,
            num_key_value_heads,
            head_dim,
        })
    }

    /// Генерирует эмбеддинги для списка текстов
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(texts.len());

        // Проверяем кэш для каждого текста
        {
            let cache = self.cache.read().await;
            for text in texts {
                let cache_key = self.cache_key(text);
                if let Some(cached) = cache.get(&cache_key) {
                    results.push(Some(cached.clone()));
                } else {
                    results.push(None);
                }
            }
        }

        // Находим тексты, которые нужно обработать
        let mut to_process = Vec::new();
        let mut indices_to_process = Vec::new();
        
        for (i, result) in results.iter().enumerate() {
            if result.is_none() {
                to_process.push(texts[i].clone());
                indices_to_process.push(i);
            }
        }

        // Обрабатываем тексты пакетами
        if !to_process.is_empty() {
            let computed_embeddings = self.compute_embeddings_batch(&to_process).await?;
            
            // Обновляем кэш и результаты
            {
                let mut cache = self.cache.write().await;
                for (i, embedding) in computed_embeddings.into_iter().enumerate() {
                    let text = &to_process[i];
                    let cache_key = self.cache_key(text);
                    cache.insert(cache_key, embedding.clone());
                    
                    let result_index = indices_to_process[i];
                    results[result_index] = Some(embedding);
                }
            }
        }

        // Конвертируем в финальный результат
        let final_results: Result<Vec<Vec<f32>>> = results
            .into_iter()
            .map(|opt| opt.ok_or_else(|| anyhow::anyhow!("Failed to compute embedding")))
            .collect();

        final_results
    }

    async fn compute_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        // Обрабатываем по одному тексту для простоты
        // В продакшене можно реализовать батчинг
        for text in texts {
            let embedding = self.compute_single_embedding(text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    async fn compute_single_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Токенизируем текст
        let encoding = self.tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize text: {}", e))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // Обрезаем или дополняем до нужной длины
        let target_length = self.max_length.min(512); // Используем 512 для производительности
        
        if input_ids.len() > target_length {
            input_ids.truncate(target_length);
            attention_mask.truncate(target_length);
        } else {
            while input_ids.len() < target_length {
                input_ids.push(self.pad_token_id);
                attention_mask.push(0);
            }
        }

        // Конвертируем в формат для ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();
        
        // Создаем position_ids (последовательные индексы позиций)
        let position_ids: Vec<i64> = (0..target_length as i64).collect();

        // Создаем тензоры - используем правильный формат для ORT
        let input_ids_value = Value::from_array(([1, target_length], input_ids_i64))?;
        let attention_mask_value = Value::from_array(([1, target_length], attention_mask_i64))?;
        let position_ids_value = Value::from_array(([1, target_length], position_ids))?;
        
        // Выполняем инференс с учетом KV-cache
        let mut session = self.session.lock().await;
        let outputs = if self.use_kv_cache {
            // Модель с KV-cache - создаем пустые past_key_values для всех слоев
            let mut inputs = ort::inputs![
                "input_ids" => &input_ids_value,
                "attention_mask" => &attention_mask_value,
                "position_ids" => &position_ids_value,
            ];
            
            // Добавляем past_key_values для всех слоев
            // Размер: [batch_size, num_key_value_heads, seq_len, head_dim]
            let kv_shape = [1, self.num_key_value_heads, 0, self.head_dim];
            let empty_kv = Value::from_array((kv_shape, vec![0.0f32; 0]))?;
            
            for layer_idx in 0..self.num_layers {
                let key_name = format!("past_key_values.{}.key", layer_idx);
                let value_name = format!("past_key_values.{}.value", layer_idx);
                inputs.insert(key_name.as_str(), &empty_kv);
                inputs.insert(value_name.as_str(), &empty_kv);
            }
            
            session.run(inputs)?
        } else {
            // Модель без KV-cache - только основные входы
            session.run(ort::inputs![
                "input_ids" => &input_ids_value,
                "attention_mask" => &attention_mask_value,
                "position_ids" => &position_ids_value,
            ])?
        };

        // Извлекаем эмбеддинги из последнего скрытого состояния
        let (shape, data) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()?;
        if shape.len() != 3 || shape[0] != 1 {
            return Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape));
        }

        let seq_len = shape[1] as usize;
        let hidden_size = shape[2] as usize;
        
        // Создаем ndarray для удобного доступа к данным
        let tensor_array = ndarray::ArrayView3::from_shape((1, seq_len, hidden_size), data)?;

        // Применяем mean pooling с учетом attention mask
        let mut pooled_embedding = vec![0.0f32; hidden_size];
        let mut valid_tokens = 0u32;

        for seq_idx in 0..seq_len {
            if attention_mask[seq_idx] == 1 {
                for hidden_idx in 0..hidden_size {
                    pooled_embedding[hidden_idx] += tensor_array[[0, seq_idx, hidden_idx]];
                }
                valid_tokens += 1;
            }
        }

        // Нормализуем по количеству валидных токенов
        if valid_tokens > 0 {
            for val in &mut pooled_embedding {
                *val /= valid_tokens as f32;
            }
        }

        // L2 нормализация для косинусного сходства
        let norm: f32 = pooled_embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut pooled_embedding {
                *val /= norm;
            }
        }

        debug!("Generated embedding for text: {:.50}...", text);
        Ok(pooled_embedding)
    }

    pub fn embedding_dim(&self) -> usize {
        self.embedding_dim
    }

    fn cache_key(&self, text: &str) -> String {
        format!("qwen3:{}", blake3::hash(text.as_bytes()).to_hex())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        info!("Cleared Qwen3 embedding cache");
        Ok(())
    }

    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        let entries = cache.len();
        let size_bytes = cache.iter()
            .map(|(k, v)| k.len() + v.len() * std::mem::size_of::<f32>())
            .sum();
        (entries, size_bytes)
    }
}

/// Настоящая Qwen3 модель для reranking через ONNX Runtime
#[derive(Debug)]
pub struct Qwen3RerankerModel {
    session: Arc<Mutex<Session>>,
    tokenizer: Tokenizer,
    max_length: usize,
    pad_token_id: u32,
    use_kv_cache: bool,
    num_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: usize,
    head_dim: usize,
}

impl Qwen3RerankerModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        // Инициализируем ORT при первом использовании
        crate::onnx_init::ensure_ort_initialized()?;
        
        info!("Loading Qwen3 Reranker model from: {}", model_path.display());
        
        // Проверяем что директория и файлы существуют
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Model directory not found: {}", model_path.display()));
        }

        // Проверяем разные варианты имен файлов
        let model_file = if model_path.join("model_fp16.onnx").exists() {
            model_path.join("model_fp16.onnx")
        } else if model_path.join("model.onnx").exists() {
            model_path.join("model.onnx")
        } else {
            return Err(anyhow::anyhow!("Model file not found in: {}", model_path.display()));
        };

        // Загружаем конфигурацию модели
        let config_file = model_path.join("config.json");
        let config_content = tokio::fs::read_to_string(&config_file).await
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;
        let config: serde_json::Value = serde_json::from_str(&config_content)
            .context("Failed to parse config.json")?;
        
        let max_position_embeddings = config["max_position_embeddings"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("max_position_embeddings not found in config"))? as usize;
        
        // Читаем параметры для KV-cache для reranker
        let use_kv_cache = config["use_cache"].as_bool().unwrap_or(false);
        let num_layers = config["num_hidden_layers"].as_u64().unwrap_or(28) as usize;
        let num_attention_heads = config["num_attention_heads"].as_u64().unwrap_or(16) as usize;
        let num_key_value_heads = config["num_key_value_heads"].as_u64().unwrap_or(8) as usize;
        let head_dim = config["head_dim"].as_u64().unwrap_or(128) as usize;

        // В ORT 2.0 инициализация происходит автоматически
        // Загружаем ONNX сессию напрямую
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(&model_file)
            .with_context(|| format!("Failed to load ONNX model: {}", model_file.display()))?;

        // Загружаем токенизатор (поддерживаем разные форматы)
        let tokenizer = load_tokenizer(&model_path).await
            .with_context(|| format!("Failed to load tokenizer from: {}", model_path.display()))?;

        let pad_token_id = 151643u32; // "<|endoftext|>" для Qwen3

        info!("Successfully loaded Qwen3 reranker model");
        info!("  - Max length: {}", max_position_embeddings);
        info!("  - Pad token ID: {}", pad_token_id);
        info!("  - Use KV-cache: {}", use_kv_cache);
        info!("  - Layers: {}, Heads: {}, KV heads: {}, Head dim: {}", 
              num_layers, num_attention_heads, num_key_value_heads, head_dim);

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer,
            max_length: max_position_embeddings.min(40960),
            pad_token_id,
            use_kv_cache,
            num_layers,
            num_attention_heads,
            num_key_value_heads,
            head_dim,
        })
    }

    /// Ранжирует документы относительно запроса
    pub async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_docs = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            let score = self.compute_relevance_score(query, doc).await?;
            scored_docs.push((idx, score));
        }

        // Сортируем по убыванию скора
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Возвращаем top-k результатов
        scored_docs.truncate(top_k);
        
        debug!("Reranked {} documents, returning top {}", documents.len(), scored_docs.len());
        Ok(scored_docs)
    }

    async fn compute_relevance_score(&self, query: &str, document: &str) -> Result<f32> {
        // Формируем input для reranker в стиле Qwen3
        // Используем шаблон: query [SEP] document
        let combined_text = format!("{} [SEP] {}", query, document);

        // Токенизируем
        let encoding = self.tokenizer
            .encode(combined_text.as_str(), false)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize text for reranking: {}", e))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // Обрезаем или дополняем до нужной длины
        let target_length = self.max_length.min(1024); // Используем 1024 для reranking
        
        if input_ids.len() > target_length {
            input_ids.truncate(target_length);
            attention_mask.truncate(target_length);
        } else {
            while input_ids.len() < target_length {
                input_ids.push(self.pad_token_id);
                attention_mask.push(0);
            }
        }

        // Конвертируем в формат для ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();
        
        // Создаем position_ids (последовательные индексы позиций)
        let position_ids: Vec<i64> = (0..target_length as i64).collect();

        // Создаем тензоры - используем правильный формат для ORT
        let input_ids_value = Value::from_array(([1, target_length], input_ids_i64))?;
        let attention_mask_value = Value::from_array(([1, target_length], attention_mask_i64))?;
        let position_ids_value = Value::from_array(([1, target_length], position_ids))?;
        
        // Выполняем инференс с учетом KV-cache
        let mut session = self.session.lock().await;
        let outputs = if self.use_kv_cache {
            // Модель с KV-cache - создаем пустые past_key_values для всех слоев
            let mut inputs = ort::inputs![
                "input_ids" => &input_ids_value,
                "attention_mask" => &attention_mask_value,
                "position_ids" => &position_ids_value,
            ];
            
            // Добавляем past_key_values для всех слоев
            // Размер: [batch_size, num_key_value_heads, seq_len, head_dim]
            let kv_shape = [1, self.num_key_value_heads, 0, self.head_dim];
            let empty_kv = Value::from_array((kv_shape, vec![0.0f32; 0]))?;
            
            for layer_idx in 0..self.num_layers {
                let key_name = format!("past_key_values.{}.key", layer_idx);
                let value_name = format!("past_key_values.{}.value", layer_idx);
                inputs.insert(key_name.as_str(), &empty_kv);
                inputs.insert(value_name.as_str(), &empty_kv);
            }
            
            session.run(inputs)?
        } else {
            // Модель без KV-cache - только основные входы
            session.run(ort::inputs![
                "input_ids" => &input_ids_value,
                "attention_mask" => &attention_mask_value,
                "position_ids" => &position_ids_value,
            ])?
        };

        // Извлекаем logits для классификации релевантности
        let (shape, data) = outputs["logits"]
            .try_extract_tensor::<f32>()?;
        if shape.len() < 2 {
            return Err(anyhow::anyhow!("Unexpected logits shape: {:?}", shape));
        }

        // Для reranking обычно берем последний токен или делаем pooling
        // Простой подход: берем среднее значение активных токенов
        let seq_len = shape[1] as usize;
        let vocab_size = if shape.len() > 2 { shape[2] as usize } else { 1usize };
        
        let mut relevance_score = 0.0f32;
        let mut active_tokens = 0;

        // Используем простой подход - берем среднее по всем активным токенам
        for seq_idx in 0..seq_len {
            if seq_idx < attention_mask.len() && attention_mask[seq_idx] == 1 {
                if shape.len() == 3 && vocab_size > 1 {
                    // 3D тензор: берем максимальный logit по vocab размерности
                    let mut max_logit = f32::NEG_INFINITY;
                    for vocab_idx in 0..vocab_size {
                        let data_idx = seq_idx * vocab_size + vocab_idx;
                        if data_idx < data.len() {
                            let logit = data[data_idx];
                            if logit > max_logit {
                                max_logit = logit;
                            }
                        }
                    }
                    relevance_score += max_logit;
                } else {
                    // 2D тензор: просто берем значение
                    if seq_idx < data.len() {
                        relevance_score += data[seq_idx];
                    }
                }
                active_tokens += 1;
            }
        }

        if active_tokens > 0 {
            relevance_score /= active_tokens as f32;
        }

        // Применяем sigmoid для нормализации в [0, 1]
        let normalized_score = 1.0 / (1.0 + (-relevance_score).exp());

        Ok(normalized_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Эти тесты будут работать только при наличии реальных моделей
}