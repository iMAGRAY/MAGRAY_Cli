use crate::EmbeddingConfig;
#[cfg(feature = "gpu")]
use crate::{GpuConfig, GpuInfo};
use crate::tokenization::{OptimizedTokenizer, TokenizedInput as OptTokenizedInput, BatchTokenized};
use crate::memory_pool::{GLOBAL_MEMORY_POOL, PoolStats};
use anyhow::Result as AnyhowResult;
use ort::{session::Session, value::Tensor, inputs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{info, debug};
#[cfg(feature = "gpu")]
use tracing::warn;

/// @component: {"k":"C","id":"embeddings_cpu","t":"CPU-based embeddings","m":{"cur":90,"tgt":95,"u":"%"}}
/// CPU-based Embedding Service with real tokenization and batching (supports BGE-M3 and Qwen3)
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
    /// Create new optimized embedding service (supports BGE-M3 and Qwen3)
    pub fn new(config: EmbeddingConfig) -> AnyhowResult<Self> {
        info!("Initializing CPU embedding service with model: {}", config.model_name);
        
        // Получаем путь относительно корня проекта
        let current_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        
        // Определяем путь к модели в зависимости от типа
        let (model_filename, hidden_size) = match config.model_name.as_str() {
            "qwen3emb" => ("model.onnx", 1024),  // Исправлено: используем стандартное имя файла
            "bge-m3" => ("model.onnx", 1024),
            _ => ("model.onnx", config.embedding_dim.unwrap_or(1024)),
        };
        
        // Ищем модели в разных возможных местах
        let possible_paths = vec![
            // Если запускаемся из корня проекта
            current_dir.join(format!("crates/memory/models/{}/{}", config.model_name, model_filename)),
            // Если модели в корне проекта
            current_dir.join(format!("models/{}/{}", config.model_name, model_filename)),
            // Если запускаемся из crates/memory
            current_dir.join(format!("models/{}/{}", config.model_name, model_filename)),
            // Если запускаемся из другого места
            current_dir.join(format!("../memory/models/{}/{}", config.model_name, model_filename)),
            current_dir.join(format!("../../models/{}/{}", config.model_name, model_filename)),
            // Абсолютный путь из переменной окружения
            PathBuf::from(format!("models/{}/{}", config.model_name, model_filename)),
        ];
        
        let model_path = possible_paths.iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow::anyhow!("Model file not found. Tried paths: {:?}", possible_paths))?
            .clone();
        
        // Аналогично для tokenizer
        let tokenizer_possible_paths = vec![
            current_dir.join(format!("crates/memory/models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!("models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!("models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!("../memory/models/{}/tokenizer.json", config.model_name)),
            current_dir.join(format!("../../models/{}/tokenizer.json", config.model_name)),
            PathBuf::from(format!("models/{}/tokenizer.json", config.model_name)),
        ];
        
        let tokenizer_path = tokenizer_possible_paths.iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow::anyhow!("Tokenizer file not found. Tried paths: {:?}", tokenizer_possible_paths))?
            .clone();
        
        
        // Setup DLL path for Windows
        #[cfg(target_os = "windows")]
        {
            let mut possible_paths = vec![
                std::env::current_dir().unwrap().join("scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("./scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("../scripts/onnxruntime/lib/onnxruntime.dll"),
                PathBuf::from("../../scripts/onnxruntime/lib/onnxruntime.dll"),
            ];
            
            // Also search in target/debug/build for any onnxruntime-sys build
            if let Ok(target_dir) = std::env::current_dir().map(|d| d.join("target/debug/build")) {
                if let Ok(entries) = std::fs::read_dir(&target_dir) {
                    for entry in entries.flatten() {
                        if entry.file_name().to_string_lossy().starts_with("onnxruntime-sys-") {
                            let dll_path = entry.path()
                                .join("out/onnxruntime/onnxruntime-win-x64-1.8.1/lib/onnxruntime.dll");
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
                    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
                    break;
                }
            }
        }
        
        // Initialize ONNX Runtime
        ort::init()
            .with_name("optimized_bge_m3")
            .commit()?;
        
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
                            session_builder = session_builder.with_execution_providers(providers)?;
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
                            session_builder = session_builder.with_execution_providers(providers)?;
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
        debug!("Batch padded in {}ms", start_time.elapsed().as_millis() - tokenization_time);
        
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
        
        info!("✅ OPTIMIZED batch completed in {}ms ({:.1}ms/text)", 
              total_time, total_time as f64 / texts.len() as f64);
        
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
        let input_ids_tensor = Tensor::from_array(([1, seq_len], input_ids_buf.clone()))?;
        let attention_mask_tensor = Tensor::from_array(([1, seq_len], attention_mask_buf.clone()))?;
        let token_type_ids_tensor = Tensor::from_array(([1, seq_len], token_type_buf.clone()))?;
        
        // Run inference
        let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
        
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
        
        // Return buffers to pool
        GLOBAL_MEMORY_POOL.return_input_buffer(input_ids_buf);
        GLOBAL_MEMORY_POOL.return_attention_buffer(attention_mask_buf);
        GLOBAL_MEMORY_POOL.return_token_type_buffer(token_type_buf);
        
        // Extract and process embeddings
        self.extract_and_pool_embedding(&outputs, seq_len)
    }
    
    /// Process entire batch at once with memory pooling optimization
    fn process_batch_optimized(&self, batch: &BatchTokenized) -> AnyhowResult<Vec<Vec<f32>>> {
        let batch_size = batch.input_ids.len();
        let seq_len = batch.max_length;
        let total_elements = batch_size * seq_len;
        
        debug!("Processing batch: {} x {} tokens ({} total elements)", batch_size, seq_len, total_elements);
        
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
        let input_ids_tensor = Tensor::from_array(([batch_size, seq_len], flat_input_ids.clone()))?;
        let attention_mask_tensor = Tensor::from_array(([batch_size, seq_len], flat_attention_masks.clone()))?;
        let token_type_ids_tensor = Tensor::from_array(([batch_size, seq_len], flat_token_type_ids.clone()))?;
        
        // Single ONNX call for entire batch
        let mut session = self.session.lock().map_err(|e| anyhow::anyhow!("Session lock error: {}", e))?;
        
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
        
        // Return large buffers to pool
        GLOBAL_MEMORY_POOL.return_input_buffer(flat_input_ids);
        GLOBAL_MEMORY_POOL.return_attention_buffer(flat_attention_masks);
        GLOBAL_MEMORY_POOL.return_token_type_buffer(flat_token_type_ids);
        
        // Extract batch embeddings
        self.extract_batch_embeddings(&outputs, batch)
    }
    
    /// Extract embeddings from single output
    fn extract_and_pool_embedding(&self, outputs: &ort::session::SessionOutputs, seq_len: usize) -> AnyhowResult<Vec<f32>> {
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
        
        Err(anyhow::anyhow!("Could not extract embeddings from model outputs"))
    }
    
    /// Extract embeddings from batch output
    fn extract_batch_embeddings(&self, outputs: &ort::session::SessionOutputs, batch: &BatchTokenized) -> AnyhowResult<Vec<Vec<f32>>> {
        let batch_size = batch.input_ids.len();
        
        for (_name, output) in outputs.iter() {
            if let Ok((shape, data)) = output.try_extract_tensor::<f32>() {
                let shape_vec: Vec<i64> = (0..shape.len()).map(|i| shape[i]).collect();
                
                // Look for batch hidden states [batch_size, seq_len, hidden_size]
                if shape_vec.len() == 3 && shape_vec[0] == batch_size as i64 {
                    let seq_len = shape_vec[1] as usize;
                    let hidden_size = shape_vec[2] as usize;
                    
                    debug!("Processing batch output: [{}, {}, {}]", batch_size, seq_len, hidden_size);
                    
                    let mut batch_embeddings = Vec::with_capacity(batch_size);
                    
                    // Process each item in batch
                    for batch_idx in 0..batch_size {
                        let start_offset = batch_idx * seq_len * hidden_size;
                        let end_offset = start_offset + batch.lengths[batch_idx] * hidden_size;
                        
                        if end_offset <= data.len() {
                            let item_data = &data[start_offset..end_offset];
                            let actual_seq_len = batch.lengths[batch_idx];
                            
                            // Optimized pooling for this item
                            let pooled = self.optimized_mean_pooling(item_data, actual_seq_len, hidden_size);
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
        
        Err(anyhow::anyhow!("Could not extract batch embeddings from model outputs"))
    }
    
    /// Optimized mean pooling with memory pooling
    fn optimized_mean_pooling(&self, data: &[f32], seq_len: usize, hidden_size: usize) -> Vec<f32> {
        // Use memory pool for output buffer
        let mut pooled = GLOBAL_MEMORY_POOL.get_output_buffer(hidden_size);
        pooled.resize(hidden_size, 0.0f32);
        
        // Vectorized pooling
        for seq_idx in 0..seq_len {
            let seq_start = seq_idx * hidden_size;
            for hidden_idx in 0..hidden_size {
                let data_idx = seq_start + hidden_idx;
                if data_idx < data.len() {
                    pooled[hidden_idx] += data[data_idx];
                }
            }
        }
        
        // Average (in-place to avoid allocation)
        let seq_len_f32 = seq_len as f32;
        for val in &mut pooled {
            *val /= seq_len_f32;
        }
        
        // Clone result and return buffer to pool
        let result = pooled.clone();
        GLOBAL_MEMORY_POOL.return_output_buffer(pooled);
        result
    }
    
    /// Optimized L2 normalization
    fn optimized_normalize(&self, mut embedding: Vec<f32>) -> Vec<f32> {
        // Calculate norm squared
        let norm_sq: f32 = embedding.iter().map(|x| x * x).sum();
        let norm = norm_sq.sqrt();
        
        if norm > 1e-8 {
            let inv_norm = 1.0 / norm;
            // In-place normalization
            for val in &mut embedding {
                *val *= inv_norm;
            }
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
    pub fn get_pool_stats(&self) -> PoolStats {
        GLOBAL_MEMORY_POOL.get_stats()
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
    use crate::GpuConfig;
    
    #[test]
    fn test_optimized_service_creation() {
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
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
            }
        }
    }
    
    #[test]
    fn test_gpu_service_creation() {
        let mut config = EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            batch_size: 32, // Больше batch для GPU
            use_gpu: true,
            gpu_config: Some(GpuConfig::default()),
            embedding_dim: Some(1024),
        };
        
        // Проверяем доступность GPU
        let gpu_detector = crate::gpu_detector::GpuDetector::detect();
        println!("GPU доступность: {}", gpu_detector.available);
        
        if !gpu_detector.available {
            println!("⚠️ GPU не доступен, тест будет использовать CPU fallback");
            config.use_gpu = false;
            config.gpu_config = None;
        }
        
        match CpuEmbeddingService::new(config) {
            Ok(_service) => {
                println!("✅ GPU-enabled service created successfully");
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
            }
        }
    }
}