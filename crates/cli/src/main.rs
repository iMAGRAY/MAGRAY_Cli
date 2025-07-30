use anyhow::Result;
use clap::{Parser, Subcommand};
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use llm::LlmClient;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

mod agent;
mod commands;

use agent::{UnifiedAgent, AgentResponse};
use commands::GpuCommand;


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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка логирования (скрываем для красоты)
    tracing_subscriber::fmt()
        .with_env_filter("error")
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    // Красивое приветствие
    show_welcome_animation().await?;

    match cli.command {
        Some(Commands::Chat { message }) => {
            handle_chat(message).await?;
        }
        Some(Commands::Read { path }) => {
            let llm_client = LlmClient::from_env()?;
            let agent = UnifiedAgent::new(llm_client);
            let message = format!("прочитай файл {}", path);
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::Write { path, content }) => {
            let llm_client = LlmClient::from_env()?;
            let agent = UnifiedAgent::new(llm_client);
            let message = format!("создай файл {} с содержимым: {}", path, content);
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::List { path }) => {
            let llm_client = LlmClient::from_env()?;
            let agent = UnifiedAgent::new(llm_client);
            let message = format!("покажи содержимое папки {}", path.as_deref().unwrap_or("."));
            let response = agent.process_message(&message).await?;
            display_response(response).await;
        }
        Some(Commands::Tool { action }) => {
            let llm_client = LlmClient::from_env()?;
            let agent = UnifiedAgent::new(llm_client);
            let response = agent.process_message(&action).await?;
            display_response(response).await;
        }
        Some(Commands::Smart { task }) => {
            let llm_client = LlmClient::from_env()?;
            let agent = UnifiedAgent::new(llm_client);
            let response = agent.process_message(&task).await?;
            display_response(response).await;
        }
        Some(Commands::Gpu(gpu_command)) => {
            gpu_command.execute().await?;
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
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[|][/][-][\\]")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
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
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[●][◐][◑][◒][◓][●]")
            .template("{spinner} {msg}")
            .unwrap()
    );
    spinner.set_message("Подключение к нейронной сети...");
    
    let llm_client = match LlmClient::from_env() {
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
                style(format!("{}", e)).red()
            );
            println!();
            println!("{} {}", 
                style("[i] Решение:").yellow().bold(),
                "Создайте файл .env с настройками:"
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
    let agent = UnifiedAgent::new(llm_client);

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
            println!("{}", result);
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
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(20)).await;
    }
    println!();
}







async fn show_goodbye_animation() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◄][◁][◀][■]")
            .template("{spinner} {msg}")
            .unwrap()
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
