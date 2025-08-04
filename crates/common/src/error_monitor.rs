use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{error, warn, info};
use crate::errors::{MagrayError, ErrorSeverity};

/// Мониторинг и метрики для ошибок
// @component: {"k":"C","id":"error_monitor","t":"Error monitoring and alerting system","m":{"cur":0,"tgt":95,"u":"%"},"f":["monitoring","errors","alerting"]}
pub struct ErrorMonitor {
    /// Общее количество ошибок
    total_errors: AtomicU64,
    /// Ошибки по типам
    errors_by_type: DashMap<String, AtomicU64>,
    /// Ошибки по severity
    errors_by_severity: DashMap<ErrorSeverity, AtomicU64>,
    /// Rate limiter для алертов
    alert_limiter: Arc<RwLock<RateLimiter>>,
    /// История ошибок для анализа
    error_history: Arc<RwLock<ErrorHistory>>,
    /// Конфигурация
    config: ErrorMonitorConfig,
}

#[derive(Debug, Clone)]
pub struct ErrorMonitorConfig {
    /// Максимальный rate ошибок перед алертом (errors/sec)
    pub error_rate_threshold: f64,
    /// Размер окна для rate limiting (секунды)
    pub rate_window_seconds: u64,
    /// Сколько хранить историю ошибок
    pub history_size: usize,
    /// Минимальный интервал между алертами
    pub alert_cooldown: Duration,
}

impl Default for ErrorMonitorConfig {
    fn default() -> Self {
        Self {
            error_rate_threshold: 10.0, // 10 errors/sec
            rate_window_seconds: 60,     // 1 minute window
            history_size: 1000,
            alert_cooldown: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Rate limiter для отслеживания частоты ошибок
struct RateLimiter {
    window_start: Instant,
    window_duration: Duration,
    count: u64,
    last_alert: Option<Instant>,
}

impl RateLimiter {
    fn new(window_duration: Duration) -> Self {
        Self {
            window_start: Instant::now(),
            window_duration,
            count: 0,
            last_alert: None,
        }
    }

    fn increment(&mut self) -> f64 {
        let now = Instant::now();
        
        // Сброс окна если прошло время
        if now.duration_since(self.window_start) > self.window_duration {
            self.window_start = now;
            self.count = 1;
        } else {
            self.count += 1;
        }
        
        // Вычисляем rate
        let elapsed = now.duration_since(self.window_start).as_secs_f64().max(1.0);
        self.count as f64 / elapsed
    }

    fn should_alert(&self, cooldown: Duration) -> bool {
        match self.last_alert {
            None => true,
            Some(last) => Instant::now().duration_since(last) >= cooldown,
        }
    }

    fn mark_alert(&mut self) {
        self.last_alert = Some(Instant::now());
    }
}

/// История ошибок для анализа паттернов
struct ErrorHistory {
    entries: Vec<ErrorEntry>,
    max_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    error_type: String,
    error_code: String,
    severity: ErrorSeverity,
    message: String,
    context: Option<serde_json::Value>,
}

impl ErrorHistory {
    fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn add(&mut self, error: &MagrayError, context: Option<serde_json::Value>) {
        let entry = ErrorEntry {
            timestamp: chrono::Utc::now(),
            error_type: format!("{:?}", error),
            error_code: error.error_code().to_string(),
            severity: error.severity(),
            message: error.to_string(),
            context,
        };

        self.entries.push(entry);
        
        // Удаляем старые если превышен лимит
        if self.entries.len() > self.max_size {
            self.entries.remove(0);
        }
    }

    fn get_recent(&self, count: usize) -> Vec<&ErrorEntry> {
        let start = self.entries.len().saturating_sub(count);
        self.entries[start..].iter().collect()
    }

    fn analyze_patterns(&self) -> ErrorPatterns {
        let now = chrono::Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        
        let recent_errors: Vec<_> = self.entries.iter()
            .filter(|e| e.timestamp > one_hour_ago)
            .collect();
        
        let mut patterns = ErrorPatterns::default();
        
        // Анализируем частоту по типам
        for entry in &recent_errors {
            *patterns.frequency_by_type
                .entry(entry.error_type.clone())
                .or_insert(0) += 1;
        }
        
        // Находим spike'и
        if recent_errors.len() > 10 {
            let avg_rate = recent_errors.len() as f64 / 60.0; // per minute
            if avg_rate > 1.0 {
                patterns.has_spike = true;
                patterns.spike_severity = recent_errors
                    .iter()
                    .map(|e| e.severity)
                    .max()
                    .unwrap_or(ErrorSeverity::Low);
            }
        }
        
        // Повторяющиеся ошибки
        for (error_type, count) in &patterns.frequency_by_type {
            if *count > 5 {
                patterns.recurring_errors.push(error_type.clone());
            }
        }
        
        patterns
    }
}

#[derive(Debug, Default)]
struct ErrorPatterns {
    frequency_by_type: std::collections::HashMap<String, usize>,
    recurring_errors: Vec<String>,
    has_spike: bool,
    spike_severity: ErrorSeverity,
}

impl ErrorMonitor {
    pub fn new(config: ErrorMonitorConfig) -> Self {
        let rate_limiter = RateLimiter::new(
            Duration::from_secs(config.rate_window_seconds)
        );
        
        Self {
            total_errors: AtomicU64::new(0),
            errors_by_type: DashMap::new(),
            errors_by_severity: DashMap::new(),
            alert_limiter: Arc::new(RwLock::new(rate_limiter)),
            error_history: Arc::new(RwLock::new(ErrorHistory::new(config.history_size))),
            config,
        }
    }

    /// Записать ошибку
    pub fn record_error(&self, error: &MagrayError, context: Option<serde_json::Value>) {
        // Увеличиваем счетчики
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        
        let error_type = format!("{:?}", error);
        self.errors_by_type
            .entry(error_type)
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
        
        self.errors_by_severity
            .entry(error.severity())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
        
        // Добавляем в историю
        {
            let mut history = self.error_history.write();
            history.add(error, context);
        }
        
        // Проверяем rate и severity
        let should_alert = {
            let mut limiter = self.alert_limiter.write();
            let rate = limiter.increment();
            
            let should_alert_rate = rate > self.config.error_rate_threshold;
            let should_alert_severity = error.severity() >= ErrorSeverity::High;
            
            (should_alert_rate || should_alert_severity) 
                && limiter.should_alert(self.config.alert_cooldown)
        };
        
        if should_alert {
            self.send_alert(error);
            self.alert_limiter.write().mark_alert();
        }
        
        // Логируем в зависимости от severity
        match error.severity() {
            ErrorSeverity::Critical => {
                error!(
                    error_code = error.error_code(),
                    severity = ?error.severity(),
                    "CRITICAL ERROR: {}", error
                );
            }
            ErrorSeverity::High => {
                error!(
                    error_code = error.error_code(),
                    severity = ?error.severity(),
                    "HIGH SEVERITY ERROR: {}", error
                );
            }
            ErrorSeverity::Medium => {
                warn!(
                    error_code = error.error_code(),
                    severity = ?error.severity(),
                    "Medium severity error: {}", error
                );
            }
            ErrorSeverity::Low => {
                info!(
                    error_code = error.error_code(),
                    severity = ?error.severity(),
                    "Low severity error: {}", error
                );
            }
        }
    }

    /// Отправить алерт
    fn send_alert(&self, error: &MagrayError) {
        let history = self.error_history.read();
        let patterns = history.analyze_patterns();
        
        error!(
            alert = true,
            error_code = error.error_code(),
            severity = ?error.severity(),
            total_errors = self.total_errors.load(Ordering::Relaxed),
            recurring_errors = ?patterns.recurring_errors,
            has_spike = patterns.has_spike,
            "🚨 ERROR ALERT: {}",
            error
        );
        
        // Здесь можно добавить отправку в external системы:
        // - Slack/Discord webhook
        // - Email
        // - PagerDuty
        // - Prometheus metrics
    }

    /// Получить текущую статистику
    pub fn get_stats(&self) -> ErrorStats {
        let total = self.total_errors.load(Ordering::Relaxed);
        
        let mut by_type = std::collections::HashMap::new();
        for entry in self.errors_by_type.iter() {
            by_type.insert(entry.key().clone(), entry.value().load(Ordering::Relaxed));
        }
        
        let mut by_severity = std::collections::HashMap::new();
        for entry in self.errors_by_severity.iter() {
            by_severity.insert(*entry.key(), entry.value().load(Ordering::Relaxed));
        }
        
        let history = self.error_history.read();
        let recent_errors = history.get_recent(10);
        let patterns = history.analyze_patterns();
        
        ErrorStats {
            total_errors: total,
            errors_by_type: by_type,
            errors_by_severity: by_severity,
            recent_errors: recent_errors.into_iter().cloned().collect(),
            patterns: Some(patterns),
        }
    }

    /// Сбросить статистику (для тестов)
    pub fn reset(&self) {
        self.total_errors.store(0, Ordering::Relaxed);
        self.errors_by_type.clear();
        self.errors_by_severity.clear();
        
        let mut history = self.error_history.write();
        *history = ErrorHistory::new(self.config.history_size);
        
        let mut limiter = self.alert_limiter.write();
        *limiter = RateLimiter::new(Duration::from_secs(self.config.rate_window_seconds));
    }
}

/// Статистика ошибок
#[derive(Debug, Serialize)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_type: std::collections::HashMap<String, u64>,
    pub errors_by_severity: std::collections::HashMap<ErrorSeverity, u64>,
    pub recent_errors: Vec<ErrorEntry>,
    pub patterns: Option<ErrorPatterns>,
}

/// Global error monitor instance
static GLOBAL_MONITOR: once_cell::sync::Lazy<ErrorMonitor> = 
    once_cell::sync::Lazy::new(|| ErrorMonitor::new(ErrorMonitorConfig::default()));

/// Удобная функция для записи ошибок
pub fn record_error(error: &MagrayError) {
    GLOBAL_MONITOR.record_error(error, None);
}

/// Удобная функция для записи ошибок с контекстом
pub fn record_error_with_context(error: &MagrayError, context: serde_json::Value) {
    GLOBAL_MONITOR.record_error(error, Some(context));
}

/// Получить глобальную статистику
pub fn get_error_stats() -> ErrorStats {
    GLOBAL_MONITOR.get_stats()
}

/// Макрос для автоматической записи ошибок
#[macro_export]
macro_rules! monitor_error {
    ($result:expr) => {
        match $result {
            Ok(val) => Ok(val),
            Err(e) => {
                $crate::error_monitor::record_error(&e);
                Err(e)
            }
        }
    };
    ($result:expr, $context:expr) => {
        match $result {
            Ok(val) => Ok(val),
            Err(e) => {
                $crate::error_monitor::record_error_with_context(&e, $context);
                Err(e)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{MagrayError, NetworkError};
    
    #[test]
    fn test_error_monitoring() {
        let monitor = ErrorMonitor::new(ErrorMonitorConfig {
            error_rate_threshold: 2.0,
            rate_window_seconds: 1,
            history_size: 10,
            alert_cooldown: Duration::from_secs(1),
        });
        
        // Record some errors
        let error = MagrayError::Network(NetworkError::Timeout(30));
        monitor.record_error(&error, None);
        
        let stats = monitor.get_stats();
        assert_eq!(stats.total_errors, 1);
    }
    
    #[test]
    fn test_rate_limiting() {
        let mut limiter = RateLimiter::new(Duration::from_secs(1));
        
        // Быстро добавляем ошибки
        for _ in 0..5 {
            let rate = limiter.increment();
            assert!(rate >= 1.0);
        }
    }
    
    #[test]
    fn test_error_patterns() {
        let monitor = ErrorMonitor::new(ErrorMonitorConfig::default());
        
        // Создаем паттерн повторяющихся ошибок
        for _ in 0..10 {
            let error = MagrayError::Timeout("test".into());
            monitor.record_error(&error, None);
        }
        
        let stats = monitor.get_stats();
        assert!(stats.patterns.is_some());
        
        let patterns = stats.patterns.unwrap();
        assert!(!patterns.recurring_errors.is_empty());
    }
}