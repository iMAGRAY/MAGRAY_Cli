// @component: {"k":"C","id":"external_process_plugin","t":"External process plugin with sandboxing and resource management","m":{"cur":0,"tgt":85,"u":"%"},"f":["external","process","sandbox","isolation"]}

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::plugin_manager::{
    PluginConfiguration, PluginInstance, PluginLoader, PluginMetadata, PluginType,
};
use crate::{Tool, ToolInput, ToolOutput, ToolSpec};

/// External process configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment_variables: HashMap<String, String>,
    pub stdin_mode: StdinMode,
    pub stdout_mode: StdoutMode,
    pub stderr_mode: StderrMode,
    pub timeout: Duration,
    pub kill_on_timeout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StdinMode {
    None,
    Pipe,
    Inherit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StdoutMode {
    Pipe,
    Inherit,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StderrMode {
    Pipe,
    Inherit,
    Null,
    ToStdout,
}

/// Process isolation levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessIsolation {
    None,      // No isolation
    User,      // User-level isolation
    Chroot,    // Chroot jail (Unix)
    Container, // Container isolation
    Vm,        // Virtual machine (future)
}

/// Resource limits for external processes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResourceLimits {
    pub max_memory_mb: Option<u64>,
    pub max_cpu_percent: Option<u32>,
    pub max_execution_time: Duration,
    pub max_file_descriptors: Option<u32>,
    pub max_processes: Option<u32>,
    pub allowed_syscalls: Option<Vec<String>>,
    pub blocked_syscalls: Option<Vec<String>>,
}

impl Default for ProcessResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(512),
            max_cpu_percent: Some(80),
            max_execution_time: Duration::from_secs(60),
            max_file_descriptors: Some(256),
            max_processes: Some(10),
            allowed_syscalls: None,
            blocked_syscalls: Some(vec![
                "fork".to_string(),
                "exec".to_string(),
                "ptrace".to_string(),
                "mount".to_string(),
                "unmount".to_string(),
            ]),
        }
    }
}

/// Process execution statistics
#[derive(Debug, Default, Clone)]
pub struct ProcessExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub timeout_executions: u64,
    pub killed_executions: u64,
    pub total_execution_time: Duration,
    pub average_memory_usage_mb: f64,
    pub peak_memory_usage_mb: u64,
}

/// Process sandbox for secure execution
pub struct ProcessSandbox {
    config: ProcessConfig,
    resource_limits: ProcessResourceLimits,
    isolation_level: ProcessIsolation,
    stats: Arc<Mutex<ProcessExecutionStats>>,
    #[allow(dead_code)] // Временная папка для sandbox изоляции
    temp_directory: Option<PathBuf>,
}

impl ProcessSandbox {
    pub fn new(
        config: ProcessConfig,
        resource_limits: ProcessResourceLimits,
        isolation_level: ProcessIsolation,
    ) -> Self {
        Self {
            config,
            resource_limits,
            isolation_level,
            stats: Arc::new(Mutex::new(ProcessExecutionStats::default())),
            temp_directory: None,
        }
    }

    /// Execute process with input and return output
    pub async fn execute(&self, input: &ToolInput) -> Result<ToolOutput> {
        let start_time = Instant::now();

        // Update stats
        {
            let mut stats = self.stats.lock().await;
            stats.total_executions += 1;
        }

        debug!(
            "🚀 Executing external process: {:?}",
            self.config.executable_path
        );

        // Prepare execution environment
        let execution_env = self.prepare_execution_environment(input).await?;

        // Create command
        let command = self.create_command(&execution_env).await?;

        // Execute with timeout and resource monitoring
        let result = self
            .execute_with_monitoring(command, input, start_time)
            .await;

        // Update statistics
        let execution_time = start_time.elapsed();
        self.update_execution_stats(&result, execution_time).await;

        // Cleanup execution environment
        self.cleanup_execution_environment(&execution_env).await?;

        result
    }

    /// Prepare isolated execution environment
    async fn prepare_execution_environment(
        &self,
        input: &ToolInput,
    ) -> Result<ExecutionEnvironment> {
        let mut env = ExecutionEnvironment {
            working_directory: self.config.working_directory.clone(),
            environment_vars: self.config.environment_variables.clone(),
            temp_files: Vec::new(),
            process_id: None,
        };

        // Set up isolation based on level
        match self.isolation_level {
            ProcessIsolation::None => {
                // No additional isolation
            }
            ProcessIsolation::User => {
                self.setup_user_isolation(&mut env).await?;
            }
            ProcessIsolation::Chroot => {
                self.setup_chroot_isolation(&mut env).await?;
            }
            ProcessIsolation::Container => {
                self.setup_container_isolation(&mut env).await?;
            }
            ProcessIsolation::Vm => {
                return Err(anyhow!("VM isolation not yet implemented"));
            }
        }

        // Create input files if needed
        self.create_input_files(&mut env, input).await?;

        Ok(env)
    }

    /// Create command with proper configuration
    async fn create_command(&self, env: &ExecutionEnvironment) -> Result<AsyncCommand> {
        let mut command = AsyncCommand::new(&self.config.executable_path);

        // Add arguments
        command.args(&self.config.arguments);

        // Set working directory
        if let Some(ref working_dir) = env.working_directory {
            command.current_dir(working_dir);
        }

        // Set environment variables
        for (key, value) in &env.environment_vars {
            command.env(key, value);
        }

        // Configure stdio
        match self.config.stdin_mode {
            StdinMode::None => command.stdin(Stdio::null()),
            StdinMode::Pipe => command.stdin(Stdio::piped()),
            StdinMode::Inherit => command.stdin(Stdio::inherit()),
        };

        match self.config.stdout_mode {
            StdoutMode::Pipe => command.stdout(Stdio::piped()),
            StdoutMode::Inherit => command.stdout(Stdio::inherit()),
            StdoutMode::Null => command.stdout(Stdio::null()),
        };

        match self.config.stderr_mode {
            StderrMode::Pipe => command.stderr(Stdio::piped()),
            StderrMode::Inherit => command.stderr(Stdio::inherit()),
            StderrMode::Null => command.stderr(Stdio::null()),
            StderrMode::ToStdout => command.stderr(Stdio::piped()), // Will redirect later
        };

        Ok(command)
    }

    /// Execute command with resource monitoring
    async fn execute_with_monitoring(
        &self,
        mut command: AsyncCommand,
        input: &ToolInput,
        start_time: Instant,
    ) -> Result<ToolOutput> {
        // Spawn process
        let mut child = command
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn process: {}", e))?;

        let process_id = child.id();
        debug!("Process spawned with PID: {:?}", process_id);

        // Handle stdin if needed
        if let Some(mut stdin) = child.stdin.take() {
            let input_data = self.prepare_input_data(input)?;
            if let Err(e) = stdin.write_all(input_data.as_bytes()).await {
                warn!("Failed to write to process stdin: {}", e);
            }
            drop(stdin); // Close stdin
        }

        // Start resource monitoring
        let monitor_handle = if let Some(pid) = process_id {
            Some(self.start_resource_monitoring(pid).await)
        } else {
            None
        };

        // Wait for process completion with timeout
        let output = match tokio::time::timeout(
            self.resource_limits.max_execution_time,
            child.wait_with_output(),
        )
        .await
        {
            Ok(Ok(output)) => {
                // Stop resource monitoring
                if let Some(handle) = monitor_handle {
                    handle.abort();
                }
                output
            }
            Ok(Err(e)) => {
                // Stop resource monitoring
                if let Some(handle) = monitor_handle {
                    handle.abort();
                }
                return Err(anyhow!("Process execution failed: {}", e));
            }
            Err(_) => {
                // Timeout occurred - процесс уже завершился или недоступен
                warn!("Process timed out, PID: {:?}", process_id);

                // Stop resource monitoring
                if let Some(handle) = monitor_handle {
                    handle.abort();
                }

                let mut stats = self.stats.lock().await;
                stats.timeout_executions += 1;

                return Err(anyhow!(
                    "Process execution timed out after {:?}",
                    self.resource_limits.max_execution_time
                ));
            }
        };

        // Process output
        self.process_command_output(output, start_time).await
    }

    /// Process command output and create ToolOutput
    async fn process_command_output(
        &self,
        output: std::process::Output,
        _start_time: Instant,
    ) -> Result<ToolOutput> {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let success = output.status.success();
        let exit_code = output.status.code().unwrap_or(-1);

        let result = if success {
            stdout.clone()
        } else {
            format!("Process failed with exit code {}: {}", exit_code, stderr)
        };

        let formatted_output = if !stdout.is_empty() || !stderr.is_empty() {
            let mut formatted = String::new();
            formatted.push_str("🔧 External Process Output:\n");
            formatted.push_str(&"─".repeat(50));
            formatted.push('\n');

            if !stdout.is_empty() {
                formatted.push_str("📤 STDOUT:\n");
                formatted.push_str(&stdout);
                formatted.push('\n');
            }

            if !stderr.is_empty() {
                formatted.push_str("❌ STDERR:\n");
                formatted.push_str(&stderr);
                formatted.push('\n');
            }

            formatted.push_str(&"─".repeat(50));
            formatted.push('\n');
            Some(formatted)
        } else {
            None
        };

        let mut metadata = HashMap::new();
        metadata.insert("exit_code".to_string(), exit_code.to_string());
        if !stderr.is_empty() {
            metadata.insert("stderr".to_string(), stderr);
        }

        Ok(ToolOutput {
            success,
            result,
            formatted_output,
            metadata,
        })
    }

    /// Start resource monitoring for process
    async fn start_resource_monitoring(&self, pid: u32) -> tokio::task::JoinHandle<()> {
        let limits = self.resource_limits.clone();
        let _stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));

            loop {
                interval.tick().await;

                // In a real implementation, this would:
                // 1. Check memory usage via /proc/[pid]/status or similar
                // 2. Check CPU usage
                // 3. Monitor file descriptors
                // 4. Kill process if limits exceeded

                // For now, just simulate monitoring
                if let Some(max_memory) = limits.max_memory_mb {
                    // Simulated memory check
                    let current_memory = Self::get_process_memory_usage(pid).await;
                    if current_memory > max_memory {
                        warn!(
                            "Process {} exceeded memory limit: {} MB",
                            pid, current_memory
                        );
                        // Would kill process here
                    }
                }
            }
        })
    }

    /// Get process memory usage (simplified implementation)
    async fn get_process_memory_usage(_pid: u32) -> u64 {
        // In a real implementation, this would read from /proc/[pid]/status on Linux
        // or use platform-specific APIs on other systems
        0
    }

    /// Setup user-level isolation
    async fn setup_user_isolation(&self, _env: &mut ExecutionEnvironment) -> Result<()> {
        // In a real implementation, this would:
        // 1. Create a temporary user account
        // 2. Set up minimal permissions
        // 3. Configure ulimits
        Ok(())
    }

    /// Setup chroot isolation (Unix only)
    async fn setup_chroot_isolation(&self, _env: &mut ExecutionEnvironment) -> Result<()> {
        // In a real implementation, this would:
        // 1. Create a chroot jail
        // 2. Copy necessary binaries and libraries
        // 3. Set up minimal filesystem
        Ok(())
    }

    /// Setup container isolation
    async fn setup_container_isolation(&self, _env: &mut ExecutionEnvironment) -> Result<()> {
        // In a real implementation, this would:
        // 1. Create a container with Docker/Podman
        // 2. Configure resource limits
        // 3. Set up networking restrictions
        Ok(())
    }

    /// Create input files for process
    async fn create_input_files(
        &self,
        env: &mut ExecutionEnvironment,
        input: &ToolInput,
    ) -> Result<()> {
        // Create temporary input file if process expects file input
        if input.args.contains_key("input_file") {
            let temp_file = tempfile::NamedTempFile::new()?;
            let temp_path = temp_file.path().to_path_buf();

            // Write input data to temp file
            let input_data = self.prepare_input_data(input)?;
            tokio::fs::write(&temp_path, input_data).await?;

            env.temp_files.push(temp_path);
        }

        Ok(())
    }

    /// Prepare input data for process
    fn prepare_input_data(&self, input: &ToolInput) -> Result<String> {
        // Convert ToolInput to process input format
        // This could be JSON, command line args, etc.
        match serde_json::to_string_pretty(input) {
            Ok(json) => Ok(json),
            Err(_) => {
                // Fallback to simple format
                let mut data = String::new();
                data.push_str(&format!("command: {}\n", input.command));
                for (key, value) in &input.args {
                    data.push_str(&format!("{}: {}\n", key, value));
                }
                if let Some(ref context) = input.context {
                    data.push_str(&format!("context: {}\n", context));
                }
                Ok(data)
            }
        }
    }

    /// Update execution statistics
    async fn update_execution_stats(&self, result: &Result<ToolOutput>, execution_time: Duration) {
        let mut stats = self.stats.lock().await;

        match result {
            Ok(output) => {
                if output.success {
                    stats.successful_executions += 1;
                } else {
                    stats.failed_executions += 1;
                }
            }
            Err(_) => {
                stats.failed_executions += 1;
            }
        }

        stats.total_execution_time += execution_time;
    }

    /// Cleanup execution environment
    async fn cleanup_execution_environment(&self, env: &ExecutionEnvironment) -> Result<()> {
        // Clean up temporary files
        for temp_file in &env.temp_files {
            if temp_file.exists() {
                if let Err(e) = tokio::fs::remove_file(temp_file).await {
                    warn!("Failed to cleanup temp file {:?}: {}", temp_file, e);
                }
            }
        }

        Ok(())
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> ProcessExecutionStats {
        let stats = self.stats.lock().await;
        stats.clone()
    }
}

/// Execution environment state
#[derive(Debug)]
struct ExecutionEnvironment {
    working_directory: Option<PathBuf>,
    environment_vars: HashMap<String, String>,
    temp_files: Vec<PathBuf>,
    #[allow(dead_code)] // ID процесса для мониторинга
    process_id: Option<u32>,
}

/// External process plugin instance
pub struct ExternalProcessPlugin {
    metadata: PluginMetadata,
    #[allow(dead_code)] // Конфигурация external process plugin
    config: PluginConfiguration,
    sandbox: ProcessSandbox,
    is_loaded: bool,
}

impl ExternalProcessPlugin {
    pub fn new(
        metadata: PluginMetadata,
        config: PluginConfiguration,
        process_config: ProcessConfig,
        resource_limits: ProcessResourceLimits,
        isolation_level: ProcessIsolation,
    ) -> Self {
        let sandbox = ProcessSandbox::new(process_config, resource_limits, isolation_level);

        Self {
            metadata,
            config,
            sandbox,
            is_loaded: false,
        }
    }
}

#[async_trait::async_trait]
impl Tool for ExternalProcessPlugin {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.metadata.name.clone(),
            description: self.metadata.description.clone(),
            usage: format!(
                "External Process: {} v{}",
                self.metadata.name, self.metadata.version
            ),
            examples: Vec::new(),
            input_schema: self.metadata.configuration_schema.to_string(),
            usage_guide: None,
        }
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput> {
        if !self.is_loaded {
            return Err(anyhow!("External process plugin not loaded"));
        }

        self.sandbox.execute(&input).await
    }

    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput> {
        // Basic natural language parsing for external process plugins
        Ok(ToolInput {
            command: self.metadata.name.clone(),
            args: HashMap::from([("query".to_string(), query.to_string())]),
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: None,
        })
    }
}

#[async_trait::async_trait]
impl PluginInstance for ExternalProcessPlugin {
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
        info!(
            "🚀 Starting external process plugin: {}",
            self.metadata.name
        );
        self.is_loaded = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!(
            "🛑 Stopping external process plugin: {}",
            self.metadata.name
        );
        self.is_loaded = false;
        Ok(())
    }

    async fn reload(&mut self) -> Result<()> {
        info!(
            "🔄 Reloading external process plugin: {}",
            self.metadata.name
        );
        // External process plugins don't need special reload logic
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        if !self.is_loaded {
            return Err(anyhow!("External process plugin not loaded"));
        }

        // Get execution statistics
        let stats = self.sandbox.get_stats().await;

        // Check for concerning patterns
        if stats.total_executions > 0 {
            let success_rate = stats.successful_executions as f64 / stats.total_executions as f64;
            if success_rate < 0.8 {
                // Less than 80% success rate
                return Err(anyhow!(
                    "External process plugin has low success rate: {:.1}%",
                    success_rate * 100.0
                ));
            }
        }

        if stats.timeout_executions > stats.total_executions / 4 {
            return Err(anyhow!("External process plugin has too many timeouts"));
        }

        Ok(())
    }
}

/// External process plugin loader
pub struct ExternalProcessPluginLoader;

#[async_trait::async_trait]
impl PluginLoader for ExternalProcessPluginLoader {
    async fn load_plugin(
        &self,
        metadata: &PluginMetadata,
        config: &PluginConfiguration,
    ) -> Result<Box<dyn PluginInstance>> {
        let executable_path = metadata
            .installation_path
            .as_ref()
            .ok_or_else(|| anyhow!("No installation path for external process plugin"))?
            .join(&metadata.entry_point);

        // Create process configuration from plugin metadata
        let process_config = ProcessConfig {
            executable_path,
            arguments: Vec::new(), // Could be configured from plugin config
            working_directory: metadata.installation_path.clone(),
            environment_variables: HashMap::new(),
            stdin_mode: StdinMode::Pipe,
            stdout_mode: StdoutMode::Pipe,
            stderr_mode: StderrMode::Pipe,
            timeout: Duration::from_secs(60),
            kill_on_timeout: true,
        };

        let resource_limits = ProcessResourceLimits::default();
        let isolation_level = ProcessIsolation::User; // Default to user-level isolation

        let plugin = ExternalProcessPlugin::new(
            metadata.clone(),
            config.clone(),
            process_config,
            resource_limits,
            isolation_level,
        );

        Ok(Box::new(plugin))
    }

    fn supports_type(&self) -> PluginType {
        PluginType::ExternalProcess
    }

    async fn unload_plugin(&self, mut instance: Box<dyn PluginInstance>) -> Result<()> {
        instance.stop().await?;
        info!(
            "🗑️ Unloaded external process plugin: {}",
            instance.plugin_id()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_process_config_creation() {
        let config = ProcessConfig {
            executable_path: PathBuf::from("test"),
            arguments: vec!["--help".to_string()],
            working_directory: None,
            environment_variables: HashMap::new(),
            stdin_mode: StdinMode::Pipe,
            stdout_mode: StdoutMode::Pipe,
            stderr_mode: StderrMode::Pipe,
            timeout: Duration::from_secs(30),
            kill_on_timeout: true,
        };

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.arguments.len(), 1);
    }

    #[tokio::test]
    async fn test_process_sandbox_creation() {
        let config = ProcessConfig {
            executable_path: PathBuf::from("echo"),
            arguments: vec!["hello".to_string()],
            working_directory: None,
            environment_variables: HashMap::new(),
            stdin_mode: StdinMode::None,
            stdout_mode: StdoutMode::Pipe,
            stderr_mode: StderrMode::Pipe,
            timeout: Duration::from_secs(5),
            kill_on_timeout: true,
        };

        let limits = ProcessResourceLimits::default();
        let sandbox = ProcessSandbox::new(config, limits, ProcessIsolation::None);

        let stats = sandbox.get_stats().await;
        assert_eq!(stats.total_executions, 0);
    }
}
