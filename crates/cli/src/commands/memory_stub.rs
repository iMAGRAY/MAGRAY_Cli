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
    pub async fn execute(self) -> Result<()> { handle(self.command).await }
}

async fn handle(cmd: MemorySubcommand) -> Result<()> {
    #[cfg(not(feature = "minimal"))]
    let svc = {
        let legacy = memory::di::LegacyMemoryConfig::default();
        memory::DIMemoryService::new(legacy).await?
    };
    #[cfg(feature = "minimal")]
    {
        println!("⚠️ Функции памяти отключены в минимальном профиле. Соберите без feature=minimal.");
        match cmd { _ => return Ok(()) }
    }

    match cmd {
        MemorySubcommand::Store { text, tag } => {
            let id = svc.store(&text, tag).await?;
            println!("{} Записано с id={}", "✓".green(), id);
        }
        MemorySubcommand::Search { query, top_k } => {
            let results = svc.search(&query, top_k).await?;
            println!("{} Результатов: {}", "🔎".yellow(), results.len());
            for (i, rec) in results.iter().enumerate() {
                println!("{} {} {}", format!("{}.", i + 1).bold(), rec.id, rec.created_ms);
                if !rec.tags.is_empty() { println!("   tags: {:?}", rec.tags); }
                println!("   {}", rec.text);
            }
        }
        MemorySubcommand::Stats => {
            let h = svc.check_health().await?;
            println!("{} healthy={}, records={}", "Σ".yellow(), h.healthy, h.records);
        }
    }
    Ok(())
}