use anyhow::Result;
use clap::{Parser, Subcommand};
use llm::LlmClient;
use std::io::{self, Write};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "magray")]
#[command(about = "Простой CLI чат с LLM моделями")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Чат с LLM моделью
    Chat {
        /// Сообщение для отправки (если не указано - интерактивный режим)
        message: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка логирования
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Chat { message } => {
            handle_chat(message).await?;
        }
    }

    Ok(())
}

async fn handle_chat(message: Option<String>) -> Result<()> {
    // Инициализация LLM клиента
    let llm_client = match LlmClient::from_env() {
        Ok(client) => client,
        Err(e) => {
            error!("❌ Ошибка инициализации LLM: {}", e);
            println!("💡 Создайте файл .env с настройками:");
            println!("   cp .env.example .env");
            println!("   # Отредактируйте .env и укажите ваш API ключ");
            return Err(e);
        }
    };

    info!("🤖 LLM клиент инициализирован");

    if let Some(msg) = message {
        // Одиночное сообщение
        send_message(&llm_client, &msg).await?;
    } else {
        // Интерактивный чат
        println!("🚀 Добро пожаловать в MAGRAY Chat!");
        println!("💬 Напишите ваше сообщение (или 'exit' для выхода)");
        println!();

        loop {
            print!("👤 Вы: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if input == "exit" || input == "quit" {
                println!("👋 До свидания!");
                break;
            }

            send_message(&llm_client, input).await?;
            println!();
        }
    }

    Ok(())
}

async fn send_message(client: &LlmClient, message: &str) -> Result<()> {
    print!("🤖 Думаю...");
    io::stdout().flush()?;

    match client.chat(message).await {
        Ok(response) => {
            print!("\r🤖 Ответ: ");
            println!("{}", response);
        }
        Err(e) => {
            print!("\r");
            error!("❌ Ошибка LLM: {}", e);
            println!("❌ Ошибка: {}", e);
        }
    }

    Ok(())
}
