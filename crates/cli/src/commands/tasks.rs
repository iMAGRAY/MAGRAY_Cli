use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use todo::{create_default_service, Priority, TaskState};

#[derive(Debug, Args)]
pub struct TasksCommand {
    #[command(subcommand)]
    command: TasksSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TasksSubcommand {
    /// Создать новую задачу
    #[command(name = "create")]
    Create {
        /// Заголовок задачи
        title: String,
        /// Описание задачи
        #[arg(short, long)]
        description: Option<String>,
        /// Приоритет (low, medium, high, critical)
        #[arg(short, long, default_value = "medium")]
        priority: String,
        /// Теги через запятую
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// Показать N готовых задач
    #[command(name = "list")]
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// Показать только задачи с определенным состоянием
        #[arg(long)]
        state: Option<String>,
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
    /// Добавить зависимость между задачами
    #[command(name = "depends")]
    Depends {
        /// ID задачи, которая зависит
        task_id: String,
        /// ID задачи, от которой зависит
        depends_on: String,
    },
    /// Показать граф зависимостей
    #[command(name = "graph")]
    Graph {
        /// Формат вывода (text, mermaid)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Максимальная глубина графа
        #[arg(long, default_value_t = 5)]
        depth: usize,
    },
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
        TasksSubcommand::Create {
            title,
            description,
            priority,
            tags,
        } => {
            let priority = match priority.to_lowercase().as_str() {
                "low" => Priority::Low,
                "medium" => Priority::Medium,
                "high" => Priority::High,
                "critical" => Priority::Critical,
                _ => {
                    println!(
                        "{} Unknown priority '{}'. Using 'medium'",
                        "⚠".yellow(),
                        priority
                    );
                    Priority::Medium
                }
            };

            let tags = if let Some(tags_str) = tags {
                tags_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                vec![]
            };

            let task = svc
                .create_task(
                    title,
                    description.unwrap_or_else(|| "No description".to_string()),
                    priority,
                    tags,
                )
                .await?;

            println!(
                "{} Задача создана: {} [{}]",
                "✓".green(),
                task.title.bold(),
                task.id
            );
            println!("   приоритет: {:?}", task.priority);
            println!("   состояние: {:?}", task.state);
        }
        TasksSubcommand::List { limit, state } => {
            let tasks = if let Some(state_str) = state {
                let state_enum = match state_str.to_lowercase().as_str() {
                    "ready" => TaskState::Ready,
                    "in_progress" => TaskState::InProgress,
                    "done" => TaskState::Done,
                    "blocked" => TaskState::Blocked,
                    "failed" => TaskState::Failed,
                    "cancelled" => TaskState::Cancelled,
                    "planned" => TaskState::Planned,
                    _ => {
                        println!(
                            "{} Unknown state '{}'. Using search",
                            "⚠".yellow(),
                            state_str
                        );
                        return Ok(());
                    }
                };
                svc.get_by_state(state_enum, limit).await?
            } else {
                svc.get_next_ready(limit).await?
            };
            println!(
                "{} {}",
                "✓".green(),
                format!("Найдено задач: {}", tasks.len()).bold()
            );
            for t in tasks {
                let state_icon = match t.state {
                    TaskState::Ready => "⚡".yellow(),
                    TaskState::InProgress => "🔄".cyan(),
                    TaskState::Done => "✅".green(),
                    TaskState::Blocked => "🚫".red(),
                    TaskState::Failed => "❌".red(),
                    TaskState::Cancelled => "⚪".white(),
                    TaskState::Planned => "📋".blue(),
                };

                println!(
                    "{} {} {} [{}] {}",
                    state_icon,
                    t.id.to_string()
                        .get(0..8)
                        .unwrap_or(&t.id.to_string())
                        .dimmed(),
                    t.title.bold(),
                    format!("{:?}", t.priority).to_lowercase(),
                    format!("{:?}", t.state).to_lowercase().dimmed()
                );

                if !t.tags.is_empty() {
                    println!("     tags: {}", t.tags.join(", ").cyan());
                }
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
                    println!("  reasoning: {r}");
                }
                if let Some(tool) = &t.tool_hint {
                    println!("  tool_hint: {tool}");
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
        TasksSubcommand::Depends {
            task_id,
            depends_on,
        } => {
            let task_uuid = uuid::Uuid::parse_str(&task_id)?;
            let depends_uuid = uuid::Uuid::parse_str(&depends_on)?;

            svc.add_dependency(&task_uuid, &depends_uuid).await?;

            println!(
                "{} Зависимость добавлена: {} → {}",
                "🔗".cyan(),
                task_id,
                depends_on
            );
        }
        TasksSubcommand::Graph { format, depth } => match format.to_lowercase().as_str() {
            "text" => {
                println!("{} Граф зависимостей:", "📊".blue());
                let graph_text = svc.visualize_graph_text(depth).await?;
                println!("{graph_text}");
            }
            "mermaid" => {
                println!("{} Mermaid граф:", "📊".blue());
                let mermaid_graph = svc.visualize_graph_mermaid(depth).await?;
                println!("{mermaid_graph}");
            }
            _ => {
                println!(
                    "{} Unknown format '{}'. Supported: text, mermaid",
                    "✗".red(),
                    format
                );
            }
        },
        TasksSubcommand::Stats => {
            let (task_stats, graph_stats) = svc.get_stats().await?;
            println!("{} Статистика задач", "📊".yellow());
            println!("  total: {}", task_stats.total);
            println!(
                "  ⚡ ready: {}  🔄 in_progress: {}  🚫 blocked: {}",
                task_stats.ready, task_stats.in_progress, task_stats.blocked
            );
            println!(
                "  ✅ done: {}  ❌ failed: {}  ⚪ cancelled: {}",
                task_stats.done, task_stats.failed, task_stats.cancelled
            );
            println!("  📋 planned: {}", task_stats.planned);
            println!(
                "{} Граф: задач={}, зависимостей={}, кэш={}",
                "🔗".blue(),
                graph_stats.total_tasks,
                graph_stats.total_dependencies,
                graph_stats.cache_size
            );
        }
    }
    Ok(())
}
