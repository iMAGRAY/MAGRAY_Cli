use anyhow::Result;
use clap::{Parser, Subcommand};
use magray_core::{DocStore, ProjectId, Request, TodoItem, TaskState};
use memory::{TodoService, SqliteStore, MemoryCoordinator};
use std::env;
use std::sync::Arc;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "magray")]
#[command(about = "Локальный CLI-агент с 5-слойной памятью и семантическим роутингом")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Инициализация DocStore для текущего проекта
    Init {
        /// Путь к проекту (по умолчанию текущая директория)
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Управление задачами Todo
    Todo {
        #[command(subcommand)]
        action: TodoAction,
    },
    /// Выполнить задачу через агента
    Run {
        /// Описание задачи для выполнения
        goal: String,
    },
    /// Векторные операции
    Vec {
        #[command(subcommand)]
        action: VecAction,
    },
    /// Операции с памятью
    Mem {
        #[command(subcommand)]
        action: MemAction,
    },
}

#[derive(Subcommand)]
enum TodoAction {
    /// Добавить новую задачу
    Add {
        /// Заголовок задачи
        title: String,
        /// Описание задачи
        #[arg(short, long, default_value = "")]
        desc: String,
        /// Дата выполнения (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,
        /// Теги задачи
        #[arg(short, long)]
        tag: Vec<String>,
        /// Приоритет
        #[arg(short, long, default_value = "0")]
        priority: i32,
    },
    /// Показать список задач
    List {
        /// Фильтр по состоянию
        #[arg(long)]
        state: Option<String>,
        /// Сортировка (priority, created, due)
        #[arg(long, default_value = "priority")]
        sort: String,
    },
    /// Отметить задачу как выполненную
    Done {
        /// ID или номер задачи
        id: String,
    },
    /// Отложить задачу
    Snooze {
        /// ID или номер задачи
        id: String,
        /// На сколько отложить (например: 3d, 1w, 2h)
        #[arg(long, default_value = "1d")]
        for_duration: String,
    },
    /// Удалить устаревшие задачи
    Prune {
        /// Удалить задачи со staleness выше указанного значения
        #[arg(long, default_value = "7.0")]
        stale: f32,
    },
}

#[derive(Subcommand)]
enum VecAction {
    /// Переиндексировать векторы
    Reindex,
    /// Показать статистику векторного индекса
    Stats,
}

#[derive(Subcommand)]
enum MemAction {
    /// Сжать память и удалить неиспользуемые данные
    Compact,
    /// Показать статистику использования памяти по слоям
    Stats,
}

fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("magray=info".parse().unwrap())
        )
        .init();
}

fn get_project_id(path: Option<String>) -> Result<ProjectId> {
    let project_path = if let Some(p) = path {
        std::path::PathBuf::from(p)
    } else {
        env::current_dir()?
    };
    
    Ok(ProjectId::from_path(&project_path))
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_tracing();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path } => {
            let project_id = get_project_id(path)?;
            let docstore = DocStore::new(&project_id)?;
            
            info!("Инициализация DocStore для проекта: {}", project_id.as_str());
            docstore.init()?;
            
            println!("✅ DocStore инициализирован: {}", docstore.root.display());
            println!("📁 Конфигурация: {}", docstore.root.join("config.toml").display());
            println!("🗄️  База данных: {}", docstore.sqlite_path().display());
            println!("📝 Задачи: {}", docstore.tasks_path().display());
        }
        
        Commands::Todo { action } => {
            let project_id = get_project_id(None)?;
            let docstore = DocStore::new(&project_id)?;
            
            if !docstore.root.exists() {
                error!("DocStore не найден. Запустите 'magray init' сначала.");
                std::process::exit(1);
            }
            
            handle_todo_action(action, &docstore).await?;
        }
        
        Commands::Run { goal } => {
            let project_id = get_project_id(None)?;
            let docstore = DocStore::new(&project_id)?;
            
            if !docstore.root.exists() {
                error!("DocStore не найден. Запустите 'magray init' сначала.");
                std::process::exit(1);
            }
            
            let request = Request::new(goal.clone(), project_id);
            info!("Создан запрос: {} (ID: {})", goal, request.id);
            
            // #INCOMPLETE: Здесь будет интеграция с Planner и Executor
            println!("🚀 Запрос принят: {}", goal);
            println!("🆔 ID запроса: {}", request.id);
            println!("⏳ Планирование и выполнение будет реализовано в следующих версиях");
        }
        
        Commands::Vec { action } => {
            let project_id = get_project_id(None)?;
            let docstore = DocStore::new(&project_id)?;
            
            if !docstore.root.exists() {
                error!("DocStore не найден. Запустите 'magray init' сначала.");
                std::process::exit(1);
            }
            
            handle_vec_action(action, &docstore).await?;
        }
        
        Commands::Mem { action } => {
            let project_id = get_project_id(None)?;
            let docstore = DocStore::new(&project_id)?;
            
            if !docstore.root.exists() {
                error!("DocStore не найден. Запустите 'magray init' сначала.");
                std::process::exit(1);
            }
            
            handle_mem_action(action, &docstore).await?;
        }
    }
    
    Ok(())
}

async fn handle_todo_action(action: TodoAction, docstore: &DocStore) -> Result<()> {
    let todo_service = TodoService::new(docstore)?;
    
    match action {
        TodoAction::Add { title, desc, due, tag, priority } => {
            let mut todo = TodoItem::new(title.clone(), desc);
            todo.priority = priority;
            todo.tags = tag;
            
            if let Some(due_str) = due {
                // #INCOMPLETE: Парсинг даты из строки
                println!("⚠️  Парсинг даты '{}' будет реализован позже", due_str);
            }
            
            let saved_todo = todo_service.add(todo).await?;
            
            println!("✅ Задача добавлена: {}", title);
            println!("🆔 ID: {}", saved_todo.id);
            println!("📅 Создана: {}", saved_todo.created_at.format("%Y-%m-%d %H:%M:%S"));
            println!("⭐ Приоритет: {}", saved_todo.priority);
            println!("📊 Staleness: {:.2}", saved_todo.staleness);
            if !saved_todo.tags.is_empty() {
                println!("🏷️  Теги: {}", saved_todo.tags.join(", "));
            }
        }
        
        TodoAction::List { state, sort } => {
            let state_filter = state.as_ref().and_then(|s| match s.as_str() {
                "planned" => Some(TaskState::Planned),
                "ready" => Some(TaskState::Ready),
                "in-progress" => Some(TaskState::InProgress),
                "blocked" => Some(TaskState::Blocked),
                "done" => Some(TaskState::Done),
                "archived" => Some(TaskState::Archived),
                _ => None,
            });
            
            let todos = todo_service.list(state_filter, &sort, Some(50)).await?;
            
            println!("📋 Список задач ({} найдено):", todos.len());
            if let Some(state_filter) = state {
                println!("🔍 Фильтр: состояние = {}", state_filter);
            }
            println!("📊 Сортировка: {}", sort);
            println!();
            
            if todos.is_empty() {
                println!("📭 Задач не найдено");
            } else {
                for (i, todo) in todos.iter().enumerate() {
                    let status_icon = match todo.state {
                        TaskState::Planned => "📋",
                        TaskState::Ready => "🔄", 
                        TaskState::InProgress => "⚡",
                        TaskState::Blocked => "🚫",
                        TaskState::Done => "✅",
                        TaskState::Archived => "📦",
                    };
                    
                    println!("{}. {} {} ({})", 
                        i + 1, 
                        status_icon, 
                        todo.title,
                        todo.state
                    );
                    
                    if !todo.desc.is_empty() {
                        println!("   📝 {}", todo.desc);
                    }
                    
                    println!("   🆔 {} | ⭐ {} | 📊 {:.1} | 📅 {}", 
                        todo.id.to_string()[..8].to_string(),
                        todo.priority,
                        todo.staleness,
                        todo.created_at.format("%m-%d %H:%M")
                    );
                    
                    if !todo.tags.is_empty() {
                        println!("   🏷️  {}", todo.tags.join(", "));
                    }
                    
                    println!();
                }
                
                // Показываем статистику
                let counts = todo_service.count_by_state().await?;
                print!("📊 Статистика: ");
                for (state, count) in counts {
                    print!("{}: {} ", state, count);
                }
                println!();
            }
        }
        
        TodoAction::Done { id } => {
            // Пытаемся найти по UUID или по префиксу
            let todo_id = if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
                uuid
            } else {
                // Поиск по префиксу ID
                let todos = todo_service.list(None, "created", None).await?;
                if let Some(todo) = todos.iter().find(|t| t.id.to_string().starts_with(&id)) {
                    todo.id
                } else {
                    println!("❌ Задача с ID '{}' не найдена", id);
                    return Ok(());
                }
            };
            
            if todo_service.update_state(&todo_id, TaskState::Done).await? {
                println!("✅ Задача отмечена как выполненная: {}", todo_id);
            } else {
                println!("❌ Задача с ID '{}' не найдена", id);
            }
        }
        
        TodoAction::Snooze { id, for_duration } => {
            println!("😴 Задача отложена: {} на {}", id, for_duration);
            // #INCOMPLETE: Парсинг duration и обновление в БД
            println!("⏳ Функция отложения будет реализована позже");
        }
        
        TodoAction::Prune { stale } => {
            let removed = todo_service.prune_stale(stale).await?;
            println!("🧹 Удалено {} задач со staleness > {}", removed, stale);
        }
    }
    
    Ok(())
}

async fn handle_vec_action(action: VecAction, docstore: &DocStore) -> Result<()> {
    match action {
        VecAction::Reindex => {
            println!("🔄 Переиндексация векторов...");
            println!("📁 Векторная база: {}", docstore.vectors_dir().display());
            // #INCOMPLETE: Интеграция с VectorizerSvc и Semantic Index
            println!("⏳ Векторная индексация будет реализована в следующих версиях");
        }
        
        VecAction::Stats => {
            println!("📊 Статистика векторного индекса:");
            println!("📁 Расположение: {}", docstore.vectors_dir().display());
            // #INCOMPLETE: Показ реальной статистики
            println!("⏳ Статистика будет реализована после создания векторного индекса");
        }
    }
    
    Ok(())
}

async fn handle_mem_action(action: MemAction, docstore: &DocStore) -> Result<()> {
    match action {
        MemAction::Compact => {
            println!("🗜️  Сжатие памяти...");
            println!("🗄️  SQLite: {}", docstore.sqlite_path().display());
            println!("📁 Blobs: {}", docstore.blobs_dir().display());
            // #INCOMPLETE: Реальное сжатие через MemoryCoordinator
            println!("⏳ Сжатие памяти будет реализовано в memory crate");
        }
        
        MemAction::Stats => {
            println!("📊 Статистика памяти по слоям:");
            println!("🧠 M0 (Ephemeral): В RAM, статистика недоступна");
            println!("⚡ M1 (Short): {}", docstore.sqlite_path().display());
            println!("📚 M2 (Medium): {}", docstore.sqlite_path().display()); 
            println!("📦 M3 (Long): {}", docstore.blobs_dir().display());
            println!("🔍 M4 (Semantic): {}", docstore.vectors_dir().display());
            // #INCOMPLETE: Реальная статистика размеров и использования
            println!("⏳ Детальная статистика будет реализована в memory crate");
        }
    }
    
    Ok(())
}
