//! Service Resolver Implementation - разрешение зависимостей в DI контейнере
//!
//! Отделен от unified_container_impl.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за разрешение зависимостей используя зарегистрированные фабрики.

use anyhow::Result as AnyResult;
use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, error, warn};

use super::{
    container_cache::ContainerCache, core_traits::LifetimeStrategy, errors::DIError,
    service_registry_impl::ServiceRegistryImpl,
};

/// Service Resolver Implementation - отвечает ТОЛЬКО за разрешение зависимостей
///
/// ПРИНЦИПЫ:
/// - SRP: единственная ответственность - разрешение зависимостей
/// - OCP: расширяемость через различные стратегии кэширования
/// - LSP: соответствует интерфейсу ServiceResolver
/// - ISP: минимальный интерфейс только для разрешения
/// - DIP: зависит от абстракций (ServiceRegistry, ContainerCache)
pub struct ServiceResolverImpl {
    /// Registry для получения фабрик
    registry: Arc<ServiceRegistryImpl>,
    /// Cache manager для singleton и scoped экземпляров
    cache: Arc<ContainerCache>,
    /// Конфигурация resolver
    config: ResolverConfig,
    /// Метрики разрешения
    metrics: RwLock<ResolverMetrics>,
}

/// Конфигурация для service resolver
#[derive(Debug, Clone)]
pub struct ResolverConfig {
    /// Максимальное время разрешения одного типа
    pub max_resolution_time: Duration,
    /// Максимальная глубина стека разрешения (для предотвращения циклов)
    pub max_resolution_depth: usize,
    /// Включить подробное логирование разрешений
    pub verbose_logging: bool,
    /// Включить кэширование результатов
    pub enable_caching: bool,
    /// Timeout для создания экземпляров
    pub instance_creation_timeout: Duration,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            max_resolution_time: Duration::from_secs(30),
            max_resolution_depth: 50,
            verbose_logging: cfg!(debug_assertions),
            enable_caching: true,
            instance_creation_timeout: Duration::from_secs(10),
        }
    }
}

impl ResolverConfig {
    /// Production конфигурация с оптимизированными параметрами
    pub fn production() -> Self {
        Self {
            max_resolution_time: Duration::from_secs(10),
            max_resolution_depth: 25,
            verbose_logging: false,
            enable_caching: true,
            instance_creation_timeout: Duration::from_secs(5),
        }
    }

    /// Development конфигурация с расширенным debugging
    pub fn development() -> Self {
        Self {
            max_resolution_time: Duration::from_secs(60),
            max_resolution_depth: 100,
            verbose_logging: true,
            enable_caching: true,
            instance_creation_timeout: Duration::from_secs(30),
        }
    }

    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            max_resolution_time: Duration::from_secs(5),
            max_resolution_depth: 10,
            verbose_logging: false,
            enable_caching: false,
            instance_creation_timeout: Duration::from_secs(2),
        }
    }
}

/// Метрики разрешения зависимостей
#[derive(Debug, Default, Clone)]
pub struct ResolverMetrics {
    /// Общее количество разрешений
    pub total_resolutions: u64,
    /// Успешные разрешения
    pub successful_resolutions: u64,
    /// Неудачные разрешения
    pub failed_resolutions: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Общее время всех разрешений
    pub total_resolution_time: Duration,
    /// Максимальное время разрешения
    pub max_resolution_time: Duration,
    /// Максимальная глубина разрешения
    pub max_resolution_depth: usize,
    /// Количество timeout-ов
    pub timeout_count: u64,
}

impl ResolverMetrics {
    /// Получить hit rate кэша
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Получить success rate разрешений
    pub fn success_rate(&self) -> f64 {
        if self.total_resolutions > 0 {
            (self.successful_resolutions as f64 / self.total_resolutions as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Получить среднее время разрешения
    pub fn avg_resolution_time(&self) -> Duration {
        if self.total_resolutions > 0 {
            self.total_resolution_time / self.total_resolutions as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Context для отслеживания стека разрешений (предотвращение циклов)
#[derive(Debug)]
struct ResolutionContext {
    /// Стек типов которые сейчас разрешаются
    resolution_stack: Vec<TypeId>,
    /// Время начала разрешения
    start_time: Instant,
}

impl ResolutionContext {
    /// Создать новый контекст
    fn new() -> Self {
        Self {
            resolution_stack: Vec::new(),
            start_time: Instant::now(),
        }
    }

    /// Проверить есть ли цикл
    fn has_cycle(&self, type_id: TypeId) -> bool {
        self.resolution_stack.contains(&type_id)
    }

    /// Добавить тип в стек
    fn push(&mut self, type_id: TypeId) {
        self.resolution_stack.push(type_id);
    }

    /// Убрать тип из стека
    fn pop(&mut self) -> Option<TypeId> {
        self.resolution_stack.pop()
    }

    /// Получить глубину стека
    fn depth(&self) -> usize {
        self.resolution_stack.len()
    }

    /// Получить время с начала разрешения
    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

impl ServiceResolverImpl {
    /// Создать новый service resolver
    pub fn new(
        registry: Arc<ServiceRegistryImpl>,
        cache: Arc<ContainerCache>,
        config: ResolverConfig,
    ) -> Self {
        debug!(
            "🔍 Создание ServiceResolverImpl с max_depth={}",
            config.max_resolution_depth
        );

        Self {
            registry,
            cache,
            config,
            metrics: RwLock::new(ResolverMetrics::default()),
        }
    }

    /// Разрешить зависимость по TypeId
    pub fn resolve_type_erased(
        &self,
        type_id: TypeId,
    ) -> Result<Arc<dyn Any + Send + Sync>, DIError> {
        let mut context = ResolutionContext::new();
        self.resolve_internal(type_id, &mut context)
    }

    /// Попытаться разрешить зависимость (безопасная версия)
    pub fn try_resolve_type_erased(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        match self.resolve_type_erased(type_id) {
            Ok(instance) => Some(instance),
            Err(e) => {
                if self.config.verbose_logging {
                    debug!("🔍 try_resolve не удался для {:?}: {}", type_id, e);
                }
                None
            }
        }
    }

    /// Проверить может ли тип быть разрешен
    pub fn can_resolve(&self, type_id: TypeId) -> bool {
        self.registry.is_registered(type_id)
    }

    /// Получить метрики разрешения
    pub fn get_metrics(&self) -> ResolverMetrics {
        self.metrics.read().clone()
    }

    /// Сбросить метрики
    pub fn reset_metrics(&self) {
        let mut metrics = self.metrics.write();
        *metrics = ResolverMetrics::default();
        debug!("🔄 Метрики ServiceResolver сброшены");
    }

    /// Получить детальный отчет о разрешениях
    pub fn get_detailed_report(&self) -> String {
        let metrics = self.get_metrics();
        let cache_stats = self.cache.stats();

        format!(
            "=== Service Resolver Detailed Report ===\n\
             Total resolutions: {}\n\
             - Successful: {} ({:.1}%)\n\
             - Failed: {} ({:.1}%)\n\
             - Timeouts: {}\n\
             Cache performance:\n\
             - Hits: {} ({:.1}%)\n\
             - Misses: {}\n\
             - Cache utilization: {:.1}%\n\
             Performance metrics:\n\
             - Average resolution time: {:?}\n\
             - Maximum resolution time: {:?}\n\
             - Maximum resolution depth: {}\n\
             =======================================",
            metrics.total_resolutions,
            metrics.successful_resolutions,
            metrics.success_rate(),
            metrics.failed_resolutions,
            100.0 - metrics.success_rate(),
            metrics.timeout_count,
            metrics.cache_hits,
            metrics.cache_hit_rate(),
            metrics.cache_misses,
            cache_stats.cache_utilization,
            metrics.avg_resolution_time(),
            metrics.max_resolution_time,
            metrics.max_resolution_depth
        )
    }

    /// Validate состояние resolver
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let metrics = self.get_metrics();

        // Проверяем success rate
        if metrics.total_resolutions > 100 && metrics.success_rate() < 50.0 {
            errors.push(format!(
                "Низкий success rate: {:.1}% (менее 50%)",
                metrics.success_rate()
            ));
        }

        // Проверяем timeout-ы
        if metrics.timeout_count > metrics.total_resolutions / 10 {
            errors.push(format!(
                "Слишком много timeout-ов: {} из {} ({:.1}%)",
                metrics.timeout_count,
                metrics.total_resolutions,
                (metrics.timeout_count as f64 / metrics.total_resolutions as f64) * 100.0
            ));
        }

        // Проверяем среднее время разрешения
        if metrics.avg_resolution_time() > Duration::from_millis(100) {
            errors.push(format!(
                "Медленное разрешение зависимостей: среднее время {:?}",
                metrics.avg_resolution_time()
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // === PRIVATE IMPLEMENTATION METHODS ===

    /// Внутренняя реализация разрешения с контекстом
    fn resolve_internal(
        &self,
        type_id: TypeId,
        context: &mut ResolutionContext,
    ) -> Result<Arc<dyn Any + Send + Sync>, DIError> {
        let start_time = Instant::now();

        // Увеличиваем счетчик попыток разрешения
        {
            let mut metrics = self.metrics.write();
            metrics.total_resolutions += 1;
        }

        // Проверяем timeout общего разрешения
        if context.elapsed() > self.config.max_resolution_time {
            self.record_timeout();
            return Err(DIError::ResolutionTimeout {
                type_id,
                timeout: self.config.max_resolution_time,
            });
        }

        // Проверяем глубину стека
        if context.depth() >= self.config.max_resolution_depth {
            self.record_failure();
            return Err(DIError::MaxDepthExceeded {
                max_depth: self.config.max_resolution_depth,
                current_depth: context.depth(),
            });
        }

        // Проверяем циклические зависимости
        if context.has_cycle(type_id) {
            self.record_failure();
            return Err(DIError::CircularDependency {
                dependency_chain: context.resolution_stack.clone(),
            });
        }

        // Проверяем кэш если кэширование включено
        if self.config.enable_caching {
            if let Some(cached) = self.try_get_from_cache(type_id) {
                self.record_cache_hit();
                self.record_success(start_time);

                if self.config.verbose_logging {
                    debug!(
                        "✅ Разрешен {:?} из кэша за {:?}",
                        type_id,
                        start_time.elapsed()
                    );
                }

                return Ok(cached);
            } else {
                self.record_cache_miss();
            }
        }

        // Добавляем тип в стек разрешения
        context.push(type_id);

        // Получаем и выполняем фабрику
        let result = self.registry.with_factory(type_id, |factory_info| {
            if self.config.verbose_logging {
                debug!(
                    "🏭 Создание экземпляра {:?} через фабрику (lifetime: {:?})",
                    type_id, factory_info.lifetime
                );
            }

            // Timeout для создания экземпляра
            let instance_start = Instant::now();

            // Вызываем фабрику - это может занять время
            let factory_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // Note: Здесь нам нужен доступ к container для передачи в factory
                // Но это создает циклическую зависимость. В реальной реализации
                // это решается через dependency injection контейнера в resolver
                (factory_info.factory)(
                    // Временно передаем dummy resolver
                    &crate::di::unified_container_impl::UnifiedContainer::minimal(),
                )
            }));

            let instance_duration = instance_start.elapsed();

            // Проверяем timeout создания экземпляра
            if instance_duration > self.config.instance_creation_timeout {
                warn!(
                    "⏱️ Создание {:?} заняло {:?} (превышен лимит {:?})",
                    type_id, instance_duration, self.config.instance_creation_timeout
                );
            }

            match factory_result {
                Ok(Ok(instance)) => {
                    // Увеличиваем счетчик разрешений для типа
                    factory_info.increment_resolution_count();

                    // Кэшируем если нужно
                    if self.config.enable_caching
                        && factory_info.lifetime != LifetimeStrategy::Transient
                    {
                        self.cache
                            .store(type_id, instance.clone(), factory_info.lifetime);
                    }

                    Ok(instance)
                }
                Ok(Err(e)) => Err(DIError::FactoryError {
                    type_id,
                    source: Box::new(e),
                }),
                Err(panic_info) => Err(DIError::FactoryPanic {
                    type_id,
                    panic_message: format!("{:?}", panic_info),
                }),
            }
        });

        // Убираем тип из стека
        context.pop();

        // Обрабатываем результат
        match result {
            Some(Ok(instance)) => {
                self.record_success(start_time);

                if self.config.verbose_logging {
                    debug!(
                        "✅ Создан новый экземпляр {:?} за {:?}",
                        type_id,
                        start_time.elapsed()
                    );
                }

                Ok(instance)
            }
            Some(Err(e)) => {
                self.record_failure();

                if self.config.verbose_logging {
                    error!("❌ Ошибка создания {:?}: {}", type_id, e);
                }

                Err(e)
            }
            None => {
                self.record_failure();
                let error = DIError::TypeNotRegistered { type_id };

                if self.config.verbose_logging {
                    error!("❌ Тип {:?} не зарегистрирован", type_id);
                }

                Err(error)
            }
        }
    }

    /// Попытаться получить из кэша
    fn try_get_from_cache(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        // Пробуем получить из кэша для разных lifetime стратегий
        self.cache
            .get::<dyn Any>(type_id, LifetimeStrategy::Singleton)
            .or_else(|| self.cache.get::<dyn Any>(type_id, LifetimeStrategy::Scoped))
    }

    /// Записать успешное разрешение
    fn record_success(&self, start_time: Instant) {
        let duration = start_time.elapsed();
        let mut metrics = self.metrics.write();

        metrics.successful_resolutions += 1;
        metrics.total_resolution_time += duration;

        if duration > metrics.max_resolution_time {
            metrics.max_resolution_time = duration;
        }
    }

    /// Записать неудачное разрешение
    fn record_failure(&self) {
        let mut metrics = self.metrics.write();
        metrics.failed_resolutions += 1;
    }

    /// Записать timeout
    fn record_timeout(&self) {
        let mut metrics = self.metrics.write();
        metrics.timeout_count += 1;
        metrics.failed_resolutions += 1;
    }

    /// Записать cache hit
    fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write();
        metrics.cache_hits += 1;
    }

    /// Записать cache miss
    fn record_cache_miss(&self) {
        let mut metrics = self.metrics.write();
        metrics.cache_misses += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::di::{container_cache::CacheConfig, core_traits::ServiceFactory};

    // Helper для создания mock factory
    fn create_mock_factory() -> ServiceFactory {
        Box::new(|_| Ok(Box::new("test_value") as Box<dyn Any + Send + Sync>))
    }

    #[test]
    fn test_resolver_creation() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::default();

        let resolver = ServiceResolverImpl::new(registry, cache, config);
        let metrics = resolver.get_metrics();

        assert_eq!(metrics.total_resolutions, 0);
        assert_eq!(metrics.successful_resolutions, 0);
    }

    #[test]
    fn test_type_not_registered() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::minimal();

        let resolver = ServiceResolverImpl::new(registry, cache, config);
        let type_id = TypeId::of::<String>();

        let result = resolver.resolve_type_erased(type_id);
        assert!(result.is_err());

        let metrics = resolver.get_metrics();
        assert_eq!(metrics.total_resolutions, 1);
        assert_eq!(metrics.failed_resolutions, 1);
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_can_resolve() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::minimal();

        let resolver = ServiceResolverImpl::new(registry.clone(), cache, config);
        let type_id = TypeId::of::<String>();

        // Тип не зарегистрирован
        assert!(!resolver.can_resolve(type_id));

        // Регистрируем тип
        registry
            .register_type_erased(
                type_id,
                "String".to_string(),
                create_mock_factory(),
                LifetimeStrategy::Singleton,
            )
            .expect("Operation failed - converted from unwrap()");

        // Теперь тип может быть разрешен
        assert!(resolver.can_resolve(type_id));
    }

    #[test]
    fn test_metrics_reset() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::minimal();

        let resolver = ServiceResolverImpl::new(registry, cache, config);
        let type_id = TypeId::of::<String>();

        // Делаем неудачную попытку разрешения
        let _ = resolver.resolve_type_erased(type_id);

        let metrics_before = resolver.get_metrics();
        assert_eq!(metrics_before.total_resolutions, 1);

        // Сбрасываем метрики
        resolver.reset_metrics();

        let metrics_after = resolver.get_metrics();
        assert_eq!(metrics_after.total_resolutions, 0);
    }

    #[test]
    fn test_detailed_report() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::minimal();

        let resolver = ServiceResolverImpl::new(registry, cache, config);

        let report = resolver.get_detailed_report();
        assert!(report.contains("Service Resolver Detailed Report"));
        assert!(report.contains("Total resolutions: 0"));
    }

    #[test]
    fn test_resolver_validation() {
        let registry = Arc::new(ServiceRegistryImpl::default());
        let cache = Arc::new(ContainerCache::new(CacheConfig::default()));
        let config = ResolverConfig::minimal();

        let resolver = ServiceResolverImpl::new(registry, cache, config);

        // Новый resolver должен быть валидным
        assert!(resolver.validate().is_ok());
    }
}
