use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::process;

mod commands;
mod config;
mod ui;

#[derive(Parser)]
#[command(
    name = "{{project_name}}",
    about = "{{description}}",
    version,
    author,
    long_about = None,
    after_help = "Use {{project_name}} --help for more detailed help"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress all output
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project
    Init {
        /// Project name
        #[arg(value_name = "NAME")]
        name: Option<String>,

        /// Project template
        #[arg(short, long, default_value = "default")]
        template: String,
    },

    /// Create various project components
    #[command(subcommand)]
    Create(CreateCommands),

    /// Analyze code and project structure
    #[command(subcommand)]
    Analyze(AnalyzeCommands),

    /// Configure {{project_name}} settings
    Config {
        /// Configuration key
        key: Option<String>,

        /// Configuration value
        value: Option<String>,

        /// List all configurations
        #[arg(short, long)]
        list: bool,
    },
}

#[derive(Subcommand)]
enum CreateCommands {
    /// Create a new module
    Module {
        /// Module name
        name: String,
    },
    /// Create a new test
    Test {
        /// Test name
        name: String,
    },
}

#[derive(Subcommand)]
enum AnalyzeCommands {
    /// Analyze code complexity
    Complexity {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: String,
    },
    /// Analyze dependencies
    Deps {
        /// Show dependency tree
        #[arg(short, long)]
        tree: bool,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet);

    // Load configuration
    let _config = config::load()?;

    match cli.command {
        Some(Commands::Init { name, template }) => {
            commands::init::run(name, template).await?;
        }
        Some(Commands::Create(cmd)) => match cmd {
            CreateCommands::Module { name } => {
                commands::create::module(&name).await?;
            }
            CreateCommands::Test { name } => {
                commands::create::test(&name).await?;
            }
        },
        Some(Commands::Analyze(cmd)) => match cmd {
            AnalyzeCommands::Complexity { path } => {
                commands::analyze::complexity(&path).await?;
            }
            AnalyzeCommands::Deps { tree } => {
                commands::analyze::deps(tree).await?;
            }
        },
        Some(Commands::Config { key, value, list }) => {
            commands::config::run(key, value, list).await?;
        }
        None => {
            // Show interactive menu if no command provided
            ui::interactive_menu().await?;
        }
    }

    Ok(())
}

fn init_logging(verbosity: u8, quiet: bool) {
    use tracing_subscriber::EnvFilter;

    if quiet {
        return;
    }

    let filter = match verbosity {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}