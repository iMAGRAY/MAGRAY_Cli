use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use todo::{create_default_service, TaskState};

#[derive(Debug, Args)]
pub struct TasksCommand {
    #[command(subcommand)]
    command: TasksSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TasksSubcommand {
    /// Показать N готовых задач
    #[command(name = "list")]
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Показать задачу по ID
    #[command(name = "show")]
    Show { id: String },
    /// Отметить задачу выполненной
    #[command(name = "done")]
    Done { id: String },
    /// Удалить задачу (пока просто смена статуса на cancelled)
    #[command(name = "rm")]
    Rm { id: String },
    /// Показать статистику
    #[command(name = "stats")]
    Stats,
}

impl TasksCommand {
    pub async fn execute(self) -> Result<()> {
        handle(self.command).await
    }
}

async fn handle(cmd: TasksSubcommand) -> Result<()> {
    let db_path = super::super::util::default_tasks_db_path();
    let svc = create_default_service(db_path).await?;

    match cmd {
        TasksSubcommand::List { limit } => {
            let tasks = svc.get_next_ready(limit).await?;
            println!(
                "{} {}",
                "✓".green(),
                format!("Готовые задачи: {}", tasks.len()).bold()
            );
            for t in tasks {
                println!(
                    "- {} {} [{}]",
                    t.id,
                    t.title.bold(),
                    format!("{:?}", t.priority).to_lowercase()
                );
            }
        }
        TasksSubcommand::Show { id } => {
            let id = uuid::Uuid::parse_str(&id)?;
            if let Some(t) = svc.get_cached(&id).await? {
                println!("{} {}", "☐".cyan(), t.title.bold());
                println!("  id: {}", t.id);
                println!("  state: {:?}", t.state);
                println!("  created: {}", t.created_at);
                println!("  updated: {}", t.updated_at);
                if let Some(r) = &t.reasoning {
                    println!("  reasoning: {}", r);
                }
                if let Some(tool) = &t.tool_hint {
                    println!("  tool_hint: {}", tool);
                }
            } else {
                println!("{} Задача не найдена", "✗".red());
            }
        }
        TasksSubcommand::Done { id } => {
            let id = uuid::Uuid::parse_str(&id)?;
            svc.update_state(&id, TaskState::Done).await?;
            println!("{} Задача помечена выполненной", "✓".green());
        }
        TasksSubcommand::Rm { id } => {
            let id = uuid::Uuid::parse_str(&id)?;
            svc.update_state(&id, TaskState::Cancelled).await?;
            println!("{} Задача отменена", "✓".green());
        }
        TasksSubcommand::Stats => {
            let (task_stats, graph_stats) = svc.get_stats().await?;
            println!("{} Статистика задач", "Σ".yellow());
            println!("  total: {}", task_stats.total);
            println!(
                "  ready: {}  in_progress: {}  blocked: {}",
                task_stats.ready, task_stats.in_progress, task_stats.blocked
            );
            println!(
                "  done: {}  failed: {}  cancelled: {}",
                task_stats.done, task_stats.failed, task_stats.cancelled
            );
            println!(
                "{} Граф: total_tasks={}, total_deps={}, cache_entries={}",
                "ℹ".blue(),
                graph_stats.total_tasks,
                graph_stats.total_dependencies,
                graph_stats.cache_size
            );
        }
    }
    Ok(())
}
