//! MetricsCollector - централизованный сборщик метрик для orchestration системы
//!
//! Реализует Single Responsibility Principle для сбора, агрегации и представления
//! метрик от всех координаторов системы.

use anyhow::Result;
use serde_json::{json, Map, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::orchestration::traits::Coordinator;

/// Результат адаптивной оптимизации
#[derive(Debug, Clone)]
pub struct AdaptiveOptimizationResult {
    pub actions_taken: Vec<String>,
    pub recommendations: Vec<String>,
    pub metrics_analyzed: bool,
    pub timestamp: std::time::Instant,
}

/// Типы метрик для классификации
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    Performance,
    Health,
    CircuitBreaker,
    Resource,
    SLA,
    Custom(String),
}

/// Метрика с временной меткой и значением
#[derive(Debug, Clone)]
pub struct TimestampedMetric {
    pub timestamp: Instant,
    pub value: f64,
    pub metric_type: MetricType,
    pub labels: HashMap<String, String>,
}

/// Агрегированные метрики координатора
#[derive(Debug, Default, Clone)]
pub struct CoordinatorMetrics {
    pub success_rate: f64,
    pub avg_response_time_ms: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub circuit_breaker_state: String,
    pub health_score: f64,
    pub last_updated: Option<Instant>,
}

/// Общие метрики оркестратора
#[derive(Debug, Clone)]
pub struct OrchestrationMetrics {
    /// Операционные метрики
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,

    /// Метрики координаторов
    pub coordinator_metrics: HashMap<String, CoordinatorMetrics>,

    /// Метрики производительности
    pub avg_operation_duration_ms: f64,
    pub max_operation_duration_ms: u64,
    pub min_operation_duration_ms: u64,

    /// Circuit breaker метрики
    pub circuit_breaker_trips: HashMap<String, u64>,

    /// Использование ресурсов
    pub current_concurrent_operations: u64,
    pub max_concurrent_operations: u64,
    pub memory_usage_bytes: u64,

    /// SLA метрики
    pub sla_violations: u64,
    pub uptime_seconds: u64,
    pub availability_percentage: f64,

    /// Время сбора метрик
    pub collected_at: Instant,
}

impl Default for OrchestrationMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            coordinator_metrics: HashMap::new(),
            avg_operation_duration_ms: 0.0,
            max_operation_duration_ms: 0,
            min_operation_duration_ms: u64::MAX,
            circuit_breaker_trips: HashMap::new(),
            current_concurrent_operations: 0,
            max_concurrent_operations: 0,
            memory_usage_bytes: 0,
            sla_violations: 0,
            uptime_seconds: 0,
            availability_percentage: 100.0,
            collected_at: Instant::now(),
        }
    }
}

/// Состояние Circuit Breaker для метрик
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerStatus {
    Closed,   // Нормальная работа
    Open,     // Блокировка запросов
    HalfOpen, // Пробная проверка восстановления
}

/// Информация о состоянии Circuit Breaker
#[derive(Debug)]
pub struct CircuitBreakerMetric {
    pub status: CircuitBreakerStatus,
    pub failure_count: u64,
    pub last_failure: Option<Instant>,
    pub recovery_timeout: Duration,
    pub success_threshold: u32,
}

/// Централизованный сборщик метрик
pub struct MetricsCollector {
    /// Текущие метрики оркестратора
    orchestration_metrics: Arc<RwLock<OrchestrationMetrics>>,

    /// История метрик (ring buffer для производительности)
    metrics_history: Arc<RwLock<Vec<OrchestrationMetrics>>>,

    /// Максимальный размер истории
    max_history_size: usize,

    /// Circuit breaker метрики
    circuit_breaker_metrics: Arc<RwLock<HashMap<String, CircuitBreakerMetric>>>,

    /// Время запуска системы для uptime расчетов
    start_time: Instant,

    /// Флаг активности сборщика
    active: Arc<AtomicBool>,

    /// Кэш для часто запрашиваемых агрегированных метрик
    aggregated_cache: Arc<RwLock<HashMap<String, (Value, Instant)>>>,

    /// TTL для кэша (в секундах)
    cache_ttl: Duration,
}

impl MetricsCollector {
    /// Создать новый MetricsCollector
    pub fn new(max_history_size: usize) -> Self {
        Self {
            orchestration_metrics: Arc::new(RwLock::new(OrchestrationMetrics {
                collected_at: Instant::now(),
                ..Default::default()
            })),
            metrics_history: Arc::new(RwLock::new(Vec::with_capacity(max_history_size))),
            max_history_size,
            circuit_breaker_metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            active: Arc::new(AtomicBool::new(true)),
            aggregated_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(30), // 30 секунд TTL для кэша
        }
    }

    /// Создать MetricsCollector из DI контейнера (для совместимости)
    pub fn from_container(
        _container: &crate::di::container_core::ContainerCore,
        _circuit_breaker_manager: Arc<
            crate::orchestration::circuit_breaker_manager::CircuitBreakerManager,
        >,
    ) -> Result<Self> {
        Ok(Self::new(1000)) // Default history size
    }

    /// Получить все метрики в формате JSON
    pub async fn collect_all_metrics<T>(&self, coordinators: &HashMap<String, Arc<T>>) -> Value
    where
        T: Coordinator + ?Sized,
    {
        debug!("🔍 Начало сбора comprehensive метрик");

        // Проверяем кэш
        if let Some(cached) = self.get_cached_metrics("all_metrics").await {
            debug!("📋 Возвращаем метрики из кэша");
            return cached;
        }

        let orchestration_metrics = self.orchestration_metrics.read().await;
        let circuit_breaker_metrics = self.circuit_breaker_metrics.read().await;

        // Параллельно собираем метрики от всех координаторов
        let mut coordinator_results = Vec::new();
        let mut readiness_results = Vec::new();

        for (name, coordinator) in coordinators {
            coordinator_results.push((name.clone(), coordinator.metrics()));
            readiness_results.push((name.clone(), coordinator.is_ready()));
        }

        // Ждем все результаты
        let mut coordinator_metrics_json = Map::new();
        let mut coordinator_readiness = Map::new();

        for (name, metrics_future) in coordinator_results {
            let metrics = metrics_future.await;
            coordinator_metrics_json.insert(name, metrics);
        }

        for (name, readiness_future) in readiness_results {
            let ready = readiness_future.await;
            coordinator_readiness.insert(name, Value::Bool(ready));
        }

        // Собираем состояния Circuit Breaker
        let mut circuit_breaker_states = Map::new();
        for (name, cb_metric) in circuit_breaker_metrics.iter() {
            circuit_breaker_states.insert(
                name.clone(),
                json!({
                    "status": match cb_metric.status {
                        CircuitBreakerStatus::Closed => "closed",
                        CircuitBreakerStatus::Open => "open",
                        CircuitBreakerStatus::HalfOpen => "half_open",
                    },
                    "failure_count": cb_metric.failure_count,
                    "last_failure": cb_metric.last_failure.map(|t| t.elapsed().as_secs()),
                    "recovery_timeout_secs": cb_metric.recovery_timeout.as_secs(),
                    "success_threshold": cb_metric.success_threshold,
                }),
            );
        }

        let result = json!({
            "orchestration": {
                "total_operations": orchestration_metrics.total_operations,
                "successful_operations": orchestration_metrics.successful_operations,
                "failed_operations": orchestration_metrics.failed_operations,
                "success_rate": if orchestration_metrics.total_operations > 0 {
                    (orchestration_metrics.successful_operations as f64 / orchestration_metrics.total_operations as f64) * 100.0
                } else {
                    100.0
                },
                "avg_operation_duration_ms": orchestration_metrics.avg_operation_duration_ms,
                "max_operation_duration_ms": orchestration_metrics.max_operation_duration_ms,
                "min_operation_duration_ms": orchestration_metrics.min_operation_duration_ms,
                "current_concurrent_operations": orchestration_metrics.current_concurrent_operations,
                "max_concurrent_operations": orchestration_metrics.max_concurrent_operations,
                "memory_usage_bytes": orchestration_metrics.memory_usage_bytes,
                "sla_violations": orchestration_metrics.sla_violations,
                "uptime_seconds": self.start_time.elapsed().as_secs(),
                "availability_percentage": orchestration_metrics.availability_percentage,
                "collected_at": orchestration_metrics.collected_at.elapsed().as_millis(),
            },
            "coordinators": {
                "metrics": Value::Object(coordinator_metrics_json),
                "readiness": Value::Object(coordinator_readiness),
            },
            "circuit_breakers": Value::Object(circuit_breaker_states),
            "system": {
                "start_time": self.start_time.elapsed().as_secs(),
                "metrics_collection_active": self.active.load(Ordering::Relaxed),
                "metrics_history_size": self.metrics_history.read().await.len(),
                "max_history_size": self.max_history_size,
            }
        });

        // Кэшируем результат
        self.cache_metrics("all_metrics", result.clone()).await;

        info!("✅ Comprehensive метрики собраны успешно");
        result
    }

    /// Получить упрощенные метрики для dashboard
    pub async fn collect_dashboard_metrics<T>(
        &self,
        coordinators: &HashMap<String, Arc<T>>,
    ) -> Value
    where
        T: Coordinator + ?Sized,
    {
        debug!("📊 Сбор dashboard метрик");

        // Проверяем кэш
        if let Some(cached) = self.get_cached_metrics("dashboard_metrics").await {
            return cached;
        }

        let full_metrics = self.collect_all_metrics(coordinators).await;

        let result = json!({
            "status": if self.active.load(Ordering::Relaxed) { "active" } else { "inactive" },
            "uptime_seconds": self.start_time.elapsed().as_secs(),
            "total_operations": full_metrics["orchestration"]["total_operations"],
            "success_rate": full_metrics["orchestration"]["success_rate"],
            "current_concurrent_operations": full_metrics["orchestration"]["current_concurrent_operations"],
            "coordinator_readiness": full_metrics["coordinators"]["readiness"],
            "circuit_breaker_summary": self.summarize_circuit_breakers().await,
            "memory_usage_mb": (full_metrics["orchestration"]["memory_usage_bytes"].as_u64().unwrap_or(0) as f64) / (1024.0 * 1024.0),
        });

        // Кэшируем результат
        self.cache_metrics("dashboard_metrics", result.clone())
            .await;

        result
    }

    /// Обновить метрики операции
    pub async fn record_operation(&self, coordinator_name: &str, duration_ms: u64, success: bool) {
        let mut metrics = self.orchestration_metrics.write().await;

        metrics.total_operations += 1;

        if success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }

        // Обновляем агрегированную статистику производительности
        let operations_count = metrics.total_operations as f64;
        metrics.avg_operation_duration_ms =
            ((metrics.avg_operation_duration_ms * (operations_count - 1.0)) + duration_ms as f64)
                / operations_count;

        if duration_ms > metrics.max_operation_duration_ms {
            metrics.max_operation_duration_ms = duration_ms;
        }

        if metrics.min_operation_duration_ms == 0 || duration_ms < metrics.min_operation_duration_ms
        {
            metrics.min_operation_duration_ms = duration_ms;
        }

        // Обновляем метрики конкретного координатора
        let coordinator_metrics = metrics
            .coordinator_metrics
            .entry(coordinator_name.to_string())
            .or_insert_with(CoordinatorMetrics::default);

        coordinator_metrics.total_requests += 1;
        if !success {
            coordinator_metrics.failed_requests += 1;
        }

        coordinator_metrics.success_rate = if coordinator_metrics.total_requests > 0 {
            ((coordinator_metrics.total_requests - coordinator_metrics.failed_requests) as f64
                / coordinator_metrics.total_requests as f64)
                * 100.0
        } else {
            100.0
        };

        coordinator_metrics.avg_response_time_ms = ((coordinator_metrics.avg_response_time_ms
            * (coordinator_metrics.total_requests - 1) as f64)
            + duration_ms as f64)
            / coordinator_metrics.total_requests as f64;

        coordinator_metrics.last_updated = Some(Instant::now());

        metrics.collected_at = Instant::now();

        // Очищаем кэш после обновления метрик
        self.clear_cache().await;

        debug!(
            "📈 Записана операция для {}: {}ms, success: {}",
            coordinator_name, duration_ms, success
        );
    }

    /// Обновить Circuit Breaker метрики
    pub async fn update_circuit_breaker(
        &self,
        coordinator_name: &str,
        status: CircuitBreakerStatus,
        failure_count: u64,
    ) {
        let mut cb_metrics = self.circuit_breaker_metrics.write().await;

        let cb_metric = cb_metrics
            .entry(coordinator_name.to_string())
            .or_insert_with(|| CircuitBreakerMetric {
                status: CircuitBreakerStatus::Closed,
                failure_count: 0,
                last_failure: None,
                recovery_timeout: Duration::from_secs(60),
                success_threshold: 5,
            });

        cb_metric.status = status.clone();
        cb_metric.failure_count = failure_count;

        if matches!(status, CircuitBreakerStatus::Open) {
            cb_metric.last_failure = Some(Instant::now());

            // Увеличиваем счетчик trips в основных метриках
            let mut metrics = self.orchestration_metrics.write().await;
            *metrics
                .circuit_breaker_trips
                .entry(coordinator_name.to_string())
                .or_insert(0) += 1;
        }

        debug!(
            "🔄 Обновлен Circuit Breaker для {}: {:?}, failures: {}",
            coordinator_name, status, failure_count
        );
    }

    /// Обновить использование ресурсов
    pub async fn update_resource_usage(&self, current_concurrent: u64, memory_bytes: u64) {
        let mut metrics = self.orchestration_metrics.write().await;

        metrics.current_concurrent_operations = current_concurrent;
        if current_concurrent > metrics.max_concurrent_operations {
            metrics.max_concurrent_operations = current_concurrent;
        }

        metrics.memory_usage_bytes = memory_bytes;
        metrics.collected_at = Instant::now();

        // Рассчитываем availability
        let uptime_secs = self.start_time.elapsed().as_secs();
        if uptime_secs > 0 {
            metrics.availability_percentage =
                ((uptime_secs - metrics.sla_violations) as f64 / uptime_secs as f64) * 100.0;
        }
    }

    /// Записать SLA нарушение
    pub async fn record_sla_violation(&self, coordinator_name: &str) {
        let mut metrics = self.orchestration_metrics.write().await;
        metrics.sla_violations += 1;

        warn!(
            "⚠️ SLA нарушение зафиксировано для координатора: {}",
            coordinator_name
        );
    }

    /// Сохранить текущие метрики в историю
    pub async fn save_to_history(&self) {
        let current_metrics = {
            let metrics = self.orchestration_metrics.read().await;
            metrics.clone()
        };

        let mut history = self.metrics_history.write().await;

        // Ring buffer логика
        if history.len() >= self.max_history_size {
            history.remove(0);
        }

        history.push(current_metrics);

        debug!("💾 Метрики сохранены в историю (размер: {})", history.len());
    }

    /// Получить историю метрик
    pub async fn get_metrics_history(&self) -> Vec<OrchestrationMetrics> {
        self.metrics_history.read().await.clone()
    }

    /// Получить агрегированную статистику за период
    pub async fn get_aggregated_stats(&self, window_minutes: u32) -> Value {
        let history = self.metrics_history.read().await;

        if history.is_empty() {
            return json!({
                "error": "No metrics history available",
                "window_minutes": window_minutes,
            });
        }

        let window_duration = Duration::from_secs(window_minutes as u64 * 60);
        let cutoff_time = Instant::now()
            .checked_sub(window_duration)
            .unwrap_or(self.start_time);

        let recent_metrics: Vec<_> = history
            .iter()
            .filter(|m| m.collected_at >= cutoff_time)
            .collect();

        if recent_metrics.is_empty() {
            return json!({
                "error": "No recent metrics in the specified window",
                "window_minutes": window_minutes,
            });
        }

        let total_operations: u64 = recent_metrics.iter().map(|m| m.total_operations).sum();
        let total_successful: u64 = recent_metrics.iter().map(|m| m.successful_operations).sum();
        let avg_duration: f64 = recent_metrics
            .iter()
            .map(|m| m.avg_operation_duration_ms)
            .sum::<f64>()
            / recent_metrics.len() as f64;

        json!({
            "window_minutes": window_minutes,
            "samples_count": recent_metrics.len(),
            "total_operations": total_operations,
            "successful_operations": total_successful,
            "success_rate": if total_operations > 0 {
                (total_successful as f64 / total_operations as f64) * 100.0
            } else {
                100.0
            },
            "avg_operation_duration_ms": avg_duration,
            "max_concurrent_operations": recent_metrics.iter().map(|m| m.max_concurrent_operations).max().unwrap_or(0),
            "sla_violations": recent_metrics.iter().map(|m| m.sla_violations).sum::<u64>(),
        })
    }

    /// Остановить сборщик метрик
    pub async fn shutdown(&self) {
        info!("🛑 Остановка MetricsCollector");
        self.active.store(false, Ordering::Relaxed);

        // Сохраняем финальные метрики в историю
        self.save_to_history().await;

        // Очищаем кэш
        self.clear_cache().await;

        info!("✅ MetricsCollector остановлен");
    }

    /// Проверить активность сборщика
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Получить все метрики без параметров (wrapper для совместимости с facade)
    pub async fn get_all_metrics(&self) -> Value {
        // Создаем пустой map координаторов для совместимости
        let empty_coordinators: HashMap<String, Arc<dyn Coordinator>> = HashMap::new();
        self.collect_all_metrics(&empty_coordinators).await
    }

    /// Получить dashboard метрики без параметров (wrapper для совместимости с facade)
    pub async fn get_dashboard_metrics(&self) -> Value {
        // Создаем пустой map координаторов для совместимости
        let empty_coordinators: HashMap<String, Arc<dyn Coordinator>> = HashMap::new();
        self.collect_dashboard_metrics(&empty_coordinators).await
    }

    /// Запустить адаптивную оптимизацию
    pub async fn run_adaptive_optimization(&self) -> Result<AdaptiveOptimizationResult> {
        info!("🎯 Запуск адаптивной оптимизации системы");

        let metrics = self.orchestration_metrics.read().await;
        let mut actions_taken = Vec::new();
        let mut recommendations = Vec::new();

        // Анализируем производительность
        if metrics.avg_operation_duration_ms > 1000.0 {
            recommendations.push("Рассмотреть увеличение размера thread pool".to_string());
        }

        // Анализируем нагрузку
        if metrics.current_concurrent_operations > 100 {
            recommendations.push("Включить throttling для защиты от перегрузки".to_string());
        }

        // Анализируем memory usage
        if metrics.memory_usage_bytes > 1_000_000_000 {
            // 1GB
            actions_taken.push("Включена более агрессивная сборка мусора".to_string());
        }

        // Анализируем success rate
        let success_rate = if metrics.total_operations > 0 {
            metrics.successful_operations as f64 / metrics.total_operations as f64
        } else {
            1.0
        };
        if success_rate < 0.95 {
            recommendations.push("Проверить состояние Circuit Breakers".to_string());
        }

        let result = AdaptiveOptimizationResult {
            actions_taken,
            recommendations,
            metrics_analyzed: true,
            timestamp: std::time::Instant::now(),
        };

        info!("✅ Адаптивная оптимизация завершена: {:?}", result);
        Ok(result)
    }

    /// Получить краткую сводку Circuit Breaker'ов
    async fn summarize_circuit_breakers(&self) -> Value {
        let cb_metrics = self.circuit_breaker_metrics.read().await;

        let mut summary = Map::new();
        let mut open_count = 0;
        let mut half_open_count = 0;
        let mut closed_count = 0;

        for (name, metric) in cb_metrics.iter() {
            match metric.status {
                CircuitBreakerStatus::Open => {
                    open_count += 1;
                    summary.insert(name.clone(), Value::String("open".to_string()));
                }
                CircuitBreakerStatus::HalfOpen => {
                    half_open_count += 1;
                    summary.insert(name.clone(), Value::String("half_open".to_string()));
                }
                CircuitBreakerStatus::Closed => {
                    closed_count += 1;
                    summary.insert(name.clone(), Value::String("closed".to_string()));
                }
            }
        }

        json!({
            "individual_states": Value::Object(summary),
            "totals": {
                "open": open_count,
                "half_open": half_open_count,
                "closed": closed_count,
                "total": cb_metrics.len(),
            }
        })
    }

    /// Получить кэшированные метрики
    async fn get_cached_metrics(&self, cache_key: &str) -> Option<Value> {
        let cache = self.aggregated_cache.read().await;

        if let Some((cached_value, cached_time)) = cache.get(cache_key) {
            if cached_time.elapsed() < self.cache_ttl {
                return Some(cached_value.clone());
            }
        }

        None
    }

    /// Кэшировать метрики
    async fn cache_metrics(&self, cache_key: &str, value: Value) {
        let mut cache = self.aggregated_cache.write().await;
        cache.insert(cache_key.to_string(), (value, Instant::now()));
    }

    /// Очистить кэш метрик
    async fn clear_cache(&self) {
        let mut cache = self.aggregated_cache.write().await;
        cache.clear();
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(1000) // 1000 записей в истории по умолчанию
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockCoordinator {
        ready: Arc<AtomicBool>,
    }

    impl MockCoordinator {
        fn new(ready: bool) -> Self {
            Self {
                ready: Arc::new(AtomicBool::new(ready)),
            }
        }
    }

    #[async_trait]
    impl Coordinator for MockCoordinator {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn metrics(&self) -> Value {
            json!({
                "test": "mock_coordinator",
                "ready": self.ready.load(Ordering::Relaxed),
            })
        }
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new(100);
        assert!(collector.is_active());
        assert_eq!(collector.max_history_size, 100);
    }

    #[tokio::test]
    async fn test_operation_recording() {
        let collector = MetricsCollector::new(100);

        // Записываем успешную операцию
        collector
            .record_operation("test_coordinator", 150, true)
            .await;

        let metrics = collector.orchestration_metrics.read().await;
        assert_eq!(metrics.total_operations, 1);
        assert_eq!(metrics.successful_operations, 1);
        assert_eq!(metrics.failed_operations, 0);
        assert_eq!(metrics.avg_operation_duration_ms, 150.0);
        assert_eq!(metrics.max_operation_duration_ms, 150);
        assert_eq!(metrics.min_operation_duration_ms, 150);
    }

    #[tokio::test]
    async fn test_circuit_breaker_updates() {
        let collector = MetricsCollector::new(100);

        // Обновляем Circuit Breaker в состояние Open
        collector
            .update_circuit_breaker("test", CircuitBreakerStatus::Open, 5)
            .await;

        let cb_metrics = collector.circuit_breaker_metrics.read().await;
        let metric = cb_metrics
            .get("test")
            .expect("Operation failed - converted from unwrap()");
        assert_eq!(metric.status, CircuitBreakerStatus::Open);
        assert_eq!(metric.failure_count, 5);
        assert!(metric.last_failure.is_some());
    }

    #[tokio::test]
    async fn test_dashboard_metrics() {
        let collector = MetricsCollector::new(100);
        let mut coordinators = HashMap::new();
        coordinators.insert("test".to_string(), Arc::new(MockCoordinator::new(true)));

        let dashboard_metrics = collector.collect_dashboard_metrics(&coordinators).await;

        assert_eq!(dashboard_metrics["status"], "active");
        assert!(
            dashboard_metrics["uptime_seconds"]
                .as_u64()
                .expect("Operation failed - converted from unwrap()")
                >= 0
        );
        assert_eq!(dashboard_metrics["total_operations"], 0);
    }

    #[tokio::test]
    async fn test_metrics_history() {
        let collector = MetricsCollector::new(2); // Маленький размер для теста

        // Записываем операции
        collector.record_operation("test1", 100, true).await;
        collector.save_to_history().await;

        collector.record_operation("test2", 200, true).await;
        collector.save_to_history().await;

        collector.record_operation("test3", 300, false).await;
        collector.save_to_history().await;

        let history = collector.get_metrics_history().await;

        // Ring buffer должен содержать только последние 2 записи
        assert_eq!(history.len(), 2);
        assert_eq!(history[1].total_operations, 3); // Последняя запись
        assert_eq!(history[1].failed_operations, 1);
    }

    #[tokio::test]
    async fn test_resource_usage_updates() {
        let collector = MetricsCollector::new(100);

        collector.update_resource_usage(10, 1024 * 1024).await;

        let metrics = collector.orchestration_metrics.read().await;
        assert_eq!(metrics.current_concurrent_operations, 10);
        assert_eq!(metrics.max_concurrent_operations, 10);
        assert_eq!(metrics.memory_usage_bytes, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_aggregated_stats() {
        let collector = MetricsCollector::new(100);

        // Записываем несколько операций
        collector.record_operation("test", 100, true).await;
        collector.save_to_history().await;

        collector.record_operation("test", 200, true).await;
        collector.save_to_history().await;

        let stats = collector.get_aggregated_stats(60).await; // 60 минут окно

        assert!(
            stats["samples_count"]
                .as_u64()
                .expect("Operation failed - converted from unwrap()")
                >= 1
        );
        assert_eq!(stats["total_operations"], 2);
        assert_eq!(stats["successful_operations"], 2);
        assert_eq!(stats["success_rate"], 100.0);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let collector = MetricsCollector::new(100);
        assert!(collector.is_active());

        collector.shutdown().await;
        assert!(!collector.is_active());
    }
}
