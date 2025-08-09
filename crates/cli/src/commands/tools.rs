use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use tools::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use common::{events, topics};
use common::policy::{PolicyDocument, PolicyEngine, PolicyRule, PolicySubjectKind, PolicyAction, default_document, load_from_path, merge_documents};

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
        /// Режим сухого прогона (не выполнять побочные эффекты)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Таймаут инструмента в миллисекундах
        #[arg(long)]
        timeout_ms: Option<u64>,
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
        ToolsSubcommand::Run { name, command, arg, context, dry_run, timeout_ms } => {
            let tool = registry.get(&name).ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;
            let mut args_map = std::collections::HashMap::new();
            for (k, v) in arg {
                args_map.insert(k, v);
            }
            // Load policy from ~/.magray/policy.json if exists and merge with defaults
            let mut home = crate::util::magray_home();
            home.push("policy.json");
            let effective_doc = if home.exists() {
                match load_from_path(&home) {
                    Ok(user_doc) => merge_documents(default_document(), user_doc),
                    Err(_) => default_document(),
                }
            } else {
                default_document()
            };
            let policy = PolicyEngine::from_document(effective_doc);
            // Enrich args for policy checks (domain for web_fetch, query kw for web_search)
            if name == "web_fetch" {
                if let Some(url) = args_map.get("url").cloned() {
                    let domain = url
                        .split('/')
                        .nth(2)
                        .unwrap_or("")
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !domain.is_empty() { args_map.insert("domain".into(), domain); }
                }
            } else if name == "web_search" {
                if let Some(q) = args_map.get("query").cloned() {
                    let lowered = q.to_lowercase();
                    if lowered.contains("internal") { args_map.insert("keyword".into(), "internal".into()); }
                    if lowered.contains("secret") { args_map.insert("keyword".into(), "secret".into()); }
                }
            }
            let decision = policy.evaluate_tool(&name, &args_map);
            if !decision.allowed {
                let reason = decision.matched_rule.and_then(|r| r.reason).unwrap_or_else(|| "blocked".into());
                let evt = serde_json::json!({"tool": name, "reason": reason});
                tokio::spawn(events::publish(topics::TOPIC_POLICY_BLOCK, evt));
                anyhow::bail!("Tool '{}' blocked by policy", name);
            }
            // Ask-mode: optional confirmation (non-interactive environments will deny)
            if matches!(decision.action, common::policy::PolicyAction::Ask) {
                let non_interactive = std::env::var("MAGRAY_NONINTERACTIVE").unwrap_or_default() == "true";
                if non_interactive {
                    anyhow::bail!("Tool '{}' requires confirmation (ask), but running non-interactive", name);
                }
                let auto_approve = std::env::var("MAGRAY_AUTO_APPROVE_ASK").unwrap_or_default() == "true";
                if !auto_approve {
                    // Run dry-run preview
                    let preview_input = tools::ToolInput { command: command.clone(), args: args_map.clone(), context: context.clone(), dry_run: true, timeout_ms };
                    let preview = tool.execute(preview_input).await.unwrap_or_else(|e| tools::ToolOutput { success: false, result: format!("preview error: {}", e), formatted_output: None, metadata: std::collections::HashMap::new() });
                    println!("\n=== Предпросмотр (dry-run) {} ===", name.bold());
                    if let Some(fmt) = preview.formatted_output {
                        println!("{}", fmt);
                    } else {
                        println!("{}", preview.result);
                    }
                    println!("Риск: {:?}", decision.risk);
                    // Ask user
                    use std::io::{self, Write};
                    print!("Продолжить выполнение? [y/N]: ");
                    let _ = io::stdout().flush();
                    let mut answer = String::new();
                    if io::stdin().read_line(&mut answer).is_err() { anyhow::bail!("confirmation failed"); }
                    let ans = answer.trim().to_lowercase();
                    if !(ans == "y" || ans == "yes" || ans == "д" || ans == "да") {
                        anyhow::bail!("Отменено пользователем");
                    }
                }
            }
            let input = tools::ToolInput { command, args: args_map, context, dry_run, timeout_ms };
            let output = tool.execute(input).await?;
            if output.success { println!("{} {}", "✓".green(), output.result); } else { println!("{} {}", "✗".red(), output.result); }
            // Publish event for observability (non-blocking)
            let evt = serde_json::json!({"tool": name, "success": output.success});
            tokio::spawn(events::publish(topics::TOPIC_TOOL_INVOKED, evt));
            Ok(())
        }
    }
}