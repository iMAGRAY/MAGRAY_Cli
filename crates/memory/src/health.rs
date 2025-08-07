use anyhow::Result;
use chrono::{DateTime, Utc};
use common::service_traits::ConfigurationProfile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

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
    Disk,
    Network,
    Api,
}

impl ComponentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComponentType::VectorStore => "vector_store",
            ComponentType::EmbeddingService => "embedding_service",
            ComponentType::RerankingService => "reranking_service",
            ComponentType::PromotionEngine => "promotion_engine",
            ComponentType::Cache => "cache",
            ComponentType::Database => "database",
            ComponentType::Memory => "memory",
            ComponentType::Disk => "disk",
            ComponentType::Network => "network",
            ComponentType::Api => "api",
        }
    }
}

/// Health статус компонента
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Default for SystemHealthStatus {
    fn default() -> Self {
        Self {
            overall_status: HealthStatus::Healthy,
            component_statuses: HashMap::new(),
            active_alerts: Vec::new(),
            metrics_summary: HashMap::new(),
            last_updated: Utc::now(),
            uptime_seconds: 0,
        }
    }
}

/// Статистика производительности компонента
#[derive(Debug, Clone)]
pub struct ComponentPerformanceStats {
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub last_error: Option<String>,
    pub last_error_time: Option<DateTime<Utc>>,
}

/// Конфигурация health monitor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthMonitorConfig {
    pub enable_alerts: bool,
    pub metrics_retention_days: u32,
    pub alert_thresholds: HashMap<ComponentType, f64>,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            enable_alerts: true,
            metrics_retention_days: 7,
            alert_thresholds: HashMap::new(),
        }
    }
}

impl ConfigurationProfile<HealthMonitorConfig> for HealthMonitorConfig {
    fn production() -> Self {
        let mut alert_thresholds = HashMap::new();
        alert_thresholds.insert(ComponentType::Memory, 0.9);
        alert_thresholds.insert(ComponentType::Disk, 0.85);
        alert_thresholds.insert(ComponentType::Network, 0.95);
        alert_thresholds.insert(ComponentType::Api, 0.99);

        Self {
            enable_alerts: true,
            metrics_retention_days: 30,
            alert_thresholds,
        }
    }

    fn minimal() -> Self {
        Self {
            enable_alerts: false,
            metrics_retention_days: 1,
            alert_thresholds: HashMap::new(),
        }
    }

    fn validate_profile(config: &HealthMonitorConfig) -> Result<(), common::ConfigError> {
        if config.metrics_retention_days == 0 {
            return Err(common::ConfigError::InvalidValue {
                config_key: "metrics_retention_days".to_string(),
                value: "0".to_string(),
                reason: "Metrics retention days must be greater than 0".to_string(),
            });
        }
        Ok(())
    }
}

pub struct HealthMonitor {
    component_stats: Arc<RwLock<HashMap<ComponentType, ComponentPerformanceStats>>>,
    metrics_history: Arc<RwLock<HashMap<String, VecDeque<HealthMetric>>>>,
    active_alerts: Arc<RwLock<HashMap<String, HealthAlert>>>,
    alert_sender: Option<mpsc::UnboundedSender<HealthAlert>>,
    start_time: Instant,
    config: HealthMonitorConfig,
}

impl HealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            component_stats: Arc::new(RwLock::new(HashMap::new())),
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_sender: None,
            start_time: Instant::now(),
            config,
        }
    }

    /// Записывает операционную статистику компонента
    pub fn record_operation(
        &self,
        component: ComponentType,
        success: bool,
        response_time_ms: f64,
        error: Option<String>,
    ) {
        let mut stats = match self.component_stats.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Component stats lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };
        let component_stats = stats
            .entry(component)
            .or_insert_with(|| ComponentPerformanceStats {
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
    pub fn get_component_metrics(
        &self,
        component: ComponentType,
        metric_name: &str,
        limit: Option<usize>,
    ) -> Vec<HealthMetric> {
        let metric_key = format!("{component:?}_{metric_name}");
        let history = match self.metrics_history.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Metrics history read lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };

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
    pub fn get_component_performance(
        &self,
        component: ComponentType,
    ) -> Option<ComponentPerformanceStats> {
        let stats = match self.component_stats.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Component stats read lock poisoned in get_component_performance, recovering data");
                poisoned.into_inner()
            }
        };
        stats.get(&component).cloned()
    }

    /// Создает custom alert
    pub fn create_alert(
        &self,
        component: ComponentType,
        severity: AlertSeverity,
        title: String,
        description: String,
    ) {
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
        let mut alerts = match self.active_alerts.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Active alerts write lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };
        alerts.insert(alert.id.clone(), alert);
    }

    /// Разрешает alert
    pub fn resolve_alert(&self, alert_id: &str) {
        let mut alerts = match self.active_alerts.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Active alerts write lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };
        if let Some(alert) = alerts.get_mut(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(Utc::now());
            info!("Alert resolved: {}", alert_id);
        }
    }

    /// Алиас для get_system_health для координатора
    pub async fn overall_health(&self) -> Result<SystemHealthStatus> {
        Ok(self.get_system_health())
    }

    /// Выполнить проверку здоровья системы
    pub async fn check_health(&self) -> Result<()> {
        // Запускаем базовую проверку всех компонентов
        let _health = self.get_system_health();
        // В реальной ситуации здесь были бы активные проверки
        Ok(())
    }

    /// Записывает метрику в историю для мониторинга
    pub fn record_metric(&self, metric: HealthMetric) -> Result<()> {
        let mut history = match self.metrics_history.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Metrics history write lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };

        let key = format!("{}:{}", metric.component.as_str(), metric.metric_name);
        let queue = history.entry(key).or_insert_with(VecDeque::new);

        // Ограничиваем размер истории (например, последние 1000 метрик)
        if queue.len() >= 1000 {
            queue.pop_front();
        }

        queue.push_back(metric);
        Ok(())
    }

    /// Вычисляет статусы компонентов на основе производительности
    fn calculate_component_statuses(&self) -> HashMap<ComponentType, HealthStatus> {
        let mut statuses = HashMap::new();
        let stats = match self.component_stats.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Component stats read lock poisoned in calculate_component_statuses, recovering data");
                poisoned.into_inner()
            }
        };

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
    fn calculate_overall_status(
        &self,
        component_statuses: &HashMap<ComponentType, HealthStatus>,
    ) -> HealthStatus {
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
                HealthStatus::Healthy => {}
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
        let alerts = match self.active_alerts.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!("Active alerts read lock poisoned, recovering data");
                poisoned.into_inner()
            }
        };
        alerts
            .values()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }

    /// Получает сводку метрик
    fn get_metrics_summary(&self) -> HashMap<String, f64> {
        let mut summary = HashMap::new();
        let history = match self.metrics_history.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                error!(
                    "Metrics history read lock poisoned in get_metrics_summary, recovering data"
                );
                poisoned.into_inner()
            }
        };

        for (metric_key, metrics) in history.iter() {
            if let Some(latest) = metrics.back() {
                summary.insert(metric_key.clone(), latest.value);
            }
        }

        summary
    }

    /// Обработчик alerts (запускается в фоне)
    #[allow(dead_code)] // Для будущего background processing
    async fn alert_processor(self, mut receiver: mpsc::UnboundedReceiver<HealthAlert>) {
        while let Some(alert) = receiver.recv().await {
            match alert.severity {
                AlertSeverity::Critical | AlertSeverity::Fatal => {
                    error!("🚨 CRITICAL ALERT: {} - {}", alert.title, alert.description);
                }
                AlertSeverity::Warning => {
                    warn!("⚠️ WARNING: {} - {}", alert.title, alert.description);
                }
                AlertSeverity::Info => {
                    info!("ℹ️ INFO: {} - {}", alert.title, alert.description);
                }
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
        $crate::health::HealthMetric {
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
        $crate::health::HealthMetric {
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
