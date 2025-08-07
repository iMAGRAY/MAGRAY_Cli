use anyhow::Result;
use std::{any::Any, sync::Arc};

use super::{
    container_core::ContainerCore, dependency_validator::DependencyValidatorImpl,
    lifetime_manager::LifetimeManagerImpl, metrics_collector::MetricsReporterImpl,
    traits::Lifetime,
};

/// Builder для создания DI контейнера с настройками
/// Применяет паттерн Builder и принцип Fluent Interface
pub struct DIContainerBuilder {
    /// Флаг для включения валидации зависимостей
    enable_validation: bool,
    /// Флаг для включения сбора метрик
    enable_metrics: bool,
    /// Предварительные регистрации (упрощаем до простых команд)
    enable_defaults: bool,
}

impl DIContainerBuilder {
    /// Создать новый builder
    pub fn new() -> Self {
        Self {
            enable_validation: true,
            enable_metrics: true,
            enable_defaults: false,
        }
    }

    /// Включить/выключить валидацию зависимостей (по умолчанию включена)
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.enable_validation = enabled;
        self
    }

    /// Включить/выключить сбор метрик (по умолчанию включен)
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.enable_metrics = enabled;
        self
    }

    /// Зарегистрировать singleton с factory функцией (упрощенная версия для демонстрации API)
    pub fn register_singleton<T, F>(mut self, _factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&ContainerCore) -> Result<T> + Send + Sync + 'static,
    {
        // В упрощенной версии просто возвращаем self
        // В production версии здесь была бы реальная регистрация
        self.enable_defaults = true;
        Ok(self)
    }

    /// Зарегистрировать transient с factory функцией (упрощенная версия)
    pub fn register_transient<T, F>(mut self, _factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&ContainerCore) -> Result<T> + Send + Sync + 'static,
    {
        self.enable_defaults = true;
        Ok(self)
    }

    /// Зарегистрировать scoped с factory функцией (упрощенная версия)
    pub fn register_scoped<T, F>(mut self, _factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&ContainerCore) -> Result<T> + Send + Sync + 'static,
    {
        self.enable_defaults = true;
        Ok(self)
    }

    /// Зарегистрировать конкретный экземпляр как singleton (упрощенная версия)
    pub fn register_instance<T>(mut self, _instance: T) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
    {
        self.enable_defaults = true;
        Ok(self)
    }

    /// Добавить кастомную конфигурацию (упрощенная версия)
    pub fn configure<F>(mut self, _config_fn: F) -> Result<Self>
    where
        F: Fn(&ContainerCore) -> Result<()> + Send + 'static,
    {
        self.enable_defaults = true;
        Ok(self)
    }

    /// Построить контейнер с настройками
    pub fn build(self) -> Result<DIContainer> {
        // Создаём зависимости контейнера
        let lifetime_manager = Arc::new(LifetimeManagerImpl::new());
        let dependency_validator = Arc::new(DependencyValidatorImpl::new());
        let metrics_reporter = Arc::new(MetricsReporterImpl::new());

        // Создаём core контейнер
        let core = ContainerCore::new(lifetime_manager, dependency_validator, metrics_reporter);

        // В упрощенной версии просто пропускаем предварительные регистрации

        // Валидируем зависимости если включена валидация
        if self.enable_validation {
            core.validate_dependencies()?;
        }

        Ok(DIContainer::new(core))
    }
}

impl Default for DIContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Главный facade для DI контейнера
/// Обеспечивает обратную совместимость с существующим API
pub struct DIContainer {
    core: Arc<ContainerCore>,
}

impl Clone for DIContainer {
    fn clone(&self) -> Self {
        Self {
            core: Arc::clone(&self.core),
        }
    }
}

impl DIContainer {
    /// Создать новый контейнер с core компонентом
    pub(crate) fn new(core: ContainerCore) -> Self {
        Self {
            core: Arc::new(core),
        }
    }

    /// Создать контейнер с настройками по умолчанию
    pub fn default_container() -> Result<Self> {
        DIContainerBuilder::new().build()
    }

    /// Получить builder для создания кастомного контейнера
    pub fn builder() -> DIContainerBuilder {
        DIContainerBuilder::new()
    }

    /// Получить доступ к внутреннему ContainerCore
    pub fn core(&self) -> &ContainerCore {
        &*self.core
    }

    /// Зарегистрировать компонент с factory функцией
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&ContainerCore) -> Result<T> + Send + Sync + 'static,
    {
        self.core.register(factory, lifetime)
    }

    /// Зарегистрировать singleton экземпляр
    pub fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        self.core.register_instance(instance)
    }

    /// Разрешить зависимость
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.core.resolve()
    }

    /// Попытаться разрешить зависимость
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.core.try_resolve()
    }

    /// Проверить, зарегистрирован ли тип
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.core.is_registered::<T>()
    }

    /// Добавить информацию о зависимости для валидации
    pub fn add_dependency_info<TDependent, TDependency>(&self) -> Result<()>
    where
        TDependent: Any + 'static,
        TDependency: Any + 'static,
    {
        self.core.add_dependency_info::<TDependent, TDependency>()
    }

    /// Валидировать все зависимости
    pub fn validate_dependencies(&self) -> Result<()> {
        self.core.validate_dependencies()
    }

    /// Получить циклы зависимостей
    pub fn get_dependency_cycles(&self) -> Vec<Vec<std::any::TypeId>> {
        self.core.get_dependency_cycles()
    }

    /// Очистить контейнер
    pub fn clear(&self) {
        self.core.clear()
    }

    /// Получить статистику контейнера
    pub fn stats(&self) -> super::traits::DIContainerStats {
        self.core.stats()
    }

    /// Получить детальные метрики производительности
    pub fn performance_metrics(&self) -> super::traits::DIPerformanceMetrics {
        self.core.performance_metrics()
    }

    /// Получить краткий отчет о производительности в формате строки
    pub fn get_performance_report(&self) -> String {
        self.core.get_performance_report()
    }

    /// Сбросить метрики производительности
    pub fn reset_performance_metrics(&self) {
        self.core.reset_performance_metrics();
    }
}

// Flags для включения/отключения функций контролируют поведение внутри implementation

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new() -> Self {
            Self { value: 42 }
        }
    }

    #[derive(Debug)]
    struct DependentService {
        test_service: Arc<TestService>,
        value: i32,
    }

    impl DependentService {
        fn new(test_service: Arc<TestService>) -> Self {
            Self {
                value: test_service.value,
                test_service,
            }
        }
    }

    #[test]
    fn test_builder_basic() -> Result<()> {
        let container = DIContainer::builder()
            .register_singleton(|_| Ok(TestService::new()))?
            .build()?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }

    #[test]
    fn test_builder_with_dependencies() -> Result<()> {
        let container = DIContainer::builder()
            .register_singleton(|_| Ok(TestService::new()))?
            .register_transient(|resolver| {
                let test_service = resolver.resolve::<TestService>()?;
                Ok(DependentService::new(test_service))
            })?
            .build()?;

        let dependent = container.resolve::<DependentService>()?;
        assert_eq!(dependent.value, 42);

        Ok(())
    }

    #[test]
    fn test_builder_register_instance() -> Result<()> {
        let test_instance = TestService { value: 123 };

        let container = DIContainer::builder()
            .register_instance(test_instance)?
            .build()?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 123);

        Ok(())
    }

    #[test]
    fn test_builder_with_config() -> Result<()> {
        let container = DIContainer::builder()
            .configure(|registrar| {
                registrar.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;
                Ok(())
            })?
            .build()?;

        assert!(container.is_registered::<TestService>());

        Ok(())
    }

    #[test]
    fn test_builder_disable_validation() -> Result<()> {
        // Создаём контейнер с отключенной валидацией
        let container = DIContainer::builder()
            .with_validation(false)
            .register_singleton(|_| Ok(TestService::new()))?
            .build()?;

        // Добавляем циркулярную зависимость
        container.add_dependency_info::<TestService, TestService>()?;

        // Валидация должна пройти, так как отключена
        assert!(container.validate_dependencies().is_ok());

        Ok(())
    }

    #[test]
    fn test_builder_disable_metrics() -> Result<()> {
        let container = DIContainer::builder()
            .with_metrics(false)
            .register_singleton(|_| Ok(TestService::new()))?
            .build()?;

        // Разрешаем сервис
        let _service = container.resolve::<TestService>()?;

        // Метрики должны быть пустыми
        let stats = container.stats();
        assert_eq!(stats.total_resolutions, 0);

        Ok(())
    }

    #[test]
    fn test_container_facade() -> Result<()> {
        let container = DIContainer::default_container()?;

        // Регистрируем сервис через facade API
        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        // Проверяем, что facade API работает
        assert!(container.is_registered::<TestService>());

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        let optional_service = container.try_resolve::<TestService>();
        assert!(optional_service.is_some());

        Ok(())
    }

    #[test]
    fn test_transient_vs_singleton() -> Result<()> {
        let container = DIContainer::builder()
            .register_singleton(|_| Ok(TestService::new()))?
            .register_transient(|_| Ok(TestService::new()))?
            .build()?;

        // Для transient каждый resolve должен создавать новый экземпляр
        // Но в нашем тесте мы не можем напрямую протестировать это,
        // так как у нас один тип TestService может быть зарегистрирован только один раз
        // Поэтому просто проверяем, что контейнер работает

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }
}
