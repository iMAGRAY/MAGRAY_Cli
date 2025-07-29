use anyhow::{Context, Result};
use ort::{Environment, GraphOptimizationLevel, Session, SessionBuilder};
use std::path::PathBuf;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};
use std::collections::HashMap;

/// Реализация Qwen3 модели для эмбеддингов через ONNX Runtime v1.16
/// 
/// Эта версия использует правильный API для ORT 1.16
#[derive(Debug)]
pub struct Qwen3EmbeddingModel {
    environment: Arc<Environment>,
    session: Arc<Mutex<Session>>,
    tokenizer: Tokenizer,
    embedding_dim: usize,
    max_length: usize,
    pad_token_id: u32,
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl Qwen3EmbeddingModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        info!("Loading Qwen3 Embedding model from: {}", model_path.display());
        
        // Проверяем существование директории и файлов
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Model directory not found: {}", model_path.display()));
        }

        // Поддерживаем разные имена файлов моделей
        let model_file = if model_path.join("model_fp16.onnx").exists() {
            model_path.join("model_fp16.onnx")
        } else if model_path.join("model.onnx").exists() {
            model_path.join("model.onnx")
        } else {
            return Err(anyhow::anyhow!("Model file not found in: {}", model_path.display()));
        };

        let tokenizer_file = model_path.join("tokenizer.json");
        if !tokenizer_file.exists() {
            return Err(anyhow::anyhow!("Tokenizer file not found: {}", tokenizer_file.display()));
        }

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

        // Создаем ONNX Runtime environment
        let environment = Arc::new(
            Environment::builder()
                .with_name("qwen3_embedding")
                .build()
                .context("Failed to create ONNX Runtime environment")?
        );
        
        // Создаем сессию с моделью
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_model_from_file(&model_file)
            .with_context(|| format!("Failed to load ONNX model: {}", model_file.display()))?;

        // Загружаем токенизатор
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        // Для Qwen3 pad_token это "<|endoftext|>" с ID 151643
        let pad_token_id = 151643u32;

        info!("Successfully loaded Qwen3 embedding model:");
        info!("  - Hidden size: {}", hidden_size);
        info!("  - Max length: {}", max_position_embeddings);
        info!("  - Model file: {}", model_file.display());

        Ok(Self {
            environment,
            session: Arc::new(Mutex::new(session)),
            tokenizer,
            embedding_dim: hidden_size,
            max_length: max_position_embeddings.min(512), // Ограничиваем для производительности
            pad_token_id,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Генерирует эмбеддинги для списка текстов
    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(texts.len());

        // Проверяем кэш
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

        // Находим тексты для обработки
        let mut to_process = Vec::new();
        let mut indices_to_process = Vec::new();
        
        for (i, result) in results.iter().enumerate() {
            if result.is_none() {
                to_process.push(texts[i].clone());
                indices_to_process.push(i);
            }
        }

        // Обрабатываем некэшированные тексты
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

        // Собираем финальные результаты
        results.into_iter()
            .map(|opt| opt.ok_or_else(|| anyhow::anyhow!("Failed to compute embedding")))
            .collect()
    }

    async fn compute_embeddings_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        // Обрабатываем по одному тексту
        for text in texts {
            let embedding = self.compute_single_embedding(text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    async fn compute_single_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Токенизация
        let encoding = self.tokenizer
            .encode(text, false)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize text: {}", e))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // Паддинг или обрезка до нужной длины
        let target_length = self.max_length;
        
        if input_ids.len() > target_length {
            input_ids.truncate(target_length);
            attention_mask.truncate(target_length);
        } else {
            while input_ids.len() < target_length {
                input_ids.push(self.pad_token_id);
                attention_mask.push(0);
            }
        }

        // Конвертируем в i64 для ONNX
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();

        // Выполняем инференс используя правильный API для ORT 1.16
        let session = self.session.lock().await;
        
        // В ORT 1.16 используем напрямую векторы с формой
        use ort::tensor::InputTensor;
        use ort::Value;
        
        // Создаём тензоры через кортежи (shape, data)
        let input_ids_tensor = InputTensor::from_array(
            vec![1_i64, target_length as i64],
            input_ids_i64.into_boxed_slice()
        ).context("Failed to create input_ids tensor")?;
        
        let attention_mask_tensor = InputTensor::from_array(
            vec![1_i64, target_length as i64], 
            attention_mask_i64.into_boxed_slice()
        ).context("Failed to create attention_mask tensor")?;
        
        // Получаем имена входов
        let input_names: Vec<_> = session.inputs.iter().map(|i| i.name.as_str()).collect();
        if input_names.len() < 2 {
            return Err(anyhow::anyhow!("Model requires at least 2 inputs, found {}", input_names.len()));
        }
        
        // Создаём мапу входов
        let mut inputs = ort::SessionInputs::new();
        inputs = inputs.with_tensor(&input_names[0], input_ids_tensor)?;
        inputs = inputs.with_tensor(&input_names[1], attention_mask_tensor)?;
        
        // Запускаем инференс
        let outputs = session.run(inputs)
            .context("Failed to run ONNX inference")?;

        // Извлекаем результат
        let output_names: Vec<_> = session.outputs.iter().map(|o| o.name.clone()).collect();
        if output_names.is_empty() {
            return Err(anyhow::anyhow!("No outputs from model"));
        }
        
        let output = outputs.get(&output_names[0])
            .ok_or_else(|| anyhow::anyhow!("Output '{}' not found", output_names[0]))?;
        
        // Извлекаем данные из тензора
        let output_tensor = output.try_extract::<f32>()
            .context("Failed to extract output as f32 tensor")?;
        
        let output_data = output_tensor.view().as_slice()
            .ok_or_else(|| anyhow::anyhow!("Failed to get tensor data as slice"))?;
        
        // Предполагаем, что выход имеет форму [1, seq_len, hidden_size]
        let hidden_size = self.embedding_dim;
        let seq_len = target_length;
        
        if output_data.len() != seq_len * hidden_size {
            return Err(anyhow::anyhow!(
                "Unexpected output size: expected {}, got {}", 
                seq_len * hidden_size, 
                output_data.len()
            ));
        }

        // Mean pooling с учетом attention mask
        let mut pooled_embedding = vec![0.0f32; hidden_size];
        let mut valid_tokens = 0u32;

        for seq_idx in 0..seq_len {
            if seq_idx < attention_mask.len() && attention_mask[seq_idx] == 1 {
                let offset = seq_idx * hidden_size;
                for hidden_idx in 0..hidden_size {
                    pooled_embedding[hidden_idx] += output_data[offset + hidden_idx];
                }
                valid_tokens += 1;
            }
        }

        // Нормализация
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

/// Реализация Qwen3 модели для reranking через ONNX Runtime v1.16
#[derive(Debug)]
pub struct Qwen3RerankerModel {
    environment: Arc<Environment>,
    session: Arc<Mutex<Session>>,
    tokenizer: Tokenizer,
    max_length: usize,
    pad_token_id: u32,
}

impl Qwen3RerankerModel {
    pub async fn new(model_path: PathBuf) -> Result<Self> {
        info!("Loading Qwen3 Reranker model from: {}", model_path.display());
        
        // Проверяем существование директории и файлов
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Model directory not found: {}", model_path.display()));
        }

        let model_file = model_path.join("model.onnx");
        if !model_file.exists() {
            return Err(anyhow::anyhow!("Model file not found: {}", model_file.display()));
        }

        let tokenizer_file = model_path.join("tokenizer.json");
        if !tokenizer_file.exists() {
            return Err(anyhow::anyhow!("Tokenizer file not found: {}", tokenizer_file.display()));
        }

        // Загружаем конфигурацию
        let config_file = model_path.join("config.json");
        let config_content = tokio::fs::read_to_string(&config_file).await
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;
        let config: serde_json::Value = serde_json::from_str(&config_content)
            .context("Failed to parse config.json")?;
        
        let max_position_embeddings = config["max_position_embeddings"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("max_position_embeddings not found in config"))? as usize;

        // Создаем ONNX Runtime environment
        let environment = Arc::new(
            Environment::builder()
                .with_name("qwen3_reranker")
                .build()
                .context("Failed to create ONNX Runtime environment")?
        );

        // Создаем сессию
        let session = SessionBuilder::new(&environment)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_model_from_file(&model_file)
            .with_context(|| format!("Failed to load ONNX model: {}", model_file.display()))?;

        // Загружаем токенизатор
        let tokenizer = Tokenizer::from_file(&tokenizer_file)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        let pad_token_id = 151643u32; // "<|endoftext|>" для Qwen3

        info!("Successfully loaded Qwen3 reranker model");

        Ok(Self {
            environment,
            session: Arc::new(Mutex::new(session)),
            tokenizer,
            max_length: max_position_embeddings.min(1024),
            pad_token_id,
        })
    }

    /// Ранжирует документы относительно запроса
    pub async fn rerank(&self, query: &str, documents: &[String], top_k: usize) -> Result<Vec<(usize, f32)>> {
        if documents.is_empty() {
            return Ok(Vec::new());
        }

        let mut scored_docs = Vec::new();

        // Вычисляем скоры для каждого документа
        for (idx, doc) in documents.iter().enumerate() {
            match self.compute_relevance_score(query, doc).await {
                Ok(score) => scored_docs.push((idx, score)),
                Err(e) => {
                    warn!("Failed to compute score for document {}: {}", idx, e);
                    scored_docs.push((idx, 0.0)); // Fallback score
                }
            }
        }

        // Сортируем по убыванию скора
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Возвращаем top-k результатов
        scored_docs.truncate(top_k);
        
        debug!("Reranked {} documents, returning top {}", documents.len(), scored_docs.len());
        Ok(scored_docs)
    }

    async fn compute_relevance_score(&self, query: &str, document: &str) -> Result<f32> {
        // Формируем input в стиле cross-encoder
        let combined_text = format!("{} [SEP] {}", query, document);

        // Токенизируем
        let encoding = self.tokenizer
            .encode(combined_text.as_str(), false)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize text for reranking: {}", e))?;

        let mut input_ids = encoding.get_ids().to_vec();
        let mut attention_mask = encoding.get_attention_mask().to_vec();

        // Паддинг или обрезка
        let target_length = self.max_length;
        
        if input_ids.len() > target_length {
            input_ids.truncate(target_length);
            attention_mask.truncate(target_length);
        } else {
            while input_ids.len() < target_length {
                input_ids.push(self.pad_token_id);
                attention_mask.push(0);
            }
        }

        // Конвертируем в i64
        let input_ids_i64: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
        let attention_mask_i64: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();

        // Выполняем инференс
        let session = self.session.lock().await;
        
        use ort::tensor::InputTensor;
        
        // Создаём тензоры
        let input_ids_tensor = InputTensor::from_array(
            vec![1_i64, target_length as i64],
            input_ids_i64.into_boxed_slice()
        )?;
        
        let attention_mask_tensor = InputTensor::from_array(
            vec![1_i64, target_length as i64],
            attention_mask_i64.into_boxed_slice()
        )?;
        
        // Получаем имена входов
        let input_names: Vec<_> = session.inputs.iter().map(|i| i.name.as_str()).collect();
        if input_names.len() < 2 {
            return Err(anyhow::anyhow!("Reranker model requires at least 2 inputs"));
        }
        
        // Создаём мапу входов
        let mut inputs = ort::SessionInputs::new();
        inputs = inputs.with_tensor(&input_names[0], input_ids_tensor)?;
        inputs = inputs.with_tensor(&input_names[1], attention_mask_tensor)?;
        
        // Запускаем инференс
        let outputs = session.run(inputs)?;

        // Извлекаем скор
        let output_names: Vec<_> = session.outputs.iter().map(|o| o.name.clone()).collect();
        if output_names.is_empty() {
            return Err(anyhow::anyhow!("No outputs from reranker model"));
        }
        
        let output = outputs.get(&output_names[0])
            .ok_or_else(|| anyhow::anyhow!("Output '{}' not found", output_names[0]))?;
        
        let output_tensor = output.try_extract::<f32>()?;
        let output_data = output_tensor.view().as_slice()
            .ok_or_else(|| anyhow::anyhow!("Failed to get reranker output as slice"))?;

        // Обычно reranker возвращает логиты или скоры
        // Берем первое значение и применяем sigmoid для нормализации
        let raw_score = output_data.first()
            .copied()
            .unwrap_or(0.0);

        // Применяем sigmoid для получения вероятности в [0, 1]
        let score = 1.0 / (1.0 + (-raw_score).exp());

        Ok(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_mock_model_dir() -> Result<TempDir> {
        let temp_dir = TempDir::new()?;
        let model_path = temp_dir.path();

        // Создаем минимальную конфигурацию
        let config = serde_json::json!({
            "hidden_size": 1024,
            "max_position_embeddings": 512,
            "model_type": "qwen2"
        });
        
        tokio::fs::write(
            model_path.join("config.json"),
            serde_json::to_string_pretty(&config)?
        ).await?;

        // Создаем заглушки для файлов модели и токенизатора
        // В реальных тестах здесь должны быть настоящие файлы
        tokio::fs::write(model_path.join("model.onnx"), b"mock model").await?;
        tokio::fs::write(model_path.join("tokenizer.json"), b"{}").await?;

        Ok(temp_dir)
    }

    #[tokio::test]
    async fn test_model_loading_fails_without_files() {
        let temp_dir = TempDir::new().unwrap();
        let result = Qwen3EmbeddingModel::new(temp_dir.path().to_path_buf()).await;
        assert!(result.is_err());
    }

    #[tokio::test] 
    async fn test_cache_operations() {
        // Этот тест можно запустить только с реальными моделями
        // или используя mock реализацию
    }
}