//! Resource Management System
//!
//! This module provides resource budgeting, monitoring, and enforcement for actors.
//! It tracks CPU time, memory usage, message queue sizes, and timeouts to ensure
//! system stability and prevent resource exhaustion.

pub mod budget;

pub use budget::{BudgetViolation, ResourceBudget, ResourceMonitor, ResourceUsage};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Resource limits and configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU time per operation
    pub max_cpu_time: Option<Duration>,

    /// Maximum memory usage in bytes
    pub max_memory: Option<u64>,

    /// Maximum message queue size
    pub max_queue_size: Option<usize>,

    /// Operation timeout
    pub timeout: Option<Duration>,

    /// Maximum number of active tasks
    pub max_concurrent_tasks: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_time: Some(Duration::from_secs(30)),
            max_memory: Some(512 * 1024 * 1024), // 512MB
            max_queue_size: Some(1000),
            timeout: Some(Duration::from_secs(60)),
            max_concurrent_tasks: Some(10),
        }
    }
}

impl ResourceLimits {
    /// Create a new ResourceLimits with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum CPU time
    pub fn cpu_time(mut self, duration: Duration) -> Self {
        self.max_cpu_time = Some(duration);
        self
    }

    /// Set maximum memory usage
    pub fn memory(mut self, bytes: u64) -> Self {
        self.max_memory = Some(bytes);
        self
    }

    /// Set maximum queue size
    pub fn queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = Some(size);
        self
    }

    /// Set operation timeout
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Set maximum concurrent tasks
    pub fn concurrent_tasks(mut self, count: usize) -> Self {
        self.max_concurrent_tasks = Some(count);
        self
    }

    /// Remove CPU time limit
    pub fn unlimited_cpu(mut self) -> Self {
        self.max_cpu_time = None;
        self
    }

    /// Remove memory limit
    pub fn unlimited_memory(mut self) -> Self {
        self.max_memory = None;
        self
    }

    /// Check if usage exceeds limits
    pub fn check_violation(&self, usage: &ResourceUsage) -> Option<BudgetViolation> {
        if let (Some(limit), Some(used)) = (self.max_cpu_time, usage.cpu_time) {
            if used > limit {
                return Some(BudgetViolation::CpuTime { limit, used });
            }
        }

        if let (Some(limit), used) = (self.max_memory, usage.memory_bytes) {
            if used > limit {
                return Some(BudgetViolation::Memory { limit, used });
            }
        }

        if let (Some(limit), used) = (self.max_queue_size, usage.queue_size) {
            if used > limit {
                return Some(BudgetViolation::QueueSize { limit, used });
            }
        }

        if let (Some(limit), used) = (self.max_concurrent_tasks, usage.active_tasks) {
            if used > limit {
                return Some(BudgetViolation::ConcurrentTasks { limit, used });
            }
        }

        None
    }
}

/// Resource enforcement policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementPolicy {
    /// Log violations but continue
    Warn,

    /// Throttle the actor (slow down processing)
    Throttle,

    /// Kill the actor on violation
    Kill,

    /// Restart the actor on violation
    Restart,
}

impl Default for EnforcementPolicy {
    fn default() -> Self {
        EnforcementPolicy::Throttle
    }
}

/// Resource configuration for the entire system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceConfig {
    /// Default limits for new actors
    pub default_limits: ResourceLimits,

    /// System-wide enforcement policy
    pub enforcement_policy: EnforcementPolicy,

    /// Resource monitoring interval
    pub monitoring_interval: Duration,

    /// Enable detailed resource tracking
    pub detailed_tracking: bool,

    /// Memory pressure thresholds
    pub memory_pressure_warning: f64, // 0.0 to 1.0
    pub memory_pressure_critical: f64, // 0.0 to 1.0
}

impl Default for SystemResourceConfig {
    fn default() -> Self {
        Self {
            default_limits: ResourceLimits::default(),
            enforcement_policy: EnforcementPolicy::default(),
            monitoring_interval: Duration::from_secs(5),
            detailed_tracking: true,
            memory_pressure_warning: 0.8,   // 80%
            memory_pressure_critical: 0.95, // 95%
        }
    }
}
