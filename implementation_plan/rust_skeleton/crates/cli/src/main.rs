use anyhow::Result;
use clap::{Parser, Subcommand};
use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use llm::LlmClient;
use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;
use tokio_stream::{StreamExt, wrappers::IntervalStream};
use tools::{ToolRegistry, ToolInput, SmartRouter};


// Анимированные ASCII иконки
struct AnimatedIcon {
    frames: &'static [&'static str],
    current: std::sync::atomic::AtomicUsize,
}

impl AnimatedIcon {
    const fn new(frames: &'static [&'static str]) -> Self {
        Self {
            frames,
            current: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    fn next_frame(&self) -> &'static str {
        let current = self.current.load(std::sync::atomic::Ordering::Relaxed);
        let next = (current + 1) % self.frames.len();
        self.current.store(next, std::sync::atomic::Ordering::Relaxed);
        self.frames[current]
    }
    
    fn get_frame(&self, index: usize) -> &'static str {
        self.frames[index % self.frames.len()]
    }
}

static ROBOT_ICON: AnimatedIcon = AnimatedIcon::new(&["[AI]", "[▲I]", "[●I]", "[♦I]"]);
static THINKING_ICON: AnimatedIcon = AnimatedIcon::new(&["[●  ]", "[●● ]", "[●●●]", "[ ●●]", "[  ●]", "[   ]"]);
static USER_ICON: &str = "[►]";
static SUCCESS_ICON: &str = "[✓]";
static ERROR_ICON: &str = "[✗]";
static INFO_ICON: &str = "[i]";
static LOADING_ICON: AnimatedIcon = AnimatedIcon::new(&["[|]", "[/]", "[-]", "[\\]"]);

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
            handle_file_read(&path).await?;
        }
        Some(Commands::Write { path, content }) => {
            handle_file_write(&path, &content).await?;
        }
        Some(Commands::List { path }) => {
            handle_dir_list(path.as_deref().unwrap_or(".")).await?;
        }
        Some(Commands::Tool { action }) => {
            handle_tool_action(&action).await?;
        }
        Some(Commands::Smart { task }) => {
            handle_smart_task(&task).await?;
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

    if let Some(msg) = message {
        // Одиночное сообщение
        send_message_with_animation(&llm_client, &msg).await?;
    } else {
        // Интерактивный чат
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
            .tick_chars("[◐][◓][◑][◒]")
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
                            style(ROBOT_ICON.get_frame(0)).bright().blue(),
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
                            style(ERROR_ICON).red(),
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

async fn handle_file_read(path: &str) -> Result<()> {
    let registry = ToolRegistry::new();
    let tool = registry.get("file_read").unwrap();
    
    let mut args = HashMap::new();
    args.insert("path".to_string(), path.to_string());
    
    let input = ToolInput {
        command: "file_read".to_string(),
        args,
        context: None,
    };
    
    let output = tool.execute(input).await?;
    
    if output.success {
        if let Some(formatted) = output.formatted_output {
            println!("{}", formatted);
        } else {
            println!("{}", output.result);
        }
    } else {
        println!("{} {}", 
            style(ERROR_ICON).red(),
            style(output.result).red()
        );
    }
    
    Ok(())
}

async fn handle_file_write(path: &str, content: &str) -> Result<()> {
    let registry = ToolRegistry::new();
    let tool = registry.get("file_write").unwrap();
    
    let mut args = HashMap::new();
    args.insert("path".to_string(), path.to_string());
    args.insert("content".to_string(), content.to_string());
    
    let input = ToolInput {
        command: "file_write".to_string(),
        args,
        context: None,
    };
    
    let output = tool.execute(input).await?;
    
    if output.success {
        if let Some(formatted) = output.formatted_output {
            println!("{}", formatted);
        } else {
            println!("{} {}", 
                style(SUCCESS_ICON).green(),
                style(output.result).green()
            );
        }
    } else {
        println!("{} {}", 
            style(ERROR_ICON).red(),
            style(output.result).red()
        );
    }
    
    Ok(())
}

async fn handle_dir_list(path: &str) -> Result<()> {
    let registry = ToolRegistry::new();
    let tool = registry.get("dir_list").unwrap();
    
    let mut args = HashMap::new();
    args.insert("path".to_string(), path.to_string());
    
    let input = ToolInput {
        command: "dir_list".to_string(),
        args,
        context: None,
    };
    
    let output = tool.execute(input).await?;
    
    if output.success {
        if let Some(formatted) = output.formatted_output {
            println!("{}", formatted);
        } else {
            println!("{}", output.result);
        }
    } else {
        println!("{} {}", 
            style(ERROR_ICON).red(),
            style(output.result).red()
        );
    }
    
    Ok(())
}

async fn handle_tool_action(action: &str) -> Result<()> {
    // Используем тот же AI планировщик что и в smart команде
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◐][◓][◑][◒]")
            .template("{spinner} {msg}")
            .unwrap()
    );
    
    spinner.set_message("Запускаю AI планировщик...");
    
    let llm_client = match LlmClient::from_env() {
        Ok(client) => {
            spinner.finish_with_message("[★] AI планировщик активирован");
            client
        },
        Err(e) => {
            spinner.finish_with_message("[✗] AI планировщик недоступен");
            eprintln!("{} {}", 
                style(ERROR_ICON).red(),
                style(format!("Ошибка: {}", e)).red()
            );
            eprintln!("{} Требуется настройка .env файла с LLM API ключом", 
                style("[i]").yellow()
            );
            return Err(e.into());
        }
    };
    
    let smart_router = SmartRouter::new(llm_client);
    let result = smart_router.process_smart_request(action).await?;
    println!("{}", result);
    
    Ok(())
}

// Fallback функция для случаев когда AI недоступен
async fn handle_smart_task(task: &str) -> Result<()> {
    // Принудительное использование AI планировщика
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("[◐][◓][◑][◒]")
            .template("{spinner} {msg}")
            .unwrap()
    );
    
    spinner.set_message("Запускаю AI планировщик...");
    
    let llm_client = match LlmClient::from_env() {
        Ok(client) => {
            spinner.finish_with_message("[★] AI планировщик активирован");
            client
        },
        Err(e) => {
            spinner.finish_with_message("[✗] AI планировщик недоступен");
            eprintln!("{} {}", 
                style(ERROR_ICON).red(),
                style(format!("Ошибка: {}", e)).red()
            );
            eprintln!("{} Для умного режима требуется настройка .env файла", 
                style("[i]").yellow()
            );
            eprintln!("{} Используйте команду 'tool' для простого режима", 
                style("[i]").yellow()
            );
            return Err(e.into());
        }
    };
    
    let smart_router = SmartRouter::new(llm_client);
    
    let result = smart_router.process_smart_request(task).await?;
    println!("{}", result);
    
    Ok(())
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
