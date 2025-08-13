//! Base Actor Trait and Context
//!
//! This module defines the core BaseActor trait that all actors in the system must implement,
//! along with the ActorContext for runtime information and lifecycle management.

use super::{ActorError, ActorHealth, ActorId, ActorMessage, ActorState};
use crate::resources::{ResourceBudget, ResourceMonitor};
use async_trait::async_trait;
use common::event_bus::{EventBus, Topic};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Core trait that all actors must implement
#[async_trait]
pub trait BaseActor: Send + Sync + 'static {
    /// Get the actor's unique identifier
    fn id(&self) -> ActorId;

    /// Get the actor's type name for logging/debugging
    fn actor_type(&self) -> &'static str;

    /// Initialize the actor (called once before processing starts)
    async fn initialize(&mut self, _context: &ActorContext) -> Result<(), ActorError> {
        debug!(actor_id = %self.id(), actor_type = %self.actor_type(), "Initializing actor");
        Ok(())
    }

    /// Handle incoming messages
    async fn handle_message(
        &mut self,
        message: ActorMessage,
        context: &ActorContext,
    ) -> Result<(), ActorError>;

    /// Called when the actor is shutting down (cleanup opportunity)
    async fn shutdown(&mut self, _context: &ActorContext) -> Result<(), ActorError> {
        info!(actor_id = %self.id(), actor_type = %self.actor_type(), "Shutting down actor");
        Ok(())
    }

    /// Called when the actor is being restarted (after a crash)
    async fn on_restart(&mut self, _context: &ActorContext) -> Result<(), ActorError> {
        warn!(actor_id = %self.id(), actor_type = %self.actor_type(), "Restarting actor");
        Ok(())
    }

    /// Health check - return current health status
    async fn health_check(&self, context: &ActorContext) -> ActorHealth {
        context.get_health().await
    }

    /// Get resource budget for this actor
    fn resource_budget(&self) -> ResourceBudget {
        ResourceBudget::default()
    }
}

/// Runtime context provided to actors
pub struct ActorContext {
    pub actor_id: ActorId,
    pub system_sender: mpsc::UnboundedSender<SystemMessage>,
    pub event_bus: Arc<EventBus<serde_json::Value>>,
    pub resource_monitor: Arc<ResourceMonitor>,
    pub state: Arc<RwLock<ActorState>>,
    pub health: Arc<RwLock<ActorHealth>>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub messages_processed: Arc<std::sync::atomic::AtomicU64>,
}

impl ActorContext {
    pub fn new(
        actor_id: ActorId,
        system_sender: mpsc::UnboundedSender<SystemMessage>,
        event_bus: Arc<EventBus<serde_json::Value>>,
        resource_monitor: Arc<ResourceMonitor>,
    ) -> Self {
        let now = chrono::Utc::now();
        let health = ActorHealth::new(actor_id);

        Self {
            actor_id,
            system_sender,
            event_bus,
            resource_monitor,
            state: Arc::new(RwLock::new(ActorState::Initializing)),
            health: Arc::new(RwLock::new(health)),
            start_time: now,
            messages_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get current actor state
    pub async fn get_state(&self) -> ActorState {
        *self.state.read().await
    }

    /// Set actor state
    pub async fn set_state(&self, state: ActorState) {
        let mut current_state = self.state.write().await;
        if *current_state != state {
            debug!(actor_id = %self.actor_id, old_state = %current_state, new_state = %state, "Actor state transition");
            *current_state = state;

            // Update health
            let mut health = self.health.write().await;
            health.state = state;
            health.uptime = chrono::Utc::now() - self.start_time;
        }
    }

    /// Get current health status
    pub async fn get_health(&self) -> ActorHealth {
        let mut health = self.health.write().await;
        health.uptime = chrono::Utc::now() - self.start_time;
        health.messages_processed = self
            .messages_processed
            .load(std::sync::atomic::Ordering::Relaxed);
        health.resource_usage = self.resource_monitor.get_usage().await;
        health.clone()
    }

    /// Record an error
    pub async fn record_error(&self, error: &str) {
        let mut health = self.health.write().await;
        health.last_error = Some(error.to_string());
        error!(actor_id = %self.actor_id, error = %error, "Actor error recorded");
    }

    /// Increment message counter
    pub fn increment_messages(&self) {
        self.messages_processed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Send system message
    pub async fn send_system_message(&self, message: SystemMessage) -> Result<(), ActorError> {
        self.system_sender
            .send(message)
            .map_err(|_| ActorError::MessageSendFailed(self.actor_id))?;
        Ok(())
    }

    /// Publish event to event bus  
    pub async fn publish_event(
        &self,
        topic: &'static str,
        data: serde_json::Value,
    ) -> Result<(), ActorError> {
        self.event_bus.publish(Topic(topic), data).await;
        Ok(())
    }

    /// Subscribe to events
    pub async fn subscribe_to_events(&self, topics: &[&'static str]) -> Result<(), ActorError> {
        for topic in topics {
            let _receiver = self.event_bus.subscribe(Topic(topic)).await;
            // TODO: Store receivers for message processing
        }
        Ok(())
    }
}

/// System messages sent between actors and the system
#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// Actor has started successfully
    ActorStarted(ActorId),

    /// Actor has stopped
    ActorStopped(ActorId),

    /// Actor has crashed
    ActorCrashed(ActorId, String),

    /// Actor health report
    HealthReport(ActorHealth),

    /// Resource budget violation
    BudgetViolation(ActorId, String),

    /// Request actor restart
    RestartRequest(ActorId, String),
}

/// Actor lifecycle management
pub struct ActorLifecycle {
    pub actor: Box<dyn BaseActor>,
    pub context: ActorContext,
    pub receiver: mpsc::UnboundedReceiver<ActorMessage>,
    pub shutdown_token: tokio_util::sync::CancellationToken,
}

impl ActorLifecycle {
    pub fn new(
        actor: Box<dyn BaseActor>,
        context: ActorContext,
        receiver: mpsc::UnboundedReceiver<ActorMessage>,
    ) -> Self {
        Self {
            actor,
            context,
            receiver,
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    /// Start the actor lifecycle
    pub async fn run(mut self) -> Result<(), ActorError> {
        let actor_id = self.actor.id();
        let actor_type = self.actor.actor_type();

        info!(actor_id = %actor_id, actor_type = %actor_type, "Starting actor");

        // Initialize actor
        if let Err(e) = self.actor.initialize(&self.context).await {
            error!(actor_id = %actor_id, error = %e, "Actor initialization failed");
            self.context.set_state(ActorState::Crashed).await;
            return Err(e);
        }

        // Set state to running
        self.context.set_state(ActorState::Running).await;

        // Send started message
        let _ = self
            .context
            .send_system_message(SystemMessage::ActorStarted(actor_id))
            .await;

        // Main message loop
        loop {
            tokio::select! {
                // Handle shutdown signal
                _ = self.shutdown_token.cancelled() => {
                    debug!(actor_id = %actor_id, "Received shutdown signal");
                    break;
                }

                // Handle incoming messages
                msg = self.receiver.recv() => {
                    match msg {
                        Some(ActorMessage::Stop) => {
                            debug!(actor_id = %actor_id, "Received stop message");
                            break;
                        }
                        Some(message) => {
                            self.context.increment_messages();

                            if let Err(e) = self.actor.handle_message(message, &self.context).await {
                                error!(actor_id = %actor_id, error = %e, "Message handling failed");
                                self.context.record_error(&e.to_string()).await;

                                // Check if this is a critical error that should crash the actor
                                match &e {
                                    ActorError::BudgetViolation(_, _) => {
                                        let _ = self.context.send_system_message(
                                            SystemMessage::BudgetViolation(actor_id, e.to_string())
                                        ).await;
                                    }
                                    _ => {
                                        // For other errors, continue processing
                                    }
                                }
                            }
                        }
                        None => {
                            debug!(actor_id = %actor_id, "Message channel closed");
                            break;
                        }
                    }
                }
            }
        }

        // Shutdown
        self.context.set_state(ActorState::Stopping).await;

        if let Err(e) = self.actor.shutdown(&self.context).await {
            error!(actor_id = %actor_id, error = %e, "Actor shutdown failed");
        }

        self.context.set_state(ActorState::Stopped).await;

        // Send stopped message
        let _ = self
            .context
            .send_system_message(SystemMessage::ActorStopped(actor_id))
            .await;

        info!(actor_id = %actor_id, "Actor stopped");
        Ok(())
    }

    /// Request graceful shutdown
    pub fn shutdown(&self) {
        self.shutdown_token.cancel();
    }
}
