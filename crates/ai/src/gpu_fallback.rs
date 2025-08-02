use anyhow::{Result, Context};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn, error, debug};
use async_trait::async_trait;

use crate::{EmbeddingConfig, embeddings_cpu::CpuEmbeddingService, embeddings_gpu::GpuEmbeddingService};
use crate::auto_device_selector::EmbeddingServiceTrait;

/// @component: {"k":"C","id":"gpu_fallback_manager","t":"Reliable GPU fallback system","m":{"cur":100,"tgt":100,"u":"%"},"f":["fallback","resilience","gpu"]}
pub struct GpuFallbackManager {
    /// Основной GPU сервис (если доступен)
    gpu_service: Option<Arc<GpuEmbeddingService>>,
    /// Резервный CPU сервис
    cpu_service: Arc<CpuEmbeddingService>,
    /// Статистика fallback'ов
    fallback_stats: Arc<Mutex<FallbackStats>>,
    /// Конфигурация fallback политики
    policy: FallbackPolicy,
    /// Блокировка GPU после серии ошибок
    gpu_circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

/// Статистика использования fallback
#[derive(Debug, Default, Clone)]
pub struct FallbackStats {
    /// Успешные GPU вызовы
    gpu_success_count: u64,
    /// Ошибки GPU
    gpu_error_count: u64,
    /// Таймауты GPU
    gpu_timeout_count: u64,
    /// Fallback на CPU
    cpu_fallback_count: u64,
    /// Общее время GPU (ms)
    #[allow(dead_code)]
    gpu_total_time_ms: u64,
    /// Общее время CPU (ms)
    cpu_total_time_ms: u64,
}

impl FallbackStats {
    pub fn gpu_success_rate(&self) -> f32 {
        let total = self.gpu_success_count + self.gpu_error_count + self.gpu_timeout_count;
        if total == 0 {
            0.0
        } else {
            self.gpu_success_count as f32 / total as f32
        }
    }
    
    pub fn fallback_rate(&self) -> f32 {
        let total = self.gpu_success_count + self.cpu_fallback_count;
        if total == 0 {
            0.0
        } else {
            self.cpu_fallback_count as f32 / total as f32
        }
    }
}

/// Политика fallback
#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    /// Максимальное время ожидания GPU операции
    pub gpu_timeout: Duration,
    /// Количество ошибок перед отключением GPU
    pub error_threshold: u32,
    /// Время восстановления после отключения
    pub recovery_time: Duration,
    /// Автоматический retry на GPU
    pub auto_retry: bool,
    /// Максимальное количество retry
    pub max_retries: u32,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            gpu_timeout: Duration::from_secs(30),
            error_threshold: 3,
            recovery_time: Duration::from_secs(300), // 5 минут
            auto_retry: true,
            max_retries: 2,
        }
    }
}

/// Circuit breaker для защиты от серии ошибок
#[derive(Debug)]
struct CircuitBreaker {
    /// Состояние: Open (заблокирован), Closed (работает), HalfOpen (пробуем)
    state: CircuitState,
    /// Счётчик последовательных ошибок
    consecutive_errors: u32,
    /// Время последней ошибки
    last_error_time: Option<Instant>,
    /// Конфигурация
    policy: FallbackPolicy,
}

#[derive(Debug, PartialEq)]
enum CircuitState {
    Closed,    // GPU работает
    Open,      // GPU заблокирован
    HalfOpen,  // Пробуем восстановить
}

impl CircuitBreaker {
    fn new(policy: FallbackPolicy) -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_errors: 0,
            last_error_time: None,
            policy,
        }
    }
    
    fn record_success(&mut self) {
        self.consecutive_errors = 0;
        self.state = CircuitState::Closed;
    }
    
    fn record_error(&mut self) {
        self.consecutive_errors += 1;
        self.last_error_time = Some(Instant::now());
        
        if self.consecutive_errors >= self.policy.error_threshold {
            self.state = CircuitState::Open;
            warn!("🔴 Circuit breaker открыт после {} ошибок подряд", self.consecutive_errors);
        }
    }
    
    fn is_gpu_available(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Проверяем, прошло ли время восстановления
                if let Some(last_error) = self.last_error_time {
                    if last_error.elapsed() >= self.policy.recovery_time {
                        self.state = CircuitState::HalfOpen;
                        info!("🟡 Circuit breaker в режиме HalfOpen, пробуем GPU");
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }
}

impl GpuFallbackManager {
    /// Создать новый менеджер с автоматическим fallback
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!("🛡️ Инициализация GpuFallbackManager с надёжным fallback");
        
        let policy = FallbackPolicy::default();
        
        // Всегда создаём CPU сервис как резервный
        let mut cpu_config = config.clone();
        cpu_config.use_gpu = false;
        cpu_config.batch_size = num_cpus::get().min(32);
        
        let cpu_service = Arc::new(
            CpuEmbeddingService::new(cpu_config)
                .context("Failed to create CPU embedding service")?
        );
        info!("✅ CPU сервис создан как резервный");
        
        // Пытаемся создать GPU сервис если требуется
        let gpu_service = if config.use_gpu {
            match Self::try_create_gpu_service(&config).await {
                Ok(service) => {
                    info!("✅ GPU сервис успешно создан");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    warn!("⚠️ Не удалось создать GPU сервис: {}. Будет использоваться только CPU.", e);
                    None
                }
            }
        } else {
            info!("ℹ️ GPU отключен в конфигурации");
            None
        };
        
        Ok(Self {
            gpu_service,
            cpu_service,
            fallback_stats: Arc::new(Mutex::new(FallbackStats::default())),
            policy: policy.clone(),
            gpu_circuit_breaker: Arc::new(Mutex::new(CircuitBreaker::new(policy))),
        })
    }
    
    /// Попытка создать GPU сервис с тестированием
    async fn try_create_gpu_service(config: &EmbeddingConfig) -> Result<GpuEmbeddingService> {
        let service = GpuEmbeddingService::new(config.clone()).await?;
        
        // Тестируем работоспособность
        let test_text = vec!["Test GPU embedding service".to_string()];
        let start = Instant::now();
        
        match tokio::time::timeout(Duration::from_secs(10), service.embed_batch(test_text)).await {
            Ok(Ok(embeddings)) => {
                let elapsed = start.elapsed();
                info!("✅ GPU тест пройден за {:?}, размер embedding: {}", 
                      elapsed, embeddings.first().map(|e| e.len()).unwrap_or(0));
                Ok(service)
            }
            Ok(Err(e)) => {
                error!("❌ GPU тест провален: {}", e);
                Err(e)
            }
            Err(_) => {
                error!("❌ GPU тест timeout");
                Err(anyhow::anyhow!("GPU test timeout"))
            }
        }
    }
    
    /// Получить embeddings с автоматическим fallback
    pub async fn embed_batch_with_fallback(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let batch_size = texts.len();
        debug!("🔄 Обработка batch из {} текстов", batch_size);
        
        // Проверяем доступность GPU через circuit breaker
        let use_gpu = self.gpu_service.is_some() && 
                      self.gpu_circuit_breaker.lock().unwrap().is_gpu_available();
        
        if use_gpu {
            // Пытаемся использовать GPU
            match self.try_gpu_embed(&texts).await {
                Ok(embeddings) => {
                    self.record_gpu_success();
                    return Ok(embeddings);
                }
                Err(e) => {
                    warn!("⚠️ GPU embedding failed: {}. Falling back to CPU.", e);
                    self.record_gpu_error();
                    // Продолжаем с CPU fallback
                }
            }
        }
        
        // Используем CPU
        self.embed_with_cpu(&texts).await
    }
    
    /// Попытка получить embeddings через GPU с timeout
    async fn try_gpu_embed(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        let gpu_service = self.gpu_service.as_ref()
            .ok_or_else(|| anyhow::anyhow!("GPU service not available"))?;
        
        let start = Instant::now();
        
        // Применяем timeout
        match tokio::time::timeout(
            self.policy.gpu_timeout, 
            gpu_service.embed_batch(texts.clone())
        ).await {
            Ok(Ok(embeddings)) => {
                let elapsed = start.elapsed();
                debug!("✅ GPU embedding успешно за {:?}", elapsed);
                Ok(embeddings)
            }
            Ok(Err(e)) => {
                error!("❌ GPU embedding error: {}", e);
                Err(e)
            }
            Err(_) => {
                error!("❌ GPU embedding timeout после {:?}", self.policy.gpu_timeout);
                self.fallback_stats.lock().unwrap().gpu_timeout_count += 1;
                Err(anyhow::anyhow!("GPU embedding timeout"))
            }
        }
    }
    
    /// Получить embeddings через CPU
    async fn embed_with_cpu(&self, texts: &Vec<String>) -> Result<Vec<Vec<f32>>> {
        let start = Instant::now();
        self.fallback_stats.lock().unwrap().cpu_fallback_count += 1;
        
        let results = self.cpu_service.embed_batch(&texts[..])?;
        
        // Конвертируем OptimizedEmbeddingResult в Vec<Vec<f32>>
        let embeddings: Vec<Vec<f32>> = results
            .into_iter()
            .map(|r| r.embedding)
            .collect();
        
        let elapsed = start.elapsed();
        self.fallback_stats.lock().unwrap().cpu_total_time_ms += elapsed.as_millis() as u64;
        
        debug!("✅ CPU embedding успешно за {:?}", elapsed);
        Ok(embeddings)
    }
    
    /// Записать успешный GPU вызов
    fn record_gpu_success(&self) {
        let mut stats = self.fallback_stats.lock().unwrap();
        stats.gpu_success_count += 1;
        
        let mut breaker = self.gpu_circuit_breaker.lock().unwrap();
        breaker.record_success();
    }
    
    /// Записать ошибку GPU
    fn record_gpu_error(&self) {
        let mut stats = self.fallback_stats.lock().unwrap();
        stats.gpu_error_count += 1;
        
        let mut breaker = self.gpu_circuit_breaker.lock().unwrap();
        breaker.record_error();
    }
    
    /// Получить статистику
    pub fn get_stats(&self) -> FallbackStats {
        self.fallback_stats.lock().unwrap().clone()
    }
    
    /// Принудительно переключиться на CPU
    pub fn force_cpu_mode(&self) {
        let mut breaker = self.gpu_circuit_breaker.lock().unwrap();
        breaker.state = CircuitState::Open;
        breaker.last_error_time = Some(Instant::now());
        info!("🔴 Принудительное переключение на CPU режим");
    }
    
    /// Сбросить circuit breaker и попробовать GPU снова
    pub fn reset_circuit_breaker(&self) {
        let mut breaker = self.gpu_circuit_breaker.lock().unwrap();
        breaker.state = CircuitState::Closed;
        breaker.consecutive_errors = 0;
        breaker.last_error_time = None;
        info!("🟢 Circuit breaker сброшен, GPU снова доступен");
    }
}

#[async_trait]
impl EmbeddingServiceTrait for GpuFallbackManager {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_batch_with_fallback(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_breaker() {
        let policy = FallbackPolicy {
            error_threshold: 3,
            recovery_time: Duration::from_secs(5),
            ..Default::default()
        };
        
        let mut breaker = CircuitBreaker::new(policy);
        
        // Initially closed
        assert_eq!(breaker.state, CircuitState::Closed);
        assert!(breaker.is_gpu_available());
        
        // Record errors
        breaker.record_error();
        breaker.record_error();
        assert!(breaker.is_gpu_available()); // Still available
        
        // Third error opens the circuit
        breaker.record_error();
        assert_eq!(breaker.state, CircuitState::Open);
        assert!(!breaker.is_gpu_available());
        
        // Success resets
        breaker.record_success();
        assert_eq!(breaker.state, CircuitState::Closed);
        assert!(breaker.is_gpu_available());
    }
    
    #[test]
    fn test_fallback_stats() {
        let stats = FallbackStats {
            gpu_success_count: 80,
            gpu_error_count: 15,
            gpu_timeout_count: 5,
            cpu_fallback_count: 20,
            ..Default::default()
        };
        
        assert_eq!(stats.gpu_success_rate(), 0.8); // 80 / (80+15+5)
        assert_eq!(stats.fallback_rate(), 0.2);    // 20 / (80+20)
    }
}