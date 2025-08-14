// P1.2.5: JSON-RPC Subprocess Framework
// Tool execution in separate processes with full lifecycle management

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child as TokioChild, Command as TokioCommand};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: i32, method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(id))),
            method,
            params,
        }
    }

    pub fn notification(method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method,
            params,
        }
    }
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Subprocess execution errors
#[derive(Debug, Error)]
pub enum SubprocessError {
    #[error("Process spawn failed: {0}")]
    SpawnError(String),
    
    #[error("Process communication failed: {0}")]
    CommunicationError(String),
    
    #[error("Process timeout after {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Process crashed with exit code: {}", match code { Some(c) => c.to_string(), None => "unknown".to_string() })]
    ProcessCrashed { code: Option<i32> },
    
    #[error("JSON-RPC error: {code} - {message}")]
    JsonRpcError { code: i32, message: String },
    
    #[error("Subprocess not found: {id}")]
    ProcessNotFound { id: String },
    
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },
    
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: String },
    
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
}

/// Subprocess configuration
#[derive(Debug, Clone)]
pub struct SubprocessConfig {
    /// Working directory for the subprocess
    pub working_directory: Option<std::path::PathBuf>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Execution timeout
    pub timeout: Duration,
    /// Maximum memory usage (MB)
    pub max_memory_mb: Option<u64>,
    /// CPU limit percentage (0-100)
    pub cpu_limit: Option<u8>,
    /// Enable debug logging
    pub debug_mode: bool,
    /// Kill subprocess on drop
    pub kill_on_drop: bool,
}

impl Default for SubprocessConfig {
    fn default() -> Self {
        Self {
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(30),
            max_memory_mb: Some(128), // 128MB default limit
            cpu_limit: Some(80),      // 80% CPU limit
            debug_mode: false,
            kill_on_drop: true,
        }
    }
}

impl SubprocessConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn with_working_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }
    
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }
    
    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.max_memory_mb = Some(limit_mb);
        self
    }
    
    pub fn with_debug(mut self, enable: bool) -> Self {
        self.debug_mode = enable;
        self
    }
}

/// Subprocess lifecycle states
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Crashed,
    Timeout,
}

/// Process lifecycle manager
pub struct ProcessLifecycleManager {
    process_id: String,
    child: Option<TokioChild>,
    state: ProcessState,
    start_time: Instant,
    config: SubprocessConfig,
    stdin_tx: Option<mpsc::UnboundedSender<String>>,
    stdout_rx: Option<mpsc::UnboundedReceiver<String>>,
    stderr_rx: Option<mpsc::UnboundedReceiver<String>>,
}

impl ProcessLifecycleManager {
    /// Create new process lifecycle manager
    pub fn new(process_id: String, config: SubprocessConfig) -> Self {
        Self {
            process_id,
            child: None,
            state: ProcessState::Starting,
            start_time: Instant::now(),
            config,
            stdin_tx: None,
            stdout_rx: None,
            stderr_rx: None,
        }
    }
    
    /// Start subprocess with command and arguments
    pub async fn start(&mut self, cmd: &str, args: Vec<String>) -> Result<(), SubprocessError> {
        if self.child.is_some() {
            return Err(SubprocessError::InvalidConfig {
                reason: "Process already started".to_string(),
            });
        }
        
        debug!("Starting subprocess: {} {:?}", cmd, args);
        
        let mut command = TokioCommand::new(cmd);
        command.args(args)
               .stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());
        
        // Set working directory
        if let Some(ref dir) = self.config.working_directory {
            command.current_dir(dir);
        }
        
        // Set environment variables
        for (key, value) in &self.config.environment {
            command.env(key, value);
        }
        
        // Spawn process
        let mut child = command.spawn()
            .map_err(|e| SubprocessError::SpawnError(e.to_string()))?;
        
        // Setup communication channels
        let stdin = child.stdin.take()
            .ok_or_else(|| SubprocessError::CommunicationError("Failed to get stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| SubprocessError::CommunicationError("Failed to get stdout".to_string()))?;
        let stderr = child.stderr.take()
            .ok_or_else(|| SubprocessError::CommunicationError("Failed to get stderr".to_string()))?;
        
        // Create channels for communication
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();
        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel::<String>();
        let (stderr_tx, stderr_rx) = mpsc::unbounded_channel::<String>();
        
        // Spawn stdin writer task
        let mut stdin_writer = tokio::io::BufWriter::new(stdin);
        tokio::spawn(async move {
            while let Some(data) = stdin_rx.recv().await {
                if let Err(e) = stdin_writer.write_all(data.as_bytes()).await {
                    error!("Failed to write to subprocess stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin_writer.flush().await {
                    error!("Failed to flush subprocess stdin: {}", e);
                    break;
                }
            }
        });
        
        // Spawn stdout reader task
        let mut stdout_reader = BufReader::new(stdout);
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match stdout_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if stdout_tx.send(line.clone()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from subprocess stdout: {}", e);
                        break;
                    }
                }
            }
        });
        
        // Spawn stderr reader task
        let mut stderr_reader = BufReader::new(stderr);
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match stderr_reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if stderr_tx.send(line.clone()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from subprocess stderr: {}", e);
                        break;
                    }
                }
            }
        });
        
        self.child = Some(child);
        self.stdin_tx = Some(stdin_tx);
        self.stdout_rx = Some(stdout_rx);
        self.stderr_rx = Some(stderr_rx);
        self.state = ProcessState::Running;
        self.start_time = Instant::now();
        
        info!("Subprocess started: {}", self.process_id);
        Ok(())
    }
    
    /// Send data to subprocess stdin
    pub fn send_stdin(&self, data: String) -> Result<(), SubprocessError> {
        if let Some(ref tx) = self.stdin_tx {
            tx.send(data)
                .map_err(|_| SubprocessError::CommunicationError("Failed to send to stdin".to_string()))?;
            Ok(())
        } else {
            Err(SubprocessError::CommunicationError("Stdin not available".to_string()))
        }
    }
    
    /// Receive data from subprocess stdout (non-blocking)
    pub fn recv_stdout(&mut self) -> Option<String> {
        if let Some(ref mut rx) = self.stdout_rx {
            rx.try_recv().ok()
        } else {
            None
        }
    }
    
    /// Receive data from subprocess stderr (non-blocking)
    pub fn recv_stderr(&mut self) -> Option<String> {
        if let Some(ref mut rx) = self.stderr_rx {
            rx.try_recv().ok()
        } else {
            None
        }
    }
    
    /// Check if process is still running
    pub async fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.state = ProcessState::Stopped;
                    false
                }
                Ok(None) => true,
                Err(e) => {
                    warn!("Failed to check process status: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }
    
    /// Stop subprocess gracefully
    pub async fn stop(&mut self) -> Result<(), SubprocessError> {
        if let Some(mut child) = self.child.take() {
            self.state = ProcessState::Stopping;
            
            // Try graceful shutdown first
            if let Some(id) = child.id() {
                debug!("Sending SIGTERM to process {}", id);
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(id as i32, libc::SIGTERM);
                    }
                }
                #[cfg(windows)]
                {
                    // On Windows, we can only kill forcefully
                    let _ = child.kill().await;
                }
                
                // Wait for graceful shutdown
                match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                    Ok(Ok(status)) => {
                        info!("Process {} exited with status: {}", self.process_id, status);
                        self.state = ProcessState::Stopped;
                        return Ok(());
                    }
                    Ok(Err(e)) => {
                        warn!("Error waiting for process {}: {}", self.process_id, e);
                    }
                    Err(_) => {
                        warn!("Process {} did not exit gracefully, force killing", self.process_id);
                    }
                }
            }
            
            // Force kill if graceful shutdown failed
            match child.kill().await {
                Ok(()) => {
                    info!("Process {} force killed", self.process_id);
                    self.state = ProcessState::Stopped;
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to kill process {}: {}", self.process_id, e);
                    self.state = ProcessState::Crashed;
                    Err(SubprocessError::CommunicationError(format!("Failed to kill process: {}", e)))
                }
            }
        } else {
            Ok(())
        }
    }
    
    /// Get process state
    pub fn state(&self) -> &ProcessState {
        &self.state
    }
    
    /// Get process uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
    
    /// Get process ID
    pub fn process_id(&self) -> &str {
        &self.process_id
    }
}

impl Drop for ProcessLifecycleManager {
    fn drop(&mut self) {
        if self.config.kill_on_drop && self.child.is_some() {
            if let Some(mut child) = self.child.take() {
                let process_id = self.process_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = child.kill().await {
                        error!("Failed to kill subprocess {} on drop: {}", process_id, e);
                    }
                });
            }
        }
    }
}

/// Main subprocess runner
pub struct SubprocessRunner {
    processes: Arc<Mutex<HashMap<String, ProcessLifecycleManager>>>,
    next_id: Arc<Mutex<u32>>,
}

impl Default for SubprocessRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SubprocessRunner {
    /// Create new subprocess runner
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }
    
    /// Spawn new subprocess
    pub async fn spawn(
        &self,
        cmd: &str,
        args: Vec<String>,
        config: SubprocessConfig,
    ) -> Result<String, SubprocessError> {
        let process_id = {
            let mut next_id = self.next_id.lock()
                .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
            let id = format!("process_{}", *next_id);
            *next_id += 1;
            id
        };
        
        let mut manager = ProcessLifecycleManager::new(process_id.clone(), config);
        manager.start(cmd, args).await?;
        
        {
            let mut processes = self.processes.lock()
                .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
            processes.insert(process_id.clone(), manager);
        }
        
        Ok(process_id)
    }
    
    /// Send JSON-RPC request to subprocess
    pub async fn send_request(
        &self,
        process_id: &str,
        request: JsonRpcRequest,
    ) -> Result<(), SubprocessError> {
        let request_json = serde_json::to_string(&request)
            .map_err(|e| SubprocessError::CommunicationError(e.to_string()))?;
        
        let mut processes = self.processes.lock()
            .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
        
        if let Some(manager) = processes.get_mut(process_id) {
            manager.send_stdin(request_json + "\n")?;
            Ok(())
        } else {
            Err(SubprocessError::ProcessNotFound {
                id: process_id.to_string(),
            })
        }
    }
    
    /// Receive JSON-RPC response from subprocess
    pub async fn recv_response(&self, process_id: &str) -> Result<Option<JsonRpcResponse>, SubprocessError> {
        let mut processes = self.processes.lock()
            .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
        
        if let Some(manager) = processes.get_mut(process_id) {
            if let Some(line) = manager.recv_stdout() {
                let response: JsonRpcResponse = serde_json::from_str(&line)
                    .map_err(|e| SubprocessError::CommunicationError(e.to_string()))?;
                Ok(Some(response))
            } else {
                Ok(None)
            }
        } else {
            Err(SubprocessError::ProcessNotFound {
                id: process_id.to_string(),
            })
        }
    }
    
    /// Stop subprocess
    pub async fn stop_process(&self, process_id: &str) -> Result<(), SubprocessError> {
        let mut manager = {
            let mut processes = self.processes.lock()
                .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
            processes.remove(process_id)
                .ok_or_else(|| SubprocessError::ProcessNotFound {
                    id: process_id.to_string(),
                })?
        };
        
        manager.stop().await
    }
    
    /// Check if process is running
    pub async fn is_process_running(&self, process_id: &str) -> Result<bool, SubprocessError> {
        let mut processes = self.processes.lock()
            .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
        
        if let Some(manager) = processes.get_mut(process_id) {
            Ok(manager.is_running().await)
        } else {
            Err(SubprocessError::ProcessNotFound {
                id: process_id.to_string(),
            })
        }
    }
    
    /// Get process state
    pub fn get_process_state(&self, process_id: &str) -> Result<ProcessState, SubprocessError> {
        let processes = self.processes.lock()
            .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
        
        if let Some(manager) = processes.get(process_id) {
            Ok(manager.state().clone())
        } else {
            Err(SubprocessError::ProcessNotFound {
                id: process_id.to_string(),
            })
        }
    }
    
    /// List all process IDs
    pub fn list_processes(&self) -> Result<Vec<String>, SubprocessError> {
        let processes = self.processes.lock()
            .map_err(|_| SubprocessError::CommunicationError("Lock error".to_string()))?;
        
        Ok(processes.keys().cloned().collect())
    }
    
    /// Stop all processes
    pub async fn stop_all_processes(&self) -> Result<(), SubprocessError> {
        let process_ids = self.list_processes()?;
        
        for process_id in process_ids {
            if let Err(e) = self.stop_process(&process_id).await {
                warn!("Failed to stop process {}: {}", process_id, e);
            }
        }
        
        Ok(())
    }
}

/// High-level convenience function for executing tool via JSON-RPC
pub async fn execute_tool_subprocess(
    cmd: &str,
    args: Vec<String>,
    tool_method: &str,
    tool_params: Option<serde_json::Value>,
    config: SubprocessConfig,
) -> Result<serde_json::Value, SubprocessError> {
    let runner = SubprocessRunner::new();
    
    // Start subprocess
    let process_id = runner.spawn(cmd, args, config).await?;
    
    // Send JSON-RPC request
    let request = JsonRpcRequest::new(1, tool_method.to_string(), tool_params);
    runner.send_request(&process_id, request).await?;
    
    // Wait for response with timeout
    let response = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(response) = runner.recv_response(&process_id).await? {
                return Ok(response);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| SubprocessError::Timeout {
        timeout: Duration::from_secs(30),
    })??;
    
    // Clean up
    let _ = runner.stop_process(&process_id).await;
    
    // Handle response
    if let Some(error) = response.error {
        Err(SubprocessError::JsonRpcError {
            code: error.code,
            message: error.message,
        })
    } else if let Some(result) = response.result {
        Ok(result)
    } else {
        Err(SubprocessError::CommunicationError(
            "Invalid JSON-RPC response".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_subprocess_config() {
        let config = SubprocessConfig::new()
            .with_timeout(Duration::from_secs(10))
            .with_memory_limit(256)
            .with_debug(true);
        
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_memory_mb, Some(256));
        assert!(config.debug_mode);
    }

    #[tokio::test]
    async fn test_json_rpc_request_creation() {
        let request = JsonRpcRequest::new(1, "test_method".to_string(), None);
        
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test_method");
        assert!(request.id.is_some());
    }

    #[tokio::test]
    async fn test_json_rpc_notification() {
        let notification = JsonRpcRequest::notification("notify".to_string(), None);
        
        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, "notify");
        assert!(notification.id.is_none());
    }

    #[tokio::test]
    async fn test_subprocess_runner_creation() {
        let runner = SubprocessRunner::new();
        let processes = runner.list_processes().expect("Failed to list processes");
        assert!(processes.is_empty());
    }

    // Note: More comprehensive tests would require actual subprocesses
    // which might not be available in all test environments
}