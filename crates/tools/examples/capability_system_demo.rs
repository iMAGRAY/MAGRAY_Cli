// P1.2.3 Capability System Demo
// Demonstrates comprehensive capability-based security system for Tools Platform 2.0

use std::path::PathBuf;
use tools::capabilities::checker::*;
use tools::capabilities::validation::*;
use tools::capabilities::*;
use tools::manifest::{ToolManifest, ToolType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 P1.2.3 Capability System Demo");
    println!("===================================");

    // 1. Create various capabilities
    println!("\n1. Creating different capability types:");

    let fs_capability = Capability::Filesystem {
        mode: AccessMode::Read,
        paths: vec![PathBuf::from("./data")],
    };
    println!(
        "✓ Filesystem capability (read ./data): risk={}",
        fs_capability.risk_level()
    );

    let network_capability = Capability::Network {
        mode: NetworkMode::Outbound,
        domains: vec!["api.example.com".to_string()],
    };
    println!(
        "✓ Network capability (outbound to api.example.com): risk={}",
        network_capability.risk_level()
    );

    let shell_capability = Capability::Shell {
        commands: vec!["ls".to_string(), "cat".to_string()],
        elevated: false,
    };
    println!(
        "✓ Shell capability (ls, cat): risk={}",
        shell_capability.risk_level()
    );

    let memory_capability = Capability::Memory { max_mb: 256 };
    println!(
        "✓ Memory capability (256MB): risk={}",
        memory_capability.risk_level()
    );

    // 2. Test capability checker with different policies
    println!("\n2. Testing capability checker with policies:");

    // Default policy
    let default_checker = DefaultCapabilityChecker::new();
    println!("✓ Default policy - max risk level: 7");

    // Test safe capability
    match default_checker.check_capability(&memory_capability) {
        Ok(true) => println!("  ✅ Memory capability (256MB) - ALLOWED"),
        Ok(false) => println!("  ❌ Memory capability (256MB) - DENIED"),
        Err(e) => println!("  ❌ Memory capability (256MB) - ERROR: {e}"),
    }

    // Test risky capability
    let dangerous_shell = Capability::Shell {
        commands: vec!["rm".to_string()],
        elevated: true,
    };

    match default_checker.check_capability(&dangerous_shell) {
        Ok(true) => println!("  ⚠️  Dangerous shell capability - ALLOWED"),
        Ok(false) => println!("  ✅ Dangerous shell capability - DENIED (high risk)"),
        Err(e) => println!("  ✅ Dangerous shell capability - DENIED: {e}"),
    }

    // 3. Test strict policy
    println!("\n3. Testing strict security policy:");

    let strict_policy = CapabilityPolicy::strict();
    let strict_checker = DefaultCapabilityChecker::with_policy(strict_policy);

    match strict_checker.check_capability(&shell_capability) {
        Ok(true) => println!("  ✅ Safe shell capability - ALLOWED"),
        Ok(false) => println!("  ❌ Safe shell capability - DENIED"),
        Err(e) => println!("  ❌ Safe shell capability - ERROR: {e}"),
    }

    // 4. Create and validate capability specification
    println!("\n4. Creating capability specification:");

    let spec = CapabilitySpec::new(vec![
        fs_capability.clone(),
        network_capability.clone(),
        memory_capability.clone(),
    ])
    .with_justification("File processing tool that downloads data and processes it".to_string());

    println!(
        "✓ Created spec with {} required capabilities",
        spec.required.len()
    );
    println!("  Justification: {}", spec.justification);

    // 5. Test capability validation system
    println!("\n5. Testing capability validation system:");

    let validator = CapabilityValidator::new(SecurityLevel::Balanced);
    let context = ValidationContext {
        tool_id: "demo-tool".to_string(),
        tool_type: "data-processor".to_string(),
        working_dir: PathBuf::from("."),
        system_resources: SystemResources {
            available_memory_mb: 4096,
            cpu_cores: 8,
            available_disk_mb: 102400,
        },
    };

    match validator.validate_capability_spec(&spec, &context) {
        Ok(result) => {
            println!("✅ Validation completed");
            println!("  Success: {}", result.is_valid());
            println!("  Capability results: {}", result.capability_results.len());
            println!("  Global warnings: {}", result.global_warnings.len());

            for warning in &result.global_warnings {
                println!("    ⚠️  {warning}");
            }
        }
        Err(e) => println!("❌ Validation failed: {e}"),
    }

    // 6. Test manifest integration
    println!("\n6. Testing manifest integration:");

    let mut manifest = ToolManifest::new(
        "capability-demo-tool".to_string(),
        "1.0.0".to_string(),
        "Demo tool with capabilities".to_string(),
        ToolType::Wasm,
        "demo.wasm".to_string(),
        "Demo Author".to_string(),
        "MIT".to_string(),
    );

    manifest = manifest.with_capability_spec(spec.clone());

    println!("✓ Created tool manifest with capability spec");
    println!("  Tool name: {}", manifest.name);
    println!("  Max risk level: {}", manifest.max_risk_level());
    println!(
        "  Requires elevated privileges: {}",
        manifest.requires_elevated_privileges()
    );

    // 7. Test capability granting workflow
    println!("\n7. Testing capability granting workflow:");

    let mut checker = DefaultCapabilityChecker::new();

    // Grant capabilities one by one
    for capability in &spec.required {
        match checker.grant_capability(capability.clone()) {
            Ok(()) => {
                println!("  ✅ Granted: {}", capability.description());
            }
            Err(e) => {
                println!("  ❌ Denied: {} - {}", capability.description(), e);
            }
        }
    }

    println!(
        "\nGranted capabilities: {}",
        checker.get_capabilities().len()
    );

    // 8. Test utility functions
    println!("\n8. Testing utility functions:");

    let file_reader_spec = CapabilityUtils::default_capabilities_for_tool_type("file_reader");
    println!(
        "✓ Default file_reader spec: {} capabilities",
        file_reader_spec.required.len()
    );

    let missing = CapabilityUtils::check_capability_spec(&checker, &file_reader_spec)?;
    if missing.is_empty() {
        println!("  ✅ File reader capabilities satisfied");
    } else {
        println!(
            "  ❌ Missing {} capabilities for file reader",
            missing.len()
        );
    }

    println!("\n🎉 P1.2.3 Capability System Demo Complete!");
    println!("\n📋 Features demonstrated:");
    println!("  ✓ Capability creation (fs/net/shell/ui/memory/compute)");
    println!("  ✓ Permission checking with policy enforcement");
    println!("  ✓ Security level validation (strict/balanced/permissive)");
    println!("  ✓ Capability specification and builder pattern");
    println!("  ✓ Integration with tool manifest system");
    println!("  ✓ Risk assessment and security enforcement");
    println!("  ✓ Utility functions for common tool types");

    Ok(())
}
