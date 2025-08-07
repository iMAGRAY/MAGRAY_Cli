//! Test Fixtures and Utilities for DI System Testing
//! 
//! Comprehensive test fixtures and utilities including:
//! - Container factory methods for different test scenarios
//! - Configuration builders with preset test data
//! - Test data generators for various entities
//! - Test assertion helpers and matchers
//! - Resource cleanup and lifecycle management
//! - Performance measurement utilities

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::{
    di::{
        unified_container::UnifiedDIContainer,
        unified_config::UnifiedDIConfiguration,
        errors::{DIError, DIResult},
        traits::ServiceLifetime,
    },
    services::{
        unified_factory::{UnifiedServiceFactory, FactoryPreset},
        monitoring_service::MonitoringService,
        cache_service::CacheService,
    },
    types::MemoryRecord,
};

/// Builder для создания тестовых контейнеров с различными конфигурациями
pub struct TestContainerBuilder {
    config: UnifiedDIConfiguration,
    preset: FactoryPreset,
    custom_services: Vec<Box<dyn Fn(&mut UnifiedDIContainer) -> DIResult<()> + Send + Sync>>,
    cleanup_callbacks: Vec<Box<dyn Fn() -> DIResult<()> + Send + Sync>>,
}

impl TestContainerBuilder {
    pub fn new() -> Self {
        Self {
            config: UnifiedDIConfiguration::test_config().unwrap(),
            preset: FactoryPreset::Test,
            custom_services: Vec::new(),
            cleanup_callbacks: Vec::new(),
        }
    }
    
    pub fn with_config(mut self, config: UnifiedDIConfiguration) -> Self {
        self.config = config;
        self
    }
    
    pub fn with_preset(mut self, preset: FactoryPreset) -> Self {
        self.preset = preset;
        self
    }
    
    pub fn with_minimal_config(mut self) -> Self {
        self.config = UnifiedDIConfiguration::minimal_config().unwrap();
        self.preset = FactoryPreset::Minimal;
        self
    }
    
    pub fn with_production_config(mut self) -> Self {
        self.config = UnifiedDIConfiguration::production_config().unwrap();
        self.preset = FactoryPreset::Production;
        self
    }
    
    pub fn with_custom_service<T, F>(mut self, name: &str, lifetime: ServiceLifetime, factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> DIResult<Arc<T>> + Send + Sync + 'static,
    {
        let service_name = name.to_string();
        let service_factory = Box::new(move |container: &mut UnifiedDIContainer| -> DIResult<()> {
            match lifetime {
                ServiceLifetime::Singleton => container.register_singleton::<T, _>(&service_name, &factory),
                ServiceLifetime::Transient => container.register_transient::<T, _>(&service_name, &factory),
                ServiceLifetime::Scoped => container.register_scoped::<T, _>(&service_name, &factory),
            }
        });
        self.custom_services.push(service_factory);
        self
    }
    
    pub fn with_cleanup_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> DIResult<()> + Send + Sync + 'static,
    {
        self.cleanup_callbacks.push(Box::new(callback));
        self
    }
    
    pub async fn build(self) -> DIResult<TestContainer> {
        let factory = UnifiedServiceFactory::with_preset(self.preset)?;
        let mut container = factory.build_container(&self.config).await?;
        
        // Регистрируем кастомные сервисы
        for service_factory in self.custom_services {
            service_factory(&mut container)?;
        }
        
        Ok(TestContainer {
            container: Arc::new(container),
            cleanup_callbacks: self.cleanup_callbacks,
        })
    }
}

/// Wrapper для тестового контейнера с автоматической очисткой
pub struct TestContainer {
    pub container: Arc<UnifiedDIContainer>,
    cleanup_callbacks: Vec<Box<dyn Fn() -> DIResult<()> + Send + Sync>>,
}

impl TestContainer {
    pub async fn resolve<T>(&self) -> DIResult<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.container.resolve::<T>().await
    }
    
    pub async fn resolve_named<T>(&self, name: &str) -> DIResult<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.container.resolve_named::<T>(name).await
    }
    
    pub async fn get_statistics(&self) -> DIResult<crate::di::unified_container::ContainerStatistics> {
        self.container.get_statistics().await
    }
    
    pub async fn is_healthy(&self) -> bool {
        self.container.is_healthy().await
    }
    
    pub async fn shutdown(self) -> DIResult<()> {
        // Выполняем cleanup callbacks
        for callback in &self.cleanup_callbacks {
            if let Err(e) = callback() {
                eprintln!("Cleanup callback failed: {}", e);
            }
        }
        
        // Shutdown контейнера
        self.container.shutdown().await
    }
}

/// Factory для создания стандартных тестовых контейнеров
pub struct TestContainerFactory;

impl TestContainerFactory {
    /// Создает базовый тестовый контейнер с минимальными сервисами
    pub async fn create_basic() -> DIResult<TestContainer> {
        TestContainerBuilder::new()
            .with_minimal_config()
            .build().await
    }
    
    /// Создает полнофункциональный тестовый контейнер
    pub async fn create_full_featured() -> DIResult<TestContainer> {
        TestContainerBuilder::new()
            .with_config(UnifiedDIConfiguration::development_config()?)
            .with_preset(FactoryPreset::Development)
            .build().await
    }
    
    /// Создает высокопроизводительный тестовый контейнер
    pub async fn create_performance() -> DIResult<TestContainer> {
        let mut config = UnifiedDIConfiguration::production_config()?;
        config.max_services = 1000;
        config.enable_advanced_features = true;
        
        TestContainerBuilder::new()
            .with_config(config)
            .with_preset(FactoryPreset::Production)
            .build().await
    }
    
    /// Создает контейнер для тестирования конкурентности
    pub async fn create_concurrent_test() -> DIResult<TestContainer> {
        let mut config = UnifiedDIConfiguration::test_config()?;
        config.enable_thread_safety = true;
        config.max_concurrent_operations = 100;
        
        TestContainerBuilder::new()
            .with_config(config)
            .with_custom_service::<TestConcurrentService, _>(
                "ConcurrentService",
                ServiceLifetime::Singleton,
                || Ok(Arc::new(TestConcurrentService::new()))
            )
            .build().await
    }
}

/// Generator для создания тестовых данных
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Создает тестовый MemoryRecord
    pub fn create_memory_record(id: &str, content: &str) -> MemoryRecord {
        MemoryRecord {
            id: if id.is_empty() { uuid::Uuid::new_v4().to_string() } else { id.to_string() },
            content: content.to_string(),
            embedding: Self::generate_random_embedding(384),
            metadata: Self::create_test_metadata(id),
            created_at: chrono::Utc::now(),
            access_count: 0,
        }
    }
    
    /// Создает множество тестовых записей
    pub fn create_memory_records(count: usize, prefix: &str) -> Vec<MemoryRecord> {
        (0..count)
            .map(|i| Self::create_memory_record(
                &format!("{}_{}", prefix, i),
                &format!("Test content for record {}", i)
            ))
            .collect()
    }
    
    /// Генерирует случайный embedding вектор
    pub fn generate_random_embedding(dimension: usize) -> Vec<f32> {
        (0..dimension)
            .map(|i| (i as f32 * 0.1) % 1.0) // Детерминированный для тестов
            .collect()
    }
    
    /// Создает тестовые метаданные
    pub fn create_test_metadata(id: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("test_id".to_string(), id.to_string());
        metadata.insert("created_by".to_string(), "test_suite".to_string());
        metadata.insert("environment".to_string(), "test".to_string());
        metadata
    }
    
    /// Создает конфигурации для различных тестовых сценариев
    pub fn create_test_configurations() -> Vec<(&'static str, UnifiedDIConfiguration)> {
        vec![
            ("minimal", UnifiedDIConfiguration::minimal_config().unwrap()),
            ("test", UnifiedDIConfiguration::test_config().unwrap()),
            ("development", UnifiedDIConfiguration::development_config().unwrap()),
            ("production", UnifiedDIConfiguration::production_config().unwrap()),
        ]
    }
}

/// Утилиты для измерения производительности в тестах
pub struct PerformanceMeasurement {
    start_time: Instant,
    operation_name: String,
    measurements: Arc<Mutex<Vec<Duration>>>,
}

impl PerformanceMeasurement {
    pub fn new(operation_name: &str) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.to_string(),
            measurements: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn measure<F, R>(&self, operation: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = operation();
        let duration = start.elapsed();
        
        self.measurements.lock().unwrap().push(duration);
        result
    }
    
    pub async fn measure_async<F, Fut, R>(&self, operation: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = operation().await;
        let duration = start.elapsed();
        
        self.measurements.lock().unwrap().push(duration);
        result
    }
    
    pub fn get_average_duration(&self) -> Duration {
        let measurements = self.measurements.lock().unwrap();
        if measurements.is_empty() {
            return Duration::from_secs(0);
        }
        
        let total: Duration = measurements.iter().sum();
        total / measurements.len() as u32
    }
    
    pub fn get_min_duration(&self) -> Duration {
        self.measurements.lock().unwrap()
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::from_secs(0))
    }
    
    pub fn get_max_duration(&self) -> Duration {
        self.measurements.lock().unwrap()
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::from_secs(0))
    }
    
    pub fn print_statistics(&self) {
        let measurements = self.measurements.lock().unwrap();
        if measurements.is_empty() {
            println!("No measurements for operation: {}", self.operation_name);
            return;
        }
        
        println!("Performance statistics for '{}' ({} samples):", 
            self.operation_name, measurements.len());
        println!("  Average: {:?}", self.get_average_duration());
        println!("  Min: {:?}", self.get_min_duration());
        println!("  Max: {:?}", self.get_max_duration());
    }
}

/// Assertion helpers для тестирования DI системы
pub struct DITestAsserts;

impl DITestAsserts {
    /// Проверяет что контейнер здоров
    pub async fn assert_container_healthy(container: &UnifiedDIContainer) {
        assert!(container.is_healthy().await, "Container should be healthy");
    }
    
    /// Проверяет что сервис разрешается без ошибок
    pub async fn assert_service_resolves<T>(container: &UnifiedDIContainer) 
    where
        T: Send + Sync + 'static,
    {
        let result = container.resolve::<T>().await;
        assert!(result.is_ok(), "Service should resolve successfully: {:?}", result.err());
    }
    
    /// Проверяет что именованный сервис разрешается без ошибок
    pub async fn assert_named_service_resolves<T>(container: &UnifiedDIContainer, name: &str)
    where
        T: Send + Sync + 'static,
    {
        let result = container.resolve_named::<T>(name).await;
        assert!(result.is_ok(), "Named service '{}' should resolve successfully: {:?}", name, result.err());
    }
    
    /// Проверяет что операция завершается в разумное время
    pub async fn assert_operation_completes_within<F, Fut>(
        operation: F, 
        max_duration: Duration,
        operation_name: &str
    ) where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let result = timeout(max_duration, operation()).await;
        assert!(result.is_ok(), "Operation '{}' should complete within {:?}", operation_name, max_duration);
    }
    
    /// Проверяет статистики контейнера
    pub async fn assert_container_statistics(
        container: &UnifiedDIContainer,
        min_services: usize,
        min_instances: usize,
    ) -> DIResult<()> {
        let stats = container.get_statistics().await?;
        assert!(stats.registered_services >= min_services, 
            "Container should have at least {} registered services, has {}", 
            min_services, stats.registered_services);
        assert!(stats.active_instances >= min_instances,
            "Container should have at least {} active instances, has {}",
            min_instances, stats.active_instances);
        Ok(())
    }
}

/// Resource tracker для мониторинга использования ресурсов в тестах
pub struct TestResourceTracker {
    memory_usage: AtomicUsize,
    active_containers: AtomicUsize,
    active_services: AtomicUsize,
    is_tracking: AtomicBool,
}

impl TestResourceTracker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            memory_usage: AtomicUsize::new(0),
            active_containers: AtomicUsize::new(0),
            active_services: AtomicUsize::new(0),
            is_tracking: AtomicBool::new(true),
        })
    }
    
    pub fn track_container_created(&self) {
        if self.is_tracking.load(Ordering::SeqCst) {
            self.active_containers.fetch_add(1, Ordering::SeqCst);
        }
    }
    
    pub fn track_container_destroyed(&self) {
        if self.is_tracking.load(Ordering::SeqCst) {
            self.active_containers.fetch_sub(1, Ordering::SeqCst);
        }
    }
    
    pub fn track_service_created(&self, estimated_memory: usize) {
        if self.is_tracking.load(Ordering::SeqCst) {
            self.active_services.fetch_add(1, Ordering::SeqCst);
            self.memory_usage.fetch_add(estimated_memory, Ordering::SeqCst);
        }
    }
    
    pub fn track_service_destroyed(&self, estimated_memory: usize) {
        if self.is_tracking.load(Ordering::SeqCst) {
            self.active_services.fetch_sub(1, Ordering::SeqCst);
            self.memory_usage.fetch_sub(estimated_memory, Ordering::SeqCst);
        }
    }
    
    pub fn get_active_containers(&self) -> usize {
        self.active_containers.load(Ordering::SeqCst)
    }
    
    pub fn get_active_services(&self) -> usize {
        self.active_services.load(Ordering::SeqCst)
    }
    
    pub fn get_estimated_memory_usage(&self) -> usize {
        self.memory_usage.load(Ordering::SeqCst)
    }
    
    pub fn stop_tracking(&self) {
        self.is_tracking.store(false, Ordering::SeqCst);
    }
    
    pub fn print_summary(&self) {
        println!("Test Resource Summary:");
        println!("  Active Containers: {}", self.get_active_containers());
        println!("  Active Services: {}", self.get_active_services());
        println!("  Estimated Memory Usage: {} bytes", self.get_estimated_memory_usage());
    }
}

// Mock services для тестирования

/// Простой тестовый сервис
#[derive(Debug, Clone)]
pub struct TestSimpleService {
    pub id: String,
    pub created_at: Instant,
    pub call_count: Arc<AtomicUsize>,
}

impl TestSimpleService {
    pub fn new(id: String) -> Self {
        Self {
            id,
            created_at: Instant::now(),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn do_work(&self) -> String {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        format!("Work done by {}", self.id)
    }
    
    pub fn get_call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

/// Сервис для тестирования конкурентности
#[derive(Debug)]
pub struct TestConcurrentService {
    pub counter: Arc<AtomicUsize>,
    pub operations: Arc<Mutex<Vec<String>>>,
}

impl TestConcurrentService {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(AtomicUsize::new(0)),
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn increment(&self) -> usize {
        let new_value = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        self.operations.lock().unwrap().push(format!("increment -> {}", new_value));
        new_value
    }
    
    pub fn get_counter(&self) -> usize {
        self.counter.load(Ordering::SeqCst)
    }
    
    pub fn get_operations_count(&self) -> usize {
        self.operations.lock().unwrap().len()
    }
}

/// Сервис с зависимостями для тестирования DI
#[derive(Debug)]
pub struct TestDependentService {
    pub dependency: Arc<TestSimpleService>,
    pub instance_id: String,
}

impl TestDependentService {
    pub fn new(dependency: Arc<TestSimpleService>) -> Self {
        Self {
            dependency,
            instance_id: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    pub fn perform_complex_operation(&self) -> String {
        let work_result = self.dependency.do_work();
        format!("Complex operation {} completed with result: {}", 
            self.instance_id, work_result)
    }
    
    pub fn get_dependency_id(&self) -> &str {
        &self.dependency.id
    }
}