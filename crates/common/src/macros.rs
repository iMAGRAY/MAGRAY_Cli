/// Общие макросы для устранения дублирования кода в MAGRAY CLI
/// 
/// Этот модуль содержит derive macros и helper macros для автоматической
/// генерации повторяющихся impl блоков для Config структур и Service patterns

/// Макрос для автоматической генерации impl Default для Config структур
/// с общими полями и паттернами
#[macro_export]
macro_rules! impl_config_default {
    // Базовая версия для простых Config структур
    ($type:ty, {$($field:ident: $value:expr),*}) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    $($field: $value),*
                }
            }
        }
    };
    
    // Версия для Config с batch_size и timeout паттернами
    ($type:ty, batch: $batch:expr, timeout: $timeout:expr) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    batch_size: $batch,
                    timeout: std::time::Duration::from_secs($timeout),
                    max_queue_size: 10,
                    worker_threads: num_cpus::get(),
                }
            }
        }
    };
    
    // Версия для GPU Config паттернов
    ($type:ty, gpu_config) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    use_gpu: true,
                    gpu_provider: crate::GpuProviderType::Auto,
                    memory_pool_size: 1024 * 1024 * 1024, // 1GB
                    batch_size: 32,
                    max_length: 512,
                }
            }
        }
    };
    
    // Версия для Memory Config паттернов
    ($type:ty, memory_config: {max_size: $max_size:expr, max_entries: $max_entries:expr}) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    max_size_bytes: $max_size,
                    max_entries: $max_entries,
                    ttl_seconds: Some(86400 * 7), // 7 days
                    eviction_batch_size: 100,
                }
            }
        }
    };
}

/// Макрос для автоматической генерации impl AsRef для path-like структур
#[macro_export]
macro_rules! impl_as_ref_path {
    ($type:ty, $field:ident) => {
        impl AsRef<std::path::Path> for $type {
            fn as_ref(&self) -> &std::path::Path {
                self.$field.as_ref()
            }
        }
    };
}

/// Макрос для автоматической генерации impl Display для структур с описанием
#[macro_export]
macro_rules! impl_display {
    ($type:ty, $format:expr) => {
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $format, self)
            }
        }
    };
    
    ($type:ty, $field:ident) => {
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.$field)
            }
        }
    };
}

/// Макрос для автоматической генерации basic Service implementations
#[macro_export]
macro_rules! impl_service_basic {
    ($type:ty, $trait:ty) => {
        impl $trait for $type {
            fn name(&self) -> &'static str {
                stringify!($type)
            }
            
            fn is_healthy(&self) -> bool {
                true // Default health check
            }
            
            fn version(&self) -> &'static str {
                env!("CARGO_PKG_VERSION")
            }
        }
    };
}

/// Макрос для генерации CircuitBreaker implementations с общими паттернами
#[macro_export]
macro_rules! impl_circuit_breaker {
    ($type:ty) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    failure_threshold: 5,
                    timeout: std::time::Duration::from_secs(60),
                    success_threshold: 3,
                    state: crate::CircuitBreakerState::Closed,
                    failure_count: 0,
                    last_failure_time: None,
                }
            }
        }
        
        impl $type {
            pub fn new() -> Self {
                Self::default()
            }
            
            pub fn with_threshold(mut self, threshold: usize) -> Self {
                self.failure_threshold = threshold;
                self
            }
            
            pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
                self.timeout = timeout;
                self
            }
        }
    };
}

/// Макрос для генерации Retry implementations с экспоненциальным backoff
#[macro_export]
macro_rules! impl_retry_policy {
    ($type:ty) => {
        impl Default for $type {
            fn default() -> Self {
                Self {
                    max_retries: 3,
                    base_delay: std::time::Duration::from_millis(100),
                    max_delay: std::time::Duration::from_secs(30),
                    exponential_base: 2.0,
                    jitter: true,
                }
            }
        }
        
        impl $type {
            pub fn new() -> Self {
                Self::default()
            }
            
            pub fn with_max_retries(mut self, max_retries: usize) -> Self {
                self.max_retries = max_retries;
                self
            }
            
            pub fn with_base_delay(mut self, delay: std::time::Duration) -> Self {
                self.base_delay = delay;
                self
            }
        }
    };
}

/// Макрос для генерации Builder pattern implementations
#[macro_export]
macro_rules! impl_builder {
    ($type:ty, $builder:ty, $($field:ident: $field_type:ty),*) => {
        impl $builder {
            pub fn new() -> Self {
                Self::default()
            }
            
            $(
                pub fn $field(mut self, $field: $field_type) -> Self {
                    self.$field = Some($field);
                    self
                }
            )*
            
            pub fn build(self) -> Result<$type, crate::errors::MagrayError> {
                Ok(<$type>::new())
            }
        }
    };
}

/// Макрос для генерации Clone implementations для Arc-wrapped types
#[macro_export]
macro_rules! impl_arc_clone {
    ($type:ty) => {
        impl Clone for $type {
            fn clone(&self) -> Self {
                Self {
                    inner: Arc::clone(&self.inner)
                }
            }
        }
    };
}

/// Макрос для генерации Debug implementations с полями-секретами
#[macro_export]
macro_rules! impl_debug_redacted {
    ($type:ty, redact: [$($redacted_field:ident),*]) => {
        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut debug_struct = f.debug_struct(stringify!($type));
                
                // Все поля кроме redacted
                $(
                    debug_struct.field(stringify!($redacted_field), &"[REDACTED]");
                )*
                
                debug_struct.finish()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    struct TestConfig {
        batch_size: usize,
        timeout: Duration,
        max_queue_size: usize,
        worker_threads: usize,
    }
    
    impl_config_default!(TestConfig, batch: 100, timeout: 30);
    
    #[test]
    fn test_config_default_macro() {
        let config = TestConfig::default();
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_queue_size, 10);
    }
}