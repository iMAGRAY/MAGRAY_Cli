//! PerformanceTracker - отслеживание и анализ производительности
//!
//! Реализует Single Responsibility Principle для мониторинга метрик
//! производительности с минимальным overhead.

use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Метрика одной операции
#[derive(Debug, Clone, Serialize)]
pub struct OperationMetric {
    pub operation_id: String,
    pub component: String,
    pub operation_type: String,
    #[serde(skip)]
    pub start_time: Instant,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    pub success: bool,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
}

mod duration_serde {
    use serde::{Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }
}

/// Агрегированные метрики компонента
#[derive(Debug, Clone, Serialize)]
pub struct ComponentMetrics {
    pub component_name: String,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub p95_duration_ms: u64,
    pub p99_duration_ms: u64,
    pub operations_per_second: f64,
    pub error_rate: f64,
    #[serde(skip)]
    pub last_operation_time: Option<Instant>,
}

/// Системные метрики
#[derive(Debug, Clone, Serialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub total_operations: u64,
    pub operations_per_second: f64,
    pub average_response_time_ms: f64,
    pub error_rate: f64,
    pub active_operations: u64,
    pub peak_concurrent_operations: u64,
    pub components: HashMap<String, ComponentMetrics>,
}

/// Конфигурация tracker'а
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Максимальное количество метрик в памяти
    pub max_metrics_in_memory: usize,
    /// Интервал агрегации метрик
    pub aggregation_interval: Duration,
    /// Включить детальное логирование
    pub enable_detailed_logging: bool,
    /// Пороговые значения для предупреждений
    pub warning_thresholds: WarningThresholds,
}

/// Пороговые значения для предупреждений
#[derive(Debug, Clone)]
pub struct WarningThresholds {
    pub max_response_time_ms: u64,
    pub max_error_rate: f64,
    pub max_concurrent_operations: u64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            max_metrics_in_memory: 10_000,
            aggregation_interval: Duration::from_secs(60),
            enable_detailed_logging: false,
            warning_thresholds: WarningThresholds {
                max_response_time_ms: 1000,
                max_error_rate: 0.05, // 5%
                max_concurrent_operations: 100,
            },
        }
    }
}

/// Активная операция
#[derive(Debug)]
struct ActiveOperation {
    operation_id: String,
    component: String,
    operation_type: String,
    start_time: Instant,
    metadata: HashMap<String, String>,
}

/// Трекер производительности
pub struct PerformanceTracker {
    config: TrackerConfig,
    start_time: Instant,

    // Активные операции
    active_operations: Arc<RwLock<HashMap<String, ActiveOperation>>>,

    // История метрик
    metrics_history: Arc<RwLock<VecDeque<OperationMetric>>>,

    // Агрегированные метрики по компонентам
    component_metrics: Arc<RwLock<HashMap<String, ComponentMetrics>>>,

    // Счетчики
    peak_concurrent_operations: Arc<RwLock<u64>>,
}

impl PerformanceTracker {
    /// Создать новый трекер
    pub fn new(config: TrackerConfig) -> Self {
        info!(
            "Инициализация PerformanceTracker с лимитом {} метрик",
            config.max_metrics_in_memory
        );

        Self {
            config,
            start_time: Instant::now(),
            active_operations: Arc::new(RwLock::new(HashMap::new())),
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            component_metrics: Arc::new(RwLock::new(HashMap::new())),
            peak_concurrent_operations: Arc::new(RwLock::new(0)),
        }
    }

    /// Начать отслеживание операции
    pub async fn start_operation(&self, component: &str, operation_type: &str) -> String {
        self.start_operation_with_metadata(component, operation_type, HashMap::new())
            .await
    }

    /// Начать отслеживание операции с метаданными
    pub async fn start_operation_with_metadata(
        &self,
        component: &str,
        operation_type: &str,
        metadata: HashMap<String, String>,
    ) -> String {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        let operation = ActiveOperation {
            operation_id: operation_id.clone(),
            component: component.to_string(),
            operation_type: operation_type.to_string(),
            start_time,
            metadata,
        };

        {
            let mut active_ops = self.active_operations.write().await;
            active_ops.insert(operation_id.clone(), operation);

            // Обновляем пиковое значение concurrent операций
            let current_count = active_ops.len() as u64;
            let mut peak = self.peak_concurrent_operations.write().await;
            if current_count > *peak {
                *peak = current_count;
            }

            // Проверяем пороговое значение
            if current_count > self.config.warning_thresholds.max_concurrent_operations {
                warn!("Высокое количество активных операций: {}", current_count);
            }
        }

        if self.config.enable_detailed_logging {
            debug!(
                "Начата операция {} ({}::{})",
                operation_id, component, operation_type
            );
        }

        operation_id
    }

    /// Завершить отслеживание операции
    pub async fn finish_operation(&self, operation_id: &str, success: bool) -> Result<()> {
        self.finish_operation_with_error(operation_id, success, None)
            .await
    }

    /// Завершить отслеживание операции с ошибкой
    pub async fn finish_operation_with_error(
        &self,
        operation_id: &str,
        success: bool,
        error_message: Option<String>,
    ) -> Result<()> {
        let operation = {
            let mut active_ops = self.active_operations.write().await;
            active_ops.remove(operation_id)
        };

        let operation = match operation {
            Some(op) => op,
            None => {
                warn!(
                    "Попытка завершить несуществующую операцию: {}",
                    operation_id
                );
                return Ok(());
            }
        };

        let duration = operation.start_time.elapsed();

        // Проверяем пороговое значение времени ответа
        if duration.as_millis() > self.config.warning_thresholds.max_response_time_ms as u128 {
            warn!(
                "Медленная операция {}::{} выполнялась {:?}",
                operation.component, operation.operation_type, duration
            );
        }

        let metric = OperationMetric {
            operation_id: operation.operation_id,
            component: operation.component.clone(),
            operation_type: operation.operation_type.clone(),
            start_time: operation.start_time,
            duration,
            success,
            error_message,
            metadata: operation.metadata,
        };

        // Добавляем метрику в историю
        {
            let mut history = self.metrics_history.write().await;
            history.push_back(metric.clone());

            // Ограничиваем размер истории
            while history.len() > self.config.max_metrics_in_memory {
                history.pop_front();
            }
        }

        // Обновляем агрегированные метрики компонента
        self.update_component_metrics(&metric).await;

        if self.config.enable_detailed_logging {
            debug!(
                "Завершена операция {} за {:?} (success: {})",
                operation_id, duration, success
            );
        }

        Ok(())
    }

    /// Обновить агрегированные метрики компонента
    async fn update_component_metrics(&self, metric: &OperationMetric) {
        let mut components = self.component_metrics.write().await;

        let component_metrics = components
            .entry(metric.component.clone())
            .or_insert_with(|| ComponentMetrics {
                component_name: metric.component.clone(),
                total_operations: 0,
                successful_operations: 0,
                failed_operations: 0,
                average_duration_ms: 0.0,
                min_duration_ms: u64::MAX,
                max_duration_ms: 0,
                p95_duration_ms: 0,
                p99_duration_ms: 0,
                operations_per_second: 0.0,
                error_rate: 0.0,
                last_operation_time: None,
            });

        // Обновляем счетчики
        component_metrics.total_operations += 1;
        if metric.success {
            component_metrics.successful_operations += 1;
        } else {
            component_metrics.failed_operations += 1;
        }

        // Обновляем временные метрики
        let duration_ms = metric.duration.as_millis() as u64;
        component_metrics.min_duration_ms = component_metrics.min_duration_ms.min(duration_ms);
        component_metrics.max_duration_ms = component_metrics.max_duration_ms.max(duration_ms);
        component_metrics.last_operation_time = Some(metric.start_time);

        // Обновляем скользящее среднее времени отклика
        let alpha = 0.1; // коэффициент сглаживания
        component_metrics.average_duration_ms =
            alpha * duration_ms as f64 + (1.0 - alpha) * component_metrics.average_duration_ms;

        // Обновляем error rate
        component_metrics.error_rate =
            component_metrics.failed_operations as f64 / component_metrics.total_operations as f64;

        // Проверяем пороговое значение error rate
        if component_metrics.error_rate > self.config.warning_thresholds.max_error_rate {
            warn!(
                "Высокий уровень ошибок для компонента {}: {:.2}%",
                metric.component,
                component_metrics.error_rate * 100.0
            );
        }
    }

    /// Получить метрики компонента
    pub async fn get_component_metrics(&self, component: &str) -> Option<ComponentMetrics> {
        let components = self.component_metrics.read().await;
        components.get(component).cloned()
    }

    /// Получить системные метрики
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        let components = self.component_metrics.read().await;
        let active_ops = self.active_operations.read().await;
        let peak_concurrent = *self.peak_concurrent_operations.read().await;

        let uptime_seconds = self.start_time.elapsed().as_secs();

        // Агрегируем метрики по всем компонентам
        let mut total_operations = 0;
        let mut _total_successful = 0;
        let mut total_failed = 0;
        let mut weighted_avg_duration = 0.0;

        for metrics in components.values() {
            total_operations += metrics.total_operations;
            _total_successful += metrics.successful_operations;
            total_failed += metrics.failed_operations;

            // Взвешенное среднее по количеству операций
            weighted_avg_duration += metrics.average_duration_ms * metrics.total_operations as f64;
        }

        let average_response_time_ms = if total_operations > 0 {
            weighted_avg_duration / total_operations as f64
        } else {
            0.0
        };

        let error_rate = if total_operations > 0 {
            total_failed as f64 / total_operations as f64
        } else {
            0.0
        };

        let operations_per_second = if uptime_seconds > 0 {
            total_operations as f64 / uptime_seconds as f64
        } else {
            0.0
        };

        SystemMetrics {
            uptime_seconds,
            total_operations,
            operations_per_second,
            average_response_time_ms,
            error_rate,
            active_operations: active_ops.len() as u64,
            peak_concurrent_operations: peak_concurrent,
            components: components.clone(),
        }
    }

    /// Получить детальную статистику за период
    pub async fn get_detailed_metrics(&self, last_seconds: u64) -> Result<String> {
        let system_metrics = self.get_system_metrics().await;
        let cutoff_time = Instant::now() - Duration::from_secs(last_seconds);

        // Получаем метрики за последний период
        let history = self.metrics_history.read().await;
        let recent_metrics: Vec<_> = history
            .iter()
            .filter(|metric| metric.start_time >= cutoff_time)
            .collect();

        let mut report = format!(
            "=== Performance Report (последние {} сек) ===\n\n",
            last_seconds
        );

        report.push_str(&format!("Системные метрики:\n"));
        report.push_str(&format!(
            "├─ Uptime: {} сек\n",
            system_metrics.uptime_seconds
        ));
        report.push_str(&format!(
            "├─ Всего операций: {}\n",
            system_metrics.total_operations
        ));
        report.push_str(&format!(
            "├─ Операций в секунду: {:.2}\n",
            system_metrics.operations_per_second
        ));
        report.push_str(&format!(
            "├─ Среднее время отклика: {:.2} мс\n",
            system_metrics.average_response_time_ms
        ));
        report.push_str(&format!(
            "├─ Процент ошибок: {:.2}%\n",
            system_metrics.error_rate * 100.0
        ));
        report.push_str(&format!(
            "├─ Активные операции: {}\n",
            system_metrics.active_operations
        ));
        report.push_str(&format!(
            "└─ Пик concurrent операций: {}\n\n",
            system_metrics.peak_concurrent_operations
        ));

        report.push_str("Метрики по компонентам:\n");
        for (name, metrics) in &system_metrics.components {
            report.push_str(&format!(
                "├─ {}: {} ops, {:.2} мс avg, {:.1}% errors\n",
                name,
                metrics.total_operations,
                metrics.average_duration_ms,
                metrics.error_rate * 100.0
            ));
        }

        report.push_str(&format!(
            "\nРазбивка по последним {} операциям:\n",
            recent_metrics.len()
        ));
        let mut by_component: HashMap<String, Vec<&OperationMetric>> = HashMap::new();

        for metric in &recent_metrics {
            by_component
                .entry(metric.component.clone())
                .or_insert_with(Vec::new)
                .push(metric);
        }

        for (component, metrics) in by_component {
            let successful = metrics.iter().filter(|m| m.success).count();
            let avg_duration = metrics
                .iter()
                .map(|m| m.duration.as_millis() as f64)
                .sum::<f64>()
                / metrics.len() as f64;

            report.push_str(&format!(
                "└─ {}: {}/{} успешных, {:.2} мс среднее время\n",
                component,
                successful,
                metrics.len(),
                avg_duration
            ));
        }

        Ok(report)
    }

    /// Получить активные операции
    pub async fn get_active_operations(&self) -> Result<Vec<(String, String, Duration)>> {
        let active_ops = self.active_operations.read().await;

        let mut operations = Vec::new();
        for (id, op) in active_ops.iter() {
            operations.push((
                id.clone(),
                format!("{}::{}", op.component, op.operation_type),
                op.start_time.elapsed(),
            ));
        }

        Ok(operations)
    }

    /// Очистить старые метрики
    pub async fn cleanup_old_metrics(&self, older_than_hours: u64) -> Result<usize> {
        let cutoff_time = Instant::now() - Duration::from_secs(older_than_hours * 3600);

        let mut history = self.metrics_history.write().await;
        let original_len = history.len();

        // Удаляем старые метрики
        history.retain(|metric| metric.start_time >= cutoff_time);

        let removed_count = original_len - history.len();

        if removed_count > 0 {
            info!(
                "Очищено {} старых метрик (старше {} часов)",
                removed_count, older_than_hours
            );
        }

        Ok(removed_count)
    }

    /// Сбросить все метрики
    pub async fn reset_metrics(&self) {
        let mut history = self.metrics_history.write().await;
        let mut components = self.component_metrics.write().await;
        let mut peak = self.peak_concurrent_operations.write().await;

        history.clear();
        components.clear();
        *peak = 0;

        info!("Все метрики производительности сброшены");
    }

    /// Получить общую статистику
    pub async fn get_summary(&self) -> String {
        let system_metrics = self.get_system_metrics().await;
        let active_ops = self.get_active_operations().await.unwrap_or_default();

        format!(
            "Performance Summary: {} ops total, {:.1} ops/sec, {:.1} ms avg, {:.1}% errors, {} active",
            system_metrics.total_operations,
            system_metrics.operations_per_second,
            system_metrics.average_response_time_ms,
            system_metrics.error_rate * 100.0,
            active_ops.len()
        )
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new(TrackerConfig::default())
    }
}

/// Builder для PerformanceTracker
pub struct PerformanceTrackerBuilder {
    config: TrackerConfig,
}

impl PerformanceTrackerBuilder {
    pub fn new() -> Self {
        Self {
            config: TrackerConfig::default(),
        }
    }

    pub fn with_max_metrics(mut self, max_metrics: usize) -> Self {
        self.config.max_metrics_in_memory = max_metrics;
        self
    }

    pub fn with_detailed_logging(mut self, enabled: bool) -> Self {
        self.config.enable_detailed_logging = enabled;
        self
    }

    pub fn with_warning_thresholds(mut self, thresholds: WarningThresholds) -> Self {
        self.config.warning_thresholds = thresholds;
        self
    }

    pub fn build(self) -> PerformanceTracker {
        PerformanceTracker::new(self.config)
    }
}

impl Default for PerformanceTrackerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_basic_operation_tracking() {
        let tracker = PerformanceTracker::default();

        // Start operation
        let op_id = tracker
            .start_operation("test_component", "test_operation")
            .await;

        // Simulate some work
        sleep(Duration::from_millis(10)).await;

        // Finish operation
        tracker.finish_operation(&op_id, true).await.unwrap();

        // Check metrics
        let component_metrics = tracker.get_component_metrics("test_component").await;
        assert!(component_metrics.is_some());

        let metrics = component_metrics.unwrap();
        assert_eq!(metrics.total_operations, 1);
        assert_eq!(metrics.successful_operations, 1);
        assert_eq!(metrics.failed_operations, 0);
        assert!(metrics.average_duration_ms > 0.0);
    }

    #[tokio::test]
    async fn test_system_metrics_aggregation() {
        let tracker = PerformanceTracker::default();

        // Execute multiple operations
        for i in 0..5 {
            let op_id = tracker.start_operation("component1", "operation").await;
            sleep(Duration::from_millis(5)).await;
            tracker.finish_operation(&op_id, i < 4).await.unwrap(); // 1 failure
        }

        for i in 0..3 {
            let op_id = tracker.start_operation("component2", "operation").await;
            sleep(Duration::from_millis(3)).await;
            tracker.finish_operation(&op_id, true).await.unwrap();
        }

        let system_metrics = tracker.get_system_metrics().await;

        assert_eq!(system_metrics.total_operations, 8);
        assert!(system_metrics.error_rate > 0.0);
        assert!(system_metrics.operations_per_second > 0.0);
        assert_eq!(system_metrics.components.len(), 2);

        // Check component-specific metrics
        let comp1_metrics = system_metrics.components.get("component1").unwrap();
        assert_eq!(comp1_metrics.total_operations, 5);
        assert_eq!(comp1_metrics.failed_operations, 1);
    }

    #[tokio::test]
    async fn test_active_operations_tracking() {
        let tracker = PerformanceTracker::default();

        // Start but don't finish operation
        let _op_id1 = tracker.start_operation("component", "long_operation").await;
        let _op_id2 = tracker
            .start_operation("component", "another_operation")
            .await;

        let active_ops = tracker.get_active_operations().await.unwrap();
        assert_eq!(active_ops.len(), 2);

        // Check system metrics show active operations
        let system_metrics = tracker.get_system_metrics().await;
        assert_eq!(system_metrics.active_operations, 2);
    }

    #[tokio::test]
    async fn test_metrics_cleanup() {
        let config = TrackerConfig {
            max_metrics_in_memory: 3,
            ..Default::default()
        };
        let tracker = PerformanceTracker::new(config);

        // Add more metrics than the limit
        for i in 0..5 {
            let op_id = tracker.start_operation("component", "operation").await;
            tracker.finish_operation(&op_id, true).await.unwrap();
        }

        // Check that history is limited
        let history = tracker.metrics_history.read().await;
        assert_eq!(history.len(), 3); // Should be limited by max_metrics_in_memory
    }

    #[tokio::test]
    async fn test_performance_tracker_builder() {
        let thresholds = WarningThresholds {
            max_response_time_ms: 500,
            max_error_rate: 0.1,
            max_concurrent_operations: 50,
        };

        let tracker = PerformanceTrackerBuilder::new()
            .with_max_metrics(1000)
            .with_detailed_logging(true)
            .with_warning_thresholds(thresholds)
            .build();

        assert_eq!(tracker.config.max_metrics_in_memory, 1000);
        assert!(tracker.config.enable_detailed_logging);
        assert_eq!(tracker.config.warning_thresholds.max_response_time_ms, 500);
    }
}
