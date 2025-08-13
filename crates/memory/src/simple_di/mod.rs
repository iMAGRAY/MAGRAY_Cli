//! Простая DI система - заменяет сложную di/ папку с 25+ файлами
//!
//! ПРИНЦИПЫ УПРОЩЕНИЯ:
//! - Единый простой контейнер без избыточных абстракций
//! - Только Singleton/Transient lifetime (без сложных стратегий)
//! - Arc<dyn Any> storage вместо сложных generic систем
//! - Простые Fn() -> Result<T> factory functions
//! - Минимум trait'ов, максимум конкретных типов

pub mod builder;
pub mod container;
pub mod integration_example;
pub mod simple_config;
pub mod simple_factory;

pub use builder::DIContainerBuilder;
pub use container::{DIContainer, Lifetime};
pub use integration_example::*;
pub use simple_config::{ConfigBuilder, SimpleConfig};
pub use simple_factory::{CommonFactories, SimpleServiceFactory};

use anyhow::Result;
use std::sync::Arc;

/// Factory function type для создания сервисов
pub type ServiceFactory<T> = Box<dyn Fn() -> Result<T> + Send + Sync>;

/// Основной API для создания DI контейнера
pub fn create_container() -> DIContainerBuilder {
    DIContainerBuilder::new()
}

/// Быстрое создание контейнера с настройками по умолчанию
pub fn create_default_container() -> DIContainer {
    DIContainer::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new() -> Self {
            Self { value: 42 }
        }
    }

    struct DependentService {
        test_service: Arc<TestService>,
        value: i32,
    }

    impl DependentService {
        fn new(test_service: Arc<TestService>) -> Self {
            Self {
                value: test_service.value * 2,
                test_service,
            }
        }
    }

    #[test]
    fn test_basic_registration_and_resolution() {
        let container = create_default_container();

        // Регистрируем singleton
        container
            .register_singleton(|| Ok(TestService::new()))
            .expect("Failed to register singleton TestService in basic registration test");

        // Разрешаем
        let service = container.resolve::<TestService>().expect("Failed to resolve TestService in basic registration test");
        assert_eq!(service.value, 42);

        // Singleton - должны получить тот же экземпляр
        let service2 = container.resolve::<TestService>().expect("Failed to resolve TestService (singleton check) in basic registration test");
        assert!(Arc::ptr_eq(&service, &service2));
    }

    #[test]
    fn test_transient_services() {
        let container = create_default_container();

        // Регистрируем transient
        container
            .register_transient(|| Ok(TestService::new()))
            .expect("Failed to register transient TestService in transient services test");

        // Transient - должны получить разные экземпляры
        let service1 = container.resolve::<TestService>().expect("Failed to resolve TestService (first) in transient services test");
        let service2 = container.resolve::<TestService>().expect("Failed to resolve TestService (second) in transient services test");
        assert!(!Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    fn test_dependency_injection() {
        let container = create_default_container();

        // Регистрируем базовый сервис
        container
            .register_singleton(|| Ok(TestService::new()))
            .expect("Failed to register singleton TestService in dependency injection test");

        // Регистрируем зависимый сервис с manual injection
        container
            .register_singleton({
                let container = container.clone();
                move || {
                    let test_service = container.resolve::<TestService>()?;
                    Ok(DependentService::new(test_service))
                }
            })
            .expect("Failed to register DependentService in dependency injection test");

        let dependent = container.resolve::<DependentService>().expect("Failed to resolve DependentService in dependency injection test");
        assert_eq!(dependent.value, 84); // 42 * 2
    }

    #[test]
    fn test_builder_pattern() {
        let container = create_container()
            .register_singleton::<TestService>(|| Ok(TestService::new()))
            .register_transient::<DependentService>({
                let container = create_default_container();
                move || {
                    let test_service = container.resolve::<TestService>()?;
                    Ok(DependentService::new(test_service))
                }
            })
            .build();

        let dependent = container.resolve::<DependentService>().expect("Failed to resolve DependentService in builder pattern test");
        assert_eq!(dependent.value, 84);
    }

    #[test]
    fn test_optional_resolution() {
        let container = create_default_container();

        // Попытка разрешить незарегистрированный сервис
        let service = container.try_resolve::<TestService>();
        assert!(service.is_none());

        // Регистрируем и пробуем еще раз
        container
            .register_singleton(|| Ok(TestService::new()))
            .expect("Failed to register TestService in optional resolution test");

        let service = container.try_resolve::<TestService>();
        assert!(service.is_some());
        assert_eq!(service.expect("TestService should be available after registration").value, 42);
    }
}
