//! CoordinatorRegistry - реестр и управление координаторами
//!
//! Реализует Single Responsibility Principle для регистрации координаторов,
//! управления их зависимостями и метаданными.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::orchestration::traits::Coordinator;

/// Метаданные координатора
#[derive(Debug, Clone)]
pub struct CoordinatorMetadata {
    pub name: String,
    pub priority: u32,
    pub dependencies: Vec<String>,
    pub capabilities: Vec<String>,
    pub tags: HashMap<String, String>,
    pub initialization_order: u32,
    pub is_critical: bool,
}

impl CoordinatorMetadata {
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            priority: 100, // default priority
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            tags: HashMap::new(),
            initialization_order: 100, // default order
            is_critical: false,
        }
    }

    /// Builder методы для удобного создания метаданных
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn add_dependency(mut self, dep: String) -> Self {
        self.dependencies.push(dep);
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn add_capability(mut self, cap: String) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    pub fn with_initialization_order(mut self, order: u32) -> Self {
        self.initialization_order = order;
        self
    }

    pub fn as_critical(mut self) -> Self {
        self.is_critical = true;
        self
    }
}

/// Статус регистрации координатора
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationStatus {
    Registered,
    DependenciesNotMet,
    CircularDependency,
    Duplicate,
}

/// Результат регистрации
#[derive(Debug)]
pub struct RegistrationResult {
    pub status: RegistrationStatus,
    pub message: String,
    pub missing_dependencies: Vec<String>,
}

/// Запись о координаторе в реестре
#[derive(Debug)]
pub struct CoordinatorEntry {
    pub coordinator: Arc<dyn Coordinator>,
    pub metadata: CoordinatorMetadata,
    pub registration_time: std::time::Instant,
    pub is_active: bool,
}

/// Реестр координаторов с управлением зависимостями
pub struct CoordinatorRegistry {
    // Зарегистрированные координаторы
    coordinators: HashMap<String, CoordinatorEntry>,

    // Граф зависимостей для проверки циклов
    dependency_graph: HashMap<String, HashSet<String>>,

    // Порядок инициализации (топологическая сортировка)
    initialization_order: Vec<String>,

    // Кэш для результатов проверки зависимостей
    dependency_cache: HashMap<String, bool>,

    // Группы координаторов по тегам
    tag_groups: HashMap<String, HashSet<String>>,
}

impl CoordinatorRegistry {
    /// Создать новый реестр
    pub fn new() -> Self {
        Self {
            coordinators: HashMap::new(),
            dependency_graph: HashMap::new(),
            initialization_order: Vec::new(),
            dependency_cache: HashMap::new(),
            tag_groups: HashMap::new(),
        }
    }

    /// Зарегистрировать координатор
    pub fn register_coordinator(
        &mut self,
        coordinator: Arc<dyn Coordinator>,
        metadata: CoordinatorMetadata,
    ) -> RegistrationResult {
        let name = metadata.name.clone();

        // Проверяем дубликаты
        if self.coordinators.contains_key(&name) {
            return RegistrationResult {
                status: RegistrationStatus::Duplicate,
                message: format!("Координатор '{}' уже зарегистрирован", name),
                missing_dependencies: Vec::new(),
            };
        }

        // Проверяем зависимости
        let missing_deps = self.check_dependencies(&metadata.dependencies);
        if !missing_deps.is_empty() {
            return RegistrationResult {
                status: RegistrationStatus::DependenciesNotMet,
                message: format!("Не все зависимости доступны для '{}'", name),
                missing_dependencies: missing_deps,
            };
        }

        // Проверяем циклические зависимости
        if self.would_create_cycle(&name, &metadata.dependencies) {
            return RegistrationResult {
                status: RegistrationStatus::CircularDependency,
                message: format!("Регистрация '{}' создала бы циклическую зависимость", name),
                missing_dependencies: Vec::new(),
            };
        }

        // Регистрируем координатор
        let entry = CoordinatorEntry {
            coordinator,
            metadata: metadata.clone(),
            registration_time: std::time::Instant::now(),
            is_active: true,
        };

        self.coordinators.insert(name.clone(), entry);

        // Обновляем граф зависимостей
        self.dependency_graph.insert(
            name.clone(),
            metadata.dependencies.iter().cloned().collect(),
        );

        // Обновляем группы по тегам
        for (tag_key, tag_value) in &metadata.tags {
            let group_key = format!("{}:{}", tag_key, tag_value);
            self.tag_groups
                .entry(group_key)
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        // Пересчитываем порядок инициализации
        self.recalculate_initialization_order();

        // Очищаем кэш зависимостей
        self.dependency_cache.clear();

        info!(
            "Координатор '{}' успешно зарегистрирован (приоритет: {})",
            name, metadata.priority
        );

        RegistrationResult {
            status: RegistrationStatus::Registered,
            message: format!("Координатор '{}' успешно зарегистрирован", name),
            missing_dependencies: Vec::new(),
        }
    }

    /// Отменить регистрацию координатора
    pub fn unregister_coordinator(&mut self, name: &str) -> Result<()> {
        // Проверяем есть ли координаторы, зависящие от этого
        let dependents = self.find_dependents(name);
        if !dependents.is_empty() {
            return Err(anyhow::anyhow!(
                "Нельзя удалить '{}' - от него зависят: {:?}",
                name,
                dependents
            ));
        }

        // Удаляем координатор
        if self.coordinators.remove(name).is_some() {
            // Удаляем из графа зависимостей
            self.dependency_graph.remove(name);

            // Удаляем из групп тегов
            for group in self.tag_groups.values_mut() {
                group.remove(name);
            }

            // Пересчитываем порядок инициализации
            self.recalculate_initialization_order();

            // Очищаем кэш
            self.dependency_cache.clear();

            info!("Координатор '{}' удален из реестра", name);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Координатор '{}' не найден", name))
        }
    }

    /// Получить координатор по имени
    pub fn get_coordinator(&self, name: &str) -> Option<Arc<dyn Coordinator>> {
        self.coordinators
            .get(name)
            .map(|entry| Arc::clone(&entry.coordinator))
    }

    /// Получить все координаторы
    pub fn get_all_coordinators(&self) -> HashMap<String, Arc<dyn Coordinator>> {
        self.coordinators
            .iter()
            .map(|(name, entry)| (name.clone(), Arc::clone(&entry.coordinator)))
            .collect()
    }

    /// Получить координаторы в порядке инициализации
    pub fn get_initialization_order(&self) -> Vec<(String, Arc<dyn Coordinator>)> {
        self.initialization_order
            .iter()
            .filter_map(|name| {
                self.coordinators
                    .get(name)
                    .map(|entry| (name.clone(), Arc::clone(&entry.coordinator)))
            })
            .collect()
    }

    /// Получить координаторы по тегу
    pub fn get_coordinators_by_tag(
        &self,
        tag_key: &str,
        tag_value: &str,
    ) -> Vec<Arc<dyn Coordinator>> {
        let group_key = format!("{}:{}", tag_key, tag_value);

        match self.tag_groups.get(&group_key) {
            Some(names) => names
                .iter()
                .filter_map(|name| self.get_coordinator(name))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Получить критические координаторы
    pub fn get_critical_coordinators(&self) -> Vec<Arc<dyn Coordinator>> {
        self.coordinators
            .values()
            .filter(|entry| entry.metadata.is_critical)
            .map(|entry| Arc::clone(&entry.coordinator))
            .collect()
    }

    /// Получить метаданные координатора
    pub fn get_metadata(&self, name: &str) -> Option<&CoordinatorMetadata> {
        self.coordinators.get(name).map(|entry| &entry.metadata)
    }

    /// Получить все метаданные
    pub fn get_all_metadata(&self) -> HashMap<String, &CoordinatorMetadata> {
        self.coordinators
            .iter()
            .map(|(name, entry)| (name.clone(), &entry.metadata))
            .collect()
    }

    /// Проверить готовность всех зависимостей координатора
    pub async fn check_coordinator_dependencies_ready(&self, name: &str) -> bool {
        if let Some(metadata) = self.get_metadata(name) {
            for dep_name in &metadata.dependencies {
                if let Some(coordinator) = self.get_coordinator(dep_name) {
                    if !coordinator.is_ready().await {
                        debug!("Зависимость '{}' не готова для '{}'", dep_name, name);
                        return false;
                    }
                } else {
                    warn!("Зависимость '{}' не найдена для '{}'", dep_name, name);
                    return false;
                }
            }
        }
        true
    }

    /// Получить статистику реестра
    pub fn get_registry_stats(&self) -> String {
        let mut stats = String::new();

        stats.push_str("=== CoordinatorRegistry Statistics ===\n\n");
        stats.push_str(&format!(
            "Total coordinators: {}\n",
            self.coordinators.len()
        ));
        stats.push_str(&format!(
            "Active coordinators: {}\n",
            self.coordinators.values().filter(|e| e.is_active).count()
        ));
        stats.push_str(&format!(
            "Critical coordinators: {}\n",
            self.coordinators
                .values()
                .filter(|e| e.metadata.is_critical)
                .count()
        ));

        stats.push_str("\nInitialization Order:\n");
        for (i, name) in self.initialization_order.iter().enumerate() {
            if let Some(entry) = self.coordinators.get(name) {
                stats.push_str(&format!(
                    "{}. {} (priority: {}, critical: {})\n",
                    i + 1,
                    name,
                    entry.metadata.priority,
                    entry.metadata.is_critical
                ));
            }
        }

        stats.push_str("\nTag Groups:\n");
        for (tag, names) in &self.tag_groups {
            if !names.is_empty() {
                stats.push_str(&format!("├─ {}: {} coordinators\n", tag, names.len()));
            }
        }

        stats.push_str("\nDependency Graph:\n");
        for (name, deps) in &self.dependency_graph {
            if !deps.is_empty() {
                stats.push_str(&format!("├─ {} depends on: {:?}\n", name, deps));
            }
        }

        stats
    }

    /// Проверить есть ли отсутствующие зависимости
    fn check_dependencies(&self, dependencies: &[String]) -> Vec<String> {
        dependencies
            .iter()
            .filter(|dep| !self.coordinators.contains_key(*dep))
            .cloned()
            .collect()
    }

    /// Проверить создаст ли добавление новой зависимости цикл
    fn would_create_cycle(&self, new_name: &str, new_dependencies: &[String]) -> bool {
        // Создаем временный граф с новой зависимостью
        let mut temp_graph = self.dependency_graph.clone();
        temp_graph.insert(
            new_name.to_string(),
            new_dependencies.iter().cloned().collect(),
        );

        self.has_cycle_in_graph(&temp_graph)
    }

    /// Проверить есть ли циклы в графе зависимостей (DFS)
    fn has_cycle_in_graph(&self, graph: &HashMap<String, HashSet<String>>) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                if self.has_cycle_dfs(node, graph, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    /// DFS для поиска циклов
    fn has_cycle_dfs(
        &self,
        node: &str,
        graph: &HashMap<String, HashSet<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle_dfs(neighbor, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Найти координаторы, зависящие от данного
    fn find_dependents(&self, name: &str) -> Vec<String> {
        self.dependency_graph
            .iter()
            .filter(|(_, deps)| deps.contains(name))
            .map(|(coord_name, _)| coord_name.clone())
            .collect()
    }

    /// Пересчитать порядок инициализации (топологическая сортировка)
    fn recalculate_initialization_order(&mut self) {
        // Используем алгоритм Кана для топологической сортировки
        let mut in_degree = HashMap::new();
        let mut graph = HashMap::new();

        // Подсчитываем входящие рёбра и строим граф
        for (name, _) in &self.coordinators {
            in_degree.insert(name.clone(), 0);
            graph.insert(name.clone(), Vec::new());
        }

        for (dependent, dependencies) in &self.dependency_graph {
            for dependency in dependencies {
                if let Some(count) = in_degree.get_mut(dependent) {
                    *count += 1;
                }
                if let Some(adj_list) = graph.get_mut(dependency) {
                    adj_list.push(dependent.clone());
                }
            }
        }

        // Добавляем узлы с нулевой степенью входа
        let mut queue: Vec<_> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        // Сортируем по приоритету для детерминированного порядка
        queue.sort_by(|a, b| {
            let priority_a = self
                .coordinators
                .get(a)
                .map(|e| e.metadata.priority)
                .unwrap_or(0);
            let priority_b = self
                .coordinators
                .get(b)
                .map(|e| e.metadata.priority)
                .unwrap_or(0);
            priority_b.cmp(&priority_a) // Высокий приоритет сначала
        });

        let mut result = Vec::new();

        while let Some(current) = queue.pop() {
            result.push(current.clone());

            if let Some(neighbors) = graph.get(&current) {
                for neighbor in neighbors {
                    if let Some(count) = in_degree.get_mut(neighbor) {
                        *count -= 1;
                        if *count == 0 {
                            // Вставляем в правильную позицию с учетом приоритета
                            let neighbor_priority = self
                                .coordinators
                                .get(neighbor)
                                .map(|e| e.metadata.priority)
                                .unwrap_or(0);

                            let insert_pos = queue
                                .iter()
                                .position(|n| {
                                    let n_priority = self
                                        .coordinators
                                        .get(n)
                                        .map(|e| e.metadata.priority)
                                        .unwrap_or(0);
                                    neighbor_priority > n_priority
                                })
                                .unwrap_or(queue.len());

                            queue.insert(insert_pos, neighbor.clone());
                        }
                    }
                }
            }
        }

        self.initialization_order = result;

        debug!(
            "Пересчитан порядок инициализации: {:?}",
            self.initialization_order
        );
    }

    /// Количество зарегистрированных координаторов
    pub fn coordinator_count(&self) -> usize {
        self.coordinators.len()
    }

    /// Получить имена всех координаторов
    pub fn coordinator_names(&self) -> Vec<String> {
        self.coordinators.keys().cloned().collect()
    }

    /// Проверить зарегистрирован ли координатор
    pub fn is_registered(&self, name: &str) -> bool {
        self.coordinators.contains_key(name)
    }

    /// Активировать/деактивировать координатор
    pub fn set_coordinator_active(&mut self, name: &str, active: bool) -> Result<()> {
        match self.coordinators.get_mut(name) {
            Some(entry) => {
                entry.is_active = active;
                info!(
                    "Координатор '{}' {} активирован",
                    name,
                    if active { "" } else { "де" }
                );
                Ok(())
            }
            None => Err(anyhow::anyhow!("Координатор '{}' не найден", name)),
        }
    }

    /// Получить статус готовности всех координаторов
    pub async fn get_readiness_status(&self) -> ReadinessStatus {
        let total_coordinators = self.coordinators.len();
        let mut ready_coordinators = 0;
        let mut not_ready_coordinators = Vec::new();
        let mut critical_coordinators_ready = true;

        // Проверяем готовность всех координаторов параллельно
        let mut readiness_futures = Vec::new();

        for (name, entry) in &self.coordinators {
            if entry.is_active {
                let coordinator = Arc::clone(&entry.coordinator);
                let name_clone = name.clone();
                let is_critical = entry.metadata.is_critical;

                let future = tokio::spawn(async move {
                    let ready = coordinator.is_ready().await;
                    (name_clone, ready, is_critical)
                });
                readiness_futures.push(future);
            }
        }

        // Собираем результаты
        for future in readiness_futures {
            if let Ok((name, ready, is_critical)) = future.await {
                if ready {
                    ready_coordinators += 1;
                } else {
                    not_ready_coordinators.push(name);
                    if is_critical {
                        critical_coordinators_ready = false;
                    }
                }
            }
        }

        let readiness_percentage = if total_coordinators > 0 {
            (ready_coordinators as f64 / total_coordinators as f64) * 100.0
        } else {
            100.0
        };

        ReadinessStatus {
            total_coordinators,
            ready_coordinators,
            readiness_percentage,
            not_ready_coordinators,
            critical_coordinators_ready,
        }
    }

    /// Проверить готовность всех координаторов (простой метод для совместимости)
    pub async fn verify_all_ready(&self) -> bool {
        let status = self.get_readiness_status().await;
        status.ready_coordinators == status.total_coordinators && status.critical_coordinators_ready
    }
}

impl Default for CoordinatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder для создания конфигурированного реестра
pub struct CoordinatorRegistryBuilder {
    registry: CoordinatorRegistry,
}

impl CoordinatorRegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: CoordinatorRegistry::new(),
        }
    }

    pub fn with_coordinator(
        mut self,
        coordinator: Arc<dyn Coordinator>,
        metadata: CoordinatorMetadata,
    ) -> Result<Self> {
        let result = self.registry.register_coordinator(coordinator, metadata);

        if result.status == RegistrationStatus::Registered {
            Ok(self)
        } else {
            Err(anyhow::anyhow!("Ошибка регистрации: {}", result.message))
        }
    }

    pub fn build(self) -> CoordinatorRegistry {
        self.registry
    }
}

impl Default for CoordinatorRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockCoordinator {
        name: String,
        ready: Arc<AtomicBool>,
    }

    impl MockCoordinator {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                ready: Arc::new(AtomicBool::new(true)),
            }
        }

        fn set_ready(&self, ready: bool) {
            self.ready.store(ready, Ordering::Relaxed);
        }
    }

    #[async_trait]
    impl Coordinator for MockCoordinator {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            self.ready.load(Ordering::Relaxed)
        }

        async fn health_check(&self) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn metrics(&self) -> serde_json::Value {
            serde_json::json!({ "name": self.name })
        }
    }

    #[tokio::test]
    async fn test_basic_registration() {
        let mut registry = CoordinatorRegistry::new();

        let coordinator = Arc::new(MockCoordinator::new("test"));
        let metadata = CoordinatorMetadata::new("test".to_string()).with_priority(100);

        let result = registry.register_coordinator(coordinator, metadata);
        assert_eq!(result.status, RegistrationStatus::Registered);
        assert_eq!(registry.coordinator_count(), 1);
    }

    #[tokio::test]
    async fn test_dependency_management() {
        let mut registry = CoordinatorRegistry::new();

        // Регистрируем базовый координатор
        let base_coord = Arc::new(MockCoordinator::new("base"));
        let base_metadata = CoordinatorMetadata::new("base".to_string());
        registry.register_coordinator(base_coord, base_metadata);

        // Регистрируем зависимый координатор
        let dependent_coord = Arc::new(MockCoordinator::new("dependent"));
        let dependent_metadata =
            CoordinatorMetadata::new("dependent".to_string()).add_dependency("base".to_string());

        let result = registry.register_coordinator(dependent_coord, dependent_metadata);
        assert_eq!(result.status, RegistrationStatus::Registered);

        // Проверяем порядок инициализации
        let init_order = registry.get_initialization_order();
        assert_eq!(init_order[0].0, "base");
        assert_eq!(init_order[1].0, "dependent");
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let mut registry = CoordinatorRegistry::new();

        // Регистрируем первый координатор с зависимостью от не-существующего
        let coord_a = Arc::new(MockCoordinator::new("a"));
        let metadata_a = CoordinatorMetadata::new("a".to_string()).add_dependency("b".to_string());

        // Это должно не получиться из-за отсутствующей зависимости
        let result_a = registry.register_coordinator(coord_a, metadata_a);
        assert_eq!(result_a.status, RegistrationStatus::DependenciesNotMet);

        // Регистрируем сначала b
        let coord_b = Arc::new(MockCoordinator::new("b"));
        let metadata_b = CoordinatorMetadata::new("b".to_string());
        let result_b = registry.register_coordinator(coord_b, metadata_b);
        assert_eq!(result_b.status, RegistrationStatus::Registered);

        // Теперь регистрируем a
        let coord_a2 = Arc::new(MockCoordinator::new("a"));
        let metadata_a2 = CoordinatorMetadata::new("a".to_string()).add_dependency("b".to_string());
        let result_a2 = registry.register_coordinator(coord_a2, metadata_a2);
        assert_eq!(result_a2.status, RegistrationStatus::Registered);

        // Пытаемся добавить циклическую зависимость
        let coord_c = Arc::new(MockCoordinator::new("c"));
        let metadata_c = CoordinatorMetadata::new("c".to_string()).add_dependency("a".to_string());
        let result_c = registry.register_coordinator(coord_c, metadata_c);
        assert_eq!(result_c.status, RegistrationStatus::Registered);
    }

    #[tokio::test]
    async fn test_tag_grouping() {
        let mut registry = CoordinatorRegistry::new();

        let coordinator = Arc::new(MockCoordinator::new("test"));
        let metadata = CoordinatorMetadata::new("test".to_string())
            .with_tag("type".to_string(), "storage".to_string())
            .with_tag("env".to_string(), "production".to_string());

        registry.register_coordinator(coordinator, metadata);

        let storage_coords = registry.get_coordinators_by_tag("type", "storage");
        assert_eq!(storage_coords.len(), 1);

        let prod_coords = registry.get_coordinators_by_tag("env", "production");
        assert_eq!(prod_coords.len(), 1);
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let coord1 = Arc::new(MockCoordinator::new("first"));
        let metadata1 = CoordinatorMetadata::new("first".to_string());

        let coord2 = Arc::new(MockCoordinator::new("second"));
        let metadata2 =
            CoordinatorMetadata::new("second".to_string()).add_dependency("first".to_string());

        let registry = CoordinatorRegistryBuilder::new()
            .with_coordinator(coord1, metadata1)
            .unwrap()
            .with_coordinator(coord2, metadata2)
            .unwrap()
            .build();

        assert_eq!(registry.coordinator_count(), 2);

        let init_order = registry.get_initialization_order();
        assert_eq!(init_order[0].0, "first");
        assert_eq!(init_order[1].0, "second");
    }
}

/// Статус готовности системы координаторов
#[derive(Debug, Clone)]
pub struct ReadinessStatus {
    pub total_coordinators: usize,
    pub ready_coordinators: usize,
    pub readiness_percentage: f64,
    pub not_ready_coordinators: Vec<String>,
    pub critical_coordinators_ready: bool,
}
