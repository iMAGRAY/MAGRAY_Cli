use anyhow::Result;
use std::collections::HashMap;
use tracing::{info, debug};
use crate::{ExecutionPlan, PlanNode, StepStatus};

// === DAG Planner ===

pub struct DagPlanner {
    // #INCOMPLETE: Здесь будет интеграция с LLM для умного планирования
    templates: HashMap<String, PlanTemplate>,
}

impl DagPlanner {
    pub fn new() -> Self {
        let mut planner = Self {
            templates: HashMap::new(),
        };
        
        // Загружаем базовые шаблоны планов
        planner.load_basic_templates();
        planner
    }

    pub async fn create_plan(&self, goal: &str) -> Result<ExecutionPlan> {
        info!("🔍 Создаю план для цели: {}", goal);
        
        // #INCOMPLETE: Здесь нужен NLU для классификации запроса
        let intent = self.classify_intent(goal);
        
        match intent.as_str() {
            "analyze_code" => self.create_analysis_plan(goal),
            "file_operation" => self.create_file_plan(goal),
            "research" => self.create_research_plan(goal),
            _ => self.create_generic_plan(goal),
        }
    }

    fn classify_intent(&self, goal: &str) -> String {
        let goal_lower = goal.to_lowercase();
        
        if goal_lower.contains("анализ") || goal_lower.contains("analyze") {
            "analyze_code".to_string()
        } else if goal_lower.contains("файл") || goal_lower.contains("file") {
            "file_operation".to_string()
        } else if goal_lower.contains("исследов") || goal_lower.contains("research") {
            "research".to_string()
        } else {
            "generic".to_string()
        }
    }

    fn create_analysis_plan(&self, goal: &str) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        
        // Шаг 1: Анализ запроса
        let analyze_node = PlanNode::new(
            "analyze_request".to_string(),
            "analyze".to_string(),
            format!("Анализ запроса: {}", goal)
        ).with_tool("analyze".to_string());
        plan.add_node(analyze_node)?;
        
        // Шаг 2: Поиск релевантных файлов
        let search_node = PlanNode::new(
            "search_files".to_string(), 
            "search".to_string(),
            "Поиск релевантных файлов проекта".to_string()
        ).with_tool("file_search".to_string())
        .depends_on("analyze_request".to_string());
        plan.add_node(search_node)?;
        
        // Шаг 3: Чтение ключевых файлов
        let read_node = PlanNode::new(
            "read_files".to_string(),
            "read".to_string(), 
            "Чтение найденных файлов".to_string()
        ).with_tool("file_read".to_string())
        .depends_on("search_files".to_string());
        plan.add_node(read_node)?;
        
        // Шаг 4: Анализ кода
        let code_analysis_node = PlanNode::new(
            "analyze_code".to_string(),
            "think".to_string(),
            "Глубокий анализ кода".to_string()
        ).with_tool("think".to_string())
        .depends_on("read_files".to_string());
        plan.add_node(code_analysis_node)?;
        
        // Шаг 5: Формирование отчета
        let report_node = PlanNode::new(
            "generate_report".to_string(),
            "report".to_string(),
            "Генерация отчета анализа".to_string()
        ).depends_on("analyze_code".to_string());
        plan.add_node(report_node)?;
        
        // Добавляем зависимости в граф
        plan.add_dependency("search_files", "analyze_request")?;
        plan.add_dependency("read_files", "search_files")?;
        plan.add_dependency("analyze_code", "read_files")?;
        plan.add_dependency("generate_report", "analyze_code")?;
        
        plan.validate()?;
        debug!("✅ Создан план анализа с {} шагами", plan.nodes.len());
        
        Ok(plan)
    }

    fn create_file_plan(&self, goal: &str) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        
        // Простой план для работы с файлами
        let read_node = PlanNode::new(
            "read_target_file".to_string(),
            "read".to_string(),
            format!("Работа с файлом: {}", goal)
        ).with_tool("file_read".to_string())
        .with_param("path".to_string(), serde_json::Value::String("./".to_string()));
        
        plan.add_node(read_node)?;
        
        let process_node = PlanNode::new(
            "process_file".to_string(),
            "process".to_string(),
            "Обработка файла".to_string()
        ).depends_on("read_target_file".to_string());
        
        plan.add_node(process_node)?;
        plan.add_dependency("process_file", "read_target_file")?;
        
        plan.validate()?;
        debug!("✅ Создан план работы с файлами с {} шагами", plan.nodes.len());
        
        Ok(plan)
    }

    fn create_research_plan(&self, goal: &str) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        
        // План для исследовательских задач
        let research_node = PlanNode::new(
            "research_topic".to_string(),
            "research".to_string(),
            format!("Исследование темы: {}", goal)
        ).with_tool("web_search".to_string());
        
        plan.add_node(research_node)?;
        
        let analyze_node = PlanNode::new(
            "analyze_findings".to_string(),
            "think".to_string(),
            "Анализ найденной информации".to_string()
        ).with_tool("think".to_string())
        .depends_on("research_topic".to_string());
        
        plan.add_node(analyze_node)?;
        plan.add_dependency("analyze_findings", "research_topic")?;
        
        plan.validate()?;
        debug!("✅ Создан план исследования с {} шагами", plan.nodes.len());
        
        Ok(plan)
    }

    fn create_generic_plan(&self, goal: &str) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        
        // Базовый универсальный план
        let analyze_node = PlanNode::new(
            "analyze_goal".to_string(),
            "analyze".to_string(),
            format!("Анализ задачи: {}", goal)
        ).with_tool("analyze".to_string());
        
        plan.add_node(analyze_node)?;
        
        let execute_node = PlanNode::new(
            "execute_action".to_string(),
            "execute".to_string(),
            "Выполнение основного действия".to_string()
        ).depends_on("analyze_goal".to_string());
        
        plan.add_node(execute_node)?;
        plan.add_dependency("execute_action", "analyze_goal")?;
        
        plan.validate()?;
        debug!("✅ Создан универсальный план с {} шагами", plan.nodes.len());
        
        Ok(plan)
    }

    fn load_basic_templates(&mut self) {
        // #INCOMPLETE: Загрузка шаблонов планов из конфигурации
        let analysis_template = PlanTemplate {
            name: "code_analysis".to_string(),
            description: "Шаблон для анализа кода".to_string(),
            steps: vec![
                "analyze_request".to_string(),
                "search_files".to_string(),
                "read_files".to_string(),
                "analyze_code".to_string(),
                "generate_report".to_string(),
            ],
        };
        
        self.templates.insert("analyze_code".to_string(), analysis_template);
        
        debug!("📋 Загружено {} шаблонов планов", self.templates.len());
    }
}

// === Plan Template ===

#[derive(Debug, Clone)]
struct PlanTemplate {
    name: String,
    description: String,
    steps: Vec<String>,
}

impl PlanTemplate {
    pub fn instantiate(&self, goal: &str) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        
        for (i, step) in self.steps.iter().enumerate() {
            let node = PlanNode::new(
                format!("{}_{}", step, i),
                step.clone(),
                format!("Шаг {}: {} для цели '{}'", i + 1, step, goal)
            );
            
            plan.add_node(node)?;
            
            // Добавляем зависимость от предыдущего шага
            if i > 0 {
                let prev_step = format!("{}_{}", self.steps[i-1], i-1);
                let curr_step = format!("{}_{}", step, i);
                plan.add_dependency(&curr_step, &prev_step)?;
            }
        }
        
        plan.validate()?;
        Ok(plan)
    }
}