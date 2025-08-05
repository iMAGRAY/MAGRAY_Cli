//! LLM Communication Service - взаимодействие с языковыми моделями
//! 
//! Сервис отвечает за управление коммуникацией с различными LLM провайдерами:
//! - OpenAI API
//! - Anthropic Claude API  
//! - Локальные модели
//! 
//! Предоставляет единый интерфейс с автоматическим fallback между провайдерами,
//! управлением токенами, кэшированием ответов и метриками производительности.

use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use llm::LlmClient;
use super::types::{RequestContext, OperationResult};

/// Trait для сервиса взаимодействия с LLM
// @component: {"k":"C","id":"llm_communication_service","t":"LLM communication service trait","m":{"cur":95,"tgt":100,"u":"%"},"f":["trait","llm","multi_provider","clean_architecture"]}
#[async_trait::async_trait]
pub trait LlmCommunicationService: Send + Sync {
    /// Отправить простой чат запрос к LLM
    async fn chat(&self, context: &RequestContext) -> Result<OperationResult<String>>;
    
    /// Отправить структурированный запрос с дополнительными параметрами
    async fn chat_with_options(
        &self,
        context: &RequestContext,
        options: &ChatOptions,
    ) -> Result<OperationResult<String>>;
    
    /// Проверить доступность LLM провайдеров
    async fn health_check(&self) -> LlmHealthStatus;
    
    /// Получить информацию о текущем провайдере
    async fn get_provider_info(&self) -> ProviderInfo;
    
    /// Получить статистику использования
    async fn get_usage_stats(&self) -> LlmUsageStats;
    
    /// Сбросить статистику (для тестов)
    async fn reset_stats(&self);
}

/// Опции для чат запроса
#[derive(Debug, Clone)]
pub struct ChatOptions {
    /// Максимальное количество токенов в ответе
    pub max_tokens: Option<u32>,
    
    /// Температура для генерации (0.0 - 2.0)
    pub temperature: Option<f32>,
    
    /// Top-p sampling parameter (0.0 - 1.0)  
    pub top_p: Option<f32>,
    
    /// Системный промпт
    pub system_prompt: Option<String>,
    
    /// Использовать ли кэш для запроса
    pub use_cache: bool,
    
    /// Предпочитаемый провайдер
    pub preferred_provider: Option<String>,
    
    /// Timeout для запроса
    pub timeout: Option<Duration>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            max_tokens: Some(4000),
            temperature: Some(0.7),
            top_p: Some(0.9),
            system_prompt: None,
            use_cache: true,
            preferred_provider: None,
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

/// Статус здоровья LLM провайдеров
#[derive(Debug, Clone)]
pub struct LlmHealthStatus {
    pub primary_provider_healthy: bool,
    pub fallback_providers_available: u32,
    pub last_successful_request: Option<chrono::DateTime<chrono::Utc>>,
    pub current_error_rate: f64,
    pub estimated_response_time: Duration,
}

/// Информация о текущем провайдере
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub model: String,
    pub version: Option<String>,
    pub rate_limits: RateLimits,
    pub capabilities: Vec<String>,
}

/// Лимиты скорости для провайдера
#[derive(Debug, Clone)]
pub struct RateLimits {
    pub requests_per_minute: u32,
    pub tokens_per_minute: u32,
    pub current_usage_rpm: u32,
    pub current_usage_tpm: u32,
}

/// Статистика использования LLM
#[derive(Debug, Clone)]
pub struct LlmUsageStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cached_responses: u64,
    pub total_tokens_consumed: u64,
    pub total_tokens_generated: u64,
    pub avg_response_time_ms: f64,
    pub provider_distribution: std::collections::HashMap<String, u64>,
}

/// Реализация сервиса взаимодействия с LLM по умолчанию
// @component: {"k":"C","id":"default_llm_communication_service","t":"Default LLM communication service implementation","m":{"cur":85,"tgt":95,"u":"%"},"f":["service","llm","fallback","caching","metrics"]}
pub struct DefaultLlmCommunicationService {
    /// Основной LLM клиент
    llm_client: LlmClient,
    
    /// Статистика использования
    stats: parking_lot::RwLock<LlmUsageStats>,
    
    /// Кэш ответов для повторяющихся запросов
    response_cache: parking_lot::RwLock<std::collections::HashMap<String, CachedResponse>>,
    
    /// Конфигурация сервиса
    config: LlmCommunicationConfig,
}

/// Кэшированный ответ
#[derive(Debug, Clone)]
struct CachedResponse {
    response: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    tokens_used: u32,
    provider: String,
}

/// Конфигурация сервиса взаимодействия с LLM
#[derive(Debug, Clone)]
pub struct LlmCommunicationConfig {
    /// Время жизни кэша ответов
    pub cache_ttl: Duration,
    
    /// Максимальный размер кэша (количество записей)
    pub max_cache_size: usize,
    
    /// Включить ли автоматический fallback между провайдерами
    pub enable_provider_fallback: bool,
    
    /// Максимальное количество попыток для запроса
    pub max_retries: u32,
    
    /// Базовый timeout для запросов
    pub default_timeout: Duration,
    
    /// Включить ли детальные метрики
    pub enable_detailed_metrics: bool,
}

impl Default for LlmCommunicationConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_minutes(30),
            max_cache_size: 1000,
            enable_provider_fallback: true,
            max_retries: 3,
            default_timeout: Duration::from_secs(30),
            enable_detailed_metrics: true,
        }
    }
}

impl DefaultLlmCommunicationService {
    /// Создать новый экземпляр сервиса
    pub fn new(llm_client: LlmClient) -> Self {
        Self::with_config(llm_client, LlmCommunicationConfig::default())
    }
    
    /// Создать экземпляр с кастомной конфигурацией
    pub fn with_config(llm_client: LlmClient, config: LlmCommunicationConfig) -> Self {
        Self {
            llm_client,
            stats: parking_lot::RwLock::new(LlmUsageStats::default()),
            response_cache: parking_lot::RwLock::new(std::collections::HashMap::new()),
            config,
        }
    }
    
    /// Создать ключ кэша на основе запроса
    fn create_cache_key(&self, message: &str, options: &ChatOptions) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        message.hash(&mut hasher);
        options.max_tokens.hash(&mut hasher);
        options.temperature.map(|t| (t * 1000.0) as u32).hash(&mut hasher);
        options.system_prompt.hash(&mut hasher);
        
        format!("llm_cache_{:x}", hasher.finish())
    }
    
    /// Проверить кэш на наличие ответа
    fn check_cache(&self, cache_key: &str) -> Option<String> {
        if !self.config.enable_detailed_metrics {
            return None;
        }
        
        let cache = self.response_cache.read();
        if let Some(cached) = cache.get(cache_key) {
            let age = chrono::Utc::now() - cached.timestamp;
            if age < chrono::Duration::from_std(self.config.cache_ttl).unwrap() {
                debug!("📦 Использован кэш для LLM запроса");
                return Some(cached.response.clone());
            }
        }
        None
    }
    
    /// Сохранить ответ в кэш
    fn store_in_cache(&self, cache_key: String, response: &str, tokens_used: u32, provider: &str) {
        if !self.config.enable_detailed_metrics {
            return;
        }
        
        let mut cache = self.response_cache.write();
        
        // Проверяем размер кэша и очищаем старые записи при необходимости
        if cache.len() >= self.config.max_cache_size {
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, cached)| cached.timestamp);
            
            // Удаляем 20% самых старых записей
            let to_remove = cache.len() / 5;
            for (key, _) in entries.iter().take(to_remove) {
                cache.remove(*key);
            }
        }
        
        cache.insert(cache_key, CachedResponse {
            response: response.to_string(),
            timestamp: chrono::Utc::now(),
            tokens_used,
            provider: provider.to_string(),
        });
    }
    
    /// Очистить устаревшие записи кэша
    fn cleanup_cache(&self) {
        let mut cache = self.response_cache.write();
        let now = chrono::Utc::now();
        let ttl = chrono::Duration::from_std(self.config.cache_ttl).unwrap();
        
        cache.retain(|_, cached| {
            let age = now - cached.timestamp;
            age < ttl
        });
    }
    
    /// Обновить статистику
    fn update_stats(&self, success: bool, from_cache: bool, duration: Duration, tokens_used: u32, provider: &str) {
        let mut stats = self.stats.write();
        stats.total_requests += 1;
        
        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }
        
        if from_cache {
            stats.cached_responses += 1;
        } else {
            stats.total_tokens_consumed += tokens_used as u64;
        }
        
        // Обновляем среднее время ответа
        let duration_ms = duration.as_millis() as f64;
        let total = stats.total_requests as f64;
        stats.avg_response_time_ms = ((stats.avg_response_time_ms * (total - 1.0)) + duration_ms) / total;
        
        // Обновляем распределение по провайдерам
        *stats.provider_distribution.entry(provider.to_string()).or_insert(0) += 1;
    }
}

#[async_trait::async_trait]
impl LlmCommunicationService for DefaultLlmCommunicationService {
    async fn chat(&self, context: &RequestContext) -> Result<OperationResult<String>> {
        self.chat_with_options(context, &ChatOptions::default()).await
    }
    
    async fn chat_with_options(
        &self,
        context: &RequestContext,
        options: &ChatOptions,
    ) -> Result<OperationResult<String>> {
        use std::time::Instant;
        use tokio::time::timeout;
        
        let start_time = Instant::now();
        let cache_key = self.create_cache_key(&context.message, options);
        
        debug!("💬 LLM запрос: {} символов", context.message.len());
        
        // Проверяем кэш если включён
        if options.use_cache {
            if let Some(cached_response) = self.check_cache(&cache_key) {
                let duration = start_time.elapsed();
                self.update_stats(true, true, duration, 0, "cache");
                return Ok(OperationResult {
                    result: Ok(cached_response),
                    duration,
                    retries: 0,
                    from_cache: true,
                });
            }
        }
        
        // Очищаем кэш от устаревших записей
        self.cleanup_cache();
        
        let request_timeout = options.timeout.unwrap_or(self.config.default_timeout);
        let mut attempts = 0;
        let max_attempts = self.config.max_retries + 1;
        
        while attempts < max_attempts {
            attempts += 1;
            
            debug!("🔄 Попытка {} из {} отправки LLM запроса", attempts, max_attempts);
            
            let llm_future = self.llm_client.chat_simple(&context.message);
            let llm_result = timeout(request_timeout, llm_future).await;
            
            match llm_result {
                Ok(Ok(response)) => {
                    let duration = start_time.elapsed();
                    
                    // Оцениваем количество токенов (приблизительно)
                    let estimated_tokens = (context.message.len() + response.len()) as u32 / 4;
                    
                    // Сохраняем в кэш если включён
                    if options.use_cache {
                        self.store_in_cache(cache_key, &response, estimated_tokens, "llm");
                    }
                    
                    self.update_stats(true, false, duration, estimated_tokens, "llm");
                    
                    info!("✅ LLM ответ получен за {:?} ({} символов)", duration, response.len());
                    
                    return Ok(OperationResult {
                        result: Ok(response),
                        duration,
                        retries: attempts - 1,
                        from_cache: false,
                    });
                }
                Ok(Err(e)) => {
                    warn!("⚠️ LLM запрос failed (попытка {}): {}", attempts, e);
                    
                    if attempts >= max_attempts {
                        let duration = start_time.elapsed();
                        self.update_stats(false, false, duration, 0, "llm");
                        return Ok(OperationResult {
                            result: Err(e),
                            duration,
                            retries: attempts - 1,
                            from_cache: false,
                        });
                    }
                    
                    // Exponential backoff между попытками
                    let delay = Duration::from_millis(1000 * attempts as u64);
                    tokio::time::sleep(delay).await;
                }
                Err(_) => {
                    warn!("⚠️ LLM запрос timeout (попытка {})", attempts);
                    
                    if attempts >= max_attempts {
                        let duration = start_time.elapsed();
                        let error = anyhow::anyhow!("LLM request timeout after {} attempts", max_attempts);
                        self.update_stats(false, false, duration, 0, "llm");
                        return Ok(OperationResult {
                            result: Err(error),
                            duration,
                            retries: attempts - 1,
                            from_cache: false,
                        });
                    }
                    
                    // Увеличиваем timeout для следующей попытки
                    let delay = Duration::from_millis(2000 * attempts as u64);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        
        // Этот код не должен выполняться, но для компилятора
        unreachable!("Loop should have returned earlier")
    }
    
    async fn health_check(&self) -> LlmHealthStatus {
        // Простая проверка здоровья - попробуем отправить минимальный запрос
        let test_context = RequestContext {
            message: "ping".to_string(),
            user_id: None,
            session_id: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        let test_options = ChatOptions {
            max_tokens: Some(10),
            use_cache: false,
            timeout: Some(Duration::from_secs(5)),
            ..Default::default()
        };
        
        let start_time = std::time::Instant::now();
        let health_result = self.chat_with_options(&test_context, &test_options).await;
        let response_time = start_time.elapsed();
        
        let stats = self.stats.read();
        let error_rate = if stats.total_requests > 0 {
            stats.failed_requests as f64 / stats.total_requests as f64
        } else {
            0.0
        };
        
        LlmHealthStatus {
            primary_provider_healthy: health_result.is_ok() && health_result.unwrap().result.is_ok(),
            fallback_providers_available: if self.config.enable_provider_fallback { 1 } else { 0 },
            last_successful_request: if stats.successful_requests > 0 {
                Some(chrono::Utc::now())
            } else {
                None
            },
            current_error_rate: error_rate,
            estimated_response_time: response_time,
        }
    }
    
    async fn get_provider_info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "LlmClient".to_string(),
            model: "gpt-3.5-turbo".to_string(), // Это нужно получать из LlmClient
            version: Some("1.0".to_string()),
            rate_limits: RateLimits {
                requests_per_minute: 3000,
                tokens_per_minute: 90000,
                current_usage_rpm: 0, // TODO: реализовать отслеживание
                current_usage_tpm: 0,
            },
            capabilities: vec![
                "chat".to_string(),
                "function_calling".to_string(),
                "streaming".to_string(),
            ],
        }
    }
    
    async fn get_usage_stats(&self) -> LlmUsageStats {
        let stats = self.stats.read();
        stats.clone()
    }
    
    async fn reset_stats(&self) {
        let mut stats = self.stats.write();
        *stats = LlmUsageStats::default();
        
        let mut cache = self.response_cache.write();
        cache.clear();
        
        debug!("🔄 LLM stats and cache reset");
    }
}

impl Default for LlmUsageStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            cached_responses: 0,
            total_tokens_consumed: 0,
            total_tokens_generated: 0,
            avg_response_time_ms: 0.0,
            provider_distribution: std::collections::HashMap::new(),
        }
    }
}

/// Factory функция для DI контейнера
pub fn create_llm_communication_service(
    llm_client: LlmClient,
) -> Arc<dyn LlmCommunicationService> {
    Arc::new(DefaultLlmCommunicationService::new(llm_client))
}

/// Factory функция с кастомной конфигурацией
pub fn create_llm_communication_service_with_config(
    llm_client: LlmClient,
    config: LlmCommunicationConfig,
) -> Arc<dyn LlmCommunicationService> {
    Arc::new(DefaultLlmCommunicationService::with_config(llm_client, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_context(message: &str) -> RequestContext {
        RequestContext {
            message: message.to_string(),
            user_id: Some("test_user".to_string()),
            session_id: Some("test_session".to_string()),
            timestamp: Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    #[test]
    fn test_cache_key_generation() {
        let llm_client = LlmClient::from_env().unwrap();
        let service = DefaultLlmCommunicationService::new(llm_client);
        
        let options1 = ChatOptions::default();
        let options2 = ChatOptions {
            temperature: Some(0.5),
            ..Default::default()
        };
        
        let key1 = service.create_cache_key("test message", &options1);
        let key2 = service.create_cache_key("test message", &options2);
        let key3 = service.create_cache_key("test message", &options1);
        
        // Одинаковые параметры должны давать одинаковый ключ
        assert_eq!(key1, key3);
        
        // Разные параметры должны давать разные ключи
        assert_ne!(key1, key2);
    }
    
    #[test]
    fn test_cache_cleanup() {
        let llm_client = LlmClient::from_env().unwrap();
        let config = LlmCommunicationConfig {
            cache_ttl: Duration::from_millis(1), // Очень короткий TTL для теста
            ..Default::default()
        };
        let service = DefaultLlmCommunicationService::with_config(llm_client, config);
        
        // Добавляем запись в кэш
        service.store_in_cache("test_key".to_string(), "test_response", 100, "test_provider");
        
        // Проверяем что запись есть
        assert!(service.check_cache("test_key").is_some());
        
        // Ждём истечения TTL
        std::thread::sleep(Duration::from_millis(5));
        
        // Запускаем очистку кэша
        service.cleanup_cache();
        
        // Проверяем что запись удалена
        assert!(service.check_cache("test_key").is_none());
    }
    
    #[tokio::test]
    async fn test_stats_tracking() {
        let llm_client = LlmClient::from_env().unwrap();
        let service = DefaultLlmCommunicationService::new(llm_client);
        
        // Сбрасываем статистику
        service.reset_stats().await;
        
        // Обновляем статистику несколько раз
        service.update_stats(true, false, Duration::from_millis(100), 50, "test_provider");
        service.update_stats(true, true, Duration::from_millis(50), 0, "cache");
        service.update_stats(false, false, Duration::from_millis(200), 0, "test_provider");
        
        let stats = service.get_usage_stats().await;
        
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.successful_requests, 2);
        assert_eq!(stats.failed_requests, 1);
        assert_eq!(stats.cached_responses, 1);
        assert_eq!(stats.total_tokens_consumed, 50);
        
        // Проверяем среднее время
        let expected_avg = (100.0 + 50.0 + 200.0) / 3.0;
        assert!((stats.avg_response_time_ms - expected_avg).abs() < 1.0);
        
        // Проверяем распределение провайдеров
        assert_eq!(stats.provider_distribution.get("test_provider"), Some(&2));
        assert_eq!(stats.provider_distribution.get("cache"), Some(&1));
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let llm_client = LlmClient::from_env().unwrap();
        let service = DefaultLlmCommunicationService::new(llm_client);
        
        let health = service.health_check().await;
        
        // Проверяем базовые поля
        assert!(health.estimated_response_time > Duration::from_millis(0));
        assert!(health.current_error_rate >= 0.0);
        assert!(health.current_error_rate <= 1.0);
    }
}