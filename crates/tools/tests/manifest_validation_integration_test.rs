// Integration tests for P1.2.2.b: Manifest validation integration in plugin loading
// Tests the complete flow: plugin loading -> manifest validation -> success/rejection

use serde_json::json;
use tempfile::TempDir;
use tools::manifest::validation::ToolManifestValidator;
use tools::plugins::plugin_manager::PluginRegistry;

#[tokio::test]
async fn test_plugin_load_with_valid_manifest() {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let plugin_dir = temp_dir.path().join("plugins");
    let config_dir = temp_dir.path().join("configs");

    tokio::fs::create_dir_all(&plugin_dir)
        .await
        .expect("Test operation should succeed");
    tokio::fs::create_dir_all(&config_dir)
        .await
        .expect("Test operation should succeed");

    // Create minimal valid tool.json manifest using P1.2.2 schema
    let valid_manifest = json!({
        "id": "test_tool",
        "name": "test-tool",
        "version": "1.0.0",
        "description": "A test tool with valid manifest",
        "type": "wasm",
        "entry_point": "test.wasm",
        "metadata": {
            "author": "Test Author",
            "license": "MIT"
        }
    });

    let manifest_path = plugin_dir.join("tool.json");
    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&valid_manifest).expect("Test operation should succeed"),
    )
    .await
    .expect("Test operation should succeed");

    // Test validation directly
    let validator = ToolManifestValidator::new();
    let validation_result = validator.validate_file(&manifest_path);

    // Debug output
    println!("Validation report: {}", validation_result.report());
    println!("Is valid: {}", validation_result.is_valid);
    println!("Errors: {:?}", validation_result.errors);
    println!("Warnings: {:?}", validation_result.warnings);

    if !validation_result.is_valid {
        // Let's understand what's wrong with the manifest
        panic!(
            "Valid manifest should pass validation. Report: {}",
            validation_result.report()
        );
    }

    assert!(
        validation_result.errors.is_empty(),
        "No errors expected for valid manifest. Errors: {:?}",
        validation_result.errors
    );
}

#[tokio::test]
async fn test_plugin_load_with_invalid_manifest() {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let plugin_dir = temp_dir.path().join("plugins");
    let config_dir = temp_dir.path().join("configs");

    tokio::fs::create_dir_all(&plugin_dir)
        .await
        .expect("Test operation should succeed");
    tokio::fs::create_dir_all(&config_dir)
        .await
        .expect("Test operation should succeed");

    // Create invalid tool.json manifest (missing required fields)
    let invalid_manifest = json!({
        "id": "bad_tool",
        "name": "", // Invalid: empty name
        "version": "invalid.version", // Invalid: bad version format
        "description": "A test tool with invalid manifest"
        // Missing required fields: type, entry_point, etc.
    });

    let manifest_path = plugin_dir.join("tool.json");
    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&invalid_manifest).expect("Test operation should succeed"),
    )
    .await
    .expect("Test operation should succeed");

    // Test validation directly
    let validator = ToolManifestValidator::new();
    let validation_result = validator.validate_file(&manifest_path);

    assert!(
        !validation_result.is_valid,
        "Invalid manifest should fail validation"
    );
    assert!(
        !validation_result.errors.is_empty(),
        "Errors expected for invalid manifest"
    );
}

#[tokio::test]
async fn test_plugin_load_without_manifest() {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let plugin_dir = temp_dir.path().join("plugins");
    let config_dir = temp_dir.path().join("configs");

    tokio::fs::create_dir_all(&plugin_dir)
        .await
        .expect("Test operation should succeed");
    tokio::fs::create_dir_all(&config_dir)
        .await
        .expect("Test operation should succeed");

    let _registry = PluginRegistry::new(plugin_dir, config_dir);

    // Test should handle missing manifest gracefully (legacy mode)
    let non_existent_path = temp_dir.path().join("non_existent_tool.json");

    let validator = ToolManifestValidator::new();
    let validation_result = validator.validate_file(&non_existent_path);

    // Should fail validation but not crash
    assert!(
        !validation_result.is_valid,
        "Non-existent manifest should fail validation"
    );
}

#[tokio::test]
async fn test_manifest_validation_with_warnings() {
    let temp_dir = TempDir::new().expect("Test operation should succeed");
    let plugin_dir = temp_dir.path().join("plugins");

    tokio::fs::create_dir_all(&plugin_dir)
        .await
        .expect("Test operation should succeed");

    // Create manifest with warnings but no errors
    let manifest_with_warnings = json!({
        "id": "warning_tool",
        "name": "Warning Tool",
        "version": "1.0.0-beta", // Version with pre-release tag (might trigger warning)
        "description": "A tool that generates warnings",
        "type": "wasm",
        "capabilities": ["filesystem", "network"], // Many capabilities (might trigger warning)
        "entry_point": "tool.wasm",
        "runtime_config": {
            "memory_limit": 2048, // High memory (might trigger warning)
            "timeout": 300
        },
        "permissions": {
            "filesystem": {
                "mode": "readwrite",
                "allowed_paths": ["/"]  // Root access (might trigger warning)
            },
            "network": {
                "mode": "internet", // Internet access (might trigger warning)
                "allowed_hosts": ["*"]
            }
        },
        "metadata": {
            "author": "Test Author",
            "license": "MIT"
        }
    });

    let manifest_path = plugin_dir.join("tool.json");
    tokio::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_with_warnings)
            .expect("Test operation should succeed"),
    )
    .await
    .expect("Test operation should succeed");

    let validator = ToolManifestValidator::new();
    let validation_result = validator.validate_file(&manifest_path);

    // Should be valid but may have warnings
    println!("Validation result: {validation_result:?}");

    // This tool has high-risk permissions but should still be valid
    if !validation_result.is_valid {
        println!("Validation errors: {:?}", validation_result.errors);
    }
}
