// @component: {"k":"C","id":"resource_manager","t":"Dynamic resource allocation and monitoring","m":{"cur":5,"tgt":90,"u":"%"},"f":["resources","allocation","monitoring","limits","optimization"]}

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::ResourceRequirements;

/// System resource usage statistics
#[derive(Debug, Clone)]
pub struct SystemResourceUsage {
    pub cpu_usage: f32,     // 0.0 to 1.0
    pub memory_usage: f32,  // 0.0 to 1.0
    pub io_usage: f32,      // 0.0 to 1.0
    pub network_usage: f32, // 0.0 to 1.0
    pub active_tasks: usize,
    pub timestamp: Instant,
}

impl Default for SystemResourceUsage {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            io_usage: 0.0,
            network_usage: 0.0,
            active_tasks: 0,
            timestamp: Instant::now(),
        }
    }
}

/// Resource allocation limits and policies
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_usage: f32,     // Maximum CPU usage threshold (0.0-1.0)
    pub max_memory_usage: f32,  // Maximum memory usage threshold (0.0-1.0)
    pub max_io_usage: f32,      // Maximum IO usage threshold (0.0-1.0)
    pub max_network_usage: f32, // Maximum network usage threshold (0.0-1.0)
    pub max_concurrent_tasks: usize,
    pub priority_boost_threshold: f32, // CPU threshold to boost high-priority tasks
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_usage: 0.8,     // 80% max CPU usage
            max_memory_usage: 0.9,  // 90% max memory usage
            max_io_usage: 0.8,      // 80% max IO usage
            max_network_usage: 0.7, // 70% max network usage
            max_concurrent_tasks: 10,
            priority_boost_threshold: 0.6, // Start prioritizing high-priority tasks at 60% CPU
        }
    }
}

/// Resource reservation for active tasks
#[derive(Debug, Clone)]
struct ResourceReservation {
    task_id: String,
    requirements: ResourceRequirements,
    #[allow(dead_code)] // Метка времени выделения ресурсов
    allocated_at: Instant,
    expires_at: Option<Instant>,
}

/// Dynamic resource manager with intelligent allocation
pub struct ResourceManager {
    current_usage: Arc<Mutex<SystemResourceUsage>>,
    resource_limits: ResourceLimits,
    active_reservations: Arc<Mutex<Vec<ResourceReservation>>>,
    usage_history: Arc<Mutex<Vec<SystemResourceUsage>>>,
    allocation_stats: Arc<Mutex<AllocationStats>>,
}

#[derive(Debug, Default)]
struct AllocationStats {
    total_requests: u64,
    approved_requests: u64,
    denied_requests: u64,
    high_priority_boosts: u64,
    resource_warnings: u64,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self::with_limits(ResourceLimits::default())
    }

    pub fn with_limits(limits: ResourceLimits) -> Self {
        info!(
            "🔧 Initializing ResourceManager with limits: CPU={:.0}%, Memory={:.0}%, Concurrent={}",
            limits.max_cpu_usage * 100.0,
            limits.max_memory_usage * 100.0,
            limits.max_concurrent_tasks
        );

        Self {
            current_usage: Arc::new(Mutex::new(SystemResourceUsage::default())),
            resource_limits: limits,
            active_reservations: Arc::new(Mutex::new(Vec::new())),
            usage_history: Arc::new(Mutex::new(Vec::new())),
            allocation_stats: Arc::new(Mutex::new(AllocationStats::default())),
        }
    }

    /// Check if resources can be allocated for a task
    pub async fn can_allocate_resources(
        &self,
        requirements: &ResourceRequirements,
    ) -> Result<bool> {
        debug!(
            "🔍 Checking resource availability for task requiring: CPU={:.1}%, Memory={:.1}%",
            requirements.cpu_intensity * 100.0,
            requirements.memory_usage * 100.0
        );

        let mut stats = self.allocation_stats.lock().await;
        stats.total_requests += 1;

        // Update current system usage
        self.update_system_usage().await?;

        let current = self.current_usage.lock().await;
        let reservations = self.active_reservations.lock().await;

        // Calculate projected usage if this task is allocated
        let mut projected_cpu = current.cpu_usage;
        let mut projected_memory = current.memory_usage;
        let mut projected_io = current.io_usage;
        let mut projected_network = current.network_usage;

        // Add existing reservations
        for reservation in reservations.iter() {
            projected_cpu += reservation.requirements.cpu_intensity;
            projected_memory += reservation.requirements.memory_usage;
            projected_io += reservation.requirements.io_operations;
            projected_network += reservation.requirements.network_usage;
        }

        // Add new requirements
        projected_cpu += requirements.cpu_intensity;
        projected_memory += requirements.memory_usage;
        projected_io += requirements.io_operations;
        projected_network += requirements.network_usage;

        // Check concurrent task limit
        if reservations.len() >= self.resource_limits.max_concurrent_tasks {
            debug!("❌ Resource allocation denied: max concurrent tasks reached");
            stats.denied_requests += 1;
            return Ok(false);
        }

        // Check resource thresholds
        let can_allocate = projected_cpu <= self.resource_limits.max_cpu_usage
            && projected_memory <= self.resource_limits.max_memory_usage
            && projected_io <= self.resource_limits.max_io_usage
            && projected_network <= self.resource_limits.max_network_usage;

        if can_allocate {
            stats.approved_requests += 1;
            debug!(
                "✅ Resources can be allocated: projected usage CPU={:.1}%, Memory={:.1}%",
                projected_cpu * 100.0,
                projected_memory * 100.0
            );
        } else {
            stats.denied_requests += 1;
            stats.resource_warnings += 1;
            warn!("❌ Resource allocation denied: would exceed limits. Projected: CPU={:.1}%, Memory={:.1}%",
                  projected_cpu * 100.0, projected_memory * 100.0);
        }

        Ok(can_allocate)
    }

    /// Reserve resources for a task
    pub async fn reserve_resources(
        &self,
        task_id: &str,
        requirements: ResourceRequirements,
    ) -> Result<()> {
        debug!("📋 Reserving resources for task: {}", task_id);

        let mut reservations = self.active_reservations.lock().await;

        let reservation = ResourceReservation {
            task_id: task_id.to_string(),
            requirements,
            allocated_at: Instant::now(),
            expires_at: None, // Could add expiration based on estimated duration
        };

        reservations.push(reservation);

        // Update current usage
        self.update_system_usage().await?;

        Ok(())
    }

    /// Release resources after task completion
    pub async fn release_resources(&self, task_id: &str) -> Result<()> {
        debug!("🔄 Releasing resources for task: {}", task_id);

        let mut reservations = self.active_reservations.lock().await;
        reservations.retain(|r| r.task_id != task_id);

        // Update current usage
        self.update_system_usage().await?;

        Ok(())
    }

    /// Update system resource usage (simulated - would integrate with actual system monitoring)
    async fn update_system_usage(&self) -> Result<()> {
        let mut usage = self.current_usage.lock().await;
        let reservations = self.active_reservations.lock().await;

        // Simulate system usage calculation
        // In a real implementation, this would query actual system metrics
        let base_cpu = 0.1 + (reservations.len() as f32 * 0.05); // Base load + task overhead
        let base_memory = 0.2 + (reservations.len() as f32 * 0.03);
        let base_io = 0.05;
        let base_network = 0.02;

        // Add reservation requirements
        let mut total_cpu = base_cpu;
        let mut total_memory = base_memory;
        let mut total_io = base_io;
        let mut total_network = base_network;

        for reservation in reservations.iter() {
            total_cpu += reservation.requirements.cpu_intensity;
            total_memory += reservation.requirements.memory_usage;
            total_io += reservation.requirements.io_operations;
            total_network += reservation.requirements.network_usage;
        }

        usage.cpu_usage = total_cpu.min(1.0);
        usage.memory_usage = total_memory.min(1.0);
        usage.io_usage = total_io.min(1.0);
        usage.network_usage = total_network.min(1.0);
        usage.active_tasks = reservations.len();
        usage.timestamp = Instant::now();

        // Store in history for trend analysis
        let usage_snapshot = usage.clone();
        drop(usage);
        drop(reservations);

        let mut history = self.usage_history.lock().await;
        history.push(usage_snapshot);

        // Keep only last 100 samples
        if history.len() > 100 {
            history.remove(0);
        }

        Ok(())
    }

    /// Get current system resource status
    pub async fn get_resource_status(&self) -> SystemResourceUsage {
        self.current_usage.lock().await.clone()
    }

    /// Check if system is under high load and should prioritize high-priority tasks
    pub async fn should_prioritize_high_priority(&self) -> bool {
        let usage = self.current_usage.lock().await;
        usage.cpu_usage >= self.resource_limits.priority_boost_threshold
    }

    /// Get resource utilization trends
    pub async fn get_utilization_trends(&self) -> HashMap<String, f32> {
        let history = self.usage_history.lock().await;

        if history.is_empty() {
            return HashMap::new();
        }

        let mut trends = HashMap::new();

        // Calculate average usage over recent history
        let recent_samples = history.iter().rev().take(10).collect::<Vec<_>>();

        if !recent_samples.is_empty() {
            let avg_cpu = recent_samples.iter().map(|s| s.cpu_usage).sum::<f32>()
                / recent_samples.len() as f32;
            let avg_memory = recent_samples.iter().map(|s| s.memory_usage).sum::<f32>()
                / recent_samples.len() as f32;
            let avg_io = recent_samples.iter().map(|s| s.io_usage).sum::<f32>()
                / recent_samples.len() as f32;
            let avg_network = recent_samples.iter().map(|s| s.network_usage).sum::<f32>()
                / recent_samples.len() as f32;

            trends.insert("cpu_trend".to_string(), avg_cpu);
            trends.insert("memory_trend".to_string(), avg_memory);
            trends.insert("io_trend".to_string(), avg_io);
            trends.insert("network_trend".to_string(), avg_network);
        }

        trends
    }

    /// Get comprehensive resource management statistics
    pub async fn get_resource_stats(&self) -> String {
        let mut stats = String::new();
        stats.push_str("🔧 Resource Manager Statistics\n\n");

        let usage = self.get_resource_status().await;
        let allocation_stats = self.allocation_stats.lock().await;
        let reservations = self.active_reservations.lock().await;

        stats.push_str("📊 Current System Usage:\n");
        stats.push_str(&format!(
            "  • CPU: {:.1}% (limit: {:.0}%)\n",
            usage.cpu_usage * 100.0,
            self.resource_limits.max_cpu_usage * 100.0
        ));
        stats.push_str(&format!(
            "  • Memory: {:.1}% (limit: {:.0}%)\n",
            usage.memory_usage * 100.0,
            self.resource_limits.max_memory_usage * 100.0
        ));
        stats.push_str(&format!(
            "  • IO: {:.1}% (limit: {:.0}%)\n",
            usage.io_usage * 100.0,
            self.resource_limits.max_io_usage * 100.0
        ));
        stats.push_str(&format!(
            "  • Network: {:.1}% (limit: {:.0}%)\n",
            usage.network_usage * 100.0,
            self.resource_limits.max_network_usage * 100.0
        ));

        stats.push_str("\n📋 Resource Allocation:\n");
        stats.push_str(&format!(
            "  • Active tasks: {} (limit: {})\n",
            reservations.len(),
            self.resource_limits.max_concurrent_tasks
        ));
        stats.push_str(&format!(
            "  • Total requests: {}\n",
            allocation_stats.total_requests
        ));
        stats.push_str(&format!(
            "  • Approved: {} ({:.1}%)\n",
            allocation_stats.approved_requests,
            if allocation_stats.total_requests > 0 {
                (allocation_stats.approved_requests as f32 / allocation_stats.total_requests as f32)
                    * 100.0
            } else {
                0.0
            }
        ));
        stats.push_str(&format!(
            "  • Denied: {} ({:.1}%)\n",
            allocation_stats.denied_requests,
            if allocation_stats.total_requests > 0 {
                (allocation_stats.denied_requests as f32 / allocation_stats.total_requests as f32)
                    * 100.0
            } else {
                0.0
            }
        ));

        stats.push_str("\n⚡ Performance Features:\n");
        stats.push_str(&format!(
            "  • Priority boosting: {} (threshold: {:.0}%)\n",
            if self.should_prioritize_high_priority().await {
                "Active"
            } else {
                "Inactive"
            },
            self.resource_limits.priority_boost_threshold * 100.0
        ));
        stats.push_str(&format!(
            "  • Resource warnings: {}\n",
            allocation_stats.resource_warnings
        ));
        stats.push_str(&format!(
            "  • High-priority boosts: {}\n",
            allocation_stats.high_priority_boosts
        ));

        stats
    }

    /// Cleanup expired reservations (for maintenance)
    pub async fn cleanup_expired_reservations(&self) -> usize {
        let mut reservations = self.active_reservations.lock().await;
        let initial_count = reservations.len();

        let now = Instant::now();
        reservations.retain(|r| {
            if let Some(expires_at) = r.expires_at {
                expires_at > now
            } else {
                true // Keep reservations without expiration
            }
        });

        let cleaned_count = initial_count - reservations.len();
        if cleaned_count > 0 {
            debug!(
                "🧹 Cleaned up {} expired resource reservations",
                cleaned_count
            );
        }

        cleaned_count
    }
}
