//! Mock Services for DI System Testing
//! 
//! Collection of mock implementations for testing various scenarios:
//! - Simple mock services with configurable behavior
//! - Mock services with dependencies and complex initialization
//! - Mock services for performance testing and stress scenarios
//! - Mock services for error simulation and failure testing
//! - Mock services for concurrent access and thread safety testing

use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::thread;
use tokio::time::sleep;

use crate::{
    di::errors::{DIError, DIResult},
    types::MemoryRecord,
};

/// Mock сервис мониторинга для тестирования
#[derive(Debug)]
pub struct MockMonitoringService {
    pub metrics: Arc<Mutex<HashMap<String, f64>>>,
    pub operation_count: Arc<AtomicUsize>,
    pub error_count: Arc<AtomicUsize>,
    pub is_healthy: Arc<AtomicBool>,
    pub recorded_operations: Arc<Mutex<Vec<OperationRecord>>>,
}

#[derive(Debug, Clone)]
pub struct OperationRecord {
    pub name: String,
    pub duration: Duration,
    pub timestamp: Instant,
    pub success: bool,
}

impl MockMonitoringService {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            operation_count: Arc::new(AtomicUsize::new(0)),
            error_count: Arc::new(AtomicUsize::new(0)),
            is_healthy: Arc::new(AtomicBool::new(true)),
            recorded_operations: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub async fn record_operation(&self, name: &str, duration: Duration) {
        self.operation_count.fetch_add(1, Ordering::SeqCst);
        
        let operation = OperationRecord {
            name: name.to_string(),
            duration,
            timestamp: Instant::now(),
            success: true,
        };
        
        self.recorded_operations.lock().unwrap().push(operation);
        
        // Симулируем небольшую задержку для реализма
        sleep(Duration::from_millis(1)).await;
    }
    
    pub async fn record_error(&self, operation_name: &str, _error_message: &str) {
        self.error_count.fetch_add(1, Ordering::SeqCst);
        
        let operation = OperationRecord {
            name: operation_name.to_string(),
            duration: Duration::from_millis(0),
            timestamp: Instant::now(),
            success: false,
        };
        
        self.recorded_operations.lock().unwrap().push(operation);
    }
    
    pub async fn get_stats(&self) -> DIResult<MonitoringStats> {
        Ok(MonitoringStats {
            total_operations: self.operation_count.load(Ordering::SeqCst),
            error_count: self.error_count.load(Ordering::SeqCst),
            success_rate: self.calculate_success_rate(),
            average_duration: self.calculate_average_duration(),
        })
    }
    
    pub async fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }
    
    pub fn set_health_status(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::SeqCst);
    }
    
    pub fn get_operation_count(&self) -> usize {
        self.operation_count.load(Ordering::SeqCst)
    }
    
    pub fn get_error_count(&self) -> usize {
        self.error_count.load(Ordering::SeqCst)
    }
    
    fn calculate_success_rate(&self) -> f64 {
        let operations = self.recorded_operations.lock().unwrap();
        if operations.is_empty() {
            return 1.0;
        }
        
        let successful = operations.iter().filter(|op| op.success).count();
        successful as f64 / operations.len() as f64
    }
    
    fn calculate_average_duration(&self) -> Duration {
        let operations = self.recorded_operations.lock().unwrap();
        if operations.is_empty() {
            return Duration::from_secs(0);
        }
        
        let total: Duration = operations.iter().map(|op| op.duration).sum();
        total / operations.len() as u32
    }
}

#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub total_operations: usize,
    pub error_count: usize,
    pub success_rate: f64,
    pub average_duration: Duration,
}

/// Mock сервис кеша для тестирования
#[derive(Debug)]
pub struct MockCacheService {
    pub cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    pub hit_count: Arc<AtomicUsize>,
    pub miss_count: Arc<AtomicUsize>,
    pub max_size: usize,
    pub ttl: Duration,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: String,
    created_at: Instant,
    access_count: usize,
}

impl MockCacheService {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            hit_count: Arc::new(AtomicUsize::new(0)),
            miss_count: Arc::new(AtomicUsize::new(0)),
            max_size,
            ttl,
        }
    }
    
    pub fn create_cache_key(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("cache_key_{:x}", hasher.finish())
    }
    
    pub fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().unwrap();
        
        if let Some(entry) = cache.get(key) {
            // Проверяем TTL
            if entry.created_at.elapsed() < self.ttl {
                self.hit_count.fetch_add(1, Ordering::SeqCst);
                return Some(entry.value.clone());
            }
        }
        
        self.miss_count.fetch_add(1, Ordering::SeqCst);
        None
    }
    
    pub fn put(&self, key: String, value: String) {
        let mut cache = self.cache.write().unwrap();
        
        // Простая LRU eviction если превышен размер
        if cache.len() >= self.max_size {
            // Удаляем случайный элемент (упрощенная реализация)
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }
        
        let entry = CacheEntry {
            value,
            created_at: Instant::now(),
            access_count: 1,
        };
        
        cache.insert(key, entry);
    }
    
    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }
    
    pub fn get_hit_rate(&self) -> f64 {
        let hits = self.hit_count.load(Ordering::SeqCst);
        let misses = self.miss_count.load(Ordering::SeqCst);
        
        if hits + misses == 0 {
            return 1.0;
        }
        
        hits as f64 / (hits + misses) as f64
    }
    
    pub fn get_size(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

/// Mock Database сервис для тестирования
#[derive(Debug)]
pub struct MockDatabaseService {
    pub records: Arc<RwLock<HashMap<String, MemoryRecord>>>,
    pub operation_latency: Arc<Mutex<Duration>>,
    pub failure_rate: Arc<Mutex<f64>>,
    pub is_connected: Arc<AtomicBool>,
    pub query_count: Arc<AtomicUsize>,
}

impl MockDatabaseService {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            operation_latency: Arc::Mutex::new(Duration::from_millis(10)),
            failure_rate: Arc::Mutex::new(0.0),
            is_connected: Arc::new(AtomicBool::new(true)),
            query_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn with_latency(mut self, latency: Duration) -> Self {
        *self.operation_latency.lock().unwrap() = latency;
        self
    }
    
    pub fn with_failure_rate(mut self, failure_rate: f64) -> Self {
        *self.failure_rate.lock().unwrap() = failure_rate;
        self
    }
    
    pub async fn store_record(&self, record: MemoryRecord) -> DIResult<()> {
        self.simulate_operation().await?;
        
        let mut records = self.records.write().unwrap();
        records.insert(record.id.clone(), record);
        
        Ok(())
    }
    
    pub async fn get_record(&self, id: &str) -> DIResult<Option<MemoryRecord>> {
        self.simulate_operation().await?;
        
        let records = self.records.read().unwrap();
        Ok(records.get(id).cloned())
    }
    
    pub async fn search_records(&self, query: &str) -> DIResult<Vec<MemoryRecord>> {
        self.simulate_operation().await?;
        
        let records = self.records.read().unwrap();
        let results: Vec<MemoryRecord> = records
            .values()
            .filter(|record| record.content.contains(query))
            .cloned()
            .collect();
        
        Ok(results)
    }
    
    pub async fn delete_record(&self, id: &str) -> DIResult<bool> {
        self.simulate_operation().await?;
        
        let mut records = self.records.write().unwrap();
        Ok(records.remove(id).is_some())
    }
    
    pub fn get_record_count(&self) -> usize {
        self.records.read().unwrap().len()
    }
    
    pub fn get_query_count(&self) -> usize {
        self.query_count.load(Ordering::SeqCst)
    }
    
    pub fn set_connected(&self, connected: bool) {
        self.is_connected.store(connected, Ordering::SeqCst);
    }
    
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
    
    async fn simulate_operation(&self) -> DIResult<()> {
        self.query_count.fetch_add(1, Ordering::SeqCst);
        
        // Проверяем подключение
        if !self.is_connected() {
            return Err(DIError::DatabaseError {
                operation: "connection_check".to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "Database not connected"
                )),
            });
        }
        
        // Симулируем задержку
        let latency = *self.operation_latency.lock().unwrap();
        sleep(latency).await;
        
        // Симулируем случайные сбои
        let failure_rate = *self.failure_rate.lock().unwrap();
        if fastrand::f64() < failure_rate {
            return Err(DIError::DatabaseError {
                operation: "simulated_failure".to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Simulated database failure"
                )),
            });
        }
        
        Ok(())
    }
}

/// Mock AI сервис для тестирования embeddings
#[derive(Debug)]
pub struct MockEmbeddingService {
    pub embedding_dimension: usize,
    pub processing_time: Arc<Mutex<Duration>>,
    pub processed_count: Arc<AtomicUsize>,
    pub cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl MockEmbeddingService {
    pub fn new(embedding_dimension: usize) -> Self {
        Self {
            embedding_dimension,
            processing_time: Arc::new(Mutex::new(Duration::from_millis(50))),
            processed_count: Arc::new(AtomicUsize::new(0)),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn embed_text(&self, text: &str) -> DIResult<Vec<f32>> {
        // Проверяем кеш
        {
            let cache = self.cache.read().unwrap();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }
        
        // Симулируем время обработки
        let processing_time = *self.processing_time.lock().unwrap();
        sleep(processing_time).await;
        
        self.processed_count.fetch_add(1, Ordering::SeqCst);
        
        // Генерируем детерминированный embedding на основе текста
        let embedding = self.generate_embedding(text);
        
        // Кешируем результат
        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(text.to_string(), embedding.clone());
        }
        
        Ok(embedding)
    }
    
    pub async fn embed_batch(&self, texts: Vec<String>) -> DIResult<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        
        for text in texts {
            let embedding = self.embed_text(&text).await?;
            results.push(embedding);
        }
        
        Ok(results)
    }
    
    pub fn get_processed_count(&self) -> usize {
        self.processed_count.load(Ordering::SeqCst)
    }
    
    pub fn get_cache_size(&self) -> usize {
        self.cache.read().unwrap().len()
    }
    
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }
    
    pub fn set_processing_time(&self, duration: Duration) {
        *self.processing_time.lock().unwrap() = duration;
    }
    
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Генерируем детерминированный embedding на основе хеша
        (0..self.embedding_dimension)
            .map(|i| {
                let seed = hash.wrapping_add(i as u64);
                ((seed % 1000) as f32 / 1000.0) * 2.0 - 1.0 // [-1.0, 1.0]
            })
            .collect()
    }
}

/// Mock сервис для тестирования производительности под нагрузкой
#[derive(Debug)]
pub struct MockStressTestService {
    pub cpu_intensive_operations: Arc<AtomicUsize>,
    pub memory_allocations: Arc<AtomicUsize>,
    pub concurrent_operations: Arc<AtomicUsize>,
    pub max_concurrent_limit: usize,
    pub operation_data: Arc<Mutex<Vec<u8>>>,
}

impl MockStressTestService {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            cpu_intensive_operations: Arc::new(AtomicUsize::new(0)),
            memory_allocations: Arc::new(AtomicUsize::new(0)),
            concurrent_operations: Arc::new(AtomicUsize::new(0)),
            max_concurrent_limit: max_concurrent,
            operation_data: Arc::new(Mutex::new(Vec::with_capacity(1024 * 1024))), // 1MB buffer
        }
    }
    
    pub async fn cpu_intensive_task(&self, iterations: usize) -> DIResult<u64> {
        let current_concurrent = self.concurrent_operations.fetch_add(1, Ordering::SeqCst);
        
        if current_concurrent >= self.max_concurrent_limit {
            self.concurrent_operations.fetch_sub(1, Ordering::SeqCst);
            return Err(DIError::ServiceError {
                service_name: "MockStressTestService".to_string(),
                operation: "cpu_intensive_task".to_string(),
                message: "Too many concurrent operations".to_string(),
            });
        }
        
        self.cpu_intensive_operations.fetch_add(1, Ordering::SeqCst);
        
        // Симулируем CPU-интенсивную работу
        let mut result = 0u64;
        for i in 0..iterations {
            result = result.wrapping_add((i as u64).wrapping_mul(17));
            if i % 1000 == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        self.concurrent_operations.fetch_sub(1, Ordering::SeqCst);
        Ok(result)
    }
    
    pub async fn memory_intensive_task(&self, allocation_size_mb: usize) -> DIResult<usize> {
        self.memory_allocations.fetch_add(1, Ordering::SeqCst);
        
        // Выделяем и заполняем память
        let allocation_size = allocation_size_mb * 1024 * 1024;
        let mut data = vec![0u8; allocation_size];
        
        // Заполняем данными для предотвращения оптимизации компилятора
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        
        // Симулируем работу с данными
        let checksum: u64 = data.iter().map(|&b| b as u64).sum();
        
        // Сохраняем часть данных
        {
            let mut operation_data = self.operation_data.lock().unwrap();
            operation_data.extend_from_slice(&data[..std::cmp::min(data.len(), 1024)]);
        }
        
        Ok(checksum as usize)
    }
    
    pub fn get_stats(&self) -> StressTestStats {
        StressTestStats {
            cpu_operations: self.cpu_intensive_operations.load(Ordering::SeqCst),
            memory_allocations: self.memory_allocations.load(Ordering::SeqCst),
            current_concurrent: self.concurrent_operations.load(Ordering::SeqCst),
            operation_data_size: self.operation_data.lock().unwrap().len(),
        }
    }
    
    pub fn reset_stats(&self) {
        self.cpu_intensive_operations.store(0, Ordering::SeqCst);
        self.memory_allocations.store(0, Ordering::SeqCst);
        self.concurrent_operations.store(0, Ordering::SeqCst);
        self.operation_data.lock().unwrap().clear();
    }
}

#[derive(Debug, Clone)]
pub struct StressTestStats {
    pub cpu_operations: usize,
    pub memory_allocations: usize,
    pub current_concurrent: usize,
    pub operation_data_size: usize,
}

/// Mock сервис для тестирования восстановления после сбоев
#[derive(Debug)]
pub struct MockFailureRecoveryService {
    pub failure_count: Arc<AtomicUsize>,
    pub recovery_count: Arc<AtomicUsize>,
    pub is_healthy: Arc<AtomicBool>,
    pub failure_threshold: usize,
    pub recovery_delay: Duration,
    pub last_failure: Arc<Mutex<Option<Instant>>>,
}

impl MockFailureRecoveryService {
    pub fn new(failure_threshold: usize, recovery_delay: Duration) -> Self {
        Self {
            failure_count: Arc::new(AtomicUsize::new(0)),
            recovery_count: Arc::new(AtomicUsize::new(0)),
            is_healthy: Arc::new(AtomicBool::new(true)),
            failure_threshold,
            recovery_delay,
            last_failure: Arc::new(Mutex::new(None)),
        }
    }
    
    pub async fn attempt_operation(&self, operation_name: &str) -> DIResult<String> {
        // Проверяем нужно ли восстановление
        self.check_recovery().await;
        
        if !self.is_healthy.load(Ordering::SeqCst) {
            let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
            *self.last_failure.lock().unwrap() = Some(Instant::now());
            
            return Err(DIError::ServiceError {
                service_name: "MockFailureRecoveryService".to_string(),
                operation: operation_name.to_string(),
                message: format!("Service unhealthy, failure #{}", failure_count),
            });
        }
        
        Ok(format!("Operation '{}' completed successfully", operation_name))
    }
    
    pub fn trigger_failure(&self) {
        self.is_healthy.store(false, Ordering::SeqCst);
        *self.last_failure.lock().unwrap() = Some(Instant::now());
    }
    
    pub async fn force_recovery(&self) {
        sleep(self.recovery_delay).await;
        self.is_healthy.store(true, Ordering::SeqCst);
        self.recovery_count.fetch_add(1, Ordering::SeqCst);
    }
    
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }
    
    pub fn get_failure_count(&self) -> usize {
        self.failure_count.load(Ordering::SeqCst)
    }
    
    pub fn get_recovery_count(&self) -> usize {
        self.recovery_count.load(Ordering::SeqCst)
    }
    
    async fn check_recovery(&self) {
        if self.is_healthy.load(Ordering::SeqCst) {
            return;
        }
        
        let last_failure = self.last_failure.lock().unwrap();
        if let Some(failure_time) = *last_failure {
            if failure_time.elapsed() >= self.recovery_delay {
                drop(last_failure); // Release lock before async operation
                self.force_recovery().await;
            }
        }
    }
}