//! Performance Monitor - специализированный компонент для мониторинга производительности
//! 
//! Реализует Single Responsibility для performance monitoring
//! Интегрируется через DI с системой метрик

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};
use tracing::{info, debug, warn};
use uuid::Uuid;

use crate::agent_traits::{
    PerformanceMonitoringTrait, ComponentLifecycleTrait
};

// @component: {"k":"C","id":"performance_monitor","t":"Specialized performance monitoring component","m":{"cur":90,"tgt":95,"u":"%"},"f":["single_responsibility","clean_architecture","production_ready"]}
pub struct PerformanceMonitor {
    operations: Arc<Mutex<HashMap<String, OperationMetrics>>>,
    aggregated_metrics: Arc<Mutex<HashMap<String, AggregatedMetrics>>>,
    initialized: bool,
}

/// Метрики отдельной операции
#[derive(Debug, Clone)]
struct OperationMetrics {
    id: String,
    name: String,
    start_time: Instant,
    start_timestamp: SystemTime,
}

/// Агрегированные метрики для типа операций
#[derive(Debug, Clone)]
struct AggregatedMetrics {
    operation_name: String,
    total_count: u64,
    success_count: u64,
    failure_count: u64,
    total_duration_ms: u64,
    avg_duration_ms: f64,
    min_duration_ms: u64,
    max_duration_ms: u64,
    last_operation: SystemTime,
}

impl Default for AggregatedMetrics {
    fn default() -> Self {
        Self {
            operation_name: String::new(),
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            total_duration_ms: 0,
            avg_duration_ms: 0.0,
            min_duration_ms: u64::MAX,
            max_duration_ms: 0,
            last_operation: SystemTime::now(),
        }
    }
}

impl PerformanceMonitor {
    /// Создание нового PerformanceMonitor
    pub fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(HashMap::new())),
            aggregated_metrics: Arc::new(Mutex::new(HashMap::new())),
            initialized: false,
        }
    }
    
    /// Получение детальных метрик за период
    pub async fn get_detailed_metrics(&self, period_minutes: u32) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("PerformanceMonitor не инициализирован"));
        }
        
        let aggregated = self.aggregated_metrics.lock()
            .map_err(|_| anyhow::anyhow!("Ошибка блокировки метрик"))?;
        
        let mut result = format!("=== Метрики производительности (последние {} минут) ===\n", period_minutes);
        
        if aggregated.is_empty() {
            result.push_str("Нет данных о метриках\n");
            return Ok(result);
        }
        
        for (op_name, metrics) in aggregated.iter() {
            result.push_str(&format!(
                "\n📊 Операция: {}\n\
                 ├─ Общее количество: {}\n\
                 ├─ Успешных: {} ({:.1}%)\n\
                 ├─ Неуспешных: {} ({:.1}%)\n\
                 ├─ Среднее время: {:.2} мс\n\
                 ├─ Минимальное время: {} мс\n\
                 ├─ Максимальное время: {} мс\n\
                 └─ Последняя операция: {:?}\n",
                op_name,
                metrics.total_count,
                metrics.success_count,
                if metrics.total_count > 0 { 
                    metrics.success_count as f64 / metrics.total_count as f64 * 100.0 
                } else { 0.0 },
                metrics.failure_count,
                if metrics.total_count > 0 { 
                    metrics.failure_count as f64 / metrics.total_count as f64 * 100.0 
                } else { 0.0 },
                metrics.avg_duration_ms,
                if metrics.min_duration_ms == u64::MAX { 0 } else { metrics.min_duration_ms },
                metrics.max_duration_ms,
                metrics.last_operation
            ));
        }
        
        Ok(result)
    }
    
    /// Получение активных операций
    pub fn get_active_operations(&self) -> Result<Vec<String>> {
        let operations = self.operations.lock()
            .map_err(|_| anyhow::anyhow!("Ошибка блокировки операций"))?;
        
        let active: Vec<String> = operations.iter()
            .map(|(id, op)| format!("{}: {} (запущена {:?} назад)", 
                 id, op.name, op.start_time.elapsed()))
            .collect();
        
        Ok(active)
    }
    
    /// Получение топ медленных операций
    pub fn get_slowest_operations(&self, limit: usize) -> Result<Vec<(String, f64)>> {
        let aggregated = self.aggregated_metrics.lock()
            .map_err(|_| anyhow::anyhow!("Ошибка блокировки метрик"))?;
        
        let mut operations: Vec<(String, f64)> = aggregated.iter()
            .map(|(name, metrics)| (name.clone(), metrics.avg_duration_ms))
            .collect();
        
        operations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        operations.truncate(limit);
        
        Ok(operations)
    }
    
    /// Получение статистики успешности операций
    pub fn get_success_rates(&self) -> Result<HashMap<String, f64>> {
        let aggregated = self.aggregated_metrics.lock()
            .map_err(|_| anyhow::anyhow!("Ошибка блокировки метрик"))?;
        
        let success_rates: HashMap<String, f64> = aggregated.iter()
            .map(|(name, metrics)| {
                let rate = if metrics.total_count > 0 {
                    metrics.success_count as f64 / metrics.total_count as f64 * 100.0
                } else {
                    0.0
                };
                (name.clone(), rate)
            })
            .collect();
        
        Ok(success_rates)
    }
    
    /// Очистка старых метрик (maintenance)
    pub fn cleanup_old_metrics(&self, max_age_hours: u64) -> Result<usize> {
        let mut cleaned = 0;
        
        // Очистка активных операций (возможно зависшие)
        {
            let mut operations = self.operations.lock()
                .map_err(|_| anyhow::anyhow!("Ошибка блокировки операций"))?;
            
            let max_age = std::time::Duration::from_secs(max_age_hours * 3600);
            operations.retain(|_, op| {
                if op.start_time.elapsed() > max_age {
                    cleaned += 1;
                    warn!("Удаляем зависшую операцию: {} (id: {})", op.name, op.id);
                    false
                } else {
                    true
                }
            });
        }
        
        debug!("PerformanceMonitor: очищено {} старых метрик", cleaned);
        Ok(cleaned)
    }
}

#[async_trait]
impl PerformanceMonitoringTrait for PerformanceMonitor {
    fn start_operation(&self, operation_name: &str) -> String {
        let operation_id = Uuid::new_v4().to_string();
        
        let metrics = OperationMetrics {
            id: operation_id.clone(),
            name: operation_name.to_string(),
            start_time: Instant::now(),
            start_timestamp: SystemTime::now(),
        };
        
        if let Ok(mut operations) = self.operations.lock() {
            operations.insert(operation_id.clone(), metrics);
            debug!("PerformanceMonitor: началась операция '{}' с ID {}", operation_name, operation_id);
        }
        
        operation_id
    }
    
    fn finish_operation(&self, operation_id: &str, success: bool) {
        let operation = {
            let mut operations = match self.operations.lock() {
                Ok(ops) => ops,
                Err(_) => {
                    warn!("PerformanceMonitor: не удалось заблокировать операции для завершения");
                    return;
                }
            };
            
            operations.remove(operation_id)
        };
        
        if let Some(op) = operation {
            let duration_ms = op.start_time.elapsed().as_millis() as u64;
            
            // Обновляем агрегированные метрики
            if let Ok(mut aggregated) = self.aggregated_metrics.lock() {
                let metrics = aggregated.entry(op.name.clone()).or_default();
                metrics.operation_name = op.name.clone();
                metrics.total_count += 1;
                
                if success {
                    metrics.success_count += 1;
                } else {
                    metrics.failure_count += 1;
                }
                
                metrics.total_duration_ms += duration_ms;
                metrics.avg_duration_ms = metrics.total_duration_ms as f64 / metrics.total_count as f64;
                metrics.min_duration_ms = metrics.min_duration_ms.min(duration_ms);
                metrics.max_duration_ms = metrics.max_duration_ms.max(duration_ms);
                metrics.last_operation = SystemTime::now();
            }
            
            debug!("PerformanceMonitor: завершена операция '{}' за {} мс (успех: {})", 
                   op.name, duration_ms, success);
        } else {
            warn!("PerformanceMonitor: попытка завершить несуществующую операцию: {}", operation_id);
        }
    }
    
    async fn get_metrics(&self, period_minutes: u32) -> Result<HashMap<String, f64>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("PerformanceMonitor не инициализирован"));
        }
        
        let aggregated = self.aggregated_metrics.lock()
            .map_err(|_| anyhow::anyhow!("Ошибка блокировки метрик"))?;
        
        let mut metrics = HashMap::new();
        
        for (op_name, op_metrics) in aggregated.iter() {
            metrics.insert(format!("{}_total_count", op_name), op_metrics.total_count as f64);
            metrics.insert(format!("{}_success_rate", op_name), 
                if op_metrics.total_count > 0 {
                    op_metrics.success_count as f64 / op_metrics.total_count as f64 * 100.0
                } else { 0.0 }
            );
            metrics.insert(format!("{}_avg_duration_ms", op_name), op_metrics.avg_duration_ms);
            metrics.insert(format!("{}_max_duration_ms", op_name), op_metrics.max_duration_ms as f64);
        }
        
        // Добавляем общие метрики
        let active_ops = self.operations.lock()
            .map(|ops| ops.len())
            .unwrap_or(0);
        metrics.insert("active_operations_count".to_string(), active_ops as f64);
        
        Ok(metrics)
    }
    
    fn reset_metrics(&self) {
        if let Ok(mut operations) = self.operations.lock() {
            operations.clear();
        }
        
        if let Ok(mut aggregated) = self.aggregated_metrics.lock() {
            aggregated.clear();
        }
        
        info!("PerformanceMonitor: все метрики сброшены");
    }
}

#[async_trait]
impl ComponentLifecycleTrait for PerformanceMonitor {
    async fn initialize(&self) -> Result<()> {
        info!("PerformanceMonitor: инициализация начата");
        
        // Очищаем состояние на всякий случай
        self.reset_metrics();
        
        info!("PerformanceMonitor: инициализация завершена");
        Ok(())
    }
    
    async fn health_check(&self) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("PerformanceMonitor не инициализирован"));
        }
        
        // Проверяем что блокировки не зависли
        {
            let _operations = self.operations.lock()
                .map_err(|_| anyhow::anyhow!("Заблокированные операции"))?;
            let _aggregated = self.aggregated_metrics.lock()
                .map_err(|_| anyhow::anyhow!("Заблокированные метрики"))?;
        }
        
        debug!("PerformanceMonitor: health check прошел успешно");
        Ok(())
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("PerformanceMonitor: начинаем graceful shutdown");
        
        // В production версии здесь будет:
        // - Завершение всех активных операций
        // - Сохранение метрик в persistent storage
        // - Отправка финальных метрик в monitoring систему
        
        // Для начала просто очищаем состояние
        self.reset_metrics();
        
        info!("PerformanceMonitor: shutdown завершен");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_operation_tracking() {
        let monitor = PerformanceMonitor::new();
        monitor.initialize().await.unwrap();
        
        // Запускаем операцию
        let op_id = monitor.start_operation("test_operation");
        assert!(!op_id.is_empty());
        
        // Проверяем что операция активна
        let active = monitor.get_active_operations().unwrap();
        assert_eq!(active.len(), 1);
        assert!(active[0].contains("test_operation"));
        
        // Завершаем операцию
        monitor.finish_operation(&op_id, true);
        
        // Проверяем что операция больше не активна
        let active = monitor.get_active_operations().unwrap();
        assert_eq!(active.len(), 0);
    }
    
    #[tokio::test]
    async fn test_metrics_aggregation() {
        let monitor = PerformanceMonitor::new();
        monitor.initialize().await.unwrap();
        
        // Выполняем несколько операций
        for i in 0..5 {
            let op_id = monitor.start_operation("test_op");
            sleep(Duration::from_millis(10)).await;
            monitor.finish_operation(&op_id, i % 2 == 0); // 60% успешных
        }
        
        let metrics = monitor.get_metrics(60).await.unwrap();
        
        // Проверяем агрегированные метрики
        assert_eq!(metrics.get("test_op_total_count"), Some(&5.0));
        assert_eq!(metrics.get("test_op_success_rate"), Some(&60.0));
        assert!(metrics.contains_key("test_op_avg_duration_ms"));
    }
    
    #[tokio::test] 
    async fn test_success_rates() {
        let monitor = PerformanceMonitor::new();
        monitor.initialize().await.unwrap();
        
        // Успешные операции
        for _ in 0..8 {
            let op_id = monitor.start_operation("successful_op");
            monitor.finish_operation(&op_id, true);
        }
        
        // Неуспешные операции
        for _ in 0..2 {
            let op_id = monitor.start_operation("successful_op");
            monitor.finish_operation(&op_id, false);
        }
        
        let success_rates = monitor.get_success_rates().unwrap();
        assert_eq!(success_rates.get("successful_op"), Some(&80.0));
    }
    
    #[tokio::test]
    async fn test_detailed_metrics_report() {
        let monitor = PerformanceMonitor::new();
        monitor.initialize().await.unwrap();
        
        // Выполняем операцию
        let op_id = monitor.start_operation("detailed_test");
        monitor.finish_operation(&op_id, true);
        
        let report = monitor.get_detailed_metrics(60).await.unwrap();
        assert!(report.contains("detailed_test"));
        assert!(report.contains("Общее количество: 1"));
        assert!(report.contains("Успешных: 1"));
    }
}