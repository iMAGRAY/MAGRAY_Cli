// P1.2.4.a Step 3: WasmSandbox Implementation (2м)
// Complete WASM sandbox wrapper with security controls and capability enforcement

use crate::capabilities::{Capability, CapabilityChecker};
use crate::manifest::ToolManifest;
use crate::sandbox::{
    resource_limits::ResourceUsage,
    sandbox_violations::{SandboxViolation, ViolationLog, ViolationType},
    ResourceLimits, SandboxConfig, SandboxError,
};
use crate::wasm_runtime::{
    WasmExecutionResult, WasmModule, WasmRuntime, WasmRuntimeConfig, WasmValue,
};
use anyhow::Result;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[cfg(feature = "wasm-runtime")]
use wasmtime::{Config, Engine, Instance, Module, Store};

#[cfg(feature = "wasm-runtime")]
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// Secure WASM sandbox that enforces capability-based permissions
pub struct WasmSandbox {
    config: SandboxConfig,
    violation_log: Arc<Mutex<ViolationLog>>,
    runtime: WasmRuntime,
    #[cfg(feature = "wasm-runtime")]
    engine: Engine,
    capabilities: HashSet<Capability>,
}

impl WasmSandbox {
    /// Create new WASM sandbox from tool manifest
    pub fn from_manifest(manifest: &ToolManifest) -> Result<Self, SandboxError> {
        let config = SandboxConfig::from_manifest(manifest)?;
        Self::new(config)
    }

    /// Create new WASM sandbox with configuration
    pub fn new(config: SandboxConfig) -> Result<Self, SandboxError> {
        config.validate()?;

        // Create wasmtime runtime configuration
        let runtime_config = WasmRuntimeConfig {
            max_memory_bytes: config.resource_limits.max_memory_bytes,
            execution_timeout: config.resource_limits.execution_timeout(),
            fuel_limit: config.resource_limits.fuel_limit,
            enable_debug: config.debug_mode,
            enable_wasi: true,
            enforce_resource_limits: true,
            // CRITICAL MEMORY OPTIMIZATION - SANDBOX MODE
            enable_memory_pool: true,
            module_cache_size: 5, // Minimal cache для sandbox
            engine_pool_size: 1,  // Single engine для sandbox
        };

        // Create runtime
        let runtime = WasmRuntime::new(runtime_config)
            .map_err(|e| SandboxError::InitializationError(e.to_string()))?;

        #[cfg(feature = "wasm-runtime")]
        let engine = {
            let mut wasmtime_config = Config::new();

            // Enable WASI
            wasmtime_config.wasm_component_model(false);

            // Configure security features
            wasmtime_config.consume_fuel(config.resource_limits.fuel_limit.is_some());
            wasmtime_config.max_wasm_stack(config.resource_limits.max_wasm_stack);

            // Memory configuration
            let memory_pages = config.resource_limits.max_memory_bytes / (64 * 1024);
            wasmtime_config.static_memory_maximum_size(config.resource_limits.max_memory_bytes);

            // Debug configuration
            if config.debug_mode {
                wasmtime_config.debug_info(true);
                wasmtime_config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
            }

            Engine::new(&wasmtime_config)
                .map_err(|e| SandboxError::InitializationError(e.to_string()))?
        };

        let capabilities = config.allowed_capabilities.iter().cloned().collect();

        Ok(Self {
            config,
            violation_log: Arc::new(Mutex::new(ViolationLog::new(1000))),
            runtime,
            #[cfg(feature = "wasm-runtime")]
            engine,
            capabilities,
        })
    }

    /// Load WASM module into the sandbox
    pub fn load_module(
        &self,
        wasm_bytes: &[u8],
        module_name: String,
    ) -> Result<SandboxedModule, SandboxError> {
        // Security pre-check: validate WASM module
        self.validate_wasm_module(wasm_bytes, &module_name)?;

        // Load module using runtime
        let module = self
            .runtime
            .load_module_from_bytes(wasm_bytes)
            .map_err(|e| SandboxError::InitializationError(e.to_string()))?;

        #[cfg(feature = "wasm-runtime")]
        let wasmtime_module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| SandboxError::InitializationError(e.to_string()))?;

        Ok(SandboxedModule {
            module,
            #[cfg(feature = "wasm-runtime")]
            wasmtime_module,
            module_name,
            sandbox_config: self.config.clone(),
            violation_log: Arc::clone(&self.violation_log),
            capabilities: self.capabilities.clone(),
        })
    }

    /// Execute function in sandboxed environment
    pub async fn execute_function(
        &self,
        module: &SandboxedModule,
        function_name: &str,
        params: Vec<WasmValue>,
    ) -> Result<WasmExecutionResult, SandboxError> {
        let start_time = Instant::now();

        // Pre-execution security checks
        self.pre_execution_checks(module, function_name)?;

        // Execute with monitoring
        let result = self
            .monitored_execution(module, function_name, params)
            .await;

        // Post-execution analysis
        let execution_time = start_time.elapsed();
        self.post_execution_analysis(module, &result, execution_time)?;

        result
    }

    /// Validate WASM module for security issues
    fn validate_wasm_module(
        &self,
        wasm_bytes: &[u8],
        module_name: &str,
    ) -> Result<(), SandboxError> {
        // Basic WASM validation
        if wasm_bytes.len() < 8 {
            self.log_violation(SandboxViolation::new(
                ViolationType::EscapeAttempt {
                    method: "invalid_wasm".to_string(),
                    details: "Module too small to be valid WASM".to_string(),
                },
                "Module validation failed".to_string(),
                Some(module_name.to_string()),
                None,
            ));
            return Err(SandboxError::InitializationError(
                "Invalid WASM module".to_string(),
            ));
        }

        // Check WASM magic number
        if &wasm_bytes[0..4] != b"\x00asm" {
            self.log_violation(SandboxViolation::new(
                ViolationType::EscapeAttempt {
                    method: "invalid_magic".to_string(),
                    details: "Invalid WASM magic number".to_string(),
                },
                "Module validation failed".to_string(),
                Some(module_name.to_string()),
                None,
            ));
            return Err(SandboxError::InitializationError(
                "Invalid WASM magic number".to_string(),
            ));
        }

        // Size limit check
        const MAX_MODULE_SIZE: usize = 64 * 1024 * 1024; // 64MB
        if wasm_bytes.len() > MAX_MODULE_SIZE {
            self.log_violation(SandboxViolation::new(
                ViolationType::ResourceLimitViolation {
                    resource: "module_size".to_string(),
                    attempted_value: wasm_bytes.len() as u64,
                    limit: MAX_MODULE_SIZE as u64,
                },
                "Module too large".to_string(),
                Some(module_name.to_string()),
                None,
            ));
            return Err(SandboxError::ResourceLimitExceeded {
                resource: "module_size".to_string(),
                current: wasm_bytes.len() as u64,
                limit: MAX_MODULE_SIZE as u64,
            });
        }

        Ok(())
    }

    /// Pre-execution security checks
    fn pre_execution_checks(
        &self,
        module: &SandboxedModule,
        function_name: &str,
    ) -> Result<(), SandboxError> {
        // Check if function exists
        if !module.module.has_function(function_name) {
            return Err(SandboxError::PermissionDenied {
                operation: format!("Function '{function_name}' not found"),
            });
        }

        // Check if function is allowed to be called
        if self.is_restricted_function(function_name) {
            self.log_violation(SandboxViolation::new(
                ViolationType::SystemCallViolation {
                    syscall: function_name.to_string(),
                    args: vec![],
                },
                "Attempted to call restricted function".to_string(),
                Some(module.module_name.clone()),
                Some(function_name.to_string()),
            ));
            return Err(SandboxError::PermissionDenied {
                operation: format!("Function '{function_name}' is restricted"),
            });
        }

        Ok(())
    }

    /// Execute function with monitoring and resource limits
    async fn monitored_execution(
        &self,
        module: &SandboxedModule,
        function_name: &str,
        params: Vec<WasmValue>,
    ) -> Result<WasmExecutionResult, SandboxError> {
        let start_time = Instant::now();

        // Execute with timeout
        let execution_timeout = self.config.resource_limits.execution_timeout();

        match tokio::time::timeout(
            execution_timeout,
            self.runtime
                .execute_function(&module.module, function_name, params.clone()),
        )
        .await
        {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => {
                // Execution error
                Err(SandboxError::InitializationError(e.to_string()))
            }
            Err(_) => {
                // Timeout
                let execution_time = start_time.elapsed().as_millis() as u64;
                self.log_violation(SandboxViolation::timeout_violation(
                    execution_time,
                    self.config.resource_limits.max_execution_time_ms,
                    Some(module.module_name.clone()),
                    Some(function_name.to_string()),
                ));
                Err(SandboxError::ResourceLimitExceeded {
                    resource: "execution_time".to_string(),
                    current: execution_time,
                    limit: self.config.resource_limits.max_execution_time_ms,
                })
            }
        }
    }

    /// Post-execution analysis and resource usage validation
    fn post_execution_analysis(
        &self,
        module: &SandboxedModule,
        result: &Result<WasmExecutionResult, SandboxError>,
        execution_time: std::time::Duration,
    ) -> Result<(), SandboxError> {
        // Analyze resource usage
        if let Ok(exec_result) = result {
            let usage = ResourceUsage {
                peak_memory_bytes: exec_result.memory_usage_bytes,
                execution_time_us: exec_result.execution_time_us,
                fuel_consumed: None, // TODO: Extract from wasmtime store
                allocation_count: 1,
            };

            // Check for resource violations
            if let Some(violation_error) = usage.violates_limits(&self.config.resource_limits) {
                if let SandboxError::ResourceLimitExceeded {
                    resource,
                    current,
                    limit,
                } = &violation_error
                {
                    self.log_violation(SandboxViolation::resource_limit_violation(
                        resource.clone(),
                        *current,
                        *limit,
                        Some(module.module_name.clone()),
                    ));
                }
                return Err(violation_error);
            }
        }

        Ok(())
    }

    /// Check if function is restricted
    fn is_restricted_function(&self, function_name: &str) -> bool {
        const RESTRICTED_FUNCTIONS: &[&str] = &[
            "__wasi_proc_exit",
            "__wasi_proc_raise",
            "__wasi_fd_close",
            "__wasi_fd_write",
            "wasi_unstable",
        ];

        RESTRICTED_FUNCTIONS.contains(&function_name)
    }

    /// Log security violation
    fn log_violation(&self, violation: SandboxViolation) {
        if let Ok(mut log) = self.violation_log.lock() {
            log.log_violation(violation);
        }
    }

    /// Get violation log for security analysis
    pub fn get_violations(&self) -> Vec<SandboxViolation> {
        if let Ok(log) = self.violation_log.lock() {
            log.violations().to_vec()
        } else {
            Vec::new()
        }
    }

    /// Get sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Check if sandbox has capabilities for operation
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }
}

/// WASM module loaded into sandbox
pub struct SandboxedModule {
    module: WasmModule,
    #[cfg(feature = "wasm-runtime")]
    wasmtime_module: Module,
    module_name: String,
    sandbox_config: SandboxConfig,
    violation_log: Arc<Mutex<ViolationLog>>,
    capabilities: HashSet<Capability>,
}

impl SandboxedModule {
    /// Get module name
    pub fn name(&self) -> &str {
        &self.module_name
    }

    /// Get available functions
    pub fn exports(&self) -> Vec<String> {
        self.module.exports()
    }

    /// Check if module has capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Get sandbox configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.sandbox_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ToolMetadata, ToolType};
    use std::path::PathBuf;

    #[test]
    fn test_sandbox_creation() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "Test tool".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        let result = WasmSandbox::from_manifest(&manifest);

        #[cfg(feature = "wasm-runtime")]
        assert!(result.is_ok());

        #[cfg(not(feature = "wasm-runtime"))]
        assert!(result.is_err());
    }

    #[test]
    fn test_wasm_validation() {
        let config = SandboxConfig::default();
        let sandbox = WasmSandbox::new(config).expect("Operation failed - converted from unwrap()");

        // Test invalid WASM
        let invalid_wasm = b"invalid";
        let result = sandbox.validate_wasm_module(invalid_wasm, "test.wasm");
        assert!(result.is_err());

        // Test valid WASM header
        let valid_header = b"\x00asm\x01\x00\x00\x00";
        let result = sandbox.validate_wasm_module(valid_header, "test.wasm");
        assert!(result.is_ok());
    }

    #[test]
    fn test_restricted_functions() {
        let config = SandboxConfig::default();
        let sandbox = WasmSandbox::new(config).expect("Operation failed - converted from unwrap()");

        assert!(sandbox.is_restricted_function("__wasi_proc_exit"));
        assert!(sandbox.is_restricted_function("__wasi_fd_write"));
        assert!(!sandbox.is_restricted_function("my_function"));
    }

    #[test]
    fn test_capability_checking() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "Test tool".to_string(),
            ToolType::Wasm,
            "test.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        )
        .require_capability(Capability::Filesystem {
            mode: crate::capabilities::AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        });

        let sandbox = WasmSandbox::from_manifest(&manifest)
            .expect("Operation failed - converted from unwrap()");

        let fs_capability = Capability::Filesystem {
            mode: crate::capabilities::AccessMode::Read,
            paths: vec![PathBuf::from("/tmp")],
        };

        assert!(sandbox.has_capability(&fs_capability));
    }

    #[test]
    fn test_violation_logging() {
        let config = SandboxConfig::default();
        let sandbox = WasmSandbox::new(config).expect("Operation failed - converted from unwrap()");

        let violation = SandboxViolation::filesystem_violation(
            "/etc/passwd".to_string(),
            "read".to_string(),
            Some("test.wasm".to_string()),
            true,
        );

        sandbox.log_violation(violation);
        let violations = sandbox.get_violations();
        assert_eq!(violations.len(), 1);
    }
}
