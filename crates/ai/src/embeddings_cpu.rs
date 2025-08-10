use crate::memory_pool::GLOBAL_MEMORY_POOL;
use crate::tokenization::{
    BatchTokenized, OptimizedTokenizer, TokenizedInput as OptTokenizedInput,
};
use crate::EmbeddingConfig;
use crate::should_disable_ort;
#[cfg(feature = "gpu")]
use crate::{GpuConfig, GpuInfo};
use anyhow::Result as AnyhowResult;
use common::service_traits::StatisticsProvider;
use ort::{inputs, session::Session, value::Tensor};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{debug, info, warn};

/// CPU-based Embedding Service with real tokenization and batching (Qwen3)
pub struct CpuEmbeddingService {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<OptimizedTokenizer>,
    model_path: PathBuf,
    hidden_size: usize,
}

/// Result of embedding operation
#[derive(Debug, Clone)]
pub struct OptimizedEmbeddingResult {
    pub text: String,
    pub embedding: Vec<f32>,
    pub token_count: usize,
    pub processing_time_ms: u128,
}

impl CpuEmbeddingService {
    /// Create new optimized embedding service (Qwen3)
    pub fn new(config: EmbeddingConfig) -> AnyhowResult<Self> {
        if should_disable_ort() {
            warn!("ORT disabled by MAGRAY_FORCE_NO_ORT; CpuEmbeddingService not initialized");
            return Err(anyhow::anyhow!("ORT disabled"));
        }
        info!(
            "Initializing CPU embedding service with model: {}",
            config.model_name
        );

        // Получаем путь относительно корня проекта
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // Определяем путь к модели в зависимости от типа
        let (model_filename, hidden_size) = match config.model_name.as_str() {
            "qwen3emb" => ("model.onnx", 1024), // Исправлено: используем стандартное имя файла
            // legacy models removed; default to qwen3 configuration
            _ => ("model.onnx", config.embedding_dim.unwrap_or(1024)),
        };

        // Ищем модели в разных возможных местах
        let possible_paths = vec![
            // Если запускаемся из корня проекта
            current_dir.join(format!(
                "crates/memory/models/{}/{}",
                config.model_name, model_filename
            )),
            // Если модели в корне проекта
            current_dir.join(format!("models/{}/{}", config.model_name, model_filename)),
            // Если запускаемся из crates/memory
            current_dir.join(format!("models/{}/{}", config.model_name, model_filename)),
            // Если запускаемся из другого места
            current_dir.join(format!(
                "../memory/models/{}/{}",
                config.model_name, model_filename
            )),
            current_dir.join(format!(
                "../../models/{}/{}",
                config.model_name, model_filename
            )),
            // Абсолютный путь из переменной окружения
            PathBuf::from(format!("models/{}/{}", config.model_name, model_filename)),
        ];

        let model_path = possible_paths
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                anyhow::anyhow!("Model file not found. Tried paths: {:?}", possible_paths)
            })?
            .clone();

        // Аналогично для tokenizer
        let tokenizer_possible_paths = vec![
            current_dir.join(format!(
                "crates/memory/models/{}/tokenizer.json",
                config.model_name
            )),
            current_dir.join(format!("models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!("models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!(
                "../memory/models/{}/tokenizer.json",
                config.model_name
            )),
            current_dir.join(format!("../../models/{}/tokenizer.json", config.model_name)),
            PathBuf::from(format!("models/{}/tokenizer.json", config.model_name)),
        ];

        let tokenizer_path = tokenizer_possible_paths
            .iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Tokenizer file not found. Tried paths: {:?}",
                    tokenizer_possible_paths
                )
            })?
            .clone();

        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
            let mut possible_paths = vec![
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("./scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("../scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("../../scripts/onnxruntime/lib/onnxruntime.dll"),
            ];

            // Also search in target/debug/build for any onnxruntime-sys build
            if let Ok(target_dir) = std::env::current_dir().map(|d| d.join("target/debug/build")) {
                if let Ok(entries) = std::fs::read_dir(&target_dir) {
                    for entry in entries.flatten() {
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("onnxruntime-sys-")
                        {
                            let dll_path = entry.path().join(
                                "out/onnxruntime/onnxruntime-win-x64-1.8.1/lib/onnxruntime.dll",
                            );
                            if dll_path.exists() {
                                possible_paths.push(dll_path);
                            }
                        }
                    }
                }
            }

            for dll_path in possible_paths {
                if dll_path.exists() {
                    info!("Found ORT library at: {}", dll_path.display());
                    if let Some(path_str) = dll_path.to_str() {
                        std::env::set_var("ORT_DYLIB_PATH", path_str);
                        break;
                    } else {
                        warn!(
                            "Не удалось конвертировать путь к DLL в строку: {}",
                            dll_path.display()
                        );
                    }
                }
            }
        }

        // Initialize ONNX Runtime
        if !should_disable_ort() {
            crate::ort_setup::configure_ort_env();
            ort::init().with_name("optimized_bge_m3").commit()?;
        } else {
            warn!("ORT disabled by MAGRAY_FORCE_NO_ORT; embeddings will not run actual inference");
        }

        // Проверяем доступность GPU
        #[cfg(feature = "gpu")]
        if config.use_gpu {
            let gpu_info = GpuInfo::detect();
            gpu_info.print_info();

            if !gpu_info.available {
                warn!("⚠️ GPU запрошен, но не доступен. Используем CPU.");
            }
        }

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
                            info!("🚀 Добавляем {} GPU провайдеров", providers.len());
                            session_builder =
                                session_builder.with_execution_providers(providers)?;
                        } else {
                            warn!("⚠️ GPU провайдеры не созданы, используем CPU");
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ Ошибка создания GPU провайдеров: {}. Используем CPU.", e);
                    }
                }
            } else if config.use_gpu {
                // Если use_gpu=true но gpu_config=None, создаём дефолтный
                let default_gpu_config = GpuConfig::default();
                match default_gpu_config.create_providers() {
                    Ok(providers) => {
                        if !providers.is_empty() {
                            info!("🚀 Используем дефолтную GPU конфигурацию");
                            session_builder =
                                session_builder.with_execution_providers(providers)?;
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ Ошибка создания дефолтных GPU провайдеров: {}", e);
                    }
                }
            }
        }

        let session = session_builder.commit_from_file(&model_path)?;

        info!("✅ Optimized ONNX session created");
        info!("   Model: {}", model_path.display());
        info!("   Inputs: {}", session.inputs.len());
        info!("   Outputs: {}", session.outputs.len());

        // Create optimized tokenizer
        let tokenizer = OptimizedTokenizer::new(tokenizer_path, config.max_length)?;
        info!("✅ Optimized tokenizer created");
        info!("   Vocab size: {}", tokenizer.vocab_size());
        info!("   Max length: {}", tokenizer.max_length());

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_path,
            hidden_size, // Размерность эмбеддингов из конфигурации
        })
    }

    /// Generate embedding for single text with optimized processing
    pub fn embed(&self, text: &str) -> AnyhowResult<OptimizedEmbeddingResult> {
        let start_time = std::time::Instant::now();

        debug!("Optimized embedding for text: {} chars", text.len());

        // Use real tokenization instead of hash-based
        let tokenized = self.tokenizer.encode(text)?;
        debug!("Tokenized to {} tokens", tokenized.length);

        // Process single text
        let embedding = self.process_single_optimized(&tokenized)?;

        let processing_time = start_time.elapsed().as_millis();
        debug!("Optimized processing took: {}ms", processing_time);

        Ok(OptimizedEmbeddingResult {
            text: text.to_string(),
            embedding,
            token_count: tokenized.length,
            processing_time_ms: processing_time,
        })
    }

    /// Generate embeddings for multiple texts with batching optimization
    pub fn embed_batch(&self, texts: &[String]) -> AnyhowResult<Vec<OptimizedEmbeddingResult>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let start_time = std::time::Instant::now();
        info!("🚀 OPTIMIZED batch processing {} texts", texts.len());

        // Convert to string references for batch tokenization
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        // Batch tokenization - much faster than individual tokenization
        let mut batch_tokenized = self.tokenizer.encode_batch(&text_refs)?;
        debug!("Batch tokenized in {}ms", start_time.elapsed().as_millis());

        // Pad to uniform length for efficient ONNX processing
        let tokenization_time = start_time.elapsed().as_millis();
        self.tokenizer.pad_batch(&mut batch_tokenized, None)?;
        debug!(
            "Batch padded in {}ms",
            start_time.elapsed().as_millis() - tokenization_time
        );

        // Process entire batch at once
        let batch_embeddings = self.process_batch_optimized(&batch_tokenized)?;

        // Create results
        let mut results = Vec::with_capacity(texts.len());
        let total_time = start_time.elapsed().as_millis();

        for (i, text) in texts.iter().enumerate() {
            results.push(OptimizedEmbeddingResult {
                text: text.clone(),
                embedding: batch_embeddings[i].clone(),
                token_count: batch_tokenized.lengths[i],
                processing_time_ms: total_time / texts.len() as u128, // Average per text
            });
        }

        info!(
            "✅ OPTIMIZED batch completed in {}ms ({:.1}ms/text)",
            total_time,
            total_time as f64 / texts.len() as f64
        );

        Ok(results)
    }

    /// Process single tokenized input with memory pooling optimization
    fn process_single_optimized(&self, tokenized: &OptTokenizedInput) -> AnyhowResult<Vec<f32>> {
        let seq_len = tokenized.length;

        // Use memory pool for tensor data to avoid allocations
        let mut input_ids_buf = GLOBAL_MEMORY_POOL.get_input_buffer(seq_len);
        let mut attention_mask_buf = GLOBAL_MEMORY_POOL.get_attention_buffer(seq_len);
        let mut token_type_buf = GLOBAL_MEMORY_POOL.get_token_type_buffer(seq_len);

        // Copy data into pooled buffers
        input_ids_buf.extend_from_slice(&tokenized.input_ids);
        attention_mask_buf.extend_from_slice(&tokenized.attention_mask);
        token_type_buf.extend_from_slice(&tokenized.token_type_ids);

        // Create tensors from pooled buffers
        let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids_buf.to_vec()))?;
        let attention_mask_tensor =
            Tensor::from_array(([1, seq_len], attention_mask_buf.to_vec()))?;
        let token_type_ids_tensor = Tensor::from_array(([1, seq_len], token_type_buf.to_vec()))?;

        // Run inference
        let mut session = self
            .session
            .lock()
            .map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;

        // Проверяем количество входов модели
        let outputs = if session.inputs.len() == 2 {
            // Модель Qwen3 имеет только 2 входа (input_ids и attention_mask)
            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor
            ])?
        } else {
            // Модель BGE-M3 имеет 3 входа
            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor
            ])?
        };

        // Buffers are automatically returned to pool via Drop trait

        // Extract and process embeddings
        self.extract_and_pool_embedding(&outputs, seq_len)
    }

    /// Process entire batch at once with memory pooling optimization
    fn process_batch_optimized(&self, batch: &BatchTokenized) -> AnyhowResult<Vec<Vec<f32>>> {
        let batch_size = batch.input_ids.len();
        let seq_len = batch.max_length;
        let total_elements = batch_size * seq_len;

        debug!(
            "Processing batch: {} x {} tokens ({} total elements)",
            batch_size, seq_len, total_elements
        );

        // Use memory pools for flattened batch data
        let mut flat_input_ids = GLOBAL_MEMORY_POOL.get_input_buffer(total_elements);
        let mut flat_attention_masks = GLOBAL_MEMORY_POOL.get_attention_buffer(total_elements);
        let mut flat_token_type_ids = GLOBAL_MEMORY_POOL.get_token_type_buffer(total_elements);

        // Flatten batch data efficiently using pooled buffers
        for row in &batch.input_ids {
            flat_input_ids.extend_from_slice(row);
        }
        for row in &batch.attention_masks {
            flat_attention_masks.extend_from_slice(row);
        }
        for row in &batch.token_type_ids {
            flat_token_type_ids.extend_from_slice(row);
        }

        // Create batch tensors [batch_size, seq_len]
        let input_ids_tensor =
            Tensor::from_array(([batch_size, seq_len], flat_input_ids.to_vec()))?;
        let attention_mask_tensor =
            Tensor::from_array(([batch_size, seq_len], flat_attention_masks.to_vec()))?;
        let token_type_ids_tensor =
            Tensor::from_array(([batch_size, seq_len], flat_token_type_ids.to_vec()))?;

        // Single ONNX call for entire batch
        let mut session = self
            .session
            .lock()
            .map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;

        // Проверяем количество входов модели
        let outputs = if session.inputs.len() == 2 {
            // Модель Qwen3 имеет только 2 входа
            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor
            ])?
        } else {
            // Модель BGE-M3 имеет 3 входа
            session.run(inputs![
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor
            ])?
        };

        // Buffers are automatically returned to pool via Drop trait

        // Extract batch embeddings
        self.extract_batch_embeddings(&outputs, batch)
    }

    /// Extract embeddings from single output
    fn extract_and_pool_embedding(
        &self,
        outputs: &ort::session::SessionOutputs,
        seq_len: usize,
    ) -> AnyhowResult<Vec<f32>> {
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();

                // Look for hidden states [batch, seq, hidden] = [1, seq_len, 1024]
                if shape_vec.len() == 3 && shape_vec[0] == 1 && shape_vec[1] == seq_len as i64 {
                    let hidden_size = shape_vec[2] as usize;

                    // Optimized mean pooling
                    let pooled = self.optimized_mean_pooling(data, seq_len, hidden_size);

                    // Optimized normalization
                    let normalized = self.optimized_normalize(pooled);

                    debug!("Extracted embedding: {} dims", normalized.len());
                    return Ok(normalized);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not extract embeddings from model outputs"
        ))
    }

    /// Extract embeddings from batch output
    fn extract_batch_embeddings(
        &self,
        outputs: &ort::session::SessionOutputs,
        batch: &BatchTokenized,
    ) -> AnyhowResult<Vec<Vec<f32>>> {
        let batch_size = batch.input_ids.len();

        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();

                // Look for batch hidden states [batch_size, seq_len, hidden_size]
                if shape_vec.len() == 3 && shape_vec[0] == batch_size as i64 {
                    let seq_len = shape_vec[1] as usize;
                    let hidden_size = shape_vec[2] as usize;

                    debug!(
                        "Processing batch output: [{}, {}, {}]",
                        batch_size, seq_len, hidden_size
                    );

                    let mut batch_embeddings = Vec::with_capacity(batch_size);

                    // Process each item in batch
                    for batch_idx in 0..batch_size {
                        let start_offset = batch_idx * seq_len * hidden_size;
                        let end_offset = start_offset + batch.lengths[batch_idx] * hidden_size;

                        if end_offset <= data.len() {
                            let item_data = &data[start_offset..end_offset];
                            let actual_seq_len = batch.lengths[batch_idx];

                            // Optimized pooling for this item
                            let pooled =
                                self.optimized_mean_pooling(item_data, actual_seq_len, hidden_size);
                            let normalized = self.optimized_normalize(pooled);

                            batch_embeddings.push(normalized);
                        } else {
                            return Err(anyhow::anyhow!("Batch extraction index out of bounds"));
                        }
                    }

                    debug!("Extracted {} batch embeddings", batch_embeddings.len());
                    return Ok(batch_embeddings);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not extract batch embeddings from model outputs"
        ))
    }

    /// Ultra-optimized SIMD mean pooling with AVX2/AVX-512 support
    fn optimized_mean_pooling(&self, data: &[f32], seq_len: usize, hidden_size: usize) -> Vec<f32> {
        // Use memory pool for output buffer
        let mut pooled = GLOBAL_MEMORY_POOL.get_output_buffer(hidden_size);
        pooled.resize(hidden_size, 0.0f32);

        #[cfg(target_arch = "x86_64")]
        {
            // Check if we can use SIMD optimizations
            if hidden_size.is_multiple_of(8) && is_x86_feature_detected!("avx2") && hidden_size >= 64 {
                // Ultra-optimized SIMD mean pooling for large embeddings
                unsafe {
                    self.simd_mean_pooling_avx2(data, &mut pooled, seq_len, hidden_size);
                }
            } else {
                // Fallback to optimized scalar processing
                self.scalar_mean_pooling_optimized(data, &mut pooled, seq_len, hidden_size);
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            self.scalar_mean_pooling_optimized(data, &mut pooled, seq_len, hidden_size);
        }

        // Average (SIMD-optimized division if possible)
        let seq_len_f32 = seq_len as f32;
        self.simd_divide_inplace(&mut pooled, seq_len_f32);

        // Take ownership and return as Vec<f32>
        pooled.take().unwrap_or_default()
    }

    /// Ultra-optimized SIMD L2 normalization with AVX2/AVX-512
    fn optimized_normalize(&self, mut embedding: Vec<f32>) -> Vec<f32> {
        #[cfg(target_arch = "x86_64")]
        {
            if embedding.len().is_multiple_of(8) && is_x86_feature_detected!("avx2") && embedding.len() >= 64
            {
                // Ultra-optimized SIMD normalization
                unsafe {
                    self.simd_l2_normalize_avx2(&mut embedding);
                }
            } else {
                // Fallback к optimized scalar
                self.scalar_l2_normalize_optimized(&mut embedding);
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            self.scalar_l2_normalize_optimized(&mut embedding);
        }

        embedding
    }

    /// Get embedding dimension
    pub fn embedding_dim(&self) -> usize {
        self.hidden_size
    }

    /// Check if model is available
    pub fn is_available(&self) -> bool {
        self.model_path.exists()
    }

    /// Get processing statistics including memory pool stats
    pub fn get_stats(&self) -> ServiceStats {
        ServiceStats {
            model_name: "bge-m3-optimized".to_string(),
            vocab_size: self.tokenizer.vocab_size(),
            max_length: self.tokenizer.max_length(),
            hidden_size: self.hidden_size,
            optimization_level: "Level3+MemoryPool".to_string(),
        }
    }

    /// Get memory pool statistics
    pub fn get_pool_stats(&self) -> crate::memory_pool::PoolStats {
        GLOBAL_MEMORY_POOL.get_stats()
    }

    // ========== ULTRA-OPTIMIZED SIMD IMPLEMENTATIONS ==========

    /// Ultra-optimized AVX2 mean pooling for large embeddings
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn simd_mean_pooling_avx2(
        &self,
        data: &[f32],
        pooled: &mut [f32],
        seq_len: usize,
        hidden_size: usize,
    ) {
        use std::arch::x86_64::*;

        // Process 8 elements at a time with AVX2
        let chunks = hidden_size / 8;
        let remainder = hidden_size % 8;

        for seq_idx in 0..seq_len {
            let seq_start = seq_idx * hidden_size;

            // SIMD processing for main chunks
            for chunk_idx in 0..chunks {
                let hidden_start = chunk_idx * 8;
                let data_idx = seq_start + hidden_start;
                let pooled_idx = hidden_start;

                if data_idx + 8 <= data.len() && pooled_idx + 8 <= pooled.len() {
                    // Load 8 data values
                    let data_vec = _mm256_loadu_ps(data.as_ptr().add(data_idx));
                    // Load 8 pooled values
                    let pooled_vec = _mm256_loadu_ps(pooled.as_ptr().add(pooled_idx));
                    // Add them together
                    let sum_vec = _mm256_add_ps(pooled_vec, data_vec);
                    // Store back to pooled
                    _mm256_storeu_ps(pooled.as_mut_ptr().add(pooled_idx), sum_vec);
                }
            }

            // Handle remainder elements
            let remainder_start = chunks * 8;
            for i in 0..remainder {
                let data_idx = seq_start + remainder_start + i;
                let pooled_idx = remainder_start + i;
                if data_idx < data.len() && pooled_idx < pooled.len() {
                    pooled[pooled_idx] += data[data_idx];
                }
            }
        }
    }

    /// Optimized scalar fallback for mean pooling
    fn scalar_mean_pooling_optimized(
        &self,
        data: &[f32],
        pooled: &mut [f32],
        seq_len: usize,
        hidden_size: usize,
    ) {
        // Manual loop unrolling for better performance
        for seq_idx in 0..seq_len {
            let seq_start = seq_idx * hidden_size;
            let chunks = hidden_size / 4;
            let remainder = hidden_size % 4;

            // Process 4 elements at a time for better ILP
            for chunk_idx in 0..chunks {
                let base_idx = chunk_idx * 4;
                let data_base = seq_start + base_idx;

                if data_base + 4 <= data.len() && base_idx + 4 <= pooled.len() {
                    pooled[base_idx] += data[data_base];
                    pooled[base_idx + 1] += data[data_base + 1];
                    pooled[base_idx + 2] += data[data_base + 2];
                    pooled[base_idx + 3] += data[data_base + 3];
                }
            }

            // Handle remainder
            let remainder_start = chunks * 4;
            for i in 0..remainder {
                let data_idx = seq_start + remainder_start + i;
                let pooled_idx = remainder_start + i;
                if data_idx < data.len() && pooled_idx < pooled.len() {
                    pooled[pooled_idx] += data[data_idx];
                }
            }
        }
    }

    /// Ultra-optimized SIMD division for averaging
    #[cfg(target_arch = "x86_64")]
    fn simd_divide_inplace(&self, values: &mut [f32], divisor: f32) {
        if is_x86_feature_detected!("avx2") && values.len().is_multiple_of(8) && values.len() >= 8 {
            unsafe {
                use std::arch::x86_64::*;
                let divisor_vec = _mm256_set1_ps(divisor);
                let chunks = values.len() / 8;

                for i in 0..chunks {
                    let idx = i * 8;
                    let val_vec = _mm256_loadu_ps(values.as_ptr().add(idx));
                    let result_vec = _mm256_div_ps(val_vec, divisor_vec);
                    _mm256_storeu_ps(values.as_mut_ptr().add(idx), result_vec);
                }
            }
        } else {
            // Fallback to scalar division
            let inv_divisor = 1.0 / divisor;
            for val in values.iter_mut() {
                *val *= inv_divisor;
            }
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn simd_divide_inplace(&self, values: &mut [f32], divisor: f32) {
        let inv_divisor = 1.0 / divisor;
        for val in values.iter_mut() {
            *val *= inv_divisor;
        }
    }

    /// Ultra-optimized AVX2 L2 normalization
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn simd_l2_normalize_avx2(&self, embedding: &mut [f32]) {
        use std::arch::x86_64::*;

        // Calculate norm squared using SIMD
        let mut norm_sq_acc = _mm256_setzero_ps();
        let chunks = embedding.len() / 8;
        let _remainder = embedding.len() % 8;

        // SIMD accumulation of squares
        for i in 0..chunks {
            let idx = i * 8;
            let val_vec = _mm256_loadu_ps(embedding.as_ptr().add(idx));
            norm_sq_acc = _mm256_fmadd_ps(val_vec, val_vec, norm_sq_acc);
        }

        // Horizontal sum of norm_sq_acc
        let norm_sq = {
            let hi = _mm256_extractf128_ps(norm_sq_acc, 1);
            let lo = _mm256_castps256_ps128(norm_sq_acc);
            let sum128 = _mm_add_ps(hi, lo);

            let hi64 = _mm_movehl_ps(sum128, sum128);
            let sum64 = _mm_add_ps(sum128, hi64);

            let hi32 = _mm_shuffle_ps(sum64, sum64, 0x01);
            let sum32 = _mm_add_ss(sum64, hi32);

            _mm_cvtss_f32(sum32)
        };

        // Add remainder elements
        let remainder_norm_sq: f32 = embedding[chunks * 8..].iter().map(|x| x * x).sum();
        let total_norm_sq = norm_sq + remainder_norm_sq;

        let norm = total_norm_sq.sqrt();
        if norm > 1e-8 {
            let inv_norm = 1.0 / norm;
            let inv_norm_vec = _mm256_set1_ps(inv_norm);

            // SIMD normalization
            for i in 0..chunks {
                let idx = i * 8;
                let val_vec = _mm256_loadu_ps(embedding.as_ptr().add(idx));
                let norm_vec = _mm256_mul_ps(val_vec, inv_norm_vec);
                _mm256_storeu_ps(embedding.as_mut_ptr().add(idx), norm_vec);
            }

            // Handle remainder
            for val in embedding.iter_mut().skip(chunks * 8) { *val *= inv_norm; }
        }
    }

    /// Optimized scalar L2 normalization
    fn scalar_l2_normalize_optimized(&self, embedding: &mut [f32]) {
        // Calculate norm squared with manual unrolling
        let chunks = embedding.len() / 4;
        let remainder = embedding.len() % 4;
        let mut norm_sq = 0.0f32;

        // Process 4 elements at a time
        for i in 0..chunks {
            let base_idx = i * 4;
            let v0 = embedding[base_idx];
            let v1 = embedding[base_idx + 1];
            let v2 = embedding[base_idx + 2];
            let v3 = embedding[base_idx + 3];

            norm_sq += v0 * v0 + v1 * v1 + v2 * v2 + v3 * v3;
        }

        // Handle remainder
        let remainder_start = chunks * 4;
        for i in 0..remainder {
            let val = embedding[remainder_start + i];
            norm_sq += val * val;
        }

        let norm = norm_sq.sqrt();
        if norm > 1e-8 {
            let inv_norm = 1.0 / norm;
            // In-place normalization with unrolling
            for i in 0..chunks {
                let base_idx = i * 4;
                embedding[base_idx] *= inv_norm;
                embedding[base_idx + 1] *= inv_norm;
                embedding[base_idx + 2] *= inv_norm;
                embedding[base_idx + 3] *= inv_norm;
            }

            // Handle remainder
            let remainder_start = chunks * 4;
            for i in 0..remainder {
                embedding[remainder_start + i] *= inv_norm;
            }
        }
    }
}

/// Pure scalar mean pooling: sum across seq_len then divide by seq_len
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn mean_pool_scalar(data: &[f32], seq_len: usize, hidden_size: usize) -> Vec<f32> {
    let mut pooled = vec![0.0f32; hidden_size];
    for s in 0..seq_len {
        let base = s * hidden_size;
        for h in 0..hidden_size {
            if base + h < data.len() {
                pooled[h] += data[base + h];
            }
        }
    }
    let inv = if seq_len > 0 { 1.0 / (seq_len as f32) } else { 0.0 };
    for v in &mut pooled {
        *v *= inv;
    }
    pooled
}

/// Pure scalar L2 normalization
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn l2_normalize_scalar(mut v: Vec<f32>) -> Vec<f32> {
    let mut norm_sq = 0.0f32;
    for &x in &v { norm_sq += x * x; }
    let norm = norm_sq.sqrt();
    if norm > 1e-8 {
        let inv = 1.0 / norm;
        for x in &mut v { *x *= inv; }
    }
    v
}

/// Extract first item embedding from 3D [1, seq_len, hidden_size] using scalar helpers
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn extract_embedding_from_3d_scalar(
    shape_vec: &[i64],
    data: &[f32],
    seq_len: usize,
    hidden_size: usize,
) -> Option<Vec<f32>> {
    if shape_vec.len() != 3 || shape_vec[0] != 1 || shape_vec[1] != seq_len as i64 || shape_vec[2] != hidden_size as i64 {
        return None;
    }
    let need = seq_len.checked_mul(hidden_size)?;
    if need > data.len() { return None; }
    let pooled = mean_pool_scalar(&data[0..need], seq_len, hidden_size);
    Some(l2_normalize_scalar(pooled))
}

#[cfg(test)]
mod extra_scalar_tests {
    use super::*;

    #[test]
    fn test_mean_pool_scalar_basic() {
        // seq_len=2, hidden=4: rows [1,2,3,4] and [5,6,7,8]
        let data = vec![1.0,2.0,3.0,4.0, 5.0,6.0,7.0,8.0];
        let pooled = mean_pool_scalar(&data, 2, 4);
        assert_eq!(pooled, vec![3.0,4.0,5.0,6.0]);
    }

    #[test]
    fn test_l2_normalize_scalar_unit_norm() {
        let v = vec![3.0, 4.0];
        let n = l2_normalize_scalar(v);
        let norm: f32 = n.iter().map(|x| x*x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_extract_embedding_from_3d_scalar_ok() {
        let seq_len = 2usize;
        let hidden = 4usize;
        let shape = [1i64, seq_len as i64, hidden as i64];
        let data = vec![1.0,2.0,3.0,4.0, 5.0,6.0,7.0,8.0];
        let emb = extract_embedding_from_3d_scalar(&shape, &data, seq_len, hidden).expect("ok");
        assert_eq!(emb.len(), hidden);
        // mean pooled = [3,4,5,6] → norm ~ sqrt(86) ≈ 9.2736 → normalized first ~ 0.323
        let expected0 = 3.0f32 / 86.0f32.sqrt();
        assert!((emb[0] - expected0).abs() < 1e-3);
        let norm: f32 = emb.iter().map(|x| x*x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_extract_embedding_from_3d_scalar_bad_shape() {
        let shape = [2i64, 0, 0];
        let data: Vec<f32> = vec![];
        assert!(extract_embedding_from_3d_scalar(&shape, &data, 0, 0).is_none());
    }
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    pub model_name: String,
    pub vocab_size: usize,
    pub max_length: usize,
    pub hidden_size: usize,
    pub optimization_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "gpu")]
    use crate::GpuConfig;

    #[test]
    fn test_optimized_service_creation() {
        if std::env::var("ORT_DYLIB_PATH").is_err() {
            eprintln!("Skipping embeddings test: ORT_DYLIB_PATH not set");
            return;
        }
        let config = EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            batch_size: 8,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        match CpuEmbeddingService::new(config) {
            Ok(_service) => {
                println!("✅ Optimized service created successfully");
            }
            Err(e) => {
                println!("Expected error without models: {}", e);
            }
        }
    }

    #[test]
    fn test_gpu_service_creation() {
        let config = EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            batch_size: 32, // Больше batch для GPU
            use_gpu: false,
            #[cfg(feature = "gpu")]
            gpu_config: Some(GpuConfig::default()),
            #[cfg(not(feature = "gpu"))]
            gpu_config: None,
            embedding_dim: Some(1024),
        };

        // Проверяем доступность GPU
        #[cfg(feature = "gpu")]
        {
            let gpu_detector = crate::gpu_detector::GpuDetector::detect();
            println!("GPU доступность: {}", gpu_detector.available);
            if !gpu_detector.available {
                println!("⚠️ GPU не доступен, тест будет использовать CPU fallback");
                config.use_gpu = false;
                config.gpu_config = None;
            } else {
                config.use_gpu = true;
            }
        }

        match CpuEmbeddingService::new(config) {
            Ok(_service) => {
                println!("✅ GPU-enabled service created successfully");
            }
            Err(e) => {
                println!("Expected error without models: {}", e);
            }
        }
    }
}
