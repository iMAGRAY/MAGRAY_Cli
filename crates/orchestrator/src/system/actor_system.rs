//! Main Actor System Implementation
//!
//! This module provides the core ActorSystem that manages all actors, handles registration,
//! message routing, supervision, and system-wide resource management.

use crate::actors::base_actor::SystemMessage;
use crate::actors::{
    ActorContext, ActorError, ActorHandle, ActorHealth, ActorId, ActorLifecycle, ActorMessage,
    BaseActor,
};
use crate::resources::budget::{SystemMonitor, SystemResourceStatus};
use crate::resources::{ResourceBudget, ResourceMonitor, SystemResourceConfig};
use crate::system::{MessageRouter, Supervisor, SupervisorConfig};
use common::event_bus::EventBus;

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Configuration for the Actor System
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Maximum number of actors in the system
    pub max_actors: usize,

    /// Default resource configuration
    pub resource_config: SystemResourceConfig,

    /// Default supervisor configuration
    pub supervisor_config: SupervisorConfig,

    /// System event bus buffer size
    pub event_bus_buffer: usize,

    /// Health check interval
    pub health_check_interval: Duration,

    /// System shutdown timeout
    pub shutdown_timeout: Duration,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            max_actors: 1000,
            resource_config: SystemResourceConfig::default(),
            supervisor_config: SupervisorConfig::default(),
            event_bus_buffer: 10000,
            health_check_interval: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(60),
        }
    }
}

/// Main Actor System
pub struct ActorSystem {
    config: SystemConfig,
    actors: Arc<DashMap<ActorId, ActorInfo>>,
    event_bus: Arc<EventBus<serde_json::Value>>,
    message_router: Arc<MessageRouter>,
    supervisor: Arc<Supervisor>,
    system_monitor: Arc<SystemMonitor>,
    system_sender: mpsc::UnboundedSender<SystemMessage>,
    system_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<SystemMessage>>>>,
    health_monitor_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    shutdown_token: tokio_util::sync::CancellationToken,
}

/// Information about a registered actor
#[derive(Debug)]
struct ActorInfo {
    handle: ActorHandle,
    lifecycle_handle: JoinHandle<Result<(), ActorError>>,
    resource_monitor: Arc<ResourceMonitor>,
    supervisor_id: Option<ActorId>,
    metadata: ActorMetadata,
}

/// Actor metadata for system management
#[derive(Debug, Clone)]
pub struct ActorMetadata {
    pub actor_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tags: HashMap<String, String>,
    pub description: Option<String>,
}

impl ActorMetadata {
    pub fn new(actor_type: String) -> Self {
        Self {
            actor_type,
            created_at: chrono::Utc::now(),
            tags: HashMap::new(),
            description: None,
        }
    }

    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

impl ActorSystem {
    /// Create a new ActorSystem
    pub async fn new(config: SystemConfig) -> Result<Self, ActorSystemError> {
        info!("Initializing Actor System");

        // Create event bus
        let event_bus = Arc::new(EventBus::new(
            config.event_bus_buffer,
            tokio::time::Duration::from_secs(1),
        ));

        // Create system message channel
        let (system_sender, system_receiver) = mpsc::unbounded_channel();

        // Create system monitor
        let system_monitor = Arc::new(SystemMonitor::new());
        system_monitor
            .update_config(config.resource_config.clone())
            .await;

        // Create message router
        let message_router = Arc::new(MessageRouter::new());

        // Create supervisor
        let supervisor = Arc::new(Supervisor::new(config.supervisor_config.clone()));

        let system = Self {
            config,
            actors: Arc::new(DashMap::new()),
            event_bus,
            message_router,
            supervisor,
            system_monitor,
            system_sender,
            system_receiver: Arc::new(RwLock::new(Some(system_receiver))),
            health_monitor_handle: Arc::new(RwLock::new(None)),
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        };

        // Start system services
        system.start_system_services().await?;

        info!("Actor System initialized successfully");
        Ok(system)
    }

    /// Register and start an actor
    pub async fn spawn_actor(
        &self,
        actor: Box<dyn BaseActor>,
        budget: Option<ResourceBudget>,
        metadata: Option<ActorMetadata>,
    ) -> Result<ActorId, ActorSystemError> {
        let actor_id = actor.id();
        let actor_type = actor.actor_type().to_string();

        // Check if actor already exists
        if self.actors.contains_key(&actor_id) {
            return Err(ActorSystemError::ActorAlreadyExists(actor_id));
        }

        // Check system capacity
        if self.actors.len() >= self.config.max_actors {
            return Err(ActorSystemError::SystemCapacityExceeded);
        }

        info!(actor_id = %actor_id, actor_type = %actor_type, "Spawning actor");

        // Create resource budget and monitor
        let budget = budget.unwrap_or_else(|| {
            ResourceBudget::new()
                .cpu_time_limit(
                    self.config
                        .resource_config
                        .default_limits
                        .max_cpu_time
                        .unwrap_or(Duration::from_secs(30)),
                )
                .memory_limit(
                    self.config
                        .resource_config
                        .default_limits
                        .max_memory
                        .unwrap_or(512 * 1024 * 1024),
                )
        });

        let resource_monitor = Arc::new(ResourceMonitor::new(actor_id, budget));

        // Create message channel
        let (sender, receiver) = mpsc::unbounded_channel();

        // Create actor context
        let context = ActorContext::new(
            actor_id,
            self.system_sender.clone(),
            self.event_bus.clone(),
            resource_monitor.clone(),
        );

        // Create actor lifecycle
        let lifecycle = ActorLifecycle::new(actor, context, receiver);

        // Start actor lifecycle
        let lifecycle_handle = tokio::spawn(async move { lifecycle.run().await });

        // Create actor handle
        let handle = ActorHandle {
            id: actor_id,
            sender,
        };

        // Create metadata
        let metadata = metadata.unwrap_or_else(|| ActorMetadata::new(actor_type));

        // Store actor info
        let actor_info = ActorInfo {
            handle: handle.clone(),
            lifecycle_handle,
            resource_monitor,
            supervisor_id: None,
            metadata,
        };

        self.actors.insert(actor_id, actor_info);

        // Register with supervisor
        self.supervisor
            .register_actor(actor_id, handle.clone())
            .await;

        // Send start message
        if let Err(e) = handle.send(ActorMessage::Start).await {
            error!(actor_id = %actor_id, error = %e, "Failed to send start message");
            self.actors.remove(&actor_id);
            return Err(ActorSystemError::ActorStartFailed(actor_id, e.to_string()));
        }

        info!(actor_id = %actor_id, "Actor spawned successfully");
        Ok(actor_id)
    }

    /// Stop an actor
    pub async fn stop_actor(&self, actor_id: ActorId) -> Result<(), ActorSystemError> {
        let actor_info = self
            .actors
            .get(&actor_id)
            .ok_or(ActorSystemError::ActorNotFound(actor_id))?;

        info!(actor_id = %actor_id, "Stopping actor");

        // Send stop message
        if let Err(e) = actor_info.handle.send(ActorMessage::Stop).await {
            warn!(actor_id = %actor_id, error = %e, "Failed to send stop message");
        }

        // Wait for actor to stop with timeout
        let lifecycle_handle = &actor_info.lifecycle_handle;
        let timeout_result = tokio::time::timeout(self.config.shutdown_timeout, async {
            // We can't clone JoinHandle, so we just abort if needed
            lifecycle_handle.abort();
            Ok::<(), ()>(())
        });

        match timeout_result.await {
            Ok(_) => {
                debug!(actor_id = %actor_id, "Actor stopped gracefully");
            }
            Err(_) => {
                warn!(actor_id = %actor_id, "Actor stop timeout, forcing shutdown");
                actor_info.lifecycle_handle.abort();
            }
        }

        // Unregister from supervisor
        self.supervisor.unregister_actor(actor_id).await;

        // Remove from actors map
        self.actors.remove(&actor_id);

        info!(actor_id = %actor_id, "Actor stopped");
        Ok(())
    }

    /// Get actor handle
    pub fn get_actor(&self, actor_id: ActorId) -> Option<ActorHandle> {
        self.actors.get(&actor_id).map(|info| info.handle.clone())
    }

    /// Get all actor IDs
    pub fn get_actor_ids(&self) -> Vec<ActorId> {
        self.actors.iter().map(|entry| *entry.key()).collect()
    }

    /// Get actor health
    pub async fn get_actor_health(&self, actor_id: ActorId) -> Option<ActorHealth> {
        let actor_info = self.actors.get(&actor_id)?;
        let _usage = actor_info.resource_monitor.get_usage().await;

        // Create health status (simplified for now)
        Some(ActorHealth::new(actor_id))
    }

    /// Get system health
    pub async fn get_system_health(&self) -> SystemHealth {
        let actor_count = self.actors.len();
        let mut healthy_actors = 0;
        let crashed_actors = 0;

        // This is a simplified health check
        // In practice, we'd query each actor's actual health
        for _entry in self.actors.iter() {
            // For now, assume all actors are healthy if they're in the map
            healthy_actors += 1;
        }

        let system_status = self.system_monitor.get_system_status().await;

        SystemHealth {
            total_actors: actor_count,
            healthy_actors,
            crashed_actors,
            system_status,
            uptime: chrono::Utc::now() - chrono::Utc::now(), // TODO: Track actual start time
        }
    }

    /// Shutdown the entire system
    pub async fn shutdown(self) -> Result<(), ActorSystemError> {
        info!("Shutting down Actor System");

        // Cancel health monitor
        if let Some(handle) = self.health_monitor_handle.write().await.take() {
            handle.abort();
        }

        // Stop all actors
        let actor_ids: Vec<ActorId> = self.actors.iter().map(|entry| *entry.key()).collect();

        for actor_id in actor_ids {
            if let Err(e) = self.stop_actor(actor_id).await {
                warn!(actor_id = %actor_id, error = %e, "Failed to stop actor during shutdown");
            }
        }

        // Signal system shutdown
        self.shutdown_token.cancel();

        info!("Actor System shutdown complete");
        Ok(())
    }

    /// Start system services (health monitoring, etc.)
    async fn start_system_services(&self) -> Result<(), ActorSystemError> {
        // Start health monitor
        let health_monitor = self.start_health_monitor().await;
        *self.health_monitor_handle.write().await = Some(health_monitor);

        // Start system message processor
        self.start_system_message_processor().await;

        Ok(())
    }

    /// Start health monitoring task
    async fn start_health_monitor(&self) -> JoinHandle<()> {
        let actors = self.actors.clone();
        let interval = self.config.health_check_interval;
        let shutdown_token = self.shutdown_token.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = shutdown_token.cancelled() => {
                        debug!("Health monitor shutting down");
                        break;
                    }
                    _ = interval_timer.tick() => {
                        // Perform health checks
                        for entry in actors.iter() {
                            let actor_id = *entry.key();
                            let actor_info = entry.value();

                            // Check for budget violations
                            if let Some(violation) = actor_info.resource_monitor.check_violations().await {
                                warn!(actor_id = %actor_id, violation = %violation, "Resource budget violation detected");

                                // Handle violation based on enforcement policy
                                let budget = actor_info.resource_monitor.get_budget().await;
                                match budget.enforcement {
                                    crate::resources::EnforcementPolicy::Kill => {
                                        warn!(actor_id = %actor_id, "Killing actor due to budget violation");
                                        // TODO: Kill actor
                                    },
                                    crate::resources::EnforcementPolicy::Restart => {
                                        warn!(actor_id = %actor_id, "Restarting actor due to budget violation");
                                        // TODO: Restart actor
                                    },
                                    _ => {
                                        // Log only
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    /// Start system message processor
    async fn start_system_message_processor(&self) {
        let system_receiver = self.system_receiver.write().await.take();
        if let Some(mut receiver) = system_receiver {
            let shutdown_token = self.shutdown_token.clone();
            let supervisor = self.supervisor.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown_token.cancelled() => {
                            debug!("System message processor shutting down");
                            break;
                        }
                        msg = receiver.recv() => {
                            match msg {
                                Some(SystemMessage::ActorCrashed(actor_id, reason)) => {
                                    error!(actor_id = %actor_id, reason = %reason, "Actor crashed");
                                    supervisor.handle_actor_crash(actor_id, reason).await;
                                },
                                Some(SystemMessage::BudgetViolation(actor_id, reason)) => {
                                    warn!(actor_id = %actor_id, reason = %reason, "Budget violation");
                                    // Handle budget violation
                                },
                                Some(msg) => {
                                    debug!(?msg, "Received system message");
                                },
                                None => {
                                    debug!("System message channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}

/// System-wide health information
#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub total_actors: usize,
    pub healthy_actors: usize,
    pub crashed_actors: usize,
    pub system_status: SystemResourceStatus,
    pub uptime: chrono::Duration,
}

/// Actor System specific errors
#[derive(Debug, thiserror::Error)]
pub enum ActorSystemError {
    #[error("Actor {0} not found")]
    ActorNotFound(ActorId),

    #[error("Actor {0} already exists")]
    ActorAlreadyExists(ActorId),

    #[error("System capacity exceeded")]
    SystemCapacityExceeded,

    #[error("Actor {0} failed to start: {1}")]
    ActorStartFailed(ActorId, String),

    #[error("System initialization failed: {0}")]
    InitializationFailed(String),

    #[error("System shutdown failed: {0}")]
    ShutdownFailed(String),

    #[error("Actor error: {0}")]
    ActorError(#[from] ActorError),
}
