use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use tools::ToolRegistry;

#[derive(Debug, Args)]
pub struct ToolsCommand {
    #[command(subcommand)]
    command: ToolsSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ToolsSubcommand {
    /// Показать зарегистрированные инструменты
    #[command(name = "list")]
    List,

    /// Зарегистрировать MCP инструмент (stdio)
    #[command(name = "add-mcp")]
    AddMcp {
        /// Имя инструмента в MAGRAY
        #[arg(long)]
        name: String,
        /// Команда запуска MCP сервера
        #[arg(long)]
        cmd: String,
        /// Аргументы команды (через пробел, необязательно)
        #[arg(long, default_value_t = String::new())]
        args: String,
        /// Имя удалённого MCP инструмента
        #[arg(long)]
        remote_tool: String,
        /// Описание инструмента
        #[arg(long, default_value_t = String::from("MCP proxied tool"))]
        description: String,
    },
}

impl ToolsCommand {
    pub async fn execute(self) -> Result<()> {
        handle_tools_command(self.command).await
    }
}

async fn handle_tools_command(cmd: ToolsSubcommand) -> Result<()> {
    let mut registry = ToolRegistry::new();

    match cmd {
        ToolsSubcommand::List => {
            let specs = registry.list_tools();
            println!("{}", "=== Registered Tools ===".bold().cyan());
            for spec in specs {
                println!("- {}: {}", spec.name.bold(), spec.description);
            }
            Ok(())
        }
        ToolsSubcommand::AddMcp { name, cmd, args, remote_tool, description } => {
            let args_vec: Vec<String> = if args.trim().is_empty() {
                Vec::new()
            } else {
                args.split_whitespace().map(|s| s.to_string()).collect()
            };
            registry.register_mcp_tool(&name, cmd, args_vec, remote_tool, description);
            println!("{} Зарегистрирован MCP инструмент: {}", "✓".green(), name.bold());
            Ok(())
        }
    }
}