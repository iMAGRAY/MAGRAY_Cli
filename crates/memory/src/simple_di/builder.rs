//! Builder pattern для упрощенной настройки DI контейнера
//!
//! Предоставляет fluent API для регистрации сервисов и создания контейнера.
//! Заменяет сложные builder'ы из di/ папки простым и понятным интерфейсом.

use super::container::{DIContainer, Lifetime};
use anyhow::Result;
use std::sync::Arc;

/// Builder для создания DI контейнера
///
/// ЗАМЕНЯЕТ:
/// - DIContainerBuilder (di/container_builder.rs)
/// - UnifiedDIContainerBuilder (di/unified_container.rs)
/// - OptimizedContainerBuilder (di/optimized_unified_container.rs)
/// - ContainerBuilder (di/container_factory.rs)
pub struct DIContainerBuilder {
    container: DIContainer,
}

impl DIContainerBuilder {
    /// Создать новый builder
    pub fn new() -> Self {
        Self {
            container: DIContainer::new(),
        }
    }

    /// Зарегистрировать singleton сервис
    pub fn register_singleton<T, F>(self, factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.container.register_singleton(factory).unwrap();
        self
    }

    /// Зарегистрировать transient сервис
    pub fn register_transient<T, F>(self, factory: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        self.container.register_transient(factory).unwrap();
        self
    }

    /// Зарегистрировать сервис с указанным lifetime
    pub fn register<T, F>(self, factory: F, lifetime: Lifetime) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        match lifetime {
            Lifetime::Singleton => self.register_singleton(factory),
            Lifetime::Transient => self.register_transient(factory),
        }
    }

    /// Зарегистрировать instance как singleton
    pub fn register_instance<T>(self, instance: T) -> Self
    where
        T: Send + Sync + 'static + Clone,
    {
        self.register_singleton(move || Ok(instance.clone()))
    }

    /// Создать итоговый контейнер
    pub fn build(self) -> DIContainer {
        self.container
    }
}

impl Default for DIContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience методы для частых паттернов
impl DIContainerBuilder {
    /// Зарегистрировать простой сервис без зависимостей как singleton
    pub fn add_singleton<T>(self) -> Self
    where
        T: Send + Sync + 'static + Default,
    {
        self.register_singleton(|| Ok(T::default()))
    }

    /// Зарегистрировать простой сервис без зависимостей как transient
    pub fn add_transient<T>(self) -> Self
    where
        T: Send + Sync + 'static + Default,
    {
        self.register_transient(|| Ok(T::default()))
    }

    /// Зарегистрировать Arc-wrapped instance
    pub fn register_arc<T>(self, instance: Arc<T>) -> Self
    where
        T: Send + Sync + 'static + Clone,
    {
        self.register_singleton(move || Ok((*instance).clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[derive(Debug, Clone)]
    struct DependentService {
        dependency: Arc<TestService>,
        value: i32,
    }

    impl DependentService {
        fn new(dependency: Arc<TestService>) -> Self {
            Self {
                value: dependency.value * 2,
                dependency,
            }
        }
    }

    #[test]
    fn test_basic_builder() {
        let container = DIContainerBuilder::new()
            .register_singleton::<TestService>(|| Ok(TestService::new(42)))
            .register_transient::<TestService>(|| Ok(TestService::new(100)))
            .build();

        // После повторной регистрации должен остаться последний (transient)
        let service1 = container.resolve::<TestService>().unwrap();
        let service2 = container.resolve::<TestService>().unwrap();

        assert_eq!(service1.value, 100);
        assert_eq!(service2.value, 100);
        assert!(!Arc::ptr_eq(&service1, &service2)); // transient
    }

    #[test]
    fn test_register_with_lifetime() {
        let container = DIContainerBuilder::new()
            .register(|| Ok(TestService::new(42)), Lifetime::Singleton)
            .build();

        let service1 = container.resolve::<TestService>().unwrap();
        let service2 = container.resolve::<TestService>().unwrap();

        assert_eq!(service1.value, 42);
        assert!(Arc::ptr_eq(&service1, &service2)); // singleton
    }

    #[test]
    fn test_register_instance() {
        let instance = TestService::new(99);

        let container = DIContainerBuilder::new()
            .register_instance(instance)
            .build();

        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 99);
    }

    #[test]
    fn test_convenience_methods() {
        let container = DIContainerBuilder::new()
            .add_singleton::<TestService>()
            .build();

        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 42); // default value
    }

    #[test]
    fn test_arc_registration() {
        let arc_instance = Arc::new(TestService::new(77));

        let container = DIContainerBuilder::new().register_arc(arc_instance).build();

        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 77);
    }

    #[test]
    fn test_dependency_injection_with_builder() {
        let container = DIContainerBuilder::new()
            .register_singleton::<TestService>(|| Ok(TestService::new(42)))
            .build();

        // Регистрируем зависимый сервис после создания контейнера
        container
            .register_singleton({
                let container = container.clone();
                move || {
                    let dependency = container.resolve::<TestService>()?;
                    Ok(DependentService::new(dependency))
                }
            })
            .unwrap();

        let dependent = container.resolve::<DependentService>().unwrap();
        assert_eq!(dependent.value, 84); // 42 * 2
    }

    #[test]
    fn test_builder_chaining() {
        let container = DIContainerBuilder::new()
            .add_singleton::<TestService>()
            .register_transient::<TestService>(|| Ok(TestService::new(200)))
            .register_instance(TestService::new(300))
            .build();

        // Последняя регистрация побеждает
        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 300);
    }

    #[test]
    fn test_empty_container() {
        let container = DIContainerBuilder::new().build();

        assert_eq!(container.service_count(), 0);
        assert!(!container.is_registered::<TestService>());

        let result = container.try_resolve::<TestService>();
        assert!(result.is_none());
    }

    #[test]
    fn test_default_builder() {
        let container = DIContainerBuilder::default()
            .add_singleton::<TestService>()
            .build();

        let service = container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 42);
    }
}
