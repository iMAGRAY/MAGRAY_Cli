// Integration tests for P1.2.2 Tool Manifest Validation
// Tests the complete manifest validation system with real files

use anyhow::Result;
use serde_json::json;
use std::fs;
use tempfile::TempDir;
use tools::manifest::{schema::*, validation::*};

/// Test valid WASM tool manifest
#[test]
fn test_valid_wasm_manifest() {
    let manifest = ToolManifest::new(
        "wasm-calculator".to_string(),
        "1.0.0".to_string(),
        "A WebAssembly calculator tool".to_string(),
        ToolType::Wasm,
        "calculator.wasm".to_string(),
        "MAGRAY Team".to_string(),
        "MIT".to_string(),
    )
    .with_capability(ToolCapability::Ui)
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(32),
        max_execution_time_ms: Some(5000),
        fuel_limit: Some(500_000),
    })
    .with_repository("https://github.com/magray/wasm-calculator".to_string());

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(result.is_valid);
    assert!(result.errors.is_empty());
    assert!(result.manifest.is_some());
}

/// Test invalid manifest with missing required fields
#[test]
fn test_invalid_manifest_missing_fields() {
    let manifest = ToolManifest::new(
        "".to_string(),        // Invalid empty name
        "invalid".to_string(), // Invalid version
        "".to_string(),        // Invalid empty description
        ToolType::Wasm,
        "".to_string(), // Invalid empty entry point
        "".to_string(), // Invalid empty author
        "".to_string(), // Invalid empty license
    );

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
    assert!(result.error_messages().len() >= 4); // At least 4 validation errors
}

/// Test tool type vs entry point validation
#[test]
fn test_tool_type_entry_point_validation() {
    // WASM tool with wrong extension
    let manifest = ToolManifest::new(
        "bad-wasm".to_string(),
        "1.0.0".to_string(),
        "Bad WASM tool".to_string(),
        ToolType::Wasm,
        "tool.exe".to_string(), // Wrong extension for WASM
        "Author".to_string(),
        "MIT".to_string(),
    );

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(!result.is_valid);
    assert!(result
        .error_messages()
        .iter()
        .any(|msg| msg.contains("Invalid tool type") || msg.contains("wasm")));
}

/// Test security validation in strict mode
#[test]
fn test_security_validation_strict_mode() {
    let manifest = ToolManifest::new(
        "high-risk-tool".to_string(),
        "1.0.0".to_string(),
        "Tool with all dangerous capabilities".to_string(),
        ToolType::Native,
        "dangerous-tool".to_string(),
        "Author".to_string(),
        "MIT".to_string(),
    )
    .with_capability(ToolCapability::Shell)
    .with_capability(ToolCapability::Network)
    .with_capability(ToolCapability::Filesystem)
    .with_shell_permissions(vec![ShellPermission::Execute])
    .with_network_permissions(vec![NetworkPermission::Outbound])
    .with_filesystem_permissions(vec![
        FilesystemPermission::Read,
        FilesystemPermission::Write,
    ])
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(1024),            // High memory usage
        max_execution_time_ms: Some(180_000), // 3 minutes
        fuel_limit: Some(10_000_000),
    });

    let validator = ToolManifestValidator::new().with_strict_mode(true);
    let result = validator.validate_manifest(manifest);

    assert!(!result.is_valid);
    assert!(result
        .error_messages()
        .iter()
        .any(|msg| msg.contains("security") || msg.contains("Security")));
}

/// Test JSON serialization and validation
#[test]
fn test_json_validation() -> Result<()> {
    let json = json!({
        "name": "json-test-tool",
        "version": "2.1.0",
        "description": "Tool for testing JSON validation",
        "type": "script",
        "capabilities": ["filesystem"],
        "entry_point": "script.py",
        "runtime_config": {
            "max_memory_mb": 128,
            "max_execution_time_ms": 15000,
            "fuel_limit": 2000000
        },
        "permissions": {
            "filesystem": ["read"]
        },
        "metadata": {
            "author": "JSON Test Author",
            "license": "Apache-2.0",
            "repository": "https://github.com/test/json-tool"
        }
    });

    let json_str = serde_json::to_string_pretty(&json)?;
    let result = validate_tool_manifest_json(&json_str)?;

    assert_eq!(result.name, "json-test-tool");
    assert_eq!(result.version, "2.1.0");
    assert_eq!(result.tool_type, ToolType::Script);
    assert!(result.has_capability(&ToolCapability::Filesystem));
    assert_eq!(result.effective_memory_limit(), 128);

    Ok(())
}

/// Test file validation with real files
#[test]
fn test_file_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let manifest_path = temp_dir.path().join("tool.json");
    let entry_path = temp_dir.path().join("example.wasm");

    // Create entry point file
    fs::write(&entry_path, b"dummy wasm content")?;

    // Create valid manifest
    let manifest_json = json!({
        "name": "file-test-tool",
        "version": "1.0.0",
        "description": "Tool for testing file validation",
        "type": "wasm",
        "capabilities": ["ui"],
        "entry_point": "example.wasm",
        "runtime_config": {
            "max_memory_mb": 64,
            "max_execution_time_ms": 30000,
            "fuel_limit": 1000000
        },
        "permissions": {},
        "metadata": {
            "author": "File Test Author",
            "license": "MIT"
        }
    });

    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_json)?,
    )?;

    let result = validate_tool_manifest(&manifest_path)?;
    assert_eq!(result.name, "file-test-tool");
    assert_eq!(result.tool_type, ToolType::Wasm);

    Ok(())
}

/// Test validation with missing entry point file
#[test]
fn test_missing_entry_point_validation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let manifest_path = temp_dir.path().join("tool.json");

    let manifest_json = json!({
        "name": "missing-entry-tool",
        "version": "1.0.0",
        "description": "Tool with missing entry point",
        "type": "native",
        "capabilities": [],
        "entry_point": "nonexistent.exe",
        "metadata": {
            "author": "Test Author",
            "license": "MIT"
        }
    });

    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_json)?,
    )?;

    let validator = ToolManifestValidator::new().with_file_existence_check(true);
    let result = validator.validate_file(&manifest_path);

    assert!(result.is_valid); // Should still be valid (warnings only)
    assert!(result.has_warnings_only());
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("Entry point file not found")));

    Ok(())
}

/// Test capability consistency validation
#[test]
fn test_capability_consistency() {
    let manifest = ToolManifest::new(
        "inconsistent-tool".to_string(),
        "1.0.0".to_string(),
        "Tool with inconsistent capabilities".to_string(),
        ToolType::Wasm,
        "tool.wasm".to_string(),
        "Author".to_string(),
        "MIT".to_string(),
    )
    .with_capability(ToolCapability::Filesystem)
    .with_capability(ToolCapability::Network);
    // Note: No filesystem or network permissions defined

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(!result.is_valid);
    assert!(result
        .error_messages()
        .iter()
        .any(|msg| msg.contains("capability") && msg.contains("permission")));
}

/// Test resource limit validation
#[test]
fn test_resource_limit_validation() {
    let manifest = ToolManifest::new(
        "resource-heavy-tool".to_string(),
        "1.0.0".to_string(),
        "Tool with extreme resource requirements".to_string(),
        ToolType::Wasm,
        "heavy.wasm".to_string(),
        "Author".to_string(),
        "MIT".to_string(),
    )
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(0),               // Invalid: zero memory
        max_execution_time_ms: Some(500_000), // Invalid: too long (>5 minutes)
        fuel_limit: Some(0),                  // Invalid: zero fuel
    });

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(!result.is_valid);
    assert!(result.error_messages().len() >= 2); // Multiple resource limit errors
}

/// Test warning generation
#[test]
fn test_warning_generation() {
    let manifest = ToolManifest::new(
        "warning-tool".to_string(),
        "1.0.0".to_string(),
        "Tool that should generate warnings".to_string(),
        ToolType::Native,
        "tool".to_string(),
        "Author".to_string(),
        "MIT".to_string(),
    )
    .with_capability(ToolCapability::Shell)
    .with_shell_permissions(vec![ShellPermission::Execute])
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(300),            // High memory (warning)
        max_execution_time_ms: Some(90_000), // Long timeout (warning)
        fuel_limit: Some(1_000_000),
    });
    // Note: No repository or documentation URLs

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    assert!(result.is_valid);
    assert!(result.has_warnings_only());
    assert!(result.warnings.len() >= 3); // Repository, memory, timeout, shell warnings
}

/// Test version format validation
#[test]
fn test_version_format_validation() {
    let invalid_versions = vec![
        "1",       // Too short
        "1.2",     // Too short
        "1.2.3.4", // Too long
        "v1.2.3",  // Invalid prefix
        "1.02.3",  // Leading zero
        "a.b.c",   // Non-numeric
    ];

    for version in invalid_versions {
        let manifest = ToolManifest::new(
            "version-test".to_string(),
            version.to_string(),
            "Version test tool".to_string(),
            ToolType::Native,
            "tool".to_string(),
            "Author".to_string(),
            "MIT".to_string(),
        );

        let validator = ToolManifestValidator::new();
        let result = validator.validate_manifest(manifest);

        assert!(!result.is_valid, "Version {version} should be invalid");
        assert!(result
            .error_messages()
            .iter()
            .any(|msg| msg.contains("version") || msg.contains("Version")));
    }
}

/// Test security path validation
#[test]
fn test_security_path_validation() {
    let dangerous_entry_points = vec![
        "../../../etc/passwd",            // Path traversal
        "/bin/sh",                        // Absolute path
        "C:\\Windows\\System32\\cmd.exe", // Windows system path
    ];

    for entry_point in dangerous_entry_points {
        let manifest = ToolManifest::new(
            "dangerous-path-tool".to_string(),
            "1.0.0".to_string(),
            "Tool with dangerous entry point".to_string(),
            ToolType::Native,
            entry_point.to_string(),
            "Author".to_string(),
            "MIT".to_string(),
        );

        let validator = ToolManifestValidator::new();
        let result = validator.validate_manifest(manifest);

        assert!(
            !result.is_valid,
            "Entry point {entry_point} should be invalid"
        );
        assert!(result
            .error_messages()
            .iter()
            .any(|msg| msg.contains("dangerous") || msg.contains("Security")));
    }
}

/// Test comprehensive validation report
#[test]
fn test_validation_report() {
    let manifest = ToolManifest::new(
        "".to_string(),            // Invalid name
        "bad-version".to_string(), // Invalid version
        "Test tool".to_string(),
        ToolType::Wasm,
        "tool.exe".to_string(), // Wrong extension
        "Author".to_string(),
        "MIT".to_string(),
    )
    .with_capability(ToolCapability::Filesystem); // Missing permission

    let validator = ToolManifestValidator::new();
    let result = validator.validate_manifest(manifest);

    let report = result.report();
    assert!(report.contains("FAILED"));
    assert!(report.contains("Errors:"));
    assert!(report.len() > 100); // Should be a comprehensive report
}

/// Benchmark validation performance
#[test]
fn test_validation_performance() {
    use std::time::Instant;

    let manifest = ToolManifest::new(
        "perf-test-tool".to_string(),
        "1.0.0".to_string(),
        "Performance test tool".to_string(),
        ToolType::Wasm,
        "perf.wasm".to_string(),
        "Perf Author".to_string(),
        "MIT".to_string(),
    );

    let validator = ToolManifestValidator::new();

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = validator.validate_manifest(manifest.clone());
    }
    let duration = start.elapsed();

    // Validation should be fast - 1000 validations in under 100ms
    assert!(
        duration.as_millis() < 100,
        "Validation too slow: {}ms for 1000 validations",
        duration.as_millis()
    );
}
