use anyhow::Result;
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};
use tracing::{debug, info, warn, error};
use tokio::sync::RwLock;

use crate::{
    health::{HealthMonitor, SystemHealthStatus, HealthStatus},
    orchestration::traits::{Coordinator, HealthCoordinator},
};

/// Production-ready менеджер здоровья системы с comprehensive monitoring
pub struct HealthManager {
    health_monitor: Arc<HealthMonitor>,
    ready: std::sync::atomic::AtomicBool,
    
    // === Production Health Metrics ===
    /// Метрики производительности и доступности
    health_metrics: Arc<RwLock<HealthMetrics>>,
    /// SLA метрики (доступность, задержка, ошибки)
    sla_metrics: Arc<RwLock<SlaMetrics>>,
    /// Активные алерты
    active_alerts: Arc<RwLock<Vec<Alert>>>,
    /// Компоненты для мониторинга
    monitored_components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    /// История проверок здоровья
    health_history: Arc<RwLock<Vec<HealthCheckResult>>>,
}

/// Метрики здоровья системы
#[derive(Debug, Default)]
struct HealthMetrics {
    total_checks: u64,
    successful_checks: u64,
    failed_checks: u64,
    avg_check_duration_ms: f64,
    uptime_seconds: u64,
    last_check_timestamp: Option<Instant>,
    system_load: f64,
    memory_usage_percent: f64,
    disk_usage_percent: f64,
}

/// SLA метрики
#[derive(Debug, Default)]
struct SlaMetrics {
    availability_percent: f64,
    avg_response_time_ms: f64,
    error_rate_percent: f64,
    mttr_minutes: f64, // Mean Time To Recovery
    mtbf_hours: f64,   // Mean Time Between Failures
    sla_violations: u64,
}

/// Алерт
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Alert {
    id: String,
    component: String,
    level: AlertLevel,
    message: String,
    timestamp: Instant,
    resolved: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum AlertLevel {
    Info,
    Warning,
    Critical,
}

/// Здоровье компонента
#[derive(Debug, Clone)]
struct ComponentHealth {
    name: String,
    status: HealthStatus,
    last_check: Instant,
    check_interval: Duration,
    consecutive_failures: u32,
    total_checks: u64,
    successful_checks: u64,
}

/// Результат проверки здоровья
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct HealthCheckResult {
    timestamp: Instant,
    overall_healthy: bool,
    component_count: usize,
    healthy_components: usize,
    check_duration_ms: f64,
}

impl HealthManager {
    pub fn new(health_monitor: Arc<HealthMonitor>) -> Self {
        Self {
            health_monitor,
            ready: std::sync::atomic::AtomicBool::new(false),
            health_metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            sla_metrics: Arc::new(RwLock::new(SlaMetrics::default())),
            active_alerts: Arc::new(RwLock::new(Vec::new())),
            monitored_components: Arc::new(RwLock::new(HashMap::new())),
            health_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Настройка production monitoring
    pub async fn setup_production_monitoring(&self) -> Result<()> {
        info!("🎨 Настройка production health monitoring...");
        
        // Регистрируем ключевые компоненты
        self.register_component("vector_store", Duration::from_secs(30)).await;
        self.register_component("embedding_service", Duration::from_secs(30)).await;
        self.register_component("search_coordinator", Duration::from_secs(60)).await;
        self.register_component("cache_system", Duration::from_secs(60)).await;
        self.register_component("gpu_processor", Duration::from_secs(120)).await;
        
        // Запускаем периодические проверки
        self.start_health_monitoring_loop().await;
        self.start_sla_monitoring_loop().await;
        self.start_alert_processor().await;
        
        Ok(())
    }
}

#[async_trait]
impl Coordinator for HealthManager {
    async fn initialize(&self) -> Result<()> {
        info!("🚑 Инициализация production HealthManager...");
        
        // Запускаем начальную проверку здоровья
        self.run_health_check().await?;
        
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("✅ HealthManager готов к production monitoring");
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        let health_status = self.system_health().await.unwrap_or_default();
        let health_metrics = self.health_metrics.read().await;
        let sla_metrics = self.sla_metrics.read().await;
        let alerts = self.get_alerts().await;
        let components = self.monitored_components.read().await;
        
        // Сбор статистики по компонентам
        let healthy_components = components.values().filter(|c| matches!(c.status, HealthStatus::Healthy)).count();
        let degraded_components = components.values().filter(|c| matches!(c.status, HealthStatus::Degraded)).count();
        let unhealthy_components = components.values().filter(|c| matches!(c.status, HealthStatus::Unhealthy)).count();
        
        // Считаем алерты по уровням
        let critical_alerts = alerts.iter().filter(|a| a.contains("Критический")).count();
        let warning_alerts = alerts.iter().filter(|a| a.contains("Предупреждение")).count();
        
        serde_json::json!({
            "ready": self.is_ready().await,
            "type": "health_manager",
            "system_health": {
                "overall_healthy": health_status.overall_status,
                "components_count": components.len(),
                "healthy_components": healthy_components,
                "degraded_components": degraded_components,
                "unhealthy_components": unhealthy_components,
                "last_updated": health_status.last_updated.to_rfc3339()
            },
            "performance_metrics": {
                "total_checks": health_metrics.total_checks,
                "successful_checks": health_metrics.successful_checks,
                "failed_checks": health_metrics.failed_checks,
                "success_rate_percent": if health_metrics.total_checks > 0 {
                    (health_metrics.successful_checks as f64 / health_metrics.total_checks as f64) * 100.0
                } else { 0.0 },
                "avg_check_duration_ms": health_metrics.avg_check_duration_ms,
                "uptime_seconds": health_metrics.uptime_seconds,
                "system_load": health_metrics.system_load,
                "memory_usage_percent": health_metrics.memory_usage_percent,
                "disk_usage_percent": health_metrics.disk_usage_percent
            },
            "sla_metrics": {
                "availability_percent": sla_metrics.availability_percent,
                "avg_response_time_ms": sla_metrics.avg_response_time_ms,
                "error_rate_percent": sla_metrics.error_rate_percent,
                "mttr_minutes": sla_metrics.mttr_minutes,
                "mtbf_hours": sla_metrics.mtbf_hours,
                "sla_violations": sla_metrics.sla_violations
            },
            "alerts": {
                "active_count": alerts.len(),
                "critical_count": critical_alerts,
                "warning_count": warning_alerts,
                "info_count": alerts.len() - critical_alerts - warning_alerts,
                "alerts": alerts
            },
            "component_details": components.values().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "status": format!("{:?}", c.status),
                    "last_check_seconds_ago": c.last_check.elapsed().as_secs(),
                    "consecutive_failures": c.consecutive_failures,
                    "success_rate_percent": if c.total_checks > 0 {
                        (c.successful_checks as f64 / c.total_checks as f64) * 100.0
                    } else { 0.0 }
                })
            }).collect::<Vec<_>>()
        })
    }
}

#[async_trait]
impl HealthCoordinator for HealthManager {
    async fn system_health(&self) -> Result<SystemHealthStatus> {
        self.health_monitor.overall_health().await
    }
    
    async fn component_health(&self, component: &str) -> Result<bool> {
        let components = self.monitored_components.read().await;
        
        if let Some(comp_health) = components.get(component) {
            Ok(matches!(comp_health.status, HealthStatus::Healthy))
        } else {
            warn!("Компонент '{}' не найден в мониторинге", component);
            Ok(false)
        }
    }
    
    async fn run_health_check(&self) -> Result<()> {
        let start_time = Instant::now();
        let health_result = self.health_monitor.overall_health().await;
        let check_duration = start_time.elapsed();
        
        // Обновляем метрики
        let mut metrics = self.health_metrics.write().await;
        metrics.total_checks += 1;
        
        match &health_result {
            Ok(status) => {
                metrics.successful_checks += 1;
                
                // Обновляем статусы компонентов
                let mut components = self.monitored_components.write().await;
                for (comp_type, comp_status) in &status.component_statuses {
                    let comp_name = format!("{:?}", comp_type);
                    if let Some(comp_health) = components.get_mut(&comp_name) {
                        comp_health.status = comp_status.clone();
                        comp_health.last_check = Instant::now();
                        comp_health.total_checks += 1;
                        
                        if matches!(comp_status, HealthStatus::Healthy) {
                            comp_health.successful_checks += 1;
                            comp_health.consecutive_failures = 0;
                        } else {
                            comp_health.consecutive_failures += 1;
                            
                            // Создаём алерт при необходимости
                            if comp_health.consecutive_failures >= 3 {
                                self.create_alert(
                                    &comp_name,
                                    AlertLevel::Critical,
                                    format!("Компонент {} недоступен ({} последовательных ошибок)", comp_name, comp_health.consecutive_failures)
                                ).await;
                            } else if comp_health.consecutive_failures == 1 {
                                self.create_alert(
                                    &comp_name,
                                    AlertLevel::Warning,
                                    format!("Компонент {} показывает проблемы", comp_name)
                                ).await;
                            }
                        }
                    }
                }
                
                // Сохраняем в историю
                let mut history = self.health_history.write().await;
                history.push(HealthCheckResult {
                    timestamp: Instant::now(),
                    overall_healthy: matches!(status.overall_status, HealthStatus::Healthy),
                    component_count: status.component_statuses.len(),
                    healthy_components: status.component_statuses.values().filter(|s| matches!(s, HealthStatus::Healthy)).count(),
                    check_duration_ms: check_duration.as_millis() as f64,
                });
                
                // Ограничиваем размер истории (100 последних проверок)
                if history.len() > 100 {
                    history.remove(0);
                }
            }
            Err(_) => {
                metrics.failed_checks += 1;
                self.create_alert(
                    "health_monitor",
                    AlertLevel::Critical,
                    "Ошибка выполнения проверки здоровья".to_string()
                ).await;
            }
        }
        
        // Обновляем среднюю продолжительность
        let alpha = 0.1;
        if metrics.avg_check_duration_ms == 0.0 {
            metrics.avg_check_duration_ms = check_duration.as_millis() as f64;
        } else {
            metrics.avg_check_duration_ms = alpha * check_duration.as_millis() as f64 + (1.0 - alpha) * metrics.avg_check_duration_ms;
        }
        
        metrics.last_check_timestamp = Some(Instant::now());
        
        health_result.map(|_| ())
    }
    
    async fn get_alerts(&self) -> Vec<String> {
        let alerts = self.active_alerts.read().await;
        alerts.iter()
            .filter(|alert| !alert.resolved)
            .map(|alert| {
                format!("{:?}: {} - {} ({})", 
                    alert.level, 
                    alert.component, 
                    alert.message,
                    format_duration(alert.timestamp.elapsed())
                )
            })
            .collect()
    }
    
    async fn clear_alerts(&self) -> Result<()> {
        let mut alerts = self.active_alerts.write().await;
        let cleared_count = alerts.len();
        
        // Помечаем все алерты как разрешённые
        for alert in alerts.iter_mut() {
            alert.resolved = true;
        }
        
        info!("✅ Очищено {} алертов", cleared_count);
        Ok(())
    }
}

impl HealthManager {
    /// Вспомогательные методы для production monitoring
    
    /// Регистрация компонента для мониторинга
    async fn register_component(&self, name: &str, check_interval: Duration) {
        let mut components = self.monitored_components.write().await;
        components.insert(name.to_string(), ComponentHealth {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            last_check: Instant::now(),
            check_interval,
            consecutive_failures: 0,
            total_checks: 0,
            successful_checks: 0,
        });
        
        debug!("📈 Зарегистрирован компонент '{}' с интервалом {:?}", name, check_interval);
    }
    
    /// Создание алерта
    async fn create_alert(&self, component: &str, level: AlertLevel, message: String) {
        let alert = Alert {
            id: format!("alert_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
            component: component.to_string(),
            level: level.clone(),
            message,
            timestamp: Instant::now(),
            resolved: false,
        };
        
        let mut alerts = self.active_alerts.write().await;
        alerts.push(alert.clone());
        
        // Ограничиваем количество алертов (100 макс)
        if alerts.len() > 100 {
            alerts.remove(0);
        }
        
        let level_emoji = match level {
            AlertLevel::Info => "📝",
            AlertLevel::Warning => "⚠️",
            AlertLevel::Critical => "🚨",
        };
        
        warn!("{} Новый алерт: {} - {}", level_emoji, alert.component, alert.message);
    }
    
    /// Запуск health monitoring loop
    async fn start_health_monitoring_loop(&self) {
        let health_monitor = self.health_monitor.clone();
        let _components = self.monitored_components.clone();
        let metrics = self.health_metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Проверяем общее здоровье системы
                if let Err(e) = health_monitor.check_health().await {
                    error!("Ошибка проверки здоровья: {}", e);
                }
                
                // Обновляем uptime
                {
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.uptime_seconds += 30;
                }
                
                debug!("📈 Health monitoring цикл завершён");
            }
        });
        
        debug!("📈 Health monitoring loop запущен");
    }
    
    /// Запуск SLA monitoring loop
    async fn start_sla_monitoring_loop(&self) {
        let sla_metrics = self.sla_metrics.clone();
        let health_history = self.health_history.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Каждые 5 минут
            
            loop {
                interval.tick().await;
                
                let history = health_history.read().await;
                if history.len() > 10 {
                    let mut sla_guard = sla_metrics.write().await;
                    
                    // Считаем availability
                    let healthy_checks = history.iter().filter(|h| h.overall_healthy).count();
                    sla_guard.availability_percent = (healthy_checks as f64 / history.len() as f64) * 100.0;
                    
                    // Считаем среднее время отклика
                    let total_duration: f64 = history.iter().map(|h| h.check_duration_ms).sum();
                    sla_guard.avg_response_time_ms = total_duration / history.len() as f64;
                    
                    // Обновляем error rate
                    let error_checks = history.iter().filter(|h| !h.overall_healthy).count();
                    sla_guard.error_rate_percent = (error_checks as f64 / history.len() as f64) * 100.0;
                    
                    // Проверяем SLA нарушения
                    if sla_guard.availability_percent < 99.9 {
                        sla_guard.sla_violations += 1;
                        warn!("🚨 SLA нарушение: availability {:.2}%", sla_guard.availability_percent);
                    }
                    
                    if sla_guard.avg_response_time_ms > 1000.0 {
                        sla_guard.sla_violations += 1;
                        warn!("🚨 SLA нарушение: response time {:.2}ms", sla_guard.avg_response_time_ms);
                    }
                }
                
                debug!("📈 SLA monitoring цикл завершён");
            }
        });
        
        debug!("📈 SLA monitoring loop запущен");
    }
    
    /// Запуск alert processor
    async fn start_alert_processor(&self) {
        let active_alerts = self.active_alerts.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Каждую минуту
            
            loop {
                interval.tick().await;
                
                let mut alerts = active_alerts.write().await;
                
                // Очищаем старые resolved алерты (>часа)
                alerts.retain(|alert| {
                    if alert.resolved && alert.timestamp.elapsed() > Duration::from_secs(3600) {
                        false
                    } else {
                        true
                    }
                });
                
                // Проверяем критические алерты
                let critical_count = alerts.iter().filter(|a| !a.resolved && matches!(a.level, AlertLevel::Critical)).count();
                if critical_count > 0 {
                    error!("🚨 {} активных критических алертов!", critical_count);
                }
                
                debug!("🔔 Alert processor: {} активных алертов", alerts.iter().filter(|a| !a.resolved).count());
            }
        });
        
        debug!("🔔 Alert processor запущен");
    }
    
    /// Получить подробную статистику по компоненту
    pub async fn get_component_detailed_stats(&self, component: &str) -> Option<serde_json::Value> {
        let components = self.monitored_components.read().await;
        
        if let Some(comp_health) = components.get(component) {
            Some(serde_json::json!({
                "name": comp_health.name,
                "status": format!("{:?}", comp_health.status),
                "last_check_duration": format_duration(comp_health.last_check.elapsed()),
                "check_interval_seconds": comp_health.check_interval.as_secs(),
                "consecutive_failures": comp_health.consecutive_failures,
                "total_checks": comp_health.total_checks,
                "successful_checks": comp_health.successful_checks,
                "failure_rate_percent": if comp_health.total_checks > 0 {
                    ((comp_health.total_checks - comp_health.successful_checks) as f64 / comp_health.total_checks as f64) * 100.0
                } else { 0.0 },
                "is_overdue": comp_health.last_check.elapsed() > comp_health.check_interval * 2
            }))
        } else {
            None
        }
    }
}

/// Форматирование duration для вывода
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}