use anyhow::Result;
use memory::migration::{MigrationManager, DatabaseStats};
use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "memory-migration")]
#[clap(about = "Инструмент миграции базы данных памяти MAGRAY", long_about = None)]
struct Cli {
    /// Путь к базе данных
    #[clap(short, long, default_value = "")]
    db_path: String,
    
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Показать статистику базы данных
    Stats,
    
    /// Выполнить миграцию
    Migrate {
        /// Выполнить миграцию без подтверждения
        #[clap(short, long)]
        force: bool,
    },
    
    /// Очистить все данные (ОПАСНО!)
    Clear {
        /// Подтвердить очистку
        #[clap(long)]
        confirm: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Настройка логирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let cli = Cli::parse();
    
    // Определяем путь к БД
    let db_path = if cli.db_path.is_empty() {
        // Используем путь по умолчанию
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ourcli")
            .join("lancedb")
    } else {
        PathBuf::from(&cli.db_path)
    };
    
    println!("База данных: {}", db_path.display());
    
    if !db_path.exists() {
        println!("❌ База данных не найдена!");
        return Ok(());
    }
    
    let manager = MigrationManager::new(&db_path)?;
    
    match cli.command {
        Commands::Stats => {
            show_stats(&manager).await?;
        }
        Commands::Migrate { force } => {
            run_migration(&manager, force).await?;
        }
        Commands::Clear { confirm } => {
            clear_database(&manager, confirm).await?;
        }
    }
    
    Ok(())
}

async fn show_stats(manager: &MigrationManager) -> Result<()> {
    println!("\n📊 Статистика базы данных:\n");
    
    let stats = manager.get_stats().await?;
    
    println!("Версия схемы: {}", stats.schema_version);
    println!("Общий размер: {:.2} МБ", stats.total_size_bytes as f64 / 1_048_576.0);
    println!();
    
    // Показываем статистику по слоям
    for layer_stats in [stats.interact, stats.insights, stats.assets] {
        println!("Слой {:?}:", layer_stats.layer);
        println!("  Записей: {}", layer_stats.record_count);
        println!("  Размер: {:.2} МБ", layer_stats.total_size_bytes as f64 / 1_048_576.0);
        
        if layer_stats.corrupted_count > 0 {
            println!("  ⚠️  Повреждённых: {}", layer_stats.corrupted_count);
        }
        
        if layer_stats.avg_embedding_dim > 0.0 {
            println!("  Средняя размерность: {:.0}", layer_stats.avg_embedding_dim);
        }
        println!();
    }
    
    Ok(())
}

async fn run_migration(manager: &MigrationManager, force: bool) -> Result<()> {
    if !force {
        println!("\n⚠️  ВНИМАНИЕ: Миграция изменит структуру базы данных!");
        println!("Рекомендуется сделать резервную копию перед продолжением.");
        println!("\nПродолжить? (y/N): ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Миграция отменена.");
            return Ok(());
        }
    }
    
    println!("\n🔄 Начинаем миграцию...\n");
    
    manager.migrate().await?;
    
    println!("\n✅ Миграция завершена успешно!");
    
    // Показываем новую статистику
    show_stats(manager).await?;
    
    Ok(())
}

async fn clear_database(manager: &MigrationManager, confirm: bool) -> Result<()> {
    if !confirm {
        println!("\n⚠️  ВНИМАНИЕ: Это удалит ВСЕ данные из базы!");
        println!("Для подтверждения добавьте флаг --confirm");
        return Ok(());
    }
    
    println!("\n🗑️  Очистка базы данных...\n");
    
    manager.clear_all_data().await?;
    
    println!("✅ База данных очищена!");
    
    Ok(())
}