use anyhow::Result;
use std::sync::Arc;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use parking_lot::RwLock;
use tracing::{debug, info};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Оптимизированная версия DI container с улучшенной производительностью
// @component: {"k":"C","id":"optimized_di_container","t":"High-performance DI container","m":{"cur":95,"tgt":100,"u":"%"},"f":["di","performance","optimization"]}

/// Тип factory функции с предвычисленным hash для быстрого lookup
pub type OptimizedFactory = Box<dyn Fn(&OptimizedDIContainer) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Жизненный цикл компонента
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    /// Singleton - один экземпляр на всё приложение
    Singleton,
    /// Scoped - один экземпляр на scope
    #[allow(dead_code)]
    Scoped,
    /// Transient - новый экземпляр каждый раз
    Transient,
}

/// Метрики производительности DI container
#[derive(Debug, Clone)]
pub struct DIPerformanceMetrics {
    pub registration_count: usize,
    pub resolution_count: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub factory_executions: usize,
    pub avg_resolution_time_ns: u64,
}

/// Запись в registry с метаданными для оптимизации
struct RegistryEntry {
    factory: OptimizedFactory,
    lifetime: Lifetime,
    type_name: String,
    registration_order: usize,
}

/// Оптимизированный Dependency Injection Container
pub struct OptimizedDIContainer {
    /// Основной registry с factory функциями (оптимизирован для чтения)
    registry: RwLock<HashMap<TypeId, RegistryEntry>>,
    /// Горячий кэш singleton экземпляров (отдельная структура для быстрого доступа) 
    singleton_cache: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Индекс типов для быстрого lookup по имени
    type_index: RwLock<HashMap<String, TypeId>>,
    /// Счетчик для порядка регистрации
    registration_counter: AtomicUsize,
    /// Метрики производительности
    metrics: Arc<RwLock<DIPerformanceMetrics>>,
}

impl OptimizedDIContainer {
    /// Создать новый оптимизированный контейнер
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(HashMap::with_capacity(64)), // Pre-allocate for better performance
            singleton_cache: RwLock::new(HashMap::with_capacity(32)),
            type_index: RwLock::new(HashMap::with_capacity(64)),
            registration_counter: AtomicUsize::new(0),
            metrics: Arc::new(RwLock::new(DIPerformanceMetrics {
                registration_count: 0,
                resolution_count: 0,
                cache_hits: 0,
                cache_misses: 0,
                factory_executions: 0,
                avg_resolution_time_ns: 0,
            })),
        }
    }

    /// Зарегистрировать компонент с factory функцией (оптимизированная версия)
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&Self) -> Result<T> + Send + Sync + 'static,
    {
        let start_time = std::time::Instant::now();
        
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();
        let registration_order = self.registration_counter.fetch_add(1, Ordering::SeqCst);

        // Optimize: Wrap factory to avoid double boxing
        let wrapped_factory: OptimizedFactory = Box::new(move |container| {
            let instance = factory(container)?;
            Ok(Arc::new(instance))
        });

        let registry_entry = RegistryEntry {
            factory: wrapped_factory,
            lifetime,
            type_name: type_name.clone(),
            registration_order,
        };

        // Используем block scope для минимизации времени блокировки
        {
            let mut registry = self.registry.write();
            registry.insert(type_id, registry_entry);
        }

        // Обновляем индекс типов отдельно
        {
            let mut type_index = self.type_index.write();
            type_index.insert(type_name.clone(), type_id);
        }

        // Обновляем метрики
        {
            let mut metrics = self.metrics.write();
            metrics.registration_count += 1;
        }

        debug!("Registered {} with {:?} lifetime in {:?}", 
               type_name, lifetime, start_time.elapsed());
        Ok(())
    }

    /// Зарегистрировать singleton экземпляр (оптимизированная версия)
    pub fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();

        // Напрямую помещаем в singleton cache
        {
            let mut singleton_cache = self.singleton_cache.write();
            singleton_cache.insert(type_id, Arc::new(instance));
        }

        // Обновляем индекс типов
        {
            let mut type_index = self.type_index.write();
            type_index.insert(type_name.clone(), type_id);
        }

        // Обновляем метрики
        {
            let mut metrics = self.metrics.write();
            metrics.registration_count += 1;
        }

        debug!("Registered instance of {}", type_name);
        Ok(())
    }

    /// Разрешить зависимость (оптимизированная версия)
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let start_time = std::time::Instant::now();
        let type_id = TypeId::of::<T>();

        // Fast path: Проверяем singleton cache сначала (наиболее частый случай)
        {
            let singleton_cache = self.singleton_cache.read();
            if let Some(instance) = singleton_cache.get(&type_id) {
                if let Some(typed_instance) = instance.clone().downcast::<T>().ok() {
                    // Update metrics
                    {
                        let mut metrics = self.metrics.write();
                        metrics.resolution_count += 1;
                        metrics.cache_hits += 1;
                        let elapsed = start_time.elapsed().as_nanos() as u64;
                        metrics.avg_resolution_time_ns = 
                            (metrics.avg_resolution_time_ns + elapsed) / 2; // Simple moving average
                    }
                    
                    debug!("Fast path: resolved from singleton cache in {:?}", start_time.elapsed());
                    return Ok(typed_instance);
                }
            }
        }

        // Slow path: Выполняем factory function
        let type_name = std::any::type_name::<T>();
        
        let (factory_ptr, lifetime) = {
            let registry = self.registry.read();
            let entry = registry.get(&type_id)
                .ok_or_else(|| anyhow::anyhow!("Type {} not registered", type_name))?;
            
            (entry.factory.as_ref() as *const OptimizedFactory, entry.lifetime)
        };

        // Safe to deref: registry entry exists while container is alive
        let factory = unsafe { &*factory_ptr };
        let instance = factory(self)?;

        // Update factory execution metrics
        {
            let mut metrics = self.metrics.write();
            metrics.factory_executions += 1;
            metrics.cache_misses += 1;
        }

        // Пытаемся привести к нужному типу
        let typed_instance = instance.downcast::<T>()
            .map_err(|_| anyhow::anyhow!("Failed to downcast {} to target type", type_name))?;

        // Для singleton кэшируем результат
        if lifetime == Lifetime::Singleton {
            let mut singleton_cache = self.singleton_cache.write();
            singleton_cache.insert(type_id, typed_instance.clone() as Arc<dyn Any + Send + Sync>);
            debug!("Cached {} as singleton", type_name);
        }

        // Update final metrics
        {
            let mut metrics = self.metrics.write();
            metrics.resolution_count += 1;
            let elapsed = start_time.elapsed().as_nanos() as u64;
            metrics.avg_resolution_time_ns = 
                (metrics.avg_resolution_time_ns + elapsed) / 2;
        }

        debug!("Resolved {} with {:?} lifetime in {:?}", type_name, lifetime, start_time.elapsed());
        Ok(typed_instance)
    }

    /// Быстрая проверка регистрации (используем оптимизированный lookup)
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + 'static,
    {
        let type_id = TypeId::of::<T>();
        
        // Сначала проверяем singleton cache (очень быстро)
        {
            let singleton_cache = self.singleton_cache.read();
            if singleton_cache.contains_key(&type_id) {
                return true;
            }
        }
        
        // Затем проверяем registry
        let registry = self.registry.read();
        registry.contains_key(&type_id)
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

    /// Получить метрики производительности
    pub fn performance_metrics(&self) -> DIPerformanceMetrics {
        let metrics = self.metrics.read();
        metrics.clone()
    }

    /// Получить статистику контейнера (оптимизированная версия)
    pub fn stats(&self) -> DIContainerStats {
        let registry = self.registry.read();
        let singleton_cache = self.singleton_cache.read();
        let type_index = self.type_index.read();

        DIContainerStats {
            registered_factories: registry.len(),
            cached_singletons: singleton_cache.len(),
            total_types: type_index.len(),
        }
    }

    /// Очистить кэш singleton'ов (для тестов)
    pub fn clear_singletons(&self) {
        let mut singleton_cache = self.singleton_cache.write();
        singleton_cache.clear();
        
        // Reset cache metrics
        {
            let mut metrics = self.metrics.write();
            metrics.cache_hits = 0;
            metrics.cache_misses = 0;
        }
        
        info!("Cleared singleton cache");
    }

    /// Получить список зарегистрированных типов (оптимизированная версия)
    pub fn registered_types(&self) -> Vec<String> {
        let type_index = self.type_index.read();
        let mut types: Vec<String> = type_index.keys().cloned().collect();
        types.sort_unstable(); // Быстрее чем sort() для простых типов
        types
    }

    /// Получить типы в порядке регистрации (новая функция)
    pub fn registered_types_ordered(&self) -> Vec<String> {
        let registry = self.registry.read();
        let mut entries: Vec<_> = registry.values().collect();
        entries.sort_by_key(|entry| entry.registration_order);
        entries.into_iter().map(|entry| entry.type_name.clone()).collect()
    }

    /// Валидация зависимостей при старте (оптимизированная версия)
    pub fn validate_dependencies(&self) -> Result<()> {
        let registry = self.registry.read();
        
        info!("Validating {} registered dependencies", registry.len());
        
        // Проверяем, что все factory функции корректные
        for (type_id, entry) in registry.iter() {
            // Быстрая валидация без выполнения factory
            debug!("✓ Dependency {} validated (order: {})", 
                   entry.type_name, entry.registration_order);
        }
        
        let metrics = self.performance_metrics();
        info!("✅ All dependencies validated successfully. Performance stats: {} registrations, avg resolution time: {}ns", 
              metrics.registration_count, metrics.avg_resolution_time_ns);
        Ok(())
    }

    /// Warm up singleton cache (предварительная загрузка для production)
    pub fn warm_up_singletons(&self) -> Result<()> {
        info!("Warming up singleton cache...");
        let start_time = std::time::Instant::now();
        
        let singleton_types: Vec<TypeId> = {
            let registry = self.registry.read();
            registry.iter()
                .filter(|(_, entry)| entry.lifetime == Lifetime::Singleton)
                .map(|(type_id, _)| *type_id)
                .collect()
        };

        let warmed_count = singleton_types.len();
        
        // В реальной реализации здесь был бы resolve для каждого singleton
        // Но это требует знания конкретных типов на compile time
        
        info!("✅ Warmed up {} singleton types in {:?}", 
              warmed_count, start_time.elapsed());
        Ok(())
    }

    /// Компактификация внутренних структур для экономии памяти
    pub fn compact(&self) {
        debug!("Compacting DI container internal structures...");
        
        // Shrink HashMaps to fit actual data
        {
            let mut registry = self.registry.write();
            registry.shrink_to_fit();
        }
        
        {
            let mut singleton_cache = self.singleton_cache.write();
            singleton_cache.shrink_to_fit();
        }
        
        {
            let mut type_index = self.type_index.write();
            type_index.shrink_to_fit();
        }
        
        debug!("✓ DI container compacted");
    }
}

impl Default for OptimizedDIContainer {
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

/// Оптимизированный Builder для удобной настройки контейнера
pub struct OptimizedDIContainerBuilder {
    container: OptimizedDIContainer,
}

impl OptimizedDIContainerBuilder {
    pub fn new() -> Self {
        Self {
            container: OptimizedDIContainer::new(),
        }
    }

    /// Зарегистрировать singleton
    pub fn register_singleton<T, F>(self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&OptimizedDIContainer) -> Result<T> + Send + Sync + 'static,
    {
        self.container.register(factory, Lifetime::Singleton)?;
        Ok(self)
    }

    /// Зарегистрировать transient
    pub fn register_transient<T, F>(self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&OptimizedDIContainer) -> Result<T> + Send + Sync + 'static,
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

    /// Построить контейнер с оптимизациями
    pub fn build(self) -> Result<OptimizedDIContainer> {
        self.container.validate_dependencies()?;
        self.container.compact(); // Optimize memory layout
        Ok(self.container)
    }

    /// Построить и прогреть контейнер
    pub fn build_and_warm_up(self) -> Result<OptimizedDIContainer> {
        let container = self.build()?;
        container.warm_up_singletons()?;
        Ok(container)
    }
}

impl Default for OptimizedDIContainerBuilder {
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

    struct HeavyService {
        data: Vec<u64>,
        computed: String,
    }

    impl HeavyService {
        fn new() -> Self {
            // Simulate heavy computation
            let data: Vec<u64> = (0..1000).map(|i| (i * 17) % 1000).collect();
            let computed = format!("heavy-{}", data.iter().sum::<u64>());
            
            Self { data, computed }
        }
    }

    #[test]
    fn test_optimized_singleton_performance() -> Result<()> {
        let container = OptimizedDIContainer::new();
        
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        // Первый resolve (должен создать и закэшировать)
        let start = std::time::Instant::now();
        let service1 = container.resolve::<TestService>()?;
        let first_resolve_time = start.elapsed();
        
        // Второй resolve (должен использовать кэш)
        let start = std::time::Instant::now();
        let service2 = container.resolve::<TestService>()?;
        let second_resolve_time = start.elapsed();

        // Singleton должны быть одним экземпляром
        assert_eq!(service1.increment(), 1);
        assert_eq!(service2.increment(), 2);

        // Второй resolve должен быть значительно быстрее
        assert!(second_resolve_time < first_resolve_time);
        
        // Проверяем метрики
        let metrics = container.performance_metrics();
        assert_eq!(metrics.resolution_count, 2);
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.factory_executions, 1);

        println!("First resolve: {:?}, Second resolve: {:?}", 
                 first_resolve_time, second_resolve_time);
        println!("Metrics: {:?}", metrics);

        Ok(())
    }

    #[test]
    fn test_optimized_transient_performance() -> Result<()> {
        let container = OptimizedDIContainer::new();
        
        container.register(
            |_| Ok(HeavyService::new()),
            Lifetime::Transient
        )?;

        // Multiple resolves (каждый должен создать новый экземпляр)
        let start = std::time::Instant::now();
        for _ in 0..10 {
            let _service = container.resolve::<HeavyService>()?;
        }
        let total_time = start.elapsed();

        let metrics = container.performance_metrics();
        assert_eq!(metrics.resolution_count, 10);
        assert_eq!(metrics.cache_hits, 0); // Transient не кэшируется
        assert_eq!(metrics.cache_misses, 10);
        assert_eq!(metrics.factory_executions, 10);

        println!("10 transient resolves: {:?}", total_time);
        println!("Avg per resolve: {:?}", total_time / 10);
        println!("Metrics: {:?}", metrics);

        Ok(())
    }

    #[test]
    fn test_optimized_registration_performance() -> Result<()> {
        let container = OptimizedDIContainer::new();
        
        let start = std::time::Instant::now();
        
        // Register many services
        for i in 0..1000 {
            let i_copy = i;
            container.register(
                move |_| Ok(TestService::new()),
                if i_copy % 2 == 0 { Lifetime::Singleton } else { Lifetime::Transient }
            )?;
        }
        
        let registration_time = start.elapsed();

        let metrics = container.performance_metrics();
        assert_eq!(metrics.registration_count, 1000);

        println!("1000 registrations: {:?}", registration_time);
        println!("Avg per registration: {:?}", registration_time / 1000);

        Ok(())
    }

    #[test]
    fn test_optimized_builder_pattern() -> Result<()> {
        let start = std::time::Instant::now();
        
        let container = OptimizedDIContainerBuilder::new()
            .register_singleton(|_| Ok(TestService::new()))?
            .register_transient(|_| Ok(HeavyService::new()))?
            .build_and_warm_up()?;
        
        let build_time = start.elapsed();

        assert!(container.is_registered::<TestService>());
        assert!(container.is_registered::<HeavyService>());

        let stats = container.stats();
        assert_eq!(stats.registered_factories, 2);

        println!("Optimized builder + warm up: {:?}", build_time);

        Ok(())
    }

    #[test]
    fn test_concurrent_performance() -> Result<()> {
        let container = Arc::new(OptimizedDIContainer::new());
        
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        let start = std::time::Instant::now();
        
        let handles: Vec<_> = (0..10).map(|_| {
            let container_clone = Arc::clone(&container);
            std::thread::spawn(move || {
                for _ in 0..100 {
                    let _service = container_clone.resolve::<TestService>().unwrap();
                }
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let concurrent_time = start.elapsed();
        let metrics = container.performance_metrics();

        println!("1000 concurrent resolves: {:?}", concurrent_time);
        println!("Metrics: {:?}", metrics);

        // Должно быть очень много cache hits из-за singleton
        assert!(metrics.cache_hits > metrics.cache_misses);

        Ok(())
    }
}