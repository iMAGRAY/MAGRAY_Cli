use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{info, warn, error};

/// Уровни критичности для health alerts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Fatal,
}

/// Типы компонентов системы для мониторинга
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentType {
    VectorStore,
    EmbeddingService,
    RerankingService,
    PromotionEngine,
    Cache,
    Database,
    Memory,
}

/// Health статус компонента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Down,
}

/// Health метрика
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetric {
    pub component: ComponentType,
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
    pub threshold_warning: Option<f64>,
    pub threshold_critical: Option<f64>,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    pub id: String,
    pub component: ComponentType,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub metric_value: Option<f64>,
    pub threshold: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Общий health статус системы
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthStatus {
    pub overall_status: HealthStatus,
    pub component_statuses: HashMap<ComponentType, HealthStatus>,
    pub active_alerts: Vec<HealthAlert>,
    pub metrics_summary: HashMap<String, f64>,
    pub last_updated: DateTime<Utc>,
    pub uptime_seconds: u64,
}

/// Performance статистика компонента
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPerformanceStats {
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub last_error: Option<String>,
    pub last_error_time: Option<DateTime<Utc>>,
}

/// Health Monitor - основной класс для мониторинга системы
pub struct HealthMonitor {
    component_stats: Arc<RwLock<HashMap<ComponentType, ComponentPerformanceStats>>>,
    metrics_history: Arc<RwLock<HashMap<String, VecDeque<HealthMetric>>>>,
    active_alerts: Arc<RwLock<HashMap<String, HealthAlert>>>,
    alert_sender: Option<mpsc::UnboundedSender<HealthAlert>>,
    start_time: Instant,
    config: HealthConfig,
}

/// Конфигурация health monitoring
#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub metrics_retention_minutes: u32,
    pub max_metrics_per_type: usize,
    pub alert_cooldown_minutes: u32,
    pub enable_alerts: bool,
    pub enable_real_time_metrics: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            metrics_retention_minutes: 60,
            max_metrics_per_type: 1000,
            alert_cooldown_minutes: 5,
            enable_alerts: true,
            enable_real_time_metrics: true,
        }
    }
}

impl HealthMonitor {
    /// Создает новый health monitor
    pub fn new(config: HealthConfig) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let monitor = Self {
            component_stats: Arc::new(RwLock::new(HashMap::new())),
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_sender: Some(sender),
            start_time: Instant::now(),
            config,
        };
        
        // Запускаем обработчик alerts в фоне
        if monitor.config.enable_alerts {
            tokio::spawn(monitor.clone().alert_processor(receiver));
        }
        
        monitor
    }
    
    /// Записывает метрику компонента
    pub fn record_metric(&self, metric: HealthMetric) -> Result<()> {
        if !self.config.enable_real_time_metrics {
            return Ok(());
        }
        
        let metric_key = format!("{:?}_{}", metric.component, metric.metric_name);
        
        // Добавляем метрику в историю
        {
            let mut history = self.metrics_history.write().unwrap();
            let metrics = history.entry(metric_key.clone()).or_insert_with(VecDeque::new);
            
            metrics.push_back(metric.clone());
            
            // Ограничиваем размер истории
            while metrics.len() > self.config.max_metrics_per_type {
                metrics.pop_front();
            }
            
            // Удаляем старые метрики
            let cutoff_time = Utc::now() - Duration::minutes(self.config.metrics_retention_minutes as i64);
            while let Some(front) = metrics.front() {
                if front.timestamp < cutoff_time {
                    metrics.pop_front();
                } else {
                    break;
                }
            }
        }
        
        // Проверяем пороги и генерируем alerts
        self.check_thresholds(&metric)?;
        
        Ok(())
    }
    
    /// Записывает операционную статистику компонента
    pub fn record_operation(&self, component: ComponentType, success: bool, response_time_ms: f64, error: Option<String>) {
        let mut stats = self.component_stats.write().unwrap();
        let component_stats = stats.entry(component).or_insert_with(|| ComponentPerformanceStats {
            avg_response_time_ms: 0.0,
            success_rate: 1.0,
            total_requests: 0,
            failed_requests: 0,
            last_error: None,
            last_error_time: None,
        });
        
        component_stats.total_requests += 1;
        
        if !success {
            component_stats.failed_requests += 1;
            component_stats.last_error = error;
            component_stats.last_error_time = Some(Utc::now());
        }
        
        // Обновляем среднее время ответа (простое скользящее среднее)
        let total = component_stats.total_requests as f64;
        component_stats.avg_response_time_ms = 
            (component_stats.avg_response_time_ms * (total - 1.0) + response_time_ms) / total;
        
        // Обновляем success rate
        component_stats.success_rate = 
            (component_stats.total_requests - component_stats.failed_requests) as f64 / total;
    }
    
    /// Получает текущий health статус системы
    pub fn get_system_health(&self) -> SystemHealthStatus {
        let component_statuses = self.calculate_component_statuses();
        let overall_status = self.calculate_overall_status(&component_statuses);
        let active_alerts = self.get_active_alerts();
        let metrics_summary = self.get_metrics_summary();
        
        SystemHealthStatus {
            overall_status,
            component_statuses,
            active_alerts,
            metrics_summary,
            last_updated: Utc::now(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }
    
    /// Получает метрики для компонента
    pub fn get_component_metrics(&self, component: ComponentType, metric_name: &str, limit: Option<usize>) -> Vec<HealthMetric> {
        let metric_key = format!("{:?}_{}", component, metric_name);
        let history = self.metrics_history.read().unwrap();
        
        if let Some(metrics) = history.get(&metric_key) {
            let mut result: Vec<_> = metrics.iter().cloned().collect();
            result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            if let Some(limit) = limit {
                result.truncate(limit);
            }
            
            result
        } else {
            Vec::new()
        }
    }
    
    /// Получает статистику производительности компонента
    pub fn get_component_performance(&self, component: ComponentType) -> Option<ComponentPerformanceStats> {
        let stats = self.component_stats.read().unwrap();
        stats.get(&component).cloned()
    }
    
    /// Создает custom alert
    pub fn create_alert(&self, component: ComponentType, severity: AlertSeverity, title: String, description: String) {
        if !self.config.enable_alerts {
            return;
        }
        
        let alert = HealthAlert {
            id: format!("{:?}_{:?}_{}", component, severity, Utc::now().timestamp()),
            component,
            severity: severity.clone(),
            title,
            description,
            metric_value: None,
            threshold: None,
            timestamp: Utc::now(),
            resolved: false,
            resolved_at: None,
        };
        
        if let Some(ref sender) = self.alert_sender {
            if let Err(e) = sender.send(alert.clone()) {
                error!("Failed to send alert: {}", e);
            }
        }
        
        // Сохраняем alert
        let mut alerts = self.active_alerts.write().unwrap();
        alerts.insert(alert.id.clone(), alert);
    }
    
    /// Разрешает alert
    pub fn resolve_alert(&self, alert_id: &str) {
        let mut alerts = self.active_alerts.write().unwrap();
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            info!("Alert resolved: {}", alert_id);
        }
    }
    
    /// Проверяет пороги метрики и генерирует alerts
    fn check_thresholds(&self, metric: &HealthMetric) -> Result<()> {
        // Проверяем critical threshold
        if let Some(critical_threshold) = metric.threshold_critical {
            if metric.value >= critical_threshold {
                self.create_alert(
                    metric.component.clone(),
                    AlertSeverity::Critical,
                    format!("Critical: {} exceeded threshold", metric.metric_name),
                    format!("Metric {} has value {:.2} {} which exceeds critical threshold {:.2}", 
                           metric.metric_name, metric.value, metric.unit, critical_threshold)
                );
            }
        }
        
        // Проверяем warning threshold
        else if let Some(warning_threshold) = metric.threshold_warning {
            if metric.value >= warning_threshold {
                self.create_alert(
                    metric.component.clone(),
                    AlertSeverity::Warning,
                    format!("Warning: {} approaching threshold", metric.metric_name),
                    format!("Metric {} has value {:.2} {} which exceeds warning threshold {:.2}", 
                           metric.metric_name, metric.value, metric.unit, warning_threshold)
                );
            }
        }
        
        Ok(())
    }
    
    /// Вычисляет статусы компонентов
    fn calculate_component_statuses(&self) -> HashMap<ComponentType, HealthStatus> {
        let mut statuses = HashMap::new();
        let stats = self.component_stats.read().unwrap();
        
        for (component, perf_stats) in stats.iter() {
            let status = match perf_stats.success_rate {
                rate if rate >= 0.95 => HealthStatus::Healthy,
                rate if rate >= 0.80 => HealthStatus::Degraded,
                rate if rate >= 0.50 => HealthStatus::Unhealthy,
                _ => HealthStatus::Down,
            };
            
            statuses.insert(component.clone(), status);
        }
        
        statuses
    }
    
    /// Вычисляет общий статус системы
    fn calculate_overall_status(&self, component_statuses: &HashMap<ComponentType, HealthStatus>) -> HealthStatus {
        if component_statuses.is_empty() {
            return HealthStatus::Healthy;
        }
        
        let mut has_down = false;
        let mut has_unhealthy = false;
        let mut has_degraded = false;
        
        for status in component_statuses.values() {
            match status {
                HealthStatus::Down => has_down = true,
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                HealthStatus::Healthy => {},
            }
        }
        
        if has_down {
            HealthStatus::Down
        } else if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
    
    /// Получает активные alerts
    fn get_active_alerts(&self) -> Vec<HealthAlert> {
        let alerts = self.active_alerts.read().unwrap();
        alerts.values()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }
    
    /// Получает сводку метрик
    fn get_metrics_summary(&self) -> HashMap<String, f64> {
        let mut summary = HashMap::new();
        let history = self.metrics_history.read().unwrap();
        
        for (metric_key, metrics) in history.iter() {
            if let Some(latest) = metrics.back() {
                summary.insert(metric_key.clone(), latest.value);
            }
        }
        
        summary
    }
    
    /// Обработчик alerts (запускается в фоне)
    async fn alert_processor(self, mut receiver: mpsc::UnboundedReceiver<HealthAlert>) {
        while let Some(alert) = receiver.recv().await {
            match alert.severity {
                AlertSeverity::Critical | AlertSeverity::Fatal => {
                    error!("🚨 CRITICAL ALERT: {} - {}", alert.title, alert.description);
                },
                AlertSeverity::Warning => {
                    warn!("⚠️ WARNING: {} - {}", alert.title, alert.description);
                },
                AlertSeverity::Info => {
                    info!("ℹ️ INFO: {} - {}", alert.title, alert.description);
                },
            }
            
            // Здесь можно добавить отправку уведомлений (email, Slack, etc.)
        }
    }
}

impl Clone for HealthMonitor {
    fn clone(&self) -> Self {
        Self {
            component_stats: Arc::clone(&self.component_stats),
            metrics_history: Arc::clone(&self.metrics_history),
            active_alerts: Arc::clone(&self.active_alerts),
            alert_sender: None, // Не клонируем sender для избежания проблем
            start_time: self.start_time,
            config: self.config.clone(),
        }
    }
}

/// Convenience макрос для создания метрики
#[macro_export]
macro_rules! health_metric {
    ($component:expr, $name:expr, $value:expr, $unit:expr) => {
        crate::health::HealthMetric {
            component: $component,
            metric_name: $name.to_string(),
            value: $value,
            unit: $unit.to_string(),
            timestamp: chrono::Utc::now(),
            threshold_warning: None,
            threshold_critical: None,
        }
    };
    ($component:expr, $name:expr, $value:expr, $unit:expr, $warn:expr, $crit:expr) => {
        crate::health::HealthMetric {
            component: $component,
            metric_name: $name.to_string(),
            value: $value,
            unit: $unit.to_string(),
            timestamp: chrono::Utc::now(),
            threshold_warning: Some($warn),
            threshold_critical: Some($crit),
        }
    };
}