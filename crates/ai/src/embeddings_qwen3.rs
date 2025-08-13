use crate::memory_pool::GLOBAL_MEMORY_POOL;
use crate::should_disable_ort;
use crate::tokenization::OptimizedTokenizer;
#[cfg(feature = "gpu")]
use crate::GpuInfo;
use crate::{EmbeddingConfig, ModelLoader};
use anyhow::Result as AnyhowResult;
use ort::{inputs, session::Session, value::Tensor};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{debug, info, warn};

/// Qwen3 Embedding Provider with batch processing and memory pooling
pub struct Qwen3EmbeddingProvider {
    inner: EmbeddingInner,
    model_path: PathBuf,
    max_seq_length: usize,
    batch_size: usize,
}

enum EmbeddingInner {
    Ort(Arc<Mutex<Session>>),
    Fallback,
}

impl Qwen3EmbeddingProvider {
    /// Create new Qwen3 embedding provider with GPU support
    pub fn new_with_config(config: EmbeddingConfig) -> AnyhowResult<Self> {
        // Centralized models directory
        let models_dir = std::env::var("MAGRAY_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models"));
        let model_path = models_dir.join(&config.model_name).join("model.onnx");
        let max_seq_length = config.max_length;
        let batch_size = config.batch_size;
        info!("Initializing Qwen3 embedding provider");
        info!("   Max sequence length: {}", max_seq_length);
        info!("   Batch size: {}", batch_size);
        info!("   Model path: {}", model_path.display());

        #[cfg(target_os = "windows")]
        {
            let possible_paths = vec![
                std::env::current_dir()
                    .expect("Operation should succeed")
                    .join("scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("./scripts/onnxruntime/lib/onnxruntime.dll"),
            ];

            for dll_path in possible_paths {
                if dll_path.exists() {
                    info!("Found ORT library at: {}", dll_path.display());
                    std::env::set_var(
                        "ORT_DYLIB_PATH",
                        dll_path.to_str().expect("Operation should succeed"),
                    );
                    break;
                }
            }
        }

        // Initialize ONNX Runtime or fallback
        let inner = if should_disable_ort() {
            warn!("ORT disabled by MAGRAY_FORCE_NO_ORT; using fallback text-overlap reranker");
            EmbeddingInner::Fallback
        } else {
            crate::ort_setup::configure_ort_env();
            ort::init().with_name("qwen3_embedding_provider").commit()?;
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

            // Add GPU providers if needed
            #[cfg(feature = "gpu")]
            if config.use_gpu {
                if let Some(ref gpu_config) = config.gpu_config {
                    match gpu_config.create_providers() {
                        Ok(providers) => {
                            if !providers.is_empty() {
                                info!("🚀 Adding {} GPU providers for embedding", providers.len());
                                session_builder =
                                    session_builder.with_execution_providers(providers)?;
                            }
                        }
                        Err(e) => {
                            warn!("⚠️ Error creating GPU providers: {}. Using CPU.", e);
                        }
                    }
                }
            }

            let session = session_builder.commit_from_file(&model_path)?;
            info!("✅ Qwen3 embedding session created");
            EmbeddingInner::Ort(Arc::new(Mutex::new(session)))
        };

        // Check GPU availability
        #[cfg(feature = "gpu")]
        if config.use_gpu {
            let gpu_info = GpuInfo::detect();
            gpu_info.print_info();

            if !gpu_info.available {
                warn!("⚠️ GPU requested but not available. Using CPU.");
            }
        }

        Ok(Self {
            inner,
            model_path,
            max_seq_length,
            batch_size,
        })
    }

    /// Create new Qwen3 embedding provider (legacy method)
    pub fn new(
        _model_path: PathBuf,
        max_seq_length: usize,
        batch_size: usize,
    ) -> AnyhowResult<Self> {
        let config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            batch_size,
            max_length: max_seq_length,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        };
        Self::new_with_config(config)
    }

    /// Embed a single text
    pub fn embed_text(&self, text: &str) -> AnyhowResult<Vec<f32>> {
        let tokenizer = self.create_tokenizer()?;
        let tokenized = tokenizer.encode(text)?;
        let max_len = tokenized.input_ids.len().min(self.max_seq_length);

        let mut flat_input_ids = GLOBAL_MEMORY_POOL.get_input_buffer(max_len);
        let mut flat_attention_masks = GLOBAL_MEMORY_POOL.get_attention_buffer(max_len);

        flat_input_ids.extend_from_slice(&tokenized.input_ids[..max_len]);
        flat_attention_masks.extend_from_slice(&tokenized.attention_mask[..max_len]);

        let input_ids_tensor = Tensor::from_array(([1, max_len], flat_input_ids.to_vec()))?;
        let attention_mask_tensor =
            Tensor::from_array(([1, max_len], flat_attention_masks.to_vec()))?;

        let scores: Vec<f32> = match &self.inner {
            EmbeddingInner::Ort(session_arc) => {
                let mut session = session_arc
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
                let outputs = session.run(inputs![
                    "input_ids" => input_ids_tensor,
                    "attention_mask" => attention_mask_tensor
                ])?;
                self.extract_scores(&outputs)?
            }
            EmbeddingInner::Fallback => {
                return Err(anyhow::anyhow!(
                    "Fallback path not implemented for embeddings"
                ));
            }
        };

        Ok(scores)
    }

    /// Embed a batch of texts
    pub fn embed_batch(&self, texts: &[String]) -> AnyhowResult<Vec<Vec<f32>>> {
        let tokenizer = self.create_tokenizer()?;
        let mut all_results = Vec::with_capacity(texts.len());

        for text in texts {
            let result = self.embed_text(text)?;
            all_results.push(result);
        }

        Ok(all_results)
    }

    fn create_tokenizer(&self) -> AnyhowResult<OptimizedTokenizer> {
        // Derive tokenizer path using ModelLoader to handle variants
        let models_dir = std::env::var("MAGRAY_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models"));
        let loader = ModelLoader::new(&models_dir)?;
        let tokenizer_path = loader.get_tokenizer_path("qwen3emb");
        OptimizedTokenizer::new(tokenizer_path, self.max_seq_length)
    }

    /// Extract scores from outputs
    fn extract_scores(&self, outputs: &ort::session::SessionOutputs) -> AnyhowResult<Vec<f32>> {
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();

                if let Some(scores) = try_extract_scores_from_shape_and_data(1, &shape_vec, data) {
                    return Ok(scores);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not extract embedding scores from model outputs"
        ))
    }
}

/// Pure helper: extract scores from a tensor described by shape and flat data
pub(crate) fn try_extract_scores_from_shape_and_data(
    batch_size: usize,
    shape_vec: &[i64],
    data: &[f32],
) -> Option<Vec<f32>> {
    // 2D logits [batch_size, embedding_size]
    if shape_vec.len() == 2 && shape_vec[0] == batch_size as i64 {
        let embedding_size = shape_vec[1] as usize;
        let mut out = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            out.extend_from_slice(&data[i * embedding_size..(i + 1) * embedding_size]);
        }
        return Some(out);
    }

    None
}
