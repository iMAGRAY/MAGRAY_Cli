# Error Handling Strategy для MAGRAY CLI

## 🎯 Принципы обработки ошибок

### 1. **Zero Panic Policy**
- ❌ НИКОГДА не используем `.unwrap()` или `.expect()` в production коде
- ✅ Используем `?` operator для propagation
- ✅ Graceful degradation для всех критичных операций

### 2. **Error Hierarchy**
```rust
// Основная иерархия ошибок
pub enum MagrayError {
    // Системные ошибки
    Io(std::io::Error),
    Database(DatabaseError),
    Network(NetworkError),
    
    // Бизнес-логика
    Validation(ValidationError),
    NotFound(String),
    Conflict(String),
    
    // AI/ML specific
    EmbeddingError(EmbeddingError),
    ModelLoadError(String),
    GpuError(GpuError),
    
    // Memory system
    MemoryError(MemoryError),
    CacheError(CacheError),
    IndexError(IndexError),
}
```

### 3. **Error Context Pattern**
```rust
use anyhow::{Result, Context};

// Добавляем контекст к ошибкам
let file = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read config file: {}", path))?;

// Chain contexts для debugging
let model = load_model(&config)
    .context("Loading AI model")
    .context("During service initialization")?;
```

### 4. **Graceful Degradation Levels**

#### Level 1: Try Alternative
```rust
// Сначала GPU, потом CPU
let result = match gpu_process(data).await {
    Ok(res) => res,
    Err(e) => {
        warn!("GPU failed: {}, falling back to CPU", e);
        cpu_process(data).await?
    }
};
```

#### Level 2: Use Default/Cached
```rust
// Используем кэшированное значение при сбое
let embedding = match embedding_service.embed(text).await {
    Ok(emb) => emb,
    Err(e) => {
        warn!("Embedding failed: {}, using cached", e);
        cache.get_or_default(text)
    }
};
```

#### Level 3: Partial Functionality
```rust
// Работаем с ограниченной функциональностью
let features = match load_all_features().await {
    Ok(all) => all,
    Err(e) => {
        error!("Some features unavailable: {}", e);
        load_essential_features().await?
    }
};
```

## 🛡️ Patterns для разных компонентов

### Storage Layer
```rust
impl VectorStore {
    pub async fn search(&self, query: &[f32], options: SearchOptions) -> Result<Vec<Record>> {
        // Validate input
        if query.is_empty() {
            return Err(MagrayError::Validation("Empty query vector".into()).into());
        }
        
        // Try with retry
        let mut attempts = 0;
        loop {
            match self.search_internal(query, &options).await {
                Ok(results) => return Ok(results),
                Err(e) if attempts < 3 && e.is_retriable() => {
                    warn!("Search attempt {} failed: {}, retrying", attempts + 1, e);
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(100 * attempts)).await;
                }
                Err(e) => {
                    error!("Search failed after {} attempts: {}", attempts, e);
                    return Err(e);
                }
            }
        }
    }
}
```

### Service Layer
```rust
impl MemoryService {
    pub async fn process_request(&self, req: Request) -> Result<Response> {
        // Validate request
        req.validate().context("Invalid request")?;
        
        // Process with monitoring
        let start = Instant::now();
        let result = self.process_internal(req).await;
        let duration = start.elapsed();
        
        // Record metrics regardless of outcome
        self.metrics.record_request(duration, result.is_ok());
        
        // Handle result
        match result {
            Ok(response) => Ok(response),
            Err(e) if e.is_recoverable() => {
                warn!("Recoverable error: {}", e);
                self.create_partial_response(req).await
            }
            Err(e) => {
                error!("Unrecoverable error: {}", e);
                self.alert_manager.send_alert(AlertLevel::High, &e);
                Err(e)
            }
        }
    }
}
```

### Batch Operations
```rust
impl BatchManager {
    pub async fn process_batch(&self, items: Vec<Item>) -> BatchResult {
        let mut results = BatchResult::new();
        
        for (idx, item) in items.into_iter().enumerate() {
            match self.process_item(item).await {
                Ok(result) => results.add_success(idx, result),
                Err(e) => {
                    warn!("Batch item {} failed: {}", idx, e);
                    results.add_failure(idx, e);
                }
            }
        }
        
        // Batch считается успешным если >50% успешно
        if results.success_rate() < 0.5 {
            error!("Batch failed with {} failures out of {}", 
                   results.failures.len(), results.total());
        }
        
        results
    }
}
```

## 🔄 Retry Strategies

### Exponential Backoff
```rust
pub struct RetryPolicy {
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    exponential_base: f32,
}

impl RetryPolicy {
    pub async fn execute<F, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
        E: std::fmt::Display + IsRetriable,
    {
        let mut delay = self.initial_delay;
        
        for attempt in 0..self.max_attempts {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) if !e.is_retriable() => return Err(e),
                Err(e) if attempt == self.max_attempts - 1 => return Err(e),
                Err(e) => {
                    warn!("Attempt {} failed: {}, retrying in {:?}", 
                          attempt + 1, e, delay);
                    tokio::time::sleep(delay).await;
                    delay = (delay.as_secs_f32() * self.exponential_base)
                        .min(self.max_delay.as_secs_f32())
                        .max(0.0);
                    delay = Duration::from_secs_f32(delay);
                }
            }
        }
        
        unreachable!()
    }
}
```

### Circuit Breaker
```rust
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    state: Arc<RwLock<CircuitState>>,
}

enum CircuitState {
    Closed,
    Open(Instant),
    HalfOpen,
}

impl CircuitBreaker {
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let state = self.state.read().await;
        
        match *state {
            CircuitState::Open(since) => {
                if since.elapsed() > self.recovery_timeout {
                    drop(state);
                    *self.state.write().await = CircuitState::HalfOpen;
                } else {
                    return Err(anyhow!("Circuit breaker is open"));
                }
            }
            CircuitState::HalfOpen => {
                // Try one request
                drop(state);
                match f.await {
                    Ok(result) => {
                        *self.state.write().await = CircuitState::Closed;
                        Ok(result)
                    }
                    Err(e) => {
                        *self.state.write().await = CircuitState::Open(Instant::now());
                        Err(e)
                    }
                }
            }
            CircuitState::Closed => {
                drop(state);
                f.await
            }
        }
    }
}
```

## 📊 Error Monitoring & Alerting

### Error Metrics
```rust
pub struct ErrorMetrics {
    total_errors: AtomicU64,
    errors_by_type: DashMap<String, AtomicU64>,
    error_rate: RateLimiter,
}

impl ErrorMetrics {
    pub fn record_error(&self, error: &MagrayError) {
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        
        let error_type = error.type_name();
        self.errors_by_type
            .entry(error_type.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
        
        // Alert if error rate is too high
        if self.error_rate.check() > self.threshold {
            self.alert_high_error_rate();
        }
    }
}
```

### Structured Error Logging
```rust
#[derive(Serialize)]
struct ErrorContext {
    timestamp: DateTime<Utc>,
    error_type: String,
    message: String,
    stack_trace: Option<String>,
    context: HashMap<String, Value>,
}

pub fn log_error(error: &MagrayError, context: HashMap<String, Value>) {
    let error_context = ErrorContext {
        timestamp: Utc::now(),
        error_type: error.type_name().to_string(),
        message: error.to_string(),
        stack_trace: backtrace::Backtrace::new().to_string(),
        context,
    };
    
    error!(
        error = serde_json::to_string(&error_context).unwrap(),
        "Error occurred"
    );
}
```

## 🧪 Testing Error Scenarios

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_graceful_degradation() {
        let service = TestService::new();
        
        // Simulate GPU failure
        service.mock_gpu_failure();
        
        // Should fallback to CPU
        let result = service.process(data).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().processing_type, ProcessingType::Cpu);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(5));
        let failing_service = FailingService::new();
        
        // First 3 calls should fail
        for _ in 0..3 {
            assert!(breaker.call(failing_service.call()).await.is_err());
        }
        
        // Circuit should be open now
        assert!(matches!(
            breaker.call(failing_service.call()).await,
            Err(e) if e.to_string().contains("Circuit breaker is open")
        ));
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_batch_partial_failure() {
    let service = create_test_service().await;
    
    // Create batch with some invalid items
    let items = vec![
        valid_item(),
        invalid_item(), // This will fail
        valid_item(),
    ];
    
    let result = service.process_batch(items).await;
    
    // Should process valid items despite failures
    assert_eq!(result.successes.len(), 2);
    assert_eq!(result.failures.len(), 1);
    assert!(result.success_rate() > 0.5);
}
```

## 📋 Checklist для разработчиков

### При написании нового кода:
- [ ] Использовать `Result<T, E>` вместо `Option<T>` для операций, которые могут fail
- [ ] Добавлять контекст через `.context()` для всех внешних вызовов
- [ ] Реализовать graceful degradation для критичных операций
- [ ] Логировать ошибки с достаточным контекстом
- [ ] Писать тесты для error scenarios

### Code Review Checklist:
- [ ] Нет `.unwrap()` или `.expect()` в production коде
- [ ] Все ошибки имеют понятные сообщения
- [ ] Критичные операции имеют fallback
- [ ] Ошибки правильно propagated вверх
- [ ] Метрики записываются для всех ошибок

## 🔧 Миграция существующего кода

### Step 1: Identify Panic Points
```bash
# Найти все unwrap/expect
rg "\.unwrap\(\)|\.expect\(" --type rust

# Найти все panic!
rg "panic!\(" --type rust
```

### Step 2: Replace with Proper Error Handling
```rust
// Before
let value = some_operation().unwrap();

// After
let value = some_operation()
    .context("Failed to perform operation")?;

// Or with fallback
let value = some_operation()
    .unwrap_or_else(|e| {
        warn!("Operation failed: {}, using default", e);
        default_value()
    });
```

### Step 3: Add Monitoring
```rust
// Wrap critical sections
let _timer = ErrorBoundary::new("critical_operation");
let result = critical_operation().await?;
```

## 📚 Best Practices

1. **Be Specific**: Конкретные error типы лучше чем generic `anyhow::Error`
2. **Add Context**: Всегда добавляйте контекст к ошибкам
3. **Log Once**: Логируйте ошибку только в одном месте (обычно где handle)
4. **Fail Fast**: Валидируйте input как можно раньше
5. **Graceful Degradation**: Всегда думайте о fallback сценарии
6. **Monitor**: Отслеживайте error rates и patterns
7. **Test Failures**: Тестируйте не только happy path

---

Эта стратегия обеспечивает надежную и предсказуемую обработку ошибок во всем проекте MAGRAY CLI.