//! Resource Budget and Monitoring Implementation
//!
//! This module provides detailed resource tracking, budget enforcement, and monitoring
//! for actors in the system. It integrates with system metrics to provide real-time
//! resource usage information and budget violation detection.

use super::{EnforcementPolicy, ResourceLimits, SystemResourceConfig};
use crate::actors::ActorId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Resource budget for an actor
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub limits: ResourceLimits,
    pub enforcement: EnforcementPolicy,
    pub priority: ActorPriority,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            limits: ResourceLimits::default(),
            enforcement: EnforcementPolicy::default(),
            priority: ActorPriority::Normal,
        }
    }
}

impl ResourceBudget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CPU time limit
    pub fn cpu_time_limit(mut self, duration: Duration) -> Self {
        self.limits.max_cpu_time = Some(duration);
        self
    }

    /// Set memory limit
    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.limits.max_memory = Some(bytes);
        self
    }

    /// Set message queue limit
    pub fn queue_limit(mut self, size: usize) -> Self {
        self.limits.max_queue_size = Some(size);
        self
    }

    /// Set operation timeout
    pub fn timeout_limit(mut self, duration: Duration) -> Self {
        self.limits.timeout = Some(duration);
        self
    }

    /// Set enforcement policy
    pub fn enforcement_policy(mut self, policy: EnforcementPolicy) -> Self {
        self.enforcement = policy;
        self
    }

    /// Set actor priority
    pub fn priority(mut self, priority: ActorPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Create high-priority budget with relaxed limits
    pub fn high_priority() -> Self {
        Self {
            limits: ResourceLimits {
                max_cpu_time: Some(Duration::from_secs(120)),
                max_memory: Some(1024 * 1024 * 1024), // 1GB
                max_queue_size: Some(5000),
                timeout: Some(Duration::from_secs(300)),
                max_concurrent_tasks: Some(50),
            },
            enforcement: EnforcementPolicy::Warn,
            priority: ActorPriority::High,
        }
    }

    /// Create low-priority budget with strict limits
    pub fn low_priority() -> Self {
        Self {
            limits: ResourceLimits {
                max_cpu_time: Some(Duration::from_secs(10)),
                max_memory: Some(128 * 1024 * 1024), // 128MB
                max_queue_size: Some(100),
                timeout: Some(Duration::from_secs(30)),
                max_concurrent_tasks: Some(5),
            },
            enforcement: EnforcementPolicy::Throttle,
            priority: ActorPriority::Low,
        }
    }
}

/// Actor priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActorPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for ActorPriority {
    fn default() -> Self {
        ActorPriority::Normal
    }
}

/// Current resource usage snapshot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: u64,

    /// CPU time used (if available)
    pub cpu_time: Option<Duration>,

    /// Current message queue size
    pub queue_size: usize,

    /// Number of active tasks
    pub active_tasks: usize,

    /// Timestamp of this measurement
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// System load average (if available)
    pub system_load: Option<f64>,

    /// Memory pressure (0.0 to 1.0)
    pub memory_pressure: f64,
}

impl ResourceUsage {
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            ..Default::default()
        }
    }

    /// Calculate resource pressure score (0.0 to 1.0)
    pub fn pressure_score(&self, limits: &ResourceLimits) -> f64 {
        let mut pressure = 0.0;
        let mut factors = 0;

        if let Some(max_memory) = limits.max_memory {
            pressure += (self.memory_bytes as f64) / (max_memory as f64);
            factors += 1;
        }

        if let Some(max_queue) = limits.max_queue_size {
            pressure += (self.queue_size as f64) / (max_queue as f64);
            factors += 1;
        }

        if let Some(max_tasks) = limits.max_concurrent_tasks {
            pressure += (self.active_tasks as f64) / (max_tasks as f64);
            factors += 1;
        }

        if factors > 0 {
            (pressure / factors as f64).min(1.0)
        } else {
            0.0
        }
    }
}

/// Resource budget violation
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum BudgetViolation {
    #[error("CPU time limit exceeded: used {used:?}, limit {limit:?}")]
    CpuTime { limit: Duration, used: Duration },

    #[error("Memory limit exceeded: used {used} bytes, limit {limit} bytes")]
    Memory { limit: u64, used: u64 },

    #[error("Queue size limit exceeded: used {used}, limit {limit}")]
    QueueSize { limit: usize, used: usize },

    #[error("Timeout exceeded: limit {limit:?}")]
    Timeout { limit: Duration },

    #[error("Too many concurrent tasks: used {used}, limit {limit}")]
    ConcurrentTasks { limit: usize, used: usize },
}

impl BudgetViolation {
    /// Get violation severity (0.0 to 1.0, where 1.0 is most severe)
    pub fn severity(&self) -> f64 {
        match self {
            BudgetViolation::CpuTime { limit, used } => {
                (used.as_secs_f64() / limit.as_secs_f64() - 1.0).min(10.0) / 10.0
            }
            BudgetViolation::Memory { limit, used } => {
                (*used as f64 / *limit as f64 - 1.0).min(10.0) / 10.0
            }
            BudgetViolation::QueueSize { limit, used } => {
                (*used as f64 / *limit as f64 - 1.0).min(10.0) / 10.0
            }
            BudgetViolation::Timeout { .. } => 1.0, // Timeouts are always critical
            BudgetViolation::ConcurrentTasks { limit, used } => {
                (*used as f64 / *limit as f64 - 1.0).min(5.0) / 5.0
            }
        }
    }

    /// Check if this violation is critical (should trigger immediate action)
    pub fn is_critical(&self) -> bool {
        match self {
            BudgetViolation::Timeout { .. } => true,
            _ => self.severity() > 0.5,
        }
    }
}

/// Resource monitor for tracking actor resource usage
#[derive(Debug)]
pub struct ResourceMonitor {
    actor_id: ActorId,
    budget: Arc<RwLock<ResourceBudget>>,
    usage_history: Arc<RwLock<Vec<ResourceUsage>>>,
    start_time: Instant,
    system_monitor: Arc<SystemMonitor>,
}

impl ResourceMonitor {
    pub fn new(actor_id: ActorId, budget: ResourceBudget) -> Self {
        Self {
            actor_id,
            budget: Arc::new(RwLock::new(budget)),
            usage_history: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            system_monitor: Arc::new(SystemMonitor::new()),
        }
    }

    /// Get current resource usage
    pub async fn get_usage(&self) -> ResourceUsage {
        self.system_monitor.collect_usage(self.actor_id).await
    }

    /// Update resource budget
    pub async fn update_budget(&self, budget: ResourceBudget) {
        *self.budget.write().await = budget;
        debug!(actor_id = %self.actor_id, "Resource budget updated");
    }

    /// Check for budget violations
    pub async fn check_violations(&self) -> Option<BudgetViolation> {
        let usage = self.get_usage().await;
        let budget = self.budget.read().await;

        // Store usage in history
        {
            let mut history = self.usage_history.write().await;
            history.push(usage.clone());

            // Keep only last 100 measurements
            if history.len() > 100 {
                history.remove(0);
            }
        }

        budget.limits.check_violation(&usage)
    }

    /// Get resource usage history
    pub async fn get_usage_history(&self) -> Vec<ResourceUsage> {
        self.usage_history.read().await.clone()
    }

    /// Get current budget
    pub async fn get_budget(&self) -> ResourceBudget {
        self.budget.read().await.clone()
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// System-wide resource monitoring
#[derive(Debug)]
pub struct SystemMonitor {
    sys: Arc<RwLock<sysinfo::System>>,
    actor_usage: Arc<RwLock<HashMap<ActorId, ResourceUsage>>>,
    config: Arc<RwLock<SystemResourceConfig>>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();

        Self {
            sys: Arc::new(RwLock::new(sys)),
            actor_usage: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(SystemResourceConfig::default())),
        }
    }

    /// Collect current resource usage for an actor
    pub async fn collect_usage(&self, actor_id: ActorId) -> ResourceUsage {
        let mut sys = self.sys.write().await;
        sys.refresh_memory();
        sys.refresh_cpu_all();

        // For now, we'll use system-wide metrics
        // In a real implementation, this would track per-actor usage
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let load_avg = sysinfo::System::load_average();

        let usage = ResourceUsage {
            memory_bytes: used_memory,
            cpu_time: None,  // Would need more complex tracking
            queue_size: 0,   // Would be provided by actor
            active_tasks: 0, // Would be provided by actor
            timestamp: chrono::Utc::now(),
            system_load: Some(load_avg.one),
            memory_pressure: used_memory as f64 / total_memory as f64,
        };

        // Store in actor usage map
        {
            let mut actor_usage = self.actor_usage.write().await;
            actor_usage.insert(actor_id, usage.clone());
        }

        usage
    }

    /// Get system-wide resource status
    pub async fn get_system_status(&self) -> SystemResourceStatus {
        let mut sys = self.sys.write().await;
        sys.refresh_all();

        let memory_pressure = sys.used_memory() as f64 / sys.total_memory() as f64;
        let config = self.config.read().await;

        SystemResourceStatus {
            total_memory: sys.total_memory(),
            used_memory: sys.used_memory(),
            available_memory: sys.available_memory(),
            memory_pressure,
            load_average: sysinfo::System::load_average().one,
            cpu_count: sys.cpus().len() as u32,
            memory_warning: memory_pressure > config.memory_pressure_warning,
            memory_critical: memory_pressure > config.memory_pressure_critical,
            active_actors: self.actor_usage.read().await.len(),
        }
    }

    /// Update system configuration
    pub async fn update_config(&self, config: SystemResourceConfig) {
        *self.config.write().await = config;
    }

    /// Get usage for all actors
    pub async fn get_all_usage(&self) -> HashMap<ActorId, ResourceUsage> {
        self.actor_usage.read().await.clone()
    }
}

/// System-wide resource status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceStatus {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub memory_pressure: f64,
    pub load_average: f64,
    pub cpu_count: u32,
    pub memory_warning: bool,
    pub memory_critical: bool,
    pub active_actors: usize,
}
