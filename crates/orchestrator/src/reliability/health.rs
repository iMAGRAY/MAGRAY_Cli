//! Agent Health Monitoring System
//!
//! Provides comprehensive health checking capabilities for all agents in the multi-agent orchestration system.
//! Integrates with EventBus for health event publishing and supports automatic unhealthy agent recovery.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::events::{AgentEventPublisher, AgentStatus, AgentTopics, DefaultAgentEventPublisher};

/// Health status for an agent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Agent is healthy and responding normally
    Healthy,
    /// Agent is degraded but still functional
    Degraded { reason: String },
    /// Agent is unhealthy and may need intervention
    Unhealthy { reason: String },
    /// Agent is unresponsive and needs restart
    Unresponsive,
    /// Agent status is unknown (e.g., just started)
    Unknown,
}

/// Detailed health information for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub agent_type: String,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub response_time_ms: Option<u64>,
    pub memory_usage_mb: Option<u64>,
    pub cpu_usage_percent: Option<f64>,
    pub active_tasks: u32,
    pub error_count: u32,
    pub restart_count: u32,
    pub uptime_seconds: u64,
    pub metadata: serde_json::Value,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// How often to perform health checks
    pub check_interval: Duration,
    /// Maximum time to wait for health check response
    pub check_timeout: Duration,
    /// How long to wait for heartbeat before marking as unresponsive
    pub heartbeat_timeout: Duration,
    /// Number of consecutive failures before marking as unhealthy
    pub failure_threshold: u32,
    /// Whether to automatically restart unhealthy agents
    pub auto_restart: bool,
    /// Maximum number of restart attempts
    pub max_restart_attempts: u32,
    /// Cool-down period between restart attempts
    pub restart_cooldown: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(5),
            heartbeat_timeout: Duration::from_secs(60),
            failure_threshold: 3,
            auto_restart: true,
            max_restart_attempts: 3,
            restart_cooldown: Duration::from_secs(30),
        }
    }
}

/// Health check interface that all agents must implement
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Perform a health check on the agent
    async fn check_health(&self) -> Result<HealthReport>;

    /// Get the agent's unique identifier
    fn agent_id(&self) -> Uuid;

    /// Get the agent's name
    fn agent_name(&self) -> &str;

    /// Get the agent's type
    fn agent_type(&self) -> &str;

    /// Record a heartbeat from the agent
    async fn heartbeat(&self) -> Result<()>;

    /// Get the last recorded heartbeat time
    fn last_heartbeat(&self) -> Option<DateTime<Utc>>;

    /// Check if the agent is currently healthy
    fn is_healthy(&self) -> bool;

    /// Restart the agent (if supported)
    async fn restart(&self) -> Result<()>;
}

/// Agent health tracker for monitoring individual agents
struct AgentHealthTracker {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub agent_type: String,
    pub current_status: HealthStatus,
    pub last_check: Option<DateTime<Utc>>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub consecutive_failures: u32,
    pub restart_count: u32,
    pub last_restart: Option<DateTime<Utc>>,
    pub start_time: Instant,
    pub health_checker: Arc<dyn HealthChecker>,
}

impl AgentHealthTracker {
    pub fn new(health_checker: Arc<dyn HealthChecker>) -> Self {
        Self {
            agent_id: health_checker.agent_id(),
            agent_name: health_checker.agent_name().to_string(),
            agent_type: health_checker.agent_type().to_string(),
            current_status: HealthStatus::Unknown,
            last_check: None,
            last_heartbeat: None,
            consecutive_failures: 0,
            restart_count: 0,
            last_restart: None,
            start_time: Instant::now(),
            health_checker,
        }
    }

    /// Check if agent needs restart based on configuration
    pub fn needs_restart(&self, config: &HealthCheckConfig) -> bool {
        match &self.current_status {
            HealthStatus::Unresponsive => {
                self.restart_count < config.max_restart_attempts
                    && self
                        .last_restart
                        .map(|last| {
                            Utc::now() - last
                                > chrono::Duration::from_std(config.restart_cooldown)
                                    .expect("Operation failed - converted from unwrap()")
                        })
                        .unwrap_or(true)
            }
            HealthStatus::Unhealthy { .. } => {
                config.auto_restart
                    && self.consecutive_failures >= config.failure_threshold
                    && self.restart_count < config.max_restart_attempts
                    && self
                        .last_restart
                        .map(|last| {
                            Utc::now() - last
                                > chrono::Duration::from_std(config.restart_cooldown)
                                    .expect("Operation failed - converted from unwrap()")
                        })
                        .unwrap_or(true)
            }
            _ => false,
        }
    }
}

/// Central health monitoring system for all agents
pub struct HealthMonitor {
    config: HealthCheckConfig,
    agents: Arc<RwLock<HashMap<Uuid, AgentHealthTracker>>>,
    event_publisher: Arc<dyn AgentEventPublisher>,
    is_running: Arc<RwLock<bool>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthCheckConfig) -> Self {
        let event_publisher = Arc::new(DefaultAgentEventPublisher::new(
            Uuid::new_v4(),
            "health_monitor".to_string(),
            "HealthMonitor".to_string(),
        ));

        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_publisher,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register an agent for health monitoring
    pub async fn register_agent(&self, health_checker: Arc<dyn HealthChecker>) -> Result<()> {
        let agent_id = health_checker.agent_id();
        let agent_name = health_checker.agent_name().to_string();
        let agent_type = health_checker.agent_type().to_string();

        info!(
            "Registering agent for health monitoring: {} ({})",
            agent_name, agent_type
        );

        let tracker = AgentHealthTracker::new(health_checker);

        {
            let mut agents = self.agents.write().await;
            agents.insert(agent_id, tracker);
        }

        // Publish agent registration event
        self.event_publisher
            .publish_lifecycle_event(
                AgentStatus::Started,
                serde_json::json!({
                    "action": "registered_for_health_monitoring",
                    "agent_name": agent_name,
                    "agent_type": agent_type
                }),
            )
            .await?;

        Ok(())
    }

    /// Unregister an agent from health monitoring
    pub async fn unregister_agent(&self, agent_id: Uuid) -> Result<()> {
        let agent_info = {
            let mut agents = self.agents.write().await;
            agents.remove(&agent_id)
        };

        if let Some(agent) = agent_info {
            info!(
                "Unregistering agent from health monitoring: {} ({})",
                agent.agent_name, agent.agent_type
            );

            // Publish agent unregistration event
            self.event_publisher
                .publish_lifecycle_event(
                    AgentStatus::Stopped,
                    serde_json::json!({
                        "action": "unregistered_from_health_monitoring",
                        "agent_name": agent.agent_name,
                        "agent_type": agent.agent_type
                    }),
                )
                .await?;
        }

        Ok(())
    }

    /// Start the health monitoring loop
    pub async fn start(&self) -> Result<()> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                warn!("Health monitor is already running");
                return Ok(());
            }
            *is_running = true;
        }

        info!(
            "Starting health monitor with check interval: {:?}",
            self.config.check_interval
        );

        let mut interval = interval(self.config.check_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            // Check if we should stop
            {
                let is_running = self.is_running.read().await;
                if !*is_running {
                    break;
                }
            }

            interval.tick().await;

            if let Err(e) = self.check_all_agents().await {
                error!("Error during health check cycle: {}", e);
            }
        }

        info!("Health monitor stopped");
        Ok(())
    }

    /// Stop the health monitoring loop
    pub fn stop(&self) {
        let is_running = self.is_running.clone();
        tokio::spawn(async move {
            let mut is_running = is_running.write().await;
            *is_running = false;
            info!("Stopping health monitor");
        });
    }

    /// Check health of all registered agents
    async fn check_all_agents(&self) -> Result<()> {
        let agent_ids: Vec<Uuid> = {
            let agents = self.agents.read().await;
            agents.keys().cloned().collect()
        };

        for agent_id in agent_ids {
            if let Err(e) = self.check_agent_health(agent_id).await {
                error!("Failed to check health for agent {}: {}", agent_id, e);
            }
        }

        Ok(())
    }

    /// Check health of a specific agent
    async fn check_agent_health(&self, agent_id: Uuid) -> Result<()> {
        let health_report = {
            let agents = self.agents.read().await;
            if let Some(tracker) = agents.get(&agent_id) {
                // Check if heartbeat is still valid
                let heartbeat_expired = tracker
                    .last_heartbeat
                    .map(|last| {
                        Utc::now() - last
                            > chrono::Duration::from_std(self.config.heartbeat_timeout)
                                .expect("Operation failed - converted from unwrap()")
                    })
                    .unwrap_or(true);

                if heartbeat_expired {
                    Some((HealthStatus::Unresponsive, tracker.health_checker.clone()))
                } else {
                    // Perform actual health check
                    match tokio::time::timeout(
                        self.config.check_timeout,
                        tracker.health_checker.check_health(),
                    )
                    .await
                    {
                        Ok(Ok(report)) => Some((report.status, tracker.health_checker.clone())),
                        Ok(Err(e)) => {
                            warn!("Health check failed for agent {}: {}", agent_id, e);
                            Some((
                                HealthStatus::Unhealthy {
                                    reason: e.to_string(),
                                },
                                tracker.health_checker.clone(),
                            ))
                        }
                        Err(_) => {
                            warn!("Health check timed out for agent {}", agent_id);
                            Some((
                                HealthStatus::Unhealthy {
                                    reason: "Health check timeout".to_string(),
                                },
                                tracker.health_checker.clone(),
                            ))
                        }
                    }
                }
            } else {
                None
            }
        };

        if let Some((new_status, health_checker)) = health_report {
            self.update_agent_status(agent_id, new_status, health_checker)
                .await?;
        }

        Ok(())
    }

    /// Update agent status and handle unhealthy agents
    async fn update_agent_status(
        &self,
        agent_id: Uuid,
        new_status: HealthStatus,
        health_checker: Arc<dyn HealthChecker>,
    ) -> Result<()> {
        let needs_restart = {
            let mut agents = self.agents.write().await;
            if let Some(tracker) = agents.get_mut(&agent_id) {
                let old_status = tracker.current_status.clone();
                tracker.current_status = new_status.clone();
                tracker.last_check = Some(Utc::now());

                // Update failure count
                match &new_status {
                    HealthStatus::Healthy => tracker.consecutive_failures = 0,
                    HealthStatus::Degraded { .. } => {
                        // Degraded doesn't count as failure, but doesn't reset count either
                    }
                    HealthStatus::Unhealthy { .. } | HealthStatus::Unresponsive => {
                        tracker.consecutive_failures += 1;
                    }
                    HealthStatus::Unknown => {}
                }

                // Check if status changed
                if old_status != new_status {
                    info!(
                        "Agent {} status changed: {:?} -> {:?}",
                        tracker.agent_name, old_status, new_status
                    );
                }

                tracker.needs_restart(&self.config)
            } else {
                false
            }
        };

        // Publish health status event
        let agent_status = match &new_status {
            HealthStatus::Healthy => AgentStatus::Healthy,
            HealthStatus::Degraded { reason } => AgentStatus::Unhealthy {
                reason: reason.clone(),
            },
            HealthStatus::Unhealthy { reason } => AgentStatus::Unhealthy {
                reason: reason.clone(),
            },
            HealthStatus::Unresponsive => AgentStatus::Unhealthy {
                reason: "Agent is unresponsive".to_string(),
            },
            HealthStatus::Unknown => AgentStatus::Unhealthy {
                reason: "Status unknown".to_string(),
            },
        };

        self.event_publisher
            .publish_lifecycle_event(
                agent_status,
                serde_json::json!({
                    "health_status": new_status,
                    "agent_id": agent_id,
                    "timestamp": Utc::now()
                }),
            )
            .await?;

        // Handle unhealthy agent restart if needed
        if needs_restart {
            self.restart_agent(agent_id, health_checker).await?;
        }

        Ok(())
    }

    /// Restart an unhealthy agent
    async fn restart_agent(
        &self,
        agent_id: Uuid,
        health_checker: Arc<dyn HealthChecker>,
    ) -> Result<()> {
        let (agent_name, agent_type) = {
            let agents = self.agents.read().await;
            if let Some(tracker) = agents.get(&agent_id) {
                (tracker.agent_name.clone(), tracker.agent_type.clone())
            } else {
                return Ok(());
            }
        };

        info!(
            "Attempting to restart unhealthy agent: {} ({})",
            agent_name, agent_type
        );

        // Attempt restart
        match health_checker.restart().await {
            Ok(()) => {
                // Update restart count and timestamp
                {
                    let mut agents = self.agents.write().await;
                    if let Some(tracker) = agents.get_mut(&agent_id) {
                        tracker.restart_count += 1;
                        tracker.last_restart = Some(Utc::now());
                        tracker.consecutive_failures = 0;
                        tracker.current_status = HealthStatus::Unknown; // Reset status after restart
                    }
                }

                info!(
                    "Successfully restarted agent: {} ({})",
                    agent_name, agent_type
                );

                // Publish restart event
                self.event_publisher
                    .publish_lifecycle_event(
                        AgentStatus::Started,
                        serde_json::json!({
                            "action": "agent_restarted",
                            "agent_name": agent_name,
                            "agent_type": agent_type,
                            "restart_reason": "automatic_health_check_recovery"
                        }),
                    )
                    .await?;
            }
            Err(e) => {
                error!(
                    "Failed to restart agent {} ({}): {}",
                    agent_name, agent_type, e
                );

                // Publish restart failure event
                self.event_publisher
                    .publish_lifecycle_event(
                        AgentStatus::Failed {
                            error: format!("Restart failed: {}", e),
                        },
                        serde_json::json!({
                            "action": "agent_restart_failed",
                            "agent_name": agent_name,
                            "agent_type": agent_type,
                            "error": e.to_string()
                        }),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Get health report for a specific agent
    pub async fn get_agent_health(&self, agent_id: Uuid) -> Option<HealthStatus> {
        let agents = self.agents.read().await;
        agents
            .get(&agent_id)
            .map(|tracker| tracker.current_status.clone())
    }

    /// Get health reports for all agents
    pub async fn get_all_health_reports(&self) -> Vec<(Uuid, String, String, HealthStatus)> {
        let agents = self.agents.read().await;
        agents
            .values()
            .map(|tracker| {
                (
                    tracker.agent_id,
                    tracker.agent_name.clone(),
                    tracker.agent_type.clone(),
                    tracker.current_status.clone(),
                )
            })
            .collect()
    }

    /// Record heartbeat for an agent
    pub async fn record_heartbeat(&self, agent_id: Uuid) -> Result<()> {
        {
            let mut agents = self.agents.write().await;
            if let Some(tracker) = agents.get_mut(&agent_id) {
                tracker.last_heartbeat = Some(Utc::now());

                // If agent was unresponsive and now we got heartbeat, mark as unknown for re-check
                if matches!(tracker.current_status, HealthStatus::Unresponsive) {
                    tracker.current_status = HealthStatus::Unknown;
                }
            }
        }

        Ok(())
    }

    /// Check if any agents are unhealthy
    pub async fn has_unhealthy_agents(&self) -> bool {
        let agents = self.agents.read().await;
        agents.values().any(|tracker| {
            matches!(
                tracker.current_status,
                HealthStatus::Unhealthy { .. } | HealthStatus::Unresponsive
            )
        })
    }

    /// Get count of agents by status
    pub async fn get_status_counts(&self) -> HashMap<String, u32> {
        let agents = self.agents.read().await;
        let mut counts = HashMap::new();

        for tracker in agents.values() {
            let status_key = match &tracker.current_status {
                HealthStatus::Healthy => "healthy",
                HealthStatus::Degraded { .. } => "degraded",
                HealthStatus::Unhealthy { .. } => "unhealthy",
                HealthStatus::Unresponsive => "unresponsive",
                HealthStatus::Unknown => "unknown",
            };
            *counts.entry(status_key.to_string()).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockHealthChecker {
        agent_id: Uuid,
        agent_name: String,
        agent_type: String,
        health_status: Arc<tokio::sync::RwLock<HealthStatus>>,
        heartbeat_time: Arc<tokio::sync::RwLock<Option<DateTime<Utc>>>>,
        restart_called: Arc<AtomicBool>,
    }

    impl MockHealthChecker {
        fn new(agent_name: String, agent_type: String) -> Self {
            Self {
                agent_id: Uuid::new_v4(),
                agent_name,
                agent_type,
                health_status: Arc::new(tokio::sync::RwLock::new(HealthStatus::Healthy)),
                heartbeat_time: Arc::new(tokio::sync::RwLock::new(Some(Utc::now()))),
                restart_called: Arc::new(AtomicBool::new(false)),
            }
        }

        async fn set_health_status(&self, status: HealthStatus) {
            let mut health = self.health_status.write().await;
            *health = status;
        }
    }

    #[async_trait]
    impl HealthChecker for MockHealthChecker {
        async fn check_health(&self) -> Result<HealthReport> {
            let status = self.health_status.read().await.clone();
            Ok(HealthReport {
                agent_id: self.agent_id,
                agent_name: self.agent_name.clone(),
                agent_type: self.agent_type.clone(),
                status,
                timestamp: Utc::now(),
                last_heartbeat: self.last_heartbeat(),
                response_time_ms: Some(10),
                memory_usage_mb: Some(100),
                cpu_usage_percent: Some(5.0),
                active_tasks: 0,
                error_count: 0,
                restart_count: 0,
                uptime_seconds: 3600,
                metadata: serde_json::json!({}),
            })
        }

        fn agent_id(&self) -> Uuid {
            self.agent_id
        }

        fn agent_name(&self) -> &str {
            &self.agent_name
        }

        fn agent_type(&self) -> &str {
            &self.agent_type
        }

        async fn heartbeat(&self) -> Result<()> {
            let mut heartbeat = self.heartbeat_time.write().await;
            *heartbeat = Some(Utc::now());
            Ok(())
        }

        fn last_heartbeat(&self) -> Option<DateTime<Utc>> {
            // For tests we need a blocking version - using try_read
            self.heartbeat_time.try_read().ok().and_then(|guard| *guard)
        }

        fn is_healthy(&self) -> bool {
            // For tests we need a blocking version - using try_read
            self.health_status
                .try_read()
                .map(|guard| matches!(*guard, HealthStatus::Healthy))
                .unwrap_or(false)
        }

        async fn restart(&self) -> Result<()> {
            self.restart_called.store(true, Ordering::SeqCst);
            self.set_health_status(HealthStatus::Healthy).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_health_monitor_registration() {
        let config = HealthCheckConfig::default();
        let monitor = HealthMonitor::new(config);

        let checker = Arc::new(MockHealthChecker::new(
            "test_agent".to_string(),
            "TestAgent".to_string(),
        ));

        let agent_id = checker.agent_id();

        // Register agent
        monitor
            .register_agent(checker)
            .await
            .expect("Async operation should succeed");

        // Check that agent is registered
        let health = monitor.get_agent_health(agent_id).await;
        assert!(health.is_some());
    }

    #[tokio::test]
    async fn test_health_check_unhealthy_agent() {
        let mut config = HealthCheckConfig::default();
        config.auto_restart = true;
        config.failure_threshold = 1;

        let monitor = HealthMonitor::new(config);

        let checker = Arc::new(MockHealthChecker::new(
            "test_agent".to_string(),
            "TestAgent".to_string(),
        ));

        let agent_id = checker.agent_id();

        // Set agent as unhealthy
        checker
            .set_health_status(HealthStatus::Unhealthy {
                reason: "Test failure".to_string(),
            })
            .await;

        // Register agent
        monitor
            .register_agent(checker.clone())
            .await
            .expect("Async operation should succeed");

        // Perform health check
        monitor
            .check_agent_health(agent_id)
            .await
            .expect("Async operation should succeed");

        // Give some time for restart to be triggered
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check that restart was called
        assert!(checker.restart_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_heartbeat_recording() {
        let config = HealthCheckConfig::default();
        let monitor = HealthMonitor::new(config);

        let checker = Arc::new(MockHealthChecker::new(
            "test_agent".to_string(),
            "TestAgent".to_string(),
        ));

        let agent_id = checker.agent_id();

        // Register agent
        monitor
            .register_agent(checker)
            .await
            .expect("Async operation should succeed");

        // Record heartbeat
        monitor
            .record_heartbeat(agent_id)
            .await
            .expect("Async operation should succeed");

        // Heartbeat recording is internal, so we just verify no errors
        // In a real scenario, this would update the last_heartbeat time
    }

    #[tokio::test]
    async fn test_status_counts() {
        let config = HealthCheckConfig::default();
        let monitor = HealthMonitor::new(config);

        // Initially no agents
        let counts = monitor.get_status_counts().await;
        assert!(counts.is_empty());

        // After adding agents, counts would be updated
        // This is more of an integration test with actual agent registration
    }
}
