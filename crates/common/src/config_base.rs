use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Базовая конфигурация для batch операций
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfigBase {
    pub batch_size: usize,
    pub max_batch_size: usize,
    pub batch_timeout_ms: u64,
    pub flush_interval_ms: u64,
}

impl Default for BatchConfigBase {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_batch_size: 1000,
            batch_timeout_ms: 100,
            flush_interval_ms: 1000,
        }
    }
}

impl BatchConfigBase {
    pub fn small() -> Self {
        Self {
            batch_size: 10,
            max_batch_size: 100,
            batch_timeout_ms: 50,
            flush_interval_ms: 500,
        }
    }

    pub fn large() -> Self {
        Self {
            batch_size: 500,
            max_batch_size: 5000,
            batch_timeout_ms: 200,
            flush_interval_ms: 2000,
        }
    }
}

/// Базовая конфигурация для тайм-аутов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfigBase {
    pub operation_timeout_ms: u64,
    pub connection_timeout_ms: u64,
    pub read_timeout_ms: u64,
    pub write_timeout_ms: u64,
}

impl Default for TimeoutConfigBase {
    fn default() -> Self {
        Self {
            operation_timeout_ms: 5000,
            connection_timeout_ms: 1000,
            read_timeout_ms: 3000,
            write_timeout_ms: 3000,
        }
    }
}

impl TimeoutConfigBase {
    pub fn fast() -> Self {
        Self {
            operation_timeout_ms: 1000,
            connection_timeout_ms: 500,
            read_timeout_ms: 1000,
            write_timeout_ms: 1000,
        }
    }

    pub fn slow() -> Self {
        Self {
            operation_timeout_ms: 30000,
            connection_timeout_ms: 5000,
            read_timeout_ms: 10000,
            write_timeout_ms: 10000,
        }
    }
}

/// Базовая конфигурация для кэша
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfigBase {
    pub max_cache_size: usize,
    pub cache_ttl_seconds: u64,
    pub eviction_policy: String,
    pub enable_compression: bool,
}

impl Default for CacheConfigBase {
    fn default() -> Self {
        Self {
            max_cache_size: 10000,
            cache_ttl_seconds: 3600,
            eviction_policy: "lru".to_string(),
            enable_compression: false,
        }
    }
}

impl CacheConfigBase {
    pub fn small() -> Self {
        Self {
            max_cache_size: 100,
            cache_ttl_seconds: 300,
            eviction_policy: "lru".to_string(),
            enable_compression: false,
        }
    }

    pub fn large() -> Self {
        Self {
            max_cache_size: 100000,
            cache_ttl_seconds: 86400,
            eviction_policy: "lfu".to_string(),
            enable_compression: true,
        }
    }
}

/// Базовая конфигурация для circuit breaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfigBase {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_duration_ms: u64,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfigBase {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration_ms: 60000,
            half_open_max_calls: 3,
        }
    }
}

/// Базовая конфигурация для retry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfigBase {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f32,
}

impl Default for RetryConfigBase {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Базовая конфигурация для мониторинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfigBase {
    pub metrics_interval_seconds: u64,
    pub enable_tracing: bool,
    pub enable_metrics: bool,
    pub sample_rate: f32,
}

impl Default for MonitoringConfigBase {
    fn default() -> Self {
        Self {
            metrics_interval_seconds: 60,
            enable_tracing: true,
            enable_metrics: true,
            sample_rate: 0.1,
        }
    }
}

/// Базовая конфигурация для хранилища
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfigBase {
    pub storage_path: String,
    pub max_storage_size_mb: u64,
    pub enable_compression: bool,
    pub sync_writes: bool,
}

impl Default for StorageConfigBase {
    fn default() -> Self {
        Self {
            storage_path: "./data".to_string(),
            max_storage_size_mb: 1024,
            enable_compression: true,
            sync_writes: false,
        }
    }
}

/// Базовая конфигурация для сети
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfigBase {
    pub max_connections: usize,
    pub keep_alive_seconds: u64,
    pub dns_cache_ttl_seconds: u64,
    pub enable_http2: bool,
}

impl Default for NetworkConfigBase {
    fn default() -> Self {
        Self {
            max_connections: 100,
            keep_alive_seconds: 60,
            dns_cache_ttl_seconds: 300,
            enable_http2: true,
        }
    }
}

/// Базовая конфигурация для GPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfigBase {
    pub device_id: u32,
    pub memory_fraction: f32,
    pub enable_tensor_cores: bool,
    pub batch_size_multiplier: f32,
}

impl Default for GpuConfigBase {
    fn default() -> Self {
        Self {
            device_id: 0,
            memory_fraction: 0.9,
            enable_tensor_cores: true,
            batch_size_multiplier: 1.0,
        }
    }
}

/// Trait для композиции конфигураций
pub trait ConfigComposition {
    fn batch(&self) -> &BatchConfigBase;
    fn cache(&self) -> &CacheConfigBase;
    fn timeout(&self) -> &TimeoutConfigBase;
}

/// Простой макрос для создания конфигураций с композицией
#[macro_export]
macro_rules! create_config {
    (
        $name:ident {
            batch: $batch_config:expr,
            cache: $cache_config:expr,
            timeout: $timeout_config:expr,
            custom: {
                $($field:ident: $field_type:ty = $default:expr),*
            }
        }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub batch: crate::BatchConfigBase,
            pub cache: crate::CacheConfigBase,
            pub timeout: crate::TimeoutConfigBase,
            $(pub $field: $field_type,)*
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self {
                    batch: $batch_config,
                    cache: $cache_config,
                    timeout: $timeout_config,
                    $($field: $default,)*
                }
            }
        }
        
        impl crate::ConfigComposition for $name {
            fn batch(&self) -> &crate::BatchConfigBase {
                &self.batch
            }
            
            fn cache(&self) -> &crate::CacheConfigBase {
                &self.cache
            }
            
            fn timeout(&self) -> &crate::TimeoutConfigBase {
                &self.timeout
            }
        }
    };
    
    // Вариант без дополнительных полей
    (
        $name:ident {
            batch: $batch_config:expr,
            cache: $cache_config:expr,
            timeout: $timeout_config:expr
        }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            pub batch: crate::BatchConfigBase,
            pub cache: crate::CacheConfigBase,
            pub timeout: crate::TimeoutConfigBase,
        }
        
        impl Default for $name {
            fn default() -> Self {
                Self {
                    batch: $batch_config,
                    cache: $cache_config,
                    timeout: $timeout_config,
                }
            }
        }
        
        impl crate::ConfigComposition for $name {
            fn batch(&self) -> &crate::BatchConfigBase {
                &self.batch
            }
            
            fn cache(&self) -> &crate::CacheConfigBase {
                &self.cache
            }
            
            fn timeout(&self) -> &crate::TimeoutConfigBase {
                &self.timeout
            }
        }
    };
}

/// Пример использования макроса
#[cfg(test)]
mod tests {
    use super::*;

    create_config! {
        TestConfig {
            batch: BatchConfigBase::default(),
            cache: CacheConfigBase::default(),
            timeout: TimeoutConfigBase::default(),
            custom: {
                custom_field: String = "test".to_string()
            }
        }
    }

    #[test]
    fn test_config_creation() {
        let config = TestConfig::default();
        assert_eq!(config.batch.batch_size, 100);
        assert_eq!(config.custom_field, "test");
    }
}