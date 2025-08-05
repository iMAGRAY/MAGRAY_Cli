use anyhow::Result;
use std::sync::Arc;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;
use parking_lot::RwLock;
use tracing::{debug, info, warn};
use std::future::Future;
use std::pin::Pin;

/// Тип factory функции для создания компонентов
pub type Factory = Box<dyn Fn(&DIContainer) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Тип async factory функции для создания компонентов
pub type AsyncFactory = Box<dyn Fn(&DIContainer) -> Pin<Box<dyn Future<Output = Result<Arc<dyn Any + Send + Sync>>> + Send>> + Send + Sync>;

/// Placeholder для lazy async компонентов
pub struct LazyAsync<T> {
    _phantom: std::marker::PhantomData<T>,
}

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
// @component: {"k":"C","id":"di_container","t":"Dependency injection container","m":{"cur":90,"tgt":95,"u":"%"},"f":["di","ioc","architecture","validation","performance","async"]}
pub struct DIContainer {
    /// Зарегистрированные factory функции
    factories: RwLock<HashMap<TypeId, (Factory, Lifetime)>>,
    /// Зарегистрированные async factory функции
    async_factories: RwLock<HashMap<TypeId, (AsyncFactory, Lifetime)>>,
    /// Кэш singleton экземпляров
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Имена типов для отладки
    type_names: RwLock<HashMap<TypeId, String>>,
    /// Граф зависимостей для валидации циклов
    dependency_graph: RwLock<DependencyGraph>,
    /// Performance метрики
    performance_metrics: RwLock<DIPerformanceMetrics>,
}

impl DIContainer {
    /// Создать новый контейнер
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
            async_factories: RwLock::new(HashMap::new()),
            singletons: RwLock::new(HashMap::new()),
            type_names: RwLock::new(HashMap::new()),
            dependency_graph: RwLock::new(DependencyGraph::new()),
            performance_metrics: RwLock::new(DIPerformanceMetrics::default()),
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

    /// Зарегистрировать async factory функцию (упрощенная версия)
    /// Async компоненты будут созданы заранее и зарегистрированы как instances
    pub fn register_async_placeholder<T>(&self) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>().to_string();
        debug!("Registered async placeholder for {}", type_name);
        // Это просто placeholder - реальные async компоненты создаются в конфигураторе
        Ok(())
    }

    /// Разрешить async зависимость (упрощенная версия)
    /// Просто использует обычный resolve, так как async компоненты создаются заранее
    pub async fn resolve_async<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        // Async resolve просто делегирует к sync resolve
        self.resolve::<T>()
    }

    /// Разрешить зависимость
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        use std::time::Instant;
        
        let start_time = Instant::now();
        let type_id = TypeId::of::<T>();
        let type_name = self.get_type_name(type_id);
        let from_cache = false;

        // Сначала проверяем singleton кэш
        {
            let singletons = self.singletons.read();
            if let Some(instance) = singletons.get(&type_id) {
                if let Some(typed_instance) = instance.clone().downcast::<T>().ok() {
                    let _ = from_cache; // Используется для метрик, но не читается
                    // from_cache = true; // Убрано: значение не используется
                    let resolve_duration = start_time.elapsed();
                    
                    // Записываем метрики
                    {
                        let mut metrics = self.performance_metrics.write();
                        metrics.record_resolve(&type_name, resolve_duration, true, true);
                    }
                    
                    debug!("Resolved {} from singleton cache in {:?}", type_name, resolve_duration);
                    return Ok(typed_instance);
                }
            }
        }

        // Проверяем зарегистрированные factory
        let (factory, lifetime) = {
            let factories = self.factories.read();
            factories.get(&type_id)
                .map(|(f, l)| (f as *const Factory, *l))
                .ok_or_else(|| {
                    // Записываем ошибку в метрики
                    {
                        let mut metrics = self.performance_metrics.write();
                        metrics.record_error(&type_name);
                    }
                    warn!("Type {} not registered in DI container", type_name);
                    anyhow::anyhow!("Type {} not registered", type_name)
                })?
        };

        // Засекаем время создания объекта
        let creation_start = Instant::now();
        
        // Безопасно получаем factory (мы знаем что он валиден пока держим container)
        let factory = unsafe { &*factory };
        let instance = factory(self).map_err(|e| {
            // Записываем ошибку в метрики
            {
                let mut metrics = self.performance_metrics.write();
                metrics.record_error(&type_name);
            }
            e
        })?;

        // Пытаемся привести к нужному типу
        let typed_instance = instance.downcast::<T>()
            .map_err(|_| {
                // Записываем ошибку в метрики
                {
                    let mut metrics = self.performance_metrics.write();
                    metrics.record_error(&type_name);
                }
                anyhow::anyhow!("Failed to downcast {} to target type", type_name)
            })?;

        let creation_duration = creation_start.elapsed();
        let total_duration = start_time.elapsed();
        let is_singleton = lifetime == Lifetime::Singleton;

        // Для singleton сохраняем в кэш
        if is_singleton {
            let mut singletons = self.singletons.write();
            singletons.insert(type_id, typed_instance.clone() as Arc<dyn Any + Send + Sync>);
            debug!("Cached {} as singleton", type_name);
        }

        // Записываем метрики успешного resolve
        {
            let mut metrics = self.performance_metrics.write();
            metrics.record_resolve(&type_name, total_duration, from_cache, is_singleton);
        }

        debug!("Resolved {} with {:?} lifetime in {:?} (creation: {:?})", 
               type_name, lifetime, total_duration, creation_duration);
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
                // Более детальное логирование для отладки Unknown type
                if e.to_string().contains("not registered") {
                    warn!("Optional dependency {} not registered (try_resolve)", type_name);
                }
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

    /// Добавить информацию о зависимости для валидации циклов
    pub fn add_dependency_info<TFrom, TTo>(&self) -> Result<()>
    where
        TFrom: Any + 'static,
        TTo: Any + 'static,
    {
        let from_type_id = TypeId::of::<TFrom>();
        let to_type_id = TypeId::of::<TTo>();
        let from_name = std::any::type_name::<TFrom>().to_string();
        let to_name = std::any::type_name::<TTo>().to_string();

        let mut graph = self.dependency_graph.write();
        graph.add_dependency(from_type_id, to_type_id, from_name, to_name);
        
        debug!("Added dependency: {} -> {}", 
               std::any::type_name::<TFrom>(), 
               std::any::type_name::<TTo>());
        Ok(())
    }

    /// Валидация зависимостей при старте (включая проверку циклов)
    pub fn validate_dependencies(&self) -> Result<()> {
        let factories = self.factories.read();
        let type_names = self.type_names.read();
        let graph = self.dependency_graph.read();
        
        info!("🔍 Валидация {} зарегистрированных зависимостей", factories.len());
        
        // 1. Проверяем, что все factory функции корректные
        for (type_id, _) in factories.iter() {
            let type_name = type_names.get(type_id)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            
            debug!("✓ Dependency {} зарегистрирована", type_name);
        }
        
        // 2. Проверяем граф на циркулярные зависимости
        match graph.validate_no_cycles() {
            Ok(sorted_order) => {
                if !sorted_order.is_empty() {
                    let sorted_names: Vec<String> = sorted_order
                        .iter()
                        .take(5) // Показываем первые 5 для краткости
                        .map(|type_id| {
                            graph.type_names.get(type_id)
                                .cloned()
                                .unwrap_or_else(|| "Unknown".to_string())
                        })
                        .collect();
                    
                    info!("✅ Топологический порядок инициализации: {} {} типов",
                          sorted_names.join(" → "), sorted_order.len());
                }
            }
            Err(cycle_error) => {
                // Получаем детальную информацию о циклах
                let cycles = graph.find_cycles();
                if !cycles.is_empty() {
                    warn!("🔄 Обнаружены следующие циклы:");
                    for (i, cycle) in cycles.iter().enumerate() {
                        warn!("  Цикл {}: {}", i + 1, cycle.join(" → "));
                    }
                }
                return Err(cycle_error);
            }
        }
        
        info!("✅ Все {} зависимостей прошли валидацию успешно", factories.len());
        Ok(())
    }

    /// Получить информацию о циклах зависимостей (для диагностики)
    pub fn get_dependency_cycles(&self) -> Vec<Vec<String>> {
        let graph = self.dependency_graph.read();
        graph.find_cycles()
    }

    /// Получить performance метрики DI системы
    pub fn get_performance_metrics(&self) -> DIPerformanceMetrics {
        let metrics = self.performance_metrics.read();
        metrics.clone()
    }

    /// Сбросить performance метрики (для тестов)
    pub fn reset_performance_metrics(&self) {
        let mut metrics = self.performance_metrics.write();
        *metrics = DIPerformanceMetrics::default();
        debug!("Performance metrics reset");
    }

    /// Получить краткий отчет о производительности
    pub fn get_performance_report(&self) -> String {
        let metrics = self.performance_metrics.read();
        
        if metrics.total_resolves == 0 {
            return "📊 Performance Report: No operations recorded".to_string();
        }

        let slowest_types = metrics.slowest_types(3);
        let slowest_list = if slowest_types.is_empty() {
            "None".to_string()
        } else {
            slowest_types.iter()
                .map(|(name, tm)| format!("{} ({:.1}μs)", name, tm.avg_creation_time_ns as f64 / 1000.0))
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!(
            "📊 DI Performance Report:\n\
             ┌─ Total resolves: {}\n\
             ├─ Cache hit rate: {:.1}%\n\
             ├─ Avg resolve time: {:.1}μs\n\
             ├─ Max resolve time: {:.1}μs\n\
             ├─ Min resolve time: {:.1}μs\n\
             ├─ Factory creates: {}\n\
             ├─ Unique types: {}\n\
             └─ Slowest types: {}",
            metrics.total_resolves,
            metrics.cache_hit_rate(),
            metrics.avg_resolve_time_us(),
            metrics.max_resolve_time_ns as f64 / 1000.0,
            if metrics.min_resolve_time_ns == u64::MAX { 0.0 } else { metrics.min_resolve_time_ns as f64 / 1000.0 },
            metrics.factory_creates,
            metrics.type_metrics.len(),
            slowest_list
        )
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

/// Performance метрики для DI операций
#[derive(Debug, Clone)]
pub struct DIPerformanceMetrics {
    /// Общее количество resolve операций
    pub total_resolves: u64,
    /// Количество resolve из singleton кэша
    pub cache_hits: u64,
    /// Количество новых объектов created
    pub factory_creates: u64,
    /// Среднее время resolve операции
    pub avg_resolve_time_ns: u64,
    /// Максимальное время resolve операции
    pub max_resolve_time_ns: u64,
    /// Минимальное время resolve операции  
    pub min_resolve_time_ns: u64,
    /// Метрики по типам
    pub type_metrics: HashMap<String, TypeMetrics>,
}

/// Метрики для конкретного типа
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    /// Количество resolve для этого типа
    pub resolve_count: u64,
    /// Общее время создания (наносекунды)
    pub total_creation_time_ns: u64,
    /// Среднее время создания (наносекунды)
    pub avg_creation_time_ns: u64,
    /// Количество ошибок при создании
    pub error_count: u64,
    /// Является ли singleton (кэшируется)
    pub is_singleton: bool,
}

impl Default for DIPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_resolves: 0,
            cache_hits: 0,
            factory_creates: 0,
            avg_resolve_time_ns: 0,
            max_resolve_time_ns: 0,
            min_resolve_time_ns: u64::MAX,
            type_metrics: HashMap::new(),
        }
    }
}

impl DIPerformanceMetrics {
    /// Добавить измерение resolve операции
    fn record_resolve(&mut self, type_name: &str, duration: Duration, from_cache: bool, is_singleton: bool) {
        let duration_ns = duration.as_nanos() as u64;
        
        // Обновляем общие метрики
        self.total_resolves += 1;
        if from_cache {
            self.cache_hits += 1;
        } else {
            self.factory_creates += 1;
        }
        
        // Обновляем timing метрики
        if duration_ns > self.max_resolve_time_ns {
            self.max_resolve_time_ns = duration_ns;
        }
        if duration_ns < self.min_resolve_time_ns && duration_ns > 0 {
            self.min_resolve_time_ns = duration_ns;
        }
        
        // Пересчитываем среднее время
        let total_time = self.avg_resolve_time_ns * (self.total_resolves - 1) + duration_ns;
        self.avg_resolve_time_ns = total_time / self.total_resolves;
        
        // Обновляем метрики по типу
        let type_metrics = self.type_metrics.entry(type_name.to_string()).or_insert_with(|| TypeMetrics {
            resolve_count: 0,
            total_creation_time_ns: 0,
            avg_creation_time_ns: 0,
            error_count: 0,
            is_singleton,
        });
        
        type_metrics.resolve_count += 1;
        if !from_cache {
            type_metrics.total_creation_time_ns += duration_ns;
            type_metrics.avg_creation_time_ns = type_metrics.total_creation_time_ns / type_metrics.resolve_count;
        }
    }
    
    /// Записать ошибку создания
    fn record_error(&mut self, type_name: &str) {
        let type_metrics = self.type_metrics.entry(type_name.to_string()).or_insert_with(|| TypeMetrics {
            resolve_count: 0,
            total_creation_time_ns: 0,
            avg_creation_time_ns: 0,
            error_count: 0,
            is_singleton: false,
        });
        
        type_metrics.error_count += 1;
    }

    /// Обновить метрики для типа
    pub fn update_type_metrics(&mut self, type_name: &str, duration_ns: u64, from_cache: bool, error: bool) {
        let duration = Duration::from_nanos(duration_ns);
        
        if error {
            self.record_error(type_name);
        } else {
            // Определяем is_singleton на основе того, был ли hit из кэша
            let is_singleton = from_cache || self.type_metrics.get(type_name)
                .map(|m| m.is_singleton)
                .unwrap_or(false);
            self.record_resolve(type_name, duration, from_cache, is_singleton);
        }
    }
    
    /// Получить cache hit rate в процентах
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_resolves == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.total_resolves as f64) * 100.0
        }
    }
    
    /// Получить среднее время resolve в микросекундах
    pub fn avg_resolve_time_us(&self) -> f64 {
        self.avg_resolve_time_ns as f64 / 1000.0
    }
    
    /// Найти самые медленные типы для создания
    pub fn slowest_types(&self, limit: usize) -> Vec<(&String, &TypeMetrics)> {
        let mut types: Vec<_> = self.type_metrics.iter().collect();
        types.sort_by(|a, b| b.1.avg_creation_time_ns.cmp(&a.1.avg_creation_time_ns));
        types.into_iter().take(limit).collect()
    }
}

/// Граф зависимостей для валидации циркулярных зависимостей
#[derive(Debug)]
struct DependencyGraph {
    /// Карта TypeId -> список TypeId зависимостей
    dependencies: HashMap<TypeId, HashSet<TypeId>>,
    /// Карта TypeId -> имя типа для отладки
    type_names: HashMap<TypeId, String>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            type_names: HashMap::new(),
        }
    }

    /// Добавить зависимость: from_type зависит от to_type
    fn add_dependency(&mut self, from_type: TypeId, to_type: TypeId, from_name: String, to_name: String) {
        self.dependencies.entry(from_type).or_insert_with(HashSet::new).insert(to_type);
        self.type_names.insert(from_type, from_name);
        self.type_names.insert(to_type, to_name);
    }

    /// Проверить граф на циркулярные зависимости с помощью топологической сортировки
    fn validate_no_cycles(&self) -> Result<Vec<TypeId>> {
        if self.dependencies.is_empty() {
            return Ok(Vec::new());
        }

        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();
        let mut all_types: HashSet<TypeId> = HashSet::new();

        // Собираем все типы и вычисляем in-degree
        for (&from_type, deps) in &self.dependencies {
            all_types.insert(from_type);
            in_degree.entry(from_type).or_insert(0);
            
            for &to_type in deps {
                all_types.insert(to_type);
                *in_degree.entry(to_type).or_insert(0) += 1;
            }
        }

        // Начинаем с типов без входящих зависимостей
        let mut queue: VecDeque<TypeId> = VecDeque::new();
        for (&type_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(type_id);
            }
        }

        let mut sorted_order: Vec<TypeId> = Vec::new();
        let mut processed_count = 0;

        // Топологическая сортировка
        while let Some(current_type) = queue.pop_front() {
            sorted_order.push(current_type);
            processed_count += 1;

            // Обрабатываем все зависимости текущего типа
            if let Some(deps) = self.dependencies.get(&current_type) {
                for &dep_type in deps {
                    if let Some(degree) = in_degree.get_mut(&dep_type) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep_type);
                        }
                    }
                }
            }
        }

        // Если обработали не все типы, значит есть циклы
        if processed_count != all_types.len() {
            let remaining_types: Vec<String> = all_types
                .iter()
                .filter(|type_id| !sorted_order.contains(type_id))
                .map(|type_id| {
                    self.type_names.get(type_id)
                        .cloned()
                        .unwrap_or_else(|| format!("TypeId({:?})", type_id))
                })
                .collect();

            return Err(anyhow::anyhow!(
                "Циркулярные зависимости обнаружены в типах: {}. \
                Это может привести к бесконечной рекурсии при создании объектов.",
                remaining_types.join(", ")
            ));
        }

        info!("✅ Валидация зависимостей прошла успешно. Топологический порядок: {}",
              sorted_order.len());
        
        Ok(sorted_order)
    }

    /// Найти циклы в графе зависимостей (для детальной диагностики)
    fn find_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &start_type in self.dependencies.keys() {
            if !visited.contains(&start_type) {
                self.dfs_find_cycles(
                    start_type,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    fn dfs_find_cycles(
        &self,
        current: TypeId,
        visited: &mut HashSet<TypeId>,
        rec_stack: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(current);
        rec_stack.insert(current);
        path.push(current);

        if let Some(deps) = self.dependencies.get(&current) {
            for &neighbor in deps {
                if !visited.contains(&neighbor) {
                    self.dfs_find_cycles(neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&neighbor) {
                    // Найден цикл
                    if let Some(cycle_start) = path.iter().position(|&x| x == neighbor) {
                        let cycle_types: Vec<String> = path[cycle_start..]
                            .iter()
                            .chain(std::iter::once(&neighbor))
                            .map(|type_id| {
                                self.type_names.get(type_id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("TypeId({:?})", type_id))
                            })
                            .collect();
                        cycles.push(cycle_types);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(&current);
    }
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

    /// Зарегистрировать placeholder для async singleton
    pub fn register_async_placeholder<T>(self) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
    {
        self.container.register_async_placeholder::<T>()?;
        Ok(self)
    }

    /// Получить доступ к контейнеру для разрешения зависимостей во время конфигурации
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.container.resolve::<T>()
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

    #[test]
    fn test_circular_dependency_detection() -> Result<()> {
        let container = DIContainer::new();

        // Создаем циркулярную зависимость: ServiceA -> ServiceB -> ServiceA
        struct ServiceA {
            #[allow(dead_code)]
            b: Option<Arc<ServiceB>>,
        }
        
        struct ServiceB {
            #[allow(dead_code)]
            a: Option<Arc<ServiceA>>,
        }

        // Регистрируем типы без зависимостей для начала
        container.register(
            |_| Ok(ServiceA { b: None }),
            Lifetime::Singleton
        )?;

        container.register(
            |_| Ok(ServiceB { a: None }),
            Lifetime::Singleton
        )?;

        // Добавляем информацию о циркулярных зависимостях
        container.add_dependency_info::<ServiceA, ServiceB>()?;
        container.add_dependency_info::<ServiceB, ServiceA>()?;

        // Валидация должна найти циркулярную зависимость
        let validation_result = container.validate_dependencies();
        assert!(validation_result.is_err());

        let cycles = container.get_dependency_cycles();
        assert!(!cycles.is_empty());
        assert!(cycles[0].len() >= 2); // Цикл должен содержать как минимум 2 элемента

        Ok(())
    }

    #[test]
    fn test_valid_dependency_chain() -> Result<()> {
        let container = DIContainer::new();

        struct ServiceA;
        struct ServiceB;
        struct ServiceC;

        // Создаем цепочку зависимостей: C -> B -> A (без циклов)
        container.register(|_| Ok(ServiceA), Lifetime::Singleton)?;
        container.register(|_| Ok(ServiceB), Lifetime::Singleton)?;
        container.register(|_| Ok(ServiceC), Lifetime::Singleton)?;

        // Добавляем информацию о зависимостях
        container.add_dependency_info::<ServiceC, ServiceB>()?;
        container.add_dependency_info::<ServiceB, ServiceA>()?;

        // Валидация должна пройти успешно
        let validation_result = container.validate_dependencies();
        assert!(validation_result.is_ok());

        let cycles = container.get_dependency_cycles();
        assert!(cycles.is_empty()); // Циклов быть не должно

        Ok(())
    }

    #[test]
    fn test_complex_circular_dependency() -> Result<()> {
        let container = DIContainer::new();

        struct ServiceA;
        struct ServiceB;
        struct ServiceC;
        struct ServiceD;

        // Регистрируем сервисы
        container.register(|_| Ok(ServiceA), Lifetime::Singleton)?;
        container.register(|_| Ok(ServiceB), Lifetime::Singleton)?;
        container.register(|_| Ok(ServiceC), Lifetime::Singleton)?;
        container.register(|_| Ok(ServiceD), Lifetime::Singleton)?;

        // Создаем сложный граф с циклом: A -> B -> C -> D -> B
        container.add_dependency_info::<ServiceA, ServiceB>()?;
        container.add_dependency_info::<ServiceB, ServiceC>()?;
        container.add_dependency_info::<ServiceC, ServiceD>()?;
        container.add_dependency_info::<ServiceD, ServiceB>()?; // Цикл!

        // Валидация должна найти цикл
        let validation_result = container.validate_dependencies();
        assert!(validation_result.is_err());

        let cycles = container.get_dependency_cycles();
        assert!(!cycles.is_empty());

        Ok(())
    }

    #[test]
    fn test_performance_metrics() -> Result<()> {
        let container = DIContainer::new();
        
        // Регистрируем несколько сервисов
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton
        )?;

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Transient
        )?;

        // Сбрасываем метрики для чистого теста
        container.reset_performance_metrics();

        // Выполняем несколько resolve операций
        let _service1 = container.resolve::<TestService>()?; // Первый resolve (создание)
        let _service2 = container.resolve::<TestService>()?; // Второй resolve (из кэша)
        let _service3 = container.resolve::<TestService>()?; // Третий resolve (из кэша)

        // Проверяем метрики
        let metrics = container.get_performance_metrics();
        assert_eq!(metrics.total_resolves, 3);
        assert_eq!(metrics.cache_hits, 2); // Второй и третий из кэша
        assert_eq!(metrics.factory_creates, 1); // Только первый создан factory

        // Проверяем cache hit rate
        let hit_rate = metrics.cache_hit_rate();
        assert!((hit_rate - 66.666).abs() < 1.0); // ~66.67%

        // Проверяем что времена измерены
        assert!(metrics.avg_resolve_time_ns > 0);
        assert!(metrics.max_resolve_time_ns > 0);
        assert!(metrics.min_resolve_time_ns > 0);

        // Проверяем метрики по типам
        let type_name = std::any::type_name::<TestService>();
        assert!(metrics.type_metrics.contains_key(type_name));
        let type_metrics = &metrics.type_metrics[type_name];
        assert_eq!(type_metrics.resolve_count, 3);
        assert!(type_metrics.is_singleton);
        assert_eq!(type_metrics.error_count, 0);

        // Проверяем отчет
        let report = container.get_performance_report();
        assert!(report.contains("Total resolves: 3"));
        assert!(report.contains("Cache hit rate:"));
        assert!(report.contains("Factory creates: 1"));

        Ok(())
    }

    #[test]
    fn test_performance_metrics_errors() -> Result<()> {
        let container = DIContainer::new();
        
        // НЕ регистрируем TestService
        container.reset_performance_metrics();

        // Пытаемся resolve незарегистрированный сервис
        let result = container.resolve::<TestService>();
        assert!(result.is_err());

        // Проверяем что ошибка записана в метрики
        let metrics = container.get_performance_metrics();
        let type_name = std::any::type_name::<TestService>();
        
        if let Some(type_metrics) = metrics.type_metrics.get(type_name) {
            assert_eq!(type_metrics.error_count, 1);
        }

        Ok(())
    }

    #[test]  
    fn test_performance_report_empty() {
        let container = DIContainer::new();
        container.reset_performance_metrics();

        let report = container.get_performance_report();
        assert!(report.contains("No operations recorded"));
    }
}