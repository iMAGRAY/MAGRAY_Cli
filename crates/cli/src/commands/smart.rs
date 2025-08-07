use anyhow::Result;
use clap::Args;
use colored::*;
use domain::orchestrator::{Goal, Orchestrator, Plan, Planner, Executor, StepResult};
use async_trait::async_trait;

#[derive(Debug)]
struct SimplePlanner;

#[async_trait]
impl Planner for SimplePlanner {
    async fn create_plan(&self, goal: &Goal) -> Result<Plan> {
        Ok(Plan { steps: vec![
            domain::orchestrator::PlanStep { id: "step-1".into(), description: goal.title.clone(), tool_hint: Some("tool".into()), deps: vec![] }
        ]})
    }
}

#[derive(Debug)]
struct SimpleExecutor;

#[async_trait]
impl Executor for SimpleExecutor {
    async fn execute(&self, plan: &Plan) -> Result<Vec<StepResult>> {
        Ok(plan.steps.iter().map(|s| StepResult {
            step_id: s.id.clone(),
            status: domain::orchestrator::StepStatus::Succeeded,
            output: Some(format!("Executed: {}", s.description)),
            artifacts: vec![],
        }).collect())
    }
}

struct SimpleOrchestrator<P: Planner, E: Executor> { planner: P, executor: E }
impl<P: Planner, E: Executor> SimpleOrchestrator<P, E> { fn new(planner: P, executor: E) -> Self { Self { planner, executor } } }

#[async_trait]
impl<P: Planner + Send + Sync, E: Executor + Send + Sync> Orchestrator for SimpleOrchestrator<P, E> {
    async fn plan(&self, goal: Goal) -> Result<Plan> { self.planner.create_plan(&goal).await }
    async fn run(&self, plan: Plan) -> Result<Vec<StepResult>> { self.executor.execute(&plan).await }
}

#[derive(Debug, Args)]
pub struct SmartCommand {
    /// Сложная задача на естественном языке
    pub task: String,
}

impl SmartCommand {
    pub async fn execute(self) -> Result<()> { run_smart(self.task).await }
}

async fn run_smart(task: String) -> Result<()> {
    println!("{} {}", "★".yellow(), "Smart планировщик".bold());

    let orchestrator = SimpleOrchestrator::new(SimplePlanner, SimpleExecutor);

    let goal = Goal { title: task.clone(), description: task, constraints: vec![] };

    let plan = orchestrator.plan(goal).await?;
    println!("{} План: {} шаг(ов)", "→".cyan(), plan.steps.len());
    for step in &plan.steps { println!("  - {} {}", step.id.bold(), step.description); }

    let results = orchestrator.run(plan).await?;
    println!("{} Выполнение завершено: {} шаг(ов)", "✓".green(), results.len());
    for r in results { println!("  • {}: {:?} {}", r.step_id.bold(), r.status, r.output.clone().unwrap_or_default()); }

    Ok(())
}