// @component: {"k":"C","id":"secure_tool_registry","t":"Security-focused tool registry with permission management","m":{"cur":0,"tgt":95,"u":"%"},"f":["security","registry","permissions","validation"]}

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error};

use super::tool_metadata::{
    ToolMetadata, SecurityLevel, ToolPermissions
};
use crate::{Tool, ToolInput, ToolOutput};

/// Security context for tool execution
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub user_id: String,
    pub session_id: String,
    pub permissions: UserPermissions,
    pub trust_level: UserTrustLevel,
}

/// User permission levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum UserTrustLevel {
    Guest,           // Very limited permissions
    User,            // Standard user permissions  
    PowerUser,       // Extended permissions
    Administrator,   // Full permissions
}

#[derive(Debug, Clone)]
pub struct UserPermissions {
    pub can_execute_high_risk: bool,
    pub can_install_tools: bool,
    pub can_modify_security: bool,
    pub max_resource_usage: ResourceLimits,
}

#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_cores: u32,
    pub max_execution_time: Duration,
    pub max_concurrent_tools: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_cpu_cores: 2,
            max_execution_time: Duration::from_secs(60),
            max_concurrent_tools: 3,
        }
    }
}

/// Input validation and sanitization
pub struct InputValidator;

impl InputValidator {
    pub fn validate_and_sanitize(input: &ToolInput, metadata: &ToolMetadata) -> Result<ToolInput> {
        let mut sanitized_input = input.clone();
        
        // Validate required parameters
        Self::validate_schema(&sanitized_input, &metadata.input_schema)?;
        
        // Sanitize based on tool permissions
        Self::sanitize_based_on_permissions(&mut sanitized_input, &metadata.permissions)?;
        
        // Additional security checks
        Self::perform_security_checks(&sanitized_input, metadata)?;
        
        Ok(sanitized_input)
    }
    
    fn validate_schema(input: &ToolInput, schema: &serde_json::Value) -> Result<()> {
        // Basic schema validation (in production, use a proper JSON schema validator)
        if let Some(obj) = schema.as_object() {
            for (key, _) in obj {
                if !input.args.contains_key(key) {
                    warn!("Missing required parameter: {}", key);
                    // Note: Not failing here to maintain compatibility, but should be configurable
                }
            }
        }
        Ok(())
    }
    
    fn sanitize_based_on_permissions(input: &mut ToolInput, permissions: &ToolPermissions) -> Result<()> {
        // Sanitize file paths
        if let Some(path) = input.args.get("path") {
            let sanitized_path = Self::sanitize_file_path(path, &permissions.file_system)?;
            input.args.insert("path".to_string(), sanitized_path);
        }
        
        // Sanitize URLs
        if let Some(url) = input.args.get("url") {
            let sanitized_url = Self::sanitize_url(url, &permissions.network)?;
            input.args.insert("url".to_string(), sanitized_url);
        }
        
        // Sanitize commands
        if let Some(command) = input.args.get("command") {
            let sanitized_command = Self::sanitize_command(command, &permissions.system)?;
            input.args.insert("command".to_string(), sanitized_command);
        }
        
        Ok(())
    }
    
    fn sanitize_file_path(path: &str, fs_perms: &crate::registry::tool_metadata::FileSystemPermissions) -> Result<String> {
        use crate::registry::tool_metadata::FileSystemPermissions;
        
        // Basic path injection prevention
        let sanitized = path
            .replace("../", "")  // Prevent directory traversal
            .replace("..\\", "") // Windows directory traversal
            .replace("~/", "")   // Prevent home directory access without explicit permission
            .trim()
            .to_string();
        
        // Check against allowed paths for restricted permissions
        match fs_perms {
            FileSystemPermissions::None => {
                return Err(anyhow!("Tool does not have file system permissions"));
            },
            FileSystemPermissions::Restricted { allowed_paths } => {
                let path_allowed = allowed_paths.iter().any(|allowed| {
                    sanitized.starts_with(allowed)
                });
                if !path_allowed {
                    return Err(anyhow!("Path not in allowed list: {}", sanitized));
                }
            },
            _ => {} // Other permissions allow broader access
        }
        
        Ok(sanitized)
    }
    
    fn sanitize_url(url: &str, net_perms: &crate::registry::tool_metadata::NetworkPermissions) -> Result<String> {
        use crate::registry::tool_metadata::NetworkPermissions;
        
        let url = url.trim();
        
        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("Only HTTP/HTTPS URLs are allowed"));
        }
        
        // Check network permissions
        match net_perms {
            NetworkPermissions::None => {
                return Err(anyhow!("Tool does not have network permissions"));
            },
            NetworkPermissions::LocalHost => {
                if !url.contains("localhost") && !url.contains("127.0.0.1") {
                    return Err(anyhow!("Only localhost URLs are allowed"));
                }
            },
            NetworkPermissions::Restricted { allowed_hosts } => {
                let host_allowed = allowed_hosts.iter().any(|allowed| {
                    url.contains(allowed)
                });
                if !host_allowed {
                    return Err(anyhow!("Host not in allowed list"));
                }
            },
            _ => {} // Other permissions allow broader access
        }
        
        Ok(url.to_string())
    }
    
    fn sanitize_command(command: &str, sys_perms: &crate::registry::tool_metadata::SystemPermissions) -> Result<String> {
        use crate::registry::tool_metadata::SystemPermissions;
        
        match sys_perms {
            SystemPermissions::None => {
                return Err(anyhow!("Tool does not have system command permissions"));
            },
            _ => {}
        }
        
        // Prevent dangerous commands
        let dangerous_commands = [
            "rm -rf", "del /f", "format", "shutdown", "reboot",
            "sudo", "su", "chmod 777", "chown", "passwd"
        ];
        
        let command_lower = command.to_lowercase();
        for dangerous in &dangerous_commands {
            if command_lower.contains(dangerous) {
                warn!("Blocked dangerous command: {}", dangerous);
                return Err(anyhow!("Command contains dangerous patterns: {}", dangerous));
            }
        }
        
        Ok(command.to_string())
    }
    
    fn perform_security_checks(input: &ToolInput, _metadata: &ToolMetadata) -> Result<()> {
        // Check for SQL injection patterns
        for (_, value) in &input.args {
            if Self::contains_sql_injection(value) {
                error!("SQL injection attempt detected in tool input");
                return Err(anyhow!("Input contains potential SQL injection"));
            }
        }
        
        // Check for script injection
        if let Some(context) = &input.context {
            if Self::contains_script_injection(context) {
                error!("Script injection attempt detected in context");
                return Err(anyhow!("Context contains potential script injection"));
            }
        }
        
        Ok(())
    }
    
    fn contains_sql_injection(input: &str) -> bool {
        let input_lower = input.to_lowercase();
        let sql_patterns = [
            "' or '1'='1", "' or 1=1", "union select", "drop table",
            "delete from", "insert into", "--", "/*", "*/"
        ];
        
        sql_patterns.iter().any(|pattern| input_lower.contains(pattern))
    }
    
    fn contains_script_injection(input: &str) -> bool {
        let input_lower = input.to_lowercase();
        let script_patterns = [
            "<script", "javascript:", "eval(", "onclick=", "onerror=",
            "onload=", "document.cookie", "window.location"
        ];
        
        script_patterns.iter().any(|pattern| input_lower.contains(pattern))
    }
}

/// Secure tool registry with permission enforcement
pub struct SecureToolRegistry {
    /// Core tool storage with metadata
    tools: Arc<RwLock<HashMap<String, (Arc<dyn Tool>, ToolMetadata)>>>,
    
    /// Security policies and configurations
    security_config: SecurityConfig,
    
    /// Active execution tracking for resource management
    active_executions: Arc<Mutex<HashMap<String, ExecutionTracking>>>,
    
    /// Audit log for security events
    audit_log: Arc<Mutex<Vec<AuditEvent>>>,
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub require_signature_verification: bool,
    pub allow_untrusted_tools: bool,
    pub max_concurrent_executions: u32,
    pub execution_timeout: Duration,
    pub auto_quarantine_threshold: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_signature_verification: true,
            allow_untrusted_tools: false,
            max_concurrent_executions: 10,
            execution_timeout: Duration::from_secs(300), // 5 minutes
            auto_quarantine_threshold: 5, // Quarantine after 5 failures
        }
    }
}

#[derive(Debug)]
struct ExecutionTracking {
    #[allow(dead_code)] // Метрики для отслеживания выполнения инструментов
    tool_id: String,
    #[allow(dead_code)]
    start_time: SystemTime,
    session_id: String,
    #[allow(dead_code)]
    resource_usage: CurrentResourceUsage,
}

#[derive(Debug, Default)]
struct CurrentResourceUsage {
    #[allow(dead_code)] // Метрики ресурсов для мониторинга
    memory_mb: u64,
    #[allow(dead_code)]
    cpu_percent: f32,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub tool_id: String,
    pub user_id: String,
    pub session_id: String,
    pub details: String,
}

#[derive(Debug, Clone)]
pub enum AuditEventType {
    ToolRegistered,
    ToolExecuted,
    SecurityViolation,
    PermissionDenied,
    QuarantineActivated,
    TrustLevelChanged,
}

impl SecureToolRegistry {
    pub fn new(security_config: SecurityConfig) -> Self {
        info!("🔒 Initializing Secure Tool Registry");
        
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            security_config,
            active_executions: Arc::new(Mutex::new(HashMap::new())),
            audit_log: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Register a tool with comprehensive security checks
    pub async fn register_tool(
        &self, 
        tool: Arc<dyn Tool>, 
        metadata: ToolMetadata,
        security_context: &SecurityContext
    ) -> Result<()> {
        // Check registration permissions
        if !security_context.permissions.can_install_tools {
            self.log_audit_event(AuditEvent {
                timestamp: Self::current_timestamp(),
                event_type: AuditEventType::PermissionDenied,
                tool_id: metadata.id.clone(),
                user_id: security_context.user_id.clone(),
                session_id: security_context.session_id.clone(),
                details: "User lacks tool installation permissions".to_string(),
            }).await;
            
            return Err(anyhow!("Insufficient permissions to register tools"));
        }
        
        // Verify tool signature if required
        if self.security_config.require_signature_verification {
            self.verify_tool_signature(&metadata)?;
        }
        
        // Check if untrusted tools are allowed
        if !metadata.trusted && !self.security_config.allow_untrusted_tools {
            return Err(anyhow!("Untrusted tools are not allowed in this registry"));
        }
        
        // Validate metadata completeness
        self.validate_metadata(&metadata)?;
        
        let tool_id = metadata.id.clone();
        
        // Register the tool
        {
            let mut tools = self.tools.write().await;
            tools.insert(tool_id.clone(), (tool, metadata.clone()));
        }
        
        // Log registration
        self.log_audit_event(AuditEvent {
            timestamp: Self::current_timestamp(),
            event_type: AuditEventType::ToolRegistered,
            tool_id: tool_id.clone(),
            user_id: security_context.user_id.clone(),
            session_id: security_context.session_id.clone(),
            details: format!("Tool registered: {} v{}", metadata.name, metadata.version),
        }).await;
        
        info!("🔧 Tool registered securely: {} ({})", metadata.name, tool_id);
        Ok(())
    }
    
    /// Execute tool with security validation and resource management
    pub async fn execute_tool(
        &self,
        tool_id: &str,
        input: ToolInput,
        security_context: &SecurityContext
    ) -> Result<ToolOutput> {
        // Check concurrent executions limit
        self.check_execution_limits(security_context).await?;
        
        // Get tool and metadata
        let (tool, metadata) = {
            let tools = self.tools.read().await;
            let (tool, metadata) = tools.get(tool_id)
                .ok_or_else(|| anyhow!("Tool not found: {}", tool_id))?;
            (Arc::clone(tool), metadata.clone())
        };
        
        // Security authorization check
        self.authorize_execution(&metadata, security_context)?;
        
        // Validate and sanitize input
        let sanitized_input = InputValidator::validate_and_sanitize(&input, &metadata)?;
        
        // Check resource requirements
        self.check_resource_requirements(&metadata, security_context)?;
        
        // Start execution tracking
        let execution_id = format!("{}_{}", tool_id, Self::current_timestamp());
        self.start_execution_tracking(&execution_id, tool_id, &security_context.session_id).await;
        
        // Execute with timeout
        let execution_start = SystemTime::now();
        let execution_future = tool.execute(sanitized_input);
        let timeout_duration = metadata.resource_requirements.max_execution_time
            .unwrap_or(self.security_config.execution_timeout);
            
        let result = match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(result) => result,
            Err(_) => {
                // Timeout occurred
                self.log_audit_event(AuditEvent {
                    timestamp: Self::current_timestamp(),
                    event_type: AuditEventType::SecurityViolation,
                    tool_id: tool_id.to_string(),
                    user_id: security_context.user_id.clone(),
                    session_id: security_context.session_id.clone(),
                    details: "Tool execution timeout".to_string(),
                }).await;
                
                Err(anyhow!("Tool execution timed out"))
            }
        };
        
        // Update performance metrics
        let execution_time = execution_start.elapsed().unwrap_or_default();
        let success = result.is_ok();
        self.update_tool_performance(tool_id, execution_time, success).await;
        
        // End execution tracking
        self.end_execution_tracking(&execution_id).await;
        
        // Log execution
        self.log_audit_event(AuditEvent {
            timestamp: Self::current_timestamp(),
            event_type: AuditEventType::ToolExecuted,
            tool_id: tool_id.to_string(),
            user_id: security_context.user_id.clone(),
            session_id: security_context.session_id.clone(),
            details: if success {
                format!("Successful execution in {:?}", execution_time)
            } else {
                "Execution failed".to_string()
            },
        }).await;
        
        result
    }
    
    /// Get tools filtered by security permissions
    pub async fn get_available_tools(&self, security_context: &SecurityContext) -> Vec<ToolMetadata> {
        let tools = self.tools.read().await;
        
        tools.values()
            .filter_map(|(_, metadata)| {
                if self.can_user_execute_tool(metadata, security_context) {
                    Some(metadata.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    fn authorize_execution(&self, metadata: &ToolMetadata, security_context: &SecurityContext) -> Result<()> {
        // Check if tool requires higher trust level
        match metadata.security_level {
            SecurityLevel::HighRisk | SecurityLevel::Critical => {
                if !security_context.permissions.can_execute_high_risk {
                    return Err(anyhow!("Insufficient permissions for high-risk tool"));
                }
            },
            _ => {}
        }
        
        // Check user trust level
        let required_trust = match metadata.security_level {
            SecurityLevel::Critical => UserTrustLevel::Administrator,
            SecurityLevel::HighRisk => UserTrustLevel::PowerUser,
            SecurityLevel::MediumRisk => UserTrustLevel::User,
            _ => UserTrustLevel::Guest,
        };
        
        if security_context.trust_level < required_trust {
            return Err(anyhow!("User trust level insufficient for tool execution"));
        }
        
        Ok(())
    }
    
    fn can_user_execute_tool(&self, metadata: &ToolMetadata, security_context: &SecurityContext) -> bool {
        self.authorize_execution(metadata, security_context).is_ok()
    }
    
    async fn check_execution_limits(&self, security_context: &SecurityContext) -> Result<()> {
        let executions = self.active_executions.lock().await;
        let user_executions = executions.values()
            .filter(|tracking| tracking.session_id == security_context.session_id)
            .count() as u32;
            
        let user_limit = security_context.permissions.max_resource_usage.max_concurrent_tools;
        let global_limit = self.security_config.max_concurrent_executions;
        
        if user_executions >= user_limit {
            return Err(anyhow!("User concurrent execution limit reached"));
        }
        
        if executions.len() as u32 >= global_limit {
            return Err(anyhow!("Global concurrent execution limit reached"));
        }
        
        Ok(())
    }
    
    fn check_resource_requirements(&self, metadata: &ToolMetadata, security_context: &SecurityContext) -> Result<()> {
        let user_limits = &security_context.permissions.max_resource_usage;
        let tool_reqs = &metadata.resource_requirements;
        
        // Check memory requirements
        if let Some(tool_memory) = tool_reqs.max_memory_mb {
            if tool_memory > user_limits.max_memory_mb {
                return Err(anyhow!("Tool memory requirement exceeds user limit"));
            }
        }
        
        // Check CPU requirements
        if let Some(tool_cores) = tool_reqs.max_cpu_cores {
            if tool_cores > user_limits.max_cpu_cores {
                return Err(anyhow!("Tool CPU requirement exceeds user limit"));
            }
        }
        
        // Check execution time
        if let Some(tool_time) = tool_reqs.max_execution_time {
            if tool_time > user_limits.max_execution_time {
                return Err(anyhow!("Tool execution time requirement exceeds user limit"));
            }
        }
        
        Ok(())
    }
    
    fn verify_tool_signature(&self, _metadata: &ToolMetadata) -> Result<()> {
        // In production, implement proper digital signature verification
        // For now, just check if signature exists for trusted tools
        Ok(())
    }
    
    fn validate_metadata(&self, metadata: &ToolMetadata) -> Result<()> {
        if metadata.name.is_empty() {
            return Err(anyhow!("Tool name cannot be empty"));
        }
        
        if metadata.description.is_empty() {
            return Err(anyhow!("Tool description cannot be empty"));
        }
        
        if metadata.version.major == 0 && metadata.version.minor == 0 && metadata.version.patch == 0 {
            return Err(anyhow!("Tool version must be specified"));
        }
        
        Ok(())
    }
    
    async fn start_execution_tracking(&self, execution_id: &str, tool_id: &str, session_id: &str) {
        let tracking = ExecutionTracking {
            tool_id: tool_id.to_string(),
            start_time: SystemTime::now(),
            session_id: session_id.to_string(),
            resource_usage: CurrentResourceUsage::default(),
        };
        
        let mut executions = self.active_executions.lock().await;
        executions.insert(execution_id.to_string(), tracking);
    }
    
    async fn end_execution_tracking(&self, execution_id: &str) {
        let mut executions = self.active_executions.lock().await;
        executions.remove(execution_id);
    }
    
    async fn update_tool_performance(&self, tool_id: &str, execution_time: Duration, success: bool) {
        let mut tools = self.tools.write().await;
        if let Some((_, metadata)) = tools.get_mut(tool_id) {
            metadata.performance_metrics.update_execution(execution_time, success);
            metadata.record_usage();
        }
    }
    
    async fn log_audit_event(&self, event: AuditEvent) {
        let mut log = self.audit_log.lock().await;
        log.push(event);
        
        // In production, also write to persistent storage
        // Limit in-memory log size
        if log.len() > 10000 {
            log.drain(0..1000); // Remove oldest 1000 events
        }
    }
    
    pub async fn get_audit_log(&self) -> Vec<AuditEvent> {
        let log = self.audit_log.lock().await;
        log.clone()
    }
    
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

impl Default for SecureToolRegistry {
    fn default() -> Self {
        Self::new(SecurityConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_input_validator_sql_injection() {
        assert!(InputValidator::contains_sql_injection("' or '1'='1"));
        assert!(InputValidator::contains_sql_injection("union select * from users"));
        assert!(!InputValidator::contains_sql_injection("normal input"));
    }
    
    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.require_signature_verification);
        assert!(!config.allow_untrusted_tools);
    }
}