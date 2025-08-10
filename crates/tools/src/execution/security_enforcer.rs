// @component: {"k":"C","id":"security_enforcer","t":"Security enforcement and sandboxing for tool execution","m":{"cur":0,"tgt":90,"u":"%"},"f":["security","sandbox","enforcement","isolation"]}

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// use std::process::{Command, Stdio}; // Не используется пока
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::registry::{
    FileSystemPermissions, NetworkPermissions, SystemPermissions, ToolPermissions,
};

/// Security enforcement configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub enable_sandboxing: bool,
    pub sandbox_timeout: Duration,
    pub allowed_file_extensions: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub max_file_size_mb: u64,
    pub enable_network_isolation: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_sandboxing: true,
            sandbox_timeout: Duration::from_secs(60),
            allowed_file_extensions: vec![
                ".txt".to_string(),
                ".md".to_string(),
                ".json".to_string(),
                ".toml".to_string(),
                ".yaml".to_string(),
                ".yml".to_string(),
                ".rs".to_string(),
                ".py".to_string(),
                ".js".to_string(),
            ],
            blocked_commands: vec![
                "rm -rf".to_string(),
                "del /f".to_string(),
                "format".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "sudo".to_string(),
                "su".to_string(),
                "chmod 777".to_string(),
                "passwd".to_string(),
            ],
            max_file_size_mb: 100,
            enable_network_isolation: true,
        }
    }
}

/// Execution permissions for tools
#[derive(Debug, Clone)]
pub struct ExecutionPermission {
    pub tool_id: String,
    pub granted_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub granted_by: String,
    pub permissions: ToolPermissions,
    pub restrictions: Vec<SecurityRestriction>,
}

/// Security restrictions that can be applied
#[derive(Debug, Clone)]
pub enum SecurityRestriction {
    FilePathWhitelist(Vec<PathBuf>),
    FileExtensionWhitelist(Vec<String>),
    NetworkHostWhitelist(Vec<String>),
    CommandBlacklist(Vec<String>),
    MaxFileSize(u64),
    MaxExecutionTime(Duration),
    DisallowNetworkAccess,
    ReadOnlyFileSystem,
}

/// Sandbox configuration for isolated execution
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub base_directory: PathBuf,
    pub temp_directory: PathBuf,
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub network_enabled: bool,
    pub file_system_mode: FileSystemMode,
}

#[derive(Debug, Clone)]
pub enum FileSystemMode {
    ReadOnly,
    Restricted { allowed_paths: Vec<PathBuf> },
    Isolated { sandbox_root: PathBuf },
}

/// Process isolation levels
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessIsolation {
    None,      // Run in current process
    Thread,    // Run in separate thread
    Process,   // Run in separate process
    Container, // Run in container (if available)
    VM,        // Run in virtual machine (future)
}

/// Security violation types
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    pub tool_id: String,
    pub violation_type: ViolationType,
    pub description: String,
    pub severity: ViolationSeverity,
    pub timestamp: SystemTime,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ViolationType {
    UnauthorizedFileAccess,
    UnauthorizedNetworkAccess,
    UnauthorizedCommand,
    ResourceLimitExceeded,
    SuspiciousActivity,
    PermissionEscalation,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Main security enforcer
pub struct SecurityEnforcer {
    config: SecurityConfig,
    active_permissions: Mutex<HashMap<String, ExecutionPermission>>,
    violation_history: Mutex<Vec<SecurityViolation>>,
    sandbox_instances: Mutex<HashMap<String, SandboxInstance>>,
}

/// Active sandbox instance tracking
#[derive(Debug)]
struct SandboxInstance {
    #[allow(dead_code)] // Метаданные для sandbox мониторинга
    id: String,
    #[allow(dead_code)]
    tool_id: String,
    #[allow(dead_code)]
    created_at: SystemTime,
    isolation_level: ProcessIsolation,
    #[allow(dead_code)]
    resource_usage: SandboxResourceUsage,
}

#[derive(Debug, Default)]
struct SandboxResourceUsage {
    #[allow(dead_code)] // Метрики использования ресурсов sandbox
    memory_mb: u64,
    #[allow(dead_code)]
    cpu_percent: f32,
    #[allow(dead_code)]
    disk_read_mb: u64,
    #[allow(dead_code)]
    disk_write_mb: u64,
    #[allow(dead_code)]
    network_connections: u32,
}

impl SecurityEnforcer {
    pub fn new(config: SecurityConfig) -> Self {
        info!("🛡️ Initializing Security Enforcer");

        Self {
            config,
            active_permissions: Mutex::new(HashMap::new()),
            violation_history: Mutex::new(Vec::new()),
            sandbox_instances: Mutex::new(HashMap::new()),
        }
    }

    /// Grant execution permission for a tool
    pub async fn grant_permission(
        &self,
        tool_id: String,
        permissions: ToolPermissions,
        granted_by: String,
        duration: Option<Duration>,
    ) -> Result<ExecutionPermission> {
        let granted_at = SystemTime::now();
        let expires_at = duration.map(|d| granted_at + d);

        let restrictions = self.generate_restrictions(&permissions).await;

        let permission = ExecutionPermission {
            tool_id: tool_id.clone(),
            granted_at,
            expires_at,
            granted_by,
            permissions,
            restrictions,
        };

        // Store permission
        {
            let mut perms = self.active_permissions.lock().await;
            perms.insert(tool_id.clone(), permission.clone());
        }

        info!("🔑 Granted execution permission for tool: {}", tool_id);
        Ok(permission)
    }

    /// Validate file access request
    pub async fn validate_file_access(
        &self,
        tool_id: &str,
        file_path: &Path,
        access_type: FileAccessType,
    ) -> Result<()> {
        let permissions = {
            let perms = self.active_permissions.lock().await;
            perms
                .get(tool_id)
                .ok_or_else(|| anyhow!("No permissions found for tool: {}", tool_id))?
                .clone()
        };

        // Check if permission is still valid
        if let Some(expires_at) = permissions.expires_at {
            if SystemTime::now() > expires_at {
                self.log_violation(SecurityViolation {
                    tool_id: tool_id.to_string(),
                    violation_type: ViolationType::UnauthorizedFileAccess,
                    description: "Permission expired".to_string(),
                    severity: ViolationSeverity::Medium,
                    timestamp: SystemTime::now(),
                    remediation: Some("Re-grant permissions".to_string()),
                })
                .await;
                return Err(anyhow!("Permission expired for tool: {}", tool_id));
            }
        }

        // Check file system permissions
        match &permissions.permissions.file_system {
            FileSystemPermissions::None => {
                self.log_violation(SecurityViolation {
                    tool_id: tool_id.to_string(),
                    violation_type: ViolationType::UnauthorizedFileAccess,
                    description: format!("No file system permissions for: {}", file_path.display()),
                    severity: ViolationSeverity::High,
                    timestamp: SystemTime::now(),
                    remediation: Some("Grant appropriate file system permissions".to_string()),
                })
                .await;
                return Err(anyhow!("Tool does not have file system permissions"));
            }
            FileSystemPermissions::ReadOnly => {
                if access_type != FileAccessType::Read {
                    return Err(anyhow!("Tool only has read permissions"));
                }
            }
            FileSystemPermissions::Restricted { allowed_paths } => {
                let path_allowed = allowed_paths
                    .iter()
                    .any(|allowed| file_path.starts_with(allowed));
                if !path_allowed {
                    self.log_violation(SecurityViolation {
                        tool_id: tool_id.to_string(),
                        violation_type: ViolationType::UnauthorizedFileAccess,
                        description: format!("Path not in allowed list: {}", file_path.display()),
                        severity: ViolationSeverity::High,
                        timestamp: SystemTime::now(),
                        remediation: Some("Add path to allowed list".to_string()),
                    })
                    .await;
                    return Err(anyhow!("Path not in allowed list"));
                }
            }
            _ => {} // Full access or ReadWrite allowed
        }

        // Check file size restrictions
        if access_type == FileAccessType::Write {
            if let Ok(metadata) = tokio::fs::metadata(file_path).await {
                let size_mb = metadata.len() / 1024 / 1024;
                if size_mb > self.config.max_file_size_mb {
                    return Err(anyhow!("File size exceeds limit: {} MB", size_mb));
                }
            }
        }

        // Check file extension restrictions
        if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
            let ext_with_dot = format!(".{}", extension);
            if !self.config.allowed_file_extensions.contains(&ext_with_dot) {
                warn!("🚨 Potentially unsafe file extension: {}", extension);
                // Don't block, but log the warning
            }
        }

        debug!(
            "✅ File access validated for tool: {} -> {}",
            tool_id,
            file_path.display()
        );
        Ok(())
    }

    /// Validate network access request
    pub async fn validate_network_access(
        &self,
        tool_id: &str,
        host: &str,
        port: u16,
    ) -> Result<()> {
        let permissions = {
            let perms = self.active_permissions.lock().await;
            perms
                .get(tool_id)
                .ok_or_else(|| anyhow!("No permissions found for tool: {}", tool_id))?
                .clone()
        };

        match &permissions.permissions.network {
            NetworkPermissions::None => {
                self.log_violation(SecurityViolation {
                    tool_id: tool_id.to_string(),
                    violation_type: ViolationType::UnauthorizedNetworkAccess,
                    description: format!("No network permissions for: {}:{}", host, port),
                    severity: ViolationSeverity::High,
                    timestamp: SystemTime::now(),
                    remediation: Some("Grant network permissions".to_string()),
                })
                .await;
                return Err(anyhow!("Tool does not have network permissions"));
            }
            NetworkPermissions::LocalHost => {
                if host != "localhost" && host != "127.0.0.1" && host != "::1" {
                    return Err(anyhow!("Tool only has localhost network permissions"));
                }
            }
            NetworkPermissions::Restricted { allowed_hosts } => {
                if !allowed_hosts.contains(&host.to_string()) {
                    self.log_violation(SecurityViolation {
                        tool_id: tool_id.to_string(),
                        violation_type: ViolationType::UnauthorizedNetworkAccess,
                        description: format!("Host not in allowed list: {}", host),
                        severity: ViolationSeverity::Medium,
                        timestamp: SystemTime::now(),
                        remediation: Some("Add host to allowed list".to_string()),
                    })
                    .await;
                    return Err(anyhow!("Host not in allowed list"));
                }
            }
            _ => {} // Other permissions allow broader access
        }

        debug!(
            "✅ Network access validated for tool: {} -> {}:{}",
            tool_id, host, port
        );
        Ok(())
    }

    /// Validate command execution
    pub async fn validate_command_execution(&self, tool_id: &str, command: &str) -> Result<()> {
        let permissions = {
            let perms = self.active_permissions.lock().await;
            perms
                .get(tool_id)
                .ok_or_else(|| anyhow!("No permissions found for tool: {}", tool_id))?
                .clone()
        };

        // Check system permissions
        if let SystemPermissions::None = permissions.permissions.system {
            return Err(anyhow!("Tool does not have system command permissions"));
        }

        // Check against blocked commands
        let command_lower = command.to_lowercase();
        for blocked in &self.config.blocked_commands {
            if command_lower.contains(&blocked.to_lowercase()) {
                self.log_violation(SecurityViolation {
                    tool_id: tool_id.to_string(),
                    violation_type: ViolationType::UnauthorizedCommand,
                    description: format!("Blocked command detected: {}", blocked),
                    severity: ViolationSeverity::Critical,
                    timestamp: SystemTime::now(),
                    remediation: Some("Remove or modify the command".to_string()),
                })
                .await;
                return Err(anyhow!("Command contains blocked patterns: {}", blocked));
            }
        }

        debug!(
            "✅ Command execution validated for tool: {} -> {}",
            tool_id, command
        );
        Ok(())
    }

    /// Create isolated execution environment
    pub async fn create_sandbox(
        &self,
        tool_id: &str,
        config: SandboxConfig,
        isolation_level: ProcessIsolation,
    ) -> Result<String> {
        let sandbox_id = format!("{}_{}", tool_id, self.current_timestamp());

        // Create sandbox instance
        let instance = SandboxInstance {
            id: sandbox_id.clone(),
            tool_id: tool_id.to_string(),
            created_at: SystemTime::now(),
            isolation_level: isolation_level.clone(),
            resource_usage: SandboxResourceUsage::default(),
        };

        // Store instance
        {
            let mut instances = self.sandbox_instances.lock().await;
            instances.insert(sandbox_id.clone(), instance);
        }

        // Setup sandbox environment based on isolation level
        match isolation_level {
            ProcessIsolation::None => {
                // No isolation, just return ID
            }
            ProcessIsolation::Thread => {
                // Thread-level isolation (limited)
                self.setup_thread_isolation(&sandbox_id, &config).await?;
            }
            ProcessIsolation::Process => {
                // Process-level isolation
                self.setup_process_isolation(&sandbox_id, &config).await?;
            }
            ProcessIsolation::Container => {
                // Container isolation (if available)
                self.setup_container_isolation(&sandbox_id, &config).await?;
            }
            ProcessIsolation::VM => {
                return Err(anyhow!("VM isolation not yet implemented"));
            }
        }

        info!(
            "📦 Created sandbox: {} with {:?} isolation",
            sandbox_id, isolation_level
        );
        Ok(sandbox_id)
    }

    /// Cleanup sandbox environment
    pub async fn cleanup_sandbox(&self, sandbox_id: &str) -> Result<()> {
        let instance = {
            let mut instances = self.sandbox_instances.lock().await;
            instances
                .remove(sandbox_id)
                .ok_or_else(|| anyhow!("Sandbox not found: {}", sandbox_id))?
        };

        // Cleanup based on isolation level
        match instance.isolation_level {
            ProcessIsolation::Process => {
                self.cleanup_process_isolation(sandbox_id).await?;
            }
            ProcessIsolation::Container => {
                self.cleanup_container_isolation(sandbox_id).await?;
            }
            _ => {} // Other levels don't need special cleanup
        }

        info!("🧹 Cleaned up sandbox: {}", sandbox_id);
        Ok(())
    }

    /// Get security violation history
    pub async fn get_violation_history(&self, tool_id: Option<&str>) -> Vec<SecurityViolation> {
        let history = self.violation_history.lock().await;
        match tool_id {
            Some(id) => history
                .iter()
                .filter(|v| v.tool_id == id)
                .cloned()
                .collect(),
            None => history.clone(),
        }
    }

    /// Generate security restrictions based on permissions
    async fn generate_restrictions(
        &self,
        permissions: &ToolPermissions,
    ) -> Vec<SecurityRestriction> {
        let mut restrictions = Vec::new();

        // Add file system restrictions
        match &permissions.file_system {
            FileSystemPermissions::None => {
                restrictions.push(SecurityRestriction::ReadOnlyFileSystem);
            }
            FileSystemPermissions::ReadOnly => {
                restrictions.push(SecurityRestriction::ReadOnlyFileSystem);
            }
            FileSystemPermissions::Restricted { allowed_paths } => {
                let paths: Vec<PathBuf> = allowed_paths.iter().map(PathBuf::from).collect();
                restrictions.push(SecurityRestriction::FilePathWhitelist(paths));
            }
            _ => {}
        }

        // Add network restrictions
        match &permissions.network {
            NetworkPermissions::None => {
                restrictions.push(SecurityRestriction::DisallowNetworkAccess);
            }
            NetworkPermissions::Restricted { allowed_hosts } => {
                restrictions.push(SecurityRestriction::NetworkHostWhitelist(
                    allowed_hosts.clone(),
                ));
            }
            _ => {}
        }

        // Add common restrictions
        restrictions.push(SecurityRestriction::FileExtensionWhitelist(
            self.config.allowed_file_extensions.clone(),
        ));
        restrictions.push(SecurityRestriction::CommandBlacklist(
            self.config.blocked_commands.clone(),
        ));
        restrictions.push(SecurityRestriction::MaxFileSize(
            self.config.max_file_size_mb,
        ));
        restrictions.push(SecurityRestriction::MaxExecutionTime(
            self.config.sandbox_timeout,
        ));

        restrictions
    }

    async fn setup_thread_isolation(
        &self,
        _sandbox_id: &str,
        _config: &SandboxConfig,
    ) -> Result<()> {
        // Thread isolation setup (basic resource limiting)
        Ok(())
    }

    async fn setup_process_isolation(
        &self,
        _sandbox_id: &str,
        _config: &SandboxConfig,
    ) -> Result<()> {
        // Process isolation setup
        // In production, this would create a separate process with limited privileges
        Ok(())
    }

    async fn setup_container_isolation(
        &self,
        _sandbox_id: &str,
        _config: &SandboxConfig,
    ) -> Result<()> {
        // Container isolation setup
        // Would integrate with Docker or similar container runtime
        Ok(())
    }

    async fn cleanup_process_isolation(&self, _sandbox_id: &str) -> Result<()> {
        // Cleanup process isolation
        Ok(())
    }

    async fn cleanup_container_isolation(&self, _sandbox_id: &str) -> Result<()> {
        // Cleanup container isolation
        Ok(())
    }

    async fn log_violation(&self, violation: SecurityViolation) {
        warn!(
            "🚨 Security violation: {} - {}",
            violation.violation_type_str(),
            violation.description
        );

        let mut history = self.violation_history.lock().await;
        history.push(violation);

        // Keep only recent violations (last 1000)
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[derive(Debug, PartialEq)]
pub enum FileAccessType {
    Read,
    Write,
    Delete,
}

impl SecurityViolation {
    fn violation_type_str(&self) -> &str {
        match self.violation_type {
            ViolationType::UnauthorizedFileAccess => "Unauthorized File Access",
            ViolationType::UnauthorizedNetworkAccess => "Unauthorized Network Access",
            ViolationType::UnauthorizedCommand => "Unauthorized Command",
            ViolationType::ResourceLimitExceeded => "Resource Limit Exceeded",
            ViolationType::SuspiciousActivity => "Suspicious Activity",
            ViolationType::PermissionEscalation => "Permission Escalation",
        }
    }
}

impl Default for SecurityEnforcer {
    fn default() -> Self {
        Self::new(SecurityConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_enforcer_creation() {
        let enforcer = SecurityEnforcer::default();
        let history = enforcer.get_violation_history(None).await;
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_file_access_validation() {
        let enforcer = SecurityEnforcer::default();

        // Grant permissions
        let permissions = ToolPermissions {
            file_system: FileSystemPermissions::ReadOnly,
            network: NetworkPermissions::None,
            system: SystemPermissions::None,
            custom: HashMap::new(),
        };

        let _permission = enforcer
            .grant_permission(
                "test_tool".to_string(),
                permissions,
                "test_user".to_string(),
                None,
            )
            .await
            .unwrap();

        // Test read access (should pass)
        let result = enforcer
            .validate_file_access(
                "test_tool",
                Path::new("/tmp/test.txt"),
                FileAccessType::Read,
            )
            .await;
        assert!(result.is_ok());

        // Test write access (should fail for ReadOnly)
        let result = enforcer
            .validate_file_access(
                "test_tool",
                Path::new("/tmp/test.txt"),
                FileAccessType::Write,
            )
            .await;
        assert!(result.is_err());
    }
}
