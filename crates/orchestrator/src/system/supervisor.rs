//! Supervisor Implementation for Fault Tolerance
//!
//! This module implements the Supervisor pattern for actor fault tolerance,
//! providing restart strategies, backoff policies, and hierarchical supervision.

use crate::actors::{ActorHandle, ActorId};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Restart strategies for failed actors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestartStrategy {
    /// Never restart crashed actors
    Never,

    /// Always restart crashed actors
    Always,

    /// Restart only if the actor has not exceeded the failure threshold
    FailureThreshold {
        max_failures: u32,
        within_duration: Duration,
    },

    /// One-for-one: only restart the failed actor
    OneForOne,

    /// One-for-all: restart all supervised actors when one fails
    OneForAll,

    /// Rest-for-one: restart the failed actor and all actors started after it
    RestForOne,
}

impl Default for RestartStrategy {
    fn default() -> Self {
        RestartStrategy::FailureThreshold {
            max_failures: 5,
            within_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Backoff policies for restart delays
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffPolicy {
    /// No delay between restarts
    None,

    /// Fixed delay between restarts
    Fixed(Duration),

    /// Exponential backoff with optional jitter
    Exponential {
        initial_delay: Duration,
        max_delay: Duration,
        multiplier: f64,
        jitter: bool,
    },

    /// Linear backoff
    Linear {
        initial_delay: Duration,
        increment: Duration,
        max_delay: Duration,
    },
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        BackoffPolicy::Exponential {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl BackoffPolicy {
    /// Calculate the delay for the nth restart attempt
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            BackoffPolicy::None => Duration::ZERO,
            BackoffPolicy::Fixed(delay) => *delay,
            BackoffPolicy::Exponential {
                initial_delay,
                max_delay,
                multiplier,
                jitter,
            } => {
                let delay = initial_delay.as_millis() as f64 * multiplier.powi(attempt as i32);
                let delay = Duration::from_millis(delay as u64).min(*max_delay);

                if *jitter {
                    // Add up to 10% jitter
                    let jitter_ms = (delay.as_millis() as f64 * 0.1 * rand::random()) as u64;
                    delay + Duration::from_millis(jitter_ms)
                } else {
                    delay
                }
            }
            BackoffPolicy::Linear {
                initial_delay,
                increment,
                max_delay,
            } => {
                let delay = *initial_delay + *increment * attempt;
                delay.min(*max_delay)
            }
        }
    }
}

/// Configuration for a supervisor
#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub restart_strategy: RestartStrategy,
    pub backoff_policy: BackoffPolicy,
    pub max_restart_intensity: u32, // Max restarts per intensity_period
    pub restart_intensity_period: Duration, // Time window for restart intensity
    pub escalation_threshold: u32,  // Escalate to parent after N failures
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            restart_strategy: RestartStrategy::default(),
            backoff_policy: BackoffPolicy::default(),
            max_restart_intensity: 10,
            restart_intensity_period: Duration::from_secs(60),
            escalation_threshold: 3,
        }
    }
}

/// Supervisor for managing actor lifecycles and fault tolerance
pub struct Supervisor {
    config: SupervisorConfig,
    supervised_actors: Arc<DashMap<ActorId, SupervisedActor>>,
    supervision_tree: Arc<RwLock<SupervisionTree>>,
    restart_history: Arc<RwLock<RestartHistory>>,
}

/// Information about a supervised actor
#[derive(Debug)]
struct SupervisedActor {
    handle: ActorHandle,
    supervision_info: SupervisionInfo,
}

/// Supervision information for an actor
#[derive(Debug, Clone)]
pub struct SupervisionInfo {
    pub supervisor_id: Option<ActorId>,
    pub children: Vec<ActorId>,
    pub failure_count: u32,
    pub last_failure: Option<Instant>,
    pub restart_count: u32,
    pub created_at: Instant,
}

impl SupervisionInfo {
    pub fn new() -> Self {
        Self {
            supervisor_id: None,
            children: Vec::new(),
            failure_count: 0,
            last_failure: None,
            restart_count: 0,
            created_at: Instant::now(),
        }
    }
}

/// Supervision tree structure
#[derive(Debug, Default)]
struct SupervisionTree {
    root_actors: Vec<ActorId>,
    parent_child_map: std::collections::HashMap<ActorId, Vec<ActorId>>,
    child_parent_map: std::collections::HashMap<ActorId, ActorId>,
}

impl SupervisionTree {
    fn add_actor(&mut self, actor_id: ActorId, parent_id: Option<ActorId>) {
        if let Some(parent) = parent_id {
            // Add to parent's children
            self.parent_child_map
                .entry(parent)
                .or_default()
                .push(actor_id);
            self.child_parent_map.insert(actor_id, parent);
        } else {
            // Root actor
            self.root_actors.push(actor_id);
        }
    }

    fn remove_actor(&mut self, actor_id: ActorId) {
        // Remove from parent's children
        if let Some(parent_id) = self.child_parent_map.remove(&actor_id) {
            if let Some(children) = self.parent_child_map.get_mut(&parent_id) {
                children.retain(|&id| id != actor_id);
            }
        } else {
            // Remove from root actors
            self.root_actors.retain(|&id| id != actor_id);
        }

        // Remove from parent_child_map if it has children
        self.parent_child_map.remove(&actor_id);
    }

    fn get_children(&self, actor_id: ActorId) -> Vec<ActorId> {
        self.parent_child_map
            .get(&actor_id)
            .cloned()
            .unwrap_or_default()
    }

    fn get_parent(&self, actor_id: ActorId) -> Option<ActorId> {
        self.child_parent_map.get(&actor_id).copied()
    }
}

/// History of restart attempts
#[derive(Debug, Default)]
struct RestartHistory {
    attempts: std::collections::HashMap<ActorId, Vec<RestartAttempt>>,
}

#[derive(Debug, Clone)]
struct RestartAttempt {
    timestamp: Instant,
    reason: String,
    success: bool,
}

impl RestartHistory {
    fn add_attempt(&mut self, actor_id: ActorId, reason: String, success: bool) {
        let attempt = RestartAttempt {
            timestamp: Instant::now(),
            reason,
            success,
        };

        self.attempts.entry(actor_id).or_default().push(attempt);

        // Keep only recent attempts (last hour)
        let cutoff = Instant::now() - Duration::from_secs(3600);
        if let Some(attempts) = self.attempts.get_mut(&actor_id) {
            attempts.retain(|attempt| attempt.timestamp > cutoff);
        }
    }

    fn get_failure_count(&self, actor_id: ActorId, within: Duration) -> u32 {
        let cutoff = Instant::now() - within;

        self.attempts
            .get(&actor_id)
            .map(|attempts| {
                attempts
                    .iter()
                    .filter(|attempt| attempt.timestamp > cutoff && !attempt.success)
                    .count() as u32
            })
            .unwrap_or(0)
    }
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            config,
            supervised_actors: Arc::new(DashMap::new()),
            supervision_tree: Arc::new(RwLock::new(SupervisionTree::default())),
            restart_history: Arc::new(RwLock::new(RestartHistory::default())),
        }
    }

    /// Register an actor for supervision
    pub async fn register_actor(&self, actor_id: ActorId, handle: ActorHandle) {
        debug!(actor_id = %actor_id, "Registering actor for supervision");

        let supervised_actor = SupervisedActor {
            handle,
            supervision_info: SupervisionInfo::new(),
        };

        self.supervised_actors.insert(actor_id, supervised_actor);

        // Add to supervision tree (as root actor for now)
        let mut tree = self.supervision_tree.write().await;
        tree.add_actor(actor_id, None);
    }

    /// Unregister an actor from supervision
    pub async fn unregister_actor(&self, actor_id: ActorId) {
        debug!(actor_id = %actor_id, "Unregistering actor from supervision");

        self.supervised_actors.remove(&actor_id);

        // Remove from supervision tree
        let mut tree = self.supervision_tree.write().await;
        tree.remove_actor(actor_id);
    }

    /// Handle actor crash
    pub async fn handle_actor_crash(&self, actor_id: ActorId, reason: String) {
        error!(actor_id = %actor_id, reason = %reason, "Handling actor crash");

        // Record the failure
        {
            let mut history = self.restart_history.write().await;
            history.add_attempt(actor_id, reason.clone(), false);
        }

        // Check restart strategy
        if self.should_restart_actor(actor_id, &reason).await {
            self.restart_actor(actor_id, reason).await;
        } else {
            warn!(actor_id = %actor_id, "Actor will not be restarted based on strategy");
        }
    }

    /// Check if an actor should be restarted
    async fn should_restart_actor(&self, actor_id: ActorId, _reason: &str) -> bool {
        match self.config.restart_strategy {
            RestartStrategy::Never => false,
            RestartStrategy::Always => true,
            RestartStrategy::FailureThreshold {
                max_failures,
                within_duration,
            } => {
                let history = self.restart_history.read().await;
                let failure_count = history.get_failure_count(actor_id, within_duration);
                failure_count < max_failures
            }
            RestartStrategy::OneForOne
            | RestartStrategy::OneForAll
            | RestartStrategy::RestForOne => {
                // For these strategies, check the restart intensity
                let history = self.restart_history.read().await;
                let recent_failures =
                    history.get_failure_count(actor_id, self.config.restart_intensity_period);
                recent_failures < self.config.max_restart_intensity
            }
        }
    }

    /// Restart an actor
    async fn restart_actor(&self, actor_id: ActorId, reason: String) {
        info!(actor_id = %actor_id, reason = %reason, "Restarting actor");

        // Get the supervised actor info
        let supervised_actor = match self.supervised_actors.get(&actor_id) {
            Some(actor) => actor,
            None => {
                error!(actor_id = %actor_id, "Cannot restart actor: not found in supervision");
                return;
            }
        };

        // Calculate restart delay
        let attempt_number = supervised_actor.supervision_info.restart_count;
        let delay = self.config.backoff_policy.calculate_delay(attempt_number);

        if !delay.is_zero() {
            debug!(actor_id = %actor_id, delay = ?delay, "Waiting before restart");
            tokio::time::sleep(delay).await;
        }

        // TODO: Implement actual actor restart
        // This would involve:
        // 1. Creating a new instance of the actor
        // 2. Starting it with the same configuration
        // 3. Updating the handle in supervised_actors
        // 4. Handling any restart-specific initialization

        // For now, just log that we would restart
        info!(actor_id = %actor_id, "Actor restart completed (placeholder)");

        // Record successful restart
        {
            let mut history = self.restart_history.write().await;
            history.add_attempt(actor_id, reason, true);
        }

        // Update supervision info
        if let Some(mut actor) = self.supervised_actors.get_mut(&actor_id) {
            actor.supervision_info.restart_count += 1;
        }
    }

    /// Get supervision statistics
    pub async fn get_supervision_stats(&self) -> SupervisionStats {
        let actor_count = self.supervised_actors.len();
        let history = self.restart_history.read().await;

        let mut total_restarts = 0;
        let mut failed_restarts = 0;

        for attempts in history.attempts.values() {
            for attempt in attempts {
                if attempt.success {
                    total_restarts += 1;
                } else {
                    failed_restarts += 1;
                }
            }
        }

        SupervisionStats {
            supervised_actors: actor_count,
            total_restarts,
            failed_restarts,
            restart_success_rate: if total_restarts + failed_restarts > 0 {
                total_restarts as f64 / (total_restarts + failed_restarts) as f64
            } else {
                1.0
            },
        }
    }
}

/// Supervision statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisionStats {
    pub supervised_actors: usize,
    pub total_restarts: u32,
    pub failed_restarts: u32,
    pub restart_success_rate: f64,
}

// Add rand crate as dependency for jitter calculation
// This is a simplified implementation - in practice, you might want to use a proper random number generator
mod rand {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn random() -> f64 {
        let mut hasher = DefaultHasher::new();
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Operation failed - converted from unwrap()")
            .as_nanos();
        time.hash(&mut hasher);
        let hash = hasher.finish();
        (hash % 1000) as f64 / 1000.0
    }
}
