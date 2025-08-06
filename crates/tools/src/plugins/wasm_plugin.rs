// @component: {"k":"C","id":"wasm_plugin","t":"WebAssembly plugin system with sandboxing","m":{"cur":0,"tgt":90,"u":"%"},"f":["wasm","plugin","sandbox","runtime"]}

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::plugin_manager::{PluginInstance, PluginMetadata, PluginConfiguration, PluginLoader, PluginType};
use crate::{Tool, ToolInput, ToolOutput, ToolSpec};

/// WASM runtime configuration
#[derive(Debug, Clone)]
pub struct WasmConfig {
    pub max_memory_pages: u32,          // 64KB pages
    pub max_execution_time: Duration,
    pub max_fuel: u64,                  // Instruction count limit
    pub enable_debug: bool,
    pub allowed_host_functions: Vec<String>,
    pub memory_limit_mb: u64,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory_pages: 256,  // 16MB max memory
            max_execution_time: Duration::from_secs(30),
            max_fuel: 1_000_000,    // 1M instructions
            enable_debug: false,
            allowed_host_functions: vec![
                "log".to_string(),
                "read_input".to_string(),
                "write_output".to_string(),
            ],
            memory_limit_mb: 16,
        }
    }
}

/// WASM resource limits for execution
#[derive(Debug, Clone)]
pub struct WasmResourceLimits {
    pub memory_limit: u64,
    pub instruction_limit: u64,
    pub call_depth_limit: u32,
    pub table_elements_limit: u32,
    pub execution_timeout: Duration,
}

impl Default for WasmResourceLimits {
    fn default() -> Self {
        Self {
            memory_limit: 16 * 1024 * 1024, // 16MB
            instruction_limit: 1_000_000,
            call_depth_limit: 256,
            table_elements_limit: 1000,
            execution_timeout: Duration::from_secs(30),
        }
    }
}

/// WASM plugin error types
#[derive(Debug, thiserror::Error)]
pub enum WasmPluginError {
    #[error("Runtime error: {0}")]
    Runtime(String),
    
    #[error("Compilation failed: {0}")]
    Compilation(String),
    
    #[error("Module validation failed: {0}")]
    Validation(String),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
    
    #[error("Timeout: execution exceeded {0:?}")]
    Timeout(Duration),
    
    #[error("Host function error: {0}")]
    HostFunction(String),
    
    #[error("Memory access violation: {0}")]
    MemoryViolation(String),
    
    #[error("Security violation: {0}")]
    Security(String),
}

/// WASM sandbox for isolated execution
pub struct WasmSandbox {
    config: WasmConfig,
    resource_limits: WasmResourceLimits,
    execution_stats: Arc<Mutex<ExecutionStats>>,
}

#[derive(Debug, Default)]
pub struct ExecutionStats {
    executions: u64,
    total_fuel_consumed: u64,
    total_execution_time: Duration,
    memory_peak: u64,
    errors: u64,
    timeouts: u64,
}

impl WasmSandbox {
    pub fn new(config: WasmConfig) -> Self {
        let resource_limits = WasmResourceLimits {
            memory_limit: config.memory_limit_mb * 1024 * 1024,
            execution_timeout: config.max_execution_time,
            instruction_limit: config.max_fuel,
            ..Default::default()
        };
        
        Self {
            config,
            resource_limits,
            execution_stats: Arc::new(Mutex::new(ExecutionStats::default())),
        }
    }
    
    /// Execute WASM module with input
    pub async fn execute_wasm(
        &self,
        wasm_bytes: &[u8],
        input: &ToolInput,
    ) -> Result<ToolOutput, WasmPluginError> {
        let start_time = Instant::now();
        
        // Update execution stats
        {
            let mut stats = self.execution_stats.lock().await;
            stats.executions += 1;
        }
        
        // Validate WASM module
        self.validate_wasm_module(wasm_bytes)?;
        
        // Create runtime with resource limits
        let runtime = self.create_runtime().await?;
        
        // Execute with timeout
        let execution_future = self.execute_with_runtime(runtime, wasm_bytes, input);
        let result = match tokio::time::timeout(self.resource_limits.execution_timeout, execution_future).await {
            Ok(result) => result,
            Err(_) => {
                let mut stats = self.execution_stats.lock().await;
                stats.timeouts += 1;
                return Err(WasmPluginError::Timeout(self.resource_limits.execution_timeout));
            }
        };
        
        // Update execution time stats
        let execution_time = start_time.elapsed();
        {
            let mut stats = self.execution_stats.lock().await;
            stats.total_execution_time += execution_time;
            if result.is_err() {
                stats.errors += 1;
            }
        }
        
        result
    }
    
    /// Validate WASM module for security and correctness
    fn validate_wasm_module(&self, wasm_bytes: &[u8]) -> Result<(), WasmPluginError> {
        // Basic validation - in production, use proper WASM parser/validator
        if wasm_bytes.len() < 8 {
            return Err(WasmPluginError::Validation("WASM module too small".to_string()));
        }
        
        // Check WASM magic number
        if &wasm_bytes[0..4] != b"\x00asm" {
            return Err(WasmPluginError::Validation("Invalid WASM magic number".to_string()));
        }
        
        // Check version (should be 1)
        let version = u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            return Err(WasmPluginError::Validation(format!("Unsupported WASM version: {}", version)));
        }
        
        // Additional security checks would go here
        // - Check for dangerous imports
        // - Validate memory usage patterns
        // - Check for potential infinite loops
        
        Ok(())
    }
    
    /// Create configured WASM runtime
    async fn create_runtime(&self) -> Result<WasmRuntime, WasmPluginError> {
        WasmRuntime::new(self.config.clone(), self.resource_limits.clone()).await
    }
    
    /// Execute WASM with runtime
    async fn execute_with_runtime(
        &self,
        mut runtime: WasmRuntime,
        wasm_bytes: &[u8],
        input: &ToolInput,
    ) -> Result<ToolOutput, WasmPluginError> {
        // Load module
        runtime.load_module(wasm_bytes).await?;
        
        // Set up input/output
        runtime.set_input(input).await?;
        
        // Execute main function
        runtime.execute_main().await?;
        
        // Get output
        runtime.get_output().await
    }
    
    /// Get execution statistics
    pub async fn get_stats(&self) -> ExecutionStats {
        let stats = self.execution_stats.lock().await;
        ExecutionStats {
            executions: stats.executions,
            total_fuel_consumed: stats.total_fuel_consumed,
            total_execution_time: stats.total_execution_time,
            memory_peak: stats.memory_peak,
            errors: stats.errors,
            timeouts: stats.timeouts,
        }
    }
}

/// WASM runtime implementation (simplified)
pub struct WasmRuntime {
    #[allow(dead_code)] // Конфигурация WASM runtime
    config: WasmConfig,
    #[allow(dead_code)] // Лимиты ресурсов для WASM
    resource_limits: WasmResourceLimits,
    current_fuel: u64,
    memory_usage: u64,
    input_data: Option<String>,
    output_data: Option<String>,
    host_functions: HashMap<String, Box<dyn HostFunction>>,
}

impl WasmRuntime {
    async fn new(config: WasmConfig, resource_limits: WasmResourceLimits) -> Result<Self, WasmPluginError> {
        let mut runtime = Self {
            config: config.clone(),
            resource_limits,
            current_fuel: 0,
            memory_usage: 0,
            input_data: None,
            output_data: None,
            host_functions: HashMap::new(),
        };
        
        // Register allowed host functions
        runtime.register_host_functions(&config.allowed_host_functions).await?;
        
        Ok(runtime)
    }
    
    async fn register_host_functions(&mut self, allowed_functions: &[String]) -> Result<(), WasmPluginError> {
        for func_name in allowed_functions {
            let host_func: Box<dyn HostFunction> = match func_name.as_str() {
                "log" => Box::new(LogHostFunction),
                "read_input" => Box::new(ReadInputHostFunction),
                "write_output" => Box::new(WriteOutputHostFunction),
                _ => {
                    warn!("Unknown host function requested: {}", func_name);
                    continue;
                }
            };
            
            self.host_functions.insert(func_name.clone(), host_func);
        }
        
        Ok(())
    }
    
    async fn load_module(&mut self, _wasm_bytes: &[u8]) -> Result<(), WasmPluginError> {
        // In a real implementation, this would:
        // 1. Parse the WASM module
        // 2. Validate imports/exports
        // 3. Set up memory and function tables
        // 4. Apply resource limits
        
        debug!("WASM module loaded");
        Ok(())
    }
    
    async fn set_input(&mut self, input: &ToolInput) -> Result<(), WasmPluginError> {
        // Serialize input for WASM module
        let input_json = serde_json::to_string(input)
            .map_err(|e| WasmPluginError::Runtime(format!("Failed to serialize input: {}", e)))?;
        
        self.input_data = Some(input_json);
        Ok(())
    }
    
    async fn execute_main(&mut self) -> Result<(), WasmPluginError> {
        // In a real implementation, this would:
        // 1. Find and call the main/execute function
        // 2. Monitor fuel consumption
        // 3. Handle host function calls
        // 4. Manage memory usage
        
        // Simulate execution
        self.current_fuel = 1000; // Simulated fuel usage
        self.memory_usage = 1024 * 1024; // 1MB simulated usage
        
        // Simulate processing
        if let Some(ref input) = self.input_data {
            let output = self.process_input(input).await?;
            self.output_data = Some(output);
        }
        
        debug!("WASM main function executed");
        Ok(())
    }
    
    async fn process_input(&self, input_json: &str) -> Result<String, WasmPluginError> {
        // Simulate WASM processing
        let input: ToolInput = serde_json::from_str(input_json)
            .map_err(|e| WasmPluginError::Runtime(format!("Failed to deserialize input: {}", e)))?;
        
        // Simple echo processing for demonstration
        let output = ToolOutput {
            success: true,
            result: format!("WASM processed: {}", input.command),
            formatted_output: Some(format!("🦀 WASM Plugin Output:\n{:?}", input.args)),
            metadata: HashMap::new(),
        };
        
        serde_json::to_string(&output)
            .map_err(|e| WasmPluginError::Runtime(format!("Failed to serialize output: {}", e)))
    }
    
    async fn get_output(&self) -> Result<ToolOutput, WasmPluginError> {
        let output_json = self.output_data.as_ref()
            .ok_or_else(|| WasmPluginError::Runtime("No output data available".to_string()))?;
        
        let output: ToolOutput = serde_json::from_str(output_json)
            .map_err(|e| WasmPluginError::Runtime(format!("Failed to deserialize output: {}", e)))?;
        
        Ok(output)
    }
    
    /// Call host function from WASM
    pub async fn call_host_function(&mut self, name: &str, args: &[u8]) -> Result<Vec<u8>, WasmPluginError> {
        // Get the host function first, then call it
        if let Some(_host_func) = self.host_functions.get(name) {
            // Clone function for the call to avoid borrow conflicts
            let func_name = name.to_string();
            match func_name.as_str() {
                "log" => {
                    let message = String::from_utf8(args.to_vec())
                        .map_err(|e| WasmPluginError::HostFunction(format!("Invalid UTF-8 in log message: {}", e)))?;
                    info!("WASM Plugin: {}", message);
                    Ok(Vec::new())
                }
                "read_input" => {
                    let input = self.input_data.as_ref()
                        .ok_or_else(|| WasmPluginError::HostFunction("No input data available".to_string()))?;
                    Ok(input.as_bytes().to_vec())
                }
                "write_output" => {
                    let output = String::from_utf8(args.to_vec())
                        .map_err(|e| WasmPluginError::HostFunction(format!("Invalid UTF-8 in output: {}", e)))?;
                    self.output_data = Some(output);
                    Ok(Vec::new())
                }
                _ => Err(WasmPluginError::HostFunction(format!("Unknown host function: {}", name)))
            }
        } else {
            Err(WasmPluginError::HostFunction(format!("Unknown host function: {}", name)))
        }
    }
}

/// Host function trait for WASM->host communication
trait HostFunction: Send + Sync {
    #[allow(dead_code)] // Метод будет использоваться WASM runtime
    fn call(
        &self,
        runtime: &mut WasmRuntime,
        args: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WasmPluginError>> + Send + '_>>;
}

/// Logging host function
struct LogHostFunction;

impl HostFunction for LogHostFunction {
    fn call(
        &self, 
        _runtime: &mut WasmRuntime, 
        args: &[u8]
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WasmPluginError>> + Send + '_>> {
        let args = args.to_vec();
        Box::pin(async move {
            let message = String::from_utf8(args)
                .map_err(|e| WasmPluginError::HostFunction(format!("Invalid UTF-8 in log message: {}", e)))?;
            
            info!("WASM Plugin: {}", message);
            Ok(Vec::new()) // No return value for log
        })
    }
}

/// Read input host function  
struct ReadInputHostFunction;

impl HostFunction for ReadInputHostFunction {
    fn call(
        &self, 
        runtime: &mut WasmRuntime, 
        _args: &[u8]
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WasmPluginError>> + Send + '_>> {
        let input_data = runtime.input_data.clone();
        Box::pin(async move {
            let input = input_data.as_ref()
                .ok_or_else(|| WasmPluginError::HostFunction("No input data available".to_string()))?;
            
            Ok(input.as_bytes().to_vec())
        })
    }
}

/// Write output host function
struct WriteOutputHostFunction;

impl HostFunction for WriteOutputHostFunction {
    fn call(
        &self, 
        _runtime: &mut WasmRuntime, 
        args: &[u8]
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, WasmPluginError>> + Send + '_>> {
        let args = args.to_vec();
        let output_result = String::from_utf8(args)
            .map_err(|e| WasmPluginError::HostFunction(format!("Invalid UTF-8 in output: {}", e)));
            
        Box::pin(async move {
            let _output = output_result?;
            // Note: В реальной реализации здесь будет shared state для записи output
            // Для демонстрации просто возвращаем успех
            Ok(Vec::new())
        })
    }
}

/// WASM plugin instance
pub struct WasmPlugin {
    metadata: PluginMetadata,
    #[allow(dead_code)] // Конфигурация WASM plugin
    config: PluginConfiguration,
    sandbox: WasmSandbox,
    wasm_bytes: Vec<u8>,
    is_loaded: bool,
}

impl WasmPlugin {
    pub async fn new(
        metadata: PluginMetadata,
        config: PluginConfiguration,
        wasm_path: &Path,
    ) -> Result<Self> {
        // Load WASM bytes from file
        let wasm_bytes = tokio::fs::read(wasm_path).await
            .map_err(|e| anyhow!("Failed to read WASM file: {}", e))?;
        
        // Create sandbox with configuration
        let wasm_config = WasmConfig {
            max_memory_pages: 256, // 16MB
            max_execution_time: Duration::from_secs(30),
            max_fuel: 1_000_000,
            enable_debug: false,
            allowed_host_functions: vec![
                "log".to_string(),
                "read_input".to_string(),
                "write_output".to_string(),
            ],
            memory_limit_mb: 16,
        };
        
        let sandbox = WasmSandbox::new(wasm_config);
        
        Ok(Self {
            metadata,
            config,
            sandbox,
            wasm_bytes,
            is_loaded: false,
        })
    }
}

#[async_trait::async_trait]
impl Tool for WasmPlugin {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.metadata.name.clone(),
            description: self.metadata.description.clone(),
            usage: format!("WASM Plugin: {} v{}", self.metadata.name, self.metadata.version),
            examples: Vec::new(),
            input_schema: self.metadata.configuration_schema.to_string(),
        }
    }
    
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        if !self.is_loaded {
            return Err(anyhow!("WASM plugin not loaded"));
        }
        
        let result = self.sandbox.execute_wasm(&self.wasm_bytes, &input).await
            .map_err(|e| anyhow!("WASM execution failed: {}", e))?;
        
        Ok(result)
    }
    
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        // Basic natural language parsing for WASM plugins
        Ok(ToolInput {
            command: self.metadata.name.clone(),
            args: HashMap::from([("query".to_string(), query.to_string())]),
            context: Some(query.to_string()),
        })
    }
}

#[async_trait::async_trait]
impl PluginInstance for WasmPlugin {
    fn plugin_id(&self) -> &str {
        &self.metadata.id
    }
    
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn is_loaded(&self) -> bool {
        self.is_loaded
    }
    
    async fn start(&mut self) -> Result<()> {
        info!("🚀 Starting WASM plugin: {}", self.metadata.name);
        
        // Validate WASM module
        self.sandbox.validate_wasm_module(&self.wasm_bytes)
            .map_err(|e| anyhow!("WASM validation failed: {}", e))?;
        
        self.is_loaded = true;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        info!("🛑 Stopping WASM plugin: {}", self.metadata.name);
        self.is_loaded = false;
        Ok(())
    }
    
    async fn reload(&mut self) -> Result<()> {
        info!("🔄 Reloading WASM plugin: {}", self.metadata.name);
        
        // Reload WASM bytes if path is available
        if let Some(ref install_path) = self.metadata.installation_path {
            let wasm_path = install_path.join(&self.metadata.entry_point);
            self.wasm_bytes = tokio::fs::read(wasm_path).await?;
        }
        
        // Re-validate
        self.sandbox.validate_wasm_module(&self.wasm_bytes)
            .map_err(|e| anyhow!("WASM validation failed after reload: {}", e))?;
        
        Ok(())
    }
    
    async fn health_check(&self) -> Result<()> {
        if !self.is_loaded {
            return Err(anyhow!("WASM plugin not loaded"));
        }
        
        // Get execution statistics
        let stats = self.sandbox.get_stats().await;
        
        // Check for concerning patterns
        if stats.errors > 0 && stats.executions > 0 {
            let error_rate = stats.errors as f64 / stats.executions as f64;
            if error_rate > 0.1 { // More than 10% error rate
                return Err(anyhow!("WASM plugin has high error rate: {:.1}%", error_rate * 100.0));
            }
        }
        
        if stats.timeouts > stats.executions / 2 {
            return Err(anyhow!("WASM plugin has too many timeouts"));
        }
        
        Ok(())
    }
}

/// WASM plugin loader
pub struct WasmPluginLoader;

#[async_trait::async_trait]
impl PluginLoader for WasmPluginLoader {
    async fn load_plugin(
        &self,
        metadata: &PluginMetadata,
        config: &PluginConfiguration,
    ) -> Result<Box<dyn PluginInstance>> {
        let wasm_path = metadata.installation_path.as_ref()
            .ok_or_else(|| anyhow!("No installation path for WASM plugin"))?
            .join(&metadata.entry_point);
        
        let plugin = WasmPlugin::new(metadata.clone(), config.clone(), &wasm_path).await?;
        Ok(Box::new(plugin))
    }
    
    fn supports_type(&self) -> PluginType {
        PluginType::Wasm
    }
    
    async fn unload_plugin(&self, mut instance: Box<dyn PluginInstance>) -> Result<()> {
        instance.stop().await?;
        info!("🗑️ Unloaded WASM plugin: {}", instance.plugin_id());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_wasm_sandbox_creation() {
        let config = WasmConfig::default();
        let sandbox = WasmSandbox::new(config);
        
        let stats = sandbox.get_stats().await;
        assert_eq!(stats.executions, 0);
    }
    
    #[test]
    fn test_wasm_validation() {
        let config = WasmConfig::default();
        let sandbox = WasmSandbox::new(config);
        
        // Test invalid WASM (too small)
        let invalid_wasm = b"invalid";
        assert!(sandbox.validate_wasm_module(invalid_wasm).is_err());
        
        // Test invalid magic number
        let invalid_magic = b"nope1234";
        assert!(sandbox.validate_wasm_module(invalid_magic).is_err());
        
        // Test valid magic and version
        let valid_start = b"\x00asm\x01\x00\x00\x00";
        assert!(sandbox.validate_wasm_module(valid_start).is_ok());
    }
    
    #[tokio::test]
    async fn test_wasm_runtime_creation() {
        let config = WasmConfig::default();
        let limits = WasmResourceLimits::default();
        
        let runtime = WasmRuntime::new(config, limits).await;
        assert!(runtime.is_ok());
    }
}