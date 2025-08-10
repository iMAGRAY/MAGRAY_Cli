#![allow(async_fn_in_trait)]
//! Service Macros для устранения дублирования кода в MAGRAY CLI
//! Этот модуль содержит макросы для автогенерации повторяющихся реализаций
//! service methods, используя DRY принцип.

/// Макрос для генерации стандартных health_check реализаций
/// Устраняет дублирование fn health_check в различных сервисах
#[macro_export]
macro_rules! impl_health_check_service {
    ($service_type:ty, $health_data:ty) => {
        #[async_trait::async_trait]
        impl $crate::service_traits::HealthCheckService for $service_type {
            type HealthData = $health_data;

            async fn health_check(&self) -> Result<Self::HealthData, $crate::MagrayCoreError> {
                // Базовая проверка готовности
                if !self.is_healthy() {
                    return Err($crate::MagrayCoreError::ServiceNotHealthy(
                        self.name().to_string(),
                    ));
                }

                // Создаем стандартные health data
                let health_data = self.create_health_data().await?;
                Ok(health_data)
            }
        }
    };
}

/// Макрос для генерации стандартных service lifecycle реализаций
/// Устраняет дублирование fn initialize, fn shutdown, fn is_ready
#[macro_export]
macro_rules! impl_lifecycle_service {
    ($service_type:ty, $init_config:ty) => {
        #[async_trait::async_trait]
        impl $crate::service_traits::InitializableService for $service_type {
            type InitConfig = $init_config;

            async fn initialize(
                &mut self,
                config: Self::InitConfig,
            ) -> Result<(), $crate::MagrayCoreError> {
                tracing::info!("🚀 Инициализация сервиса: {}", self.name());

                if self.is_initialized() {
                    return Err($crate::MagrayCoreError::ServiceAlreadyInitialized(
                        self.name().to_string(),
                    ));
                }

                self.perform_initialization(config).await?;
                self.set_initialized(true);

                tracing::info!("✅ Сервис {} инициализирован", self.name());
                Ok(())
            }
        }

        impl $crate::service_traits::BaseService for $service_type {
            fn name(&self) -> &'static str {
                std::any::type_name::<$service_type>()
                    .split("::")
                    .last()
                    .unwrap_or("UnknownService")
            }

            async fn shutdown(&self) -> Result<(), $crate::MagrayCoreError> {
                tracing::info!("🛑 Остановка сервиса: {}", self.name());

                self.perform_shutdown().await?;

                tracing::info!("✅ Сервис {} остановлен", self.name());
                Ok(())
            }
        }
    };
}

/// Макрос для генерации стандартных statistics provider реализаций
/// Устраняет дублирование fn get_stats
#[macro_export]
macro_rules! impl_statistics_provider {
    ($service_type:ty, $stats_type:ty) => {
        impl $crate::service_traits::StatisticsProvider for $service_type {
            type Stats = $stats_type;

            fn get_stats(&self) -> Self::Stats {
                self.collect_stats()
            }

            fn reset_stats(&mut self) {
                tracing::debug!("🔄 Сброс статистики для {}", self.name());
                self.perform_stats_reset();
            }
        }
    };
}

/// Макрос для генерации configuration profile реализаций
/// Устраняет дублирование fn production, fn minimal
#[macro_export]
macro_rules! impl_configuration_profile {
    ($config_type:ty) => {
        impl $crate::service_traits::ConfigurationProfile<$config_type> for $config_type {
            fn production() -> $config_type {
                let mut config = Self::default();
                config.optimize_for_production();
                config
            }

            fn minimal() -> $config_type {
                let mut config = Self::default();
                config.minimize_resources();
                config
            }

            fn validate_profile(config: &$config_type) -> Result<(), $crate::ConfigError> {
                config.validate_internal()
            }
        }
    };
}

/// Макрос для генерации buildable service реализаций  
/// Устраняет дублирование fn build
#[macro_export]
macro_rules! impl_buildable_service {
    ($service_type:ty, $build_config:ty, $build_error:ty) => {
        impl $crate::service_traits::BuildableService<$service_type> for $service_type {
            type BuildConfig = $build_config;
            type BuildError = $build_error;

            fn build(config: Self::BuildConfig) -> Result<$service_type, Self::BuildError> {
                tracing::info!(
                    "🔧 Сборка сервиса: {}",
                    std::any::type_name::<$service_type>()
                );

                let service = Self::build_with_config(config)?;

                tracing::info!("✅ Сервис собран успешно");
                Ok(service)
            }
        }
    };
}

/// Макрос для генерации executable service реализаций
/// Устраняет дублирование fn execute
#[macro_export]
macro_rules! impl_executable_service {
    ($service_type:ty, $input:ty, $output:ty, $error:ty) => {
        #[async_trait::async_trait]
        impl $crate::service_traits::ExecutableService<$input, $output> for $service_type {
            type ExecuteError = $error;

            async fn execute(&self, input: $input) -> Result<$output, Self::ExecuteError> {
                tracing::debug!("⚡ Выполнение операции в {}", self.name());

                let result = self.perform_execute(input).await?;

                tracing::debug!("✅ Операция выполнена в {}", self.name());
                Ok(result)
            }
        }
    };
}

/// Макрос для генерации clearable service реализаций  
/// Устраняет дублирование fn clear
#[macro_export]
macro_rules! impl_clearable_service {
    ($service_type:ty) => {
        #[async_trait::async_trait]
        impl $crate::service_traits::ClearableService for $service_type {
            async fn clear(&mut self) -> Result<(), $crate::MagrayCoreError> {
                tracing::info!("🧹 Очистка сервиса: {}", self.name());

                if !self.can_clear().await {
                    return Err($crate::MagrayCoreError::OperationNotAllowed(format!(
                        "Очистка {} не разрешена в текущем состоянии",
                        self.name()
                    )));
                }

                self.perform_clear().await?;

                tracing::info!("✅ Сервис {} очищен", self.name());
                Ok(())
            }

            async fn can_clear(&self) -> bool {
                self.is_clearable()
            }
        }
    };
}

/// Композитный макрос для полной service реализации
/// Объединяет все основные service traits для устранения максимального дублирования
#[macro_export]
macro_rules! impl_full_service {
    (
        $service_type:ty,
        health_data: $health_data:ty,
        init_config: $init_config:ty,
        stats: $stats_type:ty,
        build_config: $build_config:ty,
        build_error: $build_error:ty
    ) => {
        $crate::impl_health_check_service!($service_type, $health_data);
        $crate::impl_lifecycle_service!($service_type, $init_config);
        $crate::impl_statistics_provider!($service_type, $stats_type);
        $crate::impl_buildable_service!($service_type, $build_config, $build_error);
        $crate::impl_clearable_service!($service_type);
    };
}

/// Макрос для генерации coordinator implementations
/// Специально для orchestration coordinators, которые имеют много дублированных методов
#[macro_export]
macro_rules! impl_coordinator {
    ($coordinator_type:ty, $metrics_type:ty) => {
        #[async_trait::async_trait]
        impl $crate::orchestration::traits::Coordinator for $coordinator_type {
            async fn initialize(&self) -> anyhow::Result<()> {
                tracing::info!("🚀 Инициализация координатора: {}", self.name());
                self.perform_coordinator_init().await
            }

            async fn is_ready(&self) -> bool {
                self.check_readiness().await
            }

            async fn health_check(&self) -> anyhow::Result<()> {
                if !self.is_ready().await {
                    return Err(anyhow::anyhow!("Координатор {} не готов", self.name()));
                }
                self.perform_health_check().await
            }

            async fn shutdown(&self) -> anyhow::Result<()> {
                tracing::info!("🛑 Остановка координатора: {}", self.name());
                self.perform_coordinator_shutdown().await
            }

            async fn metrics(&self) -> serde_json::Value {
                self.collect_coordinator_metrics().await
            }
        }

        impl $crate::service_traits::BaseService for $coordinator_type {
            fn name(&self) -> &'static str {
                std::any::type_name::<$coordinator_type>()
                    .split("::")
                    .last()
                    .unwrap_or("UnknownCoordinator")
            }

            async fn shutdown(&self) -> Result<(), $crate::MagrayCoreError> {
                self.perform_coordinator_shutdown()
                    .await
                    .map_err(|e| $crate::MagrayCoreError::ServiceShutdownError(e.to_string()))
            }
        }
    };
}

/// Trait helper для требуемых методов в макросах
/// Эти методы должны быть реализованы в структуре, использующей макросы
pub trait ServiceMacroHelpers {
    type HealthData;
    type Stats;

    /// Создать health data для service
    async fn create_health_data(&self) -> Result<Self::HealthData, crate::MagrayCoreError>;

    /// Проверить инициализацию
    fn is_initialized(&self) -> bool;

    /// Установить статус инициализации
    fn set_initialized(&self, initialized: bool);

    /// Выполнить фактическую инициализацию
    async fn perform_initialization<T>(&mut self, config: T) -> Result<(), crate::MagrayCoreError>;

    /// Выполнить фактическое завершение работы
    async fn perform_shutdown(&self) -> Result<(), crate::MagrayCoreError>;

    /// Собрать статистику
    fn collect_stats(&self) -> Self::Stats;

    /// Выполнить сброс статистики
    fn perform_stats_reset(&mut self);

    /// Проверить возможность очистки
    fn is_clearable(&self) -> bool;

    /// Выполнить фактическую очистку
    async fn perform_clear(&mut self) -> Result<(), crate::MagrayCoreError>;
}

/// Trait helper для coordinator макросов
pub trait CoordinatorMacroHelpers {
    /// Выполнить инициализацию координатора
    async fn perform_coordinator_init(&self) -> anyhow::Result<()>;

    /// Проверить готовность координатора
    async fn check_readiness(&self) -> bool;

    /// Выполнить health check координатора
    async fn perform_health_check(&self) -> anyhow::Result<()>;

    /// Выполнить завершение работы координатора
    async fn perform_coordinator_shutdown(&self) -> anyhow::Result<()>;

    /// Собрать метрики координатора
    async fn collect_coordinator_metrics(&self) -> serde_json::Value;
}

/// Макрос для генерации default trait implementations с настраиваемыми параметрами
#[macro_export]
macro_rules! impl_service_defaults {
    ($service_type:ty, name: $service_name:literal) => {
        impl $crate::service_traits::BaseService for $service_type {
            fn name(&self) -> &'static str {
                $service_name
            }
        }
    };

    ($service_type:ty, name: $service_name:literal, version: $version:literal) => {
        impl $crate::service_traits::BaseService for $service_type {
            fn name(&self) -> &'static str {
                $service_name
            }

            fn version(&self) -> &'static str {
                $version
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_traits::*;
    use async_trait::async_trait;

    // Тестовая структура для проверки макросов
    struct TestService {
        initialized: bool,
        name: &'static str,
    }

    impl TestService {
        fn new(name: &'static str) -> Self {
            Self {
                initialized: false,
                name,
            }
        }
    }

    // Реализуем required helpers
    impl ServiceMacroHelpers for TestService {
        type HealthData = String;
        type Stats = u64;

        async fn create_health_data(&self) -> Result<Self::HealthData, crate::MagrayCoreError> {
            Ok("Healthy".to_string())
        }

        fn is_initialized(&self) -> bool {
            self.initialized
        }

        fn set_initialized(&self, initialized: bool) {
            // Для тестов упрощаем
        }

        async fn perform_initialization<T>(
            &mut self,
            _config: T,
        ) -> Result<(), crate::MagrayCoreError> {
            Ok(())
        }

        async fn perform_shutdown(&self) -> Result<(), crate::MagrayCoreError> {
            Ok(())
        }

        fn collect_stats(&self) -> Self::Stats {
            42
        }

        fn perform_stats_reset(&mut self) {}

        fn is_clearable(&self) -> bool {
            true
        }

        async fn perform_clear(&mut self) -> Result<(), crate::MagrayCoreError> {
            Ok(())
        }
    }

    // Применяем макрос
    impl_service_defaults!(TestService, name: "TestService", version: "1.0.0");

    #[tokio::test]
    async fn test_service_defaults_macro() {
        let service = TestService::new("test");
        assert_eq!(service.name(), "TestService");
        assert_eq!(service.version(), "1.0.0");
    }
}
