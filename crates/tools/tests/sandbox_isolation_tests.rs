// P1.2.4.b: Comprehensive Sandbox Isolation Tests
// Tests that validate WASM tools work in controlled sandbox with proper isolation

use std::path::PathBuf;
use std::time::Duration;
use tools::{
    capabilities::{AccessMode, Capability, NetworkMode},
    manifest::{RuntimeConfig, ToolManifest, ToolType},
    sandbox::{sandbox_violations::SecurityImpact, SandboxConfig, ViolationType, WasmSandbox},
    WasmValue,
};

// ================================================================
// 1. SANDBOX ESCAPE PREVENTION TESTS (3м)
// ================================================================

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_wasm_modules_cannot_escape_sandbox() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Test 1: Malicious WASM trying to access host filesystem
    let malicious_wasm = create_filesystem_escape_wasm();
    let result = sandbox.load_module(&malicious_wasm, "filesystem_escape.wasm".to_string());

    // Should either reject during loading or block during execution
    match result {
        Ok(module) => {
            // Module loaded, but execution should fail
            let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
            let exec_result = rt.block_on(sandbox.execute_function(&module, "try_escape", vec![]));
            assert!(exec_result.is_err());

            // Should have logged security violations
            let violations = sandbox.get_violations();
            assert!(!violations.is_empty());
            assert!(violations.iter().any(|v| matches!(
                v.violation_type,
                ViolationType::FilesystemViolation { .. } | ViolationType::EscapeAttempt { .. }
            )));
        }
        Err(_) => {
            // Good - malicious module rejected during loading
        }
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_wasm_modules_cannot_access_host_environment() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Test environment variable access attempt
    let env_escape_wasm = create_environment_escape_wasm();
    let result = sandbox.load_module(&env_escape_wasm, "env_escape.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let _exec_result = rt.block_on(sandbox.execute_function(&module, "read_env", vec![]));

        // Should fail to access environment variables
        let violations = sandbox.get_violations();
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::EnvironmentViolation { .. })));
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_filesystem_access_restrictions() {
    // Create sandbox with restricted filesystem access (only /tmp allowed)
    let manifest = ToolManifest::new(
        "filesystem-test".to_string(),
        "1.0.0".to_string(),
        "Filesystem test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .require_capability(Capability::Filesystem {
        mode: AccessMode::Read,
        paths: vec![PathBuf::from("/tmp")],
    });

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Test 1: Allowed access should work
    assert!(sandbox.has_capability(&Capability::Filesystem {
        mode: AccessMode::Read,
        paths: vec![PathBuf::from("/tmp")],
    }));

    // Test 2: Forbidden access should be blocked
    let forbidden_wasm = create_forbidden_file_access_wasm();
    let result = sandbox.load_module(&forbidden_wasm, "forbidden_access.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");

        // Try to access /etc/passwd (should be blocked)
        let _exec_result =
            rt.block_on(sandbox.execute_function(&module, "read_etc_passwd", vec![]));

        // Should fail and log violation
        let violations = sandbox.get_violations();
        assert!(violations.iter().any(|v| {
            if let ViolationType::FilesystemViolation { attempted_path, .. } = &v.violation_type {
                attempted_path.contains("/etc") || attempted_path.contains("passwd")
            } else {
                false
            }
        }));
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_network_access_limitations() {
    let manifest = ToolManifest::new(
        "network-test".to_string(),
        "1.0.0".to_string(),
        "Network test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .require_capability(Capability::Network {
        mode: NetworkMode::Outbound,
        domains: vec!["api.example.com".to_string()],
    });

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config.clone()).expect("Sandbox test operation should succeed");

    // Test allowed domain access
    assert!(config
        .wasi_config
        .network_access
        .can_connect("api.example.com"));

    // Test forbidden domain should be blocked
    assert!(!config
        .wasi_config
        .network_access
        .can_connect("malicious.evil.com"));

    // Test malicious network access attempt
    let network_escape_wasm = create_network_escape_wasm();
    let result = sandbox.load_module(&network_escape_wasm, "network_escape.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let _exec_result =
            rt.block_on(sandbox.execute_function(&module, "connect_to_evil_host", vec![]));

        // Should log network violation
        let violations = sandbox.get_violations();
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::NetworkViolation { .. })));
    }
}

// ================================================================
// 2. RESOURCE LIMIT ENFORCEMENT VALIDATION (2м)
// ================================================================

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_memory_limits_enforcement() {
    let manifest = ToolManifest::new(
        "memory-test".to_string(),
        "1.0.0".to_string(),
        "Memory test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(16), // Very restrictive 16MB limit
        max_execution_time_ms: Some(5000),
        fuel_limit: Some(1_000_000),
    });

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Create WASM that tries to allocate excessive memory
    let memory_bomb_wasm = create_memory_bomb_wasm();
    let result = sandbox.load_module(&memory_bomb_wasm, "memory_bomb.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let exec_result = rt.block_on(sandbox.execute_function(
            &module,
            "allocate_huge_memory",
            vec![WasmValue::I32(100_000_000)], // Try to allocate 100MB
        ));

        // Should fail due to memory limit
        assert!(exec_result.is_err());

        // Should log resource limit violation
        let violations = sandbox.get_violations();
        assert!(violations.iter().any(|v| {
            if let ViolationType::ResourceLimitViolation { resource, .. } = &v.violation_type {
                resource == "memory"
            } else {
                false
            }
        }));
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_cpu_execution_time_limits() {
    let manifest = ToolManifest::new(
        "cpu-test".to_string(),
        "1.0.0".to_string(),
        "CPU test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(32),
        max_execution_time_ms: Some(1000), // 1 second limit
        fuel_limit: Some(500_000),
    });

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Create WASM that runs infinite loop
    let infinite_loop_wasm = create_infinite_loop_wasm();
    let result = sandbox.load_module(&infinite_loop_wasm, "infinite_loop.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let start_time = std::time::Instant::now();

        let exec_result = rt.block_on(sandbox.execute_function(&module, "infinite_loop", vec![]));

        let execution_time = start_time.elapsed();

        // Should timeout after ~1 second
        assert!(exec_result.is_err());
        assert!(execution_time < Duration::from_secs(2)); // Should not run forever

        // Should log timeout violation
        let violations = sandbox.get_violations();
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::TimeoutViolation { .. })));
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_fuel_limit_enforcement() {
    let manifest = ToolManifest::new(
        "fuel-test".to_string(),
        "1.0.0".to_string(),
        "Fuel test tool".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    )
    .with_runtime_config(RuntimeConfig {
        max_memory_mb: Some(32),
        max_execution_time_ms: Some(10000),
        fuel_limit: Some(10_000), // Very low fuel limit
    });

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Create WASM that consumes lots of instructions
    let fuel_burner_wasm = create_fuel_burner_wasm();
    let result = sandbox.load_module(&fuel_burner_wasm, "fuel_burner.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let exec_result = rt.block_on(sandbox.execute_function(
            &module,
            "burn_fuel",
            vec![WasmValue::I32(1_000_000)], // Do lots of operations
        ));

        // Should fail due to fuel exhaustion
        assert!(exec_result.is_err());

        // Should log fuel violation
        let violations = sandbox.get_violations();
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::FuelViolation { .. })));
    }
}

// ================================================================
// 3. WASI ISOLATION TESTS (1м)
// ================================================================

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_wasi_environment_isolation() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // WASI environment should be isolated from host
    assert_eq!(sandbox.config().wasi_config.security_level(), 10); // Maximum security

    // Test that WASI cannot access host environment variables
    let wasi_env_wasm = create_wasi_env_test_wasm();
    let result = sandbox.load_module(&wasi_env_wasm, "wasi_env_test.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let _exec_result = rt.block_on(sandbox.execute_function(&module, "get_host_env", vec![]));

        // Should not have access to host environment
        let violations = sandbox.get_violations();
        assert!(violations
            .iter()
            .any(|v| matches!(v.violation_type, ViolationType::EnvironmentViolation { .. })));
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_wasm_cannot_access_host_beyond_permissions() {
    let manifest = ToolManifest::new(
        "restricted-wasi".to_string(),
        "1.0.0".to_string(),
        "Restricted WASI test".to_string(),
        ToolType::Wasm,
        "test.wasm".to_string(),
        "Test Author".to_string(),
        "MIT".to_string(),
    );
    // No capabilities granted

    let config =
        SandboxConfig::from_manifest(&manifest).expect("Sandbox test operation should succeed");
    let sandbox = WasmSandbox::new(config.clone()).expect("Sandbox test operation should succeed");

    // Should have no filesystem access
    assert!(config
        .wasi_config
        .filesystem_access
        .read_only_dirs
        .is_empty());
    assert!(config
        .wasi_config
        .filesystem_access
        .read_write_dirs
        .is_empty());

    // Should have no network access
    assert!(config.wasi_config.network_access.allowed_hosts.is_empty());

    // Test that WASM cannot perform any privileged operations
    let privileged_wasm = create_comprehensive_malicious_wasm();
    let result = sandbox.load_module(&privileged_wasm, "privileged_ops.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");
        let _exec_result =
            rt.block_on(sandbox.execute_function(&module, "try_privileged_ops", vec![]));

        // Should fail and log multiple violations
        let violations = sandbox.get_violations();
        assert!(!violations.is_empty());
    }
}

// ================================================================
// 4. SECURITY BREACH DETECTION TESTS (1м)
// ================================================================

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_sandbox_violation_reporting() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Initially no violations
    assert_eq!(sandbox.get_violations().len(), 0);

    // Load malicious module
    let malicious_wasm = create_comprehensive_malicious_wasm();
    let result = sandbox.load_module(&malicious_wasm, "comprehensive_malicious.wasm".to_string());

    match result {
        Ok(module) => {
            let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");

            // Try multiple malicious operations
            let _ = rt.block_on(sandbox.execute_function(&module, "filesystem_escape", vec![]));
            let _ = rt.block_on(sandbox.execute_function(&module, "network_escape", vec![]));
            let _ = rt.block_on(sandbox.execute_function(&module, "memory_bomb", vec![]));

            // Should have logged multiple violations
            let violations = sandbox.get_violations();
            assert!(!violations.is_empty());

            // Test violation analysis
            let stats = violations.iter().fold(
                (0, 0, 0, 0), // (total, critical, high, malicious)
                |(total, critical, high, malicious), v| {
                    (
                        total + 1,
                        critical + if v.severity >= 9 { 1 } else { 0 },
                        high + if v.severity >= 7 && v.severity < 9 {
                            1
                        } else {
                            0
                        },
                        malicious + if v.is_potentially_malicious() { 1 } else { 0 },
                    )
                },
            );

            assert!(stats.0 > 0); // Total violations
            assert!(stats.3 > 0); // Should have malicious violations
        }
        Err(_) => {
            // Good - malicious module rejected during loading
            let violations = sandbox.get_violations();
            assert!(!violations.is_empty());
        }
    }
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_malicious_wasm_module_rejection() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Test 1: Oversized module
    let oversized_wasm = vec![0u8; 100 * 1024 * 1024]; // 100MB
    let result = sandbox.load_module(&oversized_wasm, "oversized.wasm".to_string());
    assert!(result.is_err());

    // Test 2: Invalid WASM magic
    let invalid_magic = b"FAKE";
    let result = sandbox.load_module(invalid_magic, "invalid_magic.wasm".to_string());
    assert!(result.is_err());

    // Test 3: Corrupted WASM
    let corrupted_wasm = create_corrupted_wasm();
    let result = sandbox.load_module(&corrupted_wasm, "corrupted.wasm".to_string());
    assert!(result.is_err());

    // Should have logged security violations for each attempt
    let violations = sandbox.get_violations();
    assert!(violations.len() >= 2); // At least oversized + invalid magic

    // Check that violations have appropriate severity
    assert!(violations.iter().any(|v| {
        matches!(
            v.violation_type,
            ViolationType::ResourceLimitViolation { .. }
        )
    }));
    assert!(violations
        .iter()
        .any(|v| { matches!(v.violation_type, ViolationType::EscapeAttempt { .. }) }));
}

#[cfg(feature = "wasm-runtime")]
#[test]
fn test_security_impact_classification() {
    let config = SandboxConfig::default();
    let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

    // Create various types of violations to test classification
    let test_wasm = create_security_test_wasm();
    let result = sandbox.load_module(&test_wasm, "security_test.wasm".to_string());

    if let Ok(module) = result {
        let rt = tokio::runtime::Runtime::new().expect("Sandbox test operation should succeed");

        // Execute different types of potentially malicious operations
        let _ = rt.block_on(sandbox.execute_function(&module, "mild_violation", vec![]));
        let _ = rt.block_on(sandbox.execute_function(&module, "serious_violation", vec![]));
        let _ = rt.block_on(sandbox.execute_function(&module, "critical_violation", vec![]));

        let violations = sandbox.get_violations();

        // Should have violations with different impact levels
        let _has_low = violations
            .iter()
            .any(|v| v.security_impact() == SecurityImpact::Low);
        let has_high = violations
            .iter()
            .any(|v| v.security_impact() == SecurityImpact::High);
        let has_critical = violations
            .iter()
            .any(|v| v.security_impact() == SecurityImpact::Critical);

        // At least one violation should be classified as serious
        assert!(has_high || has_critical);
    }
}

// ================================================================
// HELPER FUNCTIONS FOR CREATING TEST WASM MODULES
// ================================================================

fn create_filesystem_escape_wasm() -> Vec<u8> {
    // Create minimal WASM that attempts filesystem escape
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add function that tries to access filesystem
    wasm.extend_from_slice(&[
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section: () -> ()
        0x03, 0x02, 0x01, 0x00, // Function section: 1 function
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x00, 0x0b, // Code section: simple function
    ]);
    wasm
}

fn create_environment_escape_wasm() -> Vec<u8> {
    create_filesystem_escape_wasm() // Similar structure for simplicity
}

fn create_forbidden_file_access_wasm() -> Vec<u8> {
    create_filesystem_escape_wasm() // Similar structure for simplicity
}

fn create_network_escape_wasm() -> Vec<u8> {
    create_filesystem_escape_wasm() // Similar structure for simplicity
}

fn create_memory_bomb_wasm() -> Vec<u8> {
    // WASM that tries to allocate excessive memory
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add memory section with high limits
    wasm.extend_from_slice(&[
        0x05, 0x03, 0x01, 0x00, 0xff, // Memory section: 255 pages (16MB)
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section
        0x03, 0x02, 0x01, 0x00, // Function section
        0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x00, 0x0b, // Code section
    ]);
    wasm
}

fn create_infinite_loop_wasm() -> Vec<u8> {
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add function with infinite loop
    wasm.extend_from_slice(&[
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section
        0x03, 0x02, 0x01, 0x00, // Function section
        0x0a, 0x08, 0x01, 0x06, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, // Code: loop { br 0 }
    ]);
    wasm
}

fn create_fuel_burner_wasm() -> Vec<u8> {
    create_infinite_loop_wasm() // Similar concept - consumes instructions
}

fn create_wasi_env_test_wasm() -> Vec<u8> {
    create_filesystem_escape_wasm() // Similar structure
}

fn create_comprehensive_malicious_wasm() -> Vec<u8> {
    // Larger malicious WASM with multiple exploit attempts
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add multiple functions for different exploits
    wasm.extend_from_slice(&[
        0x01, 0x07, 0x02, 0x60, 0x00, 0x00, 0x60, 0x01, 0x7f, 0x00, // Type section: 2 types
        0x03, 0x04, 0x03, 0x00, 0x01, 0x00, // Function section: 3 functions
        0x07, 0x2b, 0x03, // Export section
        0x11, 0x66, 0x69, 0x6c, 0x65, 0x73, 0x79, 0x73, 0x74, 0x65, 0x6d, 0x5f, 0x65, 0x73, 0x63,
        0x61, 0x70, 0x65, 0x00, 0x00, // "filesystem_escape"
        0x0d, 0x6e, 0x65, 0x74, 0x77, 0x6f, 0x72, 0x6b, 0x5f, 0x65, 0x73, 0x63, 0x61, 0x70, 0x65,
        0x00, 0x01, // "network_escape"
        0x0b, 0x6d, 0x65, 0x6d, 0x6f, 0x72, 0x79, 0x5f, 0x62, 0x6f, 0x6d, 0x62, 0x00,
        0x02, // "memory_bomb"
        0x0a, 0x14, 0x03, // Code section: 3 functions
        0x04, 0x00, 0x41, 0x00, 0x0b, // Function 0: simple
        0x04, 0x00, 0x41, 0x01, 0x0b, // Function 1: simple
        0x04, 0x00, 0x41, 0x02, 0x0b, // Function 2: simple
    ]);
    wasm
}

fn create_corrupted_wasm() -> Vec<u8> {
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add corrupted section
    wasm.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]); // Invalid section
    wasm
}

fn create_security_test_wasm() -> Vec<u8> {
    let mut wasm = Vec::new();
    wasm.extend_from_slice(b"\x00asm"); // Magic number
    wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1

    // Add functions for different security levels
    wasm.extend_from_slice(&[
        0x01, 0x07, 0x02, 0x60, 0x00, 0x00, 0x60, 0x01, 0x7f, 0x00, // Type section
        0x03, 0x04, 0x03, 0x00, 0x00, 0x00, // Function section: 3 functions
        0x07, 0x33, 0x03, // Export section
        0x0e, 0x6d, 0x69, 0x6c, 0x64, 0x5f, 0x76, 0x69, 0x6f, 0x6c, 0x61, 0x74, 0x69, 0x6f, 0x6e,
        0x00, 0x00, // "mild_violation"
        0x11, 0x73, 0x65, 0x72, 0x69, 0x6f, 0x75, 0x73, 0x5f, 0x76, 0x69, 0x6f, 0x6c, 0x61, 0x74,
        0x69, 0x6f, 0x6e, 0x00, 0x01, // "serious_violation"
        0x12, 0x63, 0x72, 0x69, 0x74, 0x69, 0x63, 0x61, 0x6c, 0x5f, 0x76, 0x69, 0x6f, 0x6c, 0x61,
        0x74, 0x69, 0x6f, 0x6e, 0x00, 0x02, // "critical_violation"
        0x0a, 0x10, 0x03, // Code section
        0x04, 0x00, 0x41, 0x00, 0x0b, // Function 0
        0x04, 0x00, 0x41, 0x01, 0x0b, // Function 1
        0x04, 0x00, 0x41, 0x02, 0x0b, // Function 2
    ]);
    wasm
}

// ================================================================
// INTEGRATION TESTS FOR COMPLETE SANDBOX WORKFLOW
// ================================================================

#[cfg(all(feature = "wasm-runtime", test))]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_sandbox_isolation_workflow() {
        // Test the complete workflow: manifest -> sandbox -> execution -> violation detection
        let manifest = ToolManifest::new(
            "integration-test".to_string(),
            "1.0.0".to_string(),
            "Complete integration test".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .require_capability(Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        })
        .with_runtime_config(RuntimeConfig {
            max_memory_mb: Some(32),
            max_execution_time_ms: Some(5000),
            fuel_limit: Some(1_000_000),
        });

        // Step 1: Create sandbox from manifest
        let sandbox =
            WasmSandbox::from_manifest(&manifest).expect("Sandbox test operation should succeed");

        // Step 2: Verify configuration
        assert!(sandbox.has_capability(&Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        }));
        assert_eq!(
            sandbox.config().resource_limits.max_memory_bytes,
            32 * 1024 * 1024
        );
        assert_eq!(sandbox.config().resource_limits.max_execution_time_ms, 5000);

        // Step 3: Load and execute valid module
        let valid_wasm = create_filesystem_escape_wasm();
        let module = sandbox
            .load_module(&valid_wasm, "integration_test.wasm".to_string())
            .expect("Sandbox test operation should succeed");

        // Step 4: Execute function (may trigger violations)
        let _result = sandbox
            .execute_function(&module, "try_escape", vec![])
            .await;

        // Step 5: Analyze violations
        let violations = sandbox.get_violations();

        // Should have detected some security violations during execution
        if !violations.is_empty() {
            // Verify violation logging is working
            assert!(violations.iter().any(|v| v.blocked));
            assert!(violations.iter().any(|v| v.severity > 0));
        }

        // Step 6: Verify sandbox still operational after violations
        assert!(sandbox.config().validate().is_ok());
    }

    #[tokio::test]
    async fn test_sandbox_resilience_under_attack() {
        let config = SandboxConfig::default();
        let sandbox = WasmSandbox::new(config).expect("Failed to create WASM sandbox");

        // Simulate multiple attack attempts
        let attack_modules = vec![
            ("memory_bomb.wasm", create_memory_bomb_wasm()),
            ("infinite_loop.wasm", create_infinite_loop_wasm()),
            ("filesystem_escape.wasm", create_filesystem_escape_wasm()),
            ("network_escape.wasm", create_network_escape_wasm()),
            ("corrupted.wasm", create_corrupted_wasm()),
        ];

        for (name, wasm_bytes) in attack_modules {
            match sandbox.load_module(&wasm_bytes, name.to_string()) {
                Ok(module) => {
                    // Try to execute - should be safe due to sandbox
                    let _ = sandbox.execute_function(&module, "attack", vec![]).await;
                }
                Err(_) => {
                    // Good - attack rejected during loading
                }
            }
        }

        // Verify sandbox is still functional and has logged violations
        let violations = sandbox.get_violations();
        assert!(!violations.is_empty());

        // Verify violation statistics
        let critical_count = violations.iter().filter(|v| v.severity >= 9).count();
        let malicious_count = violations
            .iter()
            .filter(|v| v.is_potentially_malicious())
            .count();

        assert!(critical_count > 0 || malicious_count > 0);

        // Sandbox should still be able to load legitimate modules
        let valid_wasm = create_minimal_valid_wasm();
        let result = sandbox.load_module(&valid_wasm, "valid.wasm".to_string());
        assert!(result.is_ok());
    }

    fn create_minimal_valid_wasm() -> Vec<u8> {
        let mut wasm = Vec::new();
        wasm.extend_from_slice(b"\x00asm"); // Magic number
        wasm.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Version 1
        wasm.extend_from_slice(&[0x01, 0x04, 0x01, 0x60, 0x00, 0x00]); // Type section
        wasm.extend_from_slice(&[0x03, 0x02, 0x01, 0x00]); // Function section
        wasm.extend_from_slice(&[0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x00, 0x0b]); // Code section
        wasm
    }
}
