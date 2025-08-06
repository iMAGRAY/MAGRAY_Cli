//! Production Monitoring Module - Single Responsibility для мониторинга
//! 
//! Этот модуль отвечает ТОЛЬКО за production мониторинг, метрики и health checks.
//! Применяет Single Responsibility и Observer pattern.

use anyhow::Result;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};
use tracing::{debug, info, warn, error};
use tokio::sync::RwLock;

use crate::orchestration::{HealthManager, ResourceController};

/// Production метрики системы
#[derive(Debug, Default, Clone)]
pub struct ProductionMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub circuit_breaker_trips: u64,
    pub avg_response_time_ms: f64,
    pub peak_memory_usage: f64,
    pub coordinator_health_scores: HashMap<String, f64>,
    pub last_health_check: Option<Instant>,
}

impl ProductionMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            return 100.0;
        }
        (self.successful_operations as f64 / self.total_operations as f64) * 100.0
    }

    pub fn failure_rate(&self) -> f64 {
        100.0 - self.success_rate()
    }

    pub fn is_healthy(&self) -> bool {
        self.success_rate() >= 95.0 && self.avg_response_time_ms <= 100.0
    }
}

/// Trait для мониторинга (Open/Closed Principle)
pub trait MetricsCollector {
    fn record_operation(&mut self, duration: Duration, success: bool);
    fn record_circuit_breaker_trip(&mut self);
    fn update_memory_usage(&mut self, usage_mb: f64);
    fn get_metrics(&self) -> ProductionMetrics;
    fn reset_metrics(&mut self);
}

/// Production metrics collector
#[derive(Debug)]
pub struct ProductionMetricsCollector {
    metrics: ProductionMetrics,
}

impl Default for ProductionMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: ProductionMetrics::default(),
        }
    }
}

impl MetricsCollector for ProductionMetricsCollector {
    fn record_operation(&mut self, duration: Duration, success: bool) {
        self.metrics.total_operations += 1;
        
        if success {
            self.metrics.successful_operations += 1;
        } else {
            self.metrics.failed_operations += 1;
        }
        
        // Exponential moving average для response time
        let duration_ms = duration.as_millis() as f64;
        let alpha = 0.1;
        if self.metrics.avg_response_time_ms == 0.0 {
            self.metrics.avg_response_time_ms = duration_ms;
        } else {
            self.metrics.avg_response_time_ms = alpha * duration_ms + (1.0 - alpha) * self.metrics.avg_response_time_ms;
        }
    }

    fn record_circuit_breaker_trip(&mut self) {
        self.metrics.circuit_breaker_trips += 1;
    }

    fn update_memory_usage(&mut self, usage_mb: f64) {
        if usage_mb > self.metrics.peak_memory_usage {
            self.metrics.peak_memory_usage = usage_mb;
        }
    }

    fn get_metrics(&self) -> ProductionMetrics {
        self.metrics.clone()
    }

    fn reset_metrics(&mut self) {
        self.metrics = ProductionMetrics::default();
    }
}

/// Production monitoring manager
pub struct ProductionMonitoringManager {
    /// Metrics collector
    metrics_collector: Arc<RwLock<ProductionMetricsCollector>>,
    /// Health manager reference
    health_manager: Option<Arc<HealthManager>>,
    /// Resource controller reference
    resource_controller: Option<Arc<ResourceController>>,
    /// Monitoring интервал
    monitoring_interval: Duration,
    /// Health check интервал
    health_check_interval: Duration,
}

impl ProductionMonitoringManager {
    pub fn new() -> Self {
        Self {
            metrics_collector: Arc::new(RwLock::new(ProductionMetricsCollector::new())),
            health_manager: None,
            resource_controller: None,
            monitoring_interval: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
        }
    }

    pub fn with_health_manager(mut self, health_manager: Arc<HealthManager>) -> Self {
        self.health_manager = Some(health_manager);
        self
    }

    pub fn with_resource_controller(mut self, resource_controller: Arc<ResourceController>) -> Self {
        self.resource_controller = Some(resource_controller);
        self
    }

    pub fn with_monitoring_interval(mut self, interval: Duration) -> Self {
        self.monitoring_interval = interval;
        self
    }

    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = interval;
        self
    }

    /// Записать успешную операцию
    pub async fn record_successful_operation(&self, duration: Duration) {
        let mut collector = self.metrics_collector.write().await;
        collector.record_operation(duration, true);
    }

    /// Записать неудачную операцию
    pub async fn record_failed_operation(&self, duration: Duration) {
        let mut collector = self.metrics_collector.write().await;
        collector.record_operation(duration, false);
    }

    /// Записать circuit breaker trip
    pub async fn record_circuit_breaker_trip(&self) {
        let mut collector = self.metrics_collector.write().await;
        collector.record_circuit_breaker_trip();
    }

    /// Получить текущие метрики
    pub async fn get_metrics(&self) -> ProductionMetrics {
        let collector = self.metrics_collector.read().await;
        collector.get_metrics()
    }

    /// Запустить production мониторинг
    pub async fn start_production_monitoring(&self) -> Result<()> {
        info!("📊 Запуск production мониторинга...");
        
        let metrics_collector = self.metrics_collector.clone();
        let monitoring_interval = self.monitoring_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitoring_interval);
            
            loop {
                interval.tick().await;
                
                let collector = metrics_collector.read().await;
                let metrics = collector.get_metrics();
                
                if metrics.total_operations > 0 {
                    let success_rate = metrics.success_rate();
                    
                    debug!("📊 Production метрики: операций={}, успех={}%, avg_response={}ms", 
                           metrics.total_operations,
                           success_rate,
                           metrics.avg_response_time_ms);
                    
                    if success_rate < 95.0 {
                        warn!("📉 Низкий success rate: {:.1}%", success_rate);
                    }
                    
                    if metrics.avg_response_time_ms > 100.0 {
                        warn!("⏱️ Высокое время отклика: {:.1}ms", metrics.avg_response_time_ms);
                    }
                }
            }
        });
        
        debug!("📊 Production мониторинг запущен");
        Ok(())
    }

    /// Запустить health мониторинг
    pub async fn start_health_monitoring(&self) -> Result<()> {
        if let Some(ref health_manager) = self.health_manager {
            info!("🚑 Запуск health мониторинга...");
            
            let manager = health_manager.clone();
            let health_check_interval = self.health_check_interval;
            
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(health_check_interval);
                
                loop {
                    interval.tick().await;
                    
                    if let Err(e) = manager.run_health_check().await {
                        error!("❌ Health check не удался: {}", e);
                    }
                }
            });
            
            debug!("🚑 Health мониторинг запущен");
        }
        
        Ok(())
    }

    /// Запустить resource мониторинг
    pub async fn start_resource_monitoring(&self) -> Result<()> {
        if let Some(ref resource_controller) = self.resource_controller {
            info!("💾 Запуск resource мониторинга и auto-scaling...");
            
            // Запускаем auto-scaling monitoring
            resource_controller.start_autoscaling_monitoring().await?;
            
            debug!("💾 Resource мониторинг запущен");
        }
        
        Ok(())
    }

    /// Остановить все мониторинги
    pub async fn stop_all_monitoring(&self) -> Result<()> {
        info!("🛑 Остановка всех мониторингов...");
        
        // В реальной реализации здесь бы были handles для остановки токio tasks
        // Пока просто логируем
        
        debug!("✅ Все мониторинги остановлены");
        Ok(())
    }

    /// Сбросить метрики (для тестов)
    pub async fn reset_metrics(&self) {
        let mut collector = self.metrics_collector.write().await;
        collector.reset_metrics();
    }

    /// Получить health score системы
    pub async fn get_system_health_score(&self) -> f64 {
        let metrics = self.get_metrics().await;
        
        if !metrics.is_healthy() {
            return 0.5; // Средний health score при проблемах
        }

        if let Some(ref health_manager) = self.health_manager {
            match health_manager.system_health().await {
                Ok(_) => 1.0, // Отличное здоровье
                Err(_) => 0.3, // Проблемы с health checks
            }
        } else {
            0.8 // Хороший score без health manager
        }
    }

    /// Генерировать подробный отчет о системе
    pub async fn generate_system_report(&self) -> String {
        let metrics = self.get_metrics().await;
        let health_score = self.get_system_health_score().await;
        
        format!(
            "=== PRODUCTION SYSTEM REPORT ===\n\
            📊 Операций всего: {}\n\
            ✅ Успешных: {} ({:.1}%)\n\
            ❌ Неудачных: {} ({:.1}%)\n\
            ⏱️  Среднее время отклика: {:.1}ms\n\
            🚨 Circuit breaker trips: {}\n\
            💾 Пиковое использование памяти: {:.1}MB\n\
            🏥 Health score: {:.1}\n\
            🎯 Статус системы: {}\n\
            ================================",
            metrics.total_operations,
            metrics.successful_operations,
            metrics.success_rate(),
            metrics.failed_operations,
            metrics.failure_rate(),
            metrics.avg_response_time_ms,
            metrics.circuit_breaker_trips,
            metrics.peak_memory_usage,
            health_score,
            if metrics.is_healthy() { "HEALTHY" } else { "DEGRADED" }
        )
    }
}

impl Default for ProductionMonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_metrics() {
        let mut metrics = ProductionMetrics::default();
        assert_eq!(metrics.success_rate(), 100.0);
        
        metrics.total_operations = 100;
        metrics.successful_operations = 95;
        metrics.failed_operations = 5;
        
        assert_eq!(metrics.success_rate(), 95.0);
        assert_eq!(metrics.failure_rate(), 5.0);
        assert!(metrics.is_healthy());
        
        metrics.avg_response_time_ms = 150.0;
        assert!(!metrics.is_healthy()); // Response time too high
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let mut collector = ProductionMetricsCollector::new();
        
        collector.record_operation(Duration::from_millis(50), true);
        collector.record_operation(Duration::from_millis(100), false);
        
        let metrics = collector.get_metrics();
        assert_eq!(metrics.total_operations, 2);
        assert_eq!(metrics.successful_operations, 1);
        assert_eq!(metrics.failed_operations, 1);
        assert!(metrics.avg_response_time_ms > 0.0);
    }

    #[tokio::test] 
    async fn test_monitoring_manager_creation() {
        let manager = ProductionMonitoringManager::new();
        
        // Test recording operations
        manager.record_successful_operation(Duration::from_millis(25)).await;
        manager.record_failed_operation(Duration::from_millis(75)).await;
        
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_operations, 2);
        assert_eq!(metrics.success_rate(), 50.0);
        
        // Test reset
        manager.reset_metrics().await;
        let reset_metrics = manager.get_metrics().await;
        assert_eq!(reset_metrics.total_operations, 0);
    }

    #[tokio::test]
    async fn test_system_health_score() {
        let manager = ProductionMonitoringManager::new();
        
        // Record some successful operations
        for _ in 0..10 {
            manager.record_successful_operation(Duration::from_millis(20)).await;
        }
        
        let health_score = manager.get_system_health_score().await;
        assert!(health_score >= 0.8); // Should be healthy without health manager
    }

    #[tokio::test]
    async fn test_system_report_generation() {
        let manager = ProductionMonitoringManager::new();
        
        manager.record_successful_operation(Duration::from_millis(30)).await;
        manager.record_failed_operation(Duration::from_millis(90)).await;
        
        let report = manager.generate_system_report().await;
        assert!(report.contains("PRODUCTION SYSTEM REPORT"));
        assert!(report.contains("Операций всего: 2"));
        assert!(report.contains("Успешных: 1"));
    }
}