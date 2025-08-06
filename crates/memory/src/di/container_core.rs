use anyhow::{anyhow, Result};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use parking_lot::RwLock;
use tracing::{debug, warn};

use super::traits::{Lifetime, LifetimeManager, DependencyValidator, MetricsReporter};

/// Тип factory функции для создания компонентов
pub type Factory = Box<dyn Fn(&ContainerCore) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Основной DI контейнер, применяющий принципы SOLID
/// SRP: Отвечает только за регистрацию и разрешение зависимостей
pub struct ContainerCore {
    /// Зарегистрированные factory функции
    factories: RwLock<HashMap<TypeId, (Factory, Lifetime)>>,
    /// Имена типов для отладки
    type_names: RwLock<HashMap<TypeId, String>>,
    /// Менеджер жизненного цикла (DIP: dependency injection)
    lifetime_manager: Arc<super::lifetime_manager::LifetimeManagerImpl>,
    /// Валидатор зависимостей (DIP: dependency injection)
    dependency_validator: Arc<super::dependency_validator::DependencyValidatorImpl>,
    /// Сборщик метрик (DIP: dependency injection)
    metrics_reporter: Arc<super::metrics_collector::MetricsReporterImpl>,
}

impl ContainerCore {
    /// Создать новый контейнер с инжектированными зависимостями
    /// Применяет принцип Dependency Inversion (DIP)
    pub fn new(
        lifetime_manager: Arc<super::lifetime_manager::LifetimeManagerImpl>,
        dependency_validator: Arc<super::dependency_validator::DependencyValidatorImpl>,
        metrics_reporter: Arc<super::metrics_collector::MetricsReporterImpl>,
    ) -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            type_names: RwLock::new(HashMap::new()),
            lifetime_manager,
            dependency_validator,
            metrics_reporter,
        }
    }

    /// Получить имя типа для отладки
    pub fn get_type_name(&self, type_id: TypeId) -> String {
        self.type_names
            .read()
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| format!("Unknown type {:?}", type_id))
    }

    /// Добавить информацию о зависимости для валидации
    pub fn add_dependency_info<TDependent, TDependency>(&self) -> Result<()>
    where
        TDependent: Any + 'static,
        TDependency: Any + 'static,
    {
        let dependent_id = TypeId::of::<TDependent>();
        let dependency_id = TypeId::of::<TDependency>();
        
        self.dependency_validator.add_dependency(dependent_id, dependency_id)
    }

    /// Валидировать все зависимости
    pub fn validate_dependencies(&self) -> Result<()> {
        self.dependency_validator.validate()
    }

    /// Получить циклы зависимостей
    pub fn get_dependency_cycles(&self) -> Vec<Vec<TypeId>> {
        self.dependency_validator.get_cycles()
    }

    /// Очистить все сервисы
    pub fn clear(&self) {
        {
            let mut factories = self.factories.write();
            factories.clear();
        }
        {
            let mut type_names = self.type_names.write();
            type_names.clear();
        }
        
        self.lifetime_manager.clear_caches();
        self.dependency_validator.clear();
        self.metrics_reporter.clear_metrics();
        
        debug!("Container cleared");
    }

    /// Получить статистику контейнера
    pub fn stats(&self) -> super::traits::DIContainerStats {
        self.metrics_reporter.get_stats()
    }

    /// Получить детальные метрики производительности
    pub fn performance_metrics(&self) -> super::traits::DIPerformanceMetrics {
        self.metrics_reporter.get_performance_metrics()
    }
}

impl ContainerCore {
    /// Разрешить зависимость
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = self.get_type_name(type_id);

        // Получаем factory и lifetime
        let (factory, lifetime) = {
            let factories = self.factories.read();
            match factories.get(&type_id) {
                Some((factory, lifetime)) => {
                    // Клонируем factory для использования вне lock
                    let factory: Factory = unsafe {
                        std::mem::transmute_copy(factory)
                    };
                    (factory, *lifetime)
                }
                None => {
                    return Err(anyhow!("Type {} is not registered", type_name));
                }
            }
        };

        // Используем lifetime manager для получения экземпляра
        let factory_closure = move || -> Result<Arc<dyn Any + Send + Sync>> {
            factory(self)
        };

        let result: Arc<T> = self.lifetime_manager.get_instance(
            type_id,
            &factory_closure,
            lifetime,
        )?;

        debug!("Resolved {} with {:?} lifetime", type_name, lifetime);
        Ok(result)
    }

    /// Попытаться разрешить зависимость
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        match self.resolve::<T>() {
            Ok(instance) => Some(instance),
            Err(e) => {
                let type_name = std::any::type_name::<T>();
                debug!("Failed to resolve {}: {}", type_name, e);
                None
            }
        }
    }

    /// Проверить, зарегистрирован ли тип
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let factories = self.factories.read();
        factories.contains_key(&type_id)
    }

    /// Зарегистрировать компонент с factory функцией
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&ContainerCore) -> Result<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        let wrapped_factory: Factory = Box::new(move |resolver| {
            let instance = factory(resolver)?;
            Ok(Arc::new(instance))
        });

        {
            let mut factories = self.factories.write();
            if factories.contains_key(&type_id) {
                warn!("Type {} is already registered, overwriting", type_name);
            }
            factories.insert(type_id, (wrapped_factory, lifetime));
        }

        {
            let mut type_names = self.type_names.write();
            type_names.insert(type_id, type_name.clone());
        }

        // Уведомляем metrics reporter о регистрации
        self.metrics_reporter.record_registration(type_id);

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

        // Для экземпляров создаём простую factory функцию
        let arc_instance = Arc::new(instance);
        let wrapped_factory: Factory = Box::new(move |_| {
            Ok(arc_instance.clone())
        });

        {
            let mut factories = self.factories.write();
            factories.insert(type_id, (wrapped_factory, Lifetime::Singleton));
        }

        {
            let mut type_names = self.type_names.write();
            type_names.insert(type_id, type_name.clone());
        }

        // Уведомляем metrics reporter о регистрации
        self.metrics_reporter.record_registration(type_id);

        debug!("Registered instance of {}", type_name);
        Ok(())
    }
}

