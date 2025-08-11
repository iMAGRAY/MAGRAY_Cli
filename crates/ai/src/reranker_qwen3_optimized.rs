use crate::tokenization::OptimizedTokenizer;
use crate::{AiError, RerankingConfig, Result};
use ndarray::Array2;
use ort::{inputs, session::Session, value::Tensor};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

pub struct OptimizedRerankingService {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<OptimizedTokenizer>,
    config: RerankingConfig,
    max_length: usize,
}

#[derive(Debug, Clone)]
pub struct RerankResult {
    pub query: String,
    pub document: String,
    pub score: f32,
    pub original_index: usize,
}

impl OptimizedRerankingService {
    /// Create a new optimized reranking service with real ONNX inference
    pub async fn new(config: RerankingConfig) -> Result<Self> {
        info!(
            "🚀 Инициализация OptimizedQwen3RerankingService с моделью: {}",
            config.model_name
        );

        // Проверяем локальную модель в разных местах
        let possible_paths = vec![
            PathBuf::from("crates/memory/models").join(&config.model_name),
            PathBuf::from("models").join(&config.model_name),
            // Также проверим альтернативные имена
            PathBuf::from("crates/memory/models/BGE-reranker-v2-m3"),
            PathBuf::from("crates/memory/models/bge-reranker-v2-m3_dynamic_int8_onnx"),
            PathBuf::from("crates/memory/models/qwen3_reranker"),
        ];

        // Определяем имя файла модели в зависимости от типа
        let model_filename = match config.model_name.as_str() {
            "qwen3_reranker" => "model.onnx", // Используем стандартное имя
            _ => "model.onnx",
        };

        let model_dir = possible_paths
            .into_iter()
            .find(|p| p.exists() && p.join(model_filename).exists())
            .ok_or_else(|| {
                AiError::ModelLoadError(format!(
                    "Model '{}' not found in any expected location",
                    config.model_name
                ))
            })?;

        info!("Found model at: {:?}", model_dir);
        let model_path = model_dir.join(model_filename);
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(AiError::ModelLoadError(format!(
                "Model file not found: {model_path:?}"
            )));
        }

        if !tokenizer_path.exists() {
            return Err(AiError::ModelLoadError(format!(
                "Tokenizer file not found: {tokenizer_path:?}"
            )));
        }

        // Load optimized tokenizer with full Qwen3 support
        info!("Loading optimized tokenizer from: {:?}", tokenizer_path);
        let tokenizer = OptimizedTokenizer::new(&tokenizer_path, config.max_length)
            .map_err(|e| AiError::TokenizerError(format!("Failed to load Qwen3 tokenizer: {e}")))?;

        // Create ONNX session with optimization
        #[cfg(feature = "gpu")]
        let mut session_builder = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_memory_pattern(true)?;

        #[cfg(not(feature = "gpu"))]
        let session_builder = Session::builder()?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_memory_pattern(true)?;

        #[cfg(feature = "gpu")]
        if config.use_gpu {
            if let Some(ref gpu_config) = config.gpu_config {
                match gpu_config.create_providers() {
                    Ok(providers) => {
                        if !providers.is_empty() {
                            info!(
                                "🚀 Добавляем {} GPU провайдеров для реранкера",
                                providers.len()
                            );
                            session_builder =
                                session_builder.with_execution_providers(providers)?;
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ Ошибка создания GPU провайдеров: {}. Используем CPU.", e);
                    }
                }
            }
        }

        let session = session_builder.commit_from_file(&model_path)?;

        // Validate model
        let outputs = session.outputs.len();
        let inputs = session.inputs.len();
        info!(
            "✅ Reranker model loaded: {} inputs, {} outputs",
            inputs, outputs
        );

        for (i, input) in session.inputs.iter().enumerate() {
            info!(
                "  Input {}: {} (type: {:?})",
                i, input.name, input.input_type
            );
        }
        for (i, output) in session.outputs.iter().enumerate() {
            info!(
                "  Output {}: {} (type: {:?})",
                i, output.name, output.output_type
            );
        }

        if inputs < 2 {
            warn!(
                "Expected at least 2 inputs for cross-encoder (input_ids, attention_mask), got {}",
                inputs
            );
        }

        let max_length = config.max_length;
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            config,
            max_length,
        })
    }

    /// Rerank documents for a query using real ONNX inference
    pub async fn rerank(&self, query: &str, documents: &[String]) -> Result<Vec<RerankResult>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let start_time = Instant::now();
        info!("🔍 Реранкинг {} документов для запроса", documents.len());

        let mut all_results = Vec::new();

        // Process in batches
        for (batch_idx, chunk) in documents.chunks(self.config.batch_size).enumerate() {
            let batch_start = batch_idx * self.config.batch_size;
            let batch_results = self.process_batch_real(query, chunk, batch_start).await?;
            all_results.extend(batch_results);
        }

        // Sort by score (descending)
        all_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let elapsed = start_time.elapsed();
        info!("✅ Реранкинг завершен за {:?}, топ-3 результата:", elapsed);
        for (i, result) in all_results.iter().take(3).enumerate() {
            info!(
                "  {}. Score: {:.4}, Doc: {}",
                i + 1,
                result.score,
                if result.document.len() > 50 {
                    &result.document[..50]
                } else {
                    &result.document
                }
            );
        }

        Ok(all_results)
    }

    /// Process a batch with real ONNX inference
    async fn process_batch_real(
        &self,
        query: &str,
        documents: &[String],
        start_index: usize,
    ) -> Result<Vec<RerankResult>> {
        let batch_size = documents.len();
        debug!("Processing batch of {} query-document pairs", batch_size);

        // Prepare input tensors
        let mut all_input_ids = Vec::new();
        let mut all_attention_mask = Vec::new();
        let mut all_token_type_ids = Vec::new();

        for document in documents {
            // For Qwen3 reranking: query + document (no special separator needed)
            let combined_text = format!("{query}\n{document}");
            let tokenized = self
                .tokenizer
                .encode(&combined_text)
                .map_err(|e| AiError::TokenizerError(e.to_string()))?;

            let mut input_ids: Vec<i64> = tokenized.input_ids.to_vec();
            let mut attention_mask: Vec<i64> = tokenized.attention_mask.to_vec();
            let mut token_type_ids: Vec<i64> = tokenized.token_type_ids.to_vec();

            // token_type_ids are properly set by the tokenizer

            // Pad or truncate to max_length
            if input_ids.len() > self.max_length {
                input_ids.truncate(self.max_length);
                attention_mask.truncate(self.max_length);
                token_type_ids.truncate(self.max_length);
            } else {
                let pad_len = self.max_length - input_ids.len();
                input_ids.extend(vec![0i64; pad_len]); // PAD token
                attention_mask.extend(vec![0i64; pad_len]);
                token_type_ids.extend(vec![0i64; pad_len]);
            }

            all_input_ids.extend(input_ids);
            all_attention_mask.extend(attention_mask);
            all_token_type_ids.extend(token_type_ids);
        }

        // Create tensors
        let input_ids_array = Array2::from_shape_vec((batch_size, self.max_length), all_input_ids)?;
        let attention_mask_array =
            Array2::from_shape_vec((batch_size, self.max_length), all_attention_mask)?;

        // Convert to ONNX tensors
        let input_ids_tensor = Tensor::from_array((
            input_ids_array.shape().to_vec(),
            input_ids_array.into_raw_vec(),
        ))?;

        let attention_mask_tensor = Tensor::from_array((
            attention_mask_array.shape().to_vec(),
            attention_mask_array.into_raw_vec(),
        ))?;

        let mut session = self.session.lock().await;
        let input_names: Vec<String> = session.inputs.iter().map(|i| i.name.clone()).collect();

        // Run inference based on what inputs the model expects
        let outputs = if input_names.contains(&"token_type_ids".to_string()) {
            let token_type_ids_array =
                Array2::from_shape_vec((batch_size, self.max_length), all_token_type_ids)?;
            let token_type_ids_tensor = Tensor::from_array((
                token_type_ids_array.shape().to_vec(),
                token_type_ids_array.into_raw_vec(),
            ))?;

            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor
            ])?
        } else {
            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor
            ])?
        };

        // ONNX O4 BGE-reranker v2-m3 outputs classification logits directly
        let logits_output = outputs
            .iter()
            .find(|(name, _)| name.contains("logits") || name == &"output_0")
            .or_else(|| outputs.iter().next())
            .ok_or_else(|| AiError::ModelLoadError("No output from reranker model".to_string()))?;

        debug!(
            "Using output '{}' for classification scores",
            logits_output.0
        );

        let output_tensor = &logits_output.1; // Get the value from (name, value) tuple

        // Extract logits/scores
        let (shape, data) = output_tensor.try_extract_tensor::<f32>()?;

        debug!("Reranker output shape: {:?}", shape);

        // Parse results based on output shape
        let mut results = Vec::new();

        if shape.len() == 1 && shape[0] == batch_size as i64 {
            // Direct logits output: [batch_size] - one score per query-document pair
            info!("Processing direct classification logits: shape {:?}", shape);

            for (i, document) in documents.iter().enumerate() {
                let raw_score = data[i];

                debug!("Document {}: raw logit = {:.4}", i, raw_score);

                // BGE-reranker outputs raw logits, higher = more relevant
                // Apply sigmoid to normalize to [0, 1]
                let score = 1.0 / (1.0 + (-raw_score).exp());

                results.push(RerankResult {
                    query: query.to_string(),
                    document: document.clone(),
                    score,
                    original_index: start_index + i,
                });
            }
        } else if shape.len() == 2 && shape[1] == 1 {
            // Single score output: [batch_size, 1]
            info!("Processing single score output: shape {:?}", shape);
            for (i, document) in documents.iter().enumerate() {
                let score = data[i];

                // Apply sigmoid to normalize to [0, 1]
                let normalized_score = 1.0 / (1.0 + (-score).exp());

                results.push(RerankResult {
                    query: query.to_string(),
                    document: document.clone(),
                    score: normalized_score,
                    original_index: start_index + i,
                });
            }
        } else {
            warn!("Unexpected output shape from reranker: {:?}", shape);
            for (name, tensor) in outputs.iter() {
                let (shape, _) = tensor.try_extract_tensor::<f32>()?;
                warn!("  Output '{}': shape {:?}", name, shape);
            }
            return Err(AiError::ModelLoadError(format!("Unexpected output shape from reranker: {shape:?}. Expected [batch_size] or [batch_size, 1] for classification logits.")));
        }

        info!(
            "✅ Successfully processed batch with {} results",
            results.len()
        );
        for (i, result) in results.iter().enumerate() {
            debug!("  Result {}: score={:.4}", i, result.score);
        }
        Ok(results)
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> String {
        format!(
            "OptimizedRerankingService: model={}, batch_size={}, max_length={}, gpu={}",
            self.config.model_name, self.config.batch_size, self.max_length, self.config.use_gpu
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reranker_initialization() {
        let config = RerankingConfig {
            model_name: "bge-reranker-v2-m3_dynamic_int8_onnx".to_string(),
            batch_size: 4,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
        };

        match OptimizedRerankingService::new(config).await {
            Ok(service) => {
                println!("✅ Reranker initialized: {}", service.get_metrics());
            }
            Err(e) => {
                println!("❌ Expected error without model: {}", e);
            }
        }
    }
}
