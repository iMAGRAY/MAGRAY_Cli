use anyhow::Result;
use clap::{Parser, Subcommand};
use console::{style, Term, Emoji};
use indicatif::{ProgressBar, ProgressStyle};
use llm::LlmClient;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tracing::error;

static ROBOT: Emoji<'_, '_> = Emoji("🤖", "");
static SPARKLES: Emoji<'_, '_> = Emoji("✨", "");
static ROCKET: Emoji<'_, '_> = Emoji("🚀", "");
static GEAR: Emoji<'_, '_> = Emoji("⚙️", "");
static BRAIN: Emoji<'_, '_> = Emoji("🧠", "");
static LIGHTNING: Emoji<'_, '_> = Emoji("⚡", "");

#[derive(Parser)]
#[command(name = "magray")]
#[command(about = "🤖 MAGRAY - Интеллектуальный CLI агент")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 💬 Чат с LLM моделью
    Chat {
        /// Сообщение для отправки (если не указано - интерактивный режим)
        message: Option<String>,
    },
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
            .tick_chars("⠁⠃⠇⡇⡆⡤⡴⡼⢼⢸⢹⢻⢿⡿⣿⣾⣽⣻⣯⣟⣯⣿")
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
            .tick_chars("🌍🌎🌏")
            .template("{spinner} {msg}")
            .unwrap()
    );
    spinner.set_message("Подключение к нейронной сети...");
    
    let llm_client = match LlmClient::from_env() {
        Ok(client) => {
            spinner.finish_with_message("✅ Подключено к LLM!");
            sleep(Duration::from_millis(500)).await;
            spinner.finish_and_clear();
            client
        },
        Err(e) => {
            spinner.finish_with_message("❌ Ошибка подключения!");
            println!();
            println!("{} {}", 
                style("Ошибка:").red().bold(), 
                style(format!("{}", e)).red()
            );
            println!();
            println!("{} {}", 
                style("💡 Решение:").yellow().bold(),
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

    if let Some(msg) = message {
        // Одиночное сообщение
        send_message_with_animation(&llm_client, &msg).await?;
    } else {
        // Интерактивный чат
        println!("{} {}", 
            ROCKET, 
            style("Добро пожаловать в интерактивный режим!").bright().bold()
        );
        println!("{} {}", 
            style("💬").cyan(), 
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
                style("👤").bright(),
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

            send_message_with_animation(&llm_client, input).await?;
            println!();
        }
    }

    Ok(())
}

async fn send_message_with_animation(client: &LlmClient, message: &str) -> Result<()> {
    // Анимация "думаю"
    let thinking_spinner = ProgressBar::new_spinner();
    thinking_spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("🤔💭🧠⚡🔮✨🎯🚀")
            .template("{spinner} {msg}")
            .unwrap()
    );
    
    let thinking_messages = [
        "Анализирую ваш запрос...",
        "Обрабатываю информацию...",
        "Генерирую ответ...",
        "Финальная обработка...",
    ];
    
    thinking_spinner.set_message(thinking_messages[0]);
    
    // Запускаем LLM запрос в фоне
    let client_clone = client.clone();
    let message_clone = message.to_string();
    let mut llm_task = tokio::spawn(async move {
        client_clone.chat(&message_clone).await
    });
    
    // Анимируем сообщения пока ждем
    let mut message_idx = 0;
    let mut interval = IntervalStream::new(tokio::time::interval(Duration::from_millis(800)));
    
    loop {
        tokio::select! {
            result = &mut llm_task => {
                thinking_spinner.finish_and_clear();
                
                match result? {
                    Ok(response) => {
                        // Анимация печати ответа
                        print!("{} {} ", 
                            ROBOT, 
                            style("AI:").bright().green().bold()
                        );
                        
                        // Эффект печатания
                        for char in response.chars() {
                            print!("{}", style(char).bright());
                            io::stdout().flush()?;
                            sleep(Duration::from_millis(20)).await;
                        }
                        println!();
                        
                        return Ok(());
                    }
                    Err(e) => {
                        println!("{} {} {}", 
                            style("❌").red(),
                            style("Ошибка:").red().bold(),
                            style(format!("{}", e)).red()
                        );
                        return Err(e.into());
                    }
                }
            }
            _ = interval.next() => {
                message_idx = (message_idx + 1) % thinking_messages.len();
                thinking_spinner.set_message(thinking_messages[message_idx]);
            }
        }
    }
}

async fn show_goodbye_animation() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("👋✨🌟💫⭐🌠🎆🎇")
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
        style("👋").bright(),
        style("Спасибо за использование MAGRAY CLI!").bright().bold()
    );
    println!("{} {}", 
        SPARKLES,
        style("Увидимся в следующий раз!").cyan()
    );
    println!();
    
    Ok(())
}
