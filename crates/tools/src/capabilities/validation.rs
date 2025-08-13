// Capability Validation System
// P1.2.3: Permission validation and security enforcement

use super::{AccessMode, Capability, CapabilityError, CapabilitySpec, NetworkMode, UIMode};
use std::path::PathBuf;
use tracing::{debug, error, warn};

/// Capability validation engine
pub struct CapabilityValidator {
    /// Security level configuration
    security_level: SecurityLevel,
}

/// Security enforcement levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityLevel {
    /// Minimal validation - allow most capabilities
    Permissive,
    /// Balanced validation - reasonable restrictions
    Balanced,
    /// Strict validation - maximum security
    Strict,
}

/// Validation context for capability checking
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Tool identifier requesting capabilities
    pub tool_id: String,
    /// Tool type for context-specific validation
    pub tool_type: String,
    /// Current working directory
    pub working_dir: PathBuf,
    /// Available system resources
    pub system_resources: SystemResources,
}

/// System resource information
#[derive(Debug, Clone)]
pub struct SystemResources {
    /// Available memory in MB
    pub available_memory_mb: u64,
    /// Available CPU cores
    pub cpu_cores: u32,
    /// Available disk space in MB
    pub available_disk_mb: u64,
}

impl CapabilityValidator {
    /// Create validator with specified security level
    pub fn new(security_level: SecurityLevel) -> Self {
        Self { security_level }
    }

    /// Validate a complete capability specification
    pub fn validate_capability_spec(
        &self,
        spec: &CapabilitySpec,
        context: &ValidationContext,
    ) -> Result<ValidationResult, CapabilityError> {
        debug!("Validating capability spec for tool: {}", context.tool_id);

        let mut result = ValidationResult::new();

        // Validate each required capability
        for capability in &spec.required {
            match self.validate_single_capability(capability, context) {
                Ok(cap_result) => {
                    result.add_capability_result(capability.clone(), cap_result);
                }
                Err(e) => {
                    result.add_error(capability.clone(), e);
                }
            }
        }

        // Validate optional capabilities
        for capability in &spec.optional {
            match self.validate_single_capability(capability, context) {
                Ok(cap_result) => {
                    result.add_optional_capability_result(capability.clone(), cap_result);
                }
                Err(e) => {
                    warn!("Optional capability validation failed: {:?}", e);
                    // Optional capabilities don't fail the entire validation
                }
            }
        }

        // Perform cross-capability validation
        self.validate_capability_interactions(&spec.all_capabilities(), &mut result)?;

        // Check against security level
        self.apply_security_level_constraints(spec, &mut result)?;

        Ok(result)
    }

    /// Validate a single capability
    fn validate_single_capability(
        &self,
        capability: &Capability,
        context: &ValidationContext,
    ) -> Result<SingleCapabilityResult, CapabilityError> {
        debug!("Validating capability: {:?}", capability);

        let mut result = SingleCapabilityResult {
            capability: capability.clone(),
            allowed: true,
            restrictions: Vec::new(),
            warnings: Vec::new(),
        };

        match capability {
            Capability::Filesystem { mode, paths } => {
                self.validate_filesystem_capability(mode, paths, context, &mut result)?;
            }
            Capability::Network { mode, domains } => {
                self.validate_network_capability(mode, domains, context, &mut result)?;
            }
            Capability::Shell { commands, elevated } => {
                self.validate_shell_capability(commands, *elevated, context, &mut result)?;
            }
            Capability::UI { modes } => {
                self.validate_ui_capability(modes, context, &mut result)?;
            }
            Capability::Memory { max_mb } => {
                self.validate_memory_capability(*max_mb, context, &mut result)?;
            }
            Capability::Compute {
                max_cpu_percent,
                max_duration_ms,
            } => {
                self.validate_compute_capability(
                    *max_cpu_percent,
                    *max_duration_ms,
                    context,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    fn validate_filesystem_capability(
        &self,
        mode: &AccessMode,
        paths: &[PathBuf],
        context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        for path in paths {
            // Normalize path relative to working directory
            let normalized_path = if path.is_absolute() {
                path.clone()
            } else {
                context.working_dir.join(path)
            };

            // Check for path traversal
            if normalized_path.to_string_lossy().contains("..") {
                return Err(CapabilityError::security_violation(format!(
                    "Path traversal detected in: {path:?}"
                )));
            }

            // Apply security level restrictions
            match self.security_level {
                SecurityLevel::Strict => {
                    if !normalized_path.starts_with(&context.working_dir) {
                        result
                            .restrictions
                            .push(format!("Path restricted to working directory: {path:?}"));
                        result.allowed = false;
                    }
                }
                SecurityLevel::Balanced => {
                    // Allow access to common safe directories
                    let safe_prefixes = ["/tmp", "/var/tmp", "./", "../"];
                    let is_safe = safe_prefixes
                        .iter()
                        .any(|prefix| normalized_path.starts_with(prefix));
                    if !is_safe && matches!(mode, AccessMode::Write | AccessMode::ReadWrite) {
                        result
                            .warnings
                            .push("Write access outside safe directories".to_string());
                    }
                }
                SecurityLevel::Permissive => {
                    // Minimal restrictions
                }
            }

            // Check execute permissions
            if matches!(mode, AccessMode::Execute) {
                result
                    .warnings
                    .push("Execute permission requested - high security risk".to_string());
                if matches!(self.security_level, SecurityLevel::Strict) {
                    result.allowed = false;
                    result
                        .restrictions
                        .push("Execute permissions not allowed in strict mode".to_string());
                }
            }
        }

        Ok(())
    }

    fn validate_network_capability(
        &self,
        mode: &NetworkMode,
        domains: &[String],
        _context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        for domain in domains {
            // Check for wildcard domains
            if domain == "*" {
                match self.security_level {
                    SecurityLevel::Strict => {
                        result.allowed = false;
                        result
                            .restrictions
                            .push("Wildcard domains not allowed in strict mode".to_string());
                    }
                    SecurityLevel::Balanced => {
                        result
                            .warnings
                            .push("Wildcard domain access - security risk".to_string());
                    }
                    SecurityLevel::Permissive => {
                        // Allow wildcard
                    }
                }
            }

            // Check for dangerous domains
            let dangerous_tlds = [".onion", ".bit"];
            for tld in dangerous_tlds {
                if domain.ends_with(tld) {
                    result
                        .warnings
                        .push(format!("Access to potentially dangerous domain: {domain}"));
                    if matches!(self.security_level, SecurityLevel::Strict) {
                        result.allowed = false;
                    }
                }
            }
        }

        // Check inbound connections
        if matches!(mode, NetworkMode::Inbound | NetworkMode::Both) {
            result
                .warnings
                .push("Inbound network connections - security risk".to_string());
            if matches!(self.security_level, SecurityLevel::Strict) {
                result
                    .restrictions
                    .push("Inbound connections require explicit approval".to_string());
            }
        }

        Ok(())
    }

    fn validate_shell_capability(
        &self,
        commands: &[String],
        elevated: bool,
        _context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        if elevated {
            result
                .warnings
                .push("Elevated shell access requested - high security risk".to_string());
            if !matches!(self.security_level, SecurityLevel::Permissive) {
                result.allowed = false;
                result
                    .restrictions
                    .push("Elevated shell access not allowed".to_string());
            }
        }

        // Check for dangerous commands
        let dangerous_commands = ["rm", "del", "format", "sudo", "su", "chmod", "chown"];
        for command in commands {
            if dangerous_commands.contains(&command.as_str()) {
                result
                    .warnings
                    .push(format!("Dangerous command requested: {command}"));
                if matches!(self.security_level, SecurityLevel::Strict) {
                    result.allowed = false;
                    result
                        .restrictions
                        .push(format!("Command not allowed: {command}"));
                }
            }
        }

        Ok(())
    }

    fn validate_ui_capability(
        &self,
        modes: &[UIMode],
        _context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        for mode in modes {
            match mode {
                UIMode::Display => {
                    // Display is generally safe
                }
                UIMode::Input => {
                    result
                        .warnings
                        .push("Input capture capability - privacy considerations".to_string());
                }
                UIMode::Notification => {
                    // Notifications are generally safe but can be annoying
                    if matches!(self.security_level, SecurityLevel::Strict) {
                        result
                            .warnings
                            .push("Notifications may be intrusive".to_string());
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_memory_capability(
        &self,
        max_mb: u64,
        context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        let available = context.system_resources.available_memory_mb;

        if max_mb > available {
            return Err(CapabilityError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                current: max_mb,
                limit: available,
            });
        }

        // Check against security level limits
        let limit = match self.security_level {
            SecurityLevel::Strict => 128,
            SecurityLevel::Balanced => 512,
            SecurityLevel::Permissive => 2048,
        };

        if max_mb > limit {
            result.warnings.push(format!(
                "Memory request {max_mb} MB exceeds recommended limit {limit} MB"
            ));
            if matches!(self.security_level, SecurityLevel::Strict) {
                result.allowed = false;
                result
                    .restrictions
                    .push(format!("Memory limit exceeded: {max_mb} > {limit} MB"));
            }
        }

        Ok(())
    }

    fn validate_compute_capability(
        &self,
        max_cpu_percent: u8,
        max_duration_ms: u64,
        _context: &ValidationContext,
        result: &mut SingleCapabilityResult,
    ) -> Result<(), CapabilityError> {
        // Check CPU percentage limits
        let cpu_limit = match self.security_level {
            SecurityLevel::Strict => 25,
            SecurityLevel::Balanced => 50,
            SecurityLevel::Permissive => 90,
        };

        if max_cpu_percent > cpu_limit {
            result.warnings.push(format!(
                "CPU usage {max_cpu_percent} % exceeds recommended limit {cpu_limit} %"
            ));
            if matches!(self.security_level, SecurityLevel::Strict) {
                result.allowed = false;
                result.restrictions.push(format!(
                    "CPU limit exceeded: {max_cpu_percent} > {cpu_limit} %"
                ));
            }
        }

        // Check duration limits
        let duration_limit_ms = match self.security_level {
            SecurityLevel::Strict => 30_000,        // 30 seconds
            SecurityLevel::Balanced => 300_000,     // 5 minutes
            SecurityLevel::Permissive => 3_600_000, // 1 hour
        };

        if max_duration_ms > duration_limit_ms {
            result.warnings.push(format!(
                "Duration {max_duration_ms} ms exceeds recommended limit {duration_limit_ms} ms"
            ));
        }

        Ok(())
    }

    /// Validate capability interactions
    fn validate_capability_interactions(
        &self,
        capabilities: &std::collections::HashSet<Capability>,
        result: &mut ValidationResult,
    ) -> Result<(), CapabilityError> {
        // Check for dangerous combinations
        let has_shell = capabilities
            .iter()
            .any(|c| matches!(c, Capability::Shell { .. }));
        let has_network = capabilities
            .iter()
            .any(|c| matches!(c, Capability::Network { .. }));
        let has_fs_write = capabilities.iter().any(|c| {
            matches!(
                c,
                Capability::Filesystem {
                    mode: AccessMode::Write | AccessMode::ReadWrite,
                    ..
                }
            )
        });

        if has_shell && has_network {
            result.global_warnings.push(
                "Shell + Network capabilities - potential for remote code execution".to_string(),
            );
        }

        if has_shell && has_fs_write {
            result
                .global_warnings
                .push("Shell + Filesystem write - potential for system modification".to_string());
        }

        Ok(())
    }

    /// Apply security level constraints
    fn apply_security_level_constraints(
        &self,
        spec: &CapabilitySpec,
        result: &mut ValidationResult,
    ) -> Result<(), CapabilityError> {
        let high_risk_count = spec
            .required
            .iter()
            .filter(|cap| cap.risk_level() > 7)
            .count();

        match self.security_level {
            SecurityLevel::Strict if high_risk_count > 0 => {
                result
                    .global_warnings
                    .push("High-risk capabilities not recommended in strict mode".to_string());
            }
            SecurityLevel::Balanced if high_risk_count > 2 => {
                result
                    .global_warnings
                    .push("Multiple high-risk capabilities detected".to_string());
            }
            _ => {}
        }

        Ok(())
    }
}

/// Result of capability validation
#[derive(Debug)]
pub struct ValidationResult {
    /// Results for individual capabilities
    pub capability_results: Vec<SingleCapabilityResult>,
    /// Results for optional capabilities
    pub optional_results: Vec<SingleCapabilityResult>,
    /// Global validation errors
    pub errors: Vec<(Capability, CapabilityError)>,
    /// Global warnings
    pub global_warnings: Vec<String>,
    /// Overall validation success
    pub success: bool,
}

/// Result for a single capability validation
#[derive(Debug)]
pub struct SingleCapabilityResult {
    pub capability: Capability,
    pub allowed: bool,
    pub restrictions: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            capability_results: Vec::new(),
            optional_results: Vec::new(),
            errors: Vec::new(),
            global_warnings: Vec::new(),
            success: true,
        }
    }

    fn add_capability_result(&mut self, capability: Capability, result: SingleCapabilityResult) {
        if !result.allowed {
            self.success = false;
        }
        self.capability_results.push(result);
    }

    fn add_optional_capability_result(
        &mut self,
        capability: Capability,
        result: SingleCapabilityResult,
    ) {
        self.optional_results.push(result);
    }

    fn add_error(&mut self, capability: Capability, error: CapabilityError) {
        self.success = false;
        self.errors.push((capability, error));
    }

    /// Get all denied capabilities
    pub fn denied_capabilities(&self) -> Vec<&Capability> {
        self.capability_results
            .iter()
            .filter(|r| !r.allowed)
            .map(|r| &r.capability)
            .collect()
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.success && self.errors.is_empty()
    }
}

// SecurityViolation helper method
impl CapabilityError {
    pub fn security_violation(details: String) -> Self {
        CapabilityError::PolicyViolation { details }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> ValidationContext {
        ValidationContext {
            tool_id: "test_tool".to_string(),
            tool_type: "test".to_string(),
            working_dir: PathBuf::from("/tmp"),
            system_resources: SystemResources {
                available_memory_mb: 1024,
                cpu_cores: 4,
                available_disk_mb: 10240,
            },
        }
    }

    #[test]
    fn test_filesystem_validation() {
        let validator = CapabilityValidator::new(SecurityLevel::Balanced);
        let context = test_context();

        let capability = Capability::Filesystem {
            mode: AccessMode::Read,
            paths: vec![PathBuf::from("./test.txt")],
        };

        let result = validator
            .validate_single_capability(&capability, &context)
            .expect("Operation failed - converted from unwrap()");
        assert!(result.allowed);
    }

    #[test]
    fn test_dangerous_shell_validation() {
        let validator = CapabilityValidator::new(SecurityLevel::Strict);
        let context = test_context();

        let capability = Capability::Shell {
            commands: vec!["rm".to_string()],
            elevated: true,
        };

        let result = validator
            .validate_single_capability(&capability, &context)
            .expect("Operation failed - converted from unwrap()");
        assert!(!result.allowed);
    }

    #[test]
    fn test_memory_limit_validation() {
        let validator = CapabilityValidator::new(SecurityLevel::Balanced);
        let context = test_context();

        let capability = Capability::Memory { max_mb: 2000 };

        // This should now fail due to memory limit exceeded
        let result = validator.validate_single_capability(&capability, &context);
        assert!(result.is_err());
        match result {
            Err(CapabilityError::ResourceLimitExceeded { resource, .. }) => {
                assert_eq!(resource, "memory");
            }
            _ => panic!("Expected ResourceLimitExceeded error"),
        }
    }
}
