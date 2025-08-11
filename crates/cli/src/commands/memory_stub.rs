use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;

#[derive(Debug, Args)]
pub struct MemoryCommand {
    #[command(subcommand)]
    command: MemorySubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum MemorySubcommand {
    /// Сохранить текст в локальную память
    #[command(name = "store")]
    Store {
        /// Текст для сохранения
        #[arg(long)]
        text: String,
        /// Повторяющиеся флаги для тегов
        #[arg(long, num_args=0..)]
        tag: Vec<String>,
    },
    /// Поиск по локальной памяти
    #[command(name = "search")]
    Search {
        /// Запрос (подстрока)
        #[arg(long)]
        query: String,
        /// Количество результатов
        #[arg(long, default_value_t = 10)]
        top_k: usize,
    },
    /// Показать статистику памяти
    #[command(name = "stats")]
    Stats,
}

impl MemoryCommand {
    pub async fn execute(self) -> Result<()> {
        handle(self.command).await
    }
}

async fn handle(cmd: MemorySubcommand) -> Result<()> {
    #[cfg(feature = "minimal")]
    {
        println!(
            "⚠️ Функции памяти отключены в минимальном профиле. Соберите без feature=minimal."
        );
        return Ok(());
    }

    #[cfg(not(feature = "minimal"))]
    {
        println!(
            "⚠️ Путь memory_stub недоступен в текущей конфигурации. Используйте orchestrated путь."
        );
        match cmd {
            _ => return Ok(()),
        }
    }
}
