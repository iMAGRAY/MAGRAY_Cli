//! Message Routing and Filtering System
//!
//! This module provides message routing capabilities between actors, including
//! filtering, topic-based routing, and delivery guarantees.

use crate::actors::{ActorId, ActorMessage};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Message router for inter-actor communication
pub struct MessageRouter {
    routing_table: Arc<RwLock<RoutingTable>>,
    filters: Arc<DashMap<String, MessageFilter>>,
    stats: Arc<RwLock<RoutingStats>>,
}

/// Routing table for mapping message types to actors
#[derive(Debug, Default)]
pub struct RoutingTable {
    /// Direct actor-to-actor routes
    direct_routes: HashMap<ActorId, Vec<ActorId>>,

    /// Topic-based routes
    topic_routes: HashMap<String, Vec<ActorId>>,

    /// Pattern-based routes (simplified)
    pattern_routes: HashMap<String, Vec<ActorId>>,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a direct route from one actor to another
    pub fn add_direct_route(&mut self, from: ActorId, to: ActorId) {
        self.direct_routes.entry(from).or_default().push(to);
        debug!(from = %from, to = %to, "Added direct route");
    }

    /// Remove a direct route
    pub fn remove_direct_route(&mut self, from: ActorId, to: ActorId) {
        if let Some(routes) = self.direct_routes.get_mut(&from) {
            routes.retain(|&id| id != to);
            if routes.is_empty() {
                self.direct_routes.remove(&from);
            }
        }
    }

    /// Add a topic-based route
    pub fn add_topic_route(&mut self, topic: String, actor_id: ActorId) {
        self.topic_routes
            .entry(topic.clone())
            .or_default()
            .push(actor_id);
        debug!(topic = %topic, actor_id = %actor_id, "Added topic route");
    }

    /// Remove a topic-based route
    pub fn remove_topic_route(&mut self, topic: &str, actor_id: ActorId) {
        if let Some(routes) = self.topic_routes.get_mut(topic) {
            routes.retain(|&id| id != actor_id);
            if routes.is_empty() {
                self.topic_routes.remove(topic);
            }
        }
    }

    /// Get direct routes for an actor
    pub fn get_direct_routes(&self, from: ActorId) -> Vec<ActorId> {
        self.direct_routes.get(&from).cloned().unwrap_or_default()
    }

    /// Get actors subscribed to a topic
    pub fn get_topic_subscribers(&self, topic: &str) -> Vec<ActorId> {
        self.topic_routes.get(topic).cloned().unwrap_or_default()
    }

    /// Get all topics
    pub fn get_topics(&self) -> Vec<String> {
        self.topic_routes.keys().cloned().collect()
    }
}

/// Message filter for routing decisions
#[derive(Debug, Clone)]
pub struct MessageFilter {
    pub name: String,
    pub filter_type: FilterType,
    pub enabled: bool,
}

/// Types of message filters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterType {
    /// Allow all messages
    AllowAll,

    /// Block all messages
    BlockAll,

    /// Allow messages from specific actors
    AllowActors(Vec<ActorId>),

    /// Block messages from specific actors
    BlockActors(Vec<ActorId>),

    /// Allow messages of specific types
    AllowMessageTypes(Vec<String>),

    /// Block messages of specific types
    BlockMessageTypes(Vec<String>),

    /// Custom filter with conditions
    Custom { conditions: HashMap<String, String> },
}

impl MessageFilter {
    pub fn new(name: String, filter_type: FilterType) -> Self {
        Self {
            name,
            filter_type,
            enabled: true,
        }
    }

    /// Check if a message should be allowed through this filter
    pub fn should_allow(&self, from: ActorId, message: &ActorMessage) -> bool {
        if !self.enabled {
            return true; // Disabled filters allow all
        }

        match &self.filter_type {
            FilterType::AllowAll => true,
            FilterType::BlockAll => false,
            FilterType::AllowActors(allowed) => allowed.contains(&from),
            FilterType::BlockActors(blocked) => !blocked.contains(&from),
            FilterType::AllowMessageTypes(allowed) => {
                let message_type = self.get_message_type(message);
                allowed.contains(&message_type)
            }
            FilterType::BlockMessageTypes(blocked) => {
                let message_type = self.get_message_type(message);
                !blocked.contains(&message_type)
            }
            FilterType::Custom { conditions: _ } => {
                // Simplified custom filter - in practice this would be more complex
                true
            }
        }
    }

    fn get_message_type(&self, message: &ActorMessage) -> String {
        match message {
            ActorMessage::Start => "start".to_string(),
            ActorMessage::Stop => "stop".to_string(),
            ActorMessage::Restart => "restart".to_string(),
            ActorMessage::Ping => "ping".to_string(),
            ActorMessage::Custom { message_type, .. } => message_type.clone(),
            ActorMessage::System { .. } => "system".to_string(),
            ActorMessage::Agent(_) => "agent".to_string(),
        }
    }
}

/// Routing statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RoutingStats {
    pub total_messages: u64,
    pub successful_routes: u64,
    pub failed_routes: u64,
    pub filtered_messages: u64,
    pub topic_deliveries: HashMap<String, u64>,
    pub actor_message_counts: HashMap<String, u64>, // Using String instead of ActorId for serialization
}

impl RoutingStats {
    pub fn record_message(&mut self) {
        self.total_messages += 1;
    }

    pub fn record_successful_route(&mut self) {
        self.successful_routes += 1;
    }

    pub fn record_failed_route(&mut self) {
        self.failed_routes += 1;
    }

    pub fn record_filtered_message(&mut self) {
        self.filtered_messages += 1;
    }

    pub fn record_topic_delivery(&mut self, topic: &str) {
        *self.topic_deliveries.entry(topic.to_string()).or_default() += 1;
    }

    pub fn record_actor_message(&mut self, actor_id: ActorId) {
        *self
            .actor_message_counts
            .entry(actor_id.to_string())
            .or_default() += 1;
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.total_messages == 0 {
            1.0
        } else {
            self.successful_routes as f64 / self.total_messages as f64
        }
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            routing_table: Arc::new(RwLock::new(RoutingTable::new())),
            filters: Arc::new(DashMap::new()),
            stats: Arc::new(RwLock::new(RoutingStats::default())),
        }
    }

    /// Add a direct route between actors
    pub async fn add_direct_route(&self, from: ActorId, to: ActorId) {
        let mut table = self.routing_table.write().await;
        table.add_direct_route(from, to);
    }

    /// Add a topic subscription
    pub async fn subscribe_to_topic(&self, actor_id: ActorId, topic: String) {
        let mut table = self.routing_table.write().await;
        table.add_topic_route(topic, actor_id);
    }

    /// Remove a topic subscription
    pub async fn unsubscribe_from_topic(&self, actor_id: ActorId, topic: String) {
        let mut table = self.routing_table.write().await;
        table.remove_topic_route(&topic, actor_id);
    }

    /// Add a message filter
    pub fn add_filter(&self, filter: MessageFilter) {
        self.filters.insert(filter.name.clone(), filter);
    }

    /// Remove a message filter
    pub fn remove_filter(&self, name: &str) {
        self.filters.remove(name);
    }

    /// Enable or disable a filter
    pub fn set_filter_enabled(&self, name: &str, enabled: bool) {
        if let Some(mut filter) = self.filters.get_mut(name) {
            filter.enabled = enabled;
        }
    }

    /// Route a message using direct routes
    pub async fn route_direct(&self, from: ActorId, message: ActorMessage) -> Vec<ActorId> {
        // Record message
        {
            let mut stats = self.stats.write().await;
            stats.record_message();
        }

        // Apply filters
        if !self.apply_filters(from, &message).await {
            let mut stats = self.stats.write().await;
            stats.record_filtered_message();
            return Vec::new();
        }

        // Get routes
        let table = self.routing_table.read().await;
        let routes = table.get_direct_routes(from);

        if !routes.is_empty() {
            let mut stats = self.stats.write().await;
            stats.record_successful_route();
            for &actor_id in &routes {
                stats.record_actor_message(actor_id);
            }
        }

        routes
    }

    /// Route a message using topic-based routing
    pub async fn route_topic(
        &self,
        from: ActorId,
        topic: &str,
        message: ActorMessage,
    ) -> Vec<ActorId> {
        // Record message
        {
            let mut stats = self.stats.write().await;
            stats.record_message();
        }

        // Apply filters
        if !self.apply_filters(from, &message).await {
            let mut stats = self.stats.write().await;
            stats.record_filtered_message();
            return Vec::new();
        }

        // Get topic subscribers
        let table = self.routing_table.read().await;
        let subscribers = table.get_topic_subscribers(topic);

        if !subscribers.is_empty() {
            let mut stats = self.stats.write().await;
            stats.record_successful_route();
            stats.record_topic_delivery(topic);
            for &actor_id in &subscribers {
                stats.record_actor_message(actor_id);
            }
        }

        subscribers
    }

    /// Apply all active filters to a message
    async fn apply_filters(&self, from: ActorId, message: &ActorMessage) -> bool {
        for filter_entry in self.filters.iter() {
            let filter = filter_entry.value();
            if !filter.should_allow(from, message) {
                debug!(
                    filter = %filter.name,
                    from = %from,
                    message_type = ?message,
                    "Message filtered out"
                );
                return false;
            }
        }
        true
    }

    /// Get routing statistics
    pub async fn get_stats(&self) -> RoutingStats {
        self.stats.read().await.clone()
    }

    /// Clear routing statistics
    pub async fn clear_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = RoutingStats::default();
    }

    /// Get all active topics
    pub async fn get_active_topics(&self) -> Vec<String> {
        let table = self.routing_table.read().await;
        table.get_topics()
    }

    /// Get topic subscriber count
    pub async fn get_topic_subscriber_count(&self, topic: &str) -> usize {
        let table = self.routing_table.read().await;
        table.get_topic_subscribers(topic).len()
    }

    /// Get routing table summary
    pub async fn get_routing_summary(&self) -> RoutingSummary {
        let table = self.routing_table.read().await;
        let stats = self.stats.read().await;

        RoutingSummary {
            total_direct_routes: table.direct_routes.len(),
            total_topics: table.topic_routes.len(),
            total_topic_subscriptions: table.topic_routes.values().map(|v| v.len()).sum(),
            active_filters: self.filters.len(),
            routing_stats: stats.clone(),
        }
    }
}

/// Summary of routing configuration and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSummary {
    pub total_direct_routes: usize,
    pub total_topics: usize,
    pub total_topic_subscriptions: usize,
    pub active_filters: usize,
    pub routing_stats: RoutingStats,
}
