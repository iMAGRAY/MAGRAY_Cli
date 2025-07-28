use anyhow::Result;
use async_trait::async_trait;
use magray_core::{Request, TodoItem, TaskState, ProjectId, DocStore};
use memory::MemoryCoordinator;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, debug, error};
use petgraph::{Graph, Direction};
use petgraph::graph::{NodeIndex, EdgeIndex};
use serde::{Serialize, Deserialize};

pub mod planner;
pub mod dag;
pub mod state_machine;

pub use planner::*;
pub use dag::*;
pub use state_machine::*;

// === Execution Context ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub request_id: Uuid,
    pub project_id: ProjectId,
    pub goal: String,
    pub current_step: Option<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub execution_log: Vec<ExecutionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_id: String,
    pub action: String,
    pub status: StepStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

// === Plan Node ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: String,
    pub action: String,
    pub description: String,
    pub tool: Option<String>,
    pub params: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub status: StepStatus,
    pub estimated_duration: Option<u64>, // seconds
}

impl PlanNode {
    pub fn new(id: String, action: String, description: String) -> Self {
        Self {
            id,
            action,
            description,
            tool: None,
            params: HashMap::new(),
            dependencies: Vec::new(),
            status: StepStatus::Pending,
            estimated_duration: None,
        }
    }

    pub fn with_tool(mut self, tool: String) -> Self {
        self.tool = Some(tool);
        self
    }

    pub fn with_param(mut self, key: String, value: serde_json::Value) -> Self {
        self.params.insert(key, value);
        self
    }

    pub fn depends_on(mut self, dependency: String) -> Self {
        self.dependencies.push(dependency);
        self
    }
}

// === Agent Executor ===

pub struct AgentExecutor {
    memory: Arc<MemoryCoordinator>,
    planner: DagPlanner,
    state_machine: StateMachine,
}

impl AgentExecutor {
    pub fn new(memory: Arc<MemoryCoordinator>) -> Self {
        Self {
            memory,
            planner: DagPlanner::new(),
            state_machine: StateMachine::new(),
        }
    }

    pub async fn execute_request(&self, request: &Request) -> Result<ExecutionContext> {
        info!("🚀 Начинаю выполнение запроса: {}", request.goal);
        
        // Создаем контекст выполнения
        let mut context = ExecutionContext {
            request_id: request.id,
            project_id: request.project_id.clone(),
            goal: request.goal.clone(),
            current_step: None,
            variables: HashMap::new(),
            execution_log: Vec::new(),
        };

        // Планируем задачи
        let plan = self.planner.create_plan(&request.goal).await?;
        info!("📋 Создан план из {} шагов", plan.nodes.len());

        // Выполняем план
        let result = self.execute_plan(plan, &mut context).await;
        
        match result {
            Ok(_) => {
                info!("✅ Запрос выполнен успешно");
                Ok(context)
            },
            Err(e) => {
                error!("❌ Ошибка выполнения: {}", e);
                Err(e)
            }
        }
    }

    async fn execute_plan(&self, plan: ExecutionPlan, context: &mut ExecutionContext) -> Result<()> {
        let execution_order = plan.get_execution_order();
        
        for node_id in execution_order {
            if let Some(node) = plan.nodes.get(&node_id) {
                context.current_step = Some(node_id.clone());
                
                let step_result = self.execute_step(node, context).await;
                
                let step = ExecutionStep {
                    step_id: node_id.clone(),
                    action: node.action.clone(),
                    status: if step_result.is_ok() { StepStatus::Completed } else { StepStatus::Failed },
                    result: step_result.as_ref().ok().cloned(),
                    error: step_result.as_ref().err().map(|e| e.to_string()),
                    timestamp: chrono::Utc::now(),
                };
                
                context.execution_log.push(step);
                
                if let Err(e) = step_result {
                    error!("❌ Шаг '{}' провален: {}", node_id, e);
                    return Err(e);
                }
                
                debug!("✅ Шаг '{}' выполнен", node_id);
            }
        }
        
        Ok(())
    }

    async fn execute_step(&self, node: &PlanNode, context: &ExecutionContext) -> Result<serde_json::Value> {
        info!("🔄 Выполняю шаг: {} - {}", node.id, node.description);
        
        match node.tool.as_deref() {
            Some("analyze") => {
                // Простой анализ запроса
                let analysis = serde_json::json!({
                    "type": "analysis",
                    "goal": context.goal,
                    "status": "analyzed",
                    "complexity": "medium"
                });
                Ok(analysis)
            },
            Some("file_read") => {
                // #INCOMPLETE: Интеграция с toolsvc для чтения файлов
                let fake_content = serde_json::json!({
                    "type": "file_content",
                    "path": node.params.get("path").unwrap_or(&serde_json::Value::Null),
                    "content": "// Заглушка: содержимое файла будет реализовано в toolsvc",
                    "size": 1024
                });
                Ok(fake_content)
            },
            Some("think") => {
                // #INCOMPLETE: Интеграция с LLM для размышления
                let thought = serde_json::json!({
                    "type": "thought",
                    "reasoning": format!("Размышление о задаче: {}", node.description),
                    "next_action": "Продолжить выполнение плана"
                });
                Ok(thought)
            },
            _ => {
                // Простое выполнение действия
                let result = serde_json::json!({
                    "type": "action",
                    "action": node.action,
                    "status": "completed",
                    "timestamp": chrono::Utc::now()
                });
                Ok(result)
            }
        }
    }
}
