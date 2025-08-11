//! Event Bus infrastructure for MAGRAY
//!
//! Implements the event-driven architecture described in ARCHITECTURE_PLAN_ADVANCED.md
//! Topics: intent, plan, tool.invoked, fs.diff, memory.upsert, policy.block, job.progress, llm.tokens, error

use crate::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub mod bus;
pub mod topics;

pub use bus::*;
pub use topics::*;

/// Event represents any event in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub correlation_id: Option<Uuid>,
}

/// Event handler trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle event
    async fn handle(&self, event: Event) -> Result<()>;

    /// Get topics this handler subscribes to
    fn topics(&self) -> Vec<String>;

    /// Get handler name
    fn name(&self) -> String;
}

/// Event publisher trait
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish event to topic
    async fn publish(&self, topic: &str, payload: serde_json::Value, source: &str) -> Result<()>;

    /// Publish event with correlation ID
    async fn publish_correlated(
        &self,
        topic: &str,
        payload: serde_json::Value,
        source: &str,
        correlation_id: Uuid,
    ) -> Result<()>;
}

/// Event subscriber trait  
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Subscribe to topic
    async fn subscribe(&self, topic: &str, handler: Arc<dyn EventHandler>) -> Result<()>;

    /// Unsubscribe from topic
    async fn unsubscribe(&self, topic: &str, handler_name: &str) -> Result<()>;

    /// Get subscribed topics
    async fn topics(&self) -> Vec<String>;
}

/// Backpressure configuration
#[derive(Debug, Clone)]
pub struct BackpressureConfig {
    pub max_queue_size: usize,
    pub slow_consumer_threshold_ms: u64,
    pub drop_strategy: DropStrategy,
}

#[derive(Debug, Clone)]
pub enum DropStrategy {
    DropOldest,
    DropNewest,
    BackpressurePublisher,
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            slow_consumer_threshold_ms: 1000,
            drop_strategy: DropStrategy::DropOldest,
        }
    }
}

/// Event bus statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    pub total_events_published: u64,
    pub total_events_delivered: u64,
    pub failed_deliveries: u64,
    pub active_subscriptions: usize,
    pub queue_sizes: HashMap<String, usize>,
    pub slow_consumers: Vec<String>,
}
