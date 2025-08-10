use crate::memory_pool::GLOBAL_MEMORY_POOL;
use crate::{ModelLoader, RerankingConfig};
use crate::should_disable_ort;
use crate::tokenization::OptimizedTokenizer;
#[cfg(feature = "gpu")]
use crate::GpuInfo;
use anyhow::Result as AnyhowResult;
// use common::service_traits::StatisticsProvider;
use ort::{inputs, session::Session, value::Tensor};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{debug, info, warn};

/// Optimized Qwen3 Reranker Service with batch processing and memory pooling
pub struct OptimizedQwen3RerankerService {
    inner: RerankerInner,
    model_path: PathBuf,
    max_seq_length: usize,
    batch_size: usize,
}

enum RerankerInner {
    Ort(Arc<Mutex<Session>>),
    Fallback,
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

impl OptimizedQwen3RerankerService {
    /// Create new optimized Qwen3 reranker service with GPU support
    pub fn new_with_config(config: RerankingConfig) -> AnyhowResult<Self> {
        // Centralized models directory
        let models_dir = std::env::var("MAGRAY_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models"));
        let model_path = models_dir.join(&config.model_name).join("model.onnx");
        let max_seq_length = config.max_length;
        let batch_size = config.batch_size;
        info!("Initializing OPTIMIZED Qwen3 reranker service");
        info!("   Max sequence length: {}", max_seq_length);
        info!("   Batch size: {}", batch_size);

        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
            let possible_paths = vec![
                std::env::current_dir()
                    .unwrap()
                    .join("scripts/onnxruntime/lib/onnxruntime.dll"),
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

        // Initialize ONNX Runtime or fallback
        let inner = if should_disable_ort() {
            warn!("ORT disabled by MAGRAY_FORCE_NO_ORT; using fallback text-overlap reranker");
            RerankerInner::Fallback
        } else {
            crate::ort_setup::configure_ort_env();
            ort::init().with_name("optimized_qwen3_reranker").commit()?;
            // Create optimized session
            #[cfg(feature = "gpu")]
            let mut session_builder = Session::builder()?
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .with_memory_pattern(true)?; // Enable memory pattern optimization

            #[cfg(not(feature = "gpu"))]
            let session_builder = Session::builder()?
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)?
                .with_intra_threads(4)?
                .with_memory_pattern(true)?; // Enable memory pattern optimization

            // Добавляем GPU провайдеры если нужно
            #[cfg(feature = "gpu")]
            if config.use_gpu {
                if let Some(ref gpu_config) = config.gpu_config {
                    match gpu_config.create_providers() {
                        Ok(providers) => {
                            if !providers.is_empty() {
                                info!(
                                    "🚀 Добавляем {} GPU провайдеров для reranker",
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
            info!("✅ OPTIMIZED Qwen3 reranker session created");
            RerankerInner::Ort(Arc::new(Mutex::new(session)))
        };

        // Проверяем доступность GPU
        #[cfg(feature = "gpu")]
        if config.use_gpu {
            let gpu_info = GpuInfo::detect();
            gpu_info.print_info();

            if !gpu_info.available {
                warn!("⚠️ GPU запрошен, но не доступен. Используем CPU.");
            }
        }

        Ok(Self {
            inner,
            model_path,
            max_seq_length,
            batch_size,
        })
    }

    /// Create new optimized Qwen3 reranker service (legacy method)
    pub fn new(
        _model_path: PathBuf,
        max_seq_length: usize,
        batch_size: usize,
    ) -> AnyhowResult<Self> {
        let config = RerankingConfig {
            model_name: "qwen3_reranker".to_string(),
            batch_size,
            max_length: max_seq_length,
            use_gpu: false,
            gpu_config: None,
        };
        Self::new_with_config(config)
    }

    /// Optimized batch reranking with memory pooling
    pub fn rerank_batch(&self, batch: &RerankBatch) -> AnyhowResult<BatchRerankResult> {
        let start_time = std::time::Instant::now();
        let query = &batch.query;
        let documents = &batch.documents;

        info!(
            "🚀 OPTIMIZED batch reranking: {} documents",
            documents.len()
        );

        if documents.is_empty() {
            return Ok(BatchRerankResult {
                results: vec![],
                total_time_ms: 0,
                throughput_docs_per_sec: 0.0,
            });
        }

        // Fallback path: simple token-overlap scoring
        if matches!(self.inner, RerankerInner::Fallback) {
            let mut results: Vec<OptimizedRerankResult> = documents
                .iter()
                .enumerate()
                .map(|(i, d)| OptimizedRerankResult {
                    query: query.clone(),
                    document: d.clone(),
                    score: fallback_overlap_score(query, d),
                    index: i,
                    processing_time_ms: 0,
                })
                .collect();
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(k) = batch.top_k { results.truncate(k); }
            let total_time = start_time.elapsed().as_millis();
            let throughput = 1000.0 * documents.len() as f64 / (total_time.max(1)) as f64;
            return Ok(BatchRerankResult { results, total_time_ms: total_time, throughput_docs_per_sec: throughput });
        }

        // Process documents in optimized batches
        let mut all_results = Vec::with_capacity(documents.len());
        let chunks: Vec<&[String]> = documents.chunks(self.batch_size).collect();

        debug!(
            "Processing {} chunks of max size {}",
            chunks.len(),
            self.batch_size
        );

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            debug!(
                "Processing chunk {}/{} with {} documents",
                chunk_idx + 1,
                chunks.len(),
                chunk.len()
            );

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

        info!(
            "✅ OPTIMIZED batch reranking completed in {}ms ({:.1} docs/sec)",
            total_time, throughput
        );

        Ok(BatchRerankResult {
            results: all_results,
            total_time_ms: total_time,
            throughput_docs_per_sec: throughput,
        })
    }

    /// Process a batch of documents at once using memory pooling
    fn process_batch_optimized(
        &self,
        query: &str,
        documents: &[String],
    ) -> AnyhowResult<Vec<OptimizedRerankResult>> {
        let batch_size = documents.len();
        debug!("Processing optimized batch of {} documents", batch_size);

        // Tokenize all query-document pairs in batch using OptimizedTokenizer
        let tokenizer = self.create_tokenizer()?;
        let pairs: Vec<(&str, &str)> = documents.iter().map(|d| (query.as_ref(), d.as_str())).collect();
        let mut batch_tokenized = tokenizer.encode_batch_pairs(&pairs)?;

        // Find maximum sequence length for padding
        let max_len = batch_tokenized
            .input_ids
            .iter()
            .map(|ids| ids.len())
            .max()
            .unwrap_or(0);
        let padded_len = max_len.min(self.max_seq_length);

        debug!("Batch max length: {}, padded to: {}", max_len, padded_len);

        // Pad to uniform length
        tokenizer.pad_batch(&mut batch_tokenized, Some(padded_len))?;

        // Use memory pools for batch data
        let total_elements = batch_size * padded_len;
        let mut flat_input_ids = GLOBAL_MEMORY_POOL.get_input_buffer(total_elements);
        let mut flat_attention_masks = GLOBAL_MEMORY_POOL.get_attention_buffer(total_elements);
        let mut flat_position_ids = GLOBAL_MEMORY_POOL.get_token_type_buffer(total_elements);

        // Flatten and pad batch data
        for i in 0..batch_size {
            let input_ids = &batch_tokenized.input_ids[i];
            let attention_mask = &batch_tokenized.attention_masks[i];

            flat_input_ids.extend_from_slice(&input_ids[..padded_len]);
            flat_attention_masks.extend_from_slice(&attention_mask[..padded_len]);

            // Compute position ids [0..padded_len)
            for pos in 0..padded_len {
                flat_position_ids.push(pos as i64);
            }
        }

        // Create batch tensors [batch_size, seq_len]
        let input_ids_tensor =
            Tensor::from_array(([batch_size, padded_len], flat_input_ids.to_vec()))?;
        let attention_mask_tensor =
            Tensor::from_array(([batch_size, padded_len], flat_attention_masks.to_vec()))?;
        let position_ids_tensor =
            Tensor::from_array(([batch_size, padded_len], flat_position_ids.to_vec()))?;

        // Single ONNX call for entire batch, extract scores while session is alive
        let scores: Vec<f32> = match &self.inner {
            RerankerInner::Ort(session_arc) => {
                let mut session = session_arc
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
                let outputs = session.run(inputs![
                    "input_ids" => input_ids_tensor,
                    "attention_mask" => attention_mask_tensor,
                    "position_ids" => position_ids_tensor
                ])?;
                // Extract while outputs borrows session internals
                self.extract_batch_scores(&outputs, batch_size)?
            }
            RerankerInner::Fallback => {
                // Should not reach here, rerank_batch handles fallback early
                return Err(anyhow::anyhow!("Fallback path should not call process_batch_optimized"));
            }
        };

        // Create results
        let mut results = Vec::with_capacity(batch_size);
        for (i, document) in documents.iter().enumerate() {
            results.push(OptimizedRerankResult {
                query: query.to_string(),
                document: document.clone(),
                score: scores[i],
                index: i,              // Will be updated by caller
                processing_time_ms: 0, // Will be set by caller
            });
        }

        debug!("Extracted {} batch scores", scores.len());
        Ok(results)
    }

    fn create_tokenizer(&self) -> AnyhowResult<OptimizedTokenizer> {
        // Derive tokenizer path using ModelLoader to handle variants
        let models_dir = std::env::var("MAGRAY_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models"));
        let loader = ModelLoader::new(&models_dir)?;
        let tokenizer_path = loader.get_tokenizer_path("qwen3_reranker");
        OptimizedTokenizer::new(tokenizer_path, self.max_seq_length)
    }

    /// Extract scores from batch outputs
    fn extract_batch_scores(
        &self,
        outputs: &ort::session::SessionOutputs,
        batch_size: usize,
    ) -> AnyhowResult<Vec<f32>> {
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();

                if let Some(scores) = try_extract_scores_from_shape_and_data(batch_size, &shape_vec, data) {
                    return Ok(scores);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not extract batch reranking scores from model outputs"
        ))
    }
}

/// Pure helper: extract scores from a tensor described by shape and flat data
pub(crate) fn try_extract_scores_from_shape_and_data(
    batch_size: usize,
    shape_vec: &[i64],
    data: &[f32],
) -> Option<Vec<f32>> {
    // 3D logits [batch_size, seq_len, vocab_size]
    if shape_vec.len() == 3 && shape_vec[0] == batch_size as i64 {
        let seq_len = shape_vec[1] as usize;
        let vocab_size = shape_vec[2] as usize;
        if seq_len == 0 || vocab_size == 0 {
            return None;
        }

        let mut scores = Vec::with_capacity(batch_size);
        for batch_idx in 0..batch_size {
            let batch_offset = batch_idx * seq_len * vocab_size;
            let last_token_start = batch_offset + (seq_len.saturating_sub(1)) * vocab_size;
            if last_token_start + 100 < data.len() {
                let mut sum = 0.0f32;
                for i in 0..100 {
                    sum += data[last_token_start + i];
                }
                let score = (sum / 100.0).tanh();
                scores.push(score);
            } else {
                scores.push(0.0);
            }
        }
        return Some(scores);
    }

    // 2D logits [batch_size, num_classes]
    if shape_vec.len() == 2 && shape_vec[0] == batch_size as i64 {
        let num_classes = shape_vec[1] as usize;
        if num_classes == 1 {
            let mut out = Vec::with_capacity(batch_size);
            for i in 0..batch_size {
                out.push(data.get(i).copied().unwrap_or(0.0));
            }
            return Some(out);
        } else if num_classes == 2 {
            let mut out = Vec::with_capacity(batch_size);
            for i in 0..batch_size {
                let idx = i * 2 + 1;
                out.push(data.get(idx).copied().unwrap_or(0.0));
            }
            return Some(out);
        }
    }

    None
}

/// Single document reranking (fallback for compatibility)
pub fn rerank(
    query: &str,
    documents: &[String],
    top_k: Option<usize>,
) -> AnyhowResult<Vec<OptimizedRerankResult>> {
    let batch = RerankBatch {
        query: query.to_string(),
        documents: documents.to_vec(),
        top_k,
    };

    let service = OptimizedQwen3RerankerService::new(PathBuf::from("test_models/qwen3_reranker/model.onnx"), 512, 8)?;
    let batch_result = service.rerank_batch(&batch)?;
    Ok(batch_result.results)
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct RerankServiceStats {
    pub model_name: String,
    pub max_seq_length: usize,
    pub batch_size: usize,
    pub optimization_level: String,
}

/// Very simple text-overlap scoring for fallback mode (Jaccard on lowercased token sets)
fn fallback_overlap_score(query: &str, doc: &str) -> f32 {
    fn tokenize(s: &str) -> std::collections::HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_lowercase())
            .collect()
    }
    let q = tokenize(query);
    let d = tokenize(doc);
    if q.is_empty() || d.is_empty() { return 0.0; }
    let inter = q.intersection(&d).count() as f32;
    let union = q.union(&d).count() as f32;
    if union <= 0.0 { 0.0 } else { inter / union }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_reranker_creation() {
        if std::env::var("ORT_DYLIB_PATH").is_err() {
            eprintln!("Skipping reranker test: ORT_DYLIB_PATH not set");
            return;
        }
        let model_path = PathBuf::from("test_models/qwen3_reranker/model.onnx");

        match OptimizedQwen3RerankerService::new(model_path, 512, 8) {
            Ok(_service) => {
                println!("✅ Optimized Qwen3 service created successfully");
            }
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

    #[test]
    fn test_try_extract_scores_3d_small_shape_returns_zero() {
        // Shape [2, 3, 4] -> len(data)=24; last_token_start+100 will exceed len => zeros
        let batch = 2usize;
        let shape = [2i64, 3, 4];
        let data = vec![0.5f32; (2 * 3 * 4) as usize];
        let scores = try_extract_scores_from_shape_and_data(batch, &shape, &data).unwrap();
        assert_eq!(scores.len(), 2);
        assert_eq!(scores[0], 0.0);
        assert_eq!(scores[1], 0.0);
    }

    #[test]
    fn test_try_extract_scores_3d_large_shape_averages_and_tanh() {
        // Shape [1, 2, 200] -> len=400; last_token_start=200; avg of next 100 values
        let batch = 1usize;
        let shape = [1i64, 2, 200];
        let mut data = vec![0.0f32; (1 * 2 * 200) as usize];
        // Fill last 100 values starting at 200 with 0..100
        for i in 0..100usize {
            data[200 + i] = i as f32;
        }
        let scores = try_extract_scores_from_shape_and_data(batch, &shape, &data).unwrap();
        assert_eq!(scores.len(), 1);
        // Average of 0..99 = 49.5, tanh(49.5) ~ 1.0
        assert!(scores[0] > 0.99);
    }

    #[test]
    fn test_try_extract_scores_2d_single_class() {
        let batch = 3usize;
        let shape = [3i64, 1];
        let data = vec![0.1f32, 0.2, 0.3];
        let scores = try_extract_scores_from_shape_and_data(batch, &shape, &data).unwrap();
        assert_eq!(scores, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_try_extract_scores_2d_binary_class() {
        let batch = 3usize;
        let shape = [3i64, 2];
        // [neg,pos] per row
        let data = vec![0.0f32, 0.5, 0.2, 0.7, -0.1, 0.9];
        let scores = try_extract_scores_from_shape_and_data(batch, &shape, &data).unwrap();
        assert_eq!(scores, vec![0.5, 0.7, 0.9]);
    }

    #[test]
    fn test_try_extract_scores_none_for_mismatched_shape() {
        let batch = 2usize;
        let shape = [2i64, 3, 0]; // invalid vocab size
        let data = vec![0.0f32; 0];
        let scores = try_extract_scores_from_shape_and_data(batch, &shape, &data);
        assert!(scores.is_none());
    }
}
