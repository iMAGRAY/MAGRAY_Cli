use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use tools::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::fs;
use common::{events, topics};
use common::policy::{PolicyDocument, PolicyEngine, PolicyRule, PolicySubjectKind, PolicyAction, load_effective_policy};
use tools::intelligent_selector::{IntelligentToolSelector, SelectorConfig, ToolSelectionContext, TaskComplexity, UrgencyLevel, UserExpertise};

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

// Load UsageGuide overrides from file path env and JSON env. Precedence: file < JSON env
fn load_usage_guide_overrides() -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    if let Ok(path) = std::env::var("MAGRAY_TOOL_GUIDE_PATH") {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = json.as_object() {
                    for (k, v) in obj { map.insert(k.clone(), v.clone()); }
                }
            }
        }
    }
    if let Ok(json_str) = std::env::var("MAGRAY_TOOL_GUIDE_JSON") {
        if !json_str.trim().is_empty() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(obj) = json.as_object() {
                    for (k, v) in obj { map.insert(k.clone(), v.clone()); }
                }
            }
        }
    }
    map
}

fn apply_usage_guide_override(spec: &mut tools::ToolSpec, override_v: &serde_json::Value) {
    // Merge only known fields; create UsageGuide if absent
    let mut guide = spec.usage_guide.clone().unwrap_or_else(|| tools::generate_usage_guide(spec));
    if let Some(obj) = override_v.as_object() {
        if let Some(v) = obj.get("usage_title").and_then(|v| v.as_str()) { guide.usage_title = v.into(); }
        if let Some(v) = obj.get("usage_summary").and_then(|v| v.as_str()) { guide.usage_summary = v.into(); }
        if let Some(v) = obj.get("preconditions").and_then(|v| v.as_array()) { guide.preconditions = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("arguments_brief").and_then(|v| v.as_object()) {
            let mut ab = std::collections::HashMap::new();
            for (k, val) in v { if let Some(s) = val.as_str() { ab.insert(k.clone(), s.to_string()); } }
            guide.arguments_brief = ab;
        }
        if let Some(v) = obj.get("good_for").and_then(|v| v.as_array()) { guide.good_for = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("not_for").and_then(|v| v.as_array()) { guide.not_for = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("constraints").and_then(|v| v.as_array()) { guide.constraints = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("examples").and_then(|v| v.as_array()) { guide.examples = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("platforms").and_then(|v| v.as_array()) { guide.platforms = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("cost_class").and_then(|v| v.as_str()) { guide.cost_class = v.into(); }
        if let Some(v) = obj.get("latency_class").and_then(|v| v.as_str()) { guide.latency_class = v.into(); }
        if let Some(v) = obj.get("side_effects").and_then(|v| v.as_array()) { guide.side_effects = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("risk_score").and_then(|v| v.as_u64()) { guide.risk_score = v as u8; }
        if let Some(v) = obj.get("capabilities").and_then(|v| v.as_array()) { guide.capabilities = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
        if let Some(v) = obj.get("tags").and_then(|v| v.as_array()) { guide.tags = v.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect(); }
    }
    spec.usage_guide = Some(guide);
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
    List {
        /// Показать подробности (usage + ключевые поля UsageGuide)
        #[arg(long, default_value_t = false)]
        details: bool,
        /// Вывести JSON (массив ToolSpec, включая UsageGuide)
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Показать агрегированные метрики инструментов
    #[command(name = "metrics")]
    Metrics {
        /// Вывести JSON (снимок метрик)
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Подбор инструмента по запросу с объяснением скоринга
    #[command(name = "select")]
    Select {
        /// Пользовательский запрос (натуральный язык)
        #[arg(long)]
        query: String,
        /// Уровень срочности (low|normal|high|critical)
        #[arg(long, default_value_t = String::from("normal"))]
        urgency: String,
        /// Вывести JSON (объяснения и скоринг)
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Управление песочницами (FS/NET/Shell)
    #[command(name = "sandbox")]
    Sandbox {
        /// Показать эффективный конфиг (с учётом env) в JSON
        #[arg(long, default_value_t = false)]
        show: bool,
        /// Сохранить конфиг в ~/.magray/sandbox.json (env имеет приоритет при чтении)
        #[arg(long, default_value_t = false)]
        save: bool,
        /// JSON-строка с конфигом SandboxConfig (fs/net/shell)
        #[arg(long)]
        json: Option<String>,
    },

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
    pub async fn execute(self) -> Result<()> { handle_tools_command(self.command).await }
}

fn parse_kv(s: &str) -> Result<(String, String), String> {
    let (k, v) = s.split_once('=').ok_or_else(|| "arg must be in key=value format".to_string())?;
    Ok((k.to_string(), v.to_string()))
}

async fn handle_tools_command(cmd: ToolsSubcommand) -> Result<()> {
    let mut registry = ToolRegistry::new();
    preload_persisted_into_registry(&mut registry);
    let guide_overrides = load_usage_guide_overrides();

    match cmd {
        ToolsSubcommand::List { details, json } => {
            let mut specs = registry.list_tools();
            // Apply overrides
            for spec in &mut specs {
                if let Some(ov) = guide_overrides.get(&spec.name) { apply_usage_guide_override(spec, ov); }
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&specs)?);
                return Ok(());
            }
            println!("{}", "=== Registered Tools ===".bold().cyan());
            if details {
                // Effective sandbox summary from env (centralized)
                let cfg = common::sandbox_config::SandboxConfig::from_env();
                println!("  FS sandbox: {}  roots: {}", if cfg.fs.enabled { "on" } else { "off" }, if cfg.fs.roots.is_empty() { "<none>" } else { &cfg.fs.roots.join(":") });
                let net = if cfg.net.allowlist.is_empty() { "<none>".to_string() } else { cfg.net.allowlist.join(",") };
                println!("  NET allow: {}", net);
                println!("  SHELL allow: {}", if cfg.shell.allow_shell { "yes" } else { "no" });
            }
            for spec in specs {
                println!("- {}: {}", spec.name.bold(), spec.description);
                if details {
                    println!("  usage: {}", spec.usage);
                    println!("  supports_dry_run: {}", if spec.supports_dry_run { "true" } else { "false" });
                    if let Some(perms) = &spec.permissions {
                        if !perms.fs_read_roots.is_empty() { println!("  perm.fs_read: {}", perms.fs_read_roots.join(":")); }
                        if !perms.fs_write_roots.is_empty() { println!("  perm.fs_write: {}", perms.fs_write_roots.join(":")); }
                        if !perms.net_allowlist.is_empty() { println!("  perm.net_allow: {}", perms.net_allowlist.join(",")); }
                        if perms.allow_shell { println!("  perm.shell: allow"); }
                    }
                    if let Some(guide) = &spec.usage_guide {
                        if !guide.good_for.is_empty() { println!("  good_for: {}", guide.good_for.join(", ")); }
                        if !guide.tags.is_empty() { println!("  tags: {}", guide.tags.join(", ")); }
                        if !guide.capabilities.is_empty() { println!("  capabilities: {}", guide.capabilities.join(", ")); }
                        println!("  latency: {}  risk: {}", guide.latency_class, guide.risk_score);
                    }
                }
            }
            Ok(())
        }
        ToolsSubcommand::Metrics { json } => {
            let snap = events::tool_metrics_snapshot().await;
            if json {
                println!("{}", serde_json::to_string_pretty(&snap)?);
            } else {
                println!("{}", "=== Tool Metrics ===".bold().cyan());
                if let Some(obj) = snap.get("tools").and_then(|v| v.as_object()) {
                    for (tool, v) in obj {
                        let inv = v.get("invocations").and_then(|x| x.as_u64()).unwrap_or(0);
                        let ok = v.get("successes").and_then(|x| x.as_u64()).unwrap_or(0);
                        let ask = v.get("asks").and_then(|x| x.as_u64()).unwrap_or(0);
                        let deny = v.get("denies").and_then(|x| x.as_u64()).unwrap_or(0);
                        println!("- {}: invocations={}, successes={}, asks={}, denies={}", tool.bold(), inv, ok, ask, deny);
                    }
                } else {
                    println!("(нет данных)");
                }
            }
            Ok(())
        }
        ToolsSubcommand::Select { query, urgency, json } => {
            // Build selector and register all tools from registry's specs
            let mut specs = registry.list_tools();
            for spec in &mut specs {
                if let Some(ov) = guide_overrides.get(&spec.name) { apply_usage_guide_override(spec, ov); }
            }
            let selector = IntelligentToolSelector::new(SelectorConfig::default());
            for spec in specs.clone() {
                selector.register_tool(spec).await;
            }
            let urgency_level = match urgency.to_lowercase().as_str() {
                "low" => UrgencyLevel::Low,
                "high" => UrgencyLevel::High,
                "critical" => UrgencyLevel::Critical,
                _ => UrgencyLevel::Normal,
            };
            let ctx = ToolSelectionContext {
                user_query: query.clone(),
                session_context: std::collections::HashMap::new(),
                previous_tools_used: vec![],
                task_complexity: TaskComplexity::Simple,
                urgency_level,
                user_expertise: UserExpertise::Advanced,
            };
            let explained = selector.select_tools_with_explanations(&ctx).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&explained)?);
            } else {
                println!("{}", "=== Tool Selection ===".bold().cyan());
                for e in explained {
                    println!("- {}: score={:.2} (ctx {:.2}, cap {:.2}, perf {:.2})",
                        e.tool_name.bold(), e.confidence_score, e.context_match, e.capability_match, e.performance_factor);
                    if !e.matched.tags.is_empty() { println!("  tags: {}", e.matched.tags.join(", ")); }
                    if !e.matched.capabilities.is_empty() { println!("  caps: {}", e.matched.capabilities.join(", ")); }
                    if !e.matched.good_for.is_empty() { println!("  good_for: {}", e.matched.good_for.join(", ")); }
                }
            }
            Ok(())
        }
        ToolsSubcommand::Sandbox { show, save, json } => {
            let mut cfg = common::sandbox_config::SandboxConfig::from_env();
            if let Some(js) = json {
                let parsed: common::sandbox_config::SandboxConfig = serde_json::from_str(&js)?;
                cfg = parsed;
            }
            if show {
                println!("{}", serde_json::to_string_pretty(&cfg)?);
                return Ok(());
            }
            if save {
                cfg.save_to_file()?;
                println!("{} Конфиг песочниц сохранён", "✓".green());
                return Ok(());
            }
            // If neither flag provided, show help-like hint
            println!("Использование: magray tools sandbox --show | --save --json '<SandboxConfig JSON>'");
            Ok(())
        }
        ToolsSubcommand::AddMcp { name, cmd, args, remote_tool, description } => {
            let args_vec: Vec<String> = if args.trim().is_empty() { Vec::new() } else { args.split_whitespace().map(|s| s.to_string()).collect() };
            registry.register_mcp_tool(&name, cmd.clone(), args_vec.clone(), remote_tool.clone(), description.clone());
            upsert_mcp_config(McpToolConfig { name: name.clone(), cmd, args: args_vec, remote_tool, description })?;
            println!("{} Зарегистрирован MCP инструмент: {}", "✓".green(), name.bold());
            Ok(())
        }
        ToolsSubcommand::Run { name, command, arg, context, dry_run, timeout_ms } => {
            let tool = registry.get(&name).ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;
            let mut args_map = std::collections::HashMap::new();
            for (k, v) in arg { args_map.insert(k, v); }
            // Load effective policy
            let mut home = crate::util::magray_home();
            home.push("policy.json");
            let effective = load_effective_policy(if home.exists() { Some(&home) } else { None });
            let policy = PolicyEngine::from_document(effective);
            // Enrich policy args
            if name == "web_fetch" {
                if let Some(url) = args_map.get("url").cloned() {
                    let domain = url.split('/').nth(2).unwrap_or("").split(':').next().unwrap_or("").to_string();
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

            // Dynamic guard based on UsageGuide overrides/spec
            let mut require_ask_due_to_guide = false;
            let mut spec = tool.spec();
            if let Some(ov) = guide_overrides.get(&name) { apply_usage_guide_override(&mut spec, ov); }
            if matches!(decision.action, common::policy::PolicyAction::Allow) && decision.matched_rule.is_none() {
                if let Some(guide) = &spec.usage_guide {
                    let high_risk = guide.risk_score >= 4;
                    let has_side_effects = !guide.side_effects.is_empty();
                    if high_risk || has_side_effects { require_ask_due_to_guide = true; }
                }
            }

            if matches!(decision.action, common::policy::PolicyAction::Deny) {
                let reason = decision.matched_rule.and_then(|r| r.reason).unwrap_or_else(|| "blocked".into());
                let evt = serde_json::json!({"tool": name, "reason": reason});
                tokio::spawn(events::publish(topics::TOPIC_POLICY_BLOCK, evt));
                anyhow::bail!("Tool '{}' blocked by policy", name);
            }
            if matches!(decision.action, common::policy::PolicyAction::Ask) || require_ask_due_to_guide {
                let non_interactive = std::env::var("MAGRAY_NONINTERACTIVE").unwrap_or_default() == "true";
                if non_interactive { anyhow::bail!("Tool '{}' requires confirmation (ask), but running non-interactive", name); }
                let auto_approve = std::env::var("MAGRAY_AUTO_APPROVE_ASK").unwrap_or_default() == "true";
                if !auto_approve {
                    let preview_input = tools::ToolInput { command: command.clone(), args: args_map.clone(), context: context.clone(), dry_run: true, timeout_ms };
                    let preview = tool.execute(preview_input).await.unwrap_or_else(|e| tools::ToolOutput { success: false, result: format!("preview error: {}", e), formatted_output: None, metadata: std::collections::HashMap::new() });
                    println!("\n=== Предпросмотр (dry-run) {} ===", name.bold());
                    if let Some(fmt) = preview.formatted_output { println!("{}", fmt); } else { println!("{}", preview.result); }
                    if require_ask_due_to_guide { println!("Требуется подтверждение по UsageGuide (risk/side_effects)"); }
                    use std::io::{self, Write};
                    print!("Продолжить выполнение? [y/N]: ");
                    let _ = io::stdout().flush();
                    let mut answer = String::new();
                    if io::stdin().read_line(&mut answer).is_err() { anyhow::bail!("confirmation failed"); }
                    let ans = answer.trim().to_lowercase();
                    if !(ans == "y" || ans == "yes" || ans == "д" || ans == "да") { anyhow::bail!("Отменено пользователем"); }
                }
            }
            let input = tools::ToolInput { command, args: args_map, context, dry_run, timeout_ms };
            let output = tool.execute(input).await?;
            if output.success { println!("{} {}", "✓".green(), output.result); } else { println!("{} {}", "✗".red(), output.result); }
            let evt = serde_json::json!({"tool": name, "success": output.success});
            tokio::spawn(events::publish(topics::TOPIC_TOOL_INVOKED, evt));
            Ok(())
        }
    }
}