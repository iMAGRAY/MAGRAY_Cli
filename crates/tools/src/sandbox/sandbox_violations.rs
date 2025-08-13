// P1.2.4.a Step 3: Sandbox Violations Detection (2м)
// Detection and handling of sandbox escape attempts and security violations

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Types of sandbox violations
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationType {
    /// Unauthorized filesystem access attempt
    FilesystemViolation {
        attempted_path: String,
        operation: String, // "read", "write", "execute"
    },
    /// Unauthorized network access attempt
    NetworkViolation {
        attempted_host: String,
        port: Option<u16>,
    },
    /// Resource limit exceeded
    ResourceLimitViolation {
        resource: String,
        attempted_value: u64,
        limit: u64,
    },
    /// Memory access violation (segfault, buffer overflow, etc.)
    MemoryViolation {
        violation_type: String,
        address: Option<u64>,
    },
    /// Execution time limit exceeded
    TimeoutViolation {
        execution_time_ms: u64,
        limit_ms: u64,
    },
    /// Fuel limit exceeded (instruction count)
    FuelViolation {
        instructions_executed: u64,
        limit: u64,
    },
    /// Attempted system call not allowed in sandbox
    SystemCallViolation { syscall: String, args: Vec<String> },
    /// Attempted to access prohibited environment variables
    EnvironmentViolation { variable_name: String },
    /// Sandbox escape attempt detected
    EscapeAttempt { method: String, details: String },
}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationType::FilesystemViolation {
                attempted_path,
                operation,
            } => {
                write!(f, "Filesystem {operation} violation: {attempted_path}")
            }
            ViolationType::NetworkViolation {
                attempted_host,
                port,
            } => {
                if let Some(port) = port {
                    write!(f, "Network violation: {attempted_host}:{port}")
                } else {
                    write!(f, "Network violation: {attempted_host}")
                }
            }
            ViolationType::ResourceLimitViolation {
                resource,
                attempted_value,
                limit,
            } => {
                write!(
                    f,
                    "Resource {resource} limit violation: {attempted_value} > {limit}"
                )
            }
            ViolationType::MemoryViolation {
                violation_type,
                address,
            } => {
                if let Some(addr) = address {
                    write!(f, "Memory violation ({violation_type}): 0x{addr:x}")
                } else {
                    write!(f, "Memory violation ({violation_type})")
                }
            }
            ViolationType::TimeoutViolation {
                execution_time_ms,
                limit_ms,
            } => {
                write!(f, "Timeout violation: {execution_time_ms}ms > {limit_ms}ms")
            }
            ViolationType::FuelViolation {
                instructions_executed,
                limit,
            } => {
                write!(
                    f,
                    "Fuel violation: {instructions_executed} > {limit} instructions"
                )
            }
            ViolationType::SystemCallViolation { syscall, args } => {
                write!(f, "Syscall violation: {}({})", syscall, args.join(", "))
            }
            ViolationType::EnvironmentViolation { variable_name } => {
                write!(f, "Environment variable violation: {variable_name}")
            }
            ViolationType::EscapeAttempt { method, details } => {
                write!(f, "Sandbox escape attempt via {method}: {details}")
            }
        }
    }
}

/// Sandbox violation record with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxViolation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// Timestamp when violation occurred
    pub timestamp: u64,
    /// WASM module that caused the violation
    pub module_name: Option<String>,
    /// Function that was executing when violation occurred
    pub function_name: Option<String>,
    /// Additional context or details
    pub context: String,
    /// Severity level (1-10, higher = more severe)
    pub severity: u8,
    /// Whether the violation was blocked or allowed
    pub blocked: bool,
}

impl SandboxViolation {
    /// Create a new sandbox violation record
    pub fn new(
        violation_type: ViolationType,
        context: String,
        module_name: Option<String>,
        function_name: Option<String>,
    ) -> Self {
        let severity = Self::calculate_severity(&violation_type);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            violation_type,
            timestamp,
            module_name,
            function_name,
            context,
            severity,
            blocked: true, // Default to blocked for security
        }
    }

    /// Create filesystem access violation
    pub fn filesystem_violation(
        path: String,
        operation: String,
        module_name: Option<String>,
        blocked: bool,
    ) -> Self {
        let mut violation = Self::new(
            ViolationType::FilesystemViolation {
                attempted_path: path,
                operation,
            },
            "Unauthorized filesystem access".to_string(),
            module_name,
            None,
        );
        violation.blocked = blocked;
        violation
    }

    /// Create network access violation
    pub fn network_violation(
        host: String,
        port: Option<u16>,
        module_name: Option<String>,
        blocked: bool,
    ) -> Self {
        let mut violation = Self::new(
            ViolationType::NetworkViolation {
                attempted_host: host,
                port,
            },
            "Unauthorized network access".to_string(),
            module_name,
            None,
        );
        violation.blocked = blocked;
        violation
    }

    /// Create resource limit violation
    pub fn resource_limit_violation(
        resource: String,
        attempted_value: u64,
        limit: u64,
        module_name: Option<String>,
    ) -> Self {
        Self::new(
            ViolationType::ResourceLimitViolation {
                resource,
                attempted_value,
                limit,
            },
            "Resource limit exceeded".to_string(),
            module_name,
            None,
        )
    }

    /// Create timeout violation
    pub fn timeout_violation(
        execution_time_ms: u64,
        limit_ms: u64,
        module_name: Option<String>,
        function_name: Option<String>,
    ) -> Self {
        Self::new(
            ViolationType::TimeoutViolation {
                execution_time_ms,
                limit_ms,
            },
            "Execution timeout exceeded".to_string(),
            module_name,
            function_name,
        )
    }

    /// Create fuel exhaustion violation
    pub fn fuel_violation(
        instructions_executed: u64,
        limit: u64,
        module_name: Option<String>,
        function_name: Option<String>,
    ) -> Self {
        Self::new(
            ViolationType::FuelViolation {
                instructions_executed,
                limit,
            },
            "Fuel limit exceeded".to_string(),
            module_name,
            function_name,
        )
    }

    /// Create escape attempt violation
    pub fn escape_attempt(method: String, details: String, module_name: Option<String>) -> Self {
        Self::new(
            ViolationType::EscapeAttempt { method, details },
            "Sandbox escape attempt detected".to_string(),
            module_name,
            None,
        )
    }

    /// Calculate severity based on violation type
    fn calculate_severity(violation_type: &ViolationType) -> u8 {
        match violation_type {
            ViolationType::FilesystemViolation { operation, .. } => match operation.as_str() {
                "read" => 4,
                "write" => 7,
                "execute" => 9,
                _ => 5,
            },
            ViolationType::NetworkViolation { .. } => 6,
            ViolationType::ResourceLimitViolation { resource, .. } => match resource.as_str() {
                "memory" => 5,
                "fuel" => 3,
                "time" => 4,
                _ => 4,
            },
            ViolationType::MemoryViolation { .. } => 8,
            ViolationType::TimeoutViolation { .. } => 4,
            ViolationType::FuelViolation { .. } => 3,
            ViolationType::SystemCallViolation { .. } => 8,
            ViolationType::EnvironmentViolation { .. } => 5,
            ViolationType::EscapeAttempt { .. } => 10,
        }
    }

    /// Check if violation indicates potential malicious activity
    pub fn is_potentially_malicious(&self) -> bool {
        self.severity >= 8
            || matches!(
                self.violation_type,
                ViolationType::EscapeAttempt { .. }
                    | ViolationType::SystemCallViolation { .. }
                    | ViolationType::MemoryViolation { .. }
            )
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        format!("{}: {}", self.violation_type, self.context)
    }

    /// Get security impact assessment
    pub fn security_impact(&self) -> SecurityImpact {
        match self.severity {
            1..=3 => SecurityImpact::Low,
            4..=6 => SecurityImpact::Medium,
            7..=8 => SecurityImpact::High,
            9..=10 => SecurityImpact::Critical,
            _ => SecurityImpact::Unknown,
        }
    }
}

/// Security impact levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityImpact {
    Low,
    Medium,
    High,
    Critical,
    Unknown,
}

impl fmt::Display for SecurityImpact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecurityImpact::Low => write!(f, "Low"),
            SecurityImpact::Medium => write!(f, "Medium"),
            SecurityImpact::High => write!(f, "High"),
            SecurityImpact::Critical => write!(f, "Critical"),
            SecurityImpact::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Collection of sandbox violations for analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViolationLog {
    violations: Vec<SandboxViolation>,
    max_entries: usize,
}

impl ViolationLog {
    /// Create new violation log with maximum entries
    pub fn new(max_entries: usize) -> Self {
        Self {
            violations: Vec::new(),
            max_entries,
        }
    }

    /// Add violation to log
    pub fn log_violation(&mut self, violation: SandboxViolation) {
        self.violations.push(violation);

        // Enforce maximum entries (remove oldest)
        if self.violations.len() > self.max_entries {
            self.violations.remove(0);
        }
    }

    /// Get all violations
    pub fn violations(&self) -> &[SandboxViolation] {
        &self.violations
    }

    /// Get violations by severity
    pub fn violations_by_severity(&self, min_severity: u8) -> Vec<&SandboxViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity >= min_severity)
            .collect()
    }

    /// Get violations by type
    pub fn violations_by_type(&self, violation_type: &ViolationType) -> Vec<&SandboxViolation> {
        self.violations
            .iter()
            .filter(|v| {
                std::mem::discriminant(&v.violation_type) == std::mem::discriminant(violation_type)
            })
            .collect()
    }

    /// Check if there are critical violations
    pub fn has_critical_violations(&self) -> bool {
        self.violations.iter().any(|v| v.severity >= 9)
    }

    /// Get violation statistics
    pub fn statistics(&self) -> ViolationStatistics {
        let total = self.violations.len();
        let blocked = self.violations.iter().filter(|v| v.blocked).count();
        let critical = self.violations.iter().filter(|v| v.severity >= 9).count();
        let high = self
            .violations
            .iter()
            .filter(|v| v.severity >= 7 && v.severity < 9)
            .count();
        let malicious = self
            .violations
            .iter()
            .filter(|v| v.is_potentially_malicious())
            .count();

        ViolationStatistics {
            total,
            blocked,
            critical,
            high,
            malicious,
        }
    }

    /// Clear all violations
    pub fn clear(&mut self) {
        self.violations.clear();
    }
}

/// Statistics about violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationStatistics {
    pub total: usize,
    pub blocked: usize,
    pub critical: usize,
    pub high: usize,
    pub malicious: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_violation_creation() {
        let violation = SandboxViolation::filesystem_violation(
            "/etc/passwd".to_string(),
            "read".to_string(),
            Some("malicious.wasm".to_string()),
            true,
        );

        assert_eq!(violation.severity, 4);
        assert!(violation.blocked);
        assert!(violation.module_name.is_some());
    }

    #[test]
    fn test_severity_calculation() {
        let read_violation = ViolationType::FilesystemViolation {
            attempted_path: "/tmp".to_string(),
            operation: "read".to_string(),
        };

        let execute_violation = ViolationType::FilesystemViolation {
            attempted_path: "/bin/sh".to_string(),
            operation: "execute".to_string(),
        };

        assert!(
            SandboxViolation::calculate_severity(&execute_violation)
                > SandboxViolation::calculate_severity(&read_violation)
        );
    }

    #[test]
    fn test_malicious_detection() {
        let escape_attempt = SandboxViolation::escape_attempt(
            "buffer_overflow".to_string(),
            "Attempted to overflow WASM linear memory".to_string(),
            Some("evil.wasm".to_string()),
        );

        assert!(escape_attempt.is_potentially_malicious());
        assert_eq!(escape_attempt.security_impact(), SecurityImpact::Critical);
    }

    #[test]
    fn test_violation_log() {
        let mut log = ViolationLog::new(3);

        log.log_violation(SandboxViolation::filesystem_violation(
            "/tmp".to_string(),
            "read".to_string(),
            None,
            true,
        ));

        assert_eq!(log.violations().len(), 1);

        // Add more violations to test max entries
        for i in 0..5 {
            log.log_violation(SandboxViolation::network_violation(
                format!("host{i}.com"),
                Some(80),
                None,
                true,
            ));
        }

        assert_eq!(log.violations().len(), 3); // Should be capped at max_entries
    }

    #[test]
    fn test_violation_statistics() {
        let mut log = ViolationLog::new(10);

        // Add various violations
        log.log_violation(SandboxViolation::filesystem_violation(
            "/tmp".to_string(),
            "read".to_string(),
            None,
            true,
        ));
        log.log_violation(SandboxViolation::escape_attempt(
            "overflow".to_string(),
            "details".to_string(),
            None,
        ));
        log.log_violation(SandboxViolation::network_violation(
            "evil.com".to_string(),
            None,
            None,
            false,
        ));

        let stats = log.statistics();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.blocked, 2);
        assert_eq!(stats.critical, 1);
        assert_eq!(stats.malicious, 1);
    }

    #[test]
    fn test_violation_filtering() {
        let mut log = ViolationLog::new(10);

        log.log_violation(SandboxViolation::filesystem_violation(
            "/tmp".to_string(),
            "read".to_string(),
            None,
            true,
        ));
        log.log_violation(SandboxViolation::escape_attempt(
            "overflow".to_string(),
            "details".to_string(),
            None,
        ));

        let critical_violations = log.violations_by_severity(9);
        assert_eq!(critical_violations.len(), 1);

        let filesystem_violations = log.violations_by_type(&ViolationType::FilesystemViolation {
            attempted_path: String::new(),
            operation: String::new(),
        });
        assert_eq!(filesystem_violations.len(), 1);
    }
}
