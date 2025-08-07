use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tracing::{debug, warn};

use super::{
    object_safe_resolver::ObjectSafeResolver,
    traits::{DependencyValidator, Lifetime, LifetimeManager, MetricsReporter},
};

/// Тип factory функции для создания компонентов
pub type Factory = Box<
    dyn Fn(&ContainerCore) -> Result<Arc<dyn Any + Send + Sync + 'static>> + Send + Sync + 'static,
>;

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

// NOTE: Clone удален из-за parking_lot::RwLock не имплементирующего Clone
// Вместо этого будем оборачивать ContainerCore в Arc и клонировать Arc

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

        self.dependency_validator
            .add_dependency(dependent_id, dependency_id)
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

    /// Получить краткий отчет о производительности в формате строки
    pub fn get_performance_report(&self) -> String {
        let metrics = self.performance_metrics();
        format!(
            "DI Container Performance Report:\n\
            - Total resolutions: {}\n\
            - Total resolution time: {:?}\n\
            - Average resolution time: {:?}\n\
            - Error count: {}\n\
            - Success rate: {:.2}%",
            metrics.total_resolutions,
            metrics.total_resolution_time,
            if metrics.total_resolutions > 0 {
                metrics.total_resolution_time / metrics.total_resolutions as u32
            } else {
                std::time::Duration::from_secs(0)
            },
            metrics.error_count,
            if metrics.total_resolutions > 0 {
                (metrics.total_resolutions - metrics.error_count) as f64
                    / metrics.total_resolutions as f64
                    * 100.0
            } else {
                100.0
            }
        )
    }

    /// Сбросить метрики производительности
    pub fn reset_performance_metrics(&self) {
        self.metrics_reporter.clear_metrics();
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

        // Проверяем наличие factory
        if !self.factories.read().contains_key(&type_id) {
            return Err(anyhow!("Type {} is not registered", type_name));
        }

        // Используем resolve_with_lifetime для обработки
        self.resolve_with_lifetime::<T>(type_id, Lifetime::Transient)
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
            let arc_instance: Arc<dyn Any + Send + Sync + 'static> = Arc::new(instance);
            Ok(arc_instance)
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
            let cloned: Arc<dyn Any + Send + Sync + 'static> = arc_instance.clone();
            Ok(cloned)
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

// Реализация DIResolver для ContainerCore
impl super::traits::DIResolver for ContainerCore {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // Поиск factory в контейнере
        let lifetime = {
            let factories = self.factories.read();
            match factories.get(&type_id) {
                Some((_factory, lifetime)) => *lifetime,
                None => {
                    return Err(anyhow!("Type {} is not registered", type_name));
                }
            }
        };

        self.resolve_with_lifetime::<T>(type_id, lifetime)
    }

    fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.resolve().ok()
    }

    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        self.factories.read().contains_key(&type_id)
    }
}

impl ContainerCore {
    /// Resolve with specific lifetime handling
    fn resolve_with_lifetime<T>(&self, type_id: TypeId, _lifetime: Lifetime) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();

        // Получаем factory из хранилища
        let factories = self.factories.read();
        match factories.get(&type_id) {
            Some((factory_ref, _)) => {
                // Создаем экземпляр через factory
                let instance = factory_ref(self)?;
                // Пытаемся привести к нужному типу
                match instance.downcast::<T>() {
                    Ok(typed_instance) => {
                        debug!("Successfully resolved {}", type_name);
                        Ok(typed_instance)
                    }
                    Err(_) => Err(anyhow!("Failed to downcast instance to type {}", type_name)),
                }
            }
            None => Err(anyhow!("Type {} is not registered", type_name)),
        }
    }
}

/// Реализация ObjectSafeResolver для ContainerCore
/// Это позволяет использовать ContainerCore с trait objects
impl ObjectSafeResolver for ContainerCore {
    fn resolve_by_type_id(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync + 'static>> {
        let factories = self.factories.read();
        let type_names = self.type_names.read();

        let type_name = type_names
            .get(&type_id)
            .map(|s| s.as_str())
            .unwrap_or("Unknown");

        match factories.get(&type_id) {
            Some((factory, _lifetime)) => {
                debug!("Resolving {} using object-safe resolver", type_name);
                factory(self)
            }
            None => {
                let registered: Vec<String> = type_names.values().cloned().collect();
                Err(anyhow!(
                    "Type {} is not registered. Available types: [{}]",
                    type_name,
                    registered.join(", ")
                ))
            }
        }
    }

    fn try_resolve_by_type_id(
        &self,
        type_id: TypeId,
    ) -> Option<Arc<dyn Any + Send + Sync + 'static>> {
        let factories = self.factories.read();

        if let Some((factory, _lifetime)) = factories.get(&type_id) {
            factory(self).ok()
        } else {
            None
        }
    }

    fn is_registered_by_type_id(&self, type_id: TypeId) -> bool {
        let factories = self.factories.read();
        factories.contains_key(&type_id)
    }

    fn get_registered_types(&self) -> Vec<TypeId> {
        let factories = self.factories.read();
        factories.keys().cloned().collect()
    }

    fn get_type_name(&self, type_id: TypeId) -> Option<String> {
        let type_names = self.type_names.read();
        type_names.get(&type_id).cloned()
    }
}
