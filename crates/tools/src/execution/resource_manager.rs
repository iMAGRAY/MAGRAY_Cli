use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, info, warn};

/// Resource allocation tracking
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub allocation_id: String,
    pub tool_id: String,
    pub session_id: String,
    pub memory_mb: u64,
    pub cpu_cores: u32,
    pub allocated_at: SystemTime,
    pub expires_at: Option<SystemTime>,
}

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_cores: u32,
    pub max_execution_time: Duration,
    pub max_concurrent_allocations: u32,
    pub max_disk_usage_mb: Option<u64>,
    pub max_network_bandwidth_mbps: Option<u32>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: 2048,
            max_cpu_cores: 4,
            max_execution_time: Duration::from_secs(300),
            max_concurrent_allocations: 10,
            max_disk_usage_mb: Some(1024),
            max_network_bandwidth_mbps: Some(100),
        }
    }
}

/// Current resource usage snapshot
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub memory_mb: u64,
    pub cpu_percent: f32,
    pub disk_read_mb: u64,
    pub disk_write_mb: u64,
    pub network_read_mb: u64,
    pub network_write_mb: u64,
    pub execution_time: Duration,
}

impl ResourceUsage {
    pub fn is_within_limits(&self, limits: &ResourceLimits) -> bool {
        if self.memory_mb > limits.max_memory_mb {
            return false;
        }

        if self.execution_time > limits.max_execution_time {
            return false;
        }

        if let Some(max_disk) = limits.max_disk_usage_mb {
            if (self.disk_read_mb + self.disk_write_mb) > max_disk {
                return false;
            }
        }

        true
    }
}

/// Resource monitoring for active executions
#[derive(Debug)]
pub struct ResourceMonitor {
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    usage_history: Arc<Mutex<Vec<(SystemTime, ResourceUsage)>>>,
    #[allow(dead_code)] // Лимиты ресурсов для мониторинга
    limits: ResourceLimits,
}

impl ResourceMonitor {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            usage_history: Arc::new(Mutex::new(Vec::new())),
            limits,
        }
    }

    pub async fn record_usage(&self, usage: ResourceUsage) {
        let mut history = self.usage_history.lock().await;
        history.push((SystemTime::now(), usage));

        // Keep only recent history (last 1000 entries)
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    pub async fn get_current_total_usage(&self) -> ResourceUsage {
        let allocations = self.allocations.read().await;

        // In a real implementation, this would query actual system usage
        // For now, we estimate based on allocations
        let total_memory: u64 = allocations.values().map(|a| a.memory_mb).sum();
        let total_cores: u32 = allocations.values().map(|a| a.cpu_cores).sum();

        ResourceUsage {
            memory_mb: total_memory,
            cpu_percent: (total_cores as f32 / num_cpus::get() as f32) * 100.0,
            disk_read_mb: 0,
            disk_write_mb: 0,
            network_read_mb: 0,
            network_write_mb: 0,
            execution_time: Duration::default(),
        }
    }

    pub async fn get_usage_history(&self, last_n: usize) -> Vec<(SystemTime, ResourceUsage)> {
        let history = self.usage_history.lock().await;
        history.iter().rev().take(last_n).cloned().collect()
    }
}

/// Main resource manager
pub struct ResourceManager {
    limits: ResourceLimits,
    monitor: ResourceMonitor,
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    semaphores: ResourceSemaphores,
}

/// Semaphores for controlling concurrent resource usage
#[derive(Debug)]
struct ResourceSemaphores {
    memory_semaphore: Arc<Semaphore>,
    cpu_semaphore: Arc<Semaphore>,
    concurrent_executions: Arc<Semaphore>,
}

impl ResourceManager {
    pub fn new(limits: ResourceLimits) -> Self {
        let semaphores = ResourceSemaphores {
            memory_semaphore: Arc::new(Semaphore::new(limits.max_memory_mb as usize)),
            cpu_semaphore: Arc::new(Semaphore::new(limits.max_cpu_cores as usize)),
            concurrent_executions: Arc::new(Semaphore::new(
                limits.max_concurrent_allocations as usize,
            )),
        };

        let monitor = ResourceMonitor::new(limits.clone());

        Self {
            limits: limits.clone(),
            monitor,
            allocations: Arc::new(RwLock::new(HashMap::new())),
            semaphores,
        }
    }

    /// Allocate resources for tool execution
    pub async fn allocate_resources(
        &self,
        tool_id: &str,
        session_id: &str,
        requested_memory: u64,
        requested_cores: u32,
        execution_timeout: Option<Duration>,
    ) -> Result<ResourceGuard> {
        if requested_memory > self.limits.max_memory_mb {
            return Err(anyhow!(
                "Requested memory ({} MB) exceeds limit ({} MB)",
                requested_memory,
                self.limits.max_memory_mb
            ));
        }

        if requested_cores > self.limits.max_cpu_cores {
            return Err(anyhow!(
                "Requested CPU cores ({}) exceeds limit ({})",
                requested_cores,
                self.limits.max_cpu_cores
            ));
        }

        info!(
            "🔒 Allocating resources: {} MB memory, {} CPU cores for tool: {}",
            requested_memory, requested_cores, tool_id
        );

        let _concurrent_permit = Arc::clone(&self.semaphores.concurrent_executions)
            .acquire_owned()
            .await
            .map_err(|_| anyhow!("Failed to acquire concurrent execution permit"))?;

        let _memory_permits = Arc::clone(&self.semaphores.memory_semaphore)
            .acquire_many_owned(requested_memory as u32)
            .await
            .map_err(|_| anyhow!("Failed to acquire memory permits"))?;

        let _cpu_permits = Arc::clone(&self.semaphores.cpu_semaphore)
            .acquire_many_owned(requested_cores)
            .await
            .map_err(|_| anyhow!("Failed to acquire CPU permits"))?;

        let allocation_id = format!("{}_{}", tool_id, Self::current_timestamp());
        let expires_at = execution_timeout.map(|timeout| SystemTime::now() + timeout);

        let allocation = ResourceAllocation {
            allocation_id: allocation_id.clone(),
            tool_id: tool_id.to_string(),
            session_id: session_id.to_string(),
            memory_mb: requested_memory,
            cpu_cores: requested_cores,
            allocated_at: SystemTime::now(),
            expires_at,
        };

        // Store allocation
        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(allocation_id.clone(), allocation.clone());
        }

        debug!("✅ Resources allocated: {}", allocation_id);

        // Create resource guard that will clean up on drop
        Ok(ResourceGuard {
            allocation_id: allocation_id.clone(),
            resource_manager: Arc::new(ResourceManagerRef {
                allocations: Arc::clone(&self.allocations),
                semaphores: ResourceSemaphores {
                    memory_semaphore: Arc::clone(&self.semaphores.memory_semaphore),
                    cpu_semaphore: Arc::clone(&self.semaphores.cpu_semaphore),
                    concurrent_executions: Arc::clone(&self.semaphores.concurrent_executions),
                },
                monitor: self.monitor.allocations.clone(),
            }),
            _concurrent_permit: Box::new(_concurrent_permit),
            _memory_permits: Box::new(_memory_permits),
            _cpu_permits: Box::new(_cpu_permits),
            memory_mb: requested_memory,
            cpu_cores: requested_cores,
        })
    }

    /// Get current resource usage statistics
    pub async fn get_resource_stats(&self) -> ResourceStats {
        let allocations = self.allocations.read().await;
        let current_usage = self.monitor.get_current_total_usage().await;

        ResourceStats {
            total_allocations: allocations.len(),
            total_memory_allocated: allocations.values().map(|a| a.memory_mb).sum(),
            total_cpu_cores_allocated: allocations.values().map(|a| a.cpu_cores).sum(),
            current_usage: current_usage.clone(),
            available_memory: self
                .limits
                .max_memory_mb
                .saturating_sub(current_usage.memory_mb),
            available_cpu_cores: self
                .limits
                .max_cpu_cores
                .saturating_sub(allocations.values().map(|a| a.cpu_cores).sum()),
            limits: self.limits.clone(),
        }
    }

    /// Clean up expired allocations
    pub async fn cleanup_expired_allocations(&self) {
        let current_time = SystemTime::now();
        let mut allocations = self.allocations.write().await;

        let expired_keys: Vec<_> = allocations
            .iter()
            .filter(|(_, allocation)| {
                allocation
                    .expires_at
                    .map(|expires| current_time > expires)
                    .unwrap_or(false)
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            if let Some(allocation) = allocations.remove(&key) {
                warn!(
                    "🧹 Cleaned up expired resource allocation: {} for tool: {}",
                    key, allocation.tool_id
                );
            }
        }
    }

    /// Force release resources for a specific allocation
    pub async fn force_release(&self, allocation_id: &str) -> Result<()> {
        let mut allocations = self.allocations.write().await;

        if let Some(_allocation) = allocations.remove(allocation_id) {
            info!(
                "🔓 Force released resources for allocation: {}",
                allocation_id
            );
            Ok(())
        } else {
            Err(anyhow!("Allocation not found: {}", allocation_id))
        }
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

/// Resource statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub total_allocations: usize,
    pub total_memory_allocated: u64,
    pub total_cpu_cores_allocated: u32,
    pub current_usage: ResourceUsage,
    pub available_memory: u64,
    pub available_cpu_cores: u32,
    pub limits: ResourceLimits,
}

impl ResourceStats {
    pub fn utilization_percent(&self) -> (f32, f32) {
        let memory_util =
            (self.total_memory_allocated as f32 / self.limits.max_memory_mb as f32) * 100.0;
        let cpu_util =
            (self.total_cpu_cores_allocated as f32 / self.limits.max_cpu_cores as f32) * 100.0;
        (memory_util, cpu_util)
    }

    pub fn is_under_pressure(&self) -> bool {
        let (memory_util, cpu_util) = self.utilization_percent();
        memory_util > 80.0 || cpu_util > 80.0
    }
}

/// RAII resource guard that automatically releases resources on drop
pub struct ResourceGuard {
    allocation_id: String,
    resource_manager: Arc<ResourceManagerRef>,
    // Box permits to work around lifetime issues
    _concurrent_permit: Box<tokio::sync::OwnedSemaphorePermit>,
    _memory_permits: Box<tokio::sync::OwnedSemaphorePermit>,
    _cpu_permits: Box<tokio::sync::OwnedSemaphorePermit>,
    memory_mb: u64,
    cpu_cores: u32,
}

/// Reference to resource manager components for cleanup
struct ResourceManagerRef {
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    #[allow(dead_code)] // Семафоры для контроля ресурсов
    semaphores: ResourceSemaphores,
    #[allow(dead_code)] // Монитор для отслеживания состояния
    monitor: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
}

impl ResourceGuard {
    pub fn allocation_id(&self) -> &str {
        &self.allocation_id
    }

    pub fn memory_mb(&self) -> u64 {
        self.memory_mb
    }

    pub fn cpu_cores(&self) -> u32 {
        self.cpu_cores
    }

    /// Record current resource usage for monitoring
    pub async fn record_usage(&self, usage: ResourceUsage) {
        // In a real implementation, this would update monitoring data
        debug!(
            "📊 Recording usage for allocation {}: {:?}",
            self.allocation_id, usage
        );
    }
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        debug!(
            "🔓 Releasing resources for allocation: {}",
            self.allocation_id
        );

        // Resources are automatically released when semaphore permits are dropped
        // But we still need to clean up the allocation record
        let allocation_id = self.allocation_id.clone();
        let allocations = Arc::clone(&self.resource_manager.allocations);

        // Spawn cleanup task (non-blocking)
        tokio::spawn(async move {
            let mut allocs = allocations.write().await;
            allocs.remove(&allocation_id);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resource_allocation() {
        let limits = ResourceLimits {
            max_memory_mb: 1024,
            max_cpu_cores: 4,
            max_execution_time: Duration::from_secs(30),
            max_concurrent_allocations: 2,
            max_disk_usage_mb: None,
            max_network_bandwidth_mbps: None,
        };

        let manager = ResourceManager::new(limits);

        // Test successful allocation
        let guard = manager
            .allocate_resources("test_tool", "session1", 512, 2, None)
            .await;
        assert!(guard.is_ok());

        let _guard = guard.unwrap();

        // Test resource stats
        let stats = manager.get_resource_stats().await;
        assert_eq!(stats.total_memory_allocated, 512);
        assert_eq!(stats.total_cpu_cores_allocated, 2);
    }

    #[tokio::test]
    async fn test_resource_limits() {
        let limits = ResourceLimits {
            max_memory_mb: 1024,
            max_cpu_cores: 2,
            max_execution_time: Duration::from_secs(30),
            max_concurrent_allocations: 1,
            max_disk_usage_mb: None,
            max_network_bandwidth_mbps: None,
        };

        let manager = ResourceManager::new(limits);

        // Test exceeding memory limit
        let result = manager
            .allocate_resources("test_tool", "session1", 2048, 1, None)
            .await;
        assert!(result.is_err());

        // Test exceeding CPU limit
        let result = manager
            .allocate_resources("test_tool", "session1", 512, 4, None)
            .await;
        assert!(result.is_err());
    }
}
