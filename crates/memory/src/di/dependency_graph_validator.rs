//! Dependency Graph Validator - валидация циклических зависимостей
//!
//! Отделен от unified_container.rs для следования Single Responsibility Principle.
//! Отвечает ТОЛЬКО за анализ и валидацию графа зависимостей.

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};
use tracing::{debug, error, warn};

/// Граф зависимостей между типами
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Граф зависимостей: от типа -> к типам
    graph: HashMap<TypeId, Vec<TypeId>>,
    /// Обратный граф: к типу <- от типов (для оптимизации)
    reverse_graph: HashMap<TypeId, Vec<TypeId>>,
    /// Кэш имен типов для отладки
    type_names: HashMap<TypeId, String>,
}

impl DependencyGraph {
    /// Создать новый граф зависимостей
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавить зависимость между типами
    pub fn add_dependency(
        &mut self,
        from: TypeId,
        to: TypeId,
        from_name: Option<String>,
        to_name: Option<String>,
    ) {
        // Добавляем в прямой граф
        self.graph.entry(from).or_default().push(to);

        // Добавляем в обратный граф
        self.reverse_graph
            .entry(to)
            .or_default()
            .push(from);

        // Сохраняем имена для отладки
        if let Some(name) = from_name {
            self.type_names.insert(from, name);
        }
        if let Some(name) = to_name {
            self.type_names.insert(to, name);
        }

        debug!(
            "🔗 Добавлена зависимость: {} -> {}",
            self.get_type_name(from),
            self.get_type_name(to)
        );
    }

    /// Получить все зависимости для типа
    pub fn get_dependencies(&self, type_id: TypeId) -> Vec<TypeId> {
        self.graph.get(&type_id).cloned().unwrap_or_default()
    }

    /// Получить все типы, которые зависят от данного типа
    pub fn get_dependents(&self, type_id: TypeId) -> Vec<TypeId> {
        self.reverse_graph
            .get(&type_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Проверить есть ли зависимость между типами
    pub fn has_dependency(&self, from: TypeId, to: TypeId) -> bool {
        self.graph
            .get(&from)
            .map(|deps| deps.contains(&to))
            .unwrap_or(false)
    }

    /// Получить все типы в графе
    pub fn get_all_types(&self) -> HashSet<TypeId> {
        let mut all_types = HashSet::new();

        for (&from, dependencies) in &self.graph {
            all_types.insert(from);
            for &to in dependencies {
                all_types.insert(to);
            }
        }

        all_types
    }

    /// Получить количество типов в графе
    pub fn type_count(&self) -> usize {
        self.get_all_types().len()
    }

    /// Получить количество зависимостей
    pub fn dependency_count(&self) -> usize {
        self.graph.values().map(|deps| deps.len()).sum()
    }

    /// Получить имя типа для отладки
    pub fn get_type_name(&self, type_id: TypeId) -> String {
        self.type_names
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| format!("Unknown({:?})", type_id))
    }

    /// Очистить граф
    pub fn clear(&mut self) {
        self.graph.clear();
        self.reverse_graph.clear();
        self.type_names.clear();
        debug!("🧹 Граф зависимостей очищен");
    }

    /// Создать копию графа для анализа
    pub fn clone_for_analysis(&self) -> HashMap<TypeId, Vec<TypeId>> {
        self.graph.clone()
    }

    /// Получить статистику графа
    pub fn get_stats(&self) -> DependencyGraphStats {
        let all_types = self.get_all_types();
        let type_count = all_types.len();
        let dependency_count = self.dependency_count();

        // Вычисляем средний fan-out (среднее количество исходящих зависимостей)
        let avg_fan_out = if type_count > 0 {
            dependency_count as f64 / type_count as f64
        } else {
            0.0
        };

        // Находим типы с максимальными входящими и исходящими зависимостями
        let mut max_outgoing = 0;
        let mut max_incoming = 0;
        let mut max_outgoing_type = None;
        let mut max_incoming_type = None;

        for &type_id in &all_types {
            let outgoing = self.get_dependencies(type_id).len();
            let incoming = self.get_dependents(type_id).len();

            if outgoing > max_outgoing {
                max_outgoing = outgoing;
                max_outgoing_type = Some(type_id);
            }

            if incoming > max_incoming {
                max_incoming = incoming;
                max_incoming_type = Some(type_id);
            }
        }

        DependencyGraphStats {
            total_types: type_count,
            total_dependencies: dependency_count,
            average_fan_out: avg_fan_out,
            max_outgoing_dependencies: max_outgoing,
            max_incoming_dependencies: max_incoming,
            most_dependent_type: max_outgoing_type.map(|t| self.get_type_name(t)),
            most_depended_upon_type: max_incoming_type.map(|t| self.get_type_name(t)),
        }
    }
}

/// Статистика графа зависимостей
#[derive(Debug, Clone)]
pub struct DependencyGraphStats {
    pub total_types: usize,
    pub total_dependencies: usize,
    pub average_fan_out: f64,
    pub max_outgoing_dependencies: usize,
    pub max_incoming_dependencies: usize,
    pub most_dependent_type: Option<String>,
    pub most_depended_upon_type: Option<String>,
}

/// Validator для обнаружения циклических зависимостей
pub struct DependencyGraphValidator {
    /// Граф зависимостей
    graph: RwLock<DependencyGraph>,
    /// Включена ли валидация
    enabled: bool,
}

impl DependencyGraphValidator {
    /// Создать новый validator
    pub fn new(enabled: bool) -> Self {
        Self {
            graph: RwLock::new(DependencyGraph::new()),
            enabled,
        }
    }

    /// Добавить зависимость между типами
    pub fn add_dependency<TFrom, TTo>(&self, from_name: Option<String>, to_name: Option<String>)
    where
        TFrom: 'static,
        TTo: 'static,
    {
        if !self.enabled {
            return;
        }

        let from_id = TypeId::of::<TFrom>();
        let to_id = TypeId::of::<TTo>();

        let mut graph = self.graph.write();
        graph.add_dependency(from_id, to_id, from_name, to_name);
    }

    /// Добавить зависимость по TypeId
    pub fn add_dependency_by_id(
        &self,
        from: TypeId,
        to: TypeId,
        from_name: String,
        to_name: String,
    ) {
        if !self.enabled {
            return;
        }

        let mut graph = self.graph.write();
        graph.add_dependency(from, to, Some(from_name), Some(to_name));
    }

    /// Валидировать отсутствие циклических зависимостей
    pub fn validate_no_cycles(&self) -> Result<()> {
        if !self.enabled {
            debug!("🔍 Валидация зависимостей отключена");
            return Ok(());
        }

        debug!("🔍 Валидация циклических зависимостей...");

        let graph = self.graph.read();
        let graph_clone = graph.clone_for_analysis();
        let cycles = self.detect_cycles_internal(&graph_clone, &graph);

        if !cycles.is_empty() {
            let mut error_msg = String::from("Обнаружены циклические зависимости:\n");

            for cycle in &cycles {
                let cycle_names: Vec<String> = cycle
                    .iter()
                    .map(|&type_id| graph.get_type_name(type_id))
                    .collect();
                error_msg.push_str(&format!("  🔄 {}\n", cycle_names.join(" -> ")));
            }

            error!("❌ {}", error_msg);
            return Err(anyhow!(error_msg));
        }

        debug!("✅ Валидация циклических зависимостей прошла успешно");
        Ok(())
    }

    /// Получить все обнаруженные циклы
    pub fn get_cycles(&self) -> Vec<Vec<TypeId>> {
        if !self.enabled {
            return Vec::new();
        }

        let graph = self.graph.read();
        let graph_clone = graph.clone_for_analysis();
        self.detect_cycles_internal(&graph_clone, &graph)
    }

    /// Проверить есть ли циклы в графе
    pub fn has_cycles(&self) -> bool {
        !self.get_cycles().is_empty()
    }

    /// Получить статистику графа зависимостей
    pub fn get_graph_stats(&self) -> DependencyGraphStats {
        let graph = self.graph.read();
        graph.get_stats()
    }

    /// Очистить граф зависимостей
    pub fn clear(&self) {
        let mut graph = self.graph.write();
        graph.clear();
    }

    /// Проверить валидность всех зарегистрированных типов
    pub fn validate_all_resolvable(&self, registered_types: &[TypeId]) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("🔍 Валидация разрешимости всех типов...");

        let graph = self.graph.read();
        let mut unresolvable = Vec::new();

        for &type_id in registered_types {
            if !self.can_resolve_type(type_id, registered_types, &graph) {
                unresolvable.push(graph.get_type_name(type_id));
            }
        }

        if !unresolvable.is_empty() {
            let error_msg = format!(
                "Следующие типы не могут быть разрешены из-за отсутствующих зависимостей:\n  - {}",
                unresolvable.join("\n  - ")
            );
            error!("❌ {}", error_msg);
            return Err(anyhow!(error_msg));
        }

        debug!("✅ Все типы могут быть разрешены");
        Ok(())
    }

    /// Получить отчет о графе зависимостей
    pub fn get_dependency_report(&self) -> String {
        if !self.enabled {
            return "Валидация зависимостей отключена".to_string();
        }

        let stats = self.get_graph_stats();
        let cycles = self.get_cycles();

        format!(
            "=== Dependency Graph Report ===\n\
             Total types: {}\n\
             Total dependencies: {}\n\
             Average fan-out: {:.2}\n\
             Max outgoing dependencies: {}\n\
             Max incoming dependencies: {}\n\
             Most dependent type: {}\n\
             Most depended upon type: {}\n\
             Circular dependencies found: {}\n\
             {}=============================",
            stats.total_types,
            stats.total_dependencies,
            stats.average_fan_out,
            stats.max_outgoing_dependencies,
            stats.max_incoming_dependencies,
            stats.most_dependent_type.as_deref().unwrap_or("None"),
            stats.most_depended_upon_type.as_deref().unwrap_or("None"),
            cycles.len(),
            if !cycles.is_empty() {
                let cycle_descriptions: Vec<String> = cycles
                    .iter()
                    .enumerate()
                    .map(|(i, cycle)| {
                        let graph = self.graph.read();
                        let names: Vec<String> = cycle
                            .iter()
                            .map(|&type_id| graph.get_type_name(type_id))
                            .collect();
                        format!("Cycle {}: {}\n", i + 1, names.join(" -> "))
                    })
                    .collect();
                cycle_descriptions.join("")
            } else {
                String::new()
            }
        )
    }

    // === PRIVATE HELPER METHODS ===

    /// Обнаружение циклов с помощью DFS
    fn detect_cycles_internal(
        &self,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        dependency_graph: &DependencyGraph,
    ) -> Vec<Vec<TypeId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut current_path = Vec::new();

        for &node in graph.keys() {
            if !visited.contains(&node) {
                self.dfs_cycle_detection(
                    node,
                    graph,
                    dependency_graph,
                    &mut visited,
                    &mut rec_stack,
                    &mut current_path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    #[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
    fn dfs_cycle_detection(
        &self,
        node: TypeId,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        dependency_graph: &DependencyGraph,
        visited: &mut HashSet<TypeId>,
        rec_stack: &mut HashSet<TypeId>,
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
                        dependency_graph,
                        visited,
                        rec_stack,
                        current_path,
                        cycles,
                    );
                } else if rec_stack.contains(&neighbor) {
                    // Найден цикл
                    if let Some(cycle_start) = current_path.iter().position(|&x| x == neighbor) {
                        let mut cycle = current_path[cycle_start..].to_vec();
                        cycle.push(neighbor); // Замыкаем цикл
                        let cycle_for_log = cycle.clone();
                        cycles.push(cycle);

                        debug!(
                            "🔄 Обнаружен цикл: {}",
                            cycle_for_log
                                .iter()
                                .map(|&t| dependency_graph.get_type_name(t))
                                .collect::<Vec<_>>()
                                .join(" -> ")
                        );
                    }
                }
            }
        }

        current_path.pop();
        rec_stack.remove(&node);
    }

    /// Проверить может ли тип быть разрешен
    fn can_resolve_type(
        &self,
        type_id: TypeId,
        registered_types: &[TypeId],
        dependency_graph: &DependencyGraph,
    ) -> bool {
        let dependencies = dependency_graph.get_dependencies(type_id);

        // Если нет зависимостей, тип может быть разрешен
        if dependencies.is_empty() {
            return true;
        }

        // Все зависимости должны быть зарегистрированы
        for &dep_id in &dependencies {
            if !registered_types.contains(&dep_id) {
                warn!(
                    "❌ Тип {} зависит от незарегистрированного типа {}",
                    dependency_graph.get_type_name(type_id),
                    dependency_graph.get_type_name(dep_id)
                );
                return false;
            }
        }

        true
    }
}

impl Default for DependencyGraphValidator {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ServiceA;
    struct ServiceB;
    struct ServiceC;

    #[test]
    fn test_dependency_graph_basic() {
        let validator = DependencyGraphValidator::new(true);

        // Добавляем зависимость A -> B
        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );

        let stats = validator.get_graph_stats();
        assert_eq!(stats.total_types, 2);
        assert_eq!(stats.total_dependencies, 1);
    }

    #[test]
    fn test_no_cycles() {
        let validator = DependencyGraphValidator::new(true);

        // Linear dependency: A -> B -> C
        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );
        validator.add_dependency::<ServiceB, ServiceC>(
            Some("ServiceB".to_string()),
            Some("ServiceC".to_string()),
        );

        assert!(validator.validate_no_cycles().is_ok());
        assert!(!validator.has_cycles());
    }

    #[test]
    fn test_cycle_detection() {
        let validator = DependencyGraphValidator::new(true);

        // Создаем цикл: A -> B -> C -> A
        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );
        validator.add_dependency::<ServiceB, ServiceC>(
            Some("ServiceB".to_string()),
            Some("ServiceC".to_string()),
        );
        validator.add_dependency::<ServiceC, ServiceA>(
            Some("ServiceC".to_string()),
            Some("ServiceA".to_string()),
        );

        assert!(validator.validate_no_cycles().is_err());
        assert!(validator.has_cycles());

        let cycles = validator.get_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_validator_disabled() {
        let validator = DependencyGraphValidator::new(false);

        // Даже при наличии циклов валидация должна проходить
        validator.add_dependency::<ServiceA, ServiceB>(None, None);
        validator.add_dependency::<ServiceB, ServiceA>(None, None);

        assert!(validator.validate_no_cycles().is_ok());
        assert!(!validator.has_cycles());
    }

    #[test]
    fn test_graph_stats() {
        let validator = DependencyGraphValidator::new(true);

        // A зависит от B и C, B зависит от C
        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );
        validator.add_dependency::<ServiceA, ServiceC>(
            Some("ServiceA".to_string()),
            Some("ServiceC".to_string()),
        );
        validator.add_dependency::<ServiceB, ServiceC>(
            Some("ServiceB".to_string()),
            Some("ServiceC".to_string()),
        );

        let stats = validator.get_graph_stats();
        assert_eq!(stats.total_types, 3);
        assert_eq!(stats.total_dependencies, 3);
        assert!(stats.average_fan_out > 0.0);
        assert_eq!(stats.max_outgoing_dependencies, 2); // ServiceA
        assert_eq!(stats.max_incoming_dependencies, 2); // ServiceC
    }

    #[test]
    fn test_dependency_report() {
        let validator = DependencyGraphValidator::new(true);

        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );

        let report = validator.get_dependency_report();
        assert!(report.contains("Total types: 2"));
        assert!(report.contains("Total dependencies: 1"));
        assert!(report.contains("Circular dependencies found: 0"));
    }

    #[test]
    fn test_clear_graph() {
        let validator = DependencyGraphValidator::new(true);

        validator.add_dependency::<ServiceA, ServiceB>(
            Some("ServiceA".to_string()),
            Some("ServiceB".to_string()),
        );

        let stats_before = validator.get_graph_stats();
        assert_eq!(stats_before.total_types, 2);

        validator.clear();

        let stats_after = validator.get_graph_stats();
        assert_eq!(stats_after.total_types, 0);
        assert_eq!(stats_after.total_dependencies, 0);
    }
}
