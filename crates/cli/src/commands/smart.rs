use anyhow::{anyhow, Result};
use clap::Args;
use colored::*;
use domain::orchestrator::{Executor, Goal, Orchestrator, Plan, Planner, StepResult, StepStatus};
use async_trait::async_trait;
use tools::{Tool, ToolInput, ToolOutput, ToolRegistry};
use std::collections::HashMap;
use todo::{TodoService, Priority, TaskState, TodoItem, create_default_service};
use std::path::PathBuf;

#[derive(Debug)]
struct SimplePlanner;

#[async_trait]
impl Planner for SimplePlanner {
    async fn create_plan(&self, goal: &Goal) -> Result<Plan> {
        let hint = detect_tool_hint(&goal.title);
        Ok(Plan {
            steps: vec![domain::orchestrator::PlanStep {
                id: "step-1".into(),
                description: goal.title.clone(),
                tool_hint: hint,
                deps: vec![],
            }],
        })
    }
}

#[derive(Debug)]
struct SimpleExecutor;

#[async_trait]
impl Executor for SimpleExecutor {
    async fn execute(&self, plan: &Plan) -> Result<Vec<StepResult>> {
        let registry = ToolRegistry::new();
        let mut results = Vec::with_capacity(plan.steps.len());

        // Инициализируем TodoService (локальная база в ~/.magray/tasks.db)
        let db_path = default_tasks_db_path();
        let todo_service: TodoService = create_default_service(&db_path).await?;

        for step in &plan.steps {
            // Создаём задачу под шаг
            let task = todo_service
                .create_task(
                    step.description.clone(),
                    "Создано оркестратором MAGRAY".to_string(),
                    Priority::Medium,
                    vec!["smart".to_string()],
                )
                .await?;

            // Выполняем шаг как инструмент
            let desc = step.description.clone();
            let output = execute_step(&registry, &desc, step.tool_hint.clone()).await;

            // Обновляем состояние задачи
            match output {
                Ok(out) => {
                    todo_service.update_state(&task.id, TaskState::Done).await?;
                    results.push(StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Succeeded,
                        output: Some(out.result.clone()),
                        artifacts: vec![],
                    })
                }
                Err(e) => {
                    todo_service.update_state(&task.id, TaskState::Failed).await?;
                    results.push(StepResult {
                        step_id: step.id.clone(),
                        status: StepStatus::Failed { error: e.to_string() },
                        output: None,
                        artifacts: vec![],
                    })
                }
            }
        }

        Ok(results)
    }
}

fn default_tasks_db_path() -> PathBuf {
    let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push(".magray");
    std::fs::create_dir_all(&dir).ok();
    dir.push("tasks.db");
    dir
}

struct SimpleOrchestrator<P: Planner, E: Executor> {
    planner: P,
    executor: E,
}
impl<P: Planner, E: Executor> SimpleOrchestrator<P, E> {
    fn new(planner: P, executor: E) -> Self {
        Self { planner, executor }
    }
}

#[async_trait]
impl<P: Planner + Send + Sync, E: Executor + Send + Sync> Orchestrator for SimpleOrchestrator<P, E> {
    async fn plan(&self, goal: Goal) -> Result<Plan> {
        self.planner.create_plan(&goal).await
    }
    async fn run(&self, plan: Plan) -> Result<Vec<StepResult>> {
        self.executor.execute(&plan).await
    }
}

#[derive(Debug, Args)]
pub struct SmartCommand {
    /// Сложная задача на естественном языке
    pub task: String,
}

impl SmartCommand {
    pub async fn execute(self) -> Result<()> {
        run_smart(self.task).await
    }
}

async fn run_smart(task: String) -> Result<()> {
    println!("{} {}", "★".yellow(), "Smart планировщик".bold());

    let orchestrator = SimpleOrchestrator::new(SimplePlanner, SimpleExecutor);

    let goal = Goal {
        title: task.clone(),
        description: task,
        constraints: vec![],
    };

    let plan = orchestrator.plan(goal).await?;
    println!("{} План: {} шаг(ов)", "→".cyan(), plan.steps.len());
    for step in &plan.steps {
        let hint = step.tool_hint.clone().unwrap_or_else(|| "auto".to_string());
        println!("  - {} {}  [{}]", step.id.bold(), step.description, hint);
    }

    let results = orchestrator.run(plan).await?;
    println!("{} Выполнение завершено: {} шаг(ов)", "✓".green(), results.len());
    for r in results {
        println!("  • {}: {:?}", r.step_id.bold(), r.status);
        if let Some(out) = r.output {
            if !out.trim().is_empty() { println!("{}", out); }
        }
    }

    Ok(())
}

fn detect_tool_hint(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    if (lower.contains("прочитай") || lower.contains("read") || lower.contains("покажи содержимое"))
        && looks_like_path(text)
    {
        return Some("file_read".to_string());
    }
    if lower.contains("создай файл") || lower.contains("write") || lower.contains("запиши в файл") {
        return Some("file_write".to_string());
    }
    if lower.contains("папк") || lower.contains("директор") || lower.contains("list dir") || lower.contains("ls ") {
        return Some("dir_list".to_string());
    }
    if lower.contains("git diff") || lower.contains("показать diff") {
        return Some("git_diff".to_string());
    }
    if lower.contains("git status") || lower.contains("статус git") {
        return Some("git_status".to_string());
    }
    if lower.contains("git commit") || lower.contains("коммит") {
        return Some("git_commit".to_string());
    }
    if lower.starts_with("выполни") || lower.contains("выполни команду") || lower.starts_with("exec") || lower.starts_with("run ") {
        return Some("shell_exec".to_string());
    }
    if lower.contains("найди ") || lower.contains("поиск ") || lower.contains("search ") {
        return Some("web_search".to_string());
    }
    None
}

fn looks_like_path(text: &str) -> bool {
    text.contains('/') || text.contains('\\') || text.contains(".rs") || text.contains(".md") || text.contains(".toml")
}

async fn execute_step(registry: &ToolRegistry, description: &str, hint: Option<String>) -> Result<ToolOutput> {
    if let Some(name) = hint {
        if let Some(tool) = registry.get(&name) {
            // Попробуем использовать парсер NL, иначе отправим пустые args
            let input = if tool.supports_natural_language() {
                match tool.parse_natural_language(description).await {
                    Ok(i) => i,
                    Err(_) => ToolInput { command: name.clone(), args: HashMap::new(), context: Some(description.to_string()) },
                }
            } else {
                ToolInput { command: name.clone(), args: HashMap::new(), context: Some(description.to_string()) }
            };
            return tool.execute(input).await;
        } else {
            return Err(anyhow!("Неизвестный инструмент: {}", name));
        }
    }

    // Автовыбор: пробуем набор инструментов в приоритетном порядке
    let candidates = [
        "file_read", "file_write", "dir_list", "git_status", "git_diff", "git_commit", "shell_exec", "web_search",
    ];

    for name in candidates {
        if let Some(tool) = registry.get(name) {
            if !tool.supports_natural_language() { continue; }
            if let Ok(input) = tool.parse_natural_language(description).await {
                // Если парсер выдал какие‑то аргументы — пробуем исполнить
                if !input.args.is_empty() || input.context.is_some() {
                    return tool.execute(input).await;
                }
            }
        }
    }

    Err(anyhow!("Не удалось подобрать инструмент для шага"))
}