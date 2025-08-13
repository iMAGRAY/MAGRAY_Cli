// P1.2.4.a Step 2: Resource Limits (2м)
// Memory, execution time, and fuel limits enforcement for WASM sandbox

use crate::sandbox::SandboxError;
use anyhow::Result;
use std::time::Duration;

/// Resource limits for WASM execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum execution time in milliseconds
    pub max_execution_time_ms: u64,
    /// Maximum WASM fuel (instruction count)
    pub fuel_limit: Option<u64>,
    /// Maximum WASM stack size in bytes
    pub max_wasm_stack: usize,
}

impl ResourceLimits {
    /// Create new resource limits with validation
    pub fn new(
        max_memory_mb: u64,
        max_execution_time_ms: u64,
        fuel_limit: Option<u64>,
    ) -> Result<Self, SandboxError> {
        let limits = Self {
            max_memory_bytes: max_memory_mb * 1024 * 1024, // Convert MB to bytes
            max_execution_time_ms,
            fuel_limit,
            max_wasm_stack: 512 * 1024, // 512KB default
        };

        limits.validate()?;
        Ok(limits)
    }

    /// Create restrictive limits for untrusted code
    pub fn restrictive() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024, // 16MB
            max_execution_time_ms: 5000,        // 5 seconds
            fuel_limit: Some(1_000_000),        // 1M instructions
            max_wasm_stack: 256 * 1024,         // 256KB
        }
    }

    /// Create permissive limits for trusted code
    pub fn permissive() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024, // 256MB
            max_execution_time_ms: 60_000,       // 60 seconds
            fuel_limit: Some(100_000_000),       // 100M instructions
            max_wasm_stack: 2 * 1024 * 1024,     // 2MB
        }
    }

    /// Validate limits are within acceptable ranges
    pub fn validate(&self) -> Result<(), SandboxError> {
        // Memory limits
        if self.max_memory_bytes == 0 {
            return Err(SandboxError::InitializationError(
                "Memory limit cannot be zero".to_string(),
            ));
        }

        if self.max_memory_bytes > 2 * 1024 * 1024 * 1024 {
            return Err(SandboxError::InitializationError(
                "Memory limit too high (max 2GB)".to_string(),
            ));
        }

        // Execution time limits
        if self.max_execution_time_ms == 0 {
            return Err(SandboxError::InitializationError(
                "Execution time limit cannot be zero".to_string(),
            ));
        }

        if self.max_execution_time_ms > 300_000 {
            return Err(SandboxError::InitializationError(
                "Execution time limit too high (max 5 minutes)".to_string(),
            ));
        }

        // Fuel limits
        if let Some(fuel) = self.fuel_limit {
            if fuel == 0 {
                return Err(SandboxError::InitializationError(
                    "Fuel limit cannot be zero".to_string(),
                ));
            }

            if fuel > 1_000_000_000 {
                return Err(SandboxError::InitializationError(
                    "Fuel limit too high (max 1B instructions)".to_string(),
                ));
            }
        }

        // Stack limits
        if self.max_wasm_stack == 0 {
            return Err(SandboxError::InitializationError(
                "WASM stack limit cannot be zero".to_string(),
            ));
        }

        if self.max_wasm_stack > 16 * 1024 * 1024 {
            return Err(SandboxError::InitializationError(
                "WASM stack limit too high (max 16MB)".to_string(),
            ));
        }

        Ok(())
    }

    /// Get execution timeout as Duration
    pub fn execution_timeout(&self) -> Duration {
        Duration::from_millis(self.max_execution_time_ms)
    }

    /// Get memory limit in MB
    pub fn memory_limit_mb(&self) -> u64 {
        self.max_memory_bytes / (1024 * 1024)
    }

    /// Check if another limits configuration is more restrictive
    pub fn is_more_restrictive_than(&self, other: &ResourceLimits) -> bool {
        self.max_memory_bytes <= other.max_memory_bytes
            && self.max_execution_time_ms <= other.max_execution_time_ms
            && self.max_wasm_stack <= other.max_wasm_stack
            && match (self.fuel_limit, other.fuel_limit) {
                (Some(a), Some(b)) => a <= b,
                (Some(_), None) => true, // Having a limit is more restrictive
                (None, Some(_)) => false, // No limit is less restrictive
                (None, None) => true,    // Equal restrictiveness
            }
    }

    /// Scale limits by a factor (for dynamic adjustment)
    pub fn scale(&self, factor: f64) -> Result<Self, SandboxError> {
        if factor <= 0.0 || factor > 10.0 {
            return Err(SandboxError::InitializationError(
                "Scale factor must be between 0 and 10".to_string(),
            ));
        }

        let scaled = Self {
            max_memory_bytes: ((self.max_memory_bytes as f64) * factor) as u64,
            max_execution_time_ms: ((self.max_execution_time_ms as f64) * factor) as u64,
            fuel_limit: self.fuel_limit.map(|f| ((f as f64) * factor) as u64),
            max_wasm_stack: ((self.max_wasm_stack as f64) * factor) as usize,
        };

        scaled.validate()?;
        Ok(scaled)
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_execution_time_ms: 30_000,      // 30 seconds
            fuel_limit: Some(10_000_000),       // 10M instructions
            max_wasm_stack: 512 * 1024,         // 512KB
        }
    }
}

/// Resource limiter that integrates with wasmtime
#[cfg(feature = "wasm-runtime")]
pub struct ResourceLimiter {
    limits: ResourceLimits,
    current_memory: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(feature = "wasm-runtime")]
impl ResourceLimiter {
    /// Create new resource limiter
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            current_memory: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get current memory usage
    pub fn current_memory_usage(&self) -> u64 {
        self.current_memory
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Check if memory limit would be exceeded
    pub fn would_exceed_memory(&self, additional_bytes: u64) -> bool {
        let current = self.current_memory_usage();
        current + additional_bytes > self.limits.max_memory_bytes
    }

    /// Get resource limits
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

#[cfg(feature = "wasm-runtime")]
impl wasmtime::ResourceLimiter for ResourceLimiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        let desired_bytes = desired as u64;

        if desired_bytes > self.limits.max_memory_bytes {
            return Ok(false);
        }

        // Update current memory tracking
        self.current_memory
            .store(desired_bytes, std::sync::atomic::Ordering::Relaxed);

        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        // Allow table growth but with reasonable limits
        const MAX_TABLE_SIZE: usize = 100_000;
        Ok(desired <= MAX_TABLE_SIZE)
    }
}

/// Resource usage tracking during execution
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Peak memory usage in bytes
    pub peak_memory_bytes: u64,
    /// Total execution time in microseconds
    pub execution_time_us: u64,
    /// Total fuel consumed (if available)
    pub fuel_consumed: Option<u64>,
    /// Number of memory allocations
    pub allocation_count: u64,
}

impl ResourceUsage {
    /// Check if usage violates limits
    pub fn violates_limits(&self, limits: &ResourceLimits) -> Option<SandboxError> {
        // Check memory limit
        if self.peak_memory_bytes > limits.max_memory_bytes {
            return Some(SandboxError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                current: self.peak_memory_bytes,
                limit: limits.max_memory_bytes,
            });
        }

        // Check time limit
        let execution_time_ms = self.execution_time_us / 1000;
        if execution_time_ms > limits.max_execution_time_ms {
            return Some(SandboxError::ResourceLimitExceeded {
                resource: "execution_time".to_string(),
                current: execution_time_ms,
                limit: limits.max_execution_time_ms,
            });
        }

        // Check fuel limit
        if let (Some(consumed), Some(limit)) = (self.fuel_consumed, limits.fuel_limit) {
            if consumed > limit {
                return Some(SandboxError::ResourceLimitExceeded {
                    resource: "fuel".to_string(),
                    current: consumed,
                    limit,
                });
            }
        }

        None
    }

    /// Calculate efficiency score (0.0 to 1.0)
    pub fn efficiency_score(&self, limits: &ResourceLimits) -> f64 {
        let memory_efficiency = (self.peak_memory_bytes as f64) / (limits.max_memory_bytes as f64);
        let time_efficiency =
            (self.execution_time_us as f64 / 1000.0) / (limits.max_execution_time_ms as f64);

        let fuel_efficiency =
            if let (Some(consumed), Some(limit)) = (self.fuel_consumed, limits.fuel_limit) {
                (consumed as f64) / (limit as f64)
            } else {
                0.5 // Neutral if no fuel tracking
            };

        // Lower usage = higher efficiency
        1.0 - ((memory_efficiency + time_efficiency + fuel_efficiency) / 3.0).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_creation() {
        let limits = ResourceLimits::new(64, 30000, Some(1_000_000))
            .expect("Operation failed - converted from unwrap()");
        assert_eq!(limits.max_memory_bytes, 64 * 1024 * 1024);
        assert_eq!(limits.max_execution_time_ms, 30000);
        assert_eq!(limits.fuel_limit, Some(1_000_000));
    }

    #[test]
    fn test_resource_limits_validation() {
        // Valid limits
        let valid = ResourceLimits::new(16, 5000, Some(1_000_000));
        assert!(valid.is_ok());

        // Invalid limits
        let invalid_memory = ResourceLimits::new(0, 5000, Some(1_000_000));
        assert!(invalid_memory.is_err());

        let invalid_time = ResourceLimits::new(16, 0, Some(1_000_000));
        assert!(invalid_time.is_err());
    }

    #[test]
    fn test_restrictive_vs_permissive() {
        let restrictive = ResourceLimits::restrictive();
        let permissive = ResourceLimits::permissive();

        assert!(restrictive.is_more_restrictive_than(&permissive));
        assert!(!permissive.is_more_restrictive_than(&restrictive));
    }

    #[test]
    fn test_limits_scaling() {
        let base = ResourceLimits::default();
        let scaled = base
            .scale(0.5)
            .expect("Operation failed - converted from unwrap()");

        assert_eq!(scaled.max_memory_bytes, base.max_memory_bytes / 2);
        assert_eq!(scaled.max_execution_time_ms, base.max_execution_time_ms / 2);
    }

    #[test]
    fn test_resource_usage_violation_detection() {
        let limits = ResourceLimits::restrictive();
        let usage = ResourceUsage {
            peak_memory_bytes: limits.max_memory_bytes + 1,
            execution_time_us: 1000,
            fuel_consumed: Some(100),
            allocation_count: 10,
        };

        assert!(usage.violates_limits(&limits).is_some());
    }

    #[test]
    fn test_efficiency_calculation() {
        let limits = ResourceLimits::default();
        let efficient_usage = ResourceUsage {
            peak_memory_bytes: limits.max_memory_bytes / 10, // 10% memory usage
            execution_time_us: (limits.max_execution_time_ms * 100) / 10, // 10% time usage
            fuel_consumed: limits.fuel_limit.map(|f| f / 10), // 10% fuel usage
            allocation_count: 5,
        };

        let efficiency = efficient_usage.efficiency_score(&limits);
        assert!(efficiency > 0.8); // Should be highly efficient
    }

    #[cfg(feature = "wasm-runtime")]
    #[test]
    fn test_resource_limiter_creation() {
        let limits = ResourceLimits::default();
        let limiter = ResourceLimiter::new(limits.clone());

        assert_eq!(limiter.limits().max_memory_bytes, limits.max_memory_bytes);
        assert_eq!(limiter.current_memory_usage(), 0);
    }
}
