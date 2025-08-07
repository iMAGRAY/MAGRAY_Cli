//! Простой DI контейнер - замена всех сложных контейнеров в di/
//!
//! АРХИТЕКТУРНЫЕ РЕШЕНИЯ:
//! - Arc<dyn Any> для type-erased хранения без сложных generic constraints
//! - HashMap<TypeId, Entry> для O(1) поиска по типу
//! - Простые Singleton/Transient lifetime без сложных стратегий  
//! - Closure-based factories без trait hierarchies
//! - Clone-able контейнер для удобства передачи

use anyhow::{anyhow, Result};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Жизненный цикл сервиса - только необходимые варианты
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    /// Создается новый экземпляр при каждом разрешении
    Transient,
    /// Создается один экземпляр на весь контейнер
    Singleton,
}

/// Информация о регистрации сервиса
struct ServiceEntry {
    factory: Box<dyn Fn() -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>,
    lifetime: Lifetime,
    singleton_instance: Option<Arc<dyn Any + Send + Sync>>,
}

/// Простой DI контейнер
///
/// ЗАМЕНЯЕТ ВСЕ СУЩЕСТВУЮЩИЕ КОНТЕЙНЕРЫ:
/// - UnifiedDIContainer (di/unified_container.rs)
/// - DIContainer (di/container_builder.rs)  
/// - OptimizedUnifiedContainer (di/optimized_unified_container.rs)
/// - UnifiedContainer (di/unified_container_impl.rs)
/// - И все остальные дублированные реализации
#[derive(Clone)]
pub struct DIContainer {
    /// Зарегистрированные сервисы по TypeId
    services: Arc<RwLock<HashMap<TypeId, ServiceEntry>>>,
}

impl DIContainer {
    /// Создать новый контейнер
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Зарегистрировать singleton сервис
    pub fn register_singleton<T, F>(&self, factory: F) -> Result<()>
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let wrapped_factory = Box::new(move || -> Result<Arc<dyn Any + Send + Sync>> {
            let instance = factory()?;
            Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
        });

        let entry = ServiceEntry {
            factory: wrapped_factory,
            lifetime: Lifetime::Singleton,
            singleton_instance: None,
        };

        let mut services = self.services.write().unwrap();
        services.insert(type_id, entry);

        Ok(())
    }

    /// Зарегистрировать transient сервис
    pub fn register_transient<T, F>(&self, factory: F) -> Result<()>
    where
        T: Send + Sync + 'static,
        F: Fn() -> Result<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let wrapped_factory = Box::new(move || -> Result<Arc<dyn Any + Send + Sync>> {
            let instance = factory()?;
            Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
        });

        let entry = ServiceEntry {
            factory: wrapped_factory,
            lifetime: Lifetime::Transient,
            singleton_instance: None,
        };

        let mut services = self.services.write().unwrap();
        services.insert(type_id, entry);

        Ok(())
    }

    /// Разрешить зависимость
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        // Сначала пытаемся прочитать (большинство случаев)
        {
            let services = self.services.read().unwrap();
            if let Some(entry) = services.get(&type_id) {
                if entry.lifetime == Lifetime::Singleton {
                    if let Some(ref instance) = entry.singleton_instance {
                        // Есть cached singleton
                        return instance.clone().downcast::<T>().map_err(|_| {
                            anyhow!("Type downcast failed for {}", std::any::type_name::<T>())
                        });
                    }
                }
            }
        }

        // Нужно создать экземпляр - берем write lock
        let mut services = self.services.write().unwrap();
        let entry = services
            .get_mut(&type_id)
            .ok_or_else(|| anyhow!("Service not registered: {}", std::any::type_name::<T>()))?;

        match entry.lifetime {
            Lifetime::Singleton => {
                // Double-check pattern для singleton
                if let Some(ref instance) = entry.singleton_instance {
                    return instance.clone().downcast::<T>().map_err(|_| {
                        anyhow!("Type downcast failed for {}", std::any::type_name::<T>())
                    });
                }

                // Создаем singleton экземпляр
                let instance = (entry.factory)()?;
                entry.singleton_instance = Some(instance.clone());

                instance
                    .downcast::<T>()
                    .map_err(|_| anyhow!("Type downcast failed for {}", std::any::type_name::<T>()))
            }
            Lifetime::Transient => {
                // Всегда создаем новый экземпляр
                let instance = (entry.factory)()?;
                instance
                    .downcast::<T>()
                    .map_err(|_| anyhow!("Type downcast failed for {}", std::any::type_name::<T>()))
            }
        }
    }

    /// Попытаться разрешить зависимость (возвращает None если не зарегистрировано)
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        self.resolve::<T>().ok()
    }

    /// Проверить зарегистрирован ли сервис
    pub fn is_registered<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let services = self.services.read().unwrap();
        services.contains_key(&type_id)
    }

    /// Получить количество зарегистрированных сервисов
    pub fn service_count(&self) -> usize {
        let services = self.services.read().unwrap();
        services.len()
    }

    /// Очистить все регистрации (полезно для тестов)
    pub fn clear(&self) {
        let mut services = self.services.write().unwrap();
        services.clear();
    }

    /// Получить список зарегистрированных типов (для отладки)
    pub fn registered_types(&self) -> Vec<String> {
        let services = self.services.read().unwrap();
        services
            .keys()
            .map(|type_id| format!("{:?}", type_id))
            .collect()
    }
}

impl Default for DIContainer {
    fn default() -> Self {
        Self::new()
    }
}

// Реализуем Clone вручную из-за того что ServiceEntry не может быть Clone
// (содержит Box<dyn Fn>). Но внешне контейнер ведет себя как Clone
// так как использует Arc для внутреннего состояния.

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new(value: i32) -> Self {
            Self { value }
        }
    }

    #[test]
    fn test_singleton_lifecycle() {
        let container = DIContainer::new();

        container
            .register_singleton(|| Ok(TestService::new(42)))
            .unwrap();

        let service1 = container.resolve::<TestService>().unwrap();
        let service2 = container.resolve::<TestService>().unwrap();

        assert_eq!(service1.value, 42);
        assert_eq!(service2.value, 42);

        // Singleton - должны быть тем же экземпляром
        assert!(Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    fn test_transient_lifecycle() {
        let container = DIContainer::new();

        container
            .register_transient(|| Ok(TestService::new(42)))
            .unwrap();

        let service1 = container.resolve::<TestService>().unwrap();
        let service2 = container.resolve::<TestService>().unwrap();

        assert_eq!(service1.value, 42);
        assert_eq!(service2.value, 42);

        // Transient - должны быть разными экземплярами
        assert!(!Arc::ptr_eq(&service1, &service2));
    }

    #[test]
    fn test_unregistered_service() {
        let container = DIContainer::new();

        let result = container.resolve::<TestService>();
        assert!(result.is_err());

        let optional = container.try_resolve::<TestService>();
        assert!(optional.is_none());
    }

    #[test]
    fn test_service_registration_check() {
        let container = DIContainer::new();

        assert!(!container.is_registered::<TestService>());
        assert_eq!(container.service_count(), 0);

        container
            .register_singleton(|| Ok(TestService::new(42)))
            .unwrap();

        assert!(container.is_registered::<TestService>());
        assert_eq!(container.service_count(), 1);
    }

    #[test]
    fn test_container_clear() {
        let container = DIContainer::new();

        container
            .register_singleton(|| Ok(TestService::new(42)))
            .unwrap();

        assert_eq!(container.service_count(), 1);

        container.clear();

        assert_eq!(container.service_count(), 0);
        assert!(!container.is_registered::<TestService>());
    }

    #[test]
    fn test_container_clone() {
        let container = DIContainer::new();

        container
            .register_singleton(|| Ok(TestService::new(42)))
            .unwrap();

        let cloned_container = container.clone();

        // Клонированный контейнер должен содержать те же регистрации
        assert!(cloned_container.is_registered::<TestService>());

        let service = cloned_container.resolve::<TestService>().unwrap();
        assert_eq!(service.value, 42);
    }

    #[test]
    fn test_factory_error_handling() {
        let container = DIContainer::new();

        container
            .register_singleton(|| Err(anyhow!("Factory failed")))
            .unwrap();

        let result = container.resolve::<TestService>();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Factory failed"));
    }
}
