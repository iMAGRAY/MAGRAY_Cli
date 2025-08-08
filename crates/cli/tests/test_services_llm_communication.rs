#![cfg(all(feature = "extended-tests", feature = "legacy-tests"))]

//! Comprehensive tests for LlmCommunicationService
//!
//! Покрывает:
//! - Unit тесты для DefaultLlmCommunicationService
//! - Mock'и для LlmClient
//! - Caching mechanism тестирование
//! - Retry logic и error handling
//! - Rate limiting и timeout обработка
//! - Статистика и метрики

use anyhow::{anyhow, Result};
use chrono::Utc;
use cli::services::llm_communication::{
    ChatOptions, DefaultLlmCommunicationService, LlmCommunicationConfig, LlmCommunicationService,
    LlmHealthStatus, LlmUsageStats, ProviderInfo, RateLimits,
};
use cli::services::types::{OperationResult, RequestContext};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio;

/// Mock LlmClient для тестирования
pub struct MockLlmClient {
    /// Предопределенные ответы для различных запросов
    responses: HashMap<String, Result<String>>,
    /// Задержка перед ответом (для timeout тестов)
    delay: Option<Duration>,
    /// Счетчик обращений
    call_count: Arc<std::sync::atomic::AtomicUsize>,
    /// Симуляция ошибки для определенных запросов
    should_fail: bool,
    /// Максимальное количество попыток перед успехом (для retry тестов)
    fail_attempts: usize,
    /// Текущий номер попытки
    current_attempt: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockLlmClient {
    pub fn new() -> Self {
        let mut responses = HashMap::new();

        // Предустановленные ответы для тестов
        responses.insert(
            "привет мир".to_string(),
            Ok("Привет! Как дела?".to_string()),
        );

        responses.insert(
            "что такое Rust".to_string(),
            Ok("Rust - это системный язык программирования, ориентированный на безопасность и производительность.".to_string())
        );

        responses.insert("ping".to_string(), Ok("pong".to_string()));

        responses.insert(
            "error_test".to_string(),
            Err(anyhow!("Simulated LLM error")),
        );

        Self {
            responses,
            delay: None,
            call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            should_fail: false,
            fail_attempts: 0,
            current_attempt: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn with_retry_after_fails(mut self, fail_attempts: usize) -> Self {
        self.fail_attempts = fail_attempts;
        self
    }

    pub fn get_call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_response(&mut self, query: &str, response: Result<String>) {
        self.responses.insert(query.to_string(), response);
    }

    pub async fn chat_simple(&self, message: &str) -> Result<String> {
        self.call_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }

        // Проверяем retry логику
        if self.fail_attempts > 0 {
            let attempt = self
                .current_attempt
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if attempt < self.fail_attempts {
                return Err(anyhow!("Mock client failing on attempt {}", attempt + 1));
            }
        }

        if self.should_fail {
            return Err(anyhow!("Mock client configured to fail"));
        }

        self.responses
            .get(message)
            .cloned()
            .unwrap_or_else(|| Ok(format!("Default LLM response for: {}", message)))
    }
}

fn create_test_context(message: &str) -> RequestContext {
    RequestContext {
        message: message.to_string(),
        user_id: Some("test_user".to_string()),
        session_id: Some("test_session".to_string()),
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    }
}

// Поскольку мы не можем напрямую создать LlmClient в тестах,
// создадим helper функция для тестирования
fn create_test_service(mock_client: MockLlmClient) -> DefaultLlmCommunicationService {
    DefaultLlmCommunicationService::new_for_testing(mock_client)
}

// Добавляем метод для тестирования в DefaultLlmCommunicationService
impl DefaultLlmCommunicationService {
    #[cfg(test)]
    pub fn new_for_testing(mock_client: MockLlmClient) -> Self {
        Self {
            llm_client: TestLlmClientWrapper::new(mock_client),
            stats: parking_lot::RwLock::new(LlmUsageStats::default()),
            response_cache: parking_lot::RwLock::new(HashMap::new()),
            config: LlmCommunicationConfig::default(),
        }
    }

    #[cfg(test)]
    pub fn with_config_for_testing(
        mock_client: MockLlmClient,
        config: LlmCommunicationConfig,
    ) -> Self {
        Self {
            llm_client: TestLlmClientWrapper::new(mock_client),
            stats: parking_lot::RwLock::new(LlmUsageStats::default()),
            response_cache: parking_lot::RwLock::new(HashMap::new()),
            config,
        }
    }
}

/// Wrapper для интеграции MockLlmClient с реальным API
struct TestLlmClientWrapper {
    mock: Arc<parking_lot::Mutex<MockLlmClient>>,
}

impl TestLlmClientWrapper {
    fn new(mock: MockLlmClient) -> Self {
        Self {
            mock: Arc::new(parking_lot::Mutex::new(mock)),
        }
    }

    async fn chat_simple(&self, message: &str) -> Result<String> {
        let mock = self.mock.lock();
        mock.chat_simple(message).await
    }
}

#[test]
fn test_cache_key_generation() {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let options1 = ChatOptions::default();
    let options2 = ChatOptions {
        temperature: Some(0.5),
        ..Default::default()
    };
    let options3 = ChatOptions {
        max_tokens: Some(2000),
        ..Default::default()
    };

    let key1 = service.create_cache_key("test message", &options1);
    let key2 = service.create_cache_key("test message", &options2);
    let key3 = service.create_cache_key("test message", &options1);
    let key4 = service.create_cache_key("different message", &options1);
    let key5 = service.create_cache_key("test message", &options3);

    // Одинаковые параметры должны давать одинаковый ключ
    assert_eq!(key1, key3);

    // Разные параметры должны давать разные ключи
    assert_ne!(key1, key2);
    assert_ne!(key1, key4);
    assert_ne!(key1, key5);

    // Проверяем что ключи имеют правильный формат
    assert!(key1.starts_with("llm_cache_"));
    assert!(key2.starts_with("llm_cache_"));
}

#[test]
fn test_cache_storage_and_retrieval() {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let cache_key = "test_cache_key".to_string();
    let response = "Test cached response";

    // Проверяем что кэш пуст
    assert!(service.check_cache(&cache_key).is_none());

    // Сохраняем в кэш
    service.store_in_cache(cache_key.clone(), response, 100, "test_provider");

    // Проверяем что можем получить из кэша
    assert_eq!(service.check_cache(&cache_key), Some(response.to_string()));
}

#[test]
fn test_cache_ttl_expiration() {
    let mock_client = MockLlmClient::new();
    let config = LlmCommunicationConfig {
        cache_ttl: Duration::from_millis(10), // Очень короткий TTL
        ..Default::default()
    };
    let service = DefaultLlmCommunicationService::with_config_for_testing(mock_client, config);

    let cache_key = "test_ttl_key".to_string();
    let response = "Test TTL response";

    // Сохраняем в кэш
    service.store_in_cache(cache_key.clone(), response, 50, "test_provider");

    // Проверяем что есть в кэше
    assert_eq!(service.check_cache(&cache_key), Some(response.to_string()));

    // Ждём истечения TTL
    std::thread::sleep(Duration::from_millis(20));

    // Проверяем что кэш истёк
    assert!(service.check_cache(&cache_key).is_none());
}

#[test]
fn test_cache_size_limit() {
    let mock_client = MockLlmClient::new();
    let config = LlmCommunicationConfig {
        max_cache_size: 3, // Очень маленький размер кэша
        ..Default::default()
    };
    let service = DefaultLlmCommunicationService::with_config_for_testing(mock_client, config);

    // Заполняем кэш до лимита
    for i in 0..5 {
        let key = format!("key_{}", i);
        let response = format!("response_{}", i);
        service.store_in_cache(key, &response, 10, "test");
        std::thread::sleep(Duration::from_millis(1)); // Чтобы timestamp отличались
    }

    // Проверяем что размер кэша не превышает лимит
    let cache = service.response_cache.read();
    assert!(cache.len() <= 3);
}

#[tokio::test]
async fn test_chat_success() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let context = create_test_context("привет мир");
    let result = service.chat(&context).await?;

    assert!(result.result.is_ok());
    if let Ok(response) = result.result {
        assert_eq!(response, "Привет! Как дела?");
    }

    assert!(!result.from_cache);
    assert_eq!(result.retries, 0);
    assert!(result.duration > Duration::from_millis(0));

    // Проверяем статистику
    let stats = service.get_usage_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_requests, 1);
    assert_eq!(stats.failed_requests, 0);

    Ok(())
}

#[tokio::test]
async fn test_chat_with_options() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let context = create_test_context("что такое Rust");
    let options = ChatOptions {
        max_tokens: Some(500),
        temperature: Some(0.5),
        use_cache: false,
        ..Default::default()
    };

    let result = service.chat_with_options(&context, &options).await?;

    assert!(result.result.is_ok());
    if let Ok(response) = result.result {
        assert!(response.contains("Rust"));
        assert!(response.contains("язык программирования"));
    }

    Ok(())
}

#[tokio::test]
async fn test_caching_behavior() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let context = create_test_context("привет мир");
    let options = ChatOptions {
        use_cache: true,
        ..Default::default()
    };

    // Первый запрос - не из кэша
    let result1 = service.chat_with_options(&context, &options).await?;
    assert!(!result1.from_cache);

    // Второй запрос - из кэша
    let result2 = service.chat_with_options(&context, &options).await?;
    assert!(result2.from_cache);

    // Проверяем что ответы одинаковые
    assert_eq!(result1.result, result2.result);

    // Проверяем статистику
    let stats = service.get_usage_stats().await;
    assert_eq!(stats.total_requests, 2);
    assert_eq!(stats.cached_responses, 1);

    Ok(())
}

#[tokio::test]
async fn test_cache_disabled() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let context = create_test_context("привет мир");
    let options = ChatOptions {
        use_cache: false,
        ..Default::default()
    };

    // Два запроса без кэша
    let result1 = service.chat_with_options(&context, &options).await?;
    let result2 = service.chat_with_options(&context, &options).await?;

    assert!(!result1.from_cache);
    assert!(!result2.from_cache);

    // Проверяем статистику
    let stats = service.get_usage_stats().await;
    assert_eq!(stats.total_requests, 2);
    assert_eq!(stats.cached_responses, 0);

    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let mock_client = MockLlmClient::new().with_failure();
    let service = create_test_service(mock_client);

    let context = create_test_context("любое сообщение");
    let result = service.chat(&context).await?;

    assert!(result.result.is_err());
    if let Err(e) = result.result {
        assert!(e.to_string().contains("Mock client configured to fail"));
    }

    // Проверяем статистику
    let stats = service.get_usage_stats().await;
    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.failed_requests, 1);
    assert_eq!(stats.successful_requests, 0);

    Ok(())
}

#[tokio::test]
async fn test_retry_logic() -> Result<()> {
    let mock_client = MockLlmClient::new().with_retry_after_fails(2); // Fail 2 times, succeed on 3rd
    let config = LlmCommunicationConfig {
        max_retries: 3,
        ..Default::default()
    };
    let service = DefaultLlmCommunicationService::with_config_for_testing(mock_client, config);

    let context = create_test_context("retry test");
    let result = service.chat(&context).await?;

    assert!(result.result.is_ok());
    assert_eq!(result.retries, 2); // Должно быть 2 retry

    // Проверяем что клиент был вызван 3 раза
    let call_count = {
        let client_wrapper = &service.llm_client;
        let mock = client_wrapper.mock.lock();
        mock.get_call_count()
    };
    assert_eq!(call_count, 3);

    Ok(())
}

#[tokio::test]
async fn test_retry_exhaustion() -> Result<()> {
    let mock_client = MockLlmClient::new().with_retry_after_fails(5); // More fails than max retries
    let config = LlmCommunicationConfig {
        max_retries: 2,
        ..Default::default()
    };
    let service = DefaultLlmCommunicationService::with_config_for_testing(mock_client, config);

    let context = create_test_context("retry exhaustion test");
    let result = service.chat(&context).await?;

    assert!(result.result.is_err());
    assert_eq!(result.retries, 2); // Максимальное количество retries

    let stats = service.get_usage_stats().await;
    assert_eq!(stats.failed_requests, 1);

    Ok(())
}

#[tokio::test]
async fn test_timeout_handling() -> Result<()> {
    let mock_client = MockLlmClient::new().with_delay(Duration::from_secs(2)); // Дольше timeout'а

    let context = create_test_context("timeout test");
    let options = ChatOptions {
        timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    let service = create_test_service(mock_client);
    let result = service.chat_with_options(&context, &options).await?;

    assert!(result.result.is_err());
    if let Err(e) = result.result {
        assert!(e.to_string().contains("timeout"));
    }

    let stats = service.get_usage_stats().await;
    assert_eq!(stats.failed_requests, 1);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = Arc::new(create_test_service(mock_client));

    let mut handles = vec![];

    // Создаем 10 concurrent запросов
    for i in 0..10 {
        let service_clone = service.clone();
        let handle = tokio::spawn(async move {
            let context = create_test_context(&format!("concurrent request {}", i));
            service_clone.chat(&context).await
        });
        handles.push(handle);
    }

    // Ждем завершения всех запросов
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            if result.result.is_ok() {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 10);

    let stats = service.get_usage_stats().await;
    assert_eq!(stats.total_requests, 10);
    assert_eq!(stats.successful_requests, 10);

    Ok(())
}

#[tokio::test]
async fn test_health_check() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let health = service.health_check().await;

    assert!(health.primary_provider_healthy);
    assert!(health.estimated_response_time > Duration::from_millis(0));
    assert!(health.current_error_rate >= 0.0);
    assert!(health.current_error_rate <= 1.0);

    Ok(())
}

#[tokio::test]
async fn test_health_check_with_failures() -> Result<()> {
    let mock_client = MockLlmClient::new().with_failure();
    let service = create_test_service(mock_client);

    // Сначала создаем несколько неудачных запросов для статистики
    let context = create_test_context("test");
    let _ = service.chat(&context).await;
    let _ = service.chat(&context).await;

    let health = service.health_check().await;

    assert!(!health.primary_provider_healthy);
    assert!(health.current_error_rate > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_provider_info() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    let info = service.get_provider_info().await;

    assert_eq!(info.name, "LlmClient");
    assert!(!info.model.is_empty());
    assert!(info.capabilities.contains(&"chat".to_string()));
    assert!(info.rate_limits.requests_per_minute > 0);
    assert!(info.rate_limits.tokens_per_minute > 0);

    Ok(())
}

#[tokio::test]
async fn test_usage_stats_tracking() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    // Сбрасываем статистику
    service.reset_stats().await;

    // Выполняем различные запросы
    let context1 = create_test_context("привет мир");
    let context2 = create_test_context("что такое Rust");

    // Успешный запрос
    service.chat(&context1).await?;

    // Запрос с кэшированием
    let options = ChatOptions {
        use_cache: true,
        ..Default::default()
    };
    service.chat_with_options(&context1, &options).await?; // Из кэша

    // Другой успешный запрос
    service.chat(&context2).await?;

    let stats = service.get_usage_stats().await;

    assert_eq!(stats.total_requests, 3);
    assert_eq!(stats.successful_requests, 3);
    assert_eq!(stats.failed_requests, 0);
    assert_eq!(stats.cached_responses, 1);
    assert!(stats.total_tokens_consumed > 0);
    assert!(stats.avg_response_time_ms > 0.0);

    // Проверяем распределение провайдеров
    assert!(stats.provider_distribution.contains_key("llm"));
    assert!(stats.provider_distribution.contains_key("cache"));

    Ok(())
}

#[tokio::test]
async fn test_stats_reset() -> Result<()> {
    let mock_client = MockLlmClient::new();
    let service = create_test_service(mock_client);

    // Выполняем несколько запросов
    let context = create_test_context("тест");
    service.chat(&context).await?;
    service.chat(&context).await?;

    let stats_before = service.get_usage_stats().await;
    assert_eq!(stats_before.total_requests, 2);

    // Сбрасываем статистику
    service.reset_stats().await;

    let stats_after = service.get_usage_stats().await;
    assert_eq!(stats_after.total_requests, 0);
    assert_eq!(stats_after.successful_requests, 0);
    assert_eq!(stats_after.failed_requests, 0);
    assert_eq!(stats_after.cached_responses, 0);
    assert_eq!(stats_after.total_tokens_consumed, 0);
    assert_eq!(stats_after.avg_response_time_ms, 0.0);
    assert!(stats_after.provider_distribution.is_empty());

    // Проверяем что кэш тоже очищен
    let cache = service.response_cache.read();
    assert!(cache.is_empty());

    Ok(())
}

#[test]
fn test_config_defaults() {
    let config = LlmCommunicationConfig::default();

    assert_eq!(config.cache_ttl, Duration::from_secs(30 * 60)); // 30 minutes
    assert_eq!(config.max_cache_size, 1000);
    assert_eq!(config.enable_provider_fallback, true);
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.default_timeout, Duration::from_secs(30));
    assert_eq!(config.enable_detailed_metrics, true);
}

#[test]
fn test_chat_options_defaults() {
    let options = ChatOptions::default();

    assert_eq!(options.max_tokens, Some(4000));
    assert_eq!(options.temperature, Some(0.7));
    assert_eq!(options.top_p, Some(0.9));
    assert_eq!(options.system_prompt, None);
    assert_eq!(options.use_cache, true);
    assert_eq!(options.preferred_provider, None);
    assert_eq!(options.timeout, Some(Duration::from_secs(30)));
}

#[test]
fn test_custom_config() {
    let custom_config = LlmCommunicationConfig {
        cache_ttl: Duration::from_secs(600),
        max_cache_size: 500,
        enable_provider_fallback: false,
        max_retries: 5,
        default_timeout: Duration::from_secs(60),
        enable_detailed_metrics: false,
    };

    let mock_client = MockLlmClient::new();
    let service =
        DefaultLlmCommunicationService::with_config_for_testing(mock_client, custom_config);

    // Проверяем что кэш не работает с выключенными метриками
    let cache_key = "test";
    service.store_in_cache(cache_key.to_string(), "response", 100, "provider");
    assert!(service.check_cache(cache_key).is_none());
}

#[test]
fn test_rate_limits_construction() {
    let limits = RateLimits {
        requests_per_minute: 1000,
        tokens_per_minute: 50000,
        current_usage_rpm: 100,
        current_usage_tpm: 5000,
    };

    assert_eq!(limits.requests_per_minute, 1000);
    assert_eq!(limits.tokens_per_minute, 50000);
    assert_eq!(limits.current_usage_rpm, 100);
    assert_eq!(limits.current_usage_tpm, 5000);
}

#[test]
fn test_provider_info_construction() {
    let capabilities = vec!["chat".to_string(), "function_calling".to_string()];
    let limits = RateLimits {
        requests_per_minute: 3000,
        tokens_per_minute: 90000,
        current_usage_rpm: 0,
        current_usage_tpm: 0,
    };

    let info = ProviderInfo {
        name: "Test Provider".to_string(),
        model: "gpt-4".to_string(),
        version: Some("1.0.0".to_string()),
        rate_limits: limits,
        capabilities: capabilities.clone(),
    };

    assert_eq!(info.name, "Test Provider");
    assert_eq!(info.model, "gpt-4");
    assert_eq!(info.version, Some("1.0.0".to_string()));
    assert_eq!(info.capabilities, capabilities);
}
