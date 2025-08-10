//! Общие макросы для устранения дублирования кода в MAGRAY CLI
//! Этот модуль содержит derive macros и helper macros для автоматической
//! генерации повторяющихся impl блоков для Config структур и Service patterns

#[macro_export]
macro_rules! impl_gpu_config_defaults {
    () => {
        impl Default for $crate::GpuConfig {
            fn default() -> Self {
                Self {
                    use_gpu: true,
                    gpu_provider: $crate::GpuProviderType::Auto,
                    memory_pool_size: 1024 * 1024 * 1024, // 1GB
                    batch_size: 32,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_circuit_breaker_defaults {
    () => {
        impl Default for $crate::CircuitBreakerConfig {
            fn default() -> Self {
                Self {
                    timeout: std::time::Duration::from_secs(60),
                    success_threshold: 3,
                    state: $crate::CircuitBreakerState::Closed,
                    failure_count: 0,
                    last_failure_time: None,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! builder_for {
    ($type:ty) => {
        impl $type {
            pub fn builder() -> $crate::Builder<$type> {
                $crate::Builder::<$type>::default()
            }
        }

        impl $crate::Builder<$type> {
            pub fn build(self) -> Result<$type, $crate::errors::MagrayError> {
                Ok(<$type>::new())
            }
        }
    };
}
