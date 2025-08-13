use crate::types::{TaskState, TodoItem};
use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use petgraph::algo::{has_path_connecting, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Узел графа с состоянием задачи
#[derive(Debug, Clone)]
struct TaskNode {
    id: Uuid,
    state: TaskState,
    priority: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Оптимизированный граф зависимостей с конкурентным доступом
pub struct DependencyGraphV2 {
    // Основной граф
    graph: Arc<RwLock<DiGraph<TaskNode, ()>>>,
    // Быстрый поиск узлов по ID
    node_map: Arc<DashMap<Uuid, NodeIndex>>,
    // Кэш готовых задач
    ready_cache: Arc<DashMap<Uuid, bool>>,
    // Кэш путей для быстрой проверки циклов
    path_cache: Arc<DashMap<(Uuid, Uuid), bool>>,
}

impl Default for DependencyGraphV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraphV2 {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(DiGraph::new())),
            node_map: Arc::new(DashMap::new()),
            ready_cache: Arc::new(DashMap::new()),
            path_cache: Arc::new(DashMap::new()),
        }
    }

    /// Загрузить граф из списка задач
    pub fn load_from_tasks(&self, tasks: Vec<TodoItem>) -> Result<()> {
        let mut graph = self.graph.write();
        let node_map = &self.node_map;

        // Очищаем старые данные
        graph.clear();
        node_map.clear();
        self.ready_cache.clear();
        self.path_cache.clear();

        // Создаем узлы
        for task in &tasks {
            let node = TaskNode {
                id: task.id,
                state: task.state,
                priority: task.priority as i32,
                created_at: task.created_at,
            };
            let idx = graph.add_node(node);
            node_map.insert(task.id, idx);
        }

        // Создаем рёбра
        for task in &tasks {
            if let Some(task_idx) = node_map.get(&task.id) {
                for dep_id in &task.depends_on {
                    if let Some(dep_idx) = node_map.get(dep_id) {
                        // dep -> task (task зависит от dep)
                        graph.add_edge(*dep_idx.value(), *task_idx.value(), ());
                    }
                }
            }
        }

        Ok(())
    }

    /// Добавить или обновить задачу
    pub fn upsert_task(&self, task: &TodoItem) -> Result<()> {
        let mut graph = self.graph.write();

        // Инвалидируем кэши
        self.ready_cache.remove(&task.id);
        self.invalidate_path_cache_for(&task.id);

        let node = TaskNode {
            id: task.id,
            state: task.state,
            priority: task.priority as i32,
            created_at: task.created_at,
        };

        // Обновляем или создаем узел
        if let Some(idx) = self.node_map.get(&task.id) {
            graph[*idx] = node;
        } else {
            let idx = graph.add_node(node);
            self.node_map.insert(task.id, idx);
        }

        // Обновляем зависимости
        if let Some(task_idx) = self.node_map.get(&task.id).map(|e| *e.value()) {
            // Удаляем старые входящие рёбра
            let incoming: Vec<_> = graph
                .neighbors_directed(task_idx, Direction::Incoming)
                .collect();
            for neighbor in incoming {
                if let Some(edge) = graph.find_edge(neighbor, task_idx) {
                    graph.remove_edge(edge);
                }
            }

            // Добавляем новые зависимости
            for dep_id in &task.depends_on {
                // Создаем узел зависимости если его нет
                let dep_idx = if let Some(idx) = self.node_map.get(dep_id) {
                    *idx.value()
                } else {
                    let dep_node = TaskNode {
                        id: *dep_id,
                        state: TaskState::Planned,
                        priority: 0,
                        created_at: chrono::Utc::now(),
                    };
                    let idx = graph.add_node(dep_node);
                    self.node_map.insert(*dep_id, idx);
                    idx
                };

                graph.add_edge(dep_idx, task_idx, ());
            }
        }

        Ok(())
    }

    /// Обновить состояние задачи
    pub fn update_state(&self, task_id: &Uuid, new_state: TaskState) -> Result<()> {
        let mut graph = self.graph.write();

        if let Some(idx) = self.node_map.get(task_id) {
            graph[*idx].state = new_state;

            // Инвалидируем кэш готовности для всех зависимых задач
            let dependents: Vec<_> = graph
                .neighbors_directed(*idx, Direction::Outgoing)
                .map(|n| graph[n].id)
                .collect();

            drop(graph);

            for dep_id in dependents {
                self.ready_cache.remove(&dep_id);
            }
        }

        Ok(())
    }

    /// Проверить готовность задачи (с кэшированием)
    pub fn is_ready(&self, task_id: &Uuid) -> Result<bool> {
        // Проверяем кэш
        if let Some(cached) = self.ready_cache.get(task_id) {
            return Ok(*cached);
        }

        let graph = self.graph.read();

        if let Some(idx) = self.node_map.get(task_id) {
            let node = &graph[*idx];

            // Задача не может быть готова если она уже выполнена или провалена
            if matches!(
                node.state,
                TaskState::Done | TaskState::Failed | TaskState::Cancelled
            ) {
                self.ready_cache.insert(*task_id, false);
                return Ok(false);
            }

            // Проверяем все зависимости
            let is_ready = graph
                .neighbors_directed(*idx, Direction::Incoming)
                .all(|dep_idx| graph[dep_idx].state == TaskState::Done);

            // Кэшируем результат
            self.ready_cache.insert(*task_id, is_ready);
            Ok(is_ready)
        } else {
            Ok(true) // Задачи нет в графе - считаем готовой
        }
    }

    /// Получить готовые задачи отсортированные по приоритету
    pub fn get_ready_tasks(&self, limit: usize) -> Result<Vec<Uuid>> {
        let graph = self.graph.read();

        let mut ready_tasks: Vec<(Uuid, &TaskNode)> = Vec::new();

        for idx in graph.node_indices() {
            let node = &graph[idx];

            // Пропускаем не Ready состояния
            if node.state != TaskState::Ready {
                continue;
            }

            // Проверяем готовность
            if self.is_ready(&node.id)? {
                ready_tasks.push((node.id, node));
            }
        }

        // Сортируем по приоритету и времени создания
        ready_tasks.sort_by(|a, b| {
            b.1.priority
                .cmp(&a.1.priority)
                .then(a.1.created_at.cmp(&b.1.created_at))
        });

        Ok(ready_tasks
            .into_iter()
            .take(limit)
            .map(|(id, _)| id)
            .collect())
    }

    /// Проверить, создаст ли добавление зависимости цикл (с кэшированием)
    pub fn would_create_cycle(&self, from: &Uuid, to: &Uuid) -> Result<bool> {
        // Быстрая проверка - если задачи одинаковые
        if from == to {
            return Ok(true);
        }

        // Проверяем кэш путей
        let cache_key = (*to, *from); // Обратный порядок для проверки пути
        if let Some(has_path) = self.path_cache.get(&cache_key) {
            return Ok(*has_path);
        }

        let graph = self.graph.read();

        let from_idx = self.node_map.get(from).map(|e| *e.value());
        let to_idx = self.node_map.get(to).map(|e| *e.value());

        match (from_idx, to_idx) {
            (Some(f), Some(t)) => {
                // Проверяем существует ли путь от to к from
                let has_path = has_path_connecting(&*graph, t, f, None);

                // Кэшируем результат
                self.path_cache.insert(cache_key, has_path);

                Ok(has_path)
            }
            _ => Ok(false), // Если одной из задач нет - цикла быть не может
        }
    }

    /// Получить топологически отсортированный список задач
    pub fn topological_sort(&self) -> Result<Vec<Uuid>> {
        let graph = self.graph.read();

        match toposort(&*graph, None) {
            Ok(nodes) => Ok(nodes.into_iter().map(|idx| graph[idx].id).collect()),
            Err(_) => Err(anyhow::anyhow!("Dependency graph contains a cycle")),
        }
    }

    /// Получить все задачи, которые зависят от данной
    pub fn get_dependents(&self, task_id: &Uuid) -> Vec<Uuid> {
        let graph = self.graph.read();

        if let Some(idx) = self.node_map.get(task_id) {
            graph
                .neighbors_directed(*idx, Direction::Outgoing)
                .map(|n| graph[n].id)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Получить все задачи, от которых зависит данная
    pub fn get_dependencies(&self, task_id: &Uuid) -> Vec<Uuid> {
        let graph = self.graph.read();

        if let Some(idx) = self.node_map.get(task_id) {
            graph
                .neighbors_directed(*idx, Direction::Incoming)
                .map(|n| graph[n].id)
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Получить статистику графа
    pub fn stats(&self) -> GraphStats {
        let graph = self.graph.read();

        let total_nodes = graph.node_count();
        let total_edges = graph.edge_count();

        let mut state_counts = HashMap::new();
        for node in graph.node_weights() {
            *state_counts.entry(node.state).or_insert(0) += 1;
        }

        GraphStats {
            total_tasks: total_nodes,
            total_dependencies: total_edges,
            tasks_by_state: state_counts,
            cache_size: self.ready_cache.len() + self.path_cache.len(),
        }
    }

    /// Инвалидировать кэш путей для задачи
    fn invalidate_path_cache_for(&self, task_id: &Uuid) {
        // Удаляем все записи кэша где участвует эта задача
        let keys_to_remove: Vec<_> = self
            .path_cache
            .iter()
            .filter(|entry| entry.key().0 == *task_id || entry.key().1 == *task_id)
            .map(|entry| *entry.key())
            .collect();

        for key in keys_to_remove {
            self.path_cache.remove(&key);
        }
    }
}

/// Статистика графа
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_tasks: usize,
    pub total_dependencies: usize,
    pub tasks_by_state: HashMap<TaskState, usize>,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_access() {
        let graph = Arc::new(DependencyGraphV2::new());

        // Создаем задачи в разных потоках
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let g = graph.clone();
                std::thread::spawn(move || {
                    let task = TodoItem {
                        id: Uuid::new_v4(),
                        title: format!("Task {}", i),
                        state: TaskState::Ready,
                        priority: crate::Priority::Medium,
                        ..Default::default()
                    };
                    g.upsert_task(&task)
                        .expect("Operation failed - converted from unwrap()");
                })
            })
            .collect();

        for h in handles {
            h.join()
                .expect("Operation failed - converted from unwrap()");
        }

        let stats = graph.stats();
        assert_eq!(stats.total_tasks, 10);
    }
}
