use anyhow::Result;
use domain::config::*;
use infrastructure::config::ConfigLoader;
use std::env;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_profile_detection() -> Result<()> {
    // Clean up any existing env vars first
    env::remove_var("MAGRAY_ENV");

    let loader = ConfigLoader::new();

    // Test default profile (should be dev when no env var)
    assert_eq!(loader.detect_profile(), Profile::Dev);

    // Test environment variable detection
    env::set_var("MAGRAY_ENV", "prod");
    assert_eq!(loader.detect_profile(), Profile::Prod);

    env::set_var("MAGRAY_ENV", "production");
    assert_eq!(loader.detect_profile(), Profile::Prod);

    env::set_var("MAGRAY_ENV", "custom_profile");
    assert_eq!(
        loader.detect_profile(),
        Profile::Custom("custom_profile".to_string())
    );

    // Cleanup
    env::remove_var("MAGRAY_ENV");

    Ok(())
}

#[tokio::test]
async fn test_profile_config_generation() -> Result<()> {
    // Test dev profile generation
    let dev_config = ConfigLoader::generate_profile_config(&Profile::Dev);
    assert!(dev_config.contains("permissive_mode = true"));
    assert!(dev_config.contains("debug_symbols = true"));
    assert!(dev_config.contains("require_signed_tools = false"));

    // Test prod profile generation
    let prod_config = ConfigLoader::generate_profile_config(&Profile::Prod);
    assert!(prod_config.contains("permissive_mode = false"));
    assert!(prod_config.contains("debug_symbols = false"));
    assert!(prod_config.contains("require_signed_tools = true"));
    assert!(prod_config.contains("level_override = \"warn\""));

    Ok(())
}

#[tokio::test]
async fn test_profile_validation() -> Result<()> {
    let loader = ConfigLoader::new();

    // Test valid profile config
    let valid_config = ProfileConfig::dev();
    assert!(loader.validate_profile_config(&valid_config).is_ok());

    let valid_prod_config = ProfileConfig::prod();
    assert!(loader.validate_profile_config(&valid_prod_config).is_ok());

    // Test invalid policy mode
    let mut invalid_config = ProfileConfig::dev();
    invalid_config.security.default_policy_mode = "invalid_mode".to_string();
    assert!(loader.validate_profile_config(&invalid_config).is_err());

    // Test invalid risk level
    let mut invalid_config = ProfileConfig::dev();
    invalid_config.security.risk_level = "invalid_level".to_string();
    assert!(loader.validate_profile_config(&invalid_config).is_err());

    // Test invalid memory limit
    let mut invalid_config = ProfileConfig::dev();
    invalid_config.performance.memory_limit_override_mb = Some(32); // Too low
    assert!(loader.validate_profile_config(&invalid_config).is_err());

    invalid_config.performance.memory_limit_override_mb = Some(16384); // Too high
    assert!(loader.validate_profile_config(&invalid_config).is_err());

    Ok(())
}

#[tokio::test]
async fn test_hierarchical_config_loading() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("configs");
    fs::create_dir_all(&config_dir).await?;

    // Create base config
    let base_config = r#"
profile = "dev"

[logging]
level = "info"
file_enabled = true

[performance]
worker_threads = 4
"#;
    fs::write(temp_dir.path().join("magray.toml"), base_config).await?;

    // Create dev profile config
    let dev_profile_config = r#"
[security]
default_policy_mode = "ask"
permissive_mode = true

[logging]
level_override = "debug"
console_enabled = true

[tools]
dry_run_default = true
"#;
    fs::write(config_dir.join("dev.toml"), dev_profile_config).await?;

    // Load config with profile
    env::set_var("MAGRAY_ENV", "dev");
    let original_dir = env::current_dir()?;
    env::set_current_dir(temp_dir.path())?; // Change to temp directory

    let loader = ConfigLoader::new().with_path(temp_dir.path().join("magray.toml"));

    let config = loader.load().await?;

    // Verify profile is detected
    assert_eq!(config.profile, Profile::Dev);

    // Verify base config is loaded
    assert_eq!(config.performance.worker_threads, 4);
    assert!(config.logging.file_enabled);

    // Verify profile config is applied
    assert!(config.profile_config.is_some());
    let profile_config = config
        .profile_config
        .expect("Test operation should succeed");
    assert_eq!(profile_config.security.default_policy_mode, "ask");
    assert!(profile_config.security.permissive_mode);
    assert_eq!(
        profile_config.logging.level_override,
        Some("debug".to_string())
    );
    assert!(profile_config.tools.dry_run_default);

    // Verify profile overrides are applied to base config
    assert_eq!(config.logging.level, "debug"); // Overridden by profile

    env::set_current_dir(original_dir)?; // Restore directory
    env::remove_var("MAGRAY_ENV");
    Ok(())
}

#[tokio::test]
async fn test_runtime_profile_switching() -> Result<()> {
    // Create temporary directory with proper config files
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("configs");
    fs::create_dir_all(&config_dir).await?;

    // Create prod.toml with correct content
    let prod_config_content = r#"
[security]
default_policy_mode = "ask"
risk_level = "low"
permissive_mode = false
ask_by_default = true

[logging]
level_override = "warn"
console_enabled = false
debug_symbols = false
structured_only = true

[performance]
debug_allocation_tracking = false
memory_limit_override_mb = 512
production_optimizations = true

[tools]
whitelist_mode = "minimal"
dry_run_default = false
require_signed_tools = true
"#;
    fs::write(config_dir.join("prod.toml"), prod_config_content).await?;

    // Change to temp directory to make configs discoverable
    let original_dir = env::current_dir()?;
    env::set_current_dir(temp_dir.path())?;

    let loader = ConfigLoader::new();

    // Start with dev config
    let mut config = MagrayConfig {
        profile: Profile::Dev,
        profile_config: Some(ProfileConfig::dev()),
        ..Default::default()
    };
    config.apply_profile(&ProfileConfig::dev());

    // Verify dev settings
    assert_eq!(config.profile, Profile::Dev);
    assert!(!config.plugins.sandbox_enabled); // Permissive mode disabled sandbox

    // Create a manual production config for comparison
    let manual_prod_config = ProfileConfig::prod();
    println!("Manual prod config:");
    println!(
        "  permissive_mode = {}",
        manual_prod_config.security.permissive_mode
    );

    // Check current directory and available files
    println!(
        "Current directory: {:?}",
        env::current_dir().expect("Test operation should succeed")
    );
    if let Ok(entries) = fs::read_dir(".").await {
        println!("Files in current directory:");
        let mut entries = entries;
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            println!("  {}", entry.file_name().to_string_lossy());
        }
    }

    if let Ok(entries) = fs::read_dir("configs").await {
        println!("Files in configs directory:");
        let mut entries = entries;
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            println!("  {}", entry.file_name().to_string_lossy());
        }
    } else {
        println!("configs directory not found");
    }

    // Switch to prod profile
    config = loader.switch_profile(config, Profile::Prod).await?;

    // Debug output
    println!("After switching to prod:");
    println!("config.profile = {:?}", config.profile);
    println!(
        "config.profile_config present = {}",
        config.profile_config.is_some()
    );
    if let Some(ref profile_config) = config.profile_config {
        println!(
            "profile_config.security.permissive_mode = {}",
            profile_config.security.permissive_mode
        );
        println!(
            "profile_config.security.risk_level = {}",
            profile_config.security.risk_level
        );
        println!(
            "profile_config.tools.require_signed_tools = {}",
            profile_config.tools.require_signed_tools
        );
    }
    println!(
        "config.plugins.sandbox_enabled = {}",
        config.plugins.sandbox_enabled
    );

    // Verify prod settings
    assert_eq!(config.profile, Profile::Prod);
    assert!(config.profile_config.is_some());

    let profile_config = config
        .profile_config
        .as_ref()
        .expect("Test operation should succeed");
    assert!(!profile_config.security.permissive_mode);
    assert!(profile_config.tools.require_signed_tools);
    assert_eq!(
        profile_config.logging.level_override,
        Some("warn".to_string())
    );

    // Verify sandbox is enabled in prod (non-permissive mode)
    assert!(config.plugins.sandbox_enabled);

    // Restore original directory
    env::set_current_dir(original_dir)?;

    Ok(())
}

#[tokio::test]
async fn test_profile_config_save_load() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let original_dir = env::current_dir()?;
    env::set_current_dir(temp_dir.path())?;

    let loader = ConfigLoader::new();
    let profile = Profile::Prod;
    let profile_config = ProfileConfig::prod();

    // Save profile config
    loader
        .save_profile_config(&profile, &profile_config)
        .await?;

    // Verify file was created
    let profile_path = temp_dir.path().join("configs").join("prod.toml");
    assert!(profile_path.exists());

    // Load and verify content
    let content = fs::read_to_string(&profile_path).await?;
    assert!(content.contains("permissive_mode = false"));
    assert!(content.contains("require_signed_tools = true"));

    // Restore original directory
    env::set_current_dir(original_dir)?;
    Ok(())
}

#[tokio::test]
async fn test_profile_security_integration() -> Result<()> {
    // Test dev profile security settings
    let dev_config = MagrayConfig {
        profile: Profile::Dev,
        profile_config: Some(ProfileConfig::dev()),
        ..Default::default()
    };

    let security = dev_config.effective_security();
    assert!(security.permissive_mode);
    assert!(security.ask_by_default);

    // Test prod profile security settings
    let prod_config = MagrayConfig {
        profile: Profile::Prod,
        profile_config: Some(ProfileConfig::prod()),
        ..Default::default()
    };

    let security = prod_config.effective_security();
    assert!(!security.permissive_mode);
    assert!(security.ask_by_default);
    assert_eq!(security.risk_level, "low");

    // Test tools integration
    let tools = dev_config.effective_tools();
    assert!(tools.dry_run_default);
    assert!(!tools.require_signed_tools);

    let tools = prod_config.effective_tools();
    assert!(!tools.dry_run_default);
    assert!(tools.require_signed_tools);

    Ok(())
}

#[tokio::test]
async fn test_profile_environment_override() -> Result<()> {
    // Clean up env first
    env::remove_var("MAGRAY_ENV");

    // Test MAGRAY_ENV environment variable
    env::set_var("MAGRAY_ENV", "prod");

    let loader = ConfigLoader::new();
    let config = loader.load().await?;

    assert_eq!(config.profile, Profile::Prod);

    // Test custom profile
    env::set_var("MAGRAY_ENV", "staging");
    let config = loader.load().await?;

    assert_eq!(config.profile, Profile::Custom("staging".to_string()));

    // Cleanup
    env::remove_var("MAGRAY_ENV");
    Ok(())
}

#[test]
fn test_profile_serialization() -> Result<()> {
    // Test Profile enum serialization
    let dev_profile = Profile::Dev;
    let serialized = serde_json::to_string(&dev_profile)?;
    let deserialized: Profile = serde_json::from_str(&serialized)?;
    assert_eq!(dev_profile, deserialized);

    // Test ProfileConfig serialization
    let profile_config = ProfileConfig::prod();
    let serialized = serde_json::to_string(&profile_config)?;
    let deserialized: ProfileConfig = serde_json::from_str(&serialized)?;
    assert_eq!(
        profile_config.security.permissive_mode,
        deserialized.security.permissive_mode
    );

    Ok(())
}
