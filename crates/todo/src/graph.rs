use crate::types::{TodoItem, TaskState};
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use petgraph::Direction;
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Граф зависимостей между задачами
pub struct DependencyGraph {
    graph: RwLock<DiGraph<Uuid, ()>>,
    node_map: RwLock<HashMap<Uuid, NodeIndex>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: RwLock::new(DiGraph::new()),
            node_map: RwLock::new(HashMap::new()),
        }
    }
    
    /// Добавить задачу в граф
    pub fn add_task(&self, task: &TodoItem) -> Result<()> {
        let mut graph = self.graph.write().unwrap();
        let mut node_map = self.node_map.write().unwrap();
        
        // Добавляем узел если его еще нет
        if !node_map.contains_key(&task.id) {
            let node = graph.add_node(task.id);
            node_map.insert(task.id, node);
        }
        
        // Добавляем зависимости
        for dep_id in &task.depends_on {
            // Создаем узел для зависимости если его нет
            if !node_map.contains_key(dep_id) {
                let dep_node = graph.add_node(*dep_id);
                node_map.insert(*dep_id, dep_node);
            }
            
            let from = node_map[dep_id];
            let to = node_map[&task.id];
            
            // Добавляем ребро от зависимости к задаче
            // (задача зависит от dep_id, поэтому dep_id -> task)
            graph.update_edge(from, to, ());
        }
        
        Ok(())
    }
    
    /// Проверить, готова ли задача к выполнению
    pub fn is_ready(&self, task_id: &Uuid) -> Result<bool> {
        let graph = self.graph.read().unwrap();
        let node_map = self.node_map.read().unwrap();
        
        if let Some(&node) = node_map.get(task_id) {
            // Проверяем все входящие зависимости
            for neighbor in graph.neighbors_directed(node, Direction::Incoming) {
                let _dep_id = graph[neighbor];
                // Зависимость должна быть выполнена
                // TODO: Нужно хранить состояния в графе или передавать извне
                // Пока считаем что все зависимости выполнены
            }
            Ok(true)
        } else {
            Ok(true) // Если задачи нет в графе, она готова
        }
    }
    
    /// Проверить, создаст ли добавление зависимости цикл
    pub fn would_create_cycle(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<bool> {
        let mut graph = self.graph.write().unwrap();
        let node_map = self.node_map.read().unwrap();
        
        // Получаем или создаем узлы
        let task_node = if let Some(&node) = node_map.get(task_id) {
            node
        } else {
            return Ok(false); // Новый узел не может создать цикл
        };
        
        let dep_node = if let Some(&node) = node_map.get(depends_on) {
            node
        } else {
            return Ok(false); // Новый узел не может создать цикл
        };
        
        // Временно добавляем ребро
        graph.add_edge(dep_node, task_node, ());
        
        // Проверяем на циклы
        let has_cycle = toposort(&*graph, None).is_err();
        
        // Удаляем временное ребро
        if let Some(edge) = graph.find_edge(dep_node, task_node) {
            graph.remove_edge(edge);
        }
        
        Ok(has_cycle)
    }
    
    /// Обновить зависимости задачи
    pub fn update_dependencies(&self, task: &TodoItem) -> Result<()> {
        let mut graph = self.graph.write().unwrap();
        let mut node_map = self.node_map.write().unwrap();
        
        // Получаем или создаем узел задачи
        let task_node = if let Some(&node) = node_map.get(&task.id) {
            node
        } else {
            let node = graph.add_node(task.id);
            node_map.insert(task.id, node);
            node
        };
        
        // Удаляем старые входящие ребра
        let incoming: Vec<_> = graph
            .neighbors_directed(task_node, Direction::Incoming)
            .collect();
        for neighbor in incoming {
            if let Some(edge) = graph.find_edge(neighbor, task_node) {
                graph.remove_edge(edge);
            }
        }
        
        // Добавляем новые зависимости
        for dep_id in &task.depends_on {
            let dep_node = if let Some(&node) = node_map.get(dep_id) {
                node
            } else {
                let node = graph.add_node(*dep_id);
                node_map.insert(*dep_id, node);
                node
            };
            
            graph.add_edge(dep_node, task_node, ());
        }
        
        Ok(())
    }
    
    /// Обновить состояние задачи (для будущего использования)
    pub fn update_state(&self, _task_id: &Uuid, _new_state: TaskState) -> Result<()> {
        // TODO: Хранить состояния в графе если понадобится
        Ok(())
    }
    
    /// Получить количество задач в графе
    pub fn task_count(&self) -> usize {
        self.node_map.read().unwrap().len()
    }
    
    /// Получить топологически отсортированный список задач
    pub fn topological_sort(&self) -> Result<Vec<Uuid>> {
        let graph = self.graph.read().unwrap();
        
        match toposort(&*graph, None) {
            Ok(nodes) => {
                Ok(nodes.into_iter()
                    .map(|node| graph[node])
                    .collect())
            }
            Err(_) => Err(anyhow::anyhow!("Dependency graph contains a cycle")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cycle_detection() {
        let graph = DependencyGraph::new();
        
        // Создаем задачи
        let task1 = TodoItem {
            id: Uuid::new_v4(),
            title: "Task 1".to_string(),
            ..Default::default()
        };
        
        let mut task2 = TodoItem {
            id: Uuid::new_v4(),
            title: "Task 2".to_string(),
            ..Default::default()
        };
        task2.depends_on.push(task1.id);
        
        // Добавляем в граф
        graph.add_task(&task1).unwrap();
        graph.add_task(&task2).unwrap();
        
        // Проверяем что обратная зависимость создаст цикл
        assert!(graph.would_create_cycle(&task1.id, &task2.id).unwrap());
        
        // Проверяем что обычная зависимость не создаст цикл
        let task3_id = Uuid::new_v4();
        assert!(!graph.would_create_cycle(&task3_id, &task1.id).unwrap());
    }
}