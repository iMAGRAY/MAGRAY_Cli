use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use tools::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct McpToolConfig {
    name: String,
    cmd: String,
    args: Vec<String>,
    remote_tool: String,
    description: String,
}

fn tools_registry_path() -> std::path::PathBuf {
    let mut dir = crate::util::magray_home();
    dir.push("tools.json");
    dir
}

fn load_persisted_mcp() -> Vec<McpToolConfig> {
    let path = tools_registry_path();
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str::<Vec<McpToolConfig>>(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_persisted_mcp(configs: &[McpToolConfig]) -> Result<()> {
    let path = tools_registry_path();
    let json = serde_json::to_string_pretty(configs)?;
    fs::write(path, json)?;
    Ok(())
}

fn upsert_mcp_config(new_item: McpToolConfig) -> Result<()> {
    let mut items = load_persisted_mcp();
    if let Some(idx) = items.iter().position(|i| i.name == new_item.name) {
        items[idx] = new_item;
    } else {
        items.push(new_item);
    }
    save_persisted_mcp(&items)
}

fn preload_persisted_into_registry(registry: &mut ToolRegistry) {
    for cfg in load_persisted_mcp() {
        registry.register_mcp_tool(&cfg.name, cfg.cmd.clone(), cfg.args.clone(), cfg.remote_tool.clone(), cfg.description.clone());
    }
}

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

    /// Выполнить инструмент по имени: --name <tool> --command <cmd> --arg k=v --arg x=y
    #[command(name = "run")]
    Run {
        /// Имя инструмента
        #[arg(long)]
        name: String,
        /// Команда для инструмента
        #[arg(long)]
        command: String,
        /// Аргументы в формате key=value (можно несколько раз)
        #[arg(long, num_args=0.., value_parser=parse_kv)]
        arg: Vec<(String, String)>,
        /// Необязательный контекст
        #[arg(long)]
        context: Option<String>,
    },
}

impl ToolsCommand {
    pub async fn execute(self) -> Result<()> {
        handle_tools_command(self.command).await
    }
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| "arg must be in key=value format".to_string())?;
    Ok((k.to_string(), v.to_string()))
}

async fn handle_tools_command(cmd: ToolsSubcommand) -> Result<()> {
    let mut registry = ToolRegistry::new();
    preload_persisted_into_registry(&mut registry);

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
            registry.register_mcp_tool(&name, cmd.clone(), args_vec.clone(), remote_tool.clone(), description.clone());
            upsert_mcp_config(McpToolConfig { name: name.clone(), cmd, args: args_vec, remote_tool, description })?;
            println!("{} Зарегистрирован MCP инструмент: {}", "✓".green(), name.bold());
            Ok(())
        }
        ToolsSubcommand::Run { name, command, arg, context } => {
            let tool = registry.get(&name).ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;
            let mut args_map = std::collections::HashMap::new();
            for (k, v) in arg {
                args_map.insert(k, v);
            }
            let input = tools::ToolInput { command, args: args_map, context };
            let output = tool.execute(input).await?;
            if output.success { println!("{} {}", "✓".green(), output.result); } else { println!("{} {}", "✗".red(), output.result); }
            Ok(())
        }
    }
}