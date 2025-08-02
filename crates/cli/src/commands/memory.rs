use anyhow::{anyhow, Result};
use clap::{Subcommand, Args};
use memory::{MemoryService, Layer, UnifiedMemoryAPI, MemoryContext};
use std::path::PathBuf;
use std::sync::Arc;
use colored::*;
use prettytable::{Table, row};
use indicatif::{ProgressBar, ProgressStyle};

/// Команда для управления системой памяти
#[derive(Debug, Args)]
pub struct MemoryCommand {
    #[command(subcommand)]
    command: MemorySubcommand,
}

/// Подкоманды для управления системой памяти
#[derive(Debug, Clone, Subcommand)]
enum MemorySubcommand {
    /// Показать статистику памяти
    #[command(name = "stats")]
    Stats {
        /// Показать детальную статистику
        #[arg(short, long)]
        detailed: bool,
    },

    /// Выполнить поиск в памяти
    #[command(name = "search")]
    Search {
        /// Поисковый запрос
        query: String,
        
        /// Слой для поиска (interact/insights/assets)
        #[arg(short, long)]
        layer: Option<String>,
        
        /// Количество результатов
        #[arg(short = 'k', long, default_value = "10")]
        top_k: usize,
        
        /// Минимальный score
        #[arg(short, long)]
        min_score: Option<f32>,
    },

    /// Добавить запись в память
    #[command(name = "add")]
    Add {
        /// Текст для добавления
        text: String,
        
        /// Слой (interact/insights/assets)
        #[arg(short, long, default_value = "interact")]
        layer: String,
        
        /// Теги (разделённые запятой)
        #[arg(short, long)]
        tags: Option<String>,
        
        /// Тип записи
        #[arg(short = 'k', long, default_value = "note")]
        kind: String,
    },

    /// Создать backup памяти
    #[command(name = "backup")]
    Backup {
        /// Имя backup файла
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Восстановить из backup
    #[command(name = "restore")]
    Restore {
        /// Путь к backup файлу
        backup_path: PathBuf,
    },

    /// Показать список backup файлов
    #[command(name = "list-backups")]
    ListBackups,

    /// Запустить цикл продвижения памяти
    #[command(name = "promote")]
    Promote,

    /// Проверить здоровье системы памяти
    #[command(name = "health")]
    Health {
        /// Детальная проверка
        #[arg(short, long)]
        detailed: bool,
    },

    /// Очистить кэш эмбеддингов
    #[command(name = "clear-cache")]
    ClearCache,

    /// Оптимизировать память
    #[command(name = "optimize")]
    Optimize,
    
    /// Управление лимитами памяти
    #[command(name = "limits")]
    Limits {
        /// Установить максимальное количество векторов
        #[arg(long)]
        max_vectors: Option<usize>,
        
        /// Установить максимальный размер кэша в MB
        #[arg(long)]
        max_cache_mb: Option<usize>,
        
        /// Показать текущие лимиты
        #[arg(short, long)]
        show: bool,
    },
}

impl MemoryCommand {
    pub async fn execute(self) -> Result<()> {
        handle_memory_subcommand(self.command).await
    }
}

async fn handle_memory_subcommand(cmd: MemorySubcommand) -> Result<()> {
    // Инициализируем сервис памяти
    let config = memory::default_config()?;
    let service = Arc::new(MemoryService::new(config).await?);
    let api = UnifiedMemoryAPI::new(service.clone());
    
    match cmd {
        MemorySubcommand::Stats { detailed } => {
            show_memory_stats(&api, &service, detailed).await?;
        }
        
        MemorySubcommand::Search { query, layer, top_k, min_score } => {
            search_memory(&api, &query, layer, top_k, min_score).await?;
        }
        
        MemorySubcommand::Add { text, layer, tags, kind } => {
            add_to_memory(&api, text, &layer, tags, &kind).await?;
        }
        
        MemorySubcommand::Backup { name } => {
            create_backup(&service, name).await?;
        }
        
        MemorySubcommand::Restore { backup_path } => {
            restore_backup(&service, backup_path).await?;
        }
        
        MemorySubcommand::ListBackups => {
            list_backups(&service)?;
        }
        
        MemorySubcommand::Promote => {
            run_promotion(&service).await?;
        }
        
        MemorySubcommand::Health { detailed } => {
            check_health(&api, detailed).await?;
        }
        
        MemorySubcommand::ClearCache => {
            clear_cache(&service).await?;
        }
        
        MemorySubcommand::Optimize => {
            optimize_memory(&api).await?;
        }
        
        MemorySubcommand::Limits { max_vectors, max_cache_mb, show } => {
            manage_limits(&service, max_vectors, max_cache_mb, show).await?;
        }
    }
    
    Ok(())
}

async fn show_memory_stats(api: &UnifiedMemoryAPI, service: &MemoryService, detailed: bool) -> Result<()> {
    let stats = api.get_stats().await?;
    
    println!("{}", "=== Memory System Statistics ===".bold().blue());
    println!();
    
    // Основная статистика
    println!("{}: {}", "Total records".cyan(), stats.total_records);
    println!("{}: {} ({:.1}%)", 
        "Cache hit rate".cyan(), 
        stats.cache_stats.hit_rate, 
        stats.cache_stats.hit_rate * 100.0
    );
    println!("{}: {} bytes", "Cache size".cyan(), stats.cache_stats.size_bytes);
    
    // Статистика по слоям
    println!("\n{}", "Layer Statistics:".bold());
    let mut table = Table::new();
    table.add_row(row!["Layer", "Records", "Size (KB)", "Avg Access"]);
    
    table.add_row(row![
        "Interact".yellow(),
        stats.interact_count,
        stats.interact_size / 1024,
        format!("{:.1}", stats.interact_avg_access)
    ]);
    
    table.add_row(row![
        "Insights".green(),
        stats.insights_count,
        stats.insights_size / 1024,
        format!("{:.1}", stats.insights_avg_access)
    ]);
    
    table.add_row(row![
        "Assets".blue(),
        stats.assets_count,
        stats.assets_size / 1024,
        format!("{:.1}", stats.assets_avg_access)
    ]);
    
    table.printstd();
    
    if detailed {
        // Детальная статистика
        println!("\n{}", "Performance Metrics:".bold());
        
        if let Some(metrics) = service.metrics() {
            let snapshot = metrics.snapshot();
            
            println!("{}: {} ops", "Total operations".cyan(), snapshot.total_operations);
            println!("{}: {} searches (avg: {:.2}ms)", 
                "Vector searches".cyan(),
                snapshot.vector_searches,
                snapshot.vector_search_latency_ms.avg_ms
            );
            println!("{}: {} inserts (avg: {:.2}ms)",
                "Vector inserts".cyan(),
                snapshot.vector_inserts,
                snapshot.vector_insert_latency_ms.avg_ms
            );
            
            // Promotion статистика
            println!("\n{}", "Promotion Statistics:".bold());
            println!("{}: {}", "Interact → Insights".cyan(), snapshot.promotions_interact_to_insights);
            println!("{}: {}", "Insights → Assets".cyan(), snapshot.promotions_insights_to_assets);
            println!("{}: {}", "Expired records".cyan(), snapshot.records_expired);
        }
        
        // Health статус
        let health = service.get_system_health();
        println!("\n{}: {}", "System health".bold(), 
            match health.overall_status {
                memory::health::HealthStatus::Healthy => "HEALTHY".green(),
                memory::health::HealthStatus::Degraded => "DEGRADED".yellow(),
                memory::health::HealthStatus::Unhealthy => "UNHEALTHY".red(),
                memory::health::HealthStatus::Down => "DOWN".red().bold(),
            }
        );
    }
    
    Ok(())
}

async fn search_memory(
    api: &UnifiedMemoryAPI, 
    query: &str, 
    layer: Option<String>,
    top_k: usize,
    _min_score: Option<f32>
) -> Result<()> {
    let mut options = memory::api::SearchOptions::default();
    options.limit = Some(top_k);
    
    if let Some(layer_str) = layer {
        let layer = match layer_str.as_str() {
            "interact" => Layer::Interact,
            "insights" => Layer::Insights,
            "assets" => Layer::Assets,
            _ => return Err(anyhow!("Invalid layer: {}", layer_str)),
        };
        options.layers = Some(vec![layer]);
    }
    
    // Note: min_score filter is not available in current API
    // You can filter results after retrieval if needed
    
    println!("{} '{}'...", "Searching for".cyan(), query.bold());
    
    let results = api.recall(query, options).await?;
    
    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }
    
    println!("\n{} {} results:\n", "Found".green(), results.len());
    
    for (i, result) in results.iter().enumerate() {
        println!("{}: {} (score: {:.3})", 
            format!("{}.", i + 1).bold(),
            result.text.trim(),
            result.relevance_score
        );
        
        // Показываем метаданные
        println!("   {} {:?} | {} {} | {} {}",
            "Layer:".dimmed(),
            result.layer,
            "Kind:".dimmed(),
            result.kind,
            "Tags:".dimmed(),
            result.tags.join(", ")
        );
        
        println!("   {} {} | {} {}",
            "Created:".dimmed(),
            result.created_at.format("%Y-%m-%d %H:%M"),
            "Access count:".dimmed(),
            result.access_count
        );
        
        println!();
    }
    
    Ok(())
}

async fn add_to_memory(
    api: &UnifiedMemoryAPI,
    text: String,
    layer: &str,
    tags: Option<String>,
    kind: &str,
) -> Result<()> {
    let layer = match layer {
        "interact" => Layer::Interact,
        "insights" => Layer::Insights,
        "assets" => Layer::Assets,
        _ => return Err(anyhow!("Invalid layer: {}", layer)),
    };
    
    let mut context = MemoryContext::new(kind);
    context.layer = Some(layer);
    
    if let Some(tags_str) = tags {
        let tags: Vec<String> = tags_str.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        context = context.with_tags(tags);
    }
    
    let id = api.remember(text.clone(), context).await?;
    
    println!("{} Record added successfully!", "✓".green());
    println!("{}: {}", "ID".cyan(), id);
    println!("{}: {:?}", "Layer".cyan(), layer);
    
    Ok(())
}

async fn create_backup(service: &MemoryService, name: Option<String>) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    spinner.set_message("Creating memory backup...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    
    let path = service.create_backup(name).await?;
    
    spinner.finish_with_message("✓ Backup created successfully!".green().to_string());
    println!("{}: {}", "Path".cyan(), path.display());
    
    Ok(())
}

async fn restore_backup(service: &MemoryService, backup_path: PathBuf) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );
    spinner.set_message(format!("Restoring from backup: {}", backup_path.file_name().unwrap_or_default().to_string_lossy()));
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    
    let metadata = service.restore_backup(&backup_path).await?;
    
    spinner.finish_with_message("✓ Backup restored successfully!".green().to_string());
    println!("{}: {} records", "Restored".cyan(), metadata.total_records);
    println!("{}: {}", "Created at".cyan(), metadata.created_at.format("%Y-%m-%d %H:%M:%S"));
    
    Ok(())
}

fn list_backups(service: &MemoryService) -> Result<()> {
    let backups = service.list_backups()?;
    
    if backups.is_empty() {
        println!("{}", "No backup files found.".yellow());
        return Ok(());
    }
    
    println!("{}", "Available backups:".bold());
    println!();
    
    let mut table = Table::new();
    table.add_row(row!["Name", "Created", "Records", "Size"]);
    
    for backup in backups {
        let name = backup.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        let size_mb = backup.size_bytes as f64 / (1024.0 * 1024.0);
        
        table.add_row(row![
            name,
            backup.metadata.created_at.format("%Y-%m-%d %H:%M"),
            backup.metadata.total_records,
            format!("{:.2} MB", size_mb)
        ]);
    }
    
    table.printstd();
    
    Ok(())
}

async fn run_promotion(service: &MemoryService) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.yellow} {msg}")
            .unwrap()
    );
    spinner.set_message("Running memory promotion cycle...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    
    let stats = service.run_promotion_cycle().await?;
    
    spinner.finish_with_message("✓ Promotion cycle completed!".green().to_string());
    println!("{}: {} records", "Interact → Insights".cyan(), stats.interact_to_insights);
    println!("{}: {} records", "Insights → Assets".cyan(), stats.insights_to_assets);
    println!("{}: {} records", "Expired".cyan(), stats.expired_interact + stats.expired_insights);
    println!("{}: {}ms", "Total time".cyan(), stats.total_time_ms);
    
    Ok(())
}

async fn check_health(api: &UnifiedMemoryAPI, detailed: bool) -> Result<()> {
    if detailed {
        let health = api.full_health_check().await?;
        
        println!("{}", "=== Memory System Health Check ===".bold().blue());
        println!();
        
        println!("{}: {}", "Overall status".bold(), 
            match health.overall_status {
                "healthy" => "HEALTHY".green(),
                "warning" => "WARNING".yellow(),
                "critical" => "CRITICAL".red(),
                _ => health.overall_status.normal(),
            }
        );
        
        // Компоненты
        println!("\n{}", "Component Status:".bold());
        // Note: components field not available in DetailedHealth
        // Show status based on alerts
        if health.alerts.is_empty() {
            println!("  {} All components healthy", "✓".green());
        } else {
            println!("  {} {} alert(s) active", "⚠".yellow(), health.alerts.len());
        }
        
        // Алерты
        if !health.alerts.is_empty() {
            println!("\n{}", "Alerts:".bold());
            for alert in &health.alerts {
                let severity = match alert.severity.as_str() {
                    "critical" => alert.severity.red(),
                    "warning" => alert.severity.yellow(),
                    _ => alert.severity.normal(),
                };
                println!("  [{}] {} - {}", severity, alert.component, alert.message);
            }
        }
        
        // Метрики
        if !health.metrics.is_empty() {
            println!("\n{}", "Key Metrics:".bold());
            for (metric, value) in &health.metrics {
                println!("  {}: {}", metric.cyan(), value);
            }
        }
    } else {
        let health = api.health_check().await?;
        
        let status_str = match health.status {
            "healthy" => "HEALTHY".green(),
            "warning" => "WARNING".yellow(),
            "critical" => "CRITICAL".red(),
            _ => health.status.normal(),
        };
        
        println!("{}: {}", "Memory system status".bold(), status_str);
        
        if health.alert_count > 0 {
            println!("\n{}:", "Issues found".yellow());
            println!("  • {} active alert(s)", health.alert_count);
        }
    }
    
    Ok(())
}

async fn clear_cache(service: &MemoryService) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.red} {msg}")
            .unwrap()
    );
    spinner.set_message("Clearing embedding cache...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    
    service.clear_cache().await?;
    
    spinner.finish_with_message("✓ Cache cleared successfully!".green().to_string());
    
    Ok(())
}

async fn optimize_memory(api: &UnifiedMemoryAPI) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.magenta} {msg}")
            .unwrap()
    );
    spinner.set_message("Optimizing memory system...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    
    let result = api.optimize_memory().await?;
    
    spinner.finish_with_message("✓ Optimization completed!".green().to_string());
    println!("{}: {} records", "Promoted to insights".cyan(), result.promoted_to_insights);
    println!("{}: {} records", "Promoted to assets".cyan(), result.promoted_to_assets);
    println!("{}: {} records", "Expired (Interact)".cyan(), result.expired_interact);
    println!("{}: {} records", "Expired (Insights)".cyan(), result.expired_insights);
    println!("{}: {}ms", "Total time".cyan(), result.total_time_ms);
    println!("{}: {}ms", "Promotion time".cyan(), result.promotion_time_ms);
    println!("{}: {}ms", "Index update time".cyan(), result.index_update_time_ms);
    println!("{}: {}ms", "Cleanup time".cyan(), result.cleanup_time_ms);
    
    Ok(())
}

async fn manage_limits(
    service: &MemoryService,
    max_vectors: Option<usize>,
    max_cache_mb: Option<usize>,
    show: bool,
) -> Result<()> {
    let config = service.config();
    
    if show || (max_vectors.is_none() && max_cache_mb.is_none()) {
        println!("{}", "=== Memory System Limits ===".bold().blue());
        println!();
        
        // Показываем текущие лимиты
        println!("{}: {}", "Base max vectors".cyan(), config.resource_config.base_max_vectors);
        println!("{}: {}", "Scaling max vectors".cyan(), config.resource_config.scaling_max_vectors);
        println!("{}: {} MB", 
            "Base cache size".cyan(), 
            config.resource_config.base_cache_size_bytes / (1024 * 1024)
        );
        println!("{}: {} MB", 
            "Scaling max cache size".cyan(), 
            config.resource_config.scaling_max_cache_bytes / (1024 * 1024)
        );
        println!("{}: {}%", "Target memory usage".cyan(), config.resource_config.target_memory_usage_percent);
        println!("{}: {}%", "Critical memory usage".cyan(), config.resource_config.critical_memory_usage_percent);
        
        // Показываем текущее использование
        let store = service.get_store();
        let memory_stats = store.memory_stats();
        let capacity_usage = store.capacity_usage();
        
        println!("\n{}", "Current Usage:".bold());
        println!("{}: {} / {} ({:.1}%)", 
            "Total vectors".cyan(),
            memory_stats.total_vectors,
            config.resource_config.base_max_vectors,
            (memory_stats.total_vectors as f64 / config.resource_config.base_max_vectors as f64) * 100.0
        );
        
        println!("\n{}", "Usage by layer:".bold());
        for (layer, usage_percent) in capacity_usage {
            let layer_name = match layer {
                Layer::Interact => "Interact",
                Layer::Insights => "Insights",
                Layer::Assets => "Assets",
            };
            
            let color = if usage_percent > 90.0 {
                usage_percent.to_string().red()
            } else if usage_percent > 70.0 {
                usage_percent.to_string().yellow()
            } else {
                usage_percent.to_string().green()
            };
            
            println!("  {}: {}%", layer_name.cyan(), color);
        }
        
        // Показываем здоровье кэша
        let (hits, misses, size) = service.cache_stats();
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        
        println!("\n{}", "Cache Statistics:".bold());
        println!("{}: {:.1}% ({} hits, {} misses)", 
            "Hit rate".cyan(), hit_rate, hits, misses);
        println!("{}: {} bytes", "Cache size".cyan(), size);
    }
    
    // Обновляем лимиты если указаны
    if max_vectors.is_some() || max_cache_mb.is_some() {
        if let Some(new_max_vectors) = max_vectors {
            println!("\n{} {} vectors...", 
                "Setting max vectors to".yellow(), 
                new_max_vectors
            );
            
            // TODO: Реализовать обновление лимитов в runtime
            // Это потребует добавления метода в MemoryService для обновления конфигурации
            println!("{}", "Note: Runtime limit updates not yet implemented. Restart required.".yellow());
        }
        
        if let Some(new_max_cache_mb) = max_cache_mb {
            println!("{} {} MB...", 
                "Setting max cache size to".yellow(), 
                new_max_cache_mb
            );
            
            println!("{}", "Note: Runtime limit updates not yet implemented. Restart required.".yellow());
        }
        
        println!("\n{}", "To apply new limits, update your configuration and restart.".dimmed());
    }
    
    Ok(())
}