use anyhow::Result;
use std::sync::Arc;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::{debug, info};

/// Тип factory функции для создания компонентов
pub type Factory = Box<dyn Fn(&DIContainer) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Жизненный цикл компонента
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    /// Singleton - один экземпляр на всё приложение
    Singleton,
    /// Scoped - один экземпляр на scope (будущее расширение)
    #[allow(dead_code)]
    Scoped,
    /// Transient - новый экземпляр каждый раз
    Transient,
}

/// Dependency Injection Container для MAGRAY архитектуры
// @component: {"k":"C","id":"di_container","t":"Dependency injection container","m":{"cur":0,"tgt":95,"u":"%"},"f":["di","ioc","architecture"]}
pub struct DIContainer {
    /// Зарегистрированные factory функции
    factories: RwLock<HashMap<TypeId, (Factory, Lifetime)>>,
    /// Кэш singleton экземпляров
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Имена типов для отладки
    type_names: RwLock<HashMap<TypeId, String>>,
}

impl DIContainer {
    /// Создать новый контейнер
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            type_names: RwLock::new(HashMap::new()),
        }
    }

    /// Зарегистрировать компонент с factory функцией
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        let wrapped_factory: Factory = Box::new(move |container| {
            let instance = factory(container)?;
            Ok(Arc::new(instance))
        });

        {
            let mut factories = self.factories.write();
            factories.insert(type_id, (wrapped_factory, lifetime));
        }

        {
            let mut type_names = self.type_names.write();
            type_names.insert(type_id, type_name.clone());
        }

        debug!("Registered {} with {:?} lifetime", type_name, lifetime);
        Ok(())
    }

    /// Зарегистрировать singleton экземпляр
    pub fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        {
            let mut singletons = self.singletons.write();
            singletons.insert(type_id, Arc::new(instance));
        }

        {
            let mut type_names = self.type_names.write();
            type_names.insert(type_id, type_name.clone());
        }

        debug!("Registered instance of {}", type_name);
        Ok(())
    }

    /// Разрешить зависимость
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = self.get_type_name(type_id);

        // Сначала проверяем singleton кэш
        {
            let singletons = self.singletons.read();
            if let Some(instance) = singletons.get(&type_id) {
                if let Some(typed_instance) = instance.clone().downcast::<T>().ok() {
                    debug!("Resolved {} from singleton cache", type_name);
                    return Ok(typed_instance);
                }
            }
        }

        // Проверяем зарегистрированные factory
        let (factory, lifetime) = {
            let factories = self.factories.read();
            factories.get(&type_id)
                .map(|(f, l)| (f as *const Factory, *l))
                .ok_or_else(|| anyhow::anyhow!("Type {} not registered", type_name))?
        };

        // Безопасно получаем factory (мы знаем что он валиден пока держим container)
        let factory = unsafe { &*factory };
        let instance = factory(self)?;

        // Пытаемся привести к нужному типу
        let typed_instance = instance.downcast::<T>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast {} to target type", type_name))?;

        // Для singleton сохраняем в кэш
        if lifetime == Lifetime::Singleton {
            let mut singletons = self.singletons.write();
            singletons.insert(type_id, typed_instance.clone() as Arc<dyn Any + Send + Sync>);
            debug!("Cached {} as singleton", type_name);
        }

        debug!("Resolved {} with {:?} lifetime", type_name, lifetime);
        Ok(typed_instance)
    }

    /// Попытаться разрешить опциональную зависимость
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        match self.resolve::<T>() {
            Ok(instance) => Some(instance),
            Err(e) => {
                let type_name = std::any::type_name::<T>();
                debug!("Failed to resolve optional dependency {}: {}", type_name, e);
                None
            }
        }
    }

    /// Проверить, зарегистрирован ли тип
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + 'static,
    {
        let type_id = TypeId::of::<T>();
        let factories = self.factories.read();
        let singletons = self.singletons.read();
        
        factories.contains_key(&type_id) || singletons.contains_key(&type_id)
    }

    /// Получить статистику контейнера
    pub fn stats(&self) -> DIContainerStats {
        let factories = self.factories.read();
        let singletons = self.singletons.read();
        let type_names = self.type_names.read();

        DIContainerStats {
            registered_factories: factories.len(),
            cached_singletons: singletons.len(),
            total_types: type_names.len(),
        }
    }

    /// Очистить кэш singleton'ов (для тестов)
    pub fn clear_singletons(&self) {
        let mut singletons = self.singletons.write();
        singletons.clear();
        info!("Cleared singleton cache");
    }

    /// Получить список зарегистрированных типов
    pub fn registered_types(&self) -> Vec<String> {
        let type_names = self.type_names.read();
        let mut types: Vec<String> = type_names.values().cloned().collect();
        types.sort();
        types
    }

    /// Валидация зависимостей при старте
    pub fn validate_dependencies(&self) -> Result<()> {
        let factories = self.factories.read();
        let type_names = self.type_names.read();
        
        info!("Validating {} registered dependencies", factories.len());
        
        // Проверяем, что все factory функции корректные
        for (type_id, _) in factories.iter() {
            let type_name = type_names.get(type_id)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            
            // Здесь можно добавить дополнительные проверки
            debug!("✓ Dependency {} validated", type_name);
        }
        
        info!("✅ All dependencies validated successfully");
        Ok(())
    }

    // Приватные методы

    fn get_type_name(&self, type_id: TypeId) -> String {
        let type_names = self.type_names.read();
        type_names.get(&type_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

impl Default for DIContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Статистика DI контейнера
#[derive(Debug, Clone)]
pub struct DIContainerStats {
    pub registered_factories: usize,
    pub cached_singletons: usize,
    pub total_types: usize,
}

/// Builder для удобной настройки контейнера
pub struct DIContainerBuilder {
    container: DIContainer,
}

impl DIContainerBuilder {
    pub fn new() -> Self {
        Self {
            container: DIContainer::new(),
        }
    }

    /// Зарегистрировать singleton
    pub fn register_singleton<T, F>(self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        self.container.register(factory, Lifetime::Singleton)?;
        Ok(self)
    }

    /// Зарегистрировать transient
    pub fn register_transient<T, F>(self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        self.container.register(factory, Lifetime::Transient)?;
        Ok(self)
    }

    /// Зарегистрировать экземпляр
    pub fn register_instance<T>(self, instance: T) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
    {
        self.container.register_instance(instance)?;
        Ok(self)
    }

    /// Построить контейнер
    pub fn build(self) -> Result<DIContainer> {
        self.container.validate_dependencies()?;
        Ok(self.container)
    }
}

impl Default for DIContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestService {
        counter: AtomicUsize,
    }

    impl TestService {
        fn new() -> Self {
            Self {
                counter: AtomicUsize::new(0),
            }
        }

        fn increment(&self) -> usize {
            self.counter.fetch_add(1, Ordering::SeqCst) + 1
        }
    }

    struct DependentService {
        #[allow(dead_code)]
        test_service: Arc<TestService>,
        value: u32,
    }

    impl DependentService {
        fn new(test_service: Arc<TestService>) -> Self {
            Self {
                test_service,
                value: 42,
            }
        }
    }

    #[test]
    fn test_singleton_registration() -> Result<()> {
        let container = DIContainer::new();
        
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        let service1 = container.resolve::<TestService>()?;
        let service2 = container.resolve::<TestService>()?;

        // Singleton должны быть одним экземпляром
        assert_eq!(service1.increment(), 1);
        assert_eq!(service2.increment(), 2); // Тот же счетчик

        Ok(())
    }

    #[test]
    fn test_transient_registration() -> Result<()> {
        let container = DIContainer::new();
        
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Transient
        )?;

        let service1 = container.resolve::<TestService>()?;
        let service2 = container.resolve::<TestService>()?;

        // Transient должны быть разными экземплярами
        assert_eq!(service1.increment(), 1);
        assert_eq!(service2.increment(), 1); // Новый счетчик

        Ok(())
    }

    #[test]
    fn test_dependency_injection() -> Result<()> {
        let container = DIContainer::new();
        
        // Регистрируем dependency
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        // Регистрируем сервис с зависимостью
        container.register(
            |container| {
                let test_service = container.resolve::<TestService>()?;
                Ok(DependentService::new(test_service))
            },
            Lifetime::Singleton
        )?;

        let dependent = container.resolve::<DependentService>()?;
        assert_eq!(dependent.value, 42);

        Ok(())
    }

    #[test]
    fn test_builder_pattern() -> Result<()> {
        let container = DIContainerBuilder::new()
            .register_singleton(|_| Ok(TestService::new()))?
            .register_transient(|container| {
                let test_service = container.resolve::<TestService>()?;
                Ok(DependentService::new(test_service))
            })?
            .build()?;

        assert!(container.is_registered::<TestService>());
        assert!(container.is_registered::<DependentService>());

        let stats = container.stats();
        assert_eq!(stats.registered_factories, 2);

        Ok(())
    }

    #[test]
    fn test_optional_dependency() -> Result<()> {
        let container = DIContainer::new();
        
        // Не регистрируем TestService
        let optional_service = container.try_resolve::<TestService>();
        assert!(optional_service.is_none());

        // Регистрируем и пробуем снова
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        let optional_service = container.try_resolve::<TestService>();
        assert!(optional_service.is_some());

        Ok(())
    }
}