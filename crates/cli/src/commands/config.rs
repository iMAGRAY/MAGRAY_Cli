#![allow(dead_code)] // Allow unused code during development

use anyhow::Result;
use clap::{Args, Subcommand};
use domain::config::MagrayConfig;
use infrastructure::config::{ConfigLoader, ConfigValidator};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Generate an example configuration file
    #[command(alias = "gen")]
    Generate {
        /// Output path for the configuration file
        #[arg(short, long, default_value = ".magrayrc.toml")]
        output: PathBuf,

        /// Output format (toml or json)
        #[arg(short, long, default_value = "toml")]
        format: String,
    },

    /// Validate the current configuration
    #[command(alias = "check")]
    Validate {
        /// Path to configuration file to validate
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Show the current configuration
    Show {
        /// Show resolved configuration including environment variables
        #[arg(short, long)]
        resolved: bool,
    },

    /// Initialize configuration in the current directory
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },
}

impl ConfigCommand {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            ConfigSubcommand::Generate { output, format } => {
                self.generate_config(output, format).await
            }
            ConfigSubcommand::Validate { config } => self.validate_config(config.as_deref()).await,
            ConfigSubcommand::Show { resolved } => self.show_config(*resolved).await,
            ConfigSubcommand::Init { force } => self.init_config(*force).await,
        }
    }

    async fn generate_config(&self, output: &PathBuf, format: &str) -> Result<()> {
        info!("Generating example configuration file...");

        let example_config = ConfigLoader::generate_example_config();

        let content = if format == "json" {
            // Convert TOML to JSON
            let config: MagrayConfig = toml::from_str(&example_config)?;
            serde_json::to_string_pretty(&config)?
        } else {
            example_config
        };

        // Check if file exists
        if output.exists() {
            warn!("Configuration file already exists at: {}", output.display());
            println!("Use --force to overwrite or choose a different path");
            return Ok(());
        }

        tokio::fs::write(output, content).await?;
        info!("Configuration file generated at: {}", output.display());

        println!("✅ Configuration file created successfully!");
        println!("📝 Edit {} to customize your settings", output.display());
        println!("📋 Key configuration sections:");
        println!("   - AI providers and API keys");
        println!("   - Memory and embedding settings");
        println!("   - MCP server connections");
        println!("   - Plugin configuration");
        println!("   - Logging and performance tuning");

        Ok(())
    }

    async fn validate_config(&self, config_path: Option<&Path>) -> Result<()> {
        info!("Validating configuration...");

        let loader = if let Some(path) = config_path {
            ConfigLoader::new().with_path(path.to_path_buf())
        } else {
            ConfigLoader::new()
        };

        let config = loader.load().await?;
        let validator = ConfigValidator::new();

        match validator.validate(&config) {
            Ok(()) => {
                println!("✅ Configuration is valid!");
                info!("Configuration validation successful");
            }
            Err(e) => {
                println!("❌ Configuration validation failed:");
                println!("   {e}");
                return Err(e);
            }
        }

        Ok(())
    }

    async fn show_config(&self, resolved: bool) -> Result<()> {
        let loader = ConfigLoader::new();
        let config = loader.load().await?;

        if resolved {
            println!("# Resolved Configuration (including environment variables)");
        } else {
            println!("# Current Configuration");
        }

        let output = toml::to_string_pretty(&config)?;
        println!("{output}");

        Ok(())
    }

    async fn init_config(&self, force: bool) -> Result<()> {
        let config_path = PathBuf::from(".magrayrc.toml");

        if config_path.exists() && !force {
            warn!("Configuration file already exists");
            println!("❌ Configuration file already exists at .magrayrc.toml");
            println!("   Use --force to overwrite");
            return Ok(());
        }

        info!("Initializing configuration in current directory...");

        let example_config = ConfigLoader::generate_example_config();
        tokio::fs::write(&config_path, example_config).await?;

        println!("✅ Configuration initialized successfully!");
        println!("📝 Created .magrayrc.toml in current directory");
        println!("🔧 Next steps:");
        println!("   1. Edit .magrayrc.toml to add your API keys");
        println!("   2. Configure memory and embedding settings");
        println!("   3. Set up MCP servers if needed");
        println!("   4. Run 'magray config validate' to check your configuration");

        Ok(())
    }
}

use std::path::Path;
