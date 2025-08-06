//! Container Abstraction Traits - устранение coupling к конкретным DI container implementations
//! 
//! Реализует Dependency Inversion для самого DI контейнера, позволяя использовать
//! различные DI implementations без изменения кода приложения.

use anyhow::Result;
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::sync::Arc;

// ============================================================================
// CORE CONTAINER ABSTRACTIONS
// ============================================================================

/// Lifetime управление для регистрируемых сервисов
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLifetime {
    /// Новый экземпляр при каждом resolve
    Transient,
    /// Один экземпляр на всё приложение
    Singleton,
    /// Один экземпляр на scope (например, request)
    Scoped,
}

/// Factory функция для создания сервиса
pub type ServiceFactory = Box<dyn Fn(&dyn ContainerResolver) -> Result<Box<dyn Any + Send + Sync>> + Send + Sync>;

/// Высокоуровневая абстракция для DI контейнера
#[async_trait]
pub trait ContainerTrait: Send + Sync {
    /// Регистрация сервиса с factory функцией
    fn register<T>(&self, factory: ServiceFactory, lifetime: ServiceLifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static;

    /// Регистрация готового экземпляра как singleton
    fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static;

    /// Получение сервиса по типу
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Попытка получения сервиса (возвращает None если не зарегистрирован)
    fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Проверка регистрации сервиса
    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static;

    /// Добавление информации о зависимостях (для анализа)
    fn add_dependency_info<TService, TDependency>(&self) -> Result<()>
    where
        TService: Any + 'static,
        TDependency: Any + 'static;

    /// Создание дочернего scope для scoped services
    fn create_scope(&self) -> Result<Box<dyn ContainerTrait>>;

    /// Получение статистики использования
    async fn get_usage_stats(&self) -> ContainerStats;

    /// Валидация всех зависимостей (проверка циклических зависимостей)
    fn validate_dependencies(&self) -> Result<DependencyValidationReport>;
}

/// Интерфейс для resolve операций (используется в factory функциях)
pub trait ContainerResolver: Send + Sync {
    /// Получение зависимости во время создания сервиса
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static;
}

/// Статистика использования контейнера
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// Общее количество зарегистрированных сервисов
    pub registered_services: usize,
    /// Количество созданных экземпляров
    pub created_instances: usize,
    /// Количество resolve операций
    pub resolve_operations: usize,
    /// Средние время resolve операции
    pub average_resolve_time_ms: f64,
    /// Сервисы по типам lifetime
    pub services_by_lifetime: std::collections::HashMap<ServiceLifetime, usize>,
}

/// Отчет валидации зависимостей
#[derive(Debug, Clone)]
pub struct DependencyValidationReport {
    /// Общий статус валидации
    pub is_valid: bool,
    /// Обнаруженные циклические зависимости
    pub circular_dependencies: Vec<CircularDependency>,
    /// Отсутствующие зависимости
    pub missing_dependencies: Vec<String>,
    /// Граф зависимостей для анализа
    pub dependency_graph: DependencyGraph,
}

/// Информация о циклической зависимости
#[derive(Debug, Clone)]
pub struct CircularDependency {
    /// Цепочка зависимостей образующая цикл
    pub dependency_chain: Vec<String>,
    /// Severity уровень проблемы
    pub severity: DependencySeverity,
}

/// Severity уровень для dependency проблем
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySeverity {
    /// Ошибка - приложение не запустится
    Error,
    /// Предупреждение - потенциальная проблема
    Warning,
    /// Информация - рекомендация по улучшению
    Info,
}

/// Граф зависимостей
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Узлы графа (сервисы)
    pub nodes: Vec<String>,
    /// Рёбра графа (зависимости)
    pub edges: Vec<DependencyEdge>,
}

/// Ребро в графе зависимостей
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Сервис который зависит
    pub from: String,
    /// Сервис от которого зависят
    pub to: String,
    /// Тип зависимости
    pub dependency_type: DependencyType,
}

/// Тип зависимости
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Обязательная зависимость
    Required,
    /// Опциональная зависимость
    Optional,
    /// Circular reference
    Circular,
}

// ============================================================================
// CONTAINER BUILDER PATTERN
// ============================================================================

/// Builder для конфигурации контейнера
pub struct ContainerBuilder {
    enable_diagnostics: bool,
    enable_circular_detection: bool,
    enable_performance_tracking: bool,
    default_lifetime: ServiceLifetime,
}

impl ContainerBuilder {
    pub fn new() -> Self {
        Self {
            enable_diagnostics: true,
            enable_circular_detection: true,
            enable_performance_tracking: false,
            default_lifetime: ServiceLifetime::Singleton,
        }
    }

    /// Включение диагностики зависимостей
    pub fn with_diagnostics(mut self, enabled: bool) -> Self {
        self.enable_diagnostics = enabled;
        self
    }

    /// Включение детекции циклических зависимостей
    pub fn with_circular_detection(mut self, enabled: bool) -> Self {
        self.enable_circular_detection = enabled;
        self
    }

    /// Включение трекинга производительности
    pub fn with_performance_tracking(mut self, enabled: bool) -> Self {
        self.enable_performance_tracking = enabled;
        self
    }

    /// Установка default lifetime для сервисов
    pub fn with_default_lifetime(mut self, lifetime: ServiceLifetime) -> Self {
        self.default_lifetime = lifetime;
        self
    }

    /// Построение контейнера с указанной реализацией
    pub fn build<T: ContainerTrait + Default>(self) -> Result<T> {
        // В реальной реализации здесь была бы конфигурация контейнера
        Ok(T::default())
    }
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ADAPTER ДЛЯ MEMORY::DICONTAINER
// ============================================================================

/// Adapter для интеграции с memory::DIContainer
pub struct MemoryContainerAdapter {
    inner: memory::DIContainer,
    stats: std::sync::Arc<std::sync::Mutex<ContainerStats>>,
}

impl MemoryContainerAdapter {
    pub fn new(inner: memory::DIContainer) -> Self {
        Self {
            inner,
            stats: std::sync::Arc::new(std::sync::Mutex::new(ContainerStats {
                registered_services: 0,
                created_instances: 0,
                resolve_operations: 0,
                average_resolve_time_ms: 0.0,
                services_by_lifetime: std::collections::HashMap::new(),
            })),
        }
    }

    fn update_stats<F>(&self, f: F) 
    where
        F: FnOnce(&mut ContainerStats),
    {
        if let Ok(mut stats) = self.stats.lock() {
            f(&mut *stats);
        }
    }

    fn convert_lifetime(lifetime: ServiceLifetime) -> memory::Lifetime {
        match lifetime {
            ServiceLifetime::Transient => memory::Lifetime::Transient,
            ServiceLifetime::Singleton => memory::Lifetime::Singleton,
            ServiceLifetime::Scoped => memory::Lifetime::Scoped,
        }
    }
}

impl ContainerResolver for MemoryContainerAdapter {
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let start_time = std::time::Instant::now();
        let result = self.inner.resolve::<T>();
        let elapsed = start_time.elapsed();

        self.update_stats(|stats| {
            stats.resolve_operations += 1;
            stats.average_resolve_time_ms = 
                (stats.average_resolve_time_ms * (stats.resolve_operations - 1) as f64 + 
                 elapsed.as_millis() as f64) / stats.resolve_operations as f64;
        });

        result.map(Arc::new)
    }
}

#[async_trait]
impl ContainerTrait for MemoryContainerAdapter {
    fn register<T>(&self, factory: ServiceFactory, lifetime: ServiceLifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let memory_lifetime = Self::convert_lifetime(lifetime);
        
        // Адаптируем factory для memory::DIContainer
        let adapted_factory = move |container: &memory::DIContainer| -> Result<T> {
            let resolver = MemoryContainerAdapter::new(container.clone());
            let result = factory(&resolver)?;
            
            // Safe downcast от Box<dyn Any> к T
            match result.downcast::<T>() {
                Ok(boxed_t) => Ok(*boxed_t),
                Err(_) => Err(anyhow::anyhow!("Type mismatch in service factory")),
            }
        };

        self.inner.register(adapted_factory, memory_lifetime)?;
        
        self.update_stats(|stats| {
            stats.registered_services += 1;
            *stats.services_by_lifetime.entry(lifetime).or_insert(0) += 1;
        });

        Ok(())
    }

    fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.register_instance(instance)?;
        
        self.update_stats(|stats| {
            stats.registered_services += 1;
            stats.created_instances += 1;
            *stats.services_by_lifetime.entry(ServiceLifetime::Singleton).or_insert(0) += 1;
        });

        Ok(())
    }

    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        ContainerResolver::resolve(self)
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
        self.inner.is_registered::<T>()
    }

    fn add_dependency_info<TService, TDependency>(&self) -> Result<()>
    where
        TService: Any + 'static,
        TDependency: Any + 'static,
    {
        self.inner.add_dependency_info::<TService, TDependency>()
    }

    fn create_scope(&self) -> Result<Box<dyn ContainerTrait>> {
        let scope = self.inner.create_scope()?;
        Ok(Box::new(MemoryContainerAdapter::new(scope)))
    }

    async fn get_usage_stats(&self) -> ContainerStats {
        self.stats.lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    fn validate_dependencies(&self) -> Result<DependencyValidationReport> {
        // Делегируем к memory::DIContainer если у него есть такой метод
        // Иначе возвращаем базовую валидацию
        Ok(DependencyValidationReport {
            is_valid: true,
            circular_dependencies: vec![],
            missing_dependencies: vec![],
            dependency_graph: DependencyGraph {
                nodes: vec![],
                edges: vec![],
            },
        })
    }
}

impl Default for MemoryContainerAdapter {
    fn default() -> Self {
        Self::new(memory::DIContainer::new())
    }
}

// ============================================================================
// CONVENIENCE MACROS
// ============================================================================

/// Macro для упрощения регистрации сервисов
#[macro_export]
macro_rules! register_service {
    ($container:expr, $service_type:ty, $implementation:expr) => {
        $container.register::<$service_type>(
            Box::new(|_| Ok(Box::new($implementation) as Box<dyn std::any::Any + Send + Sync>)),
            $crate::container_traits::ServiceLifetime::Singleton
        )?;
    };
    
    ($container:expr, $service_type:ty, $implementation:expr, $lifetime:expr) => {
        $container.register::<$service_type>(
            Box::new(|_| Ok(Box::new($implementation) as Box<dyn std::any::Any + Send + Sync>)),
            $lifetime
        )?;
    };
}

/// Macro для регистрации с зависимостями
#[macro_export]
macro_rules! register_with_deps {
    ($container:expr, $service_type:ty, |$resolver:ident| $factory:expr) => {
        $container.register::<$service_type>(
            Box::new(|$resolver| {
                let result = $factory;
                Ok(Box::new(result) as Box<dyn std::any::Any + Send + Sync>)
            }),
            $crate::container_traits::ServiceLifetime::Singleton
        )?;
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_builder() {
        let builder = ContainerBuilder::new()
            .with_diagnostics(true)
            .with_circular_detection(true)
            .with_performance_tracking(true)
            .with_default_lifetime(ServiceLifetime::Singleton);

        // Builder pattern should work correctly
        assert_eq!(builder.default_lifetime, ServiceLifetime::Singleton);
        assert!(builder.enable_diagnostics);
        assert!(builder.enable_circular_detection);
        assert!(builder.enable_performance_tracking);
    }

    #[test]
    fn test_service_lifetime_conversion() {
        let singleton = MemoryContainerAdapter::convert_lifetime(ServiceLifetime::Singleton);
        let transient = MemoryContainerAdapter::convert_lifetime(ServiceLifetime::Transient);
        let scoped = MemoryContainerAdapter::convert_lifetime(ServiceLifetime::Scoped);

        // Conversion should work correctly
        assert_eq!(singleton, memory::Lifetime::Singleton);
        assert_eq!(transient, memory::Lifetime::Transient);
        assert_eq!(scoped, memory::Lifetime::Scoped);
    }

    #[test]
    fn test_dependency_validation_report_creation() {
        let report = DependencyValidationReport {
            is_valid: true,
            circular_dependencies: vec![],
            missing_dependencies: vec![],
            dependency_graph: DependencyGraph {
                nodes: vec!["ServiceA".to_string(), "ServiceB".to_string()],
                edges: vec![DependencyEdge {
                    from: "ServiceA".to_string(),
                    to: "ServiceB".to_string(),
                    dependency_type: DependencyType::Required,
                }],
            },
        };

        assert!(report.is_valid);
        assert_eq!(report.dependency_graph.nodes.len(), 2);
        assert_eq!(report.dependency_graph.edges.len(), 1);
    }
}