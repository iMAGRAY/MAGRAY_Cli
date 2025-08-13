// Example: Validate tool.json manifest files
// P1.2.2 Tool Manifest Validation demonstration

use std::env;
use std::process;
use tools::manifest::validation::ToolManifestValidator;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <tool.json>", args[0]);
        process::exit(1);
    }

    let manifest_path = &args[1];
    println!("🔍 Validating tool manifest: {manifest_path}");

    // Create validator with detailed reporting
    let validator = ToolManifestValidator::new()
        .with_strict_mode(false)
        .with_file_existence_check(true);

    // Validate the manifest
    let result = validator.validate_file(manifest_path);

    // Print detailed validation report
    println!("\n{}", result.report());

    if result.is_valid {
        if let Some(manifest) = result.manifest {
            println!("\n✅ Manifest is valid!");
            println!("Tool: {} v{}", manifest.name, manifest.version);
            println!("Type: {:?}", manifest.tool_type);
            println!("Entry Point: {}", manifest.entry_point);
            println!("Capabilities: {:?}", manifest.capabilities);
            println!("Memory Limit: {}MB", manifest.effective_memory_limit());
            println!("Timeout: {}ms", manifest.effective_timeout());

            if manifest.metadata.repository.is_some() {
                println!(
                    "Repository: {}",
                    manifest
                        .metadata
                        .repository
                        .as_ref()
                        .expect("Operation failed - converted from unwrap()")
                );
            }
        }
        process::exit(0);
    } else {
        println!("\n❌ Manifest validation failed!");
        process::exit(1);
    }
}
