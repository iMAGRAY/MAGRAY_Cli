// P1.2.4.a Integration Tests
// Tests for WASM sandbox integration with capabilities and manifest validation

use std::path::PathBuf;
use tools::{
    capabilities::{AccessMode, Capability},
    manifest::{RuntimeConfig, ToolManifest, ToolType},
    SandboxConfig, SandboxError, WasmSandbox,
};

#[test]
fn test_sandbox_creation_from_manifest() {
    let manifest = create_test_manifest();
    let result = WasmSandbox::from_manifest(&manifest);

    #[cfg(feature = "wasm-runtime")]
    {
        assert!(result.is_ok());
        let sandbox = result.expect("Test operation should succeed");
        assert!(sandbox.has_capability(&Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        }));
    }

    #[cfg(not(feature = "wasm-runtime"))]
    {
        assert!(result.is_err());
    }
}

#[test]
fn test_restrictive_sandbox_config() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config);

    #[cfg(feature = "wasm-runtime")]
    {
        assert!(sandbox.is_ok());
        let sandbox = sandbox.expect("Test operation should succeed");

        // Should have restrictive defaults
        assert!(sandbox.config().resource_limits.max_memory_bytes <= 64 * 1024 * 1024);
        assert!(sandbox.config().resource_limits.max_execution_time_ms <= 30_000);
    }
}

#[test]
fn test_wasm_module_validation() {
    let config = SandboxConfig::default();

    #[cfg(feature = "wasm-runtime")]
    {
        let sandbox = WasmSandbox::new(config).expect("Test operation should succeed");

        // Test invalid WASM
        let invalid_wasm = b"this is not wasm";
        let result = sandbox.load_module(invalid_wasm, "invalid.wasm".to_string());
        assert!(result.is_err());

        // Test valid WASM header (minimal)
        let valid_wasm = create_minimal_wasm();
        let _result = sandbox.load_module(&valid_wasm, "test.wasm".to_string());
        // This should pass validation but may fail at compilation
        // We're testing the security validation layer here
    }
}

#[test]
fn test_capability_enforcement() {
    let mut manifest = create_test_manifest();

    // Add filesystem capability
    manifest = manifest.require_capability(Capability::Filesystem {
        mode: AccessMode::Write,
        paths: vec![PathBuf::from("/tmp")],
    });

    let config = SandboxConfig::from_manifest(&manifest);

    #[cfg(feature = "wasm-runtime")]
    {
        assert!(config.is_ok());
        let config = config.expect("Test operation should succeed");

        // Should allow /tmp write access
        assert!(config
            .wasi_config
            .filesystem_access
            .can_write(&PathBuf::from("/tmp")));

        // Should not allow other paths
        assert!(!config
            .wasi_config
            .filesystem_access
            .can_write(&PathBuf::from("/etc")));
    }
}

#[test]
fn test_network_capability_enforcement() {
    let mut manifest = create_test_manifest();

    // Add network capability
    manifest = manifest.require_capability(Capability::Network {
        mode: tools::capabilities::NetworkMode::Outbound,
        domains: vec!["api.example.com".to_string()],
    });

    let config = SandboxConfig::from_manifest(&manifest);

    #[cfg(feature = "wasm-runtime")]
    {
        assert!(config.is_ok());
        let config = config.expect("Test operation should succeed");

        // Should allow api.example.com
        assert!(config
            .wasi_config
            .network_access
            .can_connect("api.example.com"));

        // Should not allow other domains
        assert!(!config
            .wasi_config
            .network_access
            .can_connect("malicious.com"));
    }
}

#[test]
fn test_security_violation_detection() {
    let config = SandboxConfig::default();

    #[cfg(feature = "wasm-runtime")]
    {
        let sandbox = WasmSandbox::new(config).expect("Test operation should succeed");

        // Initially no violations
        assert_eq!(sandbox.get_violations().len(), 0);

        // Try to load an oversized "module" (this will trigger size violation)
        let oversized_wasm = vec![0u8; 100 * 1024 * 1024]; // 100MB
        let result = sandbox.load_module(&oversized_wasm, "huge.wasm".to_string());
        assert!(result.is_err());

        // Should have logged a violation
        let violations = sandbox.get_violations();
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| matches!(
            v.violation_type,
            tools::ViolationType::ResourceLimitViolation { .. }
        )));
    }
}

#[test]
fn test_execute_permissions_rejected() {
    let mut manifest = create_test_manifest();

    // Try to add execute capability (should be rejected in sandbox)
    manifest = manifest.require_capability(Capability::Filesystem {
        mode: AccessMode::Execute,
        paths: vec![PathBuf::from("/bin")],
    });

    let result = SandboxConfig::from_manifest(&manifest);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        SandboxError::PermissionDenied { .. }
    ));
}

#[test]
fn test_resource_limits_enforcement() {
    let manifest = ToolManifest::new(
        "test-tool".to_string(),
        "1.0.0".to_string(),
        "Test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(32),
        max_execution_time_ms: Some(5000),
        fuel_limit: Some(1_000_000),
    });

    let config = SandboxConfig::from_manifest(&manifest);

    #[cfg(feature = "wasm-runtime")]
    {
        assert!(config.is_ok());
        let config = config.expect("Test operation should succeed");

        assert_eq!(config.resource_limits.max_memory_bytes, 32 * 1024 * 1024);
        assert_eq!(config.resource_limits.max_execution_time_ms, 5000);
        assert_eq!(config.resource_limits.fuel_limit, Some(1_000_000));
    }
}

#[test]
fn test_sandbox_security_level() {
    // Test secure default configuration
    let secure_config = tools::WasiSandboxConfig::secure_default();
    assert_eq!(secure_config.security_level(), 10);

    // Test configuration with some permissions
    let mut permissive_config = tools::WasiSandboxConfig::default();
    let temp_dir = std::env::temp_dir();
    let _ = permissive_config.add_read_only_dir(temp_dir);
    permissive_config.allow_localhost();

    // Should have lower security level
    assert!(permissive_config.security_level() < 10);
}

// Helper functions

fn create_test_manifest() -> ToolManifest {
    ToolManifest::new(
        "test-tool".to_string(),
        "1.0.0".to_string(),
        "A test tool for sandbox validation".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .require_capability(Capability::Filesystem {
        mode: AccessMode::Read,
        paths: vec![PathBuf::from("/tmp")],
    })
}

fn create_minimal_wasm() -> Vec<u8> {
    // Minimal valid WASM module header
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add minimal empty module sections
    // Type section (empty)
    wasm.extend_from_slice(&[0x01, 0x01, 0x00]); // section id, size, empty

    wasm
}

// Integration test with real WASM if available
#[cfg(all(feature = "wasm-runtime", test))]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_execution_flow() {
        let manifest = create_test_manifest();
        let sandbox = WasmSandbox::from_manifest(&manifest).expect("Test operation should succeed");

        // Create a simple WASM module for testing
        let test_wasm = create_simple_add_wasm();
        let module = sandbox.load_module(&test_wasm, "add.wasm".to_string());

        if let Ok(module) = module {
            // Try to execute a function
            let params = vec![tools::WasmValue::I32(5), tools::WasmValue::I32(3)];

            let _result = sandbox.execute_function(&module, "add", params).await;
            // This may fail due to minimal WASM, but should not panic
            // and should properly handle security violations
        }
    }

    fn create_simple_add_wasm() -> Vec<u8> {
        // This would be a real WASM module in a full test
        // For now, return minimal valid header
        create_minimal_wasm()
    }
}
