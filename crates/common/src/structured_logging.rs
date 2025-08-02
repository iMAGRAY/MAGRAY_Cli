use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{Level, Event, Subscriber};
use tracing::field::{Field, Visit};
use tracing_subscriber::{fmt, layer::SubscriberExt, Layer, EnvFilter, Registry};
use tracing_subscriber::fmt::format::FmtSpan;
use std::io::{self, Write};
use chrono::Utc;

/// @component: {"k":"C","id":"structured_logging","t":"JSON structured logging system","m":{"cur":100,"tgt":100,"u":"%"},"f":["logging","json","production"]}
/// Структурированная запись лога в JSON формате
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogEntry {
    /// Временная метка в ISO 8601 формате
    pub timestamp: String,
    /// Уровень логирования
    pub level: String,
    /// Целевой модуль/компонент
    pub target: String,
    /// Основное сообщение
    pub message: String,
    /// Дополнительные поля
    #[serde(flatten)]
    pub fields: HashMap<String, Value>,
    /// Контекст выполнения
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<ExecutionContext>,
    /// Метрики производительности
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceMetrics>,
}

/// Контекст выполнения для отслеживания
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// ID запроса/сессии
    pub request_id: Option<String>,
    /// Имя пользователя
    pub user_id: Option<String>,
    /// Версия приложения
    pub app_version: String,
    /// Имя хоста
    pub hostname: String,
    /// ID процесса
    pub pid: u32,
    /// ID потока
    pub thread_id: String,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            request_id: None,
            user_id: None,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            hostname: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
            pid: std::process::id(),
            thread_id: format!("{:?}", std::thread::current().id()),
        }
    }
}

/// Метрики производительности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Длительность операции в миллисекундах
    pub duration_ms: Option<u64>,
    /// Использование CPU в процентах
    pub cpu_usage_percent: Option<f32>,
    /// Использование памяти в байтах
    pub memory_usage_bytes: Option<u64>,
    /// Количество обработанных элементов
    pub items_processed: Option<u64>,
    /// Пропускная способность (элементов в секунду)
    pub throughput: Option<f32>,
}

/// Форматтер для JSON логов
pub struct JsonFormatter;

impl<S> Layer<S> for JsonFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = JsonVisitor::default();
        event.record(&mut visitor);
        
        let level = match *event.metadata().level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };
        
        let performance = visitor.extract_performance_metrics();
        
        let entry = StructuredLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: event.metadata().target().to_string(),
            message: visitor.message.unwrap_or_default(),
            fields: visitor.fields,
            context: Some(ExecutionContext::default()),
            performance,
        };
        
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = writeln!(io::stdout(), "{}", json);
        }
    }
}

/// Визитор для извлечения полей из события
#[derive(Default)]
struct JsonVisitor {
    message: Option<String>,
    fields: HashMap<String, Value>,
}

impl Visit for JsonVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        } else {
            self.fields.insert(
                field.name().to_string(),
                Value::String(format!("{:?}", value)),
            );
        }
    }
    
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(
                field.name().to_string(),
                Value::String(value.to_string()),
            );
        }
    }
    
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            Value::Number(value.into()),
        );
    }
    
    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            Value::Number(value.into()),
        );
    }
    
    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields.insert(
                field.name().to_string(),
                Value::Number(n),
            );
        }
    }
    
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.insert(
            field.name().to_string(),
            Value::Bool(value),
        );
    }
}

impl JsonVisitor {
    /// Извлечь метрики производительности из полей
    fn extract_performance_metrics(&self) -> Option<PerformanceMetrics> {
        if self.fields.is_empty() {
            return None;
        }
        
        let metrics = PerformanceMetrics {
            duration_ms: self.get_u64_field("duration_ms"),
            cpu_usage_percent: self.get_f64_field("cpu_usage").map(|v| v as f32),
            memory_usage_bytes: self.get_u64_field("memory_bytes"),
            items_processed: self.get_u64_field("items_count"),
            throughput: self.get_f64_field("throughput").map(|v| v as f32),
        };
        
        // Если есть хотя бы одна метрика, возвращаем
        if metrics.duration_ms.is_some() 
            || metrics.cpu_usage_percent.is_some()
            || metrics.memory_usage_bytes.is_some()
            || metrics.items_processed.is_some()
            || metrics.throughput.is_some() {
            Some(metrics)
        } else {
            None
        }
    }
    
    fn get_u64_field(&self, name: &str) -> Option<u64> {
        self.fields.get(name)
            .and_then(|v| v.as_u64())
    }
    
    fn get_f64_field(&self, name: &str) -> Option<f64> {
        self.fields.get(name)
            .and_then(|v| v.as_f64())
    }
}

/// Конфигурация для structured logging
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Минимальный уровень логирования
    pub level: Level,
    /// Вывод в JSON формате
    pub json_output: bool,
    /// Включить цветной вывод (только для non-JSON)
    pub color_output: bool,
    /// Файл для записи логов
    pub log_file: Option<String>,
    /// Максимальный размер файла логов (в байтах)
    pub max_file_size: Option<u64>,
    /// Включить контекст выполнения
    pub include_context: bool,
    /// Включить номера строк
    pub include_line_numbers: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            json_output: false,
            color_output: true,
            log_file: None,
            max_file_size: Some(100 * 1024 * 1024), // 100MB
            include_context: true,
            include_line_numbers: cfg!(debug_assertions),
        }
    }
}

/// Инициализировать structured logging
pub fn init_structured_logging(config: LoggingConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.level.to_string()));
    
    if config.json_output {
        // JSON формат для production
        let json_layer = JsonFormatter;
        
        let subscriber = Registry::default()
            .with(env_filter)
            .with(json_layer);
            
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        // Человекочитаемый формат для разработки
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_line_number(config.include_line_numbers)
            .with_ansi(config.color_output)
            .with_span_events(FmtSpan::CLOSE);
            
        let subscriber = Registry::default()
            .with(env_filter)
            .with(fmt_layer);
            
        tracing::subscriber::set_global_default(subscriber)?;
    }
    
    Ok(())
}

/// Макрос для структурированного логирования с метриками
#[macro_export]
macro_rules! log_with_metrics {
    ($level:expr, $message:expr, $($field:tt)*) => {
        match $level {
            tracing::Level::ERROR => tracing::error!($($field)*, message = $message),
            tracing::Level::WARN => tracing::warn!($($field)*, message = $message),
            tracing::Level::INFO => tracing::info!($($field)*, message = $message),
            tracing::Level::DEBUG => tracing::debug!($($field)*, message = $message),
            tracing::Level::TRACE => tracing::trace!($($field)*, message = $message),
        }
    };
}

/// Вспомогательная структура для измерения времени операций
pub struct OperationTimer {
    start: std::time::Instant,
    operation_name: String,
    fields: HashMap<String, Value>,
}

impl OperationTimer {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            start: std::time::Instant::now(),
            operation_name: operation_name.into(),
            fields: HashMap::new(),
        }
    }
    
    pub fn add_field(&mut self, key: impl Into<String>, value: impl Serialize) {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.into(), v);
        }
    }
    
    pub fn finish(self) {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        
        tracing::info!(
            operation = %self.operation_name,
            duration_ms = duration_ms,
            success = true,
            fields = ?self.fields,
            "Operation completed"
        );
    }
    
    pub fn finish_with_result<T>(self, result: Result<T, impl std::fmt::Display>) {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        
        match result {
            Ok(_) => {
                tracing::info!(
                    operation = %self.operation_name,
                    duration_ms = duration_ms,
                    success = true,
                    fields = ?self.fields,
                    "Operation completed"
                );
            }
            Err(e) => {
                tracing::error!(
                    operation = %self.operation_name,
                    duration_ms = duration_ms,
                    success = false,
                    error = %e,
                    fields = ?self.fields,
                    "Operation failed"
                );
            }
        }
    }
}

/// Контекст запроса для отслеживания через async операции
#[derive(Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub user_id: Option<String>,
    pub start_time: std::time::Instant,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_structured_log_entry_serialization() {
        let entry = StructuredLogEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: "INFO".to_string(),
            target: "test::module".to_string(),
            message: "Test message".to_string(),
            fields: HashMap::new(),
            context: Some(ExecutionContext::default()),
            performance: Some(PerformanceMetrics {
                duration_ms: Some(100),
                cpu_usage_percent: Some(25.5),
                memory_usage_bytes: Some(1024 * 1024),
                items_processed: Some(1000),
                throughput: Some(10000.0),
            }),
        };
        
        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("INFO"));
        assert!(json.contains("Test message"));
        assert!(json.contains("duration_ms"));
    }
    
    #[test]
    fn test_operation_timer() {
        let mut timer = OperationTimer::new("test_operation");
        timer.add_field("user_id", "12345");
        timer.add_field("items_count", 100);
        
        // Симулируем работу
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        // Timer завершится и запишет лог при drop
        timer.finish();
    }
}