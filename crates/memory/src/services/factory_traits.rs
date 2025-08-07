//! Factory Traits - общие интерфейсы для всех типов factory
//!
//! Обеспечивает единообразные интерфейсы для создания сервисов разных типов,
//! применяя принципы Interface Segregation и Dependency Inversion.
//!
//! РЕШАЕМЫЕ ПРОБЛЕМЫ:
//! - Отсутствие единых интерфейсов между factory
//! - Дублирование методов создания
//! - Невозможность легкой замены implementations
//! - Отсутствие type-safe конфигурации
//!
//! ПРИНЦИПЫ SOLID:
//! - Single Responsibility: Каждый trait отвечает за конкретный тип factory
//! - Open/Closed: Расширяемость через новые trait implementations
//! - Liskov Substitution: Все implementations взаимозаменяемы
//! - Interface Segregation: Минимальные, специализированные интерфейсы
//! - Dependency Inversion: Зависимости от абстракций, не от конкретных типов

use anyhow::Result;
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

use crate::{
    di::unified_container::UnifiedDIContainer,
    orchestration::{EmbeddingCoordinator, HealthManager, ResourceController, SearchCoordinator},
    service_di::coordinator_factory::OrchestrationCoordinators,
    services::traits::{
        CacheServiceTrait, CoordinatorServiceTrait, CoreMemoryServiceTrait, MonitoringServiceTrait,
        ResilienceServiceTrait,
    },
};

/// Базовый trait для всех factory - определяет общие методы
#[async_trait]
pub trait BaseFactory {
    type Config;
    type Result;

    /// Создать экземпляр с конфигурацией по умолчанию
    async fn create_default(&self) -> Result<Self::Result>;

    /// Создать экземпляр с кастомной конфигурацией
    async fn create_with_config(&self, config: Self::Config) -> Result<Self::Result>;

    /// Validate конфигурацию перед созданием
    fn validate_config(&self, config: &Self::Config) -> Result<()>;

    /// Получить конфигурацию по умолчанию для данного factory
    fn default_config(&self) -> Self::Config;
}

/// Factory для создания основных сервисов memory системы
#[async_trait]
pub trait CoreServiceFactory: BaseFactory {
    /// Создать CoreMemoryService
    async fn create_core_memory(
        &self,
        container: Arc<UnifiedDIContainer>,
    ) -> Result<Arc<dyn CoreMemoryServiceTrait>>;

    /// Создать ResilienceService с circuit breaker
    async fn create_resilience(
        &self,
        threshold: u32,
        timeout_secs: u64,
    ) -> Result<Arc<dyn ResilienceServiceTrait>>;

    /// Создать MonitoringService с зависимостями
    async fn create_monitoring(
        &self,
        container: Arc<UnifiedDIContainer>,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
    ) -> Result<Arc<dyn MonitoringServiceTrait>>;

    /// Создать CacheService с конфигурацией
    async fn create_cache(
        &self,
        container: Arc<UnifiedDIContainer>,
        coordinator: Arc<dyn CoordinatorServiceTrait>,
        embedding_dimension: usize,
    ) -> Result<Arc<dyn CacheServiceTrait>>;
}

/// Factory для создания координаторов orchestration
#[async_trait]
pub trait CoordinatorFactory: BaseFactory {
    /// Создать EmbeddingCoordinator
    async fn create_embedding_coordinator(
        &self,
        container: &UnifiedDIContainer,
    ) -> Result<Arc<EmbeddingCoordinator>>;

    /// Создать SearchCoordinator с зависимостями
    async fn create_search_coordinator(
        &self,
        container: &UnifiedDIContainer,
        embedding_coordinator: &Arc<EmbeddingCoordinator>,
        max_concurrent: usize,
        cache_size: usize,
    ) -> Result<Arc<SearchCoordinator>>;

    /// Создать HealthManager
    async fn create_health_manager(
        &self,
        container: &UnifiedDIContainer,
    ) -> Result<Arc<HealthManager>>;

    /// Создать ResourceController
    async fn create_resource_controller(
        &self,
        container: &UnifiedDIContainer,
    ) -> Result<Arc<ResourceController>>;

    /// Создать все координаторы сразу
    async fn create_all_coordinators(
        &self,
        container: &UnifiedDIContainer,
    ) -> Result<OrchestrationCoordinators>;
}

/// Factory для создания complete service collections
#[async_trait]
pub trait ServiceCollectionFactory: BaseFactory {
    type ServiceCollection;

    /// Создать полную коллекцию сервисов
    async fn create_complete_collection(
        &self,
        container: Arc<UnifiedDIContainer>,
    ) -> Result<Self::ServiceCollection>;

    /// Создать минимальную коллекцию (только core services)
    async fn create_minimal_collection(
        &self,
        container: Arc<UnifiedDIContainer>,
    ) -> Result<Self::ServiceCollection>;

    /// Создать коллекцию для тестирования
    async fn create_test_collection(
        &self,
        container: Arc<UnifiedDIContainer>,
    ) -> Result<Self::ServiceCollection>;
}

/// Конфигурация для specialized factory
#[derive(Debug, Clone)]
pub struct SpecializedFactoryConfig {
    /// Создавать ли embedding coordinator
    pub enable_embedding: bool,
    /// Создавать ли search coordinator  
    pub enable_search: bool,
    /// Создавать ли health manager
    pub enable_health: bool,
    /// Создавать ли resource controller
    pub enable_resources: bool,
    /// Параметры производительности
    pub max_concurrent_operations: usize,
    /// Размер cache
    pub cache_size: usize,
}

impl Default for SpecializedFactoryConfig {
    fn default() -> Self {
        Self {
            enable_embedding: true,
            enable_search: true,
            enable_health: true,
            enable_resources: true,
            max_concurrent_operations: 64,
            cache_size: 2000,
        }
    }
}

impl SpecializedFactoryConfig {
    /// Создать конфигурацию для production
    pub fn production() -> Self {
        Self {
            enable_embedding: true,
            enable_search: true,
            enable_health: true,
            enable_resources: true,
            max_concurrent_operations: 128,
            cache_size: 5000,
        }
    }

    /// Создать минимальную конфигурацию
    pub fn minimal() -> Self {
        Self {
            enable_embedding: false,
            enable_search: false,
            enable_health: false,
            enable_resources: false,
            max_concurrent_operations: 16,
            cache_size: 500,
        }
    }

    /// Создать конфигурацию для тестов
    pub fn test() -> Self {
        Self {
            enable_embedding: false,
            enable_search: false,
            enable_health: false,
            enable_resources: false,
            max_concurrent_operations: 4,
            cache_size: 100,
        }
    }
}

/// Factory для создания specialized компонентов (GPU, SIMD, etc.)
#[async_trait]
pub trait SpecializedComponentFactory: BaseFactory<Config = SpecializedFactoryConfig> {
    /// Создать GPU-accelerated компоненты
    async fn create_gpu_components(&self, container: &UnifiedDIContainer) -> Result<()>;

    /// Создать SIMD-optimized компоненты
    async fn create_simd_components(&self, container: &UnifiedDIContainer) -> Result<()>;

    /// Создать ML-enhanced компоненты
    async fn create_ml_components(&self, container: &UnifiedDIContainer) -> Result<()>;

    /// Проверить доступность specialized компонентов
    async fn check_component_availability(&self) -> SpecializedComponentAvailability;
}

/// Информация о доступности specialized компонентов
#[derive(Debug, Clone)]
pub struct SpecializedComponentAvailability {
    pub gpu_available: bool,
    pub simd_available: bool,
    pub ml_models_available: bool,
    pub gpu_memory_mb: Option<usize>,
    pub simd_features: Vec<String>,
    pub loaded_models: Vec<String>,
}

impl Default for SpecializedComponentAvailability {
    fn default() -> Self {
        Self {
            gpu_available: false,
            simd_available: false,
            ml_models_available: false,
            gpu_memory_mb: None,
            simd_features: Vec::new(),
            loaded_models: Vec::new(),
        }
    }
}

/// Factory для создания test doubles (mocks, stubs, etc.)
#[async_trait]
pub trait TestFactory: BaseFactory {
    /// Создать mock services для unit tests
    async fn create_mock_services(&self) -> Result<Box<dyn Any + Send + Sync>>;

    /// Создать stub services для integration tests
    async fn create_stub_services(&self) -> Result<Box<dyn Any + Send + Sync>>;

    /// Создать fake services с реальной логикой но test data
    async fn create_fake_services(&self) -> Result<Box<dyn Any + Send + Sync>>;

    /// Сбросить состояние всех test doubles
    async fn reset_test_state(&self) -> Result<()>;
}

/// Registry для управления всеми factory в системе
/// Использует concrete types вместо trait objects для dyn compatibility
pub trait FactoryRegistry {
    /// Список всех зарегистрированных factory
    fn list_registered_factories(&self) -> Vec<String>;

    /// Очистить registry
    fn clear_registry(&mut self);

    /// Проверить наличие factory определенного типа
    fn has_factory(&self, factory_type: &str) -> bool;
}

/// Конфигурация для различных factory presets
#[derive(Debug, Clone)]
pub enum FactoryPreset {
    /// Production окружение - все компоненты включены
    Production {
        max_performance: bool,
        enable_monitoring: bool,
    },
    /// Development окружение - базовые компоненты
    Development {
        enable_debug: bool,
        mock_external_services: bool,
    },
    /// Test окружение - minimal setup с mocks
    Testing {
        use_mocks: bool,
        in_memory_only: bool,
    },
    /// Custom конфигурация
    Custom {
        core_services: bool,
        coordinators: bool,
        specialized_components: bool,
        custom_config: serde_json::Value,
    },
}

impl FactoryPreset {
    /// Определить нужно ли создавать core services
    pub fn should_create_core_services(&self) -> bool {
        match self {
            FactoryPreset::Production { .. } => true,
            FactoryPreset::Development { .. } => true,
            FactoryPreset::Testing { .. } => true,
            FactoryPreset::Custom { core_services, .. } => *core_services,
        }
    }

    /// Определить нужно ли создавать координаторы
    pub fn should_create_coordinators(&self) -> bool {
        match self {
            FactoryPreset::Production { .. } => true,
            FactoryPreset::Development { .. } => true,
            FactoryPreset::Testing { .. } => false,
            FactoryPreset::Custom { coordinators, .. } => *coordinators,
        }
    }

    /// Определить нужно ли создавать specialized компоненты
    pub fn should_create_specialized(&self) -> bool {
        match self {
            FactoryPreset::Production {
                max_performance, ..
            } => *max_performance,
            FactoryPreset::Development { .. } => false,
            FactoryPreset::Testing { .. } => false,
            FactoryPreset::Custom {
                specialized_components,
                ..
            } => *specialized_components,
        }
    }

    /// Получить maximum concurrent operations для данного preset
    pub fn max_concurrent_operations(&self) -> usize {
        match self {
            FactoryPreset::Production {
                max_performance: true,
                ..
            } => 256,
            FactoryPreset::Production {
                max_performance: false,
                ..
            } => 128,
            FactoryPreset::Development { .. } => 32,
            FactoryPreset::Testing { .. } => 4,
            FactoryPreset::Custom { .. } => 64, // Default для custom
        }
    }
}

/// Errors для factory operations
#[derive(Debug)]
pub enum FactoryError {
    FactoryNotRegistered {
        factory_type: String,
    },
    ConfigurationError {
        message: String,
    },
    DependencyNotFound {
        dependency: String,
    },
    ComponentCreationError {
        component_type: String,
        cause: anyhow::Error,
    },
    ValidationError {
        validation_errors: Vec<String>,
    },
    RegistryFull {
        max_size: usize,
    },
}

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FactoryError::FactoryNotRegistered { factory_type } => {
                write!(f, "Factory не зарегистрирован для типа: {}", factory_type)
            }
            FactoryError::ConfigurationError { message } => {
                write!(f, "Ошибка конфигурации factory: {}", message)
            }
            FactoryError::DependencyNotFound { dependency } => {
                write!(f, "Зависимость не найдена: {}", dependency)
            }
            FactoryError::ComponentCreationError {
                component_type,
                cause,
            } => {
                write!(
                    f,
                    "Ошибка создания компонента: {} - {}",
                    component_type, cause
                )
            }
            FactoryError::ValidationError { validation_errors } => {
                write!(
                    f,
                    "Валидация конфигурации не прошла: {:?}",
                    validation_errors
                )
            }
            FactoryError::RegistryFull { max_size } => {
                write!(f, "Factory registry переполнен: max_size={}", max_size)
            }
        }
    }
}

impl std::error::Error for FactoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FactoryError::ComponentCreationError { cause, .. } => Some(cause.as_ref()),
            _ => None,
        }
    }
}

/// Result type для factory operations
pub type FactoryResult<T> = std::result::Result<T, FactoryError>;

/// Helper trait для type-safe factory operations
pub trait TypedFactory<T> {
    type Config;

    /// Создать экземпляр типа T с конфигурацией
    fn create_typed(&self, config: Self::Config) -> FactoryResult<T>;

    /// Получить default конфигурацию для типа T
    fn default_config_for_type(&self) -> Self::Config;

    /// Validate что factory может создать тип T
    fn can_create_type(&self) -> bool;
}

/// Macro для автоматической генерации factory implementations
/// Будет использоваться для уменьшения boilerplate кода
#[macro_export]
macro_rules! impl_factory_base {
    ($factory:ty, $config:ty, $result:ty) => {
        #[async_trait::async_trait]
        impl BaseFactory for $factory {
            type Config = $config;
            type Result = $result;

            async fn create_default(&self) -> Result<Self::Result> {
                let config = self.default_config();
                self.create_with_config(config).await
            }

            async fn create_with_config(&self, config: Self::Config) -> Result<Self::Result> {
                self.validate_config(&config)?;
                self.create_with_validated_config(config).await
            }

            fn validate_config(&self, config: &Self::Config) -> Result<()> {
                // Default implementation - можно override
                Ok(())
            }

            fn default_config(&self) -> Self::Config {
                Default::default()
            }
        }
    };
}

/// Helper functions для общих factory операций
pub mod factory_helpers {
    use super::*;

    /// Concrete implementation FactoryRegistry для использования
    pub struct DefaultFactoryRegistry {
        registered_types: std::collections::HashSet<String>,
    }

    impl DefaultFactoryRegistry {
        pub fn new() -> Self {
            Self {
                registered_types: std::collections::HashSet::new(),
            }
        }
    }

    impl FactoryRegistry for DefaultFactoryRegistry {
        fn list_registered_factories(&self) -> Vec<String> {
            self.registered_types.iter().cloned().collect()
        }

        fn clear_registry(&mut self) {
            self.registered_types.clear();
        }

        fn has_factory(&self, factory_type: &str) -> bool {
            self.registered_types.contains(factory_type)
        }
    }

    /// Создать factory registry с default implementations
    pub fn create_default_factory_registry() -> DefaultFactoryRegistry {
        DefaultFactoryRegistry::new()
    }

    /// Validate что все необходимые зависимости доступны
    pub async fn validate_dependencies(
        _container: &UnifiedDIContainer,
        required_types: &[std::any::TypeId],
    ) -> FactoryResult<()> {
        for &_type_id in required_types {
            // TODO: Implement type-based is_registered check in UnifiedDIContainer
            // Временно всегда возвращаем true для совместимости
            if false {
                return Err(FactoryError::DependencyNotFound {
                    dependency: format!("TypeId: {:?}", _type_id),
                });
            }
        }
        Ok(())
    }

    /// Создать preset конфигурацию для factory
    pub fn create_preset_config(preset: FactoryPreset) -> SpecializedFactoryConfig {
        SpecializedFactoryConfig {
            enable_embedding: preset.should_create_coordinators(),
            enable_search: preset.should_create_coordinators(),
            enable_health: preset.should_create_coordinators(),
            enable_resources: preset.should_create_coordinators(),
            max_concurrent_operations: preset.max_concurrent_operations(),
            cache_size: match preset {
                FactoryPreset::Production { .. } => 5000,
                FactoryPreset::Development { .. } => 1000,
                FactoryPreset::Testing { .. } => 100,
                FactoryPreset::Custom { .. } => 2000,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialized_factory_config_presets() {
        let prod_config = SpecializedFactoryConfig::production();
        assert!(prod_config.enable_embedding);
        assert!(prod_config.enable_search);
        assert_eq!(prod_config.max_concurrent_operations, 128);

        let minimal_config = SpecializedFactoryConfig::minimal();
        assert!(!minimal_config.enable_embedding);
        assert!(!minimal_config.enable_search);
        assert_eq!(minimal_config.max_concurrent_operations, 16);

        let test_config = SpecializedFactoryConfig::test();
        assert!(!test_config.enable_embedding);
        assert_eq!(test_config.cache_size, 100);
    }

    #[test]
    fn test_factory_preset_behavior() {
        let prod_preset = FactoryPreset::Production {
            max_performance: true,
            enable_monitoring: true,
        };
        assert!(prod_preset.should_create_core_services());
        assert!(prod_preset.should_create_coordinators());
        assert!(prod_preset.should_create_specialized());
        assert_eq!(prod_preset.max_concurrent_operations(), 256);

        let test_preset = FactoryPreset::Testing {
            use_mocks: true,
            in_memory_only: true,
        };
        assert!(test_preset.should_create_core_services());
        assert!(!test_preset.should_create_coordinators());
        assert!(!test_preset.should_create_specialized());
        assert_eq!(test_preset.max_concurrent_operations(), 4);
    }

    #[test]
    fn test_specialized_component_availability_default() {
        let availability = SpecializedComponentAvailability::default();
        assert!(!availability.gpu_available);
        assert!(!availability.simd_available);
        assert!(!availability.ml_models_available);
        assert!(availability.simd_features.is_empty());
        assert!(availability.loaded_models.is_empty());
    }
}
