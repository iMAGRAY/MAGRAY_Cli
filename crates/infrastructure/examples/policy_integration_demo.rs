// Policy Integration Demo
// Shows how to use the policy integration system with configuration profiles

use anyhow::Result;
use domain::config::{MagrayConfig, Profile, ProfileConfig};
use infrastructure::config::policy_integration::{OperationContext, ResourceRequirements};
use infrastructure::config::{ConfigLoader, PolicyDecision, PolicyIntegrationEngine, RiskLevel};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (simple println for demo)
    // tracing_subscriber::fmt::init();

    println!("🛡️ Policy Integration Demo");
    println!("==========================\n");

    // Create policy integration engine
    let mut policy_engine = PolicyIntegrationEngine::new();

    // Demo 1: Development Profile
    println!("📋 Demo 1: Development Profile");
    println!("------------------------------");

    let mut dev_config = MagrayConfig {
        profile: Profile::Dev,
        profile_config: Some(ProfileConfig::dev()),
        ..MagrayConfig::default()
    };
    dev_config.apply_profile(&ProfileConfig::dev());

    policy_engine.apply_profile_policy(&dev_config).await?;
    let dev_policy = policy_engine.get_policy_config();

    println!("Dev Profile Configuration:");
    println!("  - Default Mode: {}", dev_policy.default_mode);
    println!("  - Risk Level: {:?}", dev_policy.risk_level);
    println!("  - Permissive Mode: {}", dev_policy.permissive_mode);
    println!("  - Sandbox Enabled: {}", dev_policy.sandbox_config.enabled);
    println!(
        "  - Require Signed Tools: {}",
        dev_policy.tool_permissions.require_signed_tools
    );
    println!(
        "  - Emergency Overrides: {}",
        dev_policy.emergency_overrides.enabled
    );

    // Test some operations with dev profile
    println!("\nTesting operations with Dev profile:");
    test_operations(&policy_engine, "Dev").await?;

    println!("\n{}\n", "=".repeat(50));

    // Demo 2: Production Profile
    println!("📋 Demo 2: Production Profile");
    println!("-----------------------------");

    let mut prod_config = MagrayConfig {
        profile: Profile::Prod,
        profile_config: Some(ProfileConfig::prod()),
        ..MagrayConfig::default()
    };
    prod_config.apply_profile(&ProfileConfig::prod());

    policy_engine.apply_profile_policy(&prod_config).await?;
    let prod_policy = policy_engine.get_policy_config();

    println!("Prod Profile Configuration:");
    println!("  - Default Mode: {}", prod_policy.default_mode);
    println!("  - Risk Level: {:?}", prod_policy.risk_level);
    println!("  - Permissive Mode: {}", prod_policy.permissive_mode);
    println!(
        "  - Sandbox Enabled: {}",
        prod_policy.sandbox_config.enabled
    );
    println!(
        "  - Require Signed Tools: {}",
        prod_policy.tool_permissions.require_signed_tools
    );
    println!(
        "  - Emergency Overrides: {}",
        prod_policy.emergency_overrides.enabled
    );

    // Test some operations with prod profile
    println!("\nTesting operations with Prod profile:");
    test_operations(&policy_engine, "Prod").await?;

    println!("\n{}\n", "=".repeat(50));

    // Demo 3: Custom Profile
    println!("📋 Demo 3: Custom Profile");
    println!("-------------------------");

    let mut custom_config = MagrayConfig {
        profile: Profile::Custom("staging".to_string()),
        profile_config: Some(ProfileConfig::default()),
        ..MagrayConfig::default()
    };
    custom_config.apply_profile(&ProfileConfig::default());

    policy_engine.apply_profile_policy(&custom_config).await?;
    let custom_policy = policy_engine.get_policy_config();

    println!("Custom Profile Configuration:");
    println!("  - Default Mode: {}", custom_policy.default_mode);
    println!("  - Risk Level: {:?}", custom_policy.risk_level);
    println!("  - Permissive Mode: {}", custom_policy.permissive_mode);
    println!(
        "  - Sandbox Enabled: {}",
        custom_policy.sandbox_config.enabled
    );
    println!(
        "  - Require Signed Tools: {}",
        custom_policy.tool_permissions.require_signed_tools
    );
    println!(
        "  - Emergency Overrides: {}",
        custom_policy.emergency_overrides.enabled
    );

    // Test some operations with custom profile
    println!("\nTesting operations with Custom profile:");
    test_operations(&policy_engine, "Custom").await?;

    println!("\n{}\n", "=".repeat(50));

    // Demo 4: Runtime Profile Switching
    println!("📋 Demo 4: Runtime Profile Switching");
    println!("------------------------------------");

    let config_loader = ConfigLoader::new();

    // Start with dev profile
    let mut runtime_config = MagrayConfig {
        profile: Profile::Dev,
        profile_config: Some(ProfileConfig::dev()),
        ..MagrayConfig::default()
    };
    runtime_config.apply_profile(&ProfileConfig::dev());

    println!("Initial profile: {}", runtime_config.profile.name());
    policy_engine.apply_profile_policy(&runtime_config).await?;
    println!(
        "Policy: permissive={}, sandbox={}",
        policy_engine.get_policy_config().permissive_mode,
        policy_engine.get_policy_config().sandbox_config.enabled
    );

    // Switch to prod profile
    runtime_config = config_loader
        .switch_profile(runtime_config, Profile::Prod)
        .await?;
    println!("\nSwitched to profile: {}", runtime_config.profile.name());
    policy_engine.apply_profile_policy(&runtime_config).await?;
    println!(
        "Policy: permissive={}, sandbox={}",
        policy_engine.get_policy_config().permissive_mode,
        policy_engine.get_policy_config().sandbox_config.enabled
    );

    // Switch back to dev
    runtime_config = config_loader
        .switch_profile(runtime_config, Profile::Dev)
        .await?;
    println!(
        "\nSwitched back to profile: {}",
        runtime_config.profile.name()
    );
    policy_engine.apply_profile_policy(&runtime_config).await?;
    println!(
        "Policy: permissive={}, sandbox={}",
        policy_engine.get_policy_config().permissive_mode,
        policy_engine.get_policy_config().sandbox_config.enabled
    );

    println!("\n✅ Policy Integration Demo completed successfully!");

    Ok(())
}

async fn test_operations(
    policy_engine: &PolicyIntegrationEngine,
    _profile_name: &str,
) -> Result<()> {
    let test_operations = vec![
        ("file_read", "file_reader", RiskLevel::Low),
        ("file_write", "file_writer", RiskLevel::Medium),
        ("shell_exec", "shell_executor", RiskLevel::High),
        ("network_request", "http_client", RiskLevel::Medium),
        ("system_info", "system_probe", RiskLevel::Low),
    ];

    for (operation, tool, risk_level) in test_operations {
        let context = OperationContext {
            operation: operation.to_string(),
            tool_name: Some(tool.to_string()),
            risk_level,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(100),
                cpu_time_secs: Some(30),
                network_required: operation == "network_request",
                filesystem_write: operation == "file_write",
            },
            user_confirmation: false,
        };

        let decision = policy_engine.check_operation_allowed(operation, &context);

        let decision_icon = match decision {
            PolicyDecision::Allow(_) => "✅",
            PolicyDecision::Ask(_) => "❓",
            PolicyDecision::Deny(_) => "❌",
        };

        println!(
            "  {} {:<15} ({:<10}): {}",
            decision_icon,
            operation,
            format!("{:?}", risk_level),
            match decision {
                PolicyDecision::Allow(reason) => format!("ALLOW - {reason}"),
                PolicyDecision::Ask(reason) => format!("ASK - {reason}"),
                PolicyDecision::Deny(reason) => format!("DENY - {reason}"),
            }
        );
    }

    Ok(())
}
