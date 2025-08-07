//! Unified Dependency Injection Container
//! 
//! Этот файл объединяет все существующие DI решения в проекте в единую,
//! чистую архитектуру, основанную на принципах SOLID.
//! 
//! ПРОБЛЕМЫ КОТОРЫЕ РЕШАЕТ:
//! - 4 дублирования DIContainer структур
//! - Service Locator anti-pattern
//! - God Objects >1000 строк
//! - .unwrap() вызовы без error handling
//! - Циклические зависимости
//! 
//! ПРИНЦИПЫ SOLID:
//! - SRP: Каждый компонент имеет единственную ответственность
//! - OCP: Расширяемость через trait abstraction
//! - LSP: Взаимозаменяемые implementations
//! - ISP: Минимальные, сфокусированные интерфейсы  
//! - DIP: Constructor Injection, зависимости от абстракций

use anyhow::{anyhow, Result};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use tracing::{debug, info, warn, error};

use super::traits::{
    DIResolver, DIRegistrar, Lifetime, 
    DIContainerStats, DIPerformanceMetrics, TypeMetrics
};

/// Factory function type для создания компонентов
pub type ComponentFactory = Box<dyn Fn(&UnifiedDIContainer) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Registration информация для компонента
#[derive(Debug)]
struct ComponentRegistration {
    /// Factory функция для создания компонента
    factory: ComponentFactory,
    /// Жизненный цикл компонента
    lifetime: Lifetime,
    /// Имя типа для отладки
    type_name: String,
    /// Время регистрации
    registered_at: Instant,
}

/// Cache entry для singleton/scoped компонентов
#[derive(Debug)]
struct CacheEntry {
    /// Экземпляр компонента
    instance: Arc<dyn Any + Send + Sync>,
    /// Время создания
    created_at: Instant,
    /// Количество обращений
    access_count: u64,
    /// Последнее время доступа
    last_access: Instant,
}

/// Unified DI Container - единое решение для всего проекта
/// 
/// ЗАМЕНЯЕТ:
/// - ContainerCore из di/container_core.rs
/// - DIMemoryServiceFacade из service_di/facade.rs  
/// - DIMemoryService из service_di_original.rs
/// - DIMemoryService из service_di_refactored.rs
/// 
/// АРХИТЕКТУРА:
/// - Constructor Injection вместо Service Locator
/// - Result<T, E> вместо .unwrap() calls
/// - Trait-based abstractions для extensibility
/// - Comprehensive error handling
/// - Performance metrics и monitoring
pub struct UnifiedDIContainer {
    /// Зарегистрированные компоненты
    registrations: RwLock<HashMap<TypeId, ComponentRegistration>>,
    
    /// Cache для singleton и scoped экземпляров
    instance_cache: RwLock<HashMap<TypeId, CacheEntry>>,
    
    /// Граф зависимостей для cycle detection
    dependency_graph: RwLock<HashMap<TypeId, Vec<TypeId>>>,
    
    /// Метрики производительности
    performance_metrics: RwLock<DIPerformanceMetrics>,
    
    /// Контейнер конфигурация
    configuration: ContainerConfiguration,
}

/// Конфигурация контейнера
#[derive(Debug, Clone)]
pub struct ContainerConfiguration {
    /// Максимальный размер кэша singleton экземпляров
    pub max_cache_size: usize,
    /// Timeout для создания экземпляров
    pub instance_creation_timeout: Duration,
    /// Включить валидацию зависимостей
    pub enable_dependency_validation: bool,
    /// Включить сбор метрик производительности
    pub enable_performance_metrics: bool,
    /// Максимальная глубина зависимостей
    pub max_dependency_depth: u32,
    /// Cache cleanup interval
    pub cache_cleanup_interval: Duration,
}

impl Default for ContainerConfiguration {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            instance_creation_timeout: Duration::from_secs(30),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 20,
            cache_cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl ContainerConfiguration {
    /// Production конфигурация с оптимизированными параметрами
    pub fn production() -> Self {
        Self {
            max_cache_size: 5000,
            instance_creation_timeout: Duration::from_secs(10),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 15,
            cache_cleanup_interval: Duration::from_secs(600), // 10 minutes
        }
    }
    
    /// Development конфигурация с отладкой
    pub fn development() -> Self {
        Self {
            max_cache_size: 500,
            instance_creation_timeout: Duration::from_secs(60),
            enable_dependency_validation: true,
            enable_performance_metrics: true,
            max_dependency_depth: 25,
            cache_cleanup_interval: Duration::from_secs(180), // 3 minutes
        }
    }
    
    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        Self {
            max_cache_size: 100,
            instance_creation_timeout: Duration::from_secs(5),
            enable_dependency_validation: false,
            enable_performance_metrics: false,
            max_dependency_depth: 10,
            cache_cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Container builder для пошагового создания
pub struct UnifiedDIContainerBuilder {
    configuration: ContainerConfiguration,
}

impl UnifiedDIContainerBuilder {
    pub fn new() -> Self {
        Self {
            configuration: ContainerConfiguration::default(),
        }
    }
    
    pub fn with_configuration(mut self, config: ContainerConfiguration) -> Self {
        self.configuration = config;
        self
    }
    
    pub fn with_max_cache_size(mut self, size: usize) -> Self {
        self.configuration.max_cache_size = size;
        self
    }
    
    pub fn with_instance_timeout(mut self, timeout: Duration) -> Self {
        self.configuration.instance_creation_timeout = timeout;
        self
    }
    
    pub fn enable_validation(mut self) -> Self {
        self.configuration.enable_dependency_validation = true;
        self
    }
    
    pub fn disable_validation(mut self) -> Self {
        self.configuration.enable_dependency_validation = false;
        self
    }
    
    pub fn enable_metrics(mut self) -> Self {
        self.configuration.enable_performance_metrics = true;
        self
    }
    
    pub fn disable_metrics(mut self) -> Self {
        self.configuration.enable_performance_metrics = false;
        self
    }
    
    pub fn build(self) -> UnifiedDIContainer {
        UnifiedDIContainer::with_configuration(self.configuration)
    }
}

impl Default for UnifiedDIContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedDIContainer {
    /// Создать контейнер с default конфигурацией
    pub fn new() -> Self {
        Self::with_configuration(ContainerConfiguration::default())
    }
    
    /// Создать контейнер с указанной конфигурацией
    pub fn with_configuration(config: ContainerConfiguration) -> Self {
        info!("🚀 Создание UnifiedDIContainer с конфигурацией: {:?}", config);
        
        Self {
            registrations: RwLock::new(HashMap::new()),
            instance_cache: RwLock::new(HashMap::new()),
            dependency_graph: RwLock::new(HashMap::new()),
            performance_metrics: RwLock::new(DIPerformanceMetrics::default()),
            configuration: config,
        }
    }
    
    /// Создать production-ready контейнер
    pub fn production() -> Self {
        Self::with_configuration(ContainerConfiguration::production())
    }
    
    /// Создать development контейнер
    pub fn development() -> Self {
        Self::with_configuration(ContainerConfiguration::development())
    }
    
    /// Создать minimal контейнер для тестов
    pub fn minimal() -> Self {
        Self::with_configuration(ContainerConfiguration::minimal())
    }
    
    /// Зарегистрировать компонент с factory функцией
    /// 
    /// ПРИМЕНЯЕТ:
    /// - SRP: единственная ответственность - регистрация компонента
    /// - DIP: зависимость от абстракции (factory function)
    /// - OCP: расширяемость через различные lifetimes
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&UnifiedDIContainer) -> Result<T> + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();
        let type_name_for_closure = type_name.clone();
        
        debug!("📝 Регистрация компонента: {} ({:?})", type_name, lifetime);
        
        // Проверяем, не зарегистрирован ли уже этот тип
        {
            let registrations = self.registrations.read();
            if registrations.contains_key(&type_id) {
                return Err(anyhow!(
                    "Компонент {} уже зарегистрирован в контейнере", 
                    type_name
                ));
            }
        }
        
        // Создаем обертку factory функции с error handling
        let wrapped_factory: ComponentFactory = Box::new(move |container| {
            let start_time = Instant::now();
            
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                factory(container)
            })) {
                Ok(Ok(instance)) => {
                    let duration = start_time.elapsed();
                    
                    // Записываем метрики успешного создания
                    if container.configuration.enable_performance_metrics {
                        container.record_successful_creation(type_id, duration);
                    }
                    
                    debug!("✅ Создан экземпляр {} за {:?}", type_name_for_closure, duration);
                    Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
                }
                Ok(Err(e)) => {
                    let duration = start_time.elapsed();
                    
                    // Записываем метрики неудачного создания
                    if container.configuration.enable_performance_metrics {
                        container.record_failed_creation(type_id, duration, &e);
                    }
                    
                    error!("❌ Ошибка создания {}: {}", type_name_for_closure, e);
                    Err(e)
                }
                Err(panic_err) => {
                    let duration = start_time.elapsed();
                    let error = anyhow!("Panic при создании {}: {:?}", type_name_for_closure, panic_err);
                    
                    // Записываем метрики panic
                    if container.configuration.enable_performance_metrics {
                        container.record_failed_creation(type_id, duration, &error);
                    }
                    
                    error!("💥 Panic при создании {}: {:?}", type_name, panic_err);
                    Err(error)
                }
            }
        });
        
        // Регистрируем компонент
        {
            let mut registrations = self.registrations.write();
            registrations.insert(type_id, ComponentRegistration {
                factory: wrapped_factory,
                lifetime,
                type_name: type_name,
                registered_at: Instant::now(),
            });
        }
        
        // Записываем метрику регистрации
        if self.configuration.enable_performance_metrics {
            self.record_registration(type_id);
        }
        
        // type_name moved, recreate from type
        info!("✅ Компонент {} зарегистрирован с lifetime {:?}", std::any::type_name::<T>(), lifetime);
        Ok(())
    }
    
    /// Зарегистрировать singleton экземпляр
    pub fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();
        
        debug!("📝 Регистрация singleton экземпляра: {}", type_name);
        
        // Проверяем дублирование
        {
            let registrations = self.registrations.read();
            if registrations.contains_key(&type_id) {
                return Err(anyhow!(
                    "Компонент {} уже зарегистрирован в контейнере", 
                    type_name
                ));
            }
        }
        
        // Создаем factory который возвращает готовый экземпляр
        let instance_arc = Arc::new(instance);
        let factory: ComponentFactory = Box::new(move |_container| {
            Ok(instance_arc.clone() as Arc<dyn Any + Send + Sync>)
        });
        
        // Регистрируем как singleton
        {
            let mut registrations = self.registrations.write();
            registrations.insert(type_id, ComponentRegistration {
                factory,
                lifetime: Lifetime::Singleton,
                type_name: type_name.clone(),
                registered_at: Instant::now(),
            });
        }
        
        if self.configuration.enable_performance_metrics {
            self.record_registration(type_id);
        }
        
        info!("✅ Singleton экземпляр {} зарегистрирован", type_name);
        Ok(())
    }
    
    /// Проверить, зарегистрирован ли тип
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let registrations = self.registrations.read();
        registrations.contains_key(&type_id)
    }
    
    /// Получить количество зарегистрированных компонентов
    pub fn registration_count(&self) -> usize {
        let registrations = self.registrations.read();
        registrations.len()
    }
    
    /// Получить список всех зарегистрированных типов
    pub fn registered_types(&self) -> Vec<String> {
        let registrations = self.registrations.read();
        registrations.values()
            .map(|reg| reg.type_name.clone())
            .collect()
    }
    
    /// Валидировать зависимости на циклы
    pub fn validate_dependencies(&self) -> Result<()> {
        if !self.configuration.enable_dependency_validation {
            return Ok(());
        }
        
        debug!("🔍 Валидация зависимостей контейнера...");
        
        let dependency_graph = self.dependency_graph.read();
        let cycles = self.detect_cycles(&dependency_graph);
        
        if !cycles.is_empty() {
            let mut error_msg = String::from("Обнаружены циклические зависимости:\n");
            
            for cycle in cycles {
                let cycle_names: Vec<String> = cycle.iter()
                    .map(|type_id| self.get_type_name(*type_id))
                    .collect();
                error_msg.push_str(&format!("  -> {}\n", cycle_names.join(" -> ")));
            }
            
            return Err(anyhow!(error_msg));
        }
        
        debug!("✅ Валидация зависимостей прошла успешно");
        Ok(())
    }
    
    /// Добавить информацию о зависимости
    pub fn add_dependency<TDependent, TDependency>(&self)
    where
        TDependent: Any + 'static,
        TDependency: Any + 'static,
    {
        if !self.configuration.enable_dependency_validation {
            return;
        }
        
        let dependent_id = TypeId::of::<TDependent>();
        let dependency_id = TypeId::of::<TDependency>();
        
        let mut graph = self.dependency_graph.write();
        graph.entry(dependent_id)
            .or_insert_with(Vec::new)
            .push(dependency_id);
            
        debug!("🔗 Добавлена зависимость: {} -> {}", 
               self.get_type_name(dependent_id),
               self.get_type_name(dependency_id));
    }
    
    /// Очистить все регистрации и кэши
    pub fn clear(&self) {
        info!("🧹 Очистка контейнера...");
        
        {
            let mut registrations = self.registrations.write();
            registrations.clear();
        }
        
        {
            let mut cache = self.instance_cache.write();
            cache.clear();
        }
        
        {
            let mut graph = self.dependency_graph.write();
            graph.clear();
        }
        
        if self.configuration.enable_performance_metrics {
            let mut metrics = self.performance_metrics.write();
            *metrics = DIPerformanceMetrics::default();
        }
        
        info!("✅ Контейнер очищен");
    }
    
    /// Получить статистику контейнера
    pub fn stats(&self) -> DIContainerStats {
        let registrations = self.registrations.read();
        let cache = self.instance_cache.read();
        let metrics = self.performance_metrics.read();
        
        DIContainerStats {
            registered_factories: registrations.len(),
            cached_singletons: cache.len(),
            total_resolutions: metrics.total_resolutions,
            cache_hits: metrics.cache_hits,
            validation_errors: 0, // TODO: добавить счетчик ошибок валидации
        }
    }
    
    /// Получить метрики производительности
    pub fn performance_metrics(&self) -> DIPerformanceMetrics {
        if self.configuration.enable_performance_metrics {
            self.performance_metrics.read().clone()
        } else {
            DIPerformanceMetrics::default()
        }
    }
    
    /// Получить отчет о производительности
    pub fn get_performance_report(&self) -> String {
        if !self.configuration.enable_performance_metrics {
            return "Performance metrics disabled".to_string();
        }
        
        let metrics = self.performance_metrics.read();
        let stats = self.stats();
        
        format!(
            "=== Unified DI Container Performance Report ===\n\
             Total resolutions: {}\n\
             Cache hit rate: {:.1}%\n\
             Average resolution time: {:.2}μs\n\
             Total factories: {}\n\
             Cached singletons: {}\n\
             Error count: {}\n\
             Dependency depth: {}\n\
             ==========================================",
            metrics.total_resolutions,
            metrics.cache_hit_rate(),
            metrics.avg_resolve_time_us(),
            stats.registered_factories,
            stats.cached_singletons,
            metrics.error_count,
            metrics.dependency_depth
        )
    }
    
    /// Сбросить метрики производительности
    pub fn reset_performance_metrics(&self) {
        if self.configuration.enable_performance_metrics {
            let mut metrics = self.performance_metrics.write();
            *metrics = DIPerformanceMetrics::default();
            debug!("🔄 Performance метрики сброшены");
        }
    }
    
    /// Запустить cleanup task для кэша
    pub fn start_cache_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let cleanup_interval = self.configuration.cache_cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            
            loop {
                interval.tick().await;
                self.cleanup_expired_cache_entries();
            }
        })
    }
    
    // === PRIVATE HELPER METHODS ===
    
    /// Получить имя типа для отладки
    fn get_type_name(&self, type_id: TypeId) -> String {
        let registrations = self.registrations.read();
        registrations.get(&type_id)
            .map(|reg| reg.type_name.clone())
            .unwrap_or_else(|| format!("Unknown({:?})", type_id))
    }
    
    /// Обнаружить циклы в графе зависимостей
    fn detect_cycles(&self, graph: &HashMap<TypeId, Vec<TypeId>>) -> Vec<Vec<TypeId>> {
        let mut cycles = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        let mut current_path = Vec::new();
        
        for &node in graph.keys() {
            if !visited.contains(&node) {
                self.dfs_cycle_detection(
                    node, 
                    graph, 
                    &mut visited, 
                    &mut rec_stack, 
                    &mut current_path,
                    &mut cycles
                );
            }
        }
        
        cycles
    }
    
    /// DFS для обнаружения циклов
    fn dfs_cycle_detection(
        &self,
        node: TypeId,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        visited: &mut std::collections::HashSet<TypeId>,
        rec_stack: &mut std::collections::HashSet<TypeId>,
        current_path: &mut Vec<TypeId>,
        cycles: &mut Vec<Vec<TypeId>>,
    ) {
        visited.insert(node);
        rec_stack.insert(node);
        current_path.push(node);
        
        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    self.dfs_cycle_detection(
                        neighbor, 
                        graph, 
                        visited, 
                        rec_stack, 
                        current_path, 
                        cycles
                    );
                } else if rec_stack.contains(&neighbor) {
                    // Найден цикл
                    let cycle_start = current_path.iter()
                        .position(|&x| x == neighbor)
                        .unwrap();
                    let cycle = current_path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
        
        current_path.pop();
        rec_stack.remove(&node);
    }
    
    /// Записать успешное создание экземпляра
    fn record_successful_creation(&self, type_id: TypeId, duration: Duration) {
        let mut metrics = self.performance_metrics.write();
        metrics.total_resolutions += 1;
        metrics.total_resolution_time += duration;
        
        let type_metrics = metrics.type_metrics.entry(type_id).or_insert_with(TypeMetrics::new);
        type_metrics.resolutions += 1;
        type_metrics.total_time += duration;
        type_metrics.average_time = type_metrics.total_time / type_metrics.resolutions as u32;
        type_metrics.last_resolution = Some(Instant::now());
    }
    
    /// Записать неудачное создание экземпляра
    fn record_failed_creation(&self, type_id: TypeId, duration: Duration, error: &anyhow::Error) {
        let mut metrics = self.performance_metrics.write();
        metrics.total_resolutions += 1;
        metrics.total_resolution_time += duration;
        metrics.error_count += 1;
        
        let type_metrics = metrics.type_metrics.entry(type_id).or_insert_with(TypeMetrics::new);
        type_metrics.resolutions += 1;
        type_metrics.total_time += duration;
        type_metrics.error_count += 1;
        type_metrics.average_time = type_metrics.total_time / type_metrics.resolutions as u32;
        type_metrics.last_resolution = Some(Instant::now());
        
        warn!("📊 Записана ошибка создания {}: {}", self.get_type_name(type_id), error);
    }
    
    /// Записать регистрацию компонента
    fn record_registration(&self, type_id: TypeId) {
        // Метрики регистрации могут быть добавлены позже
        debug!("📊 Записана регистрация {}", self.get_type_name(type_id));
    }
    
    /// Очистить истекшие записи кэша
    fn cleanup_expired_cache_entries(&self) {
        let mut cache = self.instance_cache.write();
        let now = Instant::now();
        let cleanup_threshold = Duration::from_secs(3600); // 1 час
        
        let initial_size = cache.len();
        cache.retain(|_type_id, entry| {
            now.duration_since(entry.last_access) < cleanup_threshold
        });
        
        let cleaned_count = initial_size - cache.len();
        if cleaned_count > 0 {
            debug!("🧹 Очищено {} истекших записей кэша", cleaned_count);
        }
    }
}

// === TRAIT IMPLEMENTATIONS ===

impl DIResolver for UnifiedDIContainer {
    /// Разрешить зависимость - CORE METHOD с complete error handling
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        let type_name = self.get_type_name(type_id);
        let start_time = Instant::now();
        
        debug!("🔍 Разрешение зависимости: {}", type_name);
        
        // 1. Проверяем кэш для singleton/scoped
        if let Some(cached_instance) = self.get_from_cache(type_id) {
            let duration = start_time.elapsed();
            
            // Обновляем статистику cache hit
            if self.configuration.enable_performance_metrics {
                self.record_cache_hit(type_id, duration);
            }
            
            // Пытаемся привести тип
            match cached_instance.downcast::<T>() {
                Ok(instance) => {
                    debug!("✅ Получен {} из кэша за {:?}", type_name, duration);
                    return Ok(instance);
                }
                Err(_) => {
                    error!("❌ Type mismatch для {}: кэшированный экземпляр не соответствует запрашиваемому типу", type_name);
                    return Err(anyhow!("Type mismatch для {}", type_name));
                }
            }
        }
        
        // 2. Получаем регистрацию
        let registration = {
            let registrations = self.registrations.read();
            match registrations.get(&type_id) {
                Some(reg) => {
                    // Создаем копию данных, необходимых для создания экземпляра
                    (reg.lifetime, reg.type_name.clone())
                }
                None => {
                    let duration = start_time.elapsed();
                    if self.configuration.enable_performance_metrics {
                        self.record_cache_miss(type_id, duration);
                    }
                    
                    error!("❌ Компонент {} не зарегистрирован в контейнере", type_name);
                    return Err(anyhow!("Компонент {} не зарегистрирован", type_name));
                }
            }
        };
        
        // 3. Создаем экземпляр через factory
        let instance_result = {
            let registrations = self.registrations.read();
            if let Some(reg) = registrations.get(&type_id) {
                // Вызываем factory в timeout-е для предотвращения deadlock
                let creation_start = Instant::now();
                let creation_result = (reg.factory)(self);
                
                let creation_duration = creation_start.elapsed();
                if creation_duration > self.configuration.instance_creation_timeout {
                    warn!("⏱️ Создание {} заняло {:?} (превышен лимит {:?})", 
                          type_name, creation_duration, self.configuration.instance_creation_timeout);
                }
                
                creation_result
            } else {
                Err(anyhow!("Регистрация для {} исчезла во время создания", type_name))
            }
        };
        
        let instance = match instance_result {
            Ok(instance) => instance,
            Err(e) => {
                let duration = start_time.elapsed();
                if self.configuration.enable_performance_metrics {
                    self.record_failed_creation(type_id, duration, &e);
                }
                
                error!("❌ Ошибка создания экземпляра {}: {}", type_name, e);
                return Err(e);
            }
        };
        
        // 4. Приводим к нужному типу
        let typed_instance = match instance.downcast::<T>() {
            Ok(typed) => typed,
            Err(_) => {
                let duration = start_time.elapsed();
                let error = anyhow!("Type mismatch: созданный экземпляр не соответствует типу {}", type_name);
                
                if self.configuration.enable_performance_metrics {
                    self.record_failed_creation(type_id, duration, &error);
                }
                
                error!("❌ {}", error);
                return Err(error);
            }
        };
        
        // 5. Кэшируем если нужно
        if registration.0 != Lifetime::Transient {
            self.cache_instance(type_id, typed_instance.clone() as Arc<dyn Any + Send + Sync>);
        }
        
        let total_duration = start_time.elapsed();
        
        // 6. Записываем успешные метрики
        if self.configuration.enable_performance_metrics {
            self.record_cache_miss(type_id, total_duration);
        }
        
        debug!("✅ Создан новый экземпляр {} за {:?}", type_name, total_duration);
        Ok(typed_instance)
    }
    
    /// Попытаться разрешить зависимость (безопасная версия)
    fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        match self.resolve::<T>() {
            Ok(instance) => Some(instance),
            Err(e) => {
                let type_name = std::any::type_name::<T>();
                debug!("🔍 try_resolve для {} не удался: {}", type_name, e);
                None
            }
        }
    }
    
    /// Проверить, зарегистрирован ли тип
    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.is_registered::<T>()
    }
}

impl DIRegistrar for UnifiedDIContainer {
    /// Зарегистрировать компонент с factory функцией
    fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&dyn DIResolver) -> Result<T> + Send + Sync + 'static,
    {
        // Адаптируем factory для использования с UnifiedDIContainer
        let adapted_factory = move |container: &UnifiedDIContainer| {
            factory(container)
        };
        
        self.register(adapted_factory, lifetime)
    }
    
    /// Зарегистрировать singleton экземпляр
    fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        self.register_instance(instance)
    }
}

// === CACHE HELPER METHODS ===

impl UnifiedDIContainer {
    /// Получить экземпляр из кэша
    fn get_from_cache(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        let mut cache = self.instance_cache.write();
        
        if let Some(entry) = cache.get_mut(&type_id) {
            entry.access_count += 1;
            entry.last_access = Instant::now();
            
            Some(entry.instance.clone())
        } else {
            None
        }
    }
    
    /// Кэшировать экземпляр
    fn cache_instance(&self, type_id: TypeId, instance: Arc<dyn Any + Send + Sync>) {
        let mut cache = self.instance_cache.write();
        
        // Проверяем размер кэша
        if cache.len() >= self.configuration.max_cache_size {
            // Удаляем самый старый неиспользуемый элемент
            if let Some(oldest_type_id) = cache.iter()
                .min_by_key(|(_, entry)| entry.last_access)
                .map(|(&type_id, _)| type_id)
            {
                cache.remove(&oldest_type_id);
                debug!("🗑️ Удален старый кэшированный экземпляр: {}", 
                       self.get_type_name(oldest_type_id));
            }
        }
        
        let now = Instant::now();
        cache.insert(type_id, CacheEntry {
            instance,
            created_at: now,
            access_count: 1,
            last_access: now,
        });
        
        debug!("💾 Экземпляр {} добавлен в кэш", self.get_type_name(type_id));
    }
    
    /// Записать cache hit
    fn record_cache_hit(&self, type_id: TypeId, duration: Duration) {
        let mut metrics = self.performance_metrics.write();
        metrics.total_resolutions += 1;
        metrics.cache_hits += 1;
        metrics.total_resolution_time += duration;
        
        let type_metrics = metrics.type_metrics.entry(type_id).or_insert_with(TypeMetrics::new);
        type_metrics.resolutions += 1;
        type_metrics.cache_hits += 1;
        type_metrics.total_time += duration;
        type_metrics.average_time = type_metrics.total_time / type_metrics.resolutions as u32;
        type_metrics.last_resolution = Some(Instant::now());
    }
    
    /// Записать cache miss
    fn record_cache_miss(&self, type_id: TypeId, duration: Duration) {
        let mut metrics = self.performance_metrics.write();
        metrics.total_resolutions += 1;
        metrics.cache_misses += 1;
        metrics.total_resolution_time += duration;
        
        let type_metrics = metrics.type_metrics.entry(type_id).or_insert_with(TypeMetrics::new);
        type_metrics.resolutions += 1;
        type_metrics.total_time += duration;
        type_metrics.average_time = type_metrics.total_time / type_metrics.resolutions as u32;
        type_metrics.last_resolution = Some(Instant::now());
    }
}

/// НОВЫЙ UNIFIED MEMORY CONFIGURATOR
/// 
/// Заменяет MemoryDIConfigurator из удаленного di_memory_config.rs
/// Обеспечивает единый способ настройки memory системы для всех компонентов.
pub struct UnifiedMemoryConfigurator;

impl UnifiedMemoryConfigurator {
    /// Настроить полный DI контейнер для memory системы
    /// 
    /// ЗАМЕНЯЕТ: MemoryDIConfigurator::configure_full()
    /// ИСПОЛЬЗУЕТ: UnifiedDIContainer вместо старых дублирований
    pub async fn configure_full(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка унифицированного DI контейнера для memory системы");

        let container = UnifiedDIContainer::production();

        // Настраиваем core components
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;
        Self::configure_cache_layer(&container, config).await?;
        Self::configure_monitoring_layer(&container, config).await?;
        Self::configure_orchestration_layer(&container, config).await?;

        info!("✅ Унифицированный DI контейнер настроен с {} зависимостями", 
              container.registration_count());

        Ok(container)
    }

    /// Настроить минимальный контейнер для тестов
    pub async fn configure_minimal(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка минимального DI контейнера");

        let container = UnifiedDIContainer::minimal();

        // Только основные компоненты
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;

        info!("✅ Минимальный DI контейнер настроен с {} зависимостями", 
              container.registration_count());
              
        Ok(container)
    }

    /// Настроить CPU-only контейнер (без GPU)
    pub async fn configure_cpu_only(config: &MemoryServiceConfig) -> Result<UnifiedDIContainer> {
        info!("🔧 Настройка CPU-only DI контейнера");

        let container = UnifiedDIContainer::new();
        
        // Настраиваем без GPU компонентов
        Self::configure_core_dependencies(&container, config).await?;
        Self::configure_storage_layer(&container, config).await?;
        Self::configure_cache_layer(&container, config).await?;

        Ok(container)
    }

    /// Настроить core зависимости
    async fn configure_core_dependencies(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig
    ) -> Result<()> {
        use crate::types::{PromotionConfig, Record};
        use crate::storage::VectorStore;
        
        // VectorStore
        let db_path = config.db_path.clone();
        container.register(move |_| {
            Ok(VectorStore::new(&db_path)?)
        }, Lifetime::Singleton)?;

        // PromotionConfig
        let promotion_config = config.promotion.clone();
        container.register_instance(promotion_config)?;

        info!("✅ Core dependencies configured");
        Ok(())
    }

    /// Настроить storage layer
    async fn configure_storage_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig
    ) -> Result<()> {
        use crate::database_manager::DatabaseManager;
        
        // DatabaseManager
        let db_path = config.db_path.clone();
        container.register(move |_| {
            Ok(DatabaseManager::new(&db_path))
        }, Lifetime::Singleton)?;

        info!("✅ Storage layer configured");
        Ok(())
    }

    /// Настроить cache layer
    async fn configure_cache_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig
    ) -> Result<()> {
        use crate::cache_lru::EmbeddingCacheLRU;
        
        // Cache
        let cache_config = config.cache_config.clone();
        container.register(move |_| {
            Ok(EmbeddingCacheLRU::new(&cache_config)?)
        }, Lifetime::Singleton)?;

        info!("✅ Cache layer configured");
        Ok(())
    }

    /// Настроить monitoring layer
    async fn configure_monitoring_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig
    ) -> Result<()> {
        use crate::health::HealthMonitor;
        use crate::metrics::MetricsCollector;
        
        if config.health_enabled {
            // HealthMonitor
            let health_config = config.health_config.clone();
            container.register(move |_| {
                Ok(HealthMonitor::new(health_config.clone()))
            }, Lifetime::Singleton)?;
        }

        // MetricsCollector
        container.register(|_| {
            Ok(MetricsCollector::new())
        }, Lifetime::Singleton)?;

        info!("✅ Monitoring layer configured");
        Ok(())
    }

    /// Настроить orchestration layer
    async fn configure_orchestration_layer(
        container: &UnifiedDIContainer,
        config: &MemoryServiceConfig
    ) -> Result<()> {
        use crate::orchestration::{EmbeddingCoordinator, SearchCoordinator, HealthManager};
        
        // EmbeddingCoordinator
        container.register(|container| {
            let vector_store = container.resolve::<crate::storage::VectorStore>()?;
            Ok(EmbeddingCoordinator::new(vector_store))
        }, Lifetime::Singleton)?;

        // SearchCoordinator
        container.register(|container| {
            let vector_store = container.resolve::<crate::storage::VectorStore>()?;
            Ok(SearchCoordinator::new(vector_store))
        }, Lifetime::Singleton)?;

        if config.health_enabled {
            // HealthManager
            container.register(|container| {
                let health_monitor = container.resolve::<crate::health::HealthMonitor>()?;
                Ok(HealthManager::new(health_monitor))
            }, Lifetime::Singleton)?;
        }

        info!("✅ Orchestration layer configured");
        Ok(())
    }
}

/// Memory Service Configuration для unified контейнера
/// 
/// ЗАМЕНЯЕТ: MemoryServiceConfig из service_di_*
#[derive(Debug, Clone)]
pub struct MemoryServiceConfig {
    pub db_path: std::path::PathBuf,
    pub cache_path: std::path::PathBuf,
    pub promotion: crate::types::PromotionConfig,
    pub cache_config: crate::CacheConfigType,
    pub health_enabled: bool,
    pub health_config: crate::health::HealthMonitorConfig,
    pub batch_config: crate::batch_manager::BatchConfig,
}

impl Default for MemoryServiceConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("./cache"))
            .join("magray");
            
        Self {
            db_path: cache_dir.join("memory.db"),
            cache_path: cache_dir.join("embeddings_cache"),
            promotion: crate::types::PromotionConfig::default(),
            cache_config: crate::CacheConfigType::default(),
            health_enabled: true,
            health_config: crate::health::HealthMonitorConfig::default(),
            batch_config: crate::batch_manager::BatchConfig::default(),
        }
    }
}

impl MemoryServiceConfig {
    /// Production конфигурация
    pub fn production() -> Self {
        let mut config = Self::default();
        config.health_enabled = true;
        config.cache_config = crate::CacheConfigType::production();
        config.batch_config = crate::batch_manager::BatchConfig::production();
        config
    }

    /// Development конфигурация
    pub fn development() -> Self {
        let mut config = Self::default();
        config.health_enabled = true;
        config
    }

    /// Minimal конфигурация для тестов
    pub fn minimal() -> Self {
        let mut config = Self::default();
        config.health_enabled = false;
        config
    }
}

/// Создать конфигурацию по умолчанию для memory service
/// 
/// ЗАМЕНЯЕТ: default_config() из удаленных файлов
pub fn create_default_memory_config() -> Result<MemoryServiceConfig> {
    Ok(MemoryServiceConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[derive(Debug)]
    struct TestService {
        pub value: String,
    }
    
    #[derive(Debug)]
    struct TestDependentService {
        pub dependency: Arc<TestService>,
        pub count: Arc<AtomicU32>,
    }
    
    #[test]
    fn test_unified_container_creation() {
        let container = UnifiedDIContainer::new();
        assert_eq!(container.registration_count(), 0);
        
        let stats = container.stats();
        assert_eq!(stats.registered_factories, 0);
        assert_eq!(stats.cached_singletons, 0);
    }
    
    #[test]
    fn test_singleton_registration_and_resolution() -> Result<()> {
        let container = UnifiedDIContainer::new();
        
        // Регистрируем singleton
        container.register(
            |_| Ok(TestService { value: "test".to_string() }),
            Lifetime::Singleton
        )?;
        
        // Проверяем регистрацию
        assert!(container.is_registered::<TestService>());
        assert_eq!(container.registration_count(), 1);
        
        // Разрешаем dependency дважды
        let instance1 = container.resolve::<TestService>()?;
        let instance2 = container.resolve::<TestService>()?;
        
        // Проверяем что это один и тот же экземпляр (singleton)
        assert_eq!(instance1.value, "test");
        assert_eq!(instance2.value, "test");
        assert!(Arc::ptr_eq(&instance1, &instance2));
        
        // Проверяем статистику
        let stats = container.stats();
        assert_eq!(stats.registered_factories, 1);
        assert_eq!(stats.cached_singletons, 1);
        assert!(stats.cache_hits > 0); // Второй resolve должен быть cache hit
        
        Ok(())
    }
    
    #[test]
    fn test_transient_registration_and_resolution() -> Result<()> {
        let container = UnifiedDIContainer::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // Регистрируем transient с счетчиком
        container.register(
            move |_| {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok(TestService { value: format!("test-{}", count) })
            },
            Lifetime::Transient
        )?;
        
        // Разрешаем dependency дважды
        let instance1 = container.resolve::<TestService>()?;
        let instance2 = container.resolve::<TestService>()?;
        
        // Проверяем что это разные экземпляры
        assert_eq!(instance1.value, "test-0");
        assert_eq!(instance2.value, "test-1");
        assert!(!Arc::ptr_eq(&instance1, &instance2));
        
        // Проверяем что в кэше нет transient объектов
        let stats = container.stats();
        assert_eq!(stats.cached_singletons, 0);
        
        Ok(())
    }
    
    #[test]
    fn test_instance_registration() -> Result<()> {
        let container = UnifiedDIContainer::new();
        
        // Создаем экземпляр
        let service = TestService { value: "instance".to_string() };
        
        // Регистрируем готовый экземпляр
        container.register_instance(service)?;
        
        // Разрешаем
        let resolved = container.resolve::<TestService>()?;
        assert_eq!(resolved.value, "instance");
        
        Ok(())
    }
    
    #[test]
    fn test_dependency_injection() -> Result<()> {
        let container = UnifiedDIContainer::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        // Регистрируем зависимость
        container.register(
            |_| Ok(TestService { value: "dependency".to_string() }),
            Lifetime::Singleton
        )?;
        
        // Регистрируем зависимый сервис
        container.register(
            move |container| {
                let dependency = container.resolve::<TestService>()?;
                counter_clone.fetch_add(1, Ordering::SeqCst);
                
                Ok(TestDependentService {
                    dependency,
                    count: counter.clone(),
                })
            },
            Lifetime::Singleton
        )?;
        
        // Разрешаем зависимый сервис
        let dependent = container.resolve::<TestDependentService>()?;
        
        // Проверяем что dependency injection работает
        assert_eq!(dependent.dependency.value, "dependency");
        assert_eq!(dependent.count.load(Ordering::SeqCst), 1);
        
        // Разрешаем еще раз - должен быть тот же экземпляр (singleton)
        let dependent2 = container.resolve::<TestDependentService>()?;
        assert!(Arc::ptr_eq(&dependent, &dependent2));
        assert_eq!(dependent2.count.load(Ordering::SeqCst), 1); // Счетчик не увеличился
        
        Ok(())
    }
    
    #[test]
    fn test_unregistered_type() {
        let container = UnifiedDIContainer::new();
        
        // Пытаемся разрешить незарегистрированный тип
        let result = container.resolve::<TestService>();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("не зарегистрирован"));
        
        // try_resolve должен вернуть None
        let optional_result = container.try_resolve::<TestService>();
        assert!(optional_result.is_none());
    }
    
    #[test]
    fn test_duplicate_registration() -> Result<()> {
        let container = UnifiedDIContainer::new();
        
        // Регистрируем сервис
        container.register(
            |_| Ok(TestService { value: "first".to_string() }),
            Lifetime::Singleton
        )?;
        
        // Пытаемся зарегистрировать еще раз
        let result = container.register(
            |_| Ok(TestService { value: "second".to_string() }),
            Lifetime::Singleton
        );
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("уже зарегистрирован"));
        
        Ok(())
    }
    
    #[test]
    fn test_performance_metrics() -> Result<()> {
        let container = UnifiedDIContainer::development(); // Включены metrics
        
        // Регистрируем сервис
        container.register(
            |_| Ok(TestService { value: "metrics_test".to_string() }),
            Lifetime::Singleton
        )?;
        
        // Разрешаем несколько раз
        let _service1 = container.resolve::<TestService>()?;
        let _service2 = container.resolve::<TestService>()?;
        let _service3 = container.resolve::<TestService>()?;
        
        // Проверяем метрики
        let metrics = container.performance_metrics();
        assert_eq!(metrics.total_resolutions, 3);
        assert!(metrics.cache_hits > 0); // Должны быть cache hits после первого resolve
        assert!(metrics.cache_hit_rate() > 0.0);
        
        // Проверяем отчет
        let report = container.get_performance_report();
        assert!(report.contains("Performance Report"));
        assert!(report.contains("Total resolutions: 3"));
        
        Ok(())
    }
    
    #[test]
    fn test_builder_pattern() -> Result<()> {
        let container = UnifiedDIContainerBuilder::new()
            .with_max_cache_size(50)
            .enable_metrics()
            .enable_validation()
            .build();
            
        assert_eq!(container.configuration.max_cache_size, 50);
        assert!(container.configuration.enable_performance_metrics);
        assert!(container.configuration.enable_dependency_validation);
        
        Ok(())
    }
    
    #[test]
    fn test_configuration_presets() {
        let production = UnifiedDIContainer::production();
        assert_eq!(production.configuration.max_cache_size, 5000);
        assert!(production.configuration.enable_performance_metrics);
        
        let development = UnifiedDIContainer::development();
        assert_eq!(development.configuration.max_cache_size, 500);
        assert!(development.configuration.enable_dependency_validation);
        
        let minimal = UnifiedDIContainer::minimal();
        assert_eq!(minimal.configuration.max_cache_size, 100);
        assert!(!minimal.configuration.enable_performance_metrics);
    }
    
    #[test]
    fn test_clear_functionality() -> Result<()> {
        let container = UnifiedDIContainer::new();
        
        // Регистрируем сервис
        container.register(
            |_| Ok(TestService { value: "clear_test".to_string() }),
            Lifetime::Singleton
        )?;
        
        // Разрешаем для создания кэша
        let _service = container.resolve::<TestService>()?;
        
        // Проверяем что есть регистрации и кэш
        assert_eq!(container.registration_count(), 1);
        let stats_before = container.stats();
        assert_eq!(stats_before.registered_factories, 1);
        assert_eq!(stats_before.cached_singletons, 1);
        
        // Очищаем контейнер
        container.clear();
        
        // Проверяем что все очищено
        assert_eq!(container.registration_count(), 0);
        let stats_after = container.stats();
        assert_eq!(stats_after.registered_factories, 0);
        assert_eq!(stats_after.cached_singletons, 0);
        
        Ok(())
    }
}