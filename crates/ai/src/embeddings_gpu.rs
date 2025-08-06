use crate::{EmbeddingConfig, GpuConfig};
use crate::gpu_detector::GpuDetector;
use crate::gpu_memory_pool::GPU_MEMORY_POOL;
use crate::model_downloader::ensure_model;
use anyhow::Result;
use ort::{session::Session, inputs, value::Tensor};
use ndarray::Array2;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tracing::{info, debug};
use crate::tokenization::OptimizedTokenizer;
#[cfg(feature = "gpu")]
use tracing::warn;

pub struct GpuEmbeddingService {
    session: Arc<Mutex<Session>>,
    tokenizer: Arc<OptimizedTokenizer>,
    #[allow(dead_code)]
    model_path: PathBuf,
    #[allow(dead_code)]
    hidden_size: usize,
    max_length: usize,
    optimal_batch_size: usize,
    use_gpu: bool,
    metrics: Arc<Mutex<PerformanceMetrics>>,
}

#[derive(Debug, Default, Clone)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_time_ms: u64,
    pub gpu_time_ms: u64,
    pub cpu_time_ms: u64,
    pub avg_batch_size: f32,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl PerformanceMetrics {
    pub fn tokens_per_second(&self) -> f32 {
        if self.total_time_ms == 0 {
            0.0
        } else {
            (self.total_tokens as f32 / self.total_time_ms as f32) * 1000.0
        }
    }
    
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
}

impl GpuEmbeddingService {
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!("🚀 Инициализация GpuEmbeddingService");
        
        // Определяем оптимальные параметры
        let detector = GpuDetector::detect();
        let model_size_mb = match config.model_name.as_str() {
            "qwen3emb" => 600, // Примерный размер Qwen3 модели
            "bge-m3" => 500, // Примерный размер BGE-M3 модели
            _ => 500,
        };
        
        let (optimal_batch_size, use_gpu) = if config.use_gpu && detector.available {
            let gpu_config = if let Some(ref gc) = config.gpu_config {
                gc.clone()
            } else {
                GpuConfig::auto_optimized()
            };
            
            let optimal_params = gpu_config.get_optimal_params(model_size_mb);
            
            // Увеличиваем batch size для GPU - минимум 64
            let gpu_batch_size = optimal_params.batch_size.max(64);
            
            info!("🎯 Оптимальные параметры GPU:");
            info!("  - Batch size: {} (рекомендовано: {})", gpu_batch_size, optimal_params.batch_size);
            info!("  - Max sequence: {}", optimal_params.max_sequence_length);
            info!("  - FP16: {}", optimal_params.use_fp16);
            info!("  - GPU memory: {}MB available", detector.devices.first().map(|d| d.total_memory_mb).unwrap_or(0));
            
            (gpu_batch_size, true)
        } else {
            // CPU параметры
            let cpu_batch_size = num_cpus::get().min(32);
            info!("💻 Оптимальные параметры CPU:");
            info!("  - Batch size: {}", cpu_batch_size);
            (cpu_batch_size, false)
        };
        
        // Автоматически загружаем модель если необходимо
        let model_name = &config.model_name;
        let model_dir = ensure_model(model_name).await?;
        
        // Определяем имя файла модели в зависимости от типа
        let model_filename = match model_name.as_str() {
            "qwen3emb" => "model.onnx",  // Используем стандартное имя
            "bge-m3" => "model.onnx",
            _ => "model.onnx",
        };
        
        let model_path = model_dir.join(model_filename);
        let tokenizer_path = model_dir.join("tokenizer.json");
        
        if !model_path.exists() {
            return Err(anyhow::anyhow!("Model file not found after download: {:?}", model_path));
        }
        
        if !tokenizer_path.exists() {
            return Err(anyhow::anyhow!("Tokenizer file not found: {:?}", tokenizer_path));
        }
        
        // Load tokenizer
        info!("Loading tokenizer from: {:?}", tokenizer_path);
        let tokenizer = OptimizedTokenizer::new(&tokenizer_path, config.max_length)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        
        // Create optimized session
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
        
        // Добавляем GPU провайдеры если нужно
        #[cfg(feature = "gpu")]
        if use_gpu {
            let gpu_config = config.gpu_config.unwrap_or_else(GpuConfig::auto_optimized);
            match gpu_config.create_providers() {
                Ok(providers) => {
                    if !providers.is_empty() {
                        info!("🚀 Добавляем {} GPU провайдеров", providers.len());
                        session_builder = session_builder.with_execution_providers(providers)?;
                    }
                }
                Err(e) => {
                    warn!("⚠️ Ошибка создания GPU провайдеров: {}. Используем CPU.", e);
                }
            }
        }
        
        let session = session_builder.commit_from_file(&model_path)?;
        
        // Validate model
        let outputs = session.outputs.len();
        let inputs = session.inputs.len();
        info!("✅ Model loaded: {} inputs, {} outputs", inputs, outputs);
        
        if inputs != 3 {
            tracing::warn!("Expected 3 inputs for model {} (input_ids, attention_mask, token_type_ids), got {}", model_name, inputs);
        }
        
        let hidden_size = config.embedding_dim.unwrap_or(1024);
        let max_length = config.max_length;
        
        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            tokenizer: Arc::new(tokenizer),
            model_path,
            hidden_size,
            max_length,
            optimal_batch_size,
            use_gpu,
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
        })
    }
    
    /// Динамически оптимизировать размер батча на основе метрик
    pub fn optimize_batch_size(&mut self) {
        let metrics = self.metrics.lock().unwrap();
        let current_tps = metrics.tokens_per_second();
        drop(metrics);
        
        // Простая эвристика: увеличиваем batch size если производительность растёт
        if current_tps > 0.0 {
            let test_sizes = vec![
                self.optimal_batch_size / 2,
                self.optimal_batch_size,
                self.optimal_batch_size * 2,
            ];
            
            let mut best_size = self.optimal_batch_size;
            let _best_tps = current_tps;
            
            for size in test_sizes {
                if size > 0 && size <= 512 {
                    // Здесь можно провести тест с новым размером
                    // Пока используем простую эвристику
                    if size > self.optimal_batch_size && self.use_gpu {
                        best_size = size;
                    }
                }
            }
            
            if best_size != self.optimal_batch_size {
                info!("📊 Оптимизация batch size: {} -> {}", self.optimal_batch_size, best_size);
                self.optimal_batch_size = best_size;
            }
        }
    }
    
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let start_time = Instant::now();
        let num_texts = texts.len();
        
        // Разбиваем на оптимальные батчи
        let mut all_embeddings = Vec::with_capacity(num_texts);
        
        // Динамический batch size основан на количестве текстов
        let effective_batch_size = if self.use_gpu {
            // Для GPU: адаптивный размер батча
            if num_texts <= 16 {
                num_texts // Маленький батч = без группировки
            } else if num_texts <= 64 {
                32 // Средний батч
            } else {
                64.min(self.optimal_batch_size) // Большой батч, но разумный лимит
            }
        } else {
            self.optimal_batch_size.min(32) // Меньше для CPU
        };
        
        debug!("🎯 Обработка {} текстов батчами по {}", num_texts, effective_batch_size);
        
        for chunk in texts.chunks(effective_batch_size) {
            let batch_embeddings = self.process_batch(chunk.to_vec()).await?;
            all_embeddings.extend(batch_embeddings);
        }
        
        // Обновляем метрики
        let elapsed = start_time.elapsed().as_millis() as u64;
        // Подсчитываем реальное количество токенов
        let total_tokens: usize = texts.iter()
            .map(|t| {
                self.tokenizer.encode(t.as_str())
                    .map(|tokenized| tokenized.length)
                    .unwrap_or(0)
            })
            .sum();
        
        let mut metrics = self.metrics.lock().unwrap();
        metrics.total_requests += num_texts as u64;
        metrics.total_tokens += total_tokens as u64;
        metrics.total_time_ms += elapsed;
        if self.use_gpu {
            metrics.gpu_time_ms += elapsed;
        } else {
            metrics.cpu_time_ms += elapsed;
        }
        metrics.avg_batch_size = (metrics.avg_batch_size * (metrics.total_requests - num_texts as u64) as f32 
            + self.optimal_batch_size as f32 * num_texts as f32) / metrics.total_requests as f32;
        
        info!("⚡ Обработано {} текстов за {}ms ({:.1} tokens/sec)", 
            num_texts, elapsed, (total_tokens as f32 / elapsed as f32) * 1000.0);
        
        Ok(all_embeddings)
    }
    
    async fn process_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let batch_size = texts.len();
        debug!("🏊 Processing batch of {} texts with memory pooling", batch_size);
        
        // Оценка размера буферов для memory pool
        let tensor_size = batch_size * self.max_length * std::mem::size_of::<i64>();
        
        // Используем memory pool для эффективного управления памятью
        let result = GPU_MEMORY_POOL.with_buffer_async(tensor_size * 3, |buffer| async move {
            // Используем batch tokenization для эффективности
            let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            let mut batch_tokenized = self.tokenizer.encode_batch(&text_refs)
                .map_err(|e| anyhow::anyhow!("Failed to tokenize batch: {}", e))?;
            
            // Паддинг до единой длины
            self.tokenizer.pad_batch(&mut batch_tokenized, Some(self.max_length))
                .map_err(|e| anyhow::anyhow!("Failed to pad batch: {}", e))?;
            
            // Подготовка данных для ONNX с использованием буфера из пула
            let mut all_input_ids: Vec<i64> = Vec::with_capacity(batch_size * self.max_length);
            let mut all_attention_mask: Vec<i64> = Vec::with_capacity(batch_size * self.max_length);
            let mut all_token_type_ids: Vec<i64> = Vec::with_capacity(batch_size * self.max_length);
            
            for i in 0..batch_size {
                all_input_ids.extend(&batch_tokenized.input_ids[i]);
                all_attention_mask.extend(&batch_tokenized.attention_masks[i]);
                all_token_type_ids.extend(&batch_tokenized.token_type_ids[i]);
            }
            
            // Create ndarray tensors
            let input_ids_array = Array2::from_shape_vec((batch_size, self.max_length), all_input_ids)?;
            let attention_mask_array = Array2::from_shape_vec((batch_size, self.max_length), all_attention_mask)?;
            let token_type_ids_array = Array2::from_shape_vec((batch_size, self.max_length), all_token_type_ids)?;
            
            // Run inference
            let mut session = self.session.lock().unwrap();
            
            // Convert arrays to tensors using ort 2.0 API
            let input_ids_shape = input_ids_array.shape().to_vec();
            let input_ids_data = input_ids_array.into_raw_vec();
            let input_ids_tensor = Tensor::from_array((input_ids_shape, input_ids_data))?;
            
            let attention_mask_shape = attention_mask_array.shape().to_vec();
            let attention_mask_data = attention_mask_array.into_raw_vec();
            let attention_mask_tensor = Tensor::from_array((attention_mask_shape, attention_mask_data))?;
            
            let token_type_ids_shape = token_type_ids_array.shape().to_vec();
            let token_type_ids_data = token_type_ids_array.into_raw_vec();
            let token_type_ids_tensor = Tensor::from_array((token_type_ids_shape, token_type_ids_data))?;
            
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
            
            // Extract embeddings from output
            let output = outputs.iter().next()
                .ok_or_else(|| anyhow::anyhow!("No output from model"))?
                .1; // Get the value from (name, value) tuple
            
            // Extract raw tensor (shape, data)
            let (shape, data) = output.try_extract_tensor::<f32>()?;
            
            // Determine dimensions from shape
            let result_batch_size = shape[0] as usize;
            let hidden_size = if shape.len() == 3 {
                // If output is (batch, seq_len, hidden_size), we need to pool
                shape[2] as usize
            } else {
                // If output is (batch, hidden_size), use directly
                shape[1] as usize
            };
            
            let mut result = Vec::with_capacity(result_batch_size);
            
            // Handle different output shapes
            if shape.len() == 3 {
                // Output is (batch, seq_len, hidden_size) - need mean pooling
                let seq_len = shape[1] as usize;
                for i in 0..result_batch_size {
                    let mut embedding = vec![0.0f32; hidden_size];
                    // Mean pooling over sequence length
                    for j in 0..seq_len {
                        let offset = (i * seq_len + j) * hidden_size;
                        for k in 0..hidden_size {
                            embedding[k] += data[offset + k];
                        }
                    }
                    // Average
                    embedding.iter_mut().for_each(|x| *x /= seq_len as f32);
                    
                    // L2 normalize
                    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm > 0.0 {
                        embedding.iter_mut().for_each(|x| *x /= norm);
                    }
                    
                    result.push(embedding);
                }
            } else {
                // Output is (batch, hidden_size) - use directly
                for i in 0..result_batch_size {
                    let start = i * hidden_size;
                    let end = start + hidden_size;
                    let mut embedding = data[start..end].to_vec();
                    
                    // L2 normalize
                    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm > 0.0 {
                        embedding.iter_mut().for_each(|x| *x /= norm);
                    }
                    
                    result.push(embedding);
                }
            }
            
            Ok((result, buffer))
        }).await?;
        
        debug!("💾 Memory pool статистика после обработки:");
        let _ = GPU_MEMORY_POOL.print_stats();
        
        Ok(result)
    }
    
    pub fn get_metrics(&self) -> PerformanceMetrics {
        let metrics = self.metrics.lock().unwrap();
        metrics.clone()
    }
    
    pub fn print_metrics(&self) {
        let metrics = self.get_metrics();
        info!("📊 Метрики производительности:");
        info!("  - Всего запросов: {}", metrics.total_requests);
        info!("  - Всего токенов: {}", metrics.total_tokens);
        info!("  - Tokens/sec: {:.1}", metrics.tokens_per_second());
        info!("  - Средний batch size: {:.1}", metrics.avg_batch_size);
        info!("  - Cache hit rate: {:.1}%", metrics.cache_hit_rate() * 100.0);
        if self.use_gpu {
            info!("  - GPU время: {}ms", metrics.gpu_time_ms);
        } else {
            info!("  - CPU время: {}ms", metrics.cpu_time_ms);
        }
    }
}