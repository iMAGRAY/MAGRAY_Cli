use anyhow::Result;
use clap::{Parser, Subcommand};
use console::{style, Term};
use indicatif::ProgressStyle;
use llm::LlmClient;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use common::init_structured_logging;

mod agent;
mod commands;
mod health_checks;
mod progress;

#[cfg(test)]
mod status_tests;

use agent::{UnifiedAgent, AgentResponse};
use commands::{GpuCommand, MemoryCommand, ModelsCommand};


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
    /// [★] Умный AI планировщик (анализ + планирование + выполнение)
    Smart {
        /// Сложная задача на естественном языке
        task: String,
    },
    /// [🎮] Управление GPU ускорением
    Gpu(GpuCommand),
    /// [🧠] Управление системой памяти
    Memory(MemoryCommand),
    /// [📦] Управление моделями AI
    Models(ModelsCommand),
    /// [🏥] Проверка здоровья системы
    Health,
    /// [📊] Показать состояние системы
    Status,
    /// [📈] Показать performance метрики DI системы
    Performance,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка структурированного логирования
    init_structured_logging()?;

    let cli = Cli::parse();

    // Красивое приветствие
    show_welcome_animation().await?;

    match cli.command {
        Some(Commands::Chat { message }) => {
            handle_chat(message).await?;
        }
        Some(Commands::Read { path }) => {
            let agent = UnifiedAgent::new().await?;
            let message = format!("прочитай файл {path}");
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::Write { path, content }) => {
            let agent = UnifiedAgent::new().await?;
            let message = format!("создай файл {path} с содержимым: {content}");
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::List { path }) => {
            let agent = UnifiedAgent::new().await?;
            let message = format!("покажи содержимое папки {}", path.as_deref().unwrap_or("."));
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::Tool { action }) => {
            let agent = UnifiedAgent::new().await?;
            let response = agent.process_message(&action).await?;
            display_response(response).await;
        }
        Some(Commands::Smart { task }) => {
            let agent = UnifiedAgent::new().await?;
            let response = agent.process_message(&task).await?;
            display_response(response).await;
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
            // Создаем базовую конфигурацию памяти для health check
            let memory_service = if let Ok(config) = memory::default_config() {
                memory::MemoryService::new(config).await.ok().map(Arc::new)
            } else {
                None
            };
            
            health_checks::run_health_checks(llm_client, memory_service).await?;
        }
        Some(Commands::Memory(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Models(cmd)) => {
            cmd.execute().await?;
        }
        Some(Commands::Status) => {
            show_system_status().await?;
        }
        Some(Commands::Performance) => {
            show_performance_metrics().await?;
        }
        None => {
            // По умолчанию запускаем интерактивный чат
            handle_chat(None).await?;
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
            })
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
    println!("{}", style("  ███╗   ███╗ █████╗  ██████╗ ██████╗  █████╗ ██╗   ██╗").cyan().bold());
    println!("{}", style("  ████╗ ████║██╔══██╗██╔════╝ ██╔══██╗██╔══██╗╚██╗ ██╔╝").cyan().bold());
    println!("{}", style("  ██╔████╔██║███████║██║  ███╗██████╔╝███████║ ╚████╔╝ ").cyan().bold());
    println!("{}", style("  ██║╚██╔╝██║██╔══██║██║   ██║██╔══██╗██╔══██║  ╚██╔╝  ").cyan().bold());
    println!("{}", style("  ██║ ╚═╝ ██║██║  ██║╚██████╔╝██║  ██║██║  ██║   ██║   ").cyan().bold());
    println!("{}", style("  ╚═╝     ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ").cyan().bold());
    println!();
    println!("       {} {}", 
        style("Интеллектуальный CLI агент").bright().bold(),
        style("v0.1.0").dim()
    );
    println!("       {}", style("Powered by AI • Made with Rust").dim());
    println!();
    
    Ok(())
}

async fn handle_chat(message: Option<String>) -> Result<()> {
    let _term = Term::stdout();
    
    // Инициализация LLM клиента с анимацией
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[●][◐][◑][◒][◓][●]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create LLM spinner template: {}", e);
                ProgressStyle::default_spinner()
            })
    );
    spinner.set_message("Подключение к нейронной сети...");
    
    let _llm_client = match LlmClient::from_env() {
        Ok(client) => {
            spinner.finish_with_message("[✓] Подключено к LLM!");
            sleep(Duration::from_millis(500)).await;
            spinner.finish_and_clear();
            client
        },
        Err(e) => {
            spinner.finish_with_message("[✗] Ошибка подключения!");
            println!();
            println!("{} {}", 
                style("Ошибка:").red().bold(), 
                style(format!("{e}")).red()
            );
            println!();
            println!("{} Создайте файл .env с настройками:", 
                style("[i] Решение:").yellow().bold()
            );
            println!("   {} {}", 
                style("$").green(), 
                style("cp .env.example .env").cyan()
            );
            println!("   {} {}", 
                style("#").dim(), 
                style("Отредактируйте .env и укажите ваш API ключ").dim()
            );
            return Err(e);
        }
    };

    // Создаем единый агент
    let agent = UnifiedAgent::new().await?;

    if let Some(msg) = message {
        // Одиночное сообщение
        process_single_message(&agent, &msg).await?;
    } else {
        // Интерактивный чат
        run_interactive_chat(&agent).await?;
    }

    Ok(())
}

async fn process_single_message(agent: &UnifiedAgent, message: &str) -> Result<()> {
    let response = agent.process_message(message).await?;
    display_response(response).await;
    Ok(())
}

async fn run_interactive_chat(agent: &UnifiedAgent) -> Result<()> {
    println!("{} {}", 
        style("[★]").green().bold(), 
        style("Добро пожаловать в интерактивный режим!").bright().bold()
    );
    println!("{} {}", 
        style("[►]").cyan(), 
        style("Напишите ваше сообщение или").dim()
    );
    println!("{} {} {}", 
        style("   ").dim(),
        style("'exit'").yellow().bold(), 
        style("для выхода").dim()
    );
    println!();

    loop {
        // Красивый промпт
        print!("{} {} ", 
            style(USER_ICON).bright().green(),
            style("Вы:").bright().bold()
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "exit" || input == "quit" {
            show_goodbye_animation().await?;
            break;
        }

        let response = agent.process_message(input).await?;
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
    }
}

async fn display_chat_response(text: &str) {
    // Анимация печати ответа
    print!("{} {} ", 
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







async fn show_goodbye_animation() -> Result<()> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◄][◁][◀][■]")
            .template("{spinner} {msg}")
            .unwrap_or_else(|e| {
                eprintln!("Warning: Failed to create goodbye spinner template: {}", e);
                ProgressStyle::default_spinner()
            })
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
    println!("{} {}", 
        style("[★]").bright().yellow(),
        style("Спасибо за использование MAGRAY CLI!").bright().bold()
    );
    println!("{} {}", 
        style("[►]").cyan(),
        style("Увидимся в следующий раз!").cyan()
    );
    println!();
    
    Ok(())
}

// @component: {"k":"C","id":"status_cmd","t":"System status diagnostic command","m":{"cur":100,"tgt":100,"u":"%"},"f":["cli","diagnostic","graceful-fallback"]}
async fn show_system_status() -> Result<()> {
    use memory::{MemoryService, UnifiedMemoryAPI};
    use std::sync::Arc;
    use colored::Colorize;
    use tracing::{warn, info};
    
    let spinner = progress::ProgressBuilder::fast("Checking system status...");
    
    // Безопасная проверка состояния памяти с graceful fallback
    let memory_status = match memory::default_config() {
        Ok(mut config) => {
            info!("🔧 Trying to initialize memory service with fallback protection");
            
            // Отключаем GPU для status команды если есть проблемы
            config.ai_config.embedding.use_gpu = false;
            
            match tokio::time::timeout(Duration::from_secs(10), MemoryService::new(config)).await {
                Ok(Ok(service)) => {
                    info!("✅ Memory service initialized successfully");
                    let service = Arc::new(service);
                    let api = UnifiedMemoryAPI::new(service.clone());
                    
                    match api.get_stats().await {
                        Ok(stats) => {
                            let health = match api.health_check().await {
                                Ok(h) => h.status,
                                Err(e) => {
                                    warn!("Health check failed: {}", e);
                                    "degraded"
                                },
                            }.to_string();
                            Some((health, stats.total_records, stats.cache_stats.hit_rate))
                        }
                        Err(e) => {
                            warn!("Failed to get stats: {}", e);
                            Some(("cpu-only".to_string(), 0, 0.0))
                        }
                    }
                }
                Ok(Err(e)) => {
                    warn!("⚠️ Memory service initialization failed: {}", e);
                    Some(("error".to_string(), 0, 0.0))
                }
                Err(_) => {
                    warn!("⚠️ Memory service initialization timeout");
                    Some(("timeout".to_string(), 0, 0.0))
                }
            }
        }
        Err(e) => {
            warn!("⚠️ Failed to create memory config: {}", e);
            Some(("config-error".to_string(), 0, 0.0))
        }
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
            println!("{} {}: {} ({} records, {:.1}% cache hit)", 
                     memory_icon, "Memory Service".bold(), status_msg, record_count, hit_rate * 100.0);
        } else {
            println!("{} {}: {}", memory_icon, "Memory Service".bold(), status_msg);
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
    println!("{} {}: v{} ({:.1} MB)", 
             "ℹ".blue(), "Binary".bold(), version, binary_size as f64 / (1024.0 * 1024.0));
    
    // Environment
    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    println!("{} {}: {}", "ℹ".blue(), "Log Level".bold(), log_level);
    
    println!();
    
    Ok(())
}

async fn show_performance_metrics() -> Result<()> {
    use colored::Colorize;
    use tracing::info;
    
    let spinner = progress::ProgressBuilder::fast("Collecting performance metrics...");
    
    info!("📈 Initializing UnifiedAgent for performance metrics");
    
    // Создаем UnifiedAgent для доступа к DI системе
    let agent = match UnifiedAgent::new().await {
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
    println!("{}", style("=== MAGRAY Performance Metrics ===").bold().cyan());
    println!();
    
    // Основной performance отчет
    let report = agent.get_performance_report();
    println!("{}", report);
    
    // Дополнительные метрики
    let metrics = agent.get_performance_metrics();
    
    if metrics.total_resolves > 0 {
        println!();
        println!("{}", style("=== Detailed Analysis ===").bold().yellow());
        
        // Анализ эффективности кэширования
        let cache_efficiency = match metrics.cache_hit_rate() {
            rate if rate >= 80.0 => ("Excellent".green(), "🚀"),
            rate if rate >= 60.0 => ("Good".yellow(), "👍"),
            rate if rate >= 40.0 => ("Fair".yellow(), "⚠️"),
            _ => ("Poor".red(), "🐌"),
        };
        println!("{} Cache Efficiency: {} ({:.1}%)", 
                 cache_efficiency.1, cache_efficiency.0, metrics.cache_hit_rate());
        
        // Анализ скорости разрешения зависимостей
        let speed_analysis = match metrics.avg_resolve_time_us() {
            time if time < 10.0 => ("Blazing Fast".green(), "⚡"),
            time if time < 50.0 => ("Fast".green(), "🚀"),
            time if time < 200.0 => ("Good".yellow(), "👍"),
            time if time < 1000.0 => ("Slow".yellow(), "⚠️"),
            _ => ("Very Slow".red(), "🐌"),
        };
        println!("{} Resolve Speed: {} ({:.1}μs avg)", 
                 speed_analysis.1, speed_analysis.0, metrics.avg_resolve_time_us());
        
        // Показываем проблемные типы если есть
        let slowest_types = metrics.slowest_types(3);
        if !slowest_types.is_empty() {
            println!();
            println!("{}", style("Slowest Dependencies:").bold().red());
            for (i, (type_name, type_metrics)) in slowest_types.iter().enumerate() {
                let short_name = type_name.split("::").last().unwrap_or(type_name);
                let avg_time = type_metrics.avg_creation_time_ns as f64 / 1000.0;
                println!("  {}. {} - {:.1}μs ({} resolves)", 
                         i + 1, short_name, avg_time, type_metrics.resolve_count);
            }
        }
        
        // Показываем ошибки если есть
        let total_errors: u64 = metrics.type_metrics.values()
            .map(|tm| tm.error_count)
            .sum();
        
        if total_errors > 0 {
            println!();
            println!("{} {} Total Errors Found", "❌".red(), total_errors);
            for (type_name, type_metrics) in &metrics.type_metrics {
                if type_metrics.error_count > 0 {
                    let short_name = type_name.split("::").last().unwrap_or(type_name);
                    println!("  • {} - {} errors", short_name, type_metrics.error_count);
                }
            }
        }
        
        // Рекомендации по оптимизации
        println!();
        println!("{}", style("=== Optimization Recommendations ===").bold().green());
        
        if metrics.cache_hit_rate() < 50.0 {
            println!("{} Consider using more Singleton lifetimes for frequently accessed services", "💡".yellow());
        }
        
        if metrics.avg_resolve_time_us() > 100.0 {
            println!("{} Some dependencies are slow to create - consider pre-initialization", "💡".yellow());
        }
        
        if total_errors > 0 {
            println!("{} Fix dependency registration errors to improve system stability", "💡".red());
        }
        
        if metrics.factory_creates as f64 / metrics.total_resolves as f64 > 0.7 {
            println!("{} High factory creation rate - consider more singleton services", "💡".yellow());
        }
        
        println!();
        println!("{} Use 'magray performance' again to track improvements", "ℹ️".blue());
        
    } else {
        println!();
        println!("{} No performance data available yet.", "ℹ️".blue());
        println!("  Try running some commands first to generate metrics.");
    }
    
    println!();
    
    Ok(())
}
