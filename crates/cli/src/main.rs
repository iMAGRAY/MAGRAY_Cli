use anyhow::Result;
use clap::{Parser, Subcommand, CommandFactory};
use common::init_structured_logging;
use console::{style, Term};
use indicatif::ProgressStyle;
use llm::LlmClient;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod commands;
mod health_checks;
mod progress;
mod util;

#[cfg(test)]
mod status_tests;

use cli::agent_traits::AgentResponse;
use cli::agent_traits::{RequestContext, RequestProcessorTrait};
use cli::unified_agent_v2::UnifiedAgentV2;
use commands::{GpuCommand, MemoryCommand, ModelsCommand, ToolsCommand, SmartCommand, TasksCommand};

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
    /// [🏥] Проверка здоровья системы
    Health,
    /// [📊] Показать состояние системы
    Status,
    /// [🤖] Показать статус LLM провайдеров
    LlmStatus,
    /// [📈] Показать performance метрики DI системы
    Performance,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка структурированного логирования
    init_structured_logging()?;

    let cli = Cli::parse();

    // Красивое приветствие (в тестах можно отключить через MAGRAY_NO_ANIM)
    if std::env::var("MAGRAY_NO_ANIM").is_err() {
        show_welcome_animation().await?;
    }

    match cli.command {
        Some(Commands::Chat { message }) => {
            handle_chat(message).await?;
        }
        Some(Commands::Read { path }) => {
            let agent = create_unified_agent_v2().await?;
            let message = format!("прочитай файл {path}");
            let response = process_agent_message(&agent, &message).await?;
            display_response(response).await;
        }
        Some(Commands::Write { path, content }) => {
            let agent = create_unified_agent_v2().await?;
            let message = format!("создай файл {path} с содержимым: {content}");
            let response = process_agent_message(&agent, &message).await?;
            display_response(response).await;
        }
        Some(Commands::List { path }) => {
            let agent = create_unified_agent_v2().await?;
            let message = format!("покажи содержимое папки {}", path.as_deref().unwrap_or("."));
            let response = process_agent_message(&agent, &message).await?;
            display_response(response).await;
        }
        Some(Commands::Tool { action }) => {
            let agent = create_unified_agent_v2().await?;
            let response = process_agent_message(&agent, &action).await?;
            display_response(response).await;
        }
        Some(Commands::Smart(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Gpu(gpu_command)) => {
            gpu_command.execute().await?;
        }
        // Some(Commands::Memory(memory_command)) => {
        //     commands::memory::handle_memory_command(memory_command).await?;
        // }
        Some(Commands::Health) => {
            // Инициализируем сервисы для health check
            let llm_client = LlmClient::from_env().ok().map(Arc::new);
            // Создаем базовую конфигурацию памяти для health check (минимальная совместимая)
            let legacy_config = memory::di::LegacyMemoryConfig::default();
            let memory_service = memory::DIMemoryService::new(legacy_config)
                .await
                .ok()
                .map(Arc::new);

            health_checks::run_health_checks(llm_client, memory_service).await?;
        }
        Some(Commands::Memory(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Models(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Tasks(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Status) => {
            show_system_status().await?;
        }
        Some(Commands::LlmStatus) => {
            show_llm_status().await?;
        }
        Some(Commands::Performance) => {
            show_performance_metrics().await?;
        }
        Some(Commands::Tools(cmd)) => {
            cmd.execute().await?;
        }
        None => {
            // По умолчанию показываем помощь
            println!("{}", Cli::command().render_long_help());
        }
    }

    Ok(())
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
                eprintln!("Warning: Failed to create spinner template: {}", e);
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

async fn handle_chat(message: Option<String>) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    let _term = Term::stdout();

    // Проверяем, есть ли входные данные из pipe/stdin
    let mut stdin_message = None;
    if message.is_none() {
        // Проверяем синхронно, есть ли данные в stdin
        match std::thread::spawn(|| {
            use std::io::{self, Read};
            let mut input = String::new();
            match io::stdin().read_to_string(&mut input) {
                Ok(0) => None, // Нет данных
                Ok(_) => {
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                Err(_) => None, // Ошибка чтения
            }
        })
        .join()
        {
            Ok(result) => stdin_message = result,
            Err(_) => {} // Паника в треде
        }
    }

    // Инициализация LLM клиента с анимацией
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[●][◐][◑][◒][◓][●]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create LLM spinner template: {}", e);
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

    // Создаем новый Clean Architecture агент с timeout защитой
    let agent_future = create_unified_agent_v2();
    let agent = match timeout(TokioDuration::from_secs(30), agent_future).await {
        Ok(Ok(agent)) => agent,
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            return Err(anyhow::anyhow!(
                "Agent initialization timeout after 30 seconds"
            ))
        }
    };

    // Определяем, какое сообщение обрабатывать
    let final_message = message.or(stdin_message);

    if let Some(msg) = final_message {
        // Одиночное сообщение (из аргументов или stdin)
        process_single_message(&agent, &msg).await?;
    } else {
        // Интерактивный чат
        run_interactive_chat(&agent).await?;
    }

    Ok(())
}

async fn process_single_message(agent: &UnifiedAgentV2, message: &str) -> Result<()> {
    use tokio::time::{timeout, Duration as TokioDuration};

    // Защита от зависания с таймаутом 60 секунд
    let process_future = process_agent_message(agent, message);
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

async fn run_interactive_chat(agent: &UnifiedAgentV2) -> Result<()> {
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
        let process_future = process_agent_message(agent, &input);
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
                AdminResponse::SystemStats(stats) => println!("{}", stats),
                AdminResponse::HealthStatus(status) => println!("{}", status),
                AdminResponse::PerformanceMetrics(metrics) => println!("{}", metrics),
                AdminResponse::OperationResult(result) => println!("{}", result),
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
            eprintln!("Warning: Failed to flush stdout: {}", e);
        }
        sleep(Duration::from_millis(20)).await;
    }
    println!();
}

/// Создание и инициализация UnifiedAgentV2
async fn create_unified_agent_v2() -> Result<UnifiedAgentV2> {
    let mut agent = UnifiedAgentV2::new().await?;
    agent.initialize().await?;
    Ok(agent)
}

/// Обработка сообщения через UnifiedAgentV2 API
async fn process_agent_message(agent: &UnifiedAgentV2, message: &str) -> Result<AgentResponse> {
    let context = RequestContext {
        message: message.to_string(),
        session_id: "main_session".to_string(),
        metadata: std::collections::HashMap::new(),
    };

    let result = agent.process_user_request(context).await?;

    // result.response уже является AgentResponse
    Ok(result.response)
}

async fn show_goodbye_animation() -> Result<()> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◄][◁][◀][■]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create goodbye spinner template: {}", e);
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
    use memory::DIMemoryService as MemoryService;
    use std::sync::Arc;
    use tracing::{info, warn};

    let spinner = progress::ProgressBuilder::fast("Checking system status...");

    // Безопасная проверка состояния памяти в минимальной сборке
    let legacy_config = memory::di::LegacyMemoryConfig::default();
    let memory_status = match tokio::time::timeout(Duration::from_secs(5), MemoryService::new(legacy_config)).await {
        Ok(Ok(service)) => {
            let service = Arc::new(service);
            let status = service.check_health().await.ok();
            let health_str = match status {
                Some(s) if s.healthy => "healthy".to_string(),
                Some(_) => "degraded".to_string(),
                None => "error".to_string(),
            };
            Some((health_str, 0, 0.0))
        }
        Ok(Err(e)) => {
            warn!("⚠️ Memory service init error: {}", e);
            Some(("error".to_string(), 0, 0.0))
        }
        Err(_) => Some(("timeout".to_string(), 0, 0.0)),
    };

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
                println!("\n{}", status_report);
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
                    println!("Multi-provider ошибка: {}", e);
                    println!("Single-provider ошибка: {}", single_err);
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
    let agent = match create_unified_agent_v2().await {
        Ok(agent) => {
            info!("✅ UnifiedAgent initialized successfully");
            agent
        }
        Err(e) => {
            spinner.finish_error(&format!("Failed to initialize agent: {}", e));
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

    if mock_metrics.total_resolutions > 0 {
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
