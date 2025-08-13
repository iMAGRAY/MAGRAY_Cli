//! Простые фабрики сервисов для DI контейнера
//!
//! ЗАМЕНЯЕТ ВСЕ СЛОЖНЫЕ ФАБРИЧНЫЕ СИСТЕМЫ:
//! - service_factory.rs (200+ строк)
//! - unified_factory.rs (150+ строк)  
//! - factory_traits.rs (множественные trait'ы)
//! - coordinator_factory.rs и др.
//!
//! ПРИНЦИПЫ УПРОЩЕНИЯ:
//! - Простые функции-фабрики без trait hierarchies
//! - Минимум generics и where clauses  
//! - Явные зависимости вместо Service Locator
//! - Композиция через замыкания

use super::container::DIContainer;
use anyhow::Result;
use std::sync::Arc;

/// Тип функции-фабрики для создания сервисов
pub type ServiceFactory<T> = Box<dyn Fn() -> Result<T> + Send + Sync>;

/// Упрощенная фабрика сервисов
/// Заменяет все сложные фабричные patterns из di/ папки
pub struct SimpleServiceFactory;

impl SimpleServiceFactory {
    /// Создать простую фабрику без зависимостей
    pub fn create_simple<T, F>(factory_fn: F) -> ServiceFactory<T>
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        Box::new(factory_fn)
    }

    /// Создать фабрику с одной зависимостью
    pub fn create_with_dependency<T, D, F>(
        container: DIContainer,
        factory_fn: F,
    ) -> ServiceFactory<T>
    where
        T: Send + Sync + 'static,
        D: Send + Sync + 'static,
        F: Fn(Arc<D>) -> Result<T> + Send + Sync + 'static,
    {
        Box::new(move || {
            let dependency = container.resolve::<D>()?;
            factory_fn(dependency)
        })
    }

    /// Создать фабрику с двумя зависимостями
    pub fn create_with_two_dependencies<T, D1, D2, F>(
        container: DIContainer,
        factory_fn: F,
    ) -> ServiceFactory<T>
    where
        T: Send + Sync + 'static,
        D1: Send + Sync + 'static,
        D2: Send + Sync + 'static,
        F: Fn(Arc<D1>, Arc<D2>) -> Result<T> + Send + Sync + 'static,
    {
        Box::new(move || {
            let dep1 = container.resolve::<D1>()?;
            let dep2 = container.resolve::<D2>()?;
            factory_fn(dep1, dep2)
        })
    }

    /// Создать фабрику с тремя зависимостями
    pub fn create_with_three_dependencies<T, D1, D2, D3, F>(
        container: DIContainer,
        factory_fn: F,
    ) -> ServiceFactory<T>
    where
        T: Send + Sync + 'static,
        D1: Send + Sync + 'static,
        D2: Send + Sync + 'static,
        D3: Send + Sync + 'static,
        F: Fn(Arc<D1>, Arc<D2>, Arc<D3>) -> Result<T> + Send + Sync + 'static,
    {
        Box::new(move || {
            let dep1 = container.resolve::<D1>()?;
            let dep2 = container.resolve::<D2>()?;
            let dep3 = container.resolve::<D3>()?;
            factory_fn(dep1, dep2, dep3)
        })
    }
}

/// Convenience макросы для упрощения создания фабрик
#[macro_export]
macro_rules! simple_factory {
    // Фабрика без зависимостей
    ($service:ty, $constructor:expr) => {
        Box::new(move || -> Result<$service> { Ok($constructor) }) as ServiceFactory<$service>
    };

    // Фабрика с одной зависимостью
    ($service:ty, $dep:ty, $container:expr, $constructor:expr) => {
        Box::new({
            let container = $container.clone();
            move || -> Result<$service> {
                let dep = container.resolve::<$dep>()?;
                Ok($constructor(dep))
            }
        }) as ServiceFactory<$service>
    };
}

/// Готовые фабрики для типичных паттернов
pub struct CommonFactories;

impl CommonFactories {
    /// Фабрика для Default сервисов
    pub fn default_factory<T>() -> ServiceFactory<T>
    where
        T: Default + Send + Sync + 'static,
    {
        Box::new(|| Ok(T::default()))
    }

    /// Фабрика для Clone сервисов
    pub fn clone_factory<T>(prototype: T) -> ServiceFactory<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        Box::new(move || Ok(prototype.clone()))
    }

    /// Фабрика для Arc-wrapped сервисов
    pub fn arc_factory<T>(instance: Arc<T>) -> ServiceFactory<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        Box::new(move || Ok((*instance).clone()))
    }

    /// Фабрика с ленивой инициализацией
    pub fn lazy_factory<T, F>(init_fn: F) -> ServiceFactory<T>
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        Box::new(init_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simple_di::DIContainer;

    #[derive(Debug, Clone)]
    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new(value: i32) -> Self {
            Self { value }
        }
    }

    impl Default for TestService {
        fn default() -> Self {
            Self::new(42)
        }
    }

    #[derive(Debug)]
    struct DependentService {
        dependency: Arc<TestService>,
        multiplier: i32,
    }

    impl DependentService {
        fn new(dependency: Arc<TestService>, multiplier: i32) -> Self {
            Self {
                dependency,
                multiplier,
            }
        }

        fn get_value(&self) -> i32 {
            self.dependency.value * self.multiplier
        }
    }

    #[test]
    fn test_simple_factory() {
        let factory = SimpleServiceFactory::create_simple(|| Ok(TestService::new(123)));

        let service = factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 123);
    }

    #[test]
    fn test_factory_with_dependency() {
        let container = DIContainer::new();
        container
            .register_singleton(|| Ok(TestService::new(10)))
            .expect("Simple factory operation should succeed");

        let factory = SimpleServiceFactory::create_with_dependency(
            container.clone(),
            |dep: Arc<TestService>| Ok(DependentService::new(dep, 5)),
        );

        let service = factory().expect("Simple factory operation should succeed");
        assert_eq!(service.get_value(), 50); // 10 * 5
    }

    #[test]
    fn test_factory_with_two_dependencies() {
        let container = DIContainer::new();

        // Регистрируем два сервиса как зависимости
        container
            .register_singleton(|| Ok(TestService::new(3)))
            .expect("Simple factory operation should succeed");

        container
            .register_singleton(|| Ok(7_i32)) // Простое число как вторая зависимость
            .expect("Simple factory operation should succeed");

        let factory = SimpleServiceFactory::create_with_two_dependencies(
            container.clone(),
            |dep1: Arc<TestService>, dep2: Arc<i32>| Ok(DependentService::new(dep1, *dep2)),
        );

        let service = factory().expect("Simple factory operation should succeed");
        assert_eq!(service.get_value(), 21); // 3 * 7
    }

    #[test]
    fn test_simple_factory_macro() {
        let factory = simple_factory!(TestService, TestService::new(99));
        let service = factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 99);
    }

    #[test]
    fn test_simple_factory_macro_with_dependency() {
        let container = DIContainer::new();
        container
            .register_singleton(|| Ok(TestService::new(15)))
            .expect("Simple factory operation should succeed");

        let factory = simple_factory!(DependentService, TestService, container, |dep| {
            DependentService::new(dep, 3)
        });

        let service = factory().expect("Simple factory operation should succeed");
        assert_eq!(service.get_value(), 45); // 15 * 3
    }

    #[test]
    fn test_common_factories() {
        // Default factory
        let default_factory = CommonFactories::default_factory::<TestService>();
        let service = default_factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 42);

        // Clone factory
        let prototype = TestService::new(88);
        let clone_factory = CommonFactories::clone_factory(prototype);
        let service = clone_factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 88);

        // Arc factory
        let arc_instance = Arc::new(TestService::new(77));
        let arc_factory = CommonFactories::arc_factory(arc_instance);
        let service = arc_factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 77);

        // Lazy factory
        let lazy_factory = CommonFactories::lazy_factory(|| Ok(TestService::new(66)));
        let service = lazy_factory().expect("Simple factory operation should succeed");
        assert_eq!(service.value, 66);
    }

    #[test]
    fn test_factory_error_handling() {
        let error_factory =
            SimpleServiceFactory::create_simple(|| Err(anyhow::anyhow!("Factory creation failed")));

        let result = error_factory();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Factory creation failed"));
    }

    #[test]
    fn test_factory_composition() {
        let container = DIContainer::new();

        // Регистрируем базовый сервис
        container
            .register_singleton(|| Ok(TestService::new(5)))
            .expect("Simple factory operation should succeed");

        // Создаем несколько уровней зависимостей
        let factory1 = SimpleServiceFactory::create_with_dependency(
            container.clone(),
            |dep: Arc<TestService>| Ok(DependentService::new(dep, 2)),
        );

        container.register_singleton(factory1).expect("Simple factory operation should succeed");

        // Еще один сервис, который зависит от DependentService
        let factory2 = SimpleServiceFactory::create_with_dependency(
            container.clone(),
            |dep: Arc<DependentService>| Ok(format!("Result: {}", dep.get_value())),
        );

        container.register_singleton(factory2).expect("Simple factory operation should succeed");

        let final_result = container.resolve::<String>().expect("Simple factory operation should succeed");
        assert_eq!(*final_result, "Result: 10"); // 5 * 2
    }
}
