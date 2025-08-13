//! Actor System Manager for Multi-Agent Orchestration
//!
//! This module provides the ActorSystemManager that handles lifecycle management
//! for multi-agent communication and coordination. It extends the base ActorSystem
//! with agent-specific functionality.

use super::{ActorError, ActorHandle, ActorId, ActorMessage, AgentMessage, BaseActor};
use crate::reliability::{AgentReliabilityConfig, AgentReliabilityManager, ReliabilityError};
use crate::system::{ActorSystem, SystemConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Agent types supported by the system
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AgentType {
    IntentAnalyzer,
    Planner,
    Executor,
    Critic,
    Scheduler,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::IntentAnalyzer => write!(f, "intent_analyzer"),
            AgentType::Planner => write!(f, "planner"),
            AgentType::Executor => write!(f, "executor"),
            AgentType::Critic => write!(f, "critic"),
            AgentType::Scheduler => write!(f, "scheduler"),
        }
    }
}

/// Configuration for agent communication
#[derive(Debug, Clone)]
pub struct AgentCommunicationConfig {
    /// Default message timeout in milliseconds
    pub default_timeout_ms: u64,

    /// Maximum number of pending requests per agent
    pub max_pending_requests: usize,

    /// Enable request/response correlation
    pub enable_correlation: bool,

    /// Enable message tracing for debugging
    pub enable_tracing: bool,
}

impl Default for AgentCommunicationConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30_000, // 30 seconds
            max_pending_requests: 100,
            enable_correlation: true,
            enable_tracing: true,
        }
    }
}

/// Actor System Manager for multi-agent orchestration
pub struct ActorSystemManager {
    actor_system: ActorSystem,
    agent_registry: Arc<RwLock<AgentRegistry>>,
    communication_config: AgentCommunicationConfig,
    request_tracker: Arc<RwLock<RequestTracker>>,
    reliability_manager: Arc<RwLock<AgentReliabilityManager>>,
}

/// Registry of active agents
#[derive(Debug, Default)]
struct AgentRegistry {
    /// Map of agent types to their actor handles
    agents: HashMap<AgentType, Vec<ActorHandle>>,

    /// Map of actor IDs to agent types for reverse lookup
    actor_to_agent: HashMap<ActorId, AgentType>,

    /// Load balancing state (round-robin counters)
    load_balancer: HashMap<AgentType, usize>,
}

impl AgentRegistry {
    fn register_agent(&mut self, agent_type: AgentType, handle: ActorHandle) {
        debug!(agent_type = %agent_type, actor_id = %handle.id, "Registering agent");

        self.agents
            .entry(agent_type.clone())
            .or_default()
            .push(handle.clone());
        self.actor_to_agent.insert(handle.id, agent_type.clone());

        // Initialize load balancer counter if needed
        self.load_balancer.entry(agent_type).or_insert(0);
    }

    fn unregister_agent(&mut self, actor_id: ActorId) -> Option<AgentType> {
        if let Some(agent_type) = self.actor_to_agent.remove(&actor_id) {
            debug!(agent_type = %agent_type, actor_id = %actor_id, "Unregistering agent");

            // Remove from agent list
            if let Some(handles) = self.agents.get_mut(&agent_type) {
                handles.retain(|h| h.id != actor_id);

                // Clean up empty agent type
                if handles.is_empty() {
                    self.agents.remove(&agent_type);
                    self.load_balancer.remove(&agent_type);
                }
            }

            Some(agent_type)
        } else {
            None
        }
    }

    fn get_agent_handle(&mut self, agent_type: &AgentType) -> Option<ActorHandle> {
        let handles = self.agents.get(agent_type)?;
        if handles.is_empty() {
            return None;
        }

        // Round-robin load balancing
        let counter = self.load_balancer.entry(agent_type.clone()).or_insert(0);
        let handle = handles[*counter % handles.len()].clone();
        *counter = (*counter + 1) % handles.len();

        Some(handle)
    }

    fn get_all_agents(&self, agent_type: &AgentType) -> Vec<ActorHandle> {
        self.agents.get(agent_type).cloned().unwrap_or_default()
    }

    fn get_agent_count(&self, agent_type: &AgentType) -> usize {
        self.agents.get(agent_type).map(|h| h.len()).unwrap_or(0)
    }

    fn get_all_agent_types(&self) -> Vec<AgentType> {
        self.agents.keys().cloned().collect()
    }
}

/// Request tracking for request/response correlation
#[derive(Debug, Default)]
struct RequestTracker {
    /// Pending requests by request ID
    pending_requests: HashMap<String, PendingRequest>,

    /// Request timeout tracking
    request_timeouts: HashMap<String, tokio::time::Instant>,
}

#[derive(Debug)]
struct PendingRequest {
    request_id: String,
    from_actor: ActorId,
    to_actor: ActorId,
    created_at: tokio::time::Instant,
    timeout_ms: u64,
}

impl RequestTracker {
    fn track_request(&mut self, request_id: String, from: ActorId, to: ActorId, timeout_ms: u64) {
        let now = tokio::time::Instant::now();

        let pending = PendingRequest {
            request_id: request_id.clone(),
            from_actor: from,
            to_actor: to,
            created_at: now,
            timeout_ms,
        };

        self.pending_requests.insert(request_id.clone(), pending);
        self.request_timeouts.insert(
            request_id,
            now + tokio::time::Duration::from_millis(timeout_ms),
        );
    }

    fn complete_request(&mut self, request_id: &str) -> Option<PendingRequest> {
        self.request_timeouts.remove(request_id);
        self.pending_requests.remove(request_id)
    }

    fn get_expired_requests(&self) -> Vec<String> {
        let now = tokio::time::Instant::now();
        self.request_timeouts
            .iter()
            .filter(|(_, &timeout)| now > timeout)
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn cleanup_expired(&mut self) -> Vec<PendingRequest> {
        let expired_ids = self.get_expired_requests();
        let mut expired_requests = Vec::new();

        for id in expired_ids {
            if let Some(request) = self.complete_request(&id) {
                expired_requests.push(request);
            }
        }

        expired_requests
    }
}

impl ActorSystemManager {
    /// Create a new ActorSystemManager
    pub async fn new(
        system_config: SystemConfig,
        communication_config: AgentCommunicationConfig,
    ) -> Result<Self, ActorError> {
        info!("Initializing Actor System Manager");

        let actor_system = ActorSystem::new(system_config)
            .await
            .map_err(|e| ActorError::InitializationFailed(ActorId::new(), e.to_string()))?;

        let manager = Self {
            actor_system,
            agent_registry: Arc::new(RwLock::new(AgentRegistry::default())),
            communication_config,
            request_tracker: Arc::new(RwLock::new(RequestTracker::default())),
            reliability_manager: Arc::new(RwLock::new(AgentReliabilityManager::new())),
        };

        // Start background tasks
        manager.start_background_tasks().await;

        info!("Actor System Manager initialized successfully");
        Ok(manager)
    }

    /// Create a new ActorSystemManager with custom reliability configuration
    pub async fn new_with_reliability(
        system_config: SystemConfig,
        communication_config: AgentCommunicationConfig,
        reliability_config: AgentReliabilityConfig,
    ) -> Result<Self, ActorError> {
        info!("Initializing Actor System Manager with custom reliability configuration");
        let actor_system = ActorSystem::new(system_config)
            .await
            .map_err(|e| ActorError::InitializationFailed(ActorId::new(), e.to_string()))?;

        let manager = Self {
            actor_system,
            agent_registry: Arc::new(RwLock::new(AgentRegistry::default())),
            communication_config,
            request_tracker: Arc::new(RwLock::new(RequestTracker::default())),
            reliability_manager: Arc::new(RwLock::new(AgentReliabilityManager::with_config(
                reliability_config,
            ))),
        };

        // Start background tasks
        manager.start_background_tasks().await;

        info!("Actor System Manager with reliability features initialized successfully");
        Ok(manager)
    }

    /// Register and start an agent
    pub async fn spawn_agent(
        &self,
        agent_type: AgentType,
        actor: Box<dyn BaseActor>,
    ) -> Result<ActorId, ActorError> {
        let actor_id = self
            .actor_system
            .spawn_actor(actor, None, None)
            .await
            .map_err(|e| ActorError::StartupFailed(ActorId::new(), e.to_string()))?;

        // Get the actor handle and register it
        if let Some(handle) = self.actor_system.get_actor(actor_id) {
            let mut registry = self.agent_registry.write().await;
            registry.register_agent(agent_type.clone(), handle);

            // Register with reliability manager
            let mut reliability = self.reliability_manager.write().await;
            reliability.register_agent(actor_id);
        }

        info!(agent_type = %agent_type, actor_id = %actor_id, "Agent spawned successfully");
        Ok(actor_id)
    }

    /// Stop an agent
    pub async fn stop_agent(&self, actor_id: ActorId) -> Result<(), ActorError> {
        // Unregister from agent registry
        {
            let mut registry = self.agent_registry.write().await;
            registry.unregister_agent(actor_id);
        }

        // Unregister from reliability manager
        {
            let mut reliability = self.reliability_manager.write().await;
            reliability.unregister_agent(actor_id);
        }

        // Stop the actor
        self.actor_system
            .stop_actor(actor_id)
            .await
            .map_err(|_e| ActorError::MessageSendFailed(actor_id))?;

        info!(actor_id = %actor_id, "Agent stopped successfully");
        Ok(())
    }

    /// Send a message to a specific agent type (with load balancing)
    pub async fn send_to_agent_type(
        &self,
        agent_type: AgentType,
        message: AgentMessage,
    ) -> Result<(), ActorError> {
        let handle = {
            let mut registry = self.agent_registry.write().await;
            registry.get_agent_handle(&agent_type)
        };

        if let Some(handle) = handle {
            let actor_message = ActorMessage::Agent(message);
            handle.send(actor_message).await?;
            Ok(())
        } else {
            Err(ActorError::NotFound(ActorId::new()))
        }
    }

    /// Broadcast a message to all agents of a specific type
    pub async fn broadcast_to_agent_type(
        &self,
        agent_type: AgentType,
        message: AgentMessage,
    ) -> Result<usize, ActorError> {
        let handles = {
            let registry = self.agent_registry.read().await;
            registry.get_all_agents(&agent_type)
        };

        if handles.is_empty() {
            return Err(ActorError::NotFound(ActorId::new()));
        }

        let mut sent_count = 0;
        let actor_message = ActorMessage::Agent(message);

        for handle in handles {
            if handle.send(actor_message.clone()).await.is_ok() {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    /// Send a request and track it for response correlation
    pub async fn send_request(
        &self,
        from_actor: ActorId,
        to_agent_type: AgentType,
        request: AgentMessage,
        timeout_ms: Option<u64>,
    ) -> Result<String, ActorError> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let timeout = timeout_ms.unwrap_or(self.communication_config.default_timeout_ms);

        // Wrap in request envelope if needed
        let agent_message = match request {
            AgentMessage::Request { .. } => request,
            _ => AgentMessage::Request {
                request_id: request_id.clone(),
                request_type: format!("{:?}", std::mem::discriminant(&request)),
                payload: serde_json::to_value(&request)
                    .map_err(|e| ActorError::MessageHandlingFailed(from_actor, e.to_string()))?,
            },
        };

        // Get target agent handle
        let target_handle = {
            let mut registry = self.agent_registry.write().await;
            registry.get_agent_handle(&to_agent_type)
        };

        if let Some(handle) = target_handle {
            // Track the request
            if self.communication_config.enable_correlation {
                let mut tracker = self.request_tracker.write().await;
                tracker.track_request(request_id.clone(), from_actor, handle.id, timeout);
            }

            // Send the message
            let actor_message = ActorMessage::Agent(agent_message);
            handle.send(actor_message).await?;

            Ok(request_id)
        } else {
            Err(ActorError::NotFound(ActorId::new()))
        }
    }

    /// Get agent statistics
    pub async fn get_agent_stats(&self) -> AgentSystemStats {
        let registry = self.agent_registry.read().await;
        let tracker = self.request_tracker.read().await;

        let mut agent_counts = HashMap::new();
        for agent_type in registry.get_all_agent_types() {
            agent_counts.insert(agent_type.clone(), registry.get_agent_count(&agent_type));
        }

        AgentSystemStats {
            agent_counts,
            pending_requests: tracker.pending_requests.len(),
            total_agents: registry.actor_to_agent.len(),
        }
    }

    /// Get reliability statistics for all agents
    pub async fn get_reliability_stats(&self) -> Vec<crate::reliability::AgentReliabilityStats> {
        let reliability = self.reliability_manager.read().await;
        reliability.get_all_stats()
    }

    /// Get reliability statistics for a specific agent
    pub async fn get_agent_reliability_stats(
        &self,
        actor_id: ActorId,
    ) -> Option<crate::reliability::AgentReliabilityStats> {
        let reliability = self.reliability_manager.read().await;
        reliability.get_agent_stats(actor_id)
    }

    /// Check if agent operations should be allowed based on circuit breaker state
    pub async fn should_allow_agent_operation(&self, actor_id: ActorId) -> bool {
        let mut reliability = self.reliability_manager.write().await;
        reliability.should_allow_operation(actor_id)
    }

    /// Record successful operation for reliability tracking
    pub async fn record_agent_success(&self, actor_id: ActorId) {
        let mut reliability = self.reliability_manager.write().await;
        reliability.record_success(actor_id);
    }

    /// Record failed operation for reliability tracking
    pub async fn record_agent_failure(&self, actor_id: ActorId) {
        let mut reliability = self.reliability_manager.write().await;
        reliability.record_failure(actor_id);
    }

    /// Execute agent operation with retry logic and timeout
    pub async fn execute_with_reliability<F, Fut, T>(
        &self,
        actor_id: ActorId,
        operation_name: String,
        operation: F,
    ) -> Result<T, ReliabilityError>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, ActorError>>,
        T: Clone,
    {
        // Check circuit breaker first
        if !self.should_allow_agent_operation(actor_id).await {
            return Err(ReliabilityError::CircuitBreakerOpen {
                actor_id,
                source: crate::reliability::CircuitBreakerError::Open,
            });
        }

        let (retry_policy_opt, timeout_manager_opt) = {
            let reliability = self.reliability_manager.read().await;
            (
                reliability.get_retry_policy(actor_id).cloned(),
                reliability.get_timeout_manager(actor_id).cloned(),
            )
        };

        // Get retry policy and timeout manager for the agent
        if let (Some(mut retry_policy), Some(mut timeout_manager)) =
            (retry_policy_opt, timeout_manager_opt)
        {
            // Execute with timeout
            let timeout_result = timeout_manager
                .execute(operation_name.clone(), || async {
                    // Execute with retry
                    retry_policy.execute(operation).await.map_err(|e| match e {
                        crate::reliability::RetryError::PermanentFailure { reason } => {
                            ActorError::MessageHandlingFailed(actor_id, reason)
                        }
                        _ => ActorError::MessageHandlingFailed(actor_id, e.to_string()),
                    })
                })
                .await;

            match timeout_result {
                Ok(result) => {
                    self.record_agent_success(actor_id).await;
                    result.map_err(|e| ReliabilityError::PermanentFailure {
                        actor_id,
                        error: format!("Actor operation failed: {}", e),
                    })
                }
                Err(timeout_err) => {
                    self.record_agent_failure(actor_id).await;
                    Err(ReliabilityError::Timeout {
                        actor_id,
                        source: timeout_err,
                    })
                }
            }
        } else {
            // Fallback to direct execution if reliability components not found
            let result = operation().await;
            match result {
                Ok(value) => {
                    self.record_agent_success(actor_id).await;
                    Ok(value)
                }
                Err(error) => {
                    self.record_agent_failure(actor_id).await;
                    Err(ReliabilityError::PermanentFailure {
                        actor_id,
                        error: error.to_string(),
                    })
                }
            }
        }
    }

    /// Start background maintenance tasks
    async fn start_background_tasks(&self) {
        // Start request timeout cleanup task
        if self.communication_config.enable_correlation {
            let tracker = self.request_tracker.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

                loop {
                    interval.tick().await;

                    let expired_requests = {
                        let mut tracker = tracker.write().await;
                        tracker.cleanup_expired()
                    };

                    if !expired_requests.is_empty() {
                        warn!("Cleaned up {} expired requests", expired_requests.len());
                        for request in expired_requests {
                            debug!(
                                request_id = %request.request_id,
                                from = %request.from_actor,
                                to = %request.to_actor,
                                "Request timed out"
                            );
                        }
                    }
                }
            });
        }
    }

    /// Shutdown the actor system manager
    pub async fn shutdown(self) -> Result<(), ActorError> {
        info!("Shutting down Actor System Manager");

        self.actor_system
            .shutdown()
            .await
            .map_err(|e| ActorError::InitializationFailed(ActorId::new(), e.to_string()))?;

        info!("Actor System Manager shutdown complete");
        Ok(())
    }
}

/// Statistics for the agent system
#[derive(Debug, Clone)]
pub struct AgentSystemStats {
    pub agent_counts: HashMap<AgentType, usize>,
    pub pending_requests: usize,
    pub total_agents: usize,
}

impl AgentSystemStats {
    pub fn get_agent_count(&self, agent_type: &AgentType) -> usize {
        self.agent_counts.get(agent_type).copied().unwrap_or(0)
    }

    pub fn is_agent_available(&self, agent_type: &AgentType) -> bool {
        self.get_agent_count(agent_type) > 0
    }

    pub fn get_total_agents(&self) -> usize {
        self.total_agents
    }
}
