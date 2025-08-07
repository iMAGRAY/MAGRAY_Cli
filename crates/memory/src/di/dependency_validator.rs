use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::{
    any::TypeId,
    collections::{HashMap, HashSet, VecDeque},
};
use tracing::{debug, warn};

use super::errors::ValidationError;
use super::traits::DependencyValidator;

/// Граф зависимостей для отслеживания связей между типами
/// Применяет принцип Single Responsibility (SRP)
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Отношения зависимостей: тип -> список типов, от которых он зависит
    dependencies: HashMap<TypeId, HashSet<TypeId>>,
    /// Обратные зависимости: тип -> список типов, которые от него зависят
    dependents: HashMap<TypeId, HashSet<TypeId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
        }
    }

    /// Добавить зависимость: dependent зависит от dependency
    pub fn add_dependency(&mut self, dependent: TypeId, dependency: TypeId) {
        // Добавляем прямую зависимость
        self.dependencies
            .entry(dependent)
            .or_insert_with(HashSet::new)
            .insert(dependency);

        // Добавляем обратную зависимость
        self.dependents
            .entry(dependency)
            .or_insert_with(HashSet::new)
            .insert(dependent);

        debug!("Added dependency: {:?} -> {:?}", dependent, dependency);
    }

    /// Найти все циклы в графе зависимостей
    pub fn find_cycles(&self) -> Vec<Vec<TypeId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &node in self.dependencies.keys() {
            if !visited.contains(&node) {
                self.dfs_find_cycles(node, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    /// DFS поиск циклов
    fn dfs_find_cycles(
        &self,
        node: TypeId,
        visited: &mut HashSet<TypeId>,
        rec_stack: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
        cycles: &mut Vec<Vec<TypeId>>,
    ) {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(deps) = self.dependencies.get(&node) {
            for &neighbor in deps {
                if !visited.contains(&neighbor) {
                    self.dfs_find_cycles(neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(&neighbor) {
                    // Найден цикл - извлекаем путь от neighbor до конца
                    if let Some(cycle_start) = path.iter().position(|&x| x == neighbor) {
                        let cycle: Vec<TypeId> = path[cycle_start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        rec_stack.remove(&node);
        path.pop();
    }

    /// Получить все зависимости для типа
    pub fn get_dependencies(&self, type_id: TypeId) -> Option<&HashSet<TypeId>> {
        self.dependencies.get(&type_id)
    }

    /// Получить все типы, зависящие от данного
    pub fn get_dependents(&self, type_id: TypeId) -> Option<&HashSet<TypeId>> {
        self.dependents.get(&type_id)
    }

    /// Проверить, есть ли зависимость между типами
    pub fn has_dependency(&self, dependent: TypeId, dependency: TypeId) -> bool {
        self.dependencies
            .get(&dependent)
            .map(|deps| deps.contains(&dependency))
            .unwrap_or(false)
    }

    /// Получить топологический порядок типов (если граф ациклический)
    pub fn topological_sort(&self) -> Result<Vec<TypeId>> {
        let mut result = Vec::new();
        let mut in_degree: HashMap<TypeId, usize> = HashMap::new();

        // Вычисляем входящую степень для каждого узла
        for node in self.dependencies.keys() {
            in_degree.entry(*node).or_insert(0);
        }
        for node in self.dependents.keys() {
            in_degree.entry(*node).or_insert(0);
        }

        for (dependent, deps) in &self.dependencies {
            for &_dependency in deps {
                if let Some(degree) = in_degree.get_mut(&dependent) {
                    *degree += 1;
                } else {
                    // This should never happen if all nodes were properly initialized
                    return Err(ValidationError::GraphCorrupted {
                        details: format!("Dependency {:?} not found in in_degree map", dependent),
                    }
                    .into());
                }
            }
        }

        // Добавляем узлы с нулевой входящей степенью в очередь
        let mut queue: VecDeque<TypeId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&node, _)| node)
            .collect();

        // Обрабатываем узлы
        while let Some(node) = queue.pop_front() {
            result.push(node);

            // Уменьшаем входящую степень соседних узлов
            if let Some(dependents) = self.dependents.get(&node) {
                for &dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(&dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent);
                        }
                    } else {
                        return Err(ValidationError::GraphCorrupted {
                            details: format!(
                                "Dependent {:?} not found in in_degree map",
                                dependent
                            ),
                        }
                        .into());
                    }
                }
            }
        }

        // Если не все узлы обработаны, значит есть циклы
        if result.len() != in_degree.len() {
            Err(anyhow!(
                "Graph contains cycles, topological sort impossible"
            ))
        } else {
            Ok(result)
        }
    }

    /// Очистить граф
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.dependents.clear();
        debug!("Dependency graph cleared");
    }

    /// Получить статистику графа
    pub fn stats(&self) -> DependencyGraphStats {
        let total_nodes = {
            let mut all_nodes: HashSet<TypeId> = HashSet::new();
            all_nodes.extend(self.dependencies.keys());
            all_nodes.extend(self.dependents.keys());
            all_nodes.len()
        };

        let total_edges = self.dependencies.values().map(|deps| deps.len()).sum();

        let max_dependencies = self
            .dependencies
            .values()
            .map(|deps| deps.len())
            .max()
            .unwrap_or(0);

        let max_dependents = self
            .dependents
            .values()
            .map(|deps| deps.len())
            .max()
            .unwrap_or(0);

        DependencyGraphStats {
            total_nodes,
            total_edges,
            max_dependencies,
            max_dependents,
        }
    }
}

/// Статистика графа зависимостей
#[derive(Debug, Clone)]
pub struct DependencyGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub max_dependencies: usize,
    pub max_dependents: usize,
}

/// Реализация валидатора зависимостей
/// Применяет принцип Single Responsibility (SRP)
pub struct DependencyValidatorImpl {
    /// Граф зависимостей
    graph: RwLock<DependencyGraph>,
    /// Найденные циклы (кэшированные)
    cached_cycles: RwLock<Option<Vec<Vec<TypeId>>>>,
}

impl DependencyValidatorImpl {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(DependencyGraph::new()),
            cached_cycles: RwLock::new(None),
        }
    }

    /// Получить статистику графа зависимостей
    pub fn get_graph_stats(&self) -> DependencyGraphStats {
        let graph = self.graph.read();
        graph.stats()
    }

    /// Получить топологический порядок зависимостей
    pub fn get_topological_order(&self) -> Result<Vec<TypeId>> {
        let graph = self.graph.read();
        graph.topological_sort()
    }

    /// Проверить, есть ли зависимость между типами
    pub fn has_dependency(&self, dependent: TypeId, dependency: TypeId) -> bool {
        let graph = self.graph.read();
        graph.has_dependency(dependent, dependency)
    }

    /// Получить все зависимости для типа
    pub fn get_dependencies(&self, type_id: TypeId) -> Vec<TypeId> {
        let graph = self.graph.read();
        graph
            .get_dependencies(type_id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Получить все типы, зависящие от данного
    pub fn get_dependents(&self, type_id: TypeId) -> Vec<TypeId> {
        let graph = self.graph.read();
        graph
            .get_dependents(type_id)
            .map(|deps| deps.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Инвалидировать кэш циклов
    fn invalidate_cycle_cache(&self) {
        let mut cached_cycles = self.cached_cycles.write();
        *cached_cycles = None;
    }
}

impl Default for DependencyValidatorImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyValidator for DependencyValidatorImpl {
    fn add_dependency(&self, dependent: TypeId, dependency: TypeId) -> Result<()> {
        {
            let mut graph = self.graph.write();
            graph.add_dependency(dependent, dependency);
        }

        // Инвалидируем кэш циклов
        self.invalidate_cycle_cache();

        debug!("Added dependency: {:?} -> {:?}", dependent, dependency);
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        let cycles = self.get_cycles();

        if !cycles.is_empty() {
            let cycle_info = cycles
                .iter()
                .map(|cycle| format!("{:?}", cycle))
                .collect::<Vec<_>>()
                .join(", ");

            warn!(
                "Found {} circular dependencies: {}",
                cycles.len(),
                cycle_info
            );
            Err(anyhow!("Circular dependencies detected: {}", cycle_info))
        } else {
            debug!("Dependency validation passed - no cycles found");
            Ok(())
        }
    }

    fn get_cycles(&self) -> Vec<Vec<TypeId>> {
        // Проверяем кэш
        {
            let cached_cycles = self.cached_cycles.read();
            if let Some(ref cycles) = *cached_cycles {
                return cycles.clone();
            }
        }

        // Вычисляем циклы
        let cycles = {
            let graph = self.graph.read();
            graph.find_cycles()
        };

        // Кэшируем результат
        {
            let mut cached_cycles = self.cached_cycles.write();
            *cached_cycles = Some(cycles.clone());
        }

        cycles
    }

    fn clear(&self) {
        {
            let mut graph = self.graph.write();
            graph.clear();
        }

        // Очищаем кэш
        {
            let mut cached_cycles = self.cached_cycles.write();
            *cached_cycles = None;
        }

        debug!("Dependency validator cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph_basic() {
        let mut graph = DependencyGraph::new();

        let type_a = TypeId::of::<i32>();
        let type_b = TypeId::of::<String>();

        graph.add_dependency(type_a, type_b);

        assert!(graph.has_dependency(type_a, type_b));
        assert!(!graph.has_dependency(type_b, type_a));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let type_a = TypeId::of::<i32>();
        let type_b = TypeId::of::<String>();
        let type_c = TypeId::of::<f64>();

        // Создаём цикл: A -> B -> C -> A
        graph.add_dependency(type_a, type_b);
        graph.add_dependency(type_b, type_c);
        graph.add_dependency(type_c, type_a);

        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());

        // Проверяем, что найден цикл нужной длины
        assert!(cycles.iter().any(|cycle| cycle.len() >= 3));
    }

    #[test]
    fn test_topological_sort_acyclic() -> Result<()> {
        let mut graph = DependencyGraph::new();

        let type_a = TypeId::of::<i32>();
        let type_b = TypeId::of::<String>();
        let type_c = TypeId::of::<f64>();

        // Создаём ациклический граф: C -> B -> A
        graph.add_dependency(type_b, type_a);
        graph.add_dependency(type_c, type_b);

        let sorted = graph.topological_sort()?;

        // A должна быть перед B, B должна быть перед C в результате
        let pos_a = sorted
            .iter()
            .position(|&x| x == type_a)
            .expect("Type A should be present in sorted dependencies");
        let pos_b = sorted
            .iter()
            .position(|&x| x == type_b)
            .expect("Type B should be present in sorted dependencies");
        let pos_c = sorted
            .iter()
            .position(|&x| x == type_c)
            .expect("Type C should be present in sorted dependencies");

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);

        Ok(())
    }

    #[test]
    fn test_dependency_validator() -> Result<()> {
        let validator = DependencyValidatorImpl::new();

        let type_a = TypeId::of::<i32>();
        let type_b = TypeId::of::<String>();

        // Добавляем обычную зависимость
        validator.add_dependency(type_a, type_b)?;

        // Валидация должна пройти
        assert!(validator.validate().is_ok());

        // Добавляем циклическую зависимость
        validator.add_dependency(type_b, type_a)?;

        // Теперь валидация должна провалиться
        assert!(validator.validate().is_err());

        // Проверяем, что циклы найдены
        let cycles = validator.get_cycles();
        assert!(!cycles.is_empty());

        Ok(())
    }

    #[test]
    fn test_validator_cache() -> Result<()> {
        let validator = DependencyValidatorImpl::new();

        let type_a = TypeId::of::<i32>();
        let type_b = TypeId::of::<String>();

        // Создаём цикл
        validator.add_dependency(type_a, type_b)?;
        validator.add_dependency(type_b, type_a)?;

        // Первый вызов должен вычислить циклы
        let cycles1 = validator.get_cycles();
        assert!(!cycles1.is_empty());

        // Второй вызов должен использовать кэш
        let cycles2 = validator.get_cycles();
        assert_eq!(cycles1.len(), cycles2.len());

        // После добавления новой зависимости кэш должен инвалидироваться
        let type_c = TypeId::of::<f64>();
        validator.add_dependency(type_c, type_a)?;

        let cycles3 = validator.get_cycles();
        // Теперь может быть больше циклов
        assert!(!cycles3.is_empty());

        Ok(())
    }
}
