use anyhow::Result;
use clap::{Parser, Subcommand};
use common::init_structured_logging;
use common::{events, topics};
use console::{style, Term};
use indicatif::ProgressStyle;
use llm::LlmClient;
use serde_json::json;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use ui::tui::TUIApp;

mod commands;
mod health_checks;
mod progress;
mod services;
mod tui_chat;
mod util;

#[cfg(test)]
mod status_tests;

use cli::agent_traits::AgentResponse;
use commands::{
    GpuCommand, MemoryCommand, ModelsCommand, OrchestratorCommand, SmartCommand, TasksCommand,
    ToolsCommand,
};
use orchestrator::orchestrator::AgentOrchestrator;

// Иконки для CLI интерфейса
static ROBOT_ICON: AnimatedIcon = AnimatedIcon::new(&["[AI]", "[▲I]", "[●I]", "[♦I]"]);
static USER_ICON: &str = "[►]";

// Анимированные ASCII иконки
struct AnimatedIcon {
    frames: &'static [&'static str],
}

impl AnimatedIcon {
    const fn new(frames: &'static [&'static str]) -> Self {
        Self { frames }
    }

    fn get_frame(&self, index: usize) -> &'static str {
        self.frames[index % self.frames.len()]
    }
}

#[derive(Parser)]
#[command(name = "magray")]
#[command(about = "[AI] MAGRAY - Интеллектуальный CLI агент")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// [►] Чат с LLM моделью
    Chat {
        /// Сообщение для отправки (если не указано - интерактивный режим)
        message: Option<String>,
        /// Отключить TUI интерфейс (использовать простой CLI чат)
        #[arg(long)]
        no_tui: bool,
    },
    /// [●] Читает файл с красивой подсветкой синтаксиса
    Read {
        /// Путь к файлу
        path: String,
    },
    /// [►] Записывает содержимое в файл
    Write {
        /// Путь к файлу
        path: String,
        /// Содержимое файла
        content: String,
    },
    /// [●] Показывает содержимое директории
    List {
        /// Путь к директории (по умолчанию текущая)
        path: Option<String>,
    },
    /// [AI] Выполняет команду с помощью инструментов
    Tool {
        /// Описание действия на естественном языке
        action: String,
    },
    /// [★] Умный планировщик (без LLM на данном этапе)
    Smart(SmartCommand),
    /// [🎮] Управление GPU ускорением
    Gpu(GpuCommand),
    /// [🧠] Управление системой памяти
    Memory(MemoryCommand),
    /// [📦] Управление моделями AI
    Models(ModelsCommand),
    /// [🛠] Управление инструментами (включая MCP)
    Tools(ToolsCommand),
    /// [☑] Управление задачами
    Tasks(TasksCommand),
    /// [🤖] Multi-Agent Orchestration System
    Orchestrator(OrchestratorCommand),
    /// [🏥] Проверка здоровья системы
    Health,
    /// [📊] Показать состояние системы
    Status,
    /// [🤖] Показать статус LLM провайдеров
    LlmStatus,
    /// [📈] Показать performance метрики DI системы
    Performance,
    /// [🔒] Управление политиками (аудит/настройка)
    Policy {
        /// Показать текущие правила и источник
        #[arg(long)]
        list: bool,
        /// Применить правило Allow для shell_exec через env JSON (демо)
        #[arg(long)]
        allow_shell: bool,
    },
    /// [🖥] Запуск TUI интерфейса для Plan→Preview→Execute workflow
    Tui,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Early .env loading for OPENAI_API_KEY and other configuration
    dotenv::dotenv().ok();

    // Start events metrics aggregator (non-blocking)
    events::start_tool_metrics_aggregator().await;
    // Настройка структурированного логирования
    init_structured_logging()?;

    let cli = Cli::parse();

    // Красивое приветствие (в тестах можно отключить через MAGRAY_NO_ANIM)
    if std::env::var("MAGRAY_NO_ANIM").is_err() {
        show_welcome_animation().await?;
    }

    // Если команда не указана, запускаем интерактивный чат
    let cli = if cli.command.is_none() {
        println!("🚀 Команда не указана - запускаем Claude Code-подобный TUI чат...");

        // Для TUI режима отключаем подробное логирование
        std::env::set_var("RUST_LOG", "error");

        Cli {
            command: Some(Commands::Chat {
                message: None,
                no_tui: false,
            }),
        }
    } else {
        cli
    };

    // Проверяем наличие дефолтных моделей и предлагаем установить при необходимости
    if std::env::var("MAGRAY_SKIP_AUTO_INSTALL").is_err() {
        ensure_default_models_installed_interactive()?;
    }

    // Проверяем доступность ONNX Runtime и предлагаем установить
    if std::env::var("MAGRAY_SKIP_AUTO_INSTALL").is_err() {
        ensure_ort_installed_interactive()?;
    }

    // Автозагрузка манифестов плагинов по флагу окружения
    if std::env::var("MAGRAY_LOAD_PLUGIN_MANIFESTS")
        .ok()
        .map(|s| s == "1" || s.to_lowercase() == "true")
        .unwrap_or(false)
    {
        let home = util::magray_home();
        let mut plugins_dir = home.clone();
        plugins_dir.push("plugins");
        let mut cfg_dir = home.clone();
        cfg_dir.push("plugin-configs");
        tokio::fs::create_dir_all(&plugins_dir).await.ok();
        tokio::fs::create_dir_all(&cfg_dir).await.ok();
        let registry = tools::plugins::plugin_manager::PluginRegistry::new(plugins_dir, cfg_dir);
        let _ = registry.load_manifests_from_directory().await;
    }

    // Глобальный таймаут на выполнение команды (по умолчанию 300с)
    let top_timeout_secs: u64 = std::env::var("MAGRAY_CMD_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);

    use tokio::time::{timeout, Duration};

    let exec_fut = async {
        let cmd_name = match &cli.command {
            Some(Commands::Chat { .. }) => "chat",
            Some(Commands::Read { .. }) => "read",
            Some(Commands::Write { .. }) => "write",
            Some(Commands::List { .. }) => "list",
            Some(Commands::Tool { .. }) => "tool",
            Some(Commands::Smart(_)) => "smart",
            Some(Commands::Gpu(_)) => "gpu",
            Some(Commands::Memory(_)) => "memory",
            Some(Commands::Models(_)) => "models",
            Some(Commands::Tasks(_)) => "tasks",
            Some(Commands::Orchestrator(_)) => "orchestrator",
            Some(Commands::Health) => "health",
            Some(Commands::Status) => "status",
            Some(Commands::LlmStatus) => "llm_status",
            Some(Commands::Performance) => "performance",
            Some(Commands::Policy { .. }) => "policy",
            Some(Commands::Tools(_)) => "tools",
            Some(Commands::Tui) => "tui",
            None => "help",
        };
        tokio::spawn(events::publish(
            topics::TOPIC_INTENT,
            json!({"command": cmd_name}),
        ));

        match cli.command {
            Some(Commands::Chat { message, no_tui }) => handle_chat(message, no_tui).await?,
            Some(Commands::Read { path }) => {
                let orchestrator_service = create_orchestrator_service().await?;
                let message = format!("прочитай файл {path}");
                let response =
                    process_orchestration_service_message(&orchestrator_service, &message).await?;
                display_response(response).await;
                let _ = orchestrator_service.shutdown().await;
            }
            Some(Commands::Write { path, content }) => {
                let orchestrator_service = create_orchestrator_service().await?;
                let message = format!("создай файл {path} с содержимым: {content}");
                let response =
                    process_orchestration_service_message(&orchestrator_service, &message).await?;
                display_response(response).await;
                let _ = orchestrator_service.shutdown().await;
            }
            Some(Commands::List { path }) => {
                let orchestrator_service = create_orchestrator_service().await?;
                let message = format!("покажи содержимое папки {}", path.as_deref().unwrap_or("."));
                let response =
                    process_orchestration_service_message(&orchestrator_service, &message).await?;
                display_response(response).await;
                let _ = orchestrator_service.shutdown().await;
            }
            Some(Commands::Tool { action }) => {
                let orchestrator_service = create_orchestrator_service().await?;
                let response =
                    process_orchestration_service_message(&orchestrator_service, &action).await?;
                display_response(response).await;
                let _ = orchestrator_service.shutdown().await;
            }
            Some(Commands::Smart(cmd)) => {
                cmd.execute().await?;
            }
            Some(Commands::Gpu(gpu_command)) => {
                // Локальный таймаут 300с
                timeout(Duration::from_secs(300), gpu_command.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("GPU command timeout"))??;
            }
            Some(Commands::Memory(cmd)) => {
                timeout(Duration::from_secs(180), cmd.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("Memory command timeout"))??;
            }
            Some(Commands::Models(cmd)) => {
                timeout(Duration::from_secs(120), cmd.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("Models command timeout"))??;
            }
            Some(Commands::Tasks(cmd)) => {
                timeout(Duration::from_secs(180), cmd.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("Tasks command timeout"))??;
            }
            Some(Commands::Orchestrator(cmd)) => {
                timeout(Duration::from_secs(300), cmd.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("Orchestrator command timeout"))??;
            }
            Some(Commands::Health) => {
                // Инициализируем сервисы для health check
                let llm_client = LlmClient::from_env().ok().map(Arc::new);
                let memory_service: Option<Arc<memory::di::UnifiedContainer>> = None;

                timeout(
                    Duration::from_secs(60),
                    health_checks::run_health_checks(llm_client, memory_service),
                )
                .await
                .map_err(|_| anyhow::anyhow!("Health checks timeout"))??;
            }
            Some(Commands::Status) => {
                timeout(Duration::from_secs(60), show_system_status())
                    .await
                    .map_err(|_| anyhow::anyhow!("Status command timeout"))??;
            }
            Some(Commands::LlmStatus) => {
                timeout(Duration::from_secs(60), show_llm_status())
                    .await
                    .map_err(|_| anyhow::anyhow!("LLM status timeout"))??;
            }
            Some(Commands::Performance) => {
                timeout(Duration::from_secs(120), show_performance_metrics())
                    .await
                    .map_err(|_| anyhow::anyhow!("Performance command timeout"))??;
            }
            Some(Commands::Policy { list, allow_shell }) => {
                use common::policy::{
                    load_effective_policy, PolicyAction, PolicyDocument, PolicyRule,
                    PolicySubjectKind,
                };
                let mut home = util::magray_home();
                home.push("policy.json");
                if list {
                    let effective =
                        load_effective_policy(if home.exists() { Some(&home) } else { None });
                    println!(
                        "=== Effective Policy ===\n{}",
                        serde_json::to_string_pretty(&effective).unwrap_or_else(|_| "{}".into())
                    );
                }
                if allow_shell {
                    // Merge small override into MAGRAY_POLICY_JSON
                    let override_doc = PolicyDocument {
                        rules: vec![PolicyRule {
                            subject_kind: PolicySubjectKind::Tool,
                            subject_name: "shell_exec".into(),
                            when_contains_args: None,
                            action: PolicyAction::Allow,
                            reason: Some("cli override".into()),
                        }],
                    };
                    let json = serde_json::to_string(&override_doc)?;
                    std::env::set_var("MAGRAY_POLICY_JSON", json);
                    println!("Applied in-memory override: Allow shell_exec (MAGRAY_POLICY_JSON)\nNote: persist by writing ~/.magray/policy.json");
                }
            }
            Some(Commands::Tools(cmd)) => {
                timeout(Duration::from_secs(300), cmd.execute())
                    .await
                    .map_err(|_| anyhow::anyhow!("Tools command timeout"))??;
            }
            Some(Commands::Tui) => {
                timeout(Duration::from_secs(3600), run_tui_mode())
                    .await
                    .map_err(|_| anyhow::anyhow!("TUI mode timeout"))??;
            }
            None => {
                // Интерактивный режим
                run_interactive_mode().await?;
            }
        }
        // Publish job completion progress
        tokio::spawn(events::publish(
            topics::TOPIC_JOB_PROGRESS,
            json!({"command": cmd_name, "stage": "done"}),
        ));
        Ok::<(), anyhow::Error>(())
    };

    match timeout(Duration::from_secs(top_timeout_secs), exec_fut).await {
        Ok(res) => {
            if let Err(e) = res {
                tokio::spawn(events::publish(
                    topics::TOPIC_ERROR,
                    json!({"error": e.to_string()}),
                ));
                return Err(e);
            }
        }
        Err(_) => {
            eprintln!("[✗] Команда превысила общий таймаут {top_timeout_secs}с");
            tokio::spawn(events::publish(
                topics::TOPIC_ERROR,
                json!({"error": "global_timeout", "timeout_secs": top_timeout_secs}),
            ));
            return Err(anyhow::anyhow!("Global command timeout"));
        }
    }

    Ok(())
}

async fn run_interactive_mode() -> Result<()> {
    use std::io::{self, Write};
    use tokio::time::{timeout, Duration as TokioDuration};

    // Показываем приветствие для интерактивного режима
    println!();
    println!(
        "{} {}",
        style("[★]").green().bold(),
        style("Добро пожаловать в интерактивный режим MAGRAY!")
            .bright()
            .bold()
    );
    println!(
        "{} {}",
        style("[►]").cyan(),
        style("Введите команду или 'help' для списка команд").dim()
    );
    println!(
        "{} {}",
        style("[►]").cyan(),
        style("'exit' или 'quit' для выхода").dim()
    );
    println!();

    loop {
        // Красивый промпт
        print!("{} ", style("magray>").bright().green().bold());
        io::stdout().flush()?;

        // Читаем ввод в отдельном треде чтобы избежать зависания
        let input_future = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => Ok(input),
                Err(e) => Err(e),
            }
        });

        let input = match timeout(TokioDuration::from_secs(300), input_future).await {
            Ok(Ok(Ok(input))) => input.trim().to_string(),
            Ok(Ok(Err(e))) => {
                println!("{} Input error: {}", style("[✗]").red().bold(), e);
                continue;
            }
            Ok(Err(_)) => {
                println!("{} Input thread panicked", style("[✗]").red().bold());
                continue;
            }
            Err(_) => {
                println!(
                    "{} Input timeout after 5 minutes - exiting",
                    style("[⚠]").yellow().bold()
                );
                break;
            }
        };

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            show_goodbye_animation().await?;
            break;
        }

        if input == "clear" {
            // Очистка экрана
            let term = console::Term::stdout();
            term.clear_screen()?;
            continue;
        }

        // Парсим команду
        if let Err(e) = process_interactive_command(&input).await {
            println!("{} Error: {}", style("[✗]").red().bold(), e);
        }

        println!();
    }

    Ok(())
}

async fn process_interactive_command(input: &str) -> Result<()> {
    let args: Vec<&str> = input.split_whitespace().collect();
    if args.is_empty() {
        return Ok(());
    }

    match args[0] {
        "help" => {
            show_interactive_help();
        }
        "chat" => {
            let message = if args.len() > 1 {
                Some(args[1..].join(" "))
            } else {
                None
            };
            handle_chat(message, false).await?; // false = используем TUI по умолчанию
        }
        "read" => {
            if args.len() < 2 {
                println!("{} Usage: read <path>", style("[!]").yellow().bold());
                return Ok(());
            }
            let orchestrator_service = create_orchestrator_service().await?;
            let message = format!("прочитай файл {}", args[1]);
            let response =
                process_orchestration_service_message(&orchestrator_service, &message).await?;
            display_response(response).await;
            let _ = orchestrator_service.shutdown().await;
        }
        "write" => {
            if args.len() < 3 {
                println!(
                    "{} Usage: write <path> <content>",
                    style("[!]").yellow().bold()
                );
                return Ok(());
            }
            let orchestrator_service = create_orchestrator_service().await?;
            let content = args[2..].join(" ");
            let message = format!("создай файл {} с содержимым: {}", args[1], content);
            let response =
                process_orchestration_service_message(&orchestrator_service, &message).await?;
            display_response(response).await;
            let _ = orchestrator_service.shutdown().await;
        }
        "list" => {
            let path = if args.len() > 1 { args[1] } else { "." };
            let orchestrator_service = create_orchestrator_service().await?;
            let message = format!("покажи содержимое папки {path}");
            let response =
                process_orchestration_service_message(&orchestrator_service, &message).await?;
            display_response(response).await;
            let _ = orchestrator_service.shutdown().await;
        }
        "tool" => {
            if args.len() < 2 {
                println!("{} Usage: tool <action>", style("[!]").yellow().bold());
                return Ok(());
            }
            let action = args[1..].join(" ");
            let orchestrator_service = create_orchestrator_service().await?;
            let response =
                process_orchestration_service_message(&orchestrator_service, &action).await?;
            display_response(response).await;
            let _ = orchestrator_service.shutdown().await;
        }
        "status" => {
            show_system_status().await?;
        }
        "health" => {
            // Инициализируем сервисы для health check
            let llm_client = LlmClient::from_env().ok().map(Arc::new);
            let memory_service: Option<Arc<memory::di::UnifiedContainer>> = None;
            health_checks::run_health_checks(llm_client, memory_service).await?;
        }
        unknown => {
            println!(
                "{} Unknown command: '{}'. Type 'help' for available commands.",
                style("[!]").yellow().bold(),
                unknown
            );
        }
    }

    Ok(())
}

fn show_interactive_help() {
    println!(
        "{}",
        style("=== MAGRAY Interactive Commands ===").bold().cyan()
    );
    println!();
    println!(
        "{} {} - Chat with AI",
        style("chat").green().bold(),
        style("[message]").dim()
    );
    println!(
        "{} {} - Read file",
        style("read").green().bold(),
        style("<path>").dim()
    );
    println!(
        "{} {} - Write file",
        style("write").green().bold(),
        style("<path> <content>").dim()
    );
    println!(
        "{} {} - List directory",
        style("list").green().bold(),
        style("[path]").dim()
    );
    println!(
        "{} {} - Execute tool action",
        style("tool").green().bold(),
        style("<action>").dim()
    );
    println!("{} - Show system status", style("status").green().bold());
    println!("{} - Run health checks", style("health").green().bold());
    println!("{} - Clear screen", style("clear").green().bold());
    println!("{} - Show this help", style("help").green().bold());
    println!("{} - Exit interactive mode", style("exit").green().bold());
    println!();
}

async fn show_welcome_animation() -> Result<()> {
    let term = Term::stdout();

    // Анимация загрузки
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[|][/][-][\\]")
            .template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create spinner template: {e}");
                ProgressStyle::default_spinner()
            }),
    );

    spinner.set_message("Инициализация MAGRAY CLI...");

    // Красивая анимация инициализации
    let messages = [
        "Загрузка нейронных сетей...",
        "Подключение к квантовым процессорам...",
        "Активация искусственного интеллекта...",
        "Настройка языковой модели...",
        "Готов к работе!",
    ];

    for msg in messages.iter() {
        spinner.set_message(*msg);
        sleep(Duration::from_millis(400)).await;
    }

    spinner.finish_and_clear();

    // Красивый заголовок
    term.clear_screen()?;
    println!();
    println!(
        "{}",
        style("  ███╗   ███╗ █████╗  ██████╗ ██████╗  █████╗ ██╗   ██╗")
            .cyan()
            .bold()
    );
    println!(
        "{}",
        style("  ████╗ ████║██╔══██╗██╔════╝ ██╔══██╗██╔══██╗╚██╗ ██╔╝")
            .cyan()
            .bold()
    );
    println!(
        "{}",
        style("  ██╔████╔██║███████║██║  ███╗██████╔╝███████║ ╚████╔╝ ")
            .cyan()
            .bold()
    );
    println!(
        "{}",
        style("  ██║╚██╔╝██║██╔══██║██║   ██║██╔══██╗██╔══██║  ╚██╔╝  ")
            .cyan()
            .bold()
    );
    println!(
        "{}",
        style("  ██║ ╚═╝ ██║██║  ██║╚██████╔╝██║  ██║██║  ██║   ██║   ")
            .cyan()
            .bold()
    );
    println!(
        "{}",
        style("  ╚═╝     ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ")
            .cyan()
            .bold()
    );
    println!();
    println!(
        "       {} {}",
        style("Интеллектуальный CLI агент").bright().bold(),
        style("v0.1.0").dim()
    );
    println!("       {}", style("Powered by AI • Made with Rust").dim());
    println!();

    Ok(())
}

async fn handle_chat(message: Option<String>, no_tui: bool) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    let _term = Term::stdout();

    // В интерактивном режиме не читаем stdin (блокирует TUI)
    let stdin_message = None;

    // Инициализация LLM клиента с анимацией
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[●][◐][◑][◒][◓][●]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create LLM spinner template: {e}");
                ProgressStyle::default_spinner()
            }),
    );
    spinner.set_message("Подключение к нейронной сети...");

    let _llm_client = match LlmClient::from_env() {
        Ok(client) => {
            spinner.finish_with_message("[✓] Подключено к LLM!");
            sleep(Duration::from_millis(500)).await;
            spinner.finish_and_clear();
            client
        }
        Err(e) => {
            spinner.finish_with_message("[✗] Ошибка подключения!");
            println!();
            println!(
                "{} {}",
                style("Ошибка:").red().bold(),
                style(format!("{e}")).red()
            );
            println!();
            println!(
                "{} Создайте файл .env с настройками:",
                style("[i] Решение:").yellow().bold()
            );
            println!(
                "   {} {}",
                style("$").green(),
                style("cp .env.example .env").cyan()
            );
            println!(
                "   {} {}",
                style("#").dim(),
                style("Отредактируйте .env и укажите ваш API ключ").dim()
            );
            return Err(e);
        }
    };

    // Запускаем TUI сначала, инициализируем сервис в фоне
    if let Some(msg) = message.or(stdin_message) {
        // Для одиночных сообщений нужен сервис
        let service_future = create_orchestrator_service();
        let service = match timeout(TokioDuration::from_secs(30), service_future).await {
            Ok(Ok(service)) => service,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "OrchestrationService initialization timeout after 30 seconds"
                ))
            }
        };
        process_single_message_orchestrator(&service, &msg).await?;
        let _ = service.shutdown().await;
        return Ok(());
    }

    // Интерактивный режим - запускаем TUI немедленно
    if no_tui {
        // CLI режим - нужен сервис
        let service_future = create_orchestrator_service();
        let service = match timeout(TokioDuration::from_secs(5), service_future).await {
            Ok(Ok(service)) => service,
            Ok(Err(_)) | Err(_) => {
                println!("⚠️  AgentOrchestrator недоступен. Используется простой LLM режим...");
                services::OrchestrationService::with_llm_fallback().await?
            }
        };
        run_interactive_chat_orchestrator(&service).await?;
        let _ = service.shutdown().await;
    } else {
        // TUI режим - запускаем немедленно без вывода в консоль
        run_tui_chat_with_async_init().await?;
    }

    Ok(())
}

#[allow(dead_code)]
async fn process_single_message(orchestrator: &AgentOrchestrator, message: &str) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    // Защита от зависания с таймаутом 60 секунд
    let process_future = process_orchestrator_message(orchestrator, message);
    let response = match timeout(TokioDuration::from_secs(60), process_future).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            println!(
                "{} Message processing timeout after 60 seconds",
                style("[⚠]").yellow().bold()
            );
            return Err(anyhow::anyhow!("Message processing timeout"));
        }
    };

    display_response(response).await;
    Ok(())
}

async fn process_single_message_orchestrator(
    service: &services::OrchestrationService,
    message: &str,
) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    // Защита от зависания с таймаутом 60 секунд
    let process_future = process_orchestration_service_message(service, message);
    let response = match timeout(TokioDuration::from_secs(60), process_future).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            println!(
                "{} Message processing timeout after 60 seconds",
                style("[⚠]").yellow().bold()
            );
            return Err(anyhow::anyhow!("Message processing timeout"));
        }
    };

    display_response(response).await;
    Ok(())
}

#[allow(dead_code)]
async fn run_interactive_chat(orchestrator: &AgentOrchestrator) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    println!(
        "{} {}",
        style("[★]").green().bold(),
        style("Добро пожаловать в интерактивный режим!")
            .bright()
            .bold()
    );
    println!(
        "{} {}",
        style("[►]").cyan(),
        style("Напишите ваше сообщение или").dim()
    );
    println!(
        "{} {} {}",
        style("   ").dim(),
        style("'exit'").yellow().bold(),
        style("для выхода").dim()
    );
    println!();

    loop {
        // Красивый промпт
        print!(
            "{} {} ",
            style(USER_ICON).bright().green(),
            style("Вы:").bright().bold()
        );
        io::stdout().flush()?;

        // Читаем ввод в отдельном треде чтобы избежать зависания
        let input_future = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => Ok(input),
                Err(e) => Err(e),
            }
        });

        let input = match timeout(TokioDuration::from_secs(300), input_future).await {
            Ok(Ok(Ok(input))) => input.trim().to_string(),
            Ok(Ok(Err(e))) => {
                println!("{} Input error: {}", style("[✗]").red().bold(), e);
                continue;
            }
            Ok(Err(_)) => {
                println!("{} Input thread panicked", style("[✗]").red().bold());
                continue;
            }
            Err(_) => {
                println!(
                    "{} Input timeout after 5 minutes - exiting",
                    style("[⚠]").yellow().bold()
                );
                break;
            }
        };

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            show_goodbye_animation().await?;
            break;
        }

        // Обрабатываем сообщение с timeout защитой
        let process_future = process_orchestrator_message(orchestrator, &input);
        let response = match timeout(TokioDuration::from_secs(60), process_future).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                println!("{} Processing error: {}", style("[✗]").red().bold(), e);
                continue;
            }
            Err(_) => {
                println!(
                    "{} Message processing timeout after 60 seconds",
                    style("[⚠]").yellow().bold()
                );
                continue;
            }
        };

        display_response(response).await;
        println!();
    }

    Ok(())
}

/// Запускает TUI чат интерфейс
async fn run_tui_chat(service: &services::OrchestrationService) -> Result<()> {
    tui_chat::run_tui_chat(service).await
}

async fn run_tui_chat_with_async_init() -> Result<()> {
    tui_chat::run_tui_chat_with_async_init().await
}

async fn run_interactive_chat_orchestrator(service: &services::OrchestrationService) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    println!(
        "{} {}",
        style("[★]").green().bold(),
        style("Добро пожаловать в интерактивный чат!")
            .bright()
            .bold()
    );
    println!(
        "{} {}",
        style("[💡]").cyan(),
        style("Система готова к работе - вы можете задать любой вопрос").dim()
    );
    println!(
        "{} {} {} {} {}",
        style("   ").dim(),
        style("'exit'").yellow().bold(),
        style("или").dim(),
        style("'quit'").yellow().bold(),
        style("для выхода").dim()
    );
    println!();

    loop {
        // Красивый промпт
        print!(
            "{} {} ",
            style(USER_ICON).bright().green(),
            style("Вы:").bright().bold()
        );
        io::stdout().flush()?;

        // Читаем ввод в отдельном треде чтобы избежать зависания
        let input_future = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(_) => Ok(input),
                Err(e) => Err(e),
            }
        });

        let input = match timeout(TokioDuration::from_secs(300), input_future).await {
            Ok(Ok(Ok(input))) => input.trim().to_string(),
            Ok(Ok(Err(e))) => {
                println!("{} Input error: {}", style("[✗]").red().bold(), e);
                continue;
            }
            Ok(Err(_)) => {
                println!("{} Input thread panicked", style("[✗]").red().bold());
                continue;
            }
            Err(_) => {
                println!(
                    "{} Input timeout after 5 minutes - exiting",
                    style("[⚠]").yellow().bold()
                );
                break;
            }
        };

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            show_goodbye_animation().await?;
            break;
        }

        // Обрабатываем сообщение с timeout защитой
        let process_future = process_orchestration_service_message(service, &input);
        let response = match timeout(TokioDuration::from_secs(60), process_future).await {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                println!("{} Processing error: {}", style("[✗]").red().bold(), e);
                continue;
            }
            Err(_) => {
                println!(
                    "{} Message processing timeout after 60 seconds",
                    style("[⚠]").yellow().bold()
                );
                continue;
            }
        };

        display_response(response).await;
        println!();
    }

    Ok(())
}

async fn display_response(response: AgentResponse) {
    match response {
        AgentResponse::Chat(text) => {
            display_chat_response(&text).await;
        }
        AgentResponse::ToolExecution(result) => {
            println!("{result}");
        }
        AgentResponse::Admin(admin_response) => {
            use cli::agent_traits::AdminResponse;
            match admin_response {
                AdminResponse::SystemStats(stats) => println!("{stats}"),
                AdminResponse::HealthStatus(status) => println!("{status}"),
                AdminResponse::PerformanceMetrics(metrics) => println!("{metrics}"),
                AdminResponse::OperationResult(result) => println!("{result}"),
            }
        }
        AgentResponse::Error(error_msg) => {
            println!("{} {}", style("[✗]").red().bold(), style(error_msg).red());
        }
    }
}

async fn display_chat_response(text: &str) {
    // Анимация печати ответа
    print!(
        "{} {} ",
        style(ROBOT_ICON.get_frame(0)).bright().blue(),
        style("AI:").bright().green().bold()
    );

    // Эффект печатания
    for char in text.chars() {
        print!("{}", style(char).bright());
        if let Err(e) = io::stdout().flush() {
            eprintln!("Warning: Failed to flush stdout: {e}");
        }
        sleep(Duration::from_millis(20)).await;
    }
    println!();
}

/// Создание и инициализация AgentOrchestrator
async fn create_agent_orchestrator() -> Result<AgentOrchestrator> {
    use orchestrator::events::{AgentEventPublisher, DefaultAgentEventPublisher};
    use orchestrator::orchestrator::OrchestratorConfig;
    use orchestrator::system::SystemConfig;
    use std::sync::Arc;

    let system_config = SystemConfig::default();
    let orchestrator_config = OrchestratorConfig::default();

    // Создаем AgentEventPublisher для orchestrator
    let agent_id = uuid::Uuid::new_v4();
    let event_publisher = Arc::new(DefaultAgentEventPublisher::new(
        agent_id,
        "CLI-Orchestrator".to_string(),
        "orchestrator".to_string(),
    )) as Arc<dyn AgentEventPublisher>;

    AgentOrchestrator::new(system_config, orchestrator_config, event_publisher)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create orchestrator: {}", e))
}

/// Создание и инициализация нового AgentOrchестrator-based сервиса
async fn create_orchestrator_service() -> Result<services::OrchestrationService> {
    use indicatif::{ProgressBar, ProgressStyle};

    // Показываем прогресс инициализации
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    spinner.set_message("Инициализация агентной системы...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    // Try to create with real orchestrator first
    match services::OrchestrationService::with_orchestrator().await {
        Ok(service) => {
            spinner.finish_with_message("✅ Мульти-агентная система готова!");
            info!("OrchestrationService with AgentOrchestrator created successfully");
            Ok(service)
        }
        Err(e) => {
            spinner.set_message("⚠️  Переключение на LLM режим...");

            warn!(
                "Failed to create orchestrator service: {}, falling back to LLM-powered service",
                e
            );

            // Fall back to LLM-powered orchestration service
            match services::OrchestrationService::with_llm_fallback().await {
                Ok(service) => {
                    spinner.finish_with_message("✅ LLM режим активирован!");
                    Ok(service)
                }
                Err(fallback_error) => {
                    spinner.finish_with_message("❌ Ошибка инициализации");
                    Err(anyhow::anyhow!(
                        "Orchestrator failed: {}. Fallback failed: {}",
                        e,
                        fallback_error
                    ))
                }
            }
        }
    }
}

/// Обработка сообщения через AgentOrchestrator workflow
async fn process_orchestrator_message(
    orchestrator: &AgentOrchestrator,
    message: &str,
) -> Result<AgentResponse> {
    use orchestrator::workflow::WorkflowRequest;

    // Создаем workflow request для Intent→Plan→Execute→Critic workflow
    let workflow_request = WorkflowRequest {
        user_input: message.to_string(),
        context: None,
        priority: orchestrator::actors::TaskPriority::Normal,
        dry_run: false,
        timeout_ms: Some(60000),
        config_overrides: None,
    };

    // Выполняем полный workflow: Intent→Plan→Execute→Critic
    let workflow_result = orchestrator.execute_workflow(workflow_request).await?;

    // Конвертируем результат в AgentResponse
    let content = if workflow_result.success {
        workflow_result
            .results
            .and_then(|r| r.as_str().map(String::from))
            .unwrap_or_else(|| "Задача выполнена успешно".to_string())
    } else {
        workflow_result
            .error
            .unwrap_or_else(|| "Произошла ошибка при выполнении".to_string())
    };

    Ok(AgentResponse::Chat(content))
}

/// Обработка сообщения через OrchestrationService (legacy fallback)
#[allow(dead_code)]
async fn process_orchestration_service_message(
    service: &services::OrchestrationService,
    message: &str,
) -> Result<AgentResponse> {
    let response = service.process_user_request(message).await?;
    Ok(AgentResponse::Chat(response))
}

async fn show_goodbye_animation() -> Result<()> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◄][◁][◀][■]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create goodbye spinner template: {e}");
                ProgressStyle::default_spinner()
            }),
    );

    let goodbye_messages = [
        "Сохраняю сессию...",
        "Закрываю соединения...",
        "Очищаю память...",
        "До свидания!",
    ];

    for msg in goodbye_messages.iter() {
        spinner.set_message(*msg);
        sleep(Duration::from_millis(300)).await;
    }

    spinner.finish_and_clear();

    println!();
    println!(
        "{} {}",
        style("[★]").bright().yellow(),
        style("Спасибо за использование MAGRAY CLI!")
            .bright()
            .bold()
    );
    println!(
        "{} {}",
        style("[►]").cyan(),
        style("Увидимся в следующий раз!").cyan()
    );
    println!();

    Ok(())
}

async fn show_system_status() -> Result<()> {
    use colored::Colorize;

    let spinner = progress::ProgressBuilder::fast("Checking system status...");

    // В текущем профиле memory сервис не инициализируем напрямую
    let memory_status: Option<(String, usize, f64)> = None;

    // Проверяем LLM соединение
    let llm_status = match LlmClient::from_env() {
        Ok(_client) => {
            // Простая проверка - если клиент создался, то настройки корректны
            "Connected"
        }
        Err(_) => "Not configured",
    };

    spinner.finish_success(Some("System status checked!"));

    // Выводим статус
    println!("{}", style("=== MAGRAY System Status ===").bold().cyan());
    println!();

    // LLM Status
    let llm_icon = match llm_status {
        "Connected" => "✓".green(),
        "Connection error" => "⚠".yellow(),
        _ => "✗".red(),
    };
    println!("{} {}: {}", llm_icon, "LLM Service".bold(), llm_status);

    // Memory Status с улучшенной диагностикой
    if let Some((health, record_count, hit_rate)) = memory_status {
        let (memory_icon, status_msg) = match health.as_str() {
            "healthy" => ("✓".green(), "Healthy".to_string()),
            "degraded" => ("⚠".yellow(), "Degraded (CPU only)".to_string()),
            "cpu-only" => ("⚠".yellow(), "CPU only (no GPU)".to_string()),
            "error" => ("✗".red(), "Service error".to_string()),
            "timeout" => ("⌛".yellow(), "Initialization timeout".to_string()),
            "config-error" => ("✗".red(), "Configuration error".to_string()),
            _ => ("?".cyan(), format!("Unknown ({health})")),
        };

        if record_count > 0 || hit_rate > 0.0 {
            println!(
                "{} {}: {} ({} records, {:.1}% cache hit)",
                memory_icon,
                "Memory Service".bold(),
                status_msg,
                record_count,
                hit_rate * 100.0
            );
        } else {
            println!(
                "{} {}: {}",
                memory_icon,
                "Memory Service".bold(),
                status_msg
            );
        }
    } else {
        println!("{} {}: Not available", "✗".red(), "Memory Service".bold());
    }

    // Binary info
    let binary_size = std::env::current_exe()
        .and_then(|path| path.metadata())
        .map(|meta| meta.len())
        .unwrap_or(0);

    let version = env!("CARGO_PKG_VERSION");
    println!(
        "{} {}: v{} ({:.1} MB)",
        "ℹ".blue(),
        "Binary".bold(),
        version,
        binary_size as f64 / (1024.0 * 1024.0)
    );

    // Environment
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    println!("{} {}: {}", "ℹ".blue(), "Log Level".bold(), log_level);

    // Policy audit
    use common::policy::load_effective_policy;
    let mut home = crate::util::magray_home();
    home.push("policy.json");
    let has_file = home.exists();
    let effective = load_effective_policy(if has_file { Some(&home) } else { None });
    let rules_count = effective.rules.len();
    let src = if std::env::var("MAGRAY_POLICY_JSON")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        "env-json"
    } else if std::env::var("MAGRAY_POLICY_PATH").is_ok() || has_file {
        "file"
    } else {
        "default"
    };
    println!("🔒 Policy: {src} (rules: {rules_count})");
    // Risk aggregation
    let mut low = 0usize;
    let mut med = 0usize;
    let mut high = 0usize;
    for r in &effective.rules {
        let risk = {
            // mirror logic from infer_risk_from_reason
            let reason = r.reason.as_deref();
            if let Some(rr) = reason {
                let l = rr.to_lowercase();
                if l.contains("high") || l.contains("critical") || l.contains("danger") {
                    common::policy::RiskLevel::High
                } else if l.contains("medium") || l.contains("moderate") {
                    common::policy::RiskLevel::Medium
                } else {
                    common::policy::RiskLevel::Low
                }
            } else {
                common::policy::RiskLevel::Low
            }
        };
        match risk {
            common::policy::RiskLevel::High => high += 1,
            common::policy::RiskLevel::Medium => med += 1,
            common::policy::RiskLevel::Low => low += 1,
        }
    }
    println!("  risks: low={low} medium={med} high={high}");
    // Brief audit: list up to 5 rules
    let preview_len = effective.rules.len().min(5);
    if preview_len > 0 {
        println!("  {}", "Rules preview:".dimmed());
        for rule in effective.rules.iter().take(preview_len) {
            let when = rule
                .when_contains_args
                .as_ref()
                .map(|m| {
                    if m.is_empty() {
                        String::new()
                    } else {
                        format!(" when={m:?}")
                    }
                })
                .unwrap_or_default();
            println!(
                "  • {:?} {} -> {:?}{}",
                rule.subject_kind, rule.subject_name, rule.action, when
            );
        }
        if rules_count > preview_len {
            println!("  ... and {} more", rules_count - preview_len);
        }
    }

    // Publish health summary event
    tokio::spawn(events::publish(
        topics::TOPIC_HEALTH,
        serde_json::json!({
            "llm": llm_status,
            "policy_rules": rules_count,
            "risk": {"low": low, "medium": med, "high": high}
        }),
    ));

    println!();

    Ok(())
}

async fn show_llm_status() -> Result<()> {
    use colored::Colorize;
    use tracing::info;

    let spinner = progress::ProgressBuilder::fast("Проверка статуса LLM провайдеров...");

    info!("🤖 Проверка статуса LLM системы");

    // Пытаемся создать multi-provider клиент
    let client_result = LlmClient::from_env_multi();

    match client_result {
        Ok(client) => {
            spinner.finish_success(Some("Multi-provider система доступна!"));

            if let Some(status_report) = client.get_status_report().await {
                println!("\n{status_report}");
            } else {
                println!("\n🔧 Multi-provider система инициализирована, но статус недоступен");
            }
        }
        Err(e) => {
            spinner.finish_success(Some("Fallback к single-provider режиму"));

            match LlmClient::from_env() {
                Ok(_single_client) => {
                    println!("\n🔧 Single Provider Mode");
                    println!(
                        "{} LLM провайдер настроен и готов к работе",
                        "✓".green().bold()
                    );
                    println!();
                    println!("💡 Для активации multi-provider режима настройте:");
                    println!("  • OPENAI_API_KEY=your_openai_key");
                    println!("  • ANTHROPIC_API_KEY=your_anthropic_key");
                    println!("  • GROQ_API_KEY=your_groq_key");
                    println!("  • OLLAMA_URL=http://localhost:11434");
                    println!("  • LMSTUDIO_URL=http://localhost:1234");
                }
                Err(single_err) => {
                    println!("\n{} Ошибка конфигурации LLM", "❌".red().bold());
                    println!("Multi-provider ошибка: {e}");
                    println!("Single-provider ошибка: {single_err}");
                    println!();
                    println!("🔧 Настройте хотя бы один провайдер:");
                    println!("  LLM_PROVIDER=openai");
                    println!("  OPENAI_API_KEY=your_key_here");
                }
            }
        }
    }

    println!();
    Ok(())
}

async fn show_performance_metrics() -> Result<()> {
    use colored::Colorize;
    use tracing::info;

    let spinner = progress::ProgressBuilder::fast("Collecting performance metrics...");

    info!("📈 Initializing UnifiedAgent for performance metrics");

    // Создаем UnifiedAgent для доступа к DI системе
    let _orchestrator = match create_agent_orchestrator().await {
        Ok(agent) => {
            info!("✅ UnifiedAgent initialized successfully");
            agent
        }
        Err(e) => {
            spinner.finish_error(&format!("Failed to initialize agent: {e}"));
            println!("{} Error: {}", "✗".red(), e);
            return Ok(());
        }
    };

    spinner.finish_success(Some("Performance metrics collected!"));

    // Выводим performance отчет
    println!(
        "{}",
        style("=== MAGRAY Performance Metrics ===").bold().cyan()
    );
    println!();

    // В минимальной сборке детальные DI-метрики недоступны
    let mock_metrics = memory::DIPerformanceMetrics::default();

    if mock_metrics.resolution_time_avg_ms > 0.0 {
        println!();
        println!("{}", style("=== Detailed Analysis ===").bold().yellow());
        println!("ℹ️ Detailed DI metrics are not available in minimal build.");

        println!();
        println!(
            "{} Use 'magray performance' again to track improvements",
            "ℹ️".blue()
        );
    } else {
        println!();
        println!("{} No performance data available yet.", "ℹ️".blue());
        println!("  Try running some commands first to generate metrics.");
    }

    println!();

    Ok(())
}

fn ensure_default_models_installed_interactive() -> Result<()> {
    use colored::Colorize;
    use std::io::{stdin, stdout, Write};

    // Если в CI - не спрашиваем, ставим по умолчанию (или пропускаем если FORCE_NO_ORT)
    let non_interactive = std::env::var("CI").is_ok() || std::env::var("MAGRAY_AUTO_YES").is_ok();

    // Проверяем каталоги моделей
    let models_dir = std::path::PathBuf::from("models");
    let needs_emb = !models_dir.join("qwen3emb").exists();
    let needs_rerank = !models_dir.join("qwen3_reranker").exists();

    if !(needs_emb || needs_rerank) {
        return Ok(());
    }

    if non_interactive {
        println!(
            "{} Устанавливаю отсутствующие модели Qwen3 (non-interactive)",
            "📥".blue()
        );
        run_model_install_scripts(needs_emb, needs_rerank)?;
        return Ok(());
    }

    println!("{} Обнаружены отсутствующие модели Qwen3.", "ℹ".cyan());
    println!(
        "  - embedding: {}",
        if needs_emb {
            "missing".red()
        } else {
            "ok".green()
        }
    );
    println!(
        "  - reranker: {}",
        if needs_rerank {
            "missing".red()
        } else {
            "ok".green()
        }
    );
    print!("Установить сейчас? [Y/n]: ");
    stdout().flush().ok();

    let mut answer = String::new();
    let _ = stdin().read_line(&mut answer);
    let yes =
        answer.trim().is_empty() || matches!(answer.trim().to_lowercase().as_str(), "y" | "yes");
    if yes {
        run_model_install_scripts(needs_emb, needs_rerank)?;
    }
    Ok(())
}

fn run_model_install_scripts(emb: bool, rerank: bool) -> Result<()> {
    use std::process::Command;
    if emb {
        let _ = Command::new("python3")
            .args(["scripts/install_qwen3_onnx.py", "--component", "embedding"])
            .status();
    }
    if rerank {
        let _ = Command::new("python3")
            .args(["scripts/install_qwen3_onnx.py", "--component", "reranker"])
            .status();
    }
    Ok(())
}

fn ensure_ort_installed_interactive() -> Result<()> {
    use colored::Colorize;
    use std::io::{stdin, stdout, Write};

    if std::env::var("MAGRAY_FORCE_NO_ORT")
        .ok()
        .map(|s| s == "1" || s.to_lowercase() == "true")
        .unwrap_or(false)
    {
        return Ok(());
    }

    // Если есть ORT_DYLIB_PATH или стандартная либра — выходим
    if std::env::var("ORT_DYLIB_PATH").is_ok() {
        return Ok(());
    }
    let candidate = std::path::Path::new("scripts/onnxruntime/lib/libonnxruntime.so");
    if candidate.exists() {
        std::env::set_var("ORT_DYLIB_PATH", candidate.display().to_string());
        return Ok(());
    }

    let non_interactive = std::env::var("CI").is_ok() || std::env::var("MAGRAY_AUTO_YES").is_ok();
    if non_interactive {
        println!(
            "{} Устанавливаю ONNX Runtime (non-interactive)",
            "📥".blue()
        );
        run_ort_install_script()?;
        return Ok(());
    }

    println!("{} ONNX Runtime не найден.", "ℹ".cyan());
    print!("Установить сейчас? [Y/n]: ");
    stdout().flush().ok();
    let mut answer = String::new();
    let _ = stdin().read_line(&mut answer);
    let yes =
        answer.trim().is_empty() || matches!(answer.trim().to_lowercase().as_str(), "y" | "yes");
    if yes {
        run_ort_install_script()?;
    }
    Ok(())
}

fn run_ort_install_script() -> Result<()> {
    use std::process::Command;
    let status = Command::new("bash")
        .args(["scripts/install_onnxruntime.sh"])
        .status()?;
    if status.success() {
        // Try to set default path
        let p = std::path::Path::new("scripts/onnxruntime/lib/libonnxruntime.so");
        if p.exists() {
            std::env::set_var("ORT_DYLIB_PATH", p.display().to_string());
        }
    }
    Ok(())
}

/// Запуск TUI интерфейса для Plan→Preview→Execute workflow
async fn run_tui_mode() -> Result<()> {
    println!("🖥  Starting MAGRAY TUI Interface...");

    // Создаем TUI приложение
    let mut app = TUIApp::new().map_err(|e| anyhow::anyhow!("Failed to initialize TUI: {}", e))?;

    println!("🚀 TUI initialized successfully. Press 'q' to quit, 'h' for help.");

    // Интеграция с orchestrator (заглушка для MVP)
    // В будущем здесь будет реальная интеграция с AgentOrchestrator

    // Запускаем TUI
    if let Err(e) = app.run() {
        eprintln!("TUI error: {e}");
        return Err(anyhow::anyhow!("TUI execution failed: {}", e));
    }

    println!("👋 TUI session ended.");
    Ok(())
}
