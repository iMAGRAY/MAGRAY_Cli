use anyhow::Result;
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::VecDeque,
};
use tracing::{debug, info, warn};
use tokio::sync::RwLock;

use crate::{
    resource_manager::{ResourceManager, ResourceUsage},
    orchestration::traits::{Coordinator, ResourceCoordinator},
};

/// Production-ready контроллер ресурсов с автомасштабированием
#[derive(Debug)]
pub struct ResourceController {
    resource_manager: Arc<parking_lot::RwLock<ResourceManager>>,
    ready: std::sync::atomic::AtomicBool,
    
    // === Auto-scaling Infrastructure ===
    /// Метрики использования ресурсов
    resource_metrics: Arc<RwLock<ResourceMetrics>>,
    /// Конфигурация автомасштабирования
    scaling_config: Arc<RwLock<ScalingConfig>>,
    /// История использования ресурсов
    usage_history: Arc<RwLock<VecDeque<ResourceSnapshot>>>,
    /// Прогноз нагрузки
    load_predictor: Arc<RwLock<LoadPredictor>>,
    /// Алерты по ресурсам
    resource_alerts: Arc<RwLock<Vec<ResourceAlert>>>,
}

/// Метрики использования ресурсов
#[derive(Debug, Default)]
struct ResourceMetrics {
    peak_memory_usage: f64,
    avg_memory_usage: f64,
    peak_vector_count: usize,
    avg_vector_count: f64,
    cache_hit_rate: f64,
    gc_frequency: f64,
    scaling_events: u64,
    last_scaling_event: Option<Instant>,
}

/// Конфигурация автомасштабирования
#[derive(Debug)]
struct ScalingConfig {
    memory_scale_up_threshold: f64,   // 80%
    memory_scale_down_threshold: f64, // 40%
    vector_scale_up_threshold: f64,   // 85%
    vector_scale_down_threshold: f64, // 30%
    scale_up_cooldown: Duration,      // 5 мин
    scale_down_cooldown: Duration,    // 15 мин
    aggressive_scaling: bool,
    predictive_scaling: bool,
}

/// Снимок состояния ресурсов
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ResourceSnapshot {
    timestamp: Instant,
    memory_usage_percent: f64,
    vector_usage_percent: f64,
    cache_usage_percent: f64,
    operations_per_second: f64,
}

/// Прогноз нагрузки
#[derive(Debug, Default)]
struct LoadPredictor {
    trend_memory: f64,
    trend_vectors: f64,
    seasonal_factor: f64,
    prediction_confidence: f64,
}

/// Алерт по ресурсам
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ResourceAlert {
    id: String,
    alert_type: ResourceAlertType,
    message: String,
    timestamp: Instant,
    resolved: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum ResourceAlertType {
    MemoryPressure,
    VectorLimitApproached,
    CacheOverflow,
    ScalingEvent,
    ResourceExhaustion,
}

impl ResourceController {
    pub fn new(resource_manager: Arc<parking_lot::RwLock<ResourceManager>>) -> Self {
        let scaling_config = ScalingConfig {
            memory_scale_up_threshold: 80.0,
            memory_scale_down_threshold: 40.0,
            vector_scale_up_threshold: 85.0,
            vector_scale_down_threshold: 30.0,
            scale_up_cooldown: Duration::from_secs(300),   // 5 мин
            scale_down_cooldown: Duration::from_secs(900), // 15 мин
            aggressive_scaling: false,
            predictive_scaling: true,
        };
        
        Self {
            resource_manager,
            ready: std::sync::atomic::AtomicBool::new(false),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            scaling_config: Arc::new(RwLock::new(scaling_config)),
            usage_history: Arc::new(RwLock::new(VecDeque::with_capacity(288))), // 24 часа по 5 мин
            load_predictor: Arc::new(RwLock::new(LoadPredictor::default())),
            resource_alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Production конфигурация для высокой нагрузки
    pub fn new_production(resource_manager: Arc<parking_lot::RwLock<ResourceManager>>) -> Self {
        let scaling_config = ScalingConfig {
            memory_scale_up_threshold: 75.0,    // Более агрессивно
            memory_scale_down_threshold: 35.0,
            vector_scale_up_threshold: 80.0,
            vector_scale_down_threshold: 25.0,
            scale_up_cooldown: Duration::from_secs(180),   // 3 мин
            scale_down_cooldown: Duration::from_secs(600), // 10 мин
            aggressive_scaling: true,
            predictive_scaling: true,
        };
        
        Self {
            resource_manager,
            ready: std::sync::atomic::AtomicBool::new(false),
            resource_metrics: Arc::new(RwLock::new(ResourceMetrics::default())),
            scaling_config: Arc::new(RwLock::new(scaling_config)),
            usage_history: Arc::new(RwLock::new(VecDeque::with_capacity(288))),
            load_predictor: Arc::new(RwLock::new(LoadPredictor::default())),
            resource_alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Запуск auto-scaling мониторинга
    pub async fn start_autoscaling_monitoring(&self) -> Result<()> {
        info!("🎯 Запуск auto-scaling monitoring...");
        
        // Запускаем мониторинг ресурсов
        self.start_resource_monitoring_loop().await;
        
        // Запускаем auto-scaling логику
        self.start_autoscaling_loop().await;
        
        // Запускаем predictive scaling
        self.start_predictive_scaling_loop().await;
        
        // Запускаем alert processing
        self.start_alert_processing_loop().await;
        
        info!("✅ Auto-scaling monitoring запущен");
        Ok(())
    }
}

#[async_trait]
impl Coordinator for ResourceController {
    async fn initialize(&self) -> Result<()> {
        info!("🔧 Инициализация production ResourceController...");
        
        // Проверяем начальное состояние ресурсов
        let initial_usage = self.resource_usage().await;
        info!("📈 Начальное состояние: memory={}%, vectors={}/{}", 
              initial_usage.cache_usage_percent,
              initial_usage.current_vectors,
              initial_usage.max_vectors);
        
        // Создаём первый snapshot
        self.record_resource_snapshot().await;
        
        // Запускаем auto-scaling monitoring
        self.start_autoscaling_monitoring().await?;
        
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("✅ ResourceController готов к production работе");
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
        let usage = self.resource_usage().await;
        let metrics = self.resource_metrics.read().await;
        let scaling_config = self.scaling_config.read().await;
        let alerts = self.resource_alerts.read().await;
        let predictor = self.load_predictor.read().await;
        
        // Считаем активные алерты
        let active_alerts: Vec<_> = alerts.iter()
            .filter(|alert| !alert.resolved)
            .map(|alert| format!("{:?}: {} ({})", 
                alert.alert_type,
                alert.message,
                format_elapsed(alert.timestamp.elapsed())
            ))
            .collect();
        
        serde_json::json!({
            "ready": self.is_ready().await,
            "type": "resource_controller",
            "current_usage": {
                "vectors": {
                    "current": usage.current_vectors,
                    "max": usage.max_vectors,
                    "usage_percent": usage.vector_usage_percent
                },
                "cache": {
                    "current_mb": usage.current_cache_size / 1024 / 1024,
                    "max_mb": usage.max_cache_size / 1024 / 1024,
                    "usage_percent": usage.cache_usage_percent
                }
            },
            "performance_metrics": {
                "peak_memory_usage": metrics.peak_memory_usage,
                "avg_memory_usage": metrics.avg_memory_usage,
                "peak_vector_count": metrics.peak_vector_count,
                "avg_vector_count": metrics.avg_vector_count,
                "cache_hit_rate": metrics.cache_hit_rate,
                "gc_frequency": metrics.gc_frequency,
                "scaling_events": metrics.scaling_events
            },
            "autoscaling": {
                "memory_thresholds": {
                    "scale_up": scaling_config.memory_scale_up_threshold,
                    "scale_down": scaling_config.memory_scale_down_threshold
                },
                "vector_thresholds": {
                    "scale_up": scaling_config.vector_scale_up_threshold,
                    "scale_down": scaling_config.vector_scale_down_threshold
                },
                "cooldowns": {
                    "scale_up_seconds": scaling_config.scale_up_cooldown.as_secs(),
                    "scale_down_seconds": scaling_config.scale_down_cooldown.as_secs()
                },
                "aggressive_scaling": scaling_config.aggressive_scaling,
                "predictive_scaling": scaling_config.predictive_scaling,
                "last_scaling_event": metrics.last_scaling_event.map(|t| format_elapsed(t.elapsed()))
            },
            "predictions": {
                "memory_trend": predictor.trend_memory,
                "vector_trend": predictor.trend_vectors,
                "seasonal_factor": predictor.seasonal_factor,
                "confidence": predictor.prediction_confidence
            },
            "alerts": {
                "active_count": active_alerts.len(),
                "total_alerts": alerts.len(),
                "active_alerts": active_alerts
            }
        })
    }
}

#[async_trait]
impl ResourceCoordinator for ResourceController {
    async fn resource_usage(&self) -> ResourceUsage {
        let manager = self.resource_manager.read();
        manager.current_usage()
    }
    
    async fn check_resources(&self, _operation: &str) -> Result<bool> {
        let mut manager = self.resource_manager.write();
        Ok(!manager.is_memory_pressure())
    }
    
    async fn adapt_limits(&self) -> Result<()> {
        let mut manager = self.resource_manager.write();
        manager.adapt_limits();
        Ok(())
    }
    
    async fn free_resources(&self) -> Result<()> {
        info!("🧹 Агрессивное освобождение ресурсов...");
        
        let freed_memory: u64;
        let freed_vectors = 0usize;
        
        {
            let mut manager = self.resource_manager.write();
            
            // Принудительно очищаем cache
            let cache_before = manager.current_usage().current_cache_size;
            
            // Уменьшаем лимиты на 20% для принудительной очистки
            let current_limits = manager.get_current_limits();
            let temp_limits = crate::resource_manager::ResourceLimits {
                max_vectors: (current_limits.max_vectors as f64 * 0.8) as usize,
                cache_size_bytes: (current_limits.cache_size_bytes as f64 * 0.8) as usize,
                last_scaled: current_limits.last_scaled,
                scaling_factor: current_limits.scaling_factor * 0.8,
            };
            
            // Применяем новые лимиты (force cleanup)
            manager.apply_limits(temp_limits);
            
            // Возвращаем оригинальные лимиты
            manager.apply_limits(current_limits);
            
            let cache_after = manager.current_usage().current_cache_size;
            freed_memory = cache_before.saturating_sub(cache_after) as u64;
        }
        
        // Создаём алерт о принудительном освобождении
        self.create_alert(
            ResourceAlertType::ResourceExhaustion,
            format!("Принудительно освобождено: {:.1}MB memory, {} vectors", 
                    freed_memory as f64 / 1024.0 / 1024.0, freed_vectors)
        ).await;
        
        info!("✅ Освобождено: {:.1}MB memory", freed_memory as f64 / 1024.0 / 1024.0);
        Ok(())
    }
    
    async fn get_limits(&self) -> (usize, usize) {
        let manager = self.resource_manager.read();
        let limits = manager.get_current_limits();
        (limits.max_vectors, limits.cache_size_bytes / 1024 / 1024)
    }
}

impl ResourceController {
    /// Вспомогательные методы для auto-scaling
    
    /// Записать снимок текущего состояния ресурсов
    async fn record_resource_snapshot(&self) {
        let usage = self.resource_usage().await;
        
        let snapshot = ResourceSnapshot {
            timestamp: Instant::now(),
            memory_usage_percent: usage.cache_usage_percent,
            vector_usage_percent: usage.vector_usage_percent,
            cache_usage_percent: usage.cache_usage_percent,
            operations_per_second: 0.0, // TODO: получать из метрик
        };
        
        let mut history = self.usage_history.write().await;
        history.push_back(snapshot);
        
        // Ограничиваем размер истории (24 часа по 5 минут)
        if history.len() > 288 {
            history.pop_front();
        }
        
        // Обновляем метрики
        self.update_resource_metrics(&usage).await;
    }
    
    /// Обновить метрики ресурсов
    async fn update_resource_metrics(&self, usage: &ResourceUsage) {
        let mut metrics = self.resource_metrics.write().await;
        
        // Обновляем пиковые значения
        metrics.peak_memory_usage = metrics.peak_memory_usage.max(usage.cache_usage_percent);
        metrics.peak_vector_count = metrics.peak_vector_count.max(usage.current_vectors);
        
        // Обновляем средние значения (exponential moving average)
        let alpha = 0.1;
        metrics.avg_memory_usage = alpha * usage.cache_usage_percent + (1.0 - alpha) * metrics.avg_memory_usage;
        metrics.avg_vector_count = alpha * usage.current_vectors as f64 + (1.0 - alpha) * metrics.avg_vector_count;
    }
    
    /// Создать алерт по ресурсам
    async fn create_alert(&self, alert_type: ResourceAlertType, message: String) {
        let alert = ResourceAlert {
            id: format!("resource_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
            alert_type: alert_type.clone(),
            message: message.clone(),
            timestamp: Instant::now(),
            resolved: false,
        };
        
        let mut alerts = self.resource_alerts.write().await;
        alerts.push(alert);
        
        // Ограничиваем количество алертов
        if alerts.len() > 50 {
            alerts.remove(0);
        }
        
        let emoji = match alert_type {
            ResourceAlertType::MemoryPressure => "🧠",
            ResourceAlertType::VectorLimitApproached => "📊",
            ResourceAlertType::CacheOverflow => "💾",
            ResourceAlertType::ScalingEvent => "📈",
            ResourceAlertType::ResourceExhaustion => "🚨",
        };
        
        warn!("{} Resource alert: {}", emoji, message);
    }
    
    /// Запуск мониторинга ресурсов
    async fn start_resource_monitoring_loop(&self) {
        let resource_manager = self.resource_manager.clone();
        let resource_alerts = self.resource_alerts.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Каждую минуту
            
            loop {
                interval.tick().await;
                
                // Получаем текущее использование ресурсов
                let usage = {
                    let manager = resource_manager.read();
                    manager.current_usage()
                };
                
                // Создаем алерты при высоком использовании
                if usage.cache_usage_percent > 90.0 {
                    let alert = ResourceAlert {
                        id: format!("resource_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                        alert_type: ResourceAlertType::MemoryPressure,
                        message: format!("Критическое использование памяти: {:.1}%", usage.cache_usage_percent),
                        timestamp: Instant::now(),
                        resolved: false,
                    };
                    
                    let mut alerts = resource_alerts.write().await;
                    alerts.push(alert);
                    warn!("🧠 Resource alert: Критическое использование памяти: {:.1}%", usage.cache_usage_percent);
                } else if usage.cache_usage_percent > 85.0 {
                    let alert = ResourceAlert {
                        id: format!("resource_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                        alert_type: ResourceAlertType::MemoryPressure,
                        message: format!("Высокое использование памяти: {:.1}%", usage.cache_usage_percent),
                        timestamp: Instant::now(),
                        resolved: false,
                    };
                    
                    let mut alerts = resource_alerts.write().await;
                    alerts.push(alert);
                    warn!("🧠 Resource alert: Высокое использование памяти: {:.1}%", usage.cache_usage_percent);
                }
                
                if usage.vector_usage_percent > 90.0 {
                    let alert = ResourceAlert {
                        id: format!("resource_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
                        alert_type: ResourceAlertType::VectorLimitApproached,
                        message: format!("Приближение к лимиту векторов: {}/{} ({:.1}%)", 
                                usage.current_vectors, usage.max_vectors, usage.vector_usage_percent),
                        timestamp: Instant::now(),
                        resolved: false,
                    };
                    
                    let mut alerts = resource_alerts.write().await;
                    alerts.push(alert);
                    warn!("📊 Resource alert: Приближение к лимиту векторов: {}/{} ({:.1}%)", 
                         usage.current_vectors, usage.max_vectors, usage.vector_usage_percent);
                }
                
                debug!("📊 Resource monitoring: memory={:.1}%, vectors={}/{}", 
                      usage.cache_usage_percent, usage.current_vectors, usage.max_vectors);
            }
        });
        
        debug!("📊 Resource monitoring loop запущен");
    }
    
    /// Запуск auto-scaling логики
    async fn start_autoscaling_loop(&self) {
        let resource_manager = self.resource_manager.clone();
        let scaling_config = self.scaling_config.clone();
        let resource_metrics = self.resource_metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(180)); // Каждые 3 минуты
            
            loop {
                interval.tick().await;
                
                let config = scaling_config.read().await;
                let mut metrics = resource_metrics.write().await;
                
                // Проверяем cooldown
                if let Some(last_scaling) = metrics.last_scaling_event {
                    if last_scaling.elapsed() < config.scale_up_cooldown {
                        debug!("⏳ Auto-scaling in cooldown");
                        continue;
                    }
                }
                
                let manager = resource_manager.read();
                let usage = manager.current_usage();
                let current_limits = manager.get_current_limits();
                drop(manager);
                
                let mut scaling_needed = false;
                let mut new_limits = current_limits.clone();
                
                // Проверяем необходимость масштабирования памяти
                if usage.cache_usage_percent > config.memory_scale_up_threshold {
                    let scale_factor = if config.aggressive_scaling { 1.5 } else { 1.2 };
                    new_limits.cache_size_bytes = (current_limits.cache_size_bytes as f64 * scale_factor) as usize;
                    scaling_needed = true;
                    
                    info!("📈 Memory scale up: {:.1}% -> {}MB", 
                          usage.cache_usage_percent, new_limits.cache_size_bytes / 1024 / 1024);
                } else if usage.cache_usage_percent < config.memory_scale_down_threshold {
                    let scale_factor = if config.aggressive_scaling { 0.8 } else { 0.9 };
                    new_limits.cache_size_bytes = (current_limits.cache_size_bytes as f64 * scale_factor) as usize;
                    scaling_needed = true;
                    
                    info!("📉 Memory scale down: {:.1}% -> {}MB", 
                          usage.cache_usage_percent, new_limits.cache_size_bytes / 1024 / 1024);
                }
                
                // Проверяем необходимость масштабирования векторов
                if usage.vector_usage_percent > config.vector_scale_up_threshold {
                    let scale_factor = if config.aggressive_scaling { 1.4 } else { 1.3 };
                    new_limits.max_vectors = (current_limits.max_vectors as f64 * scale_factor) as usize;
                    scaling_needed = true;
                    
                    info!("📈 Vector scale up: {:.1}% -> {} vectors", 
                          usage.vector_usage_percent, new_limits.max_vectors);
                } else if usage.vector_usage_percent < config.vector_scale_down_threshold {
                    let scale_factor = if config.aggressive_scaling { 0.7 } else { 0.85 };
                    new_limits.max_vectors = (current_limits.max_vectors as f64 * scale_factor) as usize;
                    scaling_needed = true;
                    
                    info!("📉 Vector scale down: {:.1}% -> {} vectors", 
                          usage.vector_usage_percent, new_limits.max_vectors);
                }
                
                // Применяем масштабирование
                if scaling_needed {
                    let mut manager = resource_manager.write();
                    manager.apply_limits(new_limits);
                    
                    metrics.scaling_events += 1;
                    metrics.last_scaling_event = Some(Instant::now());
                    
                    info!("✅ Auto-scaling завершён (событие #{})", metrics.scaling_events);
                }
                
                debug!("🎯 Auto-scaling check: memory={:.1}%, vectors={:.1}%", 
                       usage.cache_usage_percent, usage.vector_usage_percent);
            }
        });
        
        debug!("🎯 Auto-scaling loop запущен");
    }
    
    /// Запуск predictive scaling
    async fn start_predictive_scaling_loop(&self) {
        let usage_history = self.usage_history.clone();
        let load_predictor = self.load_predictor.clone();
        let scaling_config = self.scaling_config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(900)); // Каждые 15 минут
            
            loop {
                interval.tick().await;
                
                let config = scaling_config.read().await;
                if !config.predictive_scaling {
                    continue;
                }
                
                let history = usage_history.read().await;
                if history.len() < 20 {
                    continue; // Недостаточно данных для прогноза
                }
                
                // Простой линейный тренд
                let recent_samples: Vec<_> = history.iter().rev().take(10).collect();
                
                let memory_values: Vec<f64> = recent_samples.iter().map(|s| s.memory_usage_percent).collect();
                let vector_values: Vec<f64> = recent_samples.iter().map(|s| s.vector_usage_percent).collect();
                
                let memory_trend = calculate_trend(&memory_values);
                let vector_trend = calculate_trend(&vector_values);
                
                {
                    let mut predictor = load_predictor.write().await;
                    predictor.trend_memory = memory_trend;
                    predictor.trend_vectors = vector_trend;
                    predictor.prediction_confidence = if recent_samples.len() >= 10 { 0.8 } else { 0.5 };
                }
                
                // Если тренд показывает рост, можем заблаговременно увеличить лимиты
                if memory_trend > 2.0 || vector_trend > 2.0 {
                    info!("🔮 Predictive scaling: memory_trend={:.1}, vector_trend={:.1}", 
                          memory_trend, vector_trend);
                }
                
                debug!("🔮 Predictive analysis: memory_trend={:.1}, vector_trend={:.1}", 
                       memory_trend, vector_trend);
            }
        });
        
        debug!("🔮 Predictive scaling loop запущен");
    }
    
    /// Запуск обработки алертов
    async fn start_alert_processing_loop(&self) {
        let resource_alerts = self.resource_alerts.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Каждые 5 минут
            
            loop {
                interval.tick().await;
                
                let mut alerts = resource_alerts.write().await;
                
                // Автоматически разрешаем старые алерты (>1 часа)
                for alert in alerts.iter_mut() {
                    if !alert.resolved && alert.timestamp.elapsed() > Duration::from_secs(3600) {
                        alert.resolved = true;
                        debug!("✅ Автоматически разрешён алерт: {}", alert.message);
                    }
                }
                
                // Очищаем разрешённые алерты старше 24 часов
                alerts.retain(|alert| {
                    !(alert.resolved && alert.timestamp.elapsed() > Duration::from_secs(86400))
                });
                
                let active_count = alerts.iter().filter(|a| !a.resolved).count();
                if active_count > 0 {
                    debug!("🚨 Активных алертов по ресурсам: {}", active_count);
                }
            }
        });
        
        debug!("🚨 Alert processing loop запущен");
    }
}

/// Простое вычисление тренда
fn calculate_trend(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    
    let n = values.len() as f64;
    let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
    let sum_x2: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();
    
    // Коэффициент наклона линейной регрессии
    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
    slope
}

/// Форматирование elapsed времени
fn format_elapsed(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}