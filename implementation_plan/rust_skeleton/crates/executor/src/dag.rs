use anyhow::Result;
use petgraph::{Graph, Direction};
use petgraph::graph::{NodeIndex, UnGraph};
use std::collections::HashMap;
use tracing::{debug, warn};
use crate::{PlanNode, StepStatus};

// === Execution Plan ===

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub nodes: HashMap<String, PlanNode>,
    pub graph: Graph<String, (), petgraph::Directed>,
    node_indices: HashMap<String, NodeIndex>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            graph: Graph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: PlanNode) -> Result<()> {
        let node_id = node.id.clone();
        
        // Добавляем узел в граф
        let index = self.graph.add_node(node_id.clone());
        self.node_indices.insert(node_id.clone(), index);
        
        // Сохраняем данные узла
        self.nodes.insert(node_id, node);
        
        Ok(())
    }

    pub fn add_dependency(&mut self, from: &str, to: &str) -> Result<()> {
        let from_idx = self.node_indices.get(from)
            .ok_or_else(|| anyhow::anyhow!("Node '{}' not found", from))?;
        let to_idx = self.node_indices.get(to)
            .ok_or_else(|| anyhow::anyhow!("Node '{}' not found", to))?;
        
        // Добавляем ребро: from -> to (from зависит от to)
        self.graph.add_edge(*to_idx, *from_idx, ());
        
        debug!("Добавлена зависимость: {} зависит от {}", from, to);
        Ok(())
    }

    pub fn get_execution_order(&self) -> Vec<String> {
        // Топологическая сортировка для определения порядка выполнения
        match petgraph::algo::toposort(&self.graph, None) {
            Ok(sorted) => {
                sorted.into_iter()
                    .map(|idx| self.graph[idx].clone())
                    .collect()
            },
            Err(_) => {
                warn!("❌ Обнаружен цикл в зависимостях! Используем упрощенный порядок");
                self.nodes.keys().cloned().collect()
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        // Проверяем циклические зависимости
        if let Err(_) = petgraph::algo::toposort(&self.graph, None) {
            return Err(anyhow::anyhow!("Обнаружены циклические зависимости в плане"));
        }

        // Проверяем что все зависимости существуют
        for (node_id, node) in &self.nodes {
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(anyhow::anyhow!(
                        "Узел '{}' зависит от несуществующего узла '{}'", 
                        node_id, dep
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn get_ready_nodes(&self) -> Vec<String> {
        // Возвращает узлы, которые готовы к выполнению
        // (все их зависимости выполнены)
        let mut ready = Vec::new();
        
        for (node_id, node) in &self.nodes {
            if node.status != StepStatus::Pending {
                continue;
            }
            
            let all_deps_completed = node.dependencies.iter().all(|dep_id| {
                self.nodes.get(dep_id)
                    .map(|dep_node| dep_node.status == StepStatus::Completed)
                    .unwrap_or(false)
            });
            
            if all_deps_completed {
                ready.push(node_id.clone());
            }
        }
        
        ready
    }

    pub fn mark_completed(&mut self, node_id: &str) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = StepStatus::Completed;
            debug!("✅ Узел '{}' помечен как завершенный", node_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Узел '{}' не найден", node_id))
        }
    }

    pub fn mark_failed(&mut self, node_id: &str) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = StepStatus::Failed;
            debug!("❌ Узел '{}' помечен как проваленный", node_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Узел '{}' не найден", node_id))
        }
    }

    pub fn get_statistics(&self) -> PlanStatistics {
        let mut stats = PlanStatistics::default();
        
        for node in self.nodes.values() {
            match node.status {
                StepStatus::Pending => stats.pending += 1,
                StepStatus::Running => stats.running += 1,
                StepStatus::Completed => stats.completed += 1,
                StepStatus::Failed => stats.failed += 1,
                StepStatus::Skipped => stats.skipped += 1,
            }
            
            stats.total += 1;
            
            if let Some(duration) = node.estimated_duration {
                stats.estimated_total_duration += duration;
            }
        }
        
        stats
    }
}

#[derive(Debug, Default)]
pub struct PlanStatistics {
    pub total: usize,
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub estimated_total_duration: u64, // seconds
}

impl PlanStatistics {
    pub fn progress_percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.completed as f32 / self.total as f32) * 100.0
        }
    }

    pub fn is_completed(&self) -> bool {
        self.pending == 0 && self.running == 0 && self.failed == 0
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}